use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use hyper::Client;
use hyper_rustls::HttpsConnector;
use poem::{
    http::{header, uri::Authority, HeaderMap, HeaderValue, Method, StatusCode, Uri},
    web::RemoteAddr,
    Addr, Endpoint, Request, RequestParts, Response,
};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Role;

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
    #[serde(default)]
    websocket: bool,
}

const UPGRADE: HeaderValue = HeaderValue::from_static("Upgrade");
const WEBSOCKET: HeaderValue = HeaderValue::from_static("websocket");

#[typetag::serde(name = "upstream")]
impl EndpointConfig for Config {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>> {
        let https = HttpsConnector::with_webpki_roots();
        let client = Arc::new(Client::builder().build(https));
        let scheme = self.scheme;
        let websocket = self.websocket;
        let authority: Authority = self
            .host
            .parse()
            .with_context(|| format!("failed to parse host `{}`", self.host))?;

        Ok(Arc::new(poem::endpoint::make(move |req| {
            let client = client.clone();
            let authority = authority.clone();

            async move {
                if websocket
                    && req.headers().get(header::CONNECTION) == Some(&UPGRADE)
                    && req.headers().get(header::UPGRADE) == Some(&WEBSOCKET)
                {
                    if req.method() != Method::GET
                        || req.headers().get(header::SEC_WEBSOCKET_VERSION)
                            != Some(&HeaderValue::from_static("13"))
                    {
                        return StatusCode::BAD_REQUEST.into();
                    }

                    let upgrade = match req.take_upgrade() {
                        Ok(upgrade) => upgrade,
                        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into(),
                    };
                    let req = create_new_request(scheme, req, authority, true);

                    // websocket
                    let req = Into::<hyper::Request<_>>::into(req).map(|_| ());
                    let (upstream_ws, upstream_resp) =
                        match tokio_tungstenite::connect_async(req).await {
                            Ok(res) => res,
                            Err(err) => {
                                return Response::builder()
                                    .status(StatusCode::SERVICE_UNAVAILABLE)
                                    .body(err.to_string());
                            }
                        };

                    let upgraded = match upgrade.await {
                        Ok(upgraded) => upgraded,
                        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into(),
                    };
                    let client_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(
                        upgraded,
                        Role::Server,
                        None,
                    )
                    .await;
                    let (sink, stream) = client_stream.split();
                    let (upstream_sink, upstream_stream) = upstream_ws.split();

                    tokio::spawn(stream.forward(upstream_sink));
                    tokio::spawn(upstream_stream.forward(sink));

                    return upstream_resp.map(|_| hyper::Body::empty()).into();
                }

                match client
                    .request(create_new_request(scheme, req, authority, false).into())
                    .await
                {
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

fn create_new_request(
    scheme: UpstreamScheme,
    req: Request,
    authority: Authority,
    websocket: bool,
) -> Request {
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

    uri_parts.scheme = if !websocket {
        match scheme {
            UpstreamScheme::Http => Some(poem::http::uri::Scheme::HTTP),
            UpstreamScheme::Https => Some(poem::http::uri::Scheme::HTTPS),
        }
    } else {
        match scheme {
            UpstreamScheme::Http => Some("ws".parse().unwrap()),
            UpstreamScheme::Https => Some("wss".parse().unwrap()),
        }
    };
    uri_parts.authority = Some(authority.clone());

    add_proxy_headers(&mut headers, remote_addr);

    let new_uri = Uri::from_parts(uri_parts).unwrap();
    info!(uri = %new_uri, "forward to upstream");

    let mut new_req = Request::builder().method(method).uri(new_uri).body(body);
    *new_req.headers_mut() = headers;

    new_req
}
