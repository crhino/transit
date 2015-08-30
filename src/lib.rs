//! Transit - A transport layer library for Rust
//!
//! Currently supporting only UDP, the idea is to create easy abstraction layers around transport
//! protocols. Do not think about sending byte arrays around or handing sockets/file descriptors.
#![feature(custom_derive, plugin)]

// TODO: Only used in tests, how to make it a dev-dependency?
#![plugin(serde_macros)]
extern crate bincode;
extern crate serde;

pub mod udp;
