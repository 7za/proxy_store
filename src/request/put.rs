use std::pin::Pin;
use std::sync::{Arc, Mutex};
use futures_util::task::{Poll, Context};
use bytes::Buf;
use lazy_static::lazy_static;
use std::time::Duration;
use erasurecode::DataStorage;
use hyper::body::HttpBody;
use hyper::client::HttpConnector;
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
use crate::header::HeaderMap;

lazy_static! {

    static ref CLIENT_REQ: hyper::Client<HttpConnector, BodyStream> = {
        let mut p = hyper::client::HttpConnector::new();
        p.set_keepalive(Some(Duration::from_secs(60)));
        p.set_send_buffer_size(Some(8*1024*1024));
        hyper::client::Client::builder()
                .pool_idle_timeout(Duration::from_secs(30))
                .http1_max_buf_size(32*1024)
                .http1_writev(true)
                .build(p)
    };
}

#[derive(Debug, Copy, Clone)]
struct BodyChunk {
    ptr : *const u8,
    len : usize,
    pos: usize,
}

impl BodyChunk {
    fn new(p : *const u8, len: usize) -> Self {
        Self {
            ptr : p,
            len,
            pos : 0,
        }
    }
}

unsafe impl std::marker::Send for BodyChunk {}

impl Buf for BodyChunk {
    fn remaining(&self) -> usize {
        self.len - self.pos
    }

    fn chunk(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.ptr.add(self.pos), self.len - self.pos)
        }
    }

    fn advance(&mut self, cnt: usize) {
        self.pos += cnt;
    }
}

struct BodyStream {
    src: BodyChunk,
    d: bool,
}

impl BodyStream {
    fn new(src: *const u8, len: usize) -> Self {
        Self {
            src: BodyChunk::new(src, len),
            d: false,
        }
    }
}

impl HttpBody for BodyStream {
    type Data = BodyChunk;
    type Error = std::io::Error;

    fn poll_data(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        if self.d {
            return Poll::Ready(None);
        }

        self.d = true;
        Poll::Ready(Some(Ok(self.src)))
    }

    fn poll_trailers(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
         Poll::Ready(Ok(None))
    }
}

pub  async fn put_n_split(ctx: Arc<Mutex<DataStorage>>,
                          key: &str) {
    let mut vector = Vec::new();
    {
        let rc_storage = ctx.clone();
        let storage = &*rc_storage.lock().unwrap();

        let data_len = storage.len();
        storage.into_iter().enumerate().for_each(|(i, b)| {
            let addr = format!("http://localhost:{}/store/{}", 11111 + i, key);
            let hdr = format!("application/x-scality-storage-data; data={}", data_len);
            let req = hyper::Request::builder()
                .method(hyper::Method::PUT)
                .uri(&addr)
                .header(CONTENT_TYPE, &hdr)
                .header(CONTENT_LENGTH, data_len)
                .body(BodyStream::new(*b, data_len))
                .expect("request builder");

            let n = CLIENT_REQ.request(req);
            vector.push(n);

        });
    }
    let _ret = futures::future::join_all(vector).await;
}
