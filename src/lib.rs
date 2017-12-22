//! Sending and receiving frames (arrays of bytes of varied length)
//! over streams of bytes.
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
//! `use_std`: Use standard library, enabled by default; disable for no_std.
#![deny(warnings)]
#![feature(conservative_impl_trait)]

#![cfg_attr(not(feature = "use_std"), no_std)]

extern crate cobs;
#[cfg(not(feature = "use_std"))]
extern crate core_io;

extern crate ref_slice;

#[cfg(feature = "use_std")]
pub mod channel;

pub mod error;
#[allow(unused_imports)]
use error::{Error, Result};

#[cfg(feature = "use_std")]
use ref_slice::ref_slice_mut;

#[cfg(feature = "use_std")]
use std::io::{self, Read, Write};

#[cfg(not(feature = "use_std"))]
use core_io::{Read, Write};

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

        #[cfg(feature = "use_std")] {
            self.w.write(&code)?;
        }

        #[cfg(not(feature = "use_std"))] {
            self.w.write(&code)
                .map_err(|_| Error::Io)?;
        }

        Ok(())
    }
}

/// Receives frames from an underlying `io::Read` instance.
///
/// TODO: Add a recv() variant suitable for no_std use, e.g. one that
/// takes a `&mut [u8]`.
pub struct Receiver<R: Read> {

    #[cfg_attr(not(feature = "use_std"), allow(dead_code))]
    /// The underlying reader
    r: R,
}

impl<R: Read> Receiver<R> {
    pub fn new(r: R) -> Receiver<R> {
        Receiver::<R> {
            r: r,
        }
    }

    #[cfg(feature = "use_std")]
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
                    return Err(Error::EofDuringFrame),
                Ok(0) =>
                    return Err(Error::EofDuringFrame),
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
             .map_err(|_| Error::CobsDecodeFailed)
    }
}

#[cfg(test)]
mod tests {
    use channel::Channel;
    use error::Error;
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
            Err(Error::EofDuringFrame) => (),
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
