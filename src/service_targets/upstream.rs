use std::{io::ErrorKind, sync::Arc};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use once_cell::sync::Lazy;
use poem::{
    http::{uri::Authority, StatusCode, Uri},
    Body, Endpoint, RequestParts, Response,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::ServiceTargetConfig;

static REQWEST_CLI: Lazy<Client> = Lazy::new(|| Client::new());

#[derive(Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
enum UpstreamScheme {
    Http,
    Https,
}

#[derive(Serialize, Deserialize)]
struct UpstreamConfig {
    scheme: UpstreamScheme,
    host: String,
}

#[typetag::serde(name = "upstream")]
impl ServiceTargetConfig for UpstreamConfig {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>> {
        let scheme = self.scheme;
        let authority: Authority = self
            .host
            .parse()
            .with_context(|| format!("failed to parse host `{}`", self.host))?;

        Ok(Arc::new(poem::endpoint::make(move |req| {
            let authority = authority.clone();

            async move {
                let (
                    RequestParts {
                        method,
                        uri,
                        headers,
                        ..
                    },
                    body,
                ) = req.into_parts();
                let mut uri_parts = uri.into_parts();

                uri_parts.scheme = match scheme {
                    UpstreamScheme::Http => Some(poem::http::uri::Scheme::HTTP),
                    UpstreamScheme::Https => Some(poem::http::uri::Scheme::HTTPS),
                };
                uri_parts.authority = Some(authority.clone());

                let new_uri = Uri::from_parts(uri_parts).unwrap().to_string();
                info!(uri = %new_uri, "forward to upstream");

                let mut req = reqwest::Request::new(method, new_uri.parse().unwrap());
                *req.headers_mut() = headers;
                *req.body_mut() = Some(reqwest::Body::wrap_stream(
                    tokio_util::io::ReaderStream::new(body.into_async_read()),
                ));

                match REQWEST_CLI.execute(req).await {
                    Ok(mut resp) => {
                        let mut new_resp = Response::default();
                        new_resp.set_status(resp.status());
                        std::mem::swap(new_resp.headers_mut(), resp.headers_mut());
                        new_resp.set_body(Body::from_async_read(
                            tokio_util::io::StreamReader::new(
                                resp.bytes_stream()
                                    .map_err(|err| std::io::Error::new(ErrorKind::Other, err)),
                            ),
                        ));
                        new_resp
                    }
                    Err(err) => {
                        error!(
                            error = %err,
                            "upstream error",
                        );
                        Response::builder()
                            .status(StatusCode::SERVICE_UNAVAILABLE)
                            .finish()
                    }
                }
            }
        })))
    }
}
