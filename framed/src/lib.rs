//! Send and receive data over lossy streams of bytes.
//!
//! Living in / inspired by the [data link layer][dll] or layer 2
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
//!
//! * The "body": payload [COBS]-encoded to remove bytes equal to zero
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
//! `use_std`: Use standard library. Enabled by default, disable for no_std.
//!
//! `trace`: Enable to print all data to stdout for testing.
//!
//! `typed`: Enables the [`typed`](typed/index.html) sub-module for sending and
//!          receiving structs serialized with serde. Enabled by default.
//!
//! ## API
//!
//! Payload data and encoded frame data have separate types to avoid
//! mixing them up. You are encouraged to use these types in your own
//! code when integrating with this crate. Definitions:
//!
//! ```rust,ignore
//! /// Arbitrary user data.
//! pub type Payload = [u8];
//!
//! /// Data that is encoded as a frame. It is ready to send, or may have
//! /// just been received.
//! pub type Encoded = [u8];
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
//! Note that data typed as `Payload` or `Encoded` may be
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
#![cfg_attr(not(feature = "use_std"), no_std)]

// ## extern crate statements
extern crate cobs;
extern crate ref_slice;

#[cfg(feature = "typed")]
extern crate serde;

#[macro_use]
#[cfg(all(test, feature = "typed", feature = "use_std"))]
extern crate serde_derive;

#[cfg(feature = "typed")]
extern crate ssmarshal;


// ## Sub-modules
#[cfg(feature = "use_std")]
pub mod channel;

pub mod error;
pub use error::{Error, Result};

#[cfg(all(feature = "typed", feature = "use_std"))]
pub mod typed;

// ## use statements
#[cfg(feature = "use_std")]
use ref_slice::ref_slice_mut;

#[cfg(feature = "use_std")]
use std::io::{self, Read, Write};

#[cfg(feature = "use_std")]
use std::ops::Deref;


/// Arbitrary user data.
pub type Payload = [u8];

/// Data that is encoded as a frame. It is ready to send, or may have
/// just been received.
pub type Encoded = [u8];

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
/// Returns the number of bytes it has written to the buffer.
///
/// # Panics
///
/// This function will panic if `dest` is not large enough for the encoded frame.
/// Ensure `dest.len() >= max_encoded_len(p.len())`.
pub fn encode_to_slice(p: &Payload, dest: &mut [u8]) -> Result<usize> {
    // Panic if code won't fit in `dest` because this is a programmer error.
    assert!(max_encoded_len(p.len())? <= dest.len());

    let cobs_len = cobs::encode(&p, &mut dest[HEADER_LEN..]);
    let footer_idx = HEADER_LEN + cobs_len;
    dest[footer_idx] = FRAME_END_SYMBOL;

    #[cfg(feature = "trace")] {
        println!("framed: Frame code = {:?}", &dest[0..(footer_idx + 1)]);
    }
    Ok(cobs_len + HEADER_LEN + FOOTER_LEN)
}

/// Encode the supplied payload data as a frame and return it on the heap.
#[cfg(feature = "use_std")]
pub fn encode_to_box(p: &Payload) -> Result<BoxEncoded> {
    let mut buf = vec![0; max_encoded_len(p.len())?];
    let len = encode_to_slice(p, &mut *buf)?;
    buf.truncate(len);
    Ok(BoxEncoded::from(buf))
}

/// Encode the supplied payload data as a frame and write it to the
/// supplied `Write`.
///
/// This function will not call `flush` on the writer; the caller do
/// so if this is required.
///
/// Returns the length of the frame it has written.
#[cfg(feature = "use_std")]
pub fn encode_to_writer<W: Write>(p: &Payload, w: &mut W) -> Result<usize> {
    let b = encode_to_box(p)?;
    w.write_all(&*b.0)?;
    Ok(b.len())
}

