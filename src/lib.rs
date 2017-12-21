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

pub mod channel;

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
    use channel::Channel;
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
}
