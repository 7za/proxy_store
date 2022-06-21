use std::pin::Pin;
use bytes::{Buf, Bytes};
use futures_util::{ready, Stream};
use futures_util::task::{Context, Poll};
use lazy_static::lazy_static;
use erasurecode::{PoolRow, DataStorage, DataStorageVec};
use std::sync::Mutex;


static SPLIT_SIZE: usize = 1024*1024;
static POOL_LEN: usize =  256;

lazy_static! {
    static ref POOLBYTES: Mutex<PoolRow> =
         Mutex::new(PoolRow::new(POOL_LEN, SPLIT_SIZE));
}


pub struct ChunkStream<T>
where
    T: Stream<Item = Result<Bytes, hyper::Error>> + Send + Unpin,
{
    ck: T,
    curr: Box<DataStorageVec>,
    remaining: usize,
}

impl<T> ChunkStream<T>
where
    T: Stream<Item = Result<Bytes, hyper::Error>> + Send + Unpin,
{
    pub fn new(p: T, k: usize, n: usize,
               sub_chunk: usize, tot_len: usize) -> Self {
        let mut pool = POOLBYTES.lock().unwrap();
        ChunkStream {
            ck: p,
            curr: Box::new(DataStorageVec::new(&mut pool, k, n, sub_chunk)),
            remaining: tot_len,
        }
    }
}

impl<T> Stream for ChunkStream<T>
where
    T: Stream<Item = Result<Bytes, hyper::Error>> + Send + Unpin,
{
    type Item = DataStorage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let pres = { Pin::new(&mut self.as_mut().ck).poll_next(cx) };

        return match ready!(pres) {
            None => {
                Poll::Ready(self.curr.get_data())
            }
            Some(chunk) => match chunk {
                Ok(chunk) => {
                    self.remaining -= chunk.len();
                    self.curr.put_data(chunk.chunk());
                    if self.remaining == 0 {
                        Poll::Ready(self.curr.get_data())
                    } else if let Some(p) = self.curr.get_data_complete() {
                        Poll::Ready(Some(p))
                    } else {
                        Poll::Pending
                    }
                }
                Err(_) => Poll::Pending,
            },
        };
    }
}
