use std::marker::PhantomData;
use std::io::{self, ErrorKind};
use std::error::Error;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};

pub struct Transit<T> {
    socket: UdpSocket,
    packet_type: PhantomData<T>,
}

/// The From trait in the standard library does not allow for errors to occur. This is useful
/// particularly when converting a type from a byte array received from the network that may or may
/// not be of the particular type.
pub trait FromTransit<T> {
    fn from_transit(T) -> io::Result<Self> where Self: Sized;
}

impl<'a> FromTransit<&'a [u8]> for String {
    fn from_transit(buf: &'a [u8]) -> io::Result<String> {
        let vec = Vec::from(buf);
        let res = String::from_utf8(vec);
        match res {
            Err(utf8err) => Err(io::Error::new(ErrorKind::InvalidData, utf8err.description())),
            Ok(string) => Ok(string),
        }
    }
}

/// Like the FromTransit trait, the IntoTransit trait modifies the idea of the into trait.
/// Specifically, the IntoTransit trait takes a reference to self and specifies a lifetime. This
/// allows an implementor to keep using the object while tying the lifetime of the return type T to
/// the implementor's lifetime.
pub trait IntoTransit<'a, T> {
    fn into_transit(&'a self) -> T;
}

impl<'a> IntoTransit<'a, &'a [u8]> for String {
    fn into_transit(&'a self) -> &'a [u8] {
        self.as_bytes()
    }
}

impl<T: for<'a> FromTransit<&'a [u8]> + for<'a> IntoTransit<'a, &'a [u8]>> Transit<T> {
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
        assert!(n < 1024);
        let data = try!(T::from_transit(&buf[..n]));
        Ok((data, addr))
    }

    /// Transforms the packet into a byte array and sends it to the associated address.
    pub fn send_to<A>(&self, pkt: T, addr: A) -> io::Result<()> where A: ToSocketAddrs {
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

    impl<'a> IntoTransit<'a, &'a [u8]> for Test {
        fn into_transit(&'a self) -> &'a [u8] {
            unsafe { slice::from_raw_parts(&self.ten as *const u8, 1) }
        }
    }

    impl<'a> FromTransit<&'a [u8]> for Test {
        fn from_transit(buf: &'a [u8]) -> io::Result<Test> {
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

    impl<'a> IntoTransit<'a, &'a [u8]> for Another {
        fn into_transit(&'a self) -> &'a [u8] {
            unsafe { slice::from_raw_parts(&self.data as *const u8, 1) }
        }
    }


    impl<'a> FromTransit<&'a [u8]> for Another {
        fn from_transit(buf: &'a [u8]) -> io::Result<Another> {
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
    fn test_send_recv_string() {
        let addr1 = "127.0.0.1:63001";
        let addr2 = "127.0.0.1:63002";
        let transit1: Transit<String> = Transit::new(addr1).unwrap();
        let transit2 = Transit::new(addr2).unwrap();
        let test = String::from("hello");

        let res = transit2.send_to(test.clone(), addr1);
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
