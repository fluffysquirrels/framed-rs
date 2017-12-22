//! Send and receive slices of bytes over lossy streams of bytes.
//!
//! Conforming to / inspired by the [data link layer][dll] or layer 2
//! in the OSI networking model, this module enables sending slices of
//! bytes of definite length over an underlying lossy transport that
//! only supports sending an unstructured stream of bytes (a [physical
//! layer][pl], such as ITM, UART or SPI).
//!
//! [dll]: https://en.wikipedia.org/wiki/Data_link_layer
//! [pl]: https://en.wikipedia.org/wiki/Physical_layer
//!
//! The transport may corrupt the stream by dropping or modifying some
//! bytes en route. When the transport returns corrupt data the
//! decoder may return errors or corrupted payloads, but if the
//! transport starts operating without losses again the decoder should
//! return new uncorrupted frames.
//!
//! ## Encoding
//!
//! Currently the encoding is:
//! * The payload [COBS]-encoded to remove bytes equal to zero
//! * A terminating zero byte.
//! [COBS]: https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing
//!
//! The encoding is not stable at the moment, i.e. it can and will
//! change between minor versions. Consequently encoded data from this
//! crate is unsuitable for long-term storage or transmission between
//! different versions of an application. The API should be kept
//! stable between versions where possible and the crate version will
//! follow Rust semver rules on API changes.
//!
//! ## Cargo feature flags
//! `trace`: Enable to print all data to stdout for testing.
//!
//! `use_std`: Use standard library, enabled by default; disable for no_std.
#![deny(warnings)]
#![feature(conservative_impl_trait)]

#![cfg_attr(not(feature = "use_std"), no_std)]

extern crate cobs;

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

/// TODO
pub struct Payload(pub [u8]);

/// TODO
pub struct Encoded(pub [u8]);

const END_SYMBOL: u8 = 0;

const HEADER_LEN: usize = 0;

const FOOTER_LEN: usize = 1;

/// TODO
pub fn to_slice(p: &Payload, dest: &mut [u8]) -> Result<usize> {
    // Panic if code won't fit in `dest` because this is a programmer error.
    assert!(max_encoded_len(p.0.len())? <= dest.len());

    let cobs_len = cobs::encode(&p.0, &mut dest[HEADER_LEN..]);
    let footer_idx = HEADER_LEN + cobs_len;
    dest[footer_idx] = END_SYMBOL;

    #[cfg(feature = "trace")] {
        println!("framed: Frame code = {:?}", dest[0..(footer_idx + 1)]);
    }
    Ok(cobs_len + HEADER_LEN + FOOTER_LEN)
}

/// TODO
#[cfg(feature = "use_std")]
pub fn to_box(_p: &Payload) -> Result<Box<Encoded>> {
    unimplemented!()
}

/// TODO
#[cfg(feature = "use_std")]
pub fn to_writer<W: Write>(_p: &Payload, _w: W) -> Result<usize> {
    unimplemented!()
}

/// TODO
pub fn from_slice_to_slice(_src: &[u8], _dst: &mut [u8]) -> Result<usize> {
    unimplemented!()
}

/// TODO
#[cfg(feature = "use_std")]
pub fn from_slice_to_box(_src: &Encoded) -> Result<Box<Payload>> {
    unimplemented!()
}

/// TODO
#[cfg(feature = "use_std")]
pub fn from_reader<R: Read>(_r: &Read) -> Result<Box<Payload>> {
    unimplemented!()
}

/// Returns the maximum possible decoded length given a frame with
/// the encoded length supplied.
pub fn max_decoded_len(code_len: usize) -> Result<usize> {
    let framing_len = HEADER_LEN + FOOTER_LEN;
    if code_len < framing_len {
        return Err(Error::EncodedFrameTooShort)
    }
    let cobs_len = code_len - framing_len;
    // If every byte is a 0x00, then COBS-encoded data will be the
    // same length of 0x01.
    let cobs_decode_limit = cobs_len;
    Ok(cobs_decode_limit)
}

/// Returns the maximum possible encoded length given a frame with
/// the decoded length supplied.
pub fn max_encoded_len(frame_len: usize) -> Result<usize> {
    Ok(HEADER_LEN
        + cobs::max_encoding_length(frame_len)
        + FOOTER_LEN)
}


/// Sends frames over an underlying `io::Write` instance.
#[cfg(feature = "use_std")]
pub struct Sender<W: Write> {
    w: W,
}

#[cfg(feature = "use_std")]
impl<W: Write> Sender<W> {
    pub fn new(w: W) -> Sender<W> {
        Sender::<W> {
            w: w,
        }
    }

    pub fn into_inner(self) -> W {
        self.w
    }

    pub fn send(&mut self, p: &[u8]) -> Result<()> {
        let mut code = cobs::encode_vec(p);
        code.push(END_SYMBOL);
        #[cfg(feature = "trace")] {
            println!("framed: Sending code = {:?}", code);
        }

        #[cfg(feature = "use_std")] {
            self.w.write(&code)?;
        }

        Ok(())
    }
}

/// Receives frames from an underlying `io::Read` instance.
#[cfg(feature = "use_std")]
pub struct Receiver<R: Read> {
    r: R,
}

#[cfg(feature = "use_std")]
impl<R: Read> Receiver<R> {
    pub fn new(r: R) -> Receiver<R> {
        Receiver::<R> {
            r: r,
        }
    }

    pub fn into_inner(self) -> R {
        self.r
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
                    return Err(Error::EofDuringFrame),
                Ok(0) =>
                    return Err(Error::EofDuringFrame),
                Err(e) => return Err(Error::from(e)),
                Ok(_) => (),
            };

            #[cfg(feature = "trace")] {
                println!("framed: Read byte = {}", b);
            }
            if b == END_SYMBOL {
                break;
            } else {
                next_frame.push(b);
            }
        }
        assert!(b == END_SYMBOL);

        cobs::decode_vec(&next_frame)
             .map_err(|_| Error::CobsDecodeFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_encoded_len_ok() {
        assert_eq!(max_encoded_len(0)  .unwrap(), 1);
        assert_eq!(max_encoded_len(1)  .unwrap(), 3);
        assert_eq!(max_encoded_len(2)  .unwrap(), 4);
        assert_eq!(max_encoded_len(254).unwrap(), 256);
        assert_eq!(max_encoded_len(255).unwrap(), 258);
    }

    #[test]
    fn max_decoded_len_too_short() {
        match max_decoded_len(0) {
            Err(Error::EncodedFrameTooShort) => (),
            e @ _ => panic!("Bad output: {:?}", e)
        }
    }

    #[test]
    fn max_decoded_len_ok() {
        assert_eq!(max_decoded_len(1)  .unwrap(), 0);
        assert_eq!(max_decoded_len(2)  .unwrap(), 1);
        assert_eq!(max_decoded_len(3)  .unwrap(), 2);
        assert_eq!(max_decoded_len(255).unwrap(), 254);
    }
}

#[cfg(all(test, not(feature = "use_std")))]
mod rw_tests {
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
