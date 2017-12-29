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
//! `use_nightly`:  Enables unstable features that only work on nightly rust.
//!                 Required for practical no_std use.
//!
//! `trace`: Enable to print all data to stdout for testing.
//!
//! ## Byte slice wrapper types
//!
//! Payload data and encoded frame data are both slices of bytes but
//! have separate types to help avoid mixing them up. You are
//! encouraged to use these types in your own code when integrating
//! with this crate. Definitions:
//!
//! ```rust,ignore
//! /// Arbitrary user data.
//! pub type Payload = [u8];
//!
//! /// Data that is encoded as a frame. It is ready to send, or may have
//! /// just been received.
//! pub type Encoded = [u8];
//!
//! /// A buffer that is used as temporary storage.
//! /// There are no guarantees on its contents after use.
//! pub type TempBuffer = [u8];
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
//! Note that data typed as `Payload` or `Encoded` may be
//! efficiently passed as a function argument by reference, but is
//! returned using an opaque struct (`BoxPayload`, `BoxEncoded`)
//! containing a heap-allocated value instead. Consequently `encode_*`
//! and `decode_*` variants that require this are only available with
//! the `use_std` Cargo feature.
//!
//! ## Example usage from a std crate
//!
//! See the `decode_*` and `encode_*` functions for simple uses with
//! various input and output types.
//!
//! The `Sender` struct writes encoded payloads to an
//! inner `std::io::Write` instance, and the `Receiver` struct reads
//! and decodes payloads from an inner `std::io::Read` instance.
//!
//! ```rust
//! # use framed::*;
//! # use std::io::Cursor;
//! #
//! let payload = [1, 2, 3];
//!
//! let mut encoded = vec![];
//! {
//!     let mut sender = Sender::new(&mut encoded);
//!     sender.send(&payload).expect("send ok");
//! }
//!
//! // `encoded` now contains the encoded frame.
//!
//! let mut receiver = Receiver::new(Cursor::new(encoded));
//! let decoded = receiver.recv().expect("recv ok");
//!
//! assert_eq!(payload, *decoded);
//! ```
//!
//! ## Example usage from a no_std crate
//!
//! The `encode_to_slice` and `decode_from_slice` functions offer an
//! API for `no_std` crates that might not have a heap allocator
//! available and cannot use `std::io::Read` or `std::io::Write`.
//!
//! ```rust
//! # use framed::*;
//! #
//! // In a no_std crate without dynamic memory allocation we would typically
//! // know the maximum payload length, which we can use for payload buffers.
//! const MAX_PAYLOAD_LEN: usize = 10;
//!
//! // The maximum payload length implies a maximum encoded frame length,
//! // which we can use for frame buffers.
//! //
//! // Using a calculated frame buffer length like this requires
//! // const fn, which is currently unstable and only available on nightly rust.
//! // Enable cargo feature flag "use_nightly" to use it.
//! const MAX_FRAME_LEN: usize = max_encoded_len(MAX_PAYLOAD_LEN);
//!
//! let payload: [u8; 3] = [1, 2, 3];
//! assert!(payload.len() <= MAX_PAYLOAD_LEN);
//!
//! let mut encoded_buf = [0u8; MAX_FRAME_LEN];
//! let encoded_len = encode_to_slice(&payload, &mut encoded_buf).expect("encode ok");
//! let encoded = &encoded_buf[0..encoded_len];
//!
//! // `encoded` now contains the encoded frame.
//!
//! let mut decoded_buf = [0u8; MAX_PAYLOAD_LEN];
//! let decoded_len = decode_to_slice(&encoded, &mut decoded_buf).expect("decode ok");
//! let decoded = &decoded_buf[0..decoded_len];
//!
//! assert_eq!(payload, *decoded);
//! ```
//!

#![deny(warnings)]
#![cfg_attr(not(feature = "use_std"), no_std)]

#![cfg_attr(feature = "use_nightly", feature(const_fn))]

