use std::io::{self, Write, Read};
use std::error::Error;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use std::fmt;

use serde::{Serialize, Deserialize};

#[cfg(feature = "msgpack_serialization")]
use msgpack::{Serializer, Deserializer};
#[cfg(feature = "msgpack_serialization")]
use msgpack::decode::Error as DeserializeError;
#[cfg(feature = "msgpack_serialization")]
use msgpack::encode::Error as SerializeError;

#[cfg(feature = "json_serialization")]
use serde_json;

const MAX_UDP_SIZE: u16 = 65535;
pub struct Transit {
    socket: UdpSocket,
    buffer: Box<[u8]>,
}

pub type UnderlyingError = Box<Error + Send + Sync>;
#[derive(Debug)]
pub enum TransitError {
    IoError(io::Error),
    SerializeError(UnderlyingError),
    DeserializeError(UnderlyingError),
    Error(UnderlyingError),
}

impl Error for TransitError {
    fn description(&self) -> &str {
        match *self {
            TransitError::IoError(ref err) => err.description(),
            TransitError::SerializeError(ref err) => err.description(),
            TransitError::DeserializeError(ref err) => err.description(),
            TransitError::Error(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            TransitError::IoError(ref err) => err.cause(),
            TransitError::SerializeError(ref err) => err.cause(),
            TransitError::DeserializeError(ref err) => err.cause(),
            TransitError::Error(ref err) => err.cause(),
        }
    }
}

impl From<io::Error> for TransitError {
    fn from(err: io::Error) -> TransitError {
        TransitError::IoError(err)
    }
}

#[cfg(feature = "msgpack_serialization")]
impl From<DeserializeError> for TransitError {
    fn from(err: DeserializeError) -> TransitError {
        TransitError::DeserializeError(Box::new(err))
    }
}

#[cfg(feature = "msgpack_serialization")]
impl From<SerializeError> for TransitError {
    fn from(err: SerializeError) -> TransitError {
        TransitError::SerializeError(Box::new(err))
    }
}

#[cfg(feature = "json_serialization")]
impl From<serde_json::Error> for TransitError {
    fn from(err: serde_json::Error) -> TransitError {
        TransitError::Error(Box::new(err))
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
            TransitError::Error(ref err) =>
                write!(fmt, "Error: {}", err),
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
/// let mut transit = Transit::new("localhost:65000").unwrap();
/// let mut transit2 = Transit::new("localhost:65001").unwrap();
/// let test = String::from("hello, rust");
///
/// let res = transit.send_to(&test, "localhost:65001");
/// assert!(res.is_ok());
/// let res: Result<(String, _), TransitError> = transit2.recv_from();
/// assert!(res.is_ok());
/// let (data, _addr) = res.unwrap();
/// assert_eq!(data, "hello, rust");
/// ```
impl Transit {
    pub fn new<A>(addr: A) -> Result<Transit, TransitError> where A: ToSocketAddrs {
        let socket = try!(UdpSocket::bind(addr));
        Ok(Transit {
            socket: socket,
            buffer: udp_buffer(),
        })
    }

    /// On success, this function returns the type deserialized using the Deserialize trait
    /// implementation. It is not defined what happens when Transit trys to deserialize a different
    /// type into another currently.
    pub fn recv_from<T>(&mut self) -> Result<(T, SocketAddr), TransitError> where T: Deserialize {
        let (n, addr) = try!(self.socket.recv_from(&mut self.buffer));
        let data = try!(deserialize(&self.buffer[..n]));
        Ok((data, addr))
    }

    /// Transforms the packet into a byte array and sends it to the associated address.
    pub fn send_to<T, A>(&mut self, pkt: &T, addr: A) -> Result<(), TransitError> where T: Serialize, A: ToSocketAddrs {
        let n = {
            let bytes = &mut self.buffer[..];
            let mut buf = ByteCounter::new(bytes);
            try!(serialize(&mut buf, pkt));
            buf.write_count()
        };
        try!(self.socket.send_to(&self.buffer[..n], addr));
        Ok(())
    }

    pub fn local_addr(&self)  -> Result<SocketAddr, TransitError> {
        let addr = try!(self.socket.local_addr());
        Ok(addr)
    }
}

fn udp_buffer() -> Box<[u8]> {
    (0..MAX_UDP_SIZE as usize)
        .map(|_x| 0u8)
        .collect::<Vec<u8>>()
        .into_boxed_slice()
}

#[cfg(feature = "msgpack_serialization")]
fn serialize<W, T>(mut buf: W, val: &T) -> Result<(), TransitError> where W: Write, T: Serialize {
    try!(val.serialize(&mut Serializer::new(&mut buf)));
    Ok(())
}

#[cfg(feature = "json_serialization")]
fn serialize<W, T>(mut buf: W, val: &T) -> Result<(), TransitError> where W: Write, T: Serialize {
    try!(serde_json::to_writer(&mut buf, &val));
    Ok(())
}

#[cfg(not(any(feature = "json_serialization", feature = "msgpack_serialization")))]
fn serialize<W, T>(mut _buf: W, _val: &T) -> Result<(), TransitError> where W: Write, T: Serialize {
    panic!("Need either json or msgpack feature")
}

#[cfg(feature = "msgpack_serialization")]
fn deserialize<R, T>(buf: R) -> Result<T, TransitError> where R: Read, T: Deserialize {
    let data = try!(Deserialize::deserialize(&mut Deserializer::new(buf)));
    Ok(data)
}

#[cfg(feature = "json_serialization")]
fn deserialize<R, T>(buf: R) -> Result<T, TransitError> where R: Read, T: Deserialize {
    let data = try!(serde_json::de::from_reader(buf));
    Ok(data)
}

#[cfg(not(any(feature = "json_serialization", feature = "msgpack_serialization")))]
fn deserialize<R, T>(_buf: R) -> Result<T, TransitError> where R: Read, T: Deserialize {
    panic!("Need either json or msgpack feature")
}

struct ByteCounter<W> {
    counter: usize,
    writer: W,
}

impl<W> ByteCounter<W> {
    fn new(writer: W) -> ByteCounter<W> {
        ByteCounter {
            counter: 0,
            writer: writer,
        }
    }

    fn write_count(&self) -> usize {
        self.counter
    }
}

impl<W: Write> Write for ByteCounter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = try!(self.writer.write(buf));
        self.counter += n;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod test {
    use test::Bencher;
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
        let addr1 = "127.0.0.1:0";
        let addr2 = "127.0.0.1:0";
        let mut transit1 = Transit::new(addr1).unwrap();
        let mut transit2 = Transit::new(addr2).unwrap();
        let test = Test { ten: 10 };

        let res = transit2.send_to(&test, transit1.local_addr().unwrap());
        assert!(res.is_ok());
        let res = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr): (Test, _) = res.unwrap();
        assert_eq!(data, test);
    }

