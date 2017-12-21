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

extern crate cobs;
#[macro_use]
extern crate error_chain;

pub mod channel;

pub mod error;
use error::{Error, ErrorKind, Result};
use std::io::{Read, Write};

/// Sends frames over an underlying `io::Write` instance.
pub struct Sender<W: Write> {
    w: W,
}

impl<W: Write> Sender<W> {
    pub fn new(w: W) -> Sender<W> {
        Sender::<W> {
            w: w,
        }
    }

    pub fn send(&mut self, f: &[u8]) -> Result<()> {
        let code = cobs::encode_vec(f);
        self.w.write(&code)?;
        Ok(())
    }
}

/// Receives frames from an underlying `io::Read` instance.
pub struct Receiver<R: Read> {
    r: R,
}

impl<R: Read> Receiver<R> {
    pub fn new(r: R) -> Receiver<R> {
        Receiver::<R> {
            r: r,
        }
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        let mut code = Vec::new();
        self.r.read_to_end(&mut code)?;
        cobs::decode_vec(&code)
             .map_err(|_| Error::from(ErrorKind::CobsDecodeFailed))
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

    // TODO: Test sending 2 different frames.

    // TODO: Test buffering:
    // * Write half a frame
    // * recv() returns nothing
    // * Write the rest of the frame
    // * recv() returns the whole frame

    fn pair() -> (Sender<impl Write>, Receiver<impl Read>) {
        let c = Channel::new();
        let tx = Sender::new(c.writer());
        let rx = Receiver::new(c.reader());
        (tx, rx)
    }
}
