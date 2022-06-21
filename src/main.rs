#![feature(test)]
mod network_erasure;
mod network_split;
mod request;
mod utils;

extern crate core;


use futures_util::stream::*;

use hyper::service::service_fn;
use hyper::{header, Body, Method, Request, Response, StatusCode};
use hyper::server::conn::Http;
use tokio::net::TcpListener;

use crate::network_erasure::network_erasure::ErasureNetwork;
use crate::network_split::network_split::ChunkStream;
use crate::request::put::put_n_split;
use crate::utils::key;


extern crate jemallocator;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn content_length_get(req: &Request<Body>) -> usize {
    match req.headers().get(header::CONTENT_LENGTH) {
        None => 0,
        Some(&ref val) => val.to_str().unwrap_or("0").parse::<usize>().unwrap_or(0),
    }
}

async fn handle_request(req: Request<Body>) -> core::result::Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/store/") => {
            //let now = Instant::now();
            let content_length = content_length_get(&req);
            let tcp_splitter =
                ChunkStream::new(req.into_body(), 2, 1, 32768, content_length);

            let mut id = 0_u32;
            let key = key::new_key();

            tcp_splitter
                .then(ErasureNetwork::new)
                .for_each(|v|{
                    let key = key::key_inc(&key, id);
                    id += 1;
                    async move {
                        put_n_split(v, &key::key_str(&key)).await;
                    }
                }).await;

            Ok(Response::default())
        }

        // Return the 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

use std::{net::SocketAddr};

#[tokio::main(flavor = "multi_thread", worker_threads = 120)]
async fn main() -> core::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {

        let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

        let tcp_listener = TcpListener::bind(addr).await?;
        loop {
            let (tcp_stream, _) = tcp_listener.accept().await?;
            tcp_stream.set_nodelay(true).unwrap();

            tokio::task::spawn(async move {
                if let Err(http_err) = Http::new()
                    .http1_keep_alive(true)
                    .http1_only(true)
                    .max_buf_size(64*1024)
                    .serve_connection(tcp_stream, service_fn(handle_request))
                    .await {
                    eprintln!("Error while serving HTTP connection: {}", http_err);
                }
            });
        }
}
