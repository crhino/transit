#![feature(custom_derive, plugin)]

// TODO: Only used in tests, how to make it a dev-dependency?
#![plugin(serde_macros)]
extern crate bincode;
extern crate serde;

pub mod udp;
