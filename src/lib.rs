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
//!
//! Currently the encoding is: the frame [COBS]-encoded
//! to remove bytes equal to zero, then a terminating zero byte.
//! [COBS]: https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing
//!
//! ## Cargo feature flags
//! `trace`: Enable to print all data to stdout for testing.
//!
//! ## TODO
//! * Start frame with a zero as well, then we detect partial frames.
//! * Add a length field to the beginning of the frame and a
//!   checksum to the end. Perhaps make them optional.
//! * Support no_std.

#![deny(warnings)]
#![feature(conservative_impl_trait)]

extern crate cobs;
#[macro_use]
extern crate error_chain;
extern crate ref_slice;

pub mod channel;

pub mod error;
use error::{Error, ErrorKind, Result};
use ref_slice::ref_slice_mut;
use std::io::{self, Read, Write};

const FRAME_END: u8 = 0;

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
        let mut code = cobs::encode_vec(f);
        code.push(FRAME_END);
        #[cfg(feature = "trace")] {
            println!("framed: Sending code = {:?}", code);
        }
        self.w.write(&code)?;
        Ok(())
    }
}

/// Receives frames from an underlying `io::Read` instance.
pub struct Receiver<R: Read> {
    /// The underlying reader
    r: R,
}

impl<R: Read> Receiver<R> {
    pub fn new(r: R) -> Receiver<R> {
        Receiver::<R> {
            r: r,
        }
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        let mut next_frame = Vec::new();

        let mut b = 0u8;
        loop {
            let res = self.r.read(ref_slice_mut(&mut b));
            #[cfg(feature = "trace")] {
                println!("framed: Read result = {:?}", res);
            }
            match res {
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof =>
                    return Err(Error::from(ErrorKind::EofDuringFrame)),
                Ok(0) =>
                    return Err(Error::from(ErrorKind::EofDuringFrame)),
                Err(e) => return Err(Error::from(e)),
                Ok(_) => (),
            };

            #[cfg(feature = "trace")] {
                println!("framed: Read byte = {}", b);
            }
            if b == FRAME_END {
                break;
            } else {
                next_frame.push(b);
            }
        }
        assert!(b == FRAME_END);

        cobs::decode_vec(&next_frame)
             .map_err(|_| Error::from(ErrorKind::CobsDecodeFailed))
    }
}

#[cfg(test)]
mod tests {
    use channel::Channel;
    use error::{Error, ErrorKind};
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

    #[test]
    fn two_frames_sequentially() {
        let (mut tx, mut rx) = pair();
        {
            let sent = [0x00, 0x01, 0x02];
            tx.send(&sent).unwrap();
            let recvd = rx.recv().unwrap();
            assert_eq!(recvd, sent);
        }

        {
            let sent = [0x10, 0x11, 0x12];
            tx.send(&sent).unwrap();
            let recvd = rx.recv().unwrap();
            assert_eq!(recvd, sent);
        }
    }

    #[test]
    fn two_frames_at_once() {
        let (mut tx, mut rx) = pair();
        let s1 = [0x00, 0x01, 0x02];
        let s2 = [0x10, 0x11, 0x12];

        tx.send(&s1).unwrap();
        tx.send(&s2).unwrap();

        let r1 = rx.recv().unwrap();
        let r2 = rx.recv().unwrap();
        println!("r1: {:?}\n\
                  r2: {:?}", r1, r2);

        assert_eq!(r1, s1);
        assert_eq!(r2, s2);
    }

    #[test]
    fn empty_input() {
        let (mut _tx, mut rx) = pair();
        match rx.recv() {
            Err(Error(ErrorKind::EofDuringFrame, _)) => (),
            e @ _ => panic!("Bad value: {:?}", e)
        }
    }

    fn pair() -> (Sender<impl Write>, Receiver<impl Read>) {
        let c = Channel::new();
        let tx = Sender::new(c.writer());
        let rx = Receiver::new(c.reader());
        (tx, rx)
    }
}