/// Decode the supplied encoded frame, placing the payload at the
/// beginning of the supplied buffer `dest`.
///
/// When reading from a stream, the caller can continue reading data
/// and buffering it until a `FRAME_END_SYMBOL` is read, then pass the
/// whole buffer including `FRAME_END_SYMBOL` to this function for
/// decoding.
///
/// If there is more than 1 FRAME_END_SYMBOL within `e`, the result
/// is undefined. Make sure you only pass 1 frame at a time.
///
/// Returns the length of the payload it has decoded.
///
/// # Errors
///
/// Returns `Err(Error::EofDuringFrame` if `e` is not a complete
/// encoded frame, which should have `FRAME_END_SYMBOL` as the last
/// byte.
///
/// # Panics
///
/// This function will panic if `dest` is not large enough for the decoded frame.
/// Ensure `dest.len() >= max_decoded_len(e.len())?`.
pub fn decode_to_slice(e: &Encoded, mut dest: &mut [u8])
-> Result<usize> {
    assert!(dest.len() >= max_decoded_len(e.len())?);

    #[cfg(feature = "trace")] {
        println!("framed: Encoded input = {:?}", e);
    }

    if e[e.len()-1] != FRAME_END_SYMBOL {
        return Err(Error::EofDuringFrame)
    }

    assert_eq!(e[e.len() - 1], FRAME_END_SYMBOL);
    // Just the body (COBS-encoded payload).
    let body = &e[0..(e.len()-1)];

    let len = cobs::decode(body, &mut dest)
                   .map_err(|_| Error::CobsDecodeFailed)?;

    #[cfg(feature = "trace")] {
        println!("framed: dest = {:?}\n\
                  framed: body = {:?}\n\
                  framed: decoded = {:?}",
                 &dest, body, &dest[0..len]);
    }

    Ok(len)
}

/// Decode the supplied encoded frame, returning the payload on the heap.
#[cfg(feature = "use_std")]
pub fn decode_to_box(e: &Encoded) -> Result<BoxPayload> {
    let mut buf = vec![0; max_decoded_len(e.len())?];
    let len = decode_to_slice(e, &mut buf)?;
    buf.truncate(len);
    Ok(BoxPayload::from(buf))
}

/// Reads bytes from the supplied `Read` until it has a complete
/// encoded frame, then decodes the frame, returning the payload on the heap.
#[cfg(feature = "use_std")]
pub fn decode_from_reader<R: Read>(r: &mut Read) -> Result<BoxPayload> {
    // Read until FRAME_END_SYMBOL
    let mut next_frame = Vec::new();
    let mut b = 0u8;
    loop {
        let res = r.read(ref_slice_mut(&mut b));
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
        next_frame.push(b);
        if b == FRAME_END_SYMBOL {
            break;
        }
    }
    assert_eq!(next_frame[next_frame.len()-1], FRAME_END_SYMBOL);

    decode_to_box(&*next_frame)
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

    /// Flush all buffered data. Includes calling `flush` on the inner
    /// writer.
    pub fn flush(&mut self) -> Result<()> {
        Ok(self.w.flush()?)
    }

    /// Queue the supplied payload for transmission.
    ///
    /// This `Sender` may buffer the data indefinitely, as may the
    /// inner writer. To ensure all buffered data has been
    /// transmitted call [`flush`](#method.flush).
    ///
    /// See also: [`send`](#method.send)
    pub fn queue(&mut self, p: &Payload) -> Result<usize> {
        encode_to_writer(p, &mut self.w)
    }

    /// Encode the supplied payload as a frame, write it to the
    /// inner writer, then flush.
    ///
    /// Ensures the data has been transmitted before returning to the
    /// caller.
    ///
    /// See also: [`queue`](#method.queue)
    pub fn send(&mut self, p: &Payload) -> Result<usize> {
        let len = self.queue(p)?;
        self.flush()?;
        Ok(len)
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
        decode_from_reader::<R>(&mut self.r)
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

// TODO: Add tests for all encode_*, decode_* functions.

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
        assert_eq!(tx.send(&p).unwrap(), 5);
        let recvd = rx.recv().unwrap();
        assert_eq!(*recvd, p);
    }

    #[test]
    fn two_frames_sequentially() {
        let (mut tx, mut rx) = pair();
        {
            let sent = [0x00, 0x01, 0x02];
            assert_eq!(tx.send(&sent).unwrap(), 5);
            let recvd = rx.recv().unwrap();
            assert_eq!(*recvd, sent);
        }

        {
            let sent = [0x10, 0x11, 0x12];
            assert_eq!(tx.send(&sent).unwrap(), 5);
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

    fn pair() -> (Sender<Box<Write>>, Receiver<Box<Read>>) {
        let c = Channel::new();
        let tx = Sender::new(Box::new(c.writer()) as Box<Write>);
        let rx = Receiver::new(Box::new(c.reader()) as Box<Read>);
        (tx, rx)
    }
}
