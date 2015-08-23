use std::marker::PhantomData;
use std::io;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};

pub struct Transit<T> {
    socket: UdpSocket,
    packet_type: PhantomData<T>,
}

/// The From trait in the standard library does not allow for errors to occur. This is useful
/// particularly when converting a type from a byte array received from the network that may or may
/// not be of the particular type.
pub trait FromTransit<T> {
    fn from(T) -> io::Result<Self>;
}

impl<T: for<'a> FromTransit<&'a [u8]> + Into<Vec<u8>>> Transit<T> {
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
    pub fn recv_from(&self) -> io::Result<(T, SocketAddr)> {
        let mut buf = [0; 1024];
        let (n, addr) = try!(self.socket.recv_from(&mut buf));
        assert!(n <= 1024);
        let data = try!(T::from(&buf));
        Ok((data, addr))
    }

    /// Transforms the packet into a byte array and sends it to the associated address.
    pub fn send_to<A>(&self, pkt: T, addr: A) -> io::Result<()> where A: ToSocketAddrs {
        let buf = pkt.into();
        try!(self.socket.send_to(buf.as_slice(), addr));
        Ok(())
    }

    pub fn local_addr(&self)  -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
}

#[cfg(test)]
mod test {
    use udp::*;
    use std::io::{self, Error, ErrorKind};

    #[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
    struct Test {
        ten: u8,
    }

    impl Into<Vec<u8>> for Test {
        fn into(self) -> Vec<u8> {
            let mut vec  = Vec::new();
            vec.push(self.ten);
            vec
        }
    }

    impl<'a> FromTransit<&'a [u8]> for Test {
        fn from(buf: &'a [u8]) -> io::Result<Test> {
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

    impl Into<Vec<u8>> for Another {
        fn into(self) -> Vec<u8> {
            let mut vec  = Vec::new();
            vec.push(self.data);
            vec
        }
    }

    impl<'a> FromTransit<&'a [u8]> for Another {
        fn from(buf: &'a [u8]) -> io::Result<Another> {
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

        let res = transit2.send_to(test, addr1);
        assert!(res.is_ok());
        let res = transit1.recv_from();
        assert!(res.is_ok());
        let (data, _addr) = res.unwrap();
        assert_eq!(data, test);
    }

    #[test]
    fn test_packet_type() {
        let addr1 = "127.0.0.1:62001";
        let addr2 = "127.0.0.1:62002";
        let transit1: Transit<Another> = Transit::new(addr1).unwrap();
        let transit2: Transit<Test> = Transit::new(addr2).unwrap();
        let test = Another { data: 27 };

        let res = transit1.send_to(test, addr2);
        assert!(res.is_ok());
        let res = transit2.recv_from();
        assert!(res.is_err());
    }
}
