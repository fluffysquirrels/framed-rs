//! Sending and receiving frames (slices of bytes of varied length).
//!
//! Conforming to / inspired by the [data link layer][dll] or layer 2
//! in the OSI networking model, this module enables sending slices of
//! bytes of definite length over an underlying transport that only
//! supports sending an unstructured stream of bytes (a [physical
//! layer][pl], such as ITM, UART or SPI).
//!
//! [dll]: https://en.wikipedia.org/wiki/Data_link_layer
//! [pl]: https://en.wikipedia.org/wiki/Physical_layer

// TODO: Make this work when no_std.

#![deny(warnings)]
#![feature(conservative_impl_trait)]

#[macro_use]
extern crate error_chain;

pub mod error;
use error::{Result};
use std::io::{Read, Write};

/// Sends frames over an underlying `io::Write` instance.
pub struct Sender<W: Write> {
    _w: W,
}

impl<W: Write> Sender<W> {
    pub fn new(w: W) -> Sender<W> {
        Sender::<W> {
            _w: w,
        }
    }

    pub fn send(&mut self, _f: &[u8]) -> Result<()> {
        unimplemented!();
    }
}

/// Receives frames from an underlying `io::Read` instance.
pub struct Receiver<R: Read> {
    _r: R,
}

impl<R: Read> Receiver<R> {
    pub fn new(r: R) -> Receiver<R> {
        Receiver::<R> {
            _r: r,
        }
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use super::{Receiver, Sender};

    #[test]
    fn one_frame() {
        let (mut tx, mut rx) = pair();
        let sent = [0x00, 0x01, 0x02];
        tx.send(&sent).unwrap();
        let recvd = rx.recv().unwrap();
        assert_eq!(recvd, sent);
    }

    fn pair() -> (Sender<impl Write>, Receiver<impl Read>) {
        let c = Channel::new();
        let tx = Sender::new(c.writer());
        let rx = Receiver::new(c.reader());
        (tx, rx)
    }

    use self::channel::Channel;

    mod channel {
        use std::io::{Read, Result, Write};
        use std::rc::Rc;

        /// A Vec-backed buffer that can be read from and written to.
        pub struct Channel {
            inner: Rc<Inner>,
        }

        struct Inner {
            _v: Vec<u8>,
            _read_pos: usize,
        }

        pub struct Reader {
            _inner: Rc<Inner>,
        }

        pub struct Writer {
            _inner: Rc<Inner>,
        }

        impl Channel {
            pub fn new() -> Channel {
                Channel {
                    inner: Rc::new(Inner {
                        _v: Vec::new(),
                        _read_pos: 0
                    })
                }
            }

            pub fn reader(&self) -> Reader {
                Reader {
                    _inner: self.inner.clone(),
                }
            }

            pub fn writer(&self) -> Writer {
                Writer {
                    _inner: self.inner.clone(),
                }
            }
        }

        impl Read for Reader {
            fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
                unimplemented!()
            }
        }

        impl Write for Writer {
            fn write(&mut self, _buf: &[u8]) -> Result<usize> {
                unimplemented!()
            }

            fn flush(&mut self) -> Result<()> {
                unimplemented!()
            }
        }
    }
}
