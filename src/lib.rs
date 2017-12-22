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
//!
//! `use_std`: Use standard library, enabled by default; disable for no_std.
//!
//! `trace`: Enable to print all data to stdout for testing.
//!
//! ## API
//!
//! Payload data and encoded frame data have separate types to avoid
//! mixing them up. You are encouraged to use these types in your own
//! code when integrating with this crate. Definitions:
//!
//! ```rust,ignore
//! /// Arbitrary user data.
//! pub struct Payload(pub [u8]);
//!
//! /// Data that is encoded as a frame. It is ready to send, or may have
//! /// just been received.
//! pub struct Encoded(pub [u8]);
//!
//! /// Heap-allocated user data used as a return type.
//! #[cfg(feature = "use_std")]
//! pub struct BoxPayload(_);
//!
//! /// Heap-allocated frame data used as a return type.
//! #[cfg(feature = "use_std")]
//! pub struct BoxEncoded(_);
//! ```
//!
//! Consumers have a choice of interfaces to enable usability,
//! efficiency, and use from `no_std` crates.
//!
//! See the `decode_*` and `encode_*` functions for simple uses with
//! various input and output types.
//!
//! Note that data (`type`ed as `Payload` or `Encoded`) may be
//! efficiently passed as a function argument by reference, but is
//! returned using an opaque struct (`BoxPayload`, `BoxEncoded`)
//! containing a heap-allocated value instead. Consequently `encode_*`
//! and `decode_*` variants that require this are only available with
//! the `use_std` Cargo feature.
//!
//! For sending or receiving a stream of frames, consider the `Reader`
//! and `Writer` structs that wrap an `io::Read` or `io::Write`
//! instance.

#![deny(warnings)]
// #![feature(coerce_unsized)]
#![feature(conservative_impl_trait)]
// #![feature(unsize)]

#![cfg_attr(not(feature = "use_std"), no_std)]

extern crate cobs;
extern crate ref_slice;

#[cfg(feature = "use_std")]
use ref_slice::ref_slice_mut;

#[cfg(feature = "use_std")]
use std::io::{self, Read, Write};

// #[cfg(feature = "use_std")]
// use std::ops::{CoerceUnsized, Deref};
// #[cfg(not(feature = "use_std"))]
// use core::ops::{CoerceUnsized, Deref};

#[cfg(feature = "use_std")]
use std::ops::Deref;

// #[cfg(feature = "use_std")]
// use std::marker::Unsize;

// #[cfg(not(feature = "use_std"))]
// use core::marker::Unsize;

#[cfg(feature = "use_std")]
pub mod channel;

pub mod error;
#[allow(unused_imports)]
use error::{Error, Result};

// TODO: Move use_std stuff into sub-module use_std to clean up.


/// Arbitrary user data.
///
///
//Type parameter `T: Unsize<[u8]>` is the type of the underlying storage, which must be coercible into a [u8]; i.e. must be an array of bytes or a slice of bytes.
//pub struct Payload<T: Unsize<[u8]>>(pub T);
//pub struct Payload(pub [u8]);
pub type Payload = [u8];

// /// Construct a Payload from an Unsize<[u8]> argument, i.e. any array of bytes.
// impl<T: Unsize<[u8]>> From<T> for Payload {
//     fn from(s: T) -> Payload {
//         Payload(s)
//     }
// }

// /// Payload<T> is coercible to Payload<U> when T is coercible to U;
// /// e.g. Payload<[u8; 5]> is coercible to Payload<[u8]>.
// //impl<T: ?Sized+Unsize<U>, U: ?Sized> CoerceUnsized<Payload<U>> for Payload<T> {}
// 
// impl Deref for Payload {
//     type Target = [u8];
// 
//     fn deref(&self) -> &[u8] {
//         &self.0
//     }
// }

/// Data that is encoded as a frame. It is ready to send, or may have
/// just been received.
pub type Encoded = [u8];
// pub struct Encoded([u8]);

// impl Deref for Encoded {
//     type Target = [u8];
// 
//     fn deref(&self) -> &[u8] {
//         &self.0
//     }
// }

// Note: BoxPayload and BoxEncoded store data in a Vec<u8> as that's
// what `cobs` returns us and converting them into a Box<[u8]> (with
// Vec::into_boxed_slice(self)) would require re-allocation.
//
// See https://doc.rust-lang.org/std/vec/struct.Vec.html#method.into_boxed_slice

/// Heap-allocated user data used as a return type.
#[cfg(feature = "use_std")]
#[derive(Debug)]
pub struct BoxPayload(Vec<u8>);

#[cfg(feature = "use_std")]
impl From<Vec<u8>> for BoxPayload {
    fn from(v: Vec<u8>) -> BoxPayload {
        BoxPayload(v)
    }
}

#[cfg(feature = "use_std")]
impl Deref for BoxPayload {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &*self.0
    }
}

/// Heap-allocated frame data used as a return type.
#[cfg(feature = "use_std")]
#[derive(Debug)]
pub struct BoxEncoded(Vec<u8>);

#[cfg(feature = "use_std")]
impl From<Vec<u8>> for BoxEncoded {
    fn from(v: Vec<u8>) -> BoxEncoded {
        BoxEncoded(v)
    }
}

#[cfg(feature = "use_std")]
impl Deref for BoxEncoded {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &*self.0
    }
}

pub const FRAME_END_SYMBOL: u8 = 0;

const HEADER_LEN: usize = 0;

const FOOTER_LEN: usize = 1;

