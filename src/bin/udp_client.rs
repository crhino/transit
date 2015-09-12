#![cfg(feature = "udp_client")]
#![feature(plugin)]
#![feature(result_expect)]
#![feature(convert)]
#![plugin(docopt_macros)]

extern crate rustc_serialize;
extern crate docopt;

use std::net::UdpSocket;

// Write the Docopt usage string with the `docopt!` macro.
docopt!(Args, "
Usage: udpc [-s] <address>

       Output any received packets as a byte array.

Options:
       -s, --string  Output data as a string.
");

fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    let mut buf = [0; 65536];
    let socket = UdpSocket::bind(args.arg_address.as_str()).expect("Could not bind socket");
    loop {
        let (amt, _src) = socket.recv_from(&mut buf).expect("Could not receive packet");
        let pkt = &buf[..amt];
        if args.flag_string {
            println!("{}", String::from_utf8(
                    pkt.iter().map(|x| *x).collect::<Vec<u8>>()
                    ).unwrap());
        } else {
            println!("{}", pkt);
        }
    }
}
