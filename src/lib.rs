//! Transit - A transport layer library for Rust
//!
//! Currently supporting only UDP, the idea is to create easy abstraction layers around transport
//! protocols. Do not think about sending byte arrays around or handing sockets/file descriptors.
//!
//! For serialization, Transit depends on the `serde` framework. Both JSON encoding and Msgpack
//! encoding are supported through the use of features. Make sure to compile transit with either
//! the `json_serialization` or the `msgpack_serialization` feature.
#![cfg_attr(test, feature(custom_derive, plugin))]
#![cfg_attr(test, plugin(serde_macros))]

extern crate serde;

#[cfg(feature = "msgpack_serialization")]
extern crate rmp_serde as msgpack;
#[cfg(feature = "json_serialization")]
extern crate serde_json;

pub mod udp;
