#![feature(never_type)]

use std::error::Error;
use std::net::SocketAddr;

use hyper::client::Client;
use hyper::http::uri::{Authority, Scheme};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Server, Uri};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port = std::env::var("ROON_DISPLAY_PROXY_PORT")
        .unwrap_or_else(|_| "8675".into())
        .parse::<u16>()
        .expect("please provide a valid port in ROON_DISPLAY_PROXY_PORT");
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let target = Authority::from_maybe_shared(
        std::env::var("ROON_DISPLAY_BACKEND")
            .expect("please supply a host-port pair in ROON_DISPLAY_BACKEND"),
    )
    .expect("please supply a valid host-port pair in ROON_DISPLAY_BACKEND");

    let make_service = make_service_fn(move |_conn: &AddrStream| {
        let target = target.clone();
        async move {
            Ok::<_, !>(service_fn(move |mut req: Request<Body>| {
                let client = Client::new();
                let method = req.method().clone();
                let mut parts = req.uri().clone().into_parts();
                parts.scheme = Some(Scheme::HTTP);
                parts.authority = Some(target.clone());
                *req.uri_mut() =
                    Uri::from_parts(parts).expect("oops, couldn't rebuild url from parts");

                async move {
                    let resp = client.request(req).await;

                    match method {
                        Method::GET => {
                            let mut resp = resp?;
                            let content_type = resp
                                .headers()
                                .get(hyper::header::CONTENT_TYPE)
                                .and_then(|h| h.to_str().ok());
                            match content_type {
                                // these are specifically what roon sends, and only those
                                // only filter on JS (websocket) and html (google fonts & such)
                                Some("application/x-javascript" | "text/html") => {}
                                _ => {
                                    // otherwise, pass it through
                                    return Ok(resp);
                                }
                            }
                            let body = &hyper::body::to_bytes(resp.body_mut()).await?[..];
                            let body = std::str::from_utf8(body)?;
                            let body = body
                                .replace("ws://", "wss://")
                                .replace("http://", "https://");

                            resp.headers_mut()
                                .insert(hyper::header::CONTENT_LENGTH, body.len().into());
                            *resp.body_mut() = body.into();

                            Ok::<_, Box<dyn Error + Send + Sync + 'static>>(resp)
                        }
                        _ => Ok::<_, Box<dyn Error + Send + Sync + 'static>>(resp?),
                    }
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    server.await?;
    Ok(())
}
