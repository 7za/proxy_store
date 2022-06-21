use std::ops::Add;
use num_bigint::{BigUint, RandBigInt};
use rand::prelude::ThreadRng;

pub fn new_key() -> BigUint {
    let mut rng: ThreadRng = rand::thread_rng();
    rng.gen_biguint(128)
}

pub fn key_inc(key: &BigUint, val: u32) -> BigUint {
    key.add(val as u32)
}

pub fn key_str(key: &BigUint) -> String {
    key.to_string()
}