use std::sync::Arc;

use anyhow::{Context, Result};
use hyper::Client;
use hyper_rustls::HttpsConnector;
use poem::{
    http::{uri::Authority, HeaderMap, HeaderValue, StatusCode, Uri},
    web::RemoteAddr,
    Addr, Endpoint, Request, RequestParts, Response,
};
use serde::{Deserialize, Serialize};

use crate::config::EndpointConfig;

#[derive(Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
enum UpstreamScheme {
    Http,
    Https,
}

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    scheme: UpstreamScheme,
    host: String,
}

#[typetag::serde(name = "upstream")]
impl EndpointConfig for Config {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>> {
        let https = HttpsConnector::with_webpki_roots();
        let client = Arc::new(Client::builder().build(https));
        let scheme = self.scheme;
        let authority: Authority = self
            .host
            .parse()
            .with_context(|| format!("failed to parse host `{}`", self.host))?;

        Ok(Arc::new(poem::endpoint::make(move |req| {
            let client = client.clone();
            let authority = authority.clone();

            async move {
                let remote_addr = req.remote_addr().clone();
                let (
                    RequestParts {
                        method,
                        uri,
                        mut headers,
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

                add_proxy_headers(&mut headers, remote_addr);

                let new_uri = Uri::from_parts(uri_parts).unwrap();
                info!(uri = %new_uri, "forward to upstream");

                let mut new_req = Request::builder().method(method).uri(new_uri).body(body);
                *new_req.headers_mut() = headers;

                match client.request(new_req.into()).await {
                    Ok(resp) => resp.into(),
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

fn add_proxy_headers(headers: &mut HeaderMap, remote_addr: RemoteAddr) {
    if let RemoteAddr(Addr::SocketAddr(remote_addr)) = remote_addr {
        match headers.get("x-forwarded-for") {
            Some(value) => {
                let mut value = value.as_bytes().to_vec();
                value.extend_from_slice(&*b", ");
                value.extend_from_slice(&remote_addr.to_string().into_bytes());
                if let Ok(value) = HeaderValue::from_bytes(&value) {
                    headers.insert("x-forwarded-for", value);
                }
            }
            None => {
                if let Ok(value) = HeaderValue::from_str(&remote_addr.to_string()) {
                    headers.insert("x-forwarded-for", value);
                }
            }
        }

        if let Ok(value) = HeaderValue::from_str(&remote_addr.to_string()) {
            headers.insert("x-real-ip", value);
        }
    }
}