/// Encode the supplied payload data as a frame at the beginning of
/// the supplied buffer `dest`.
///
/// Returns the length of the frame it has written.
pub fn encode_to_slice(p: &Payload, dest: &mut [u8]) -> Result<usize> {
    // Panic if code won't fit in `dest` because this is a programmer error.
    assert!(max_encoded_len(p.len())? <= dest.len());

    let cobs_len = cobs::encode(&p, &mut dest[HEADER_LEN..]);
    let footer_idx = HEADER_LEN + cobs_len;
    dest[footer_idx] = FRAME_END_SYMBOL;

    #[cfg(feature = "trace")] {
        println!("framed: Frame code = {:?}", dest[0..(footer_idx + 1)]);
    }
    Ok(cobs_len + HEADER_LEN + FOOTER_LEN)
}

/// Encode the supplied payload data as a frame and return it on the heap.
#[cfg(feature = "use_std")]
pub fn encode_to_box(_p: &Payload) -> Result<BoxEncoded> {
    unimplemented!()
}

/// Encode the supplied payload data as a frame and write it to the
/// supplied `Write`.
///
/// Returns the length of the frame it has written.
#[cfg(feature = "use_std")]
pub fn encode_to_writer<W: Write>(_p: &Payload, _w: W) -> Result<usize> {
    unimplemented!()
}

/// Decode the supplied encoded frame, placing the payload at the
/// beginning of the supplied buffer `dest`.
///
/// When reading from a stream, the caller can continue reading data
/// and buffering it until a `FRAME_END_SYMBOL` is read, then pass the
/// whole buffer including `FRAME_END_SYMBOL` to this function for
/// decoding.
///
/// Returns the length of the payload it has decoded.
///
/// ## Errors
///
/// Returns an error if `f` does not contain a complete encoded frame,
/// which would have `FRAME_END_SYMBOL` (a `u8`) as the last byte.
pub fn decode_to_slice(_e: &Encoded, _dest: &mut [u8])
-> Result<usize> {
    unimplemented!()
}

/// Decode the supplied encoded frame, returning the payload on the heap.
#[cfg(feature = "use_std")]
pub fn decode_to_box(_e: &Encoded) -> Result<BoxPayload> {
    unimplemented!()
}

/// Reads bytes from the supplied `Read` until it has a complete
/// encoded frame, then decodes the frame, returning the payload on the heap.
#[cfg(feature = "use_std")]
pub fn decode_from_reader<R: Read>(_r: &Read) -> Result<BoxPayload> {
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

/// Returns the maximum possible encoded length for a frame with
/// the payload length supplied.
pub fn max_encoded_len(payload_len: usize) -> Result<usize> {
    Ok(HEADER_LEN
        + cobs::max_encoding_length(payload_len)
        + FOOTER_LEN)
}


/// Sends encoded frames over an inner `io::Write` instance.
#[cfg(feature = "use_std")]
pub struct Sender<W: Write> {
    w: W,
}

#[cfg(feature = "use_std")]
impl<W: Write> Sender<W> {
    /// Construct a `Sender` that sends frames over the supplied
    /// `io::Write`.
    pub fn new(w: W) -> Sender<W> {
        Sender::<W> {
            w: w,
        }
    }

    /// Consume this `Sender` and return the inner `io::Write`.
    pub fn into_inner(self) -> W {
        self.w
    }

    /// Encode the supplied payload as a frame and send it on the
    /// inner `io::Write`.
    pub fn send(&mut self, p: &Payload) -> Result<()> {
        let buf_len = max_encoded_len(p.len())?;
        let mut buf = vec![0; buf_len];
        let code_len = encode_to_slice(p, &mut buf[0..])?;
        self.w.write(&buf[0..code_len])?;
        Ok(())
    }
}

/// Receives encoded frames from an inner `io::Read` instance.
#[cfg(feature = "use_std")]
pub struct Receiver<R: Read> {
    r: R,
}

#[cfg(feature = "use_std")]
impl<R: Read> Receiver<R> {
    /// Construct a `Receiver` that receives frames from the supplied
    /// `io::Read`.
    pub fn new(r: R) -> Receiver<R> {
        Receiver::<R> {
            r: r,
        }
    }

    /// Consume this `Receiver` and return the inner `io::Read`.
    pub fn into_inner(self) -> R {
        self.r
    }

    /// Receive an encoded frame from the inner `io::Read`, decode it
    /// and return the payload.
    pub fn recv(&mut self) -> Result<BoxPayload> {
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
            if b == FRAME_END_SYMBOL {
                break;
            } else {
                next_frame.push(b);
            }
        }
        assert!(b == FRAME_END_SYMBOL);
        let v = cobs::decode_vec(&next_frame)
                     .map_err(|_| Error::CobsDecodeFailed)?;
        Ok(BoxPayload::from(v))
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

#[cfg(all(test, feature = "use_std"))]
mod rw_tests {
    use channel::Channel;
    use error::Error;
    use std::io::{Read, Write};
    use super::*;

    #[test]
    fn one_frame() {
        let (mut tx, mut rx) = pair();
        let p = [0x00, 0x01, 0x02];
        tx.send(&p).unwrap();
        let recvd = rx.recv().unwrap();
        assert_eq!(*recvd, p);
    }

    #[test]
    fn two_frames_sequentially() {
        let (mut tx, mut rx) = pair();
        {
            let sent = [0x00, 0x01, 0x02];
            tx.send(&sent).unwrap();
            let recvd = rx.recv().unwrap();
            assert_eq!(*recvd, sent);
        }

        {
            let sent = [0x10, 0x11, 0x12];
            tx.send(&sent).unwrap();
            let recvd = rx.recv().unwrap();
            assert_eq!(*recvd, sent);
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

        assert_eq!(*r1, s1);
        assert_eq!(*r2, s2);
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
