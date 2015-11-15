#![cfg_attr(test, feature(custom_derive, plugin))]
#![cfg_attr(test, plugin(serde_macros))]

#[cfg(feature = "test")] extern crate test;

extern crate serde;

#[cfg(feature = "msgpack_serialization")]
extern crate rmp_serde as msgpack;
#[cfg(feature = "json_serialization")]
extern crate serde_json;

include!(concat!(env!("OUT_DIR"), "/lib.rs"));
