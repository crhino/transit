//! Transit - A transport layer library for Rust
//!
//! Currently supporting only UDP, the idea is to create easy abstraction layers around transport
//! protocols. Do not think about sending byte arrays around or handing sockets/file descriptors.
#![cfg_attr(test, feature(custom_derive, plugin))]
#![cfg_attr(test, plugin(serde_macros))]

extern crate rmp_serde as msgpack;
extern crate serde;

pub mod udp;
