use std::marker::PhantomData;
use std::io::{self, ErrorKind};
use std::error::Error;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};

pub struct Transit<T> {
    socket: UdpSocket,
    packet_type: PhantomData<T>,
}

/// Trait implemented by types that can be read from the network.
///
/// # Warnings
///
/// A type should always make sure to check whether the passed in buffer is a legitimate packet
/// sent from a known application. This will be called on any UDP packet received on the socket.
pub trait FromTransit {
    fn from_transit(&[u8]) -> io::Result<Self> where Self: Sized;
}

impl FromTransit for String {
    fn from_transit(buf: &[u8]) -> io::Result<String> {
        let vec = Vec::from(buf);
        let res = String::from_utf8(vec);
        match res {
            Err(utf8err) => Err(io::Error::new(ErrorKind::InvalidData, utf8err.description())),
            Ok(string) => Ok(string),
        }
    }
}

impl FromTransit for Vec<u8> {
    fn from_transit(buf: &[u8]) -> io::Result<Vec<u8>> {
        Ok(buf.iter().map(|x| *x).collect())
    }
}

/// Trait implemented by types that can be written to the network.
pub trait IntoTransit {
    fn into_transit(&self) -> &[u8];
}

impl IntoTransit for String {
    fn into_transit(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'a> IntoTransit for &'a [u8] {
    fn into_transit(&self) -> &[u8] {
        *self
    }
}

/// Sends and receives types over UDP, removing any knowledge of buffers and dealing with the std
/// library.
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
/// let res: io::Result<(String, _)> = transit2.recv_from();
/// assert!(res.is_ok());
/// let (data, _addr) = res.unwrap();
/// assert_eq!(data, "hello, rust");
/// ```
impl<T> Transit<T> {
    pub fn new<A>(addr: A) -> io::Result<Transit<T>> where A: ToSocketAddrs {
        let socket = try!(UdpSocket::bind(addr));
        Ok(Transit {
            socket: socket,
            packet_type: PhantomData,
        })
    }

    /// On success, this function returns the type deserialized using the FromTransit trait
    /// implementation. The trait implementation should be able to detect whether or not the buffer
    /// contains a valid UDP message and emit an error appropriately.
    pub fn recv_from(&self) -> io::Result<(T, SocketAddr)> where T: FromTransit {
        let mut buf = [0; 1024];
        let (n, addr) = try!(self.socket.recv_from(&mut buf));
        assert!(n < 1024);
        let data = try!(T::from_transit(&buf[..n]));
        Ok((data, addr))
    }

    /// Transforms the packet into a byte array and sends it to the associated address.
    pub fn send_to<A>(&self, pkt: &T, addr: A) -> io::Result<()> where T: IntoTransit, A: ToSocketAddrs {
        let buf = pkt.into_transit();
        try!(self.socket.send_to(buf, addr));
        Ok(())
    }

    pub fn local_addr(&self)  -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
}

#[cfg(test)]
mod test {
    use udp::*;
    use std::slice;
    use std::io::{self, Error, ErrorKind};

    #[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
    struct Test {
        ten: u8,
    }

    impl IntoTransit for Test {
        fn into_transit(&self) -> &[u8] {
            unsafe { slice::from_raw_parts(&self.ten as *const u8, 1) }
        }
    }

    impl FromTransit for Test {
        fn from_transit(buf: &[u8]) -> io::Result<Test> {
            if buf[0] != 10 {
                Err(Error::new(ErrorKind::InvalidData, "failed to serialize"))
            } else {
                Ok(Test { ten: buf[0] })
            }
        }
    }

    #[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
    struct Another {
        data: u8,
    }

    impl IntoTransit for Another {
        fn into_transit(&self) -> &[u8] {
            unsafe { slice::from_raw_parts(&self.data as *const u8, 1) }
        }
    }

    impl FromTransit for Another {
        fn from_transit(buf: &[u8]) -> io::Result<Another> {
            Ok(Another { data: buf[0] })
        }
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
        let res: io::Result<(Vec<u8>, _)> = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr) = res.unwrap();
        assert_eq!(data, vec);
    }

    #[test]
    fn test_packet_type() {
        let addr1 = "127.0.0.1:62001";
        let addr2 = "127.0.0.1:62002";
        let transit1: Transit<Another> = Transit::new(addr1).unwrap();
        let transit2: Transit<Test> = Transit::new(addr2).unwrap();
        let test = Another { data: 27 };

        let res = transit1.send_to(&test, addr2);
        assert!(res.is_ok());
        let res = transit2.recv_from();
        assert!(res.is_err());
    }
}
