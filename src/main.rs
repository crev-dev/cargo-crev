#![allow(unused)]

#[macro_use]
extern crate failure;
extern crate common_failures;
extern crate blake2;
extern crate chrono;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate ed25519_dalek;
extern crate hex;
extern crate base64;
extern crate rand;
extern crate miscreant;
extern crate argonautica;

mod proof;
mod id;
mod util;

fn main() {
    println!("Hello, world!");
}