/// Macro const_fn! declares a function
/// with `pub fn`       when feature "use_nightly" is disabled, and
/// with `pub const fn` when feature "use_nightly" is enabled.
///
/// Usage:
/// ```ignore
/// const_fn! {
///     fn foo() {
///         println!("Hello, world!");
///     }
/// }
/// ```
#[cfg(feature = "use_nightly")]
macro_rules! const_fn {
    ($(#[$attr: meta])*
     pub fn $name:ident
     <$($gen_ty_name:ident : $gen_ty_ty:path),+>
     ($($arg_name:ident : $arg_ty: ty),*)
     -> $ret_ty:ty
     $body: block) =>
    ($(#[$attr])*
     pub const fn $name
     <$($gen_ty_name : $gen_ty_ty),*>
     ($($arg_name : $arg_ty),*)
     -> $ret_ty
     $body);

    ($(#[$attr: meta])*
     pub fn $name:ident
     ($($arg_name:ident : $arg_ty: ty),*)
     -> $ret_ty:ty
     $body: block) =>
    ($(#[$attr])*
     pub const fn $name
     ($($arg_name : $arg_ty),*)
     -> $ret_ty
     $body);

    ($(#[$attr: meta])*
     fn $name:ident
     ($($arg_name:ident : $arg_ty: ty),*)
     -> $ret_ty:ty
     $body: block) =>
    ($(#[$attr])*
     const fn $name
     ($($arg_name : $arg_ty),*)
     -> $ret_ty
     $body);
}

#[cfg(not(feature = "use_nightly"))]
macro_rules! const_fn {
    ($(#[$attr: meta])*
     pub fn $name:ident
     <$($gen_ty_name:ident : $gen_ty_ty:path),+>
     ($($arg_name:ident : $arg_ty: ty),*)
     -> $ret_ty:ty
     $body: block) =>
    ($(#[$attr])*
     pub  fn $name
     <$($gen_ty_name : $gen_ty_ty),*>
     ($($arg_name : $arg_ty),*)
     -> $ret_ty
     $body);

    ($(#[$attr: meta])*
     pub fn $name:ident
     ($($arg_name:ident : $arg_ty: ty),*)
     -> $ret_ty:ty
     $body: block) =>
    ($(#[$attr])*
     pub  fn $name
     ($($arg_name : $arg_ty),*)
     -> $ret_ty
     $body);

    ($(#[$attr: meta])*
     fn $name:ident
     ($($arg_name:ident : $arg_ty: ty),*)
     -> $ret_ty:ty
     $body: block) =>
    ($(#[$attr])*
      fn $name
     ($($arg_name : $arg_ty),*)
     -> $ret_ty
     $body);
}

// ## extern crate statements
extern crate cobs;

#[cfg(feature = "use_std")]
extern crate core;

extern crate ref_slice;

extern crate serde;

#[macro_use]
#[cfg(test)]
extern crate serde_derive;

extern crate ssmarshal;


// ## Sub-modules
#[cfg(feature = "use_std")]
pub mod channel;

pub mod error;
pub use error::{Error, Result};

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

/// A buffer that is used as temporary storage.
/// There are no guarantees on its contents after use.
pub type TempBuffer = [u8];

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

/// The frame ends with (and includes) this byte.
///
/// Consumers can read encoded data into a buffer until they encounter
/// this value and then use one of the `decode_*` functions to decode
/// the frame's payload.
pub const FRAME_END_SYMBOL: u8 = 0;

const HEADER_LEN: usize = 0;

const FOOTER_LEN: usize = 1;

/// Encode the supplied payload data as a frame at the beginning of
/// the supplied buffer `dest`. Available from `no_std` crates.
///
/// Returns the number of bytes it has written to the buffer.
///
/// # Panics
///
/// This function will panic if `dest` is not large enough for the encoded frame.
/// Ensure `dest.len() >= max_encoded_len(p.len())`.
pub fn encode_to_slice(p: &Payload, dest: &mut Encoded) -> Result<usize> {
    // Panic if code won't fit in `dest` because this is a programmer error.
    assert!(max_encoded_len(p.len()) <= dest.len());

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
    let mut buf = vec![0; max_encoded_len(p.len())];
    let len = encode_to_slice(p, &mut *buf)?;
    buf.truncate(len);
    Ok(BoxEncoded::from(buf))
}

/// Encode the supplied payload data as a frame and write it to the
/// supplied `Write`.
///
/// This function will not call `flush` on the writer; the caller must do
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
/// beginning of the supplied buffer `dest`. Available from `no_std` crates.
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
/// Returns `Err(Error::EofDuringFrame`) if `e` contains >= 1 bytes of
/// a frame, but not a complete frame. A complete frame should have
/// `FRAME_END_SYMBOL` as the last byte.
///
/// Returns `Err(Error::EofBeforeFrame`) if `e.len()` is 0.
///
/// # Panics
///
/// This function will panic if `dest` is not large enough for the decoded frame.
/// Ensure `dest.len() >= e.len()`.
pub fn decode_to_slice(e: &Encoded, dest: &mut [u8])
-> Result<usize> {
    #[cfg(feature = "trace")] {
        println!("framed: Encoded input = {:?}", e);
    }

    if e.len() == 0 {
        return Err(Error::EofBeforeFrame);
    }

    if e[e.len()-1] != FRAME_END_SYMBOL {
        return Err(Error::EofDuringFrame)
    }

    assert!(dest.len() >= max_decoded_len(e.len()));
    assert_eq!(e[e.len() - 1], FRAME_END_SYMBOL);

    // Just the body (COBS-encoded payload).
    let body = &e[0..(e.len()-1)];

    let len = cobs::decode(body, dest)
                   .map_err(|_| Error::CobsDecodeFailed)?;

    #[cfg(feature = "trace")] {
        println!("framed: body = {:?}\n\
                  framed: decoded = {:?}",
                 body, &dest[0..len]);
    }

    Ok(len)
}

/// Decode the supplied encoded frame, returning the payload on the heap.
#[cfg(feature = "use_std")]
pub fn decode_to_box(e: &Encoded) -> Result<BoxPayload> {
    if e.len() == 0 {
        return Err(Error::EofBeforeFrame);
    }
    let mut buf = vec![0; max_decoded_len(e.len())];
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
            // In the 2 EOF cases defer to decode_to_box to return the
            // correct error (EofBeforeFrame or EofDuringFrame).
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof =>
                return decode_to_box(&*next_frame),
            Ok(0) =>
                return decode_to_box(&*next_frame),

            Err(e) => return Err(Error::from(e)),
            Ok(1) => (),
            Ok(_) => unreachable!(),
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

const_fn! {
    /// Returns an upper bound for the decoded length of the payload
    /// within a frame with the encoded length supplied.
    ///
    /// Useful for calculating an appropriate buffer length.
    pub fn max_decoded_len(code_len: usize) -> usize {
        // This is an over-estimate of the required decoded buffer, but
        // wasting HEADER_LEN + FOOTER_LEN bytes should be acceptable and
        // we can calculate this trivially in a const fn.
        code_len
    }
}

const_fn! {
    /// Returns an upper bound for the encoded length of a frame with
    /// the payload length supplied.
    ///
    /// Useful for calculating an appropriate buffer length.
    pub fn max_encoded_len(payload_len: usize) -> usize {
        HEADER_LEN
            + cobs_max_encoded_len(payload_len)
            + FOOTER_LEN
    }
}

const_fn! {
    /// Copied from `cobs` crate and modified to make a `const` version.
    ///
    /// Source: https://github.com/awelkie/cobs.rs/blob/f8ff1ad2aa7cd069a924d75170d3def3fa6df10b/src/lib.rs#L183-L188
    ///
    /// TODO: Submit a PR to `cobs` to make `cobs::max_encoding_length` a `const fn`.
    ///       Issue for this: https://github.com/fluffysquirrels/framed-rs/issues/19
    fn cobs_max_encoded_len(payload_len: usize) -> usize {
        payload_len
            + (payload_len / 254)

            // This `+ 1` was
            // `+ if payload_len % 254 > 0 { 1 } else { 0 }` in cobs.rs,
            // but that won't compile in a const fn. `1` is less than both the
            // values in the if and else branches, so use that instead, with the
            // acceptable cost of allocating 1 byte more than required some of the
            // time.
            + 1
    }
}

/// Sends encoded frames over an inner `std::io::Write` instance.
#[cfg(feature = "use_std")]
pub struct Sender<W: Write> {
    w: W,
}

#[cfg(feature = "use_std")]
impl<W: Write> Sender<W> {
    /// Construct a `Sender` that writes encoded frames to `w`.
    pub fn new(w: W) -> Sender<W> {
        Sender::<W> {
            w: w,
        }
    }

    /// Consume this `Sender` and return the inner `std::io::Write`.
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

/// Receives encoded frames from an inner `std::io::Read` instance.
#[cfg(feature = "use_std")]
pub struct Receiver<R: Read> {
    r: R,
}

#[cfg(feature = "use_std")]
impl<R: Read> Receiver<R> {
    /// Construct a `Receiver` that receives frames from the supplied
    /// `std::io::Read`.
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
        assert_eq!(max_encoded_len(0)  , 2);
        assert_eq!(max_encoded_len(1)  , 3);
        assert_eq!(max_encoded_len(2)  , 4);
        assert_eq!(max_encoded_len(254), 257);
        assert_eq!(max_encoded_len(255), 258);
    }

    #[test]
    fn max_decoded_len_ok() {
        assert_eq!(max_decoded_len(0)  , 0);
        assert_eq!(max_decoded_len(1)  , 1);
        assert_eq!(max_decoded_len(2)  , 2);
        assert_eq!(max_decoded_len(3)  , 3);
        assert_eq!(max_decoded_len(255), 255);
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
            Err(Error::EofBeforeFrame) => (),
            e @ _ => panic!("Bad value: {:?}", e)
        }
    }

    #[test]
    fn partial_input() {
        let c = Channel::new();
        let mut rx = Receiver::new(Box::new(c.reader()) as Box<Read>);
        let mut tx_raw = c.writer();
        tx_raw.write(&[0x01]).unwrap();
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
