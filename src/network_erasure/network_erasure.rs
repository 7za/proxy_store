use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use lazy_static::lazy_static;
use liberasurecode::ErasureCoder;
use std::num::NonZeroUsize;
use erasurecode::DataStorage;

lazy_static! {
    static ref ERASURE_CODER: ErasureCoder = {
        ErasureCoder::new(non_zero(2), non_zero(1)).unwrap()
    };
}

fn non_zero(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("Must be a non zero number")
}

pub struct ErasureNetwork {
    data : Arc<Mutex<DataStorage>>,
    sent: bool,
}

impl ErasureNetwork {
    pub fn new(data: DataStorage) -> Self {
        Self {
            data: Arc::new(Mutex::new(data)),
            sent: false,
        }
    }
}


impl Clone for ErasureNetwork {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            sent: self.sent,
        }
    }
}


impl Future for ErasureNetwork {
    type Output = Arc<Mutex<DataStorage>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.sent {
            self.sent = true;
            let data_copy = self.data.clone();
            let waker = cx.waker().clone();
            rayon::spawn(move || {
                data_copy.lock().unwrap().protect(&ERASURE_CODER);
                waker.wake();
            });
            return Poll::Pending;
        }
        Poll::Ready(self.data.clone())
    }
}
