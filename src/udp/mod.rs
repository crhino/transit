use std::marker::PhantomData;
use std::io::{self};
use std::error::Error;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use std::fmt;

use bincode::serde::{DeserializeError, SerializeError, serialize, deserialize};
use bincode;
use serde::{Serialize, Deserialize};

const MAX_UDP_SIZE: u16 = 65535;
pub struct Transit<T> {
    socket: UdpSocket,
    packet_type: PhantomData<T>,
}

pub type UnderlyingError = Box<Error + Send + Sync>;
#[derive(Debug)]
pub enum TransitError {
    IoError(io::Error),
    SerializeError(UnderlyingError),
    DeserializeError(UnderlyingError),
}

impl Error for TransitError {
    fn description(&self) -> &str {
        match *self {
            TransitError::IoError(ref err) => err.description(),
            TransitError::SerializeError(ref err) => err.description(),
            TransitError::DeserializeError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            TransitError::IoError(ref err) => err.cause(),
            TransitError::SerializeError(ref err) => err.cause(),
            TransitError::DeserializeError(ref err) => err.cause(),
        }
    }
}

impl From<io::Error> for TransitError {
    fn from(err: io::Error) -> TransitError {
        TransitError::IoError(err)
    }
}

impl From<DeserializeError> for TransitError {
    fn from(err: DeserializeError) -> TransitError {
        TransitError::DeserializeError(Box::new(err))
    }
}

impl From<SerializeError> for TransitError {
    fn from(err: SerializeError) -> TransitError {
        TransitError::SerializeError(Box::new(err))
    }
}

impl fmt::Display for TransitError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TransitError::IoError(ref err) =>
                write!(fmt, "IoError: {}", err),
            TransitError::DeserializeError(ref err) =>
                write!(fmt, "DeserializeError: {}", err),
            TransitError::SerializeError(ref err) =>
                write!(fmt, "SerializeError: {}", err),
        }
    }
}

/// Sends and receives types over UDP, removing any knowledge of buffers and dealing with the std
/// library.
///
/// This use the `bincode` crate to serialize objects. Does not currently support securely sending
/// packets over the network or ensuring that only packets of the correct type are serialized.
///
/// # Examples
///
/// ```rust
/// use std::io;
/// use transit::udp::*;
///
/// let transit = Transit::new("localhost:65000").unwrap();
/// let transit2 = Transit::new("localhost:65001").unwrap();
/// let test = String::from("hello, rust");
///
/// let res = transit.send_to(&test, "localhost:65001");
/// assert!(res.is_ok());
/// let res: Result<(String, _), TransitError> = transit2.recv_from();
/// assert!(res.is_ok());
/// let (data, _addr) = res.unwrap();
/// assert_eq!(data, "hello, rust");
/// ```
impl<T> Transit<T> {
    pub fn new<A>(addr: A) -> Result<Transit<T>, TransitError> where A: ToSocketAddrs {
        let socket = try!(UdpSocket::bind(addr));
        Ok(Transit {
            socket: socket,
            packet_type: PhantomData,
        })
    }

    /// On success, this function returns the type deserialized using the Deserialize trait
    /// implementation. It is not defined what happens when Transit trys to deserialize a different
    /// type into another currently.
    pub fn recv_from(&self) -> Result<(T, SocketAddr), TransitError> where T: Deserialize {
        let mut buf = [0; MAX_UDP_SIZE as usize];
        let (n, addr) = try!(self.socket.recv_from(&mut buf));
        let data = try!(deserialize(&buf[..n]));
        Ok((data, addr))
    }

    /// Transforms the packet into a byte array and sends it to the associated address.
    pub fn send_to<A>(&self, pkt: &T, addr: A) -> Result<(), TransitError> where T: Serialize, A: ToSocketAddrs {
        let sizelimit = bincode::SizeLimit::Bounded(MAX_UDP_SIZE as u64);
        let vec = try!(serialize(pkt, sizelimit));
        try!(self.socket.send_to(&vec[..], addr));
        Ok(())
    }

    pub fn local_addr(&self)  -> Result<SocketAddr, TransitError> {
        let addr = try!(self.socket.local_addr());
        Ok(addr)
    }
}

#[cfg(test)]
mod test {
    use udp::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
    struct Test {
        ten: u8,
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
    struct Another {
        data: String,
    }

    #[test]
    fn test_send_recv() {
        let addr1 = "127.0.0.1:61001";
        let addr2 = "127.0.0.1:61002";
        let transit1: Transit<Test> = Transit::new(addr1).unwrap();
        let transit2 = Transit::new(addr2).unwrap();
        let test = Test { ten: 10 };

        let res = transit2.send_to(&test, addr1);
        assert!(res.is_ok());
        let res = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr) = res.unwrap();
        assert_eq!(data, test);
    }

    #[test]
    fn test_send_recv_string() {
        let addr1 = "127.0.0.1:63001";
        let addr2 = "127.0.0.1:63002";
        let transit1: Transit<String> = Transit::new(addr1).unwrap();
        let transit2 = Transit::new(addr2).unwrap();
        let test = String::from("hello");

        let res = transit2.send_to(&test, addr1);
        assert!(res.is_ok());
        let res = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr) = res.unwrap();
        assert_eq!(data, test);
    }

    #[test]
    fn test_send_recv_bytes() {
        let addr1 = "127.0.0.1:64001";
        let addr2 = "127.0.0.1:64002";
        let transit1 = Transit::new(addr1).unwrap();
        let transit2 = Transit::new(addr2).unwrap();
        let vec = vec!(9u8);
        let slice = &vec[..];

        let res = transit2.send_to(&slice, addr1);
        assert!(res.is_ok());
        let res: Result<(Vec<u8>, _), TransitError> = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr) = res.unwrap();
        assert_eq!(data, vec);
    }

    // TODO: How to ensure different types are not deserialized as each other with bincode?
    // #[test]
    // fn test_packet_type() {
    //     let addr1 = "127.0.0.1:62001";
    //     let addr2 = "127.0.0.1:62002";
    //     let transit1: Transit<Another> = Transit::new(addr1).unwrap();
    //     let transit2: Transit<Test> = Transit::new(addr2).unwrap();
    //     let test = Another { data: String::from("Hello") };

    //     let res = transit1.send_to(&test, addr2);
    //     assert!(res.is_ok());
    //     let res = transit2.recv_from();
    //     assert!(res.is_err());
    // }
}