    #[test]
    fn test_send_recv_string() {
        let mut transit1 = Transit::new("127.0.0.1:0").unwrap();
        let mut transit2 = Transit::new("127.0.0.1:0").unwrap();
        let test = String::from("hello");

        let res = transit2.send_to(&test, transit1.local_addr().unwrap());
        assert!(res.is_ok());
        let res = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr): (String, _) = res.unwrap();
        assert_eq!(data, test);
    }

    #[test]
    fn test_send_recv_bytes() {
        let mut transit1 = Transit::new("127.0.0.1:0").unwrap();
        let mut transit2 = Transit::new("127.0.0.1:0").unwrap();
        let vec = vec!(9u8);
        let slice = &vec[..];
        let addr1 = transit1.local_addr().unwrap();

        let res = transit2.send_to(&slice, addr1);
        assert!(res.is_ok());
        let res = transit2.send_to(&slice, addr1);
        assert!(res.is_ok());

        let res: Result<(Vec<u8>, _), TransitError> = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr) = res.unwrap();
        assert_eq!(data, vec);
        let res: Result<(Vec<u8>, _), TransitError> = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr) = res.unwrap();
        assert_eq!(data, vec);
    }

    #[test]
    fn test_packet_type() {
        let addr1 = "127.0.0.1:0";
        let addr2 = "127.0.0.1:0";
        let mut transit1 = Transit::new(addr1).unwrap();
        let mut transit2 = Transit::new(addr2).unwrap();
        let test = Another { data: String::from("Hello") };

        let res = transit1.send_to(&test, transit2.local_addr().unwrap());
        assert!(res.is_ok());
        let res: Result<(Test, _), TransitError> = transit2.recv_from();
        assert!(res.is_err());
    }

    // FIXME: rmp-serde does not current support enums, see issue #42
    #[cfg(feature = "json_serialization")]
    #[test]
    fn test_enum() {
        #[derive(Serialize, Deserialize, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
        enum Custom {
            First,
            Second(String),
        }
        let addr1 = "127.0.0.1:0";
        let addr2 = "127.0.0.1:0";
        let mut transit1 = Transit::new(addr1).unwrap();
        let mut transit2 = Transit::new(addr2).unwrap();
        let test = Custom::Second(String::from("Hello"));

        let res = transit1.send_to(&test, transit2.local_addr().unwrap());
        assert!(res.is_ok());
        let res: Result<(Custom, _), TransitError> = transit2.recv_from();
        let (data, _addr) = res.unwrap();
        assert_eq!(data, test);
    }

    #[bench]
    fn bench_send_to(b: &mut Bencher) {
        #[derive(Serialize, Deserialize, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
        struct Custom {
            integer: isize,
            string: String,
        }
        let mut transit1 = Transit::new("127.0.0.1:0").unwrap();

        let test = Custom { integer: 123456, string: String::from("Hello world.") };

        b.iter(|| {
            let _r = transit1.send_to(&test, "127.0.0.1:60000");
        });
    }
}
