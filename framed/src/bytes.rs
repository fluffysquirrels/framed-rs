//! Sending and receiving slices of bytes.
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
//! # use framed::bytes::*;
//! # use std::io::Cursor;
//! #
//! let mut config = Config::default();
//!
//! let payload = [1, 2, 3];
//!
//! let mut encoded = vec![];
//! {
//!     let mut sender = config.clone().to_sender(&mut encoded);
//!     sender.send(&payload).expect("send ok");
//! }
//!
//! // `encoded` now contains the encoded frame.
//!
//! let mut receiver = config.clone().to_receiver(Cursor::new(encoded));
//! let decoded = receiver.recv().expect("recv ok");
//!
//! assert_eq!(payload, *decoded);
//! ```
//!
//! ## Example usage from a `no_std` crate
//!
//! The `encode_to_slice` and `decode_from_slice` functions offer an
//! API for `no_std` crates that do not have a heap allocator
//! available and cannot use `std::io::Read` or `std::io::Write`.
//!
//! ```rust
//! # use framed::*;
//! # use framed::bytes::*;
//! #
//! let mut codec = Config::default().to_codec();
//!
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
//! let encoded_len = codec.encode_to_slice(&payload, &mut encoded_buf)
//!                        .expect("encode ok");
//! let encoded = &encoded_buf[0..encoded_len];
//!
//! // `encoded` now contains the encoded frame.
//!
//! let mut decoded_buf = [0u8; MAX_PAYLOAD_LEN];
//! let decoded_len = codec.decode_to_slice(&encoded, &mut decoded_buf)
//!                        .expect("decode ok");
//! let decoded = &decoded_buf[0..decoded_len];
//!
//! assert_eq!(payload, *decoded);
//! ```
//!

// ## use statements
use ::{Payload, Encoded, FRAME_END_SYMBOL};
#[cfg(feature = "use_std")]
use ::{BoxPayload, BoxEncoded};
use ::checksum::MAX_CHECKSUM_LEN;
use ::error::{Error, Result};
use ::typed;
use cobs;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[cfg(feature = "use_std")]
use ref_slice::ref_slice_mut;

#[cfg(feature = "use_std")]
use std::io::{self, Read, Write};

pub use ::checksum::Checksum;

/// Contains methods for encoding and decoding byte slices with a
/// specific configuration.
///
/// Construct an instance from a `Config` instance with the
/// `Config::to_codec` method.
pub struct Codec {
    config: Config
}

/// Configurable options for encoding and decoding byte slices using a
/// builder pattern.
///
/// Construct an instance with `Config::default()`.
#[derive(Clone, Debug, Default)]
pub struct Config {
    checksum: Checksum,
}

impl Config {
    /// Construct a `Codec` instance with this configuration.
    pub fn to_codec(&mut self) -> Codec {
        Codec {
            config: self.clone(),
        }
    }

    #[cfg(feature = "use_std")]
    /// Construct a `Receiver` instance with this configuration.
    pub fn to_receiver<R: Read>(&mut self, r: R) -> Receiver<R> {
        Receiver::<R> {
            codec: self.to_codec(),
            r,
        }
    }

    #[cfg(feature = "use_std")]
    /// Construct a `Sender` instance with this configuration.
    pub fn to_sender<W: Write>(&mut self, w: W) -> Sender<W> {
        Sender::<W> {
            codec: self.to_codec(),
            w,
        }
    }

    /// Construct a `framed::typed::Config` instance to encode and decode a
    /// serializable type `T` with this byte encoding configuration.
    pub fn typed<T: DeserializeOwned + Serialize>(&mut self) -> typed::Config<T> {
        typed::Config::<T>::new(self)
    }

    /// Get the current checksum configuration.
    #[must_use] pub fn checksum(&self) -> &Checksum {
        &self.checksum
    }

    /// Set the checksum configuration.
    pub fn set_checksum(&mut self, checksum: Checksum) -> &mut Self {
        self.checksum = checksum;
        self
    }
}

const MAX_FRAMING_LEN: usize = MAX_CHECKSUM_LEN + 1;

impl Codec {
    fn checksum(&self) -> &Checksum {
        &self.config.checksum
    }

    fn min_frame_len(&self) -> usize {
        self.checksum().len()
        + 1 // sentinel length
    }

    /// Encode the supplied payload data as a frame at the beginning of
    /// the supplied buffer `dest`. Available from `no_std` crates.
    ///
    /// Returns the number of bytes it has written to the buffer.
    ///
    /// # Panics
    ///
    /// This function will panic if `dest` is not large enough for the encoded frame.
    /// Ensure `dest.len() >= max_encoded_len(p.len())`.
    pub fn encode_to_slice(&mut self, p: &Payload, dest: &mut Encoded)
    -> Result<usize> {
        // Panic if encoded frame won't fit in `dest` because this is a
        // programmer error.
        assert!(max_encoded_len(p.len()) <= dest.len());

        #[cfg(feature = "trace")] {
            println!("framed::encode: Payload = {:?}", p);
        }

        let checksum_type = self.checksum();
        let checksum_value = checksum_type.calculate(p);

        let cobs_len = {
            let mut cobs_enc = cobs::CobsEncoder::new(dest);
            cobs_enc.push(p)
                    .map_err(|()| Error::CobsEncodeFailed)?;
            if checksum_value.len() > 0 {
                cobs_enc.push(&checksum_value)
                        .map_err(|()| Error::CobsEncodeFailed)?;
            }
            let cobs_len = cobs_enc.finalize()
                                   .map_err(|()| Error::CobsEncodeFailed)?;
            // make sure sentinel is not in output buffer
            for x in &mut dest[..cobs_len] {
                *x ^= FRAME_END_SYMBOL;
            }
            cobs_len
        };
        dest[cobs_len] = FRAME_END_SYMBOL;

        // len is cobs_len + len(FRAME_END_SYMBOL)
        let len = cobs_len + 1;
        #[cfg(feature = "trace")] {
            println!("framed::encode: Frame = {:?}", &dest[0..len]);
        }
        Ok(len)
    }

    /// Encode the supplied payload data as a frame and return it on the heap.
    #[cfg(feature = "use_std")]
    pub fn encode_to_box(&mut self, p: &Payload) -> Result<BoxEncoded> {
        let mut buf = vec![0; max_encoded_len(p.len())];
        let len = self.encode_to_slice(p, &mut buf)?;
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
    pub fn encode_to_writer<W: Write>(&mut self, p: &Payload, w: &mut W)
    -> Result<usize> {
        let b = self.encode_to_box(p)?;
        w.write_all(&b.0)?;
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
    /// If there is more than 1 `FRAME_END_SYMBOL` within `e`, the result
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
    pub fn decode_to_slice(&mut self, e: &Encoded, dest: &mut [u8])
    -> Result<usize> {
        #[cfg(feature = "trace")] {
            println!("framed::decode: Encoded = {:?}", e);
        }

        if e.is_empty() {
            return Err(Error::EofBeforeFrame);
        }

        if e.len() < self.min_frame_len() {
            return Err(Error::EofDuringFrame);
        }

        if e[e.len()-1] != FRAME_END_SYMBOL {
            return Err(Error::EofDuringFrame)
        }

        assert!(dest.len() >= max_decoded_len(e.len()));
        assert_eq!(e[e.len() - 1], FRAME_END_SYMBOL);

        let cobs_payload = &e[0..e.len() - 1];
        let cobs_decoded_len =
            if cobs_payload.is_empty() {
                0
            } else {
                cobs::decode_with_sentinel(cobs_payload, dest, FRAME_END_SYMBOL)
                    .map_err(|()| Error::CobsDecodeFailed)?
            };
        let cobs_decoded = &dest[0..cobs_decoded_len];
        if cobs_decoded_len < self.checksum().len() {
            return Err(Error::EofDuringFrame);
        }
        let payload = &cobs_decoded[0..cobs_decoded_len - self.checksum().len()];

        #[cfg(feature = "trace")] {
            println!("framed::decode: cobs_decoded = {:?}",
                     cobs_decoded);
            println!("framed::decode: payload = {:?}",
                     payload);
        }

        let checksum = self.checksum();
        let calc_checksum = checksum.calculate(payload);
        let recv_checksum = &cobs_decoded[payload.len() .. payload.len() + checksum.len()];

        #[cfg(feature = "trace")] {
            println!("framed::decode: calc_checksum = {:?}\n\
                      framed::decode: recv_checksum = {:?}",
                     calc_checksum, recv_checksum);
        }

        if &*calc_checksum != recv_checksum {
            return Err(Error::ChecksumError);
        }

        Ok(payload.len())
    }

    /// Decode the supplied encoded frame, returning the payload on the heap.
    #[cfg(feature = "use_std")]
    pub fn decode_to_box(&mut self, e: &Encoded) -> Result<BoxPayload> {
        if e.is_empty() {
            return Err(Error::EofBeforeFrame);
        }
        let mut buf = vec![0; max_decoded_len(e.len())];
        let len = self.decode_to_slice(e, &mut buf)?;
        buf.truncate(len);
        Ok(BoxPayload::from(buf))
    }

    /// Reads bytes from the supplied `Read` until it has a complete
    /// encoded frame, then decodes the frame, returning the payload on the heap.
    #[cfg(feature = "use_std")]
    pub fn decode_from_reader<R: Read>(&mut self, r: &mut dyn Read)
    -> Result<BoxPayload> {
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
                    return self.decode_to_box(&next_frame),
                Ok(0) =>
                    return self.decode_to_box(&next_frame),

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

        self.decode_to_box(&next_frame)
    }
} // End of impl Codec.

const_fn! {
    /// Returns an upper bound for the decoded length of the payload
    /// within a frame with the encoded length supplied.
    ///
    /// Useful for calculating an appropriate buffer length.
    pub fn max_decoded_len(code_len: usize) -> usize {
        // This is an over-estimate of the required decoded buffer, but
        // wasting MAX_HEADER_LEN + MAX_FOOTER_LEN bytes should be acceptable and
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
        MAX_FRAMING_LEN
            + cobs_max_encoded_len(payload_len)
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
///
/// Construct an instance using the method `Config::to_sender`
#[cfg(feature = "use_std")]
pub struct Sender<W: Write> {
    codec: Codec,
    w: W,
}

#[cfg(feature = "use_std")]
impl<W: Write> Sender<W> {
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
        self.codec.encode_to_writer(p, &mut self.w)
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
///
/// Construct an instance using the method `Config::to_receiver`
#[cfg(feature = "use_std")]
pub struct Receiver<R: Read> {
    codec: Codec,
    r: R,
}

#[cfg(feature = "use_std")]
impl<R: Read> Receiver<R> {
    /// Consume this `Receiver` and return the inner `io::Read`.
    pub fn into_inner(self) -> R {
        self.r
    }

    /// Receive an encoded frame from the inner `io::Read`, decode it
    /// and return the payload.
    pub fn recv(&mut self) -> Result<BoxPayload> {
        self.codec.decode_from_reader::<R>(&mut self.r)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "use_std")]
    use std::io::Cursor;
    use super::*;

    #[test]
    fn max_encoded_len_ok() {
        assert_eq!(max_encoded_len(0)  , 4);
        assert_eq!(max_encoded_len(1)  , 5);
        assert_eq!(max_encoded_len(2)  , 6);
        assert_eq!(max_encoded_len(254), 259);
        assert_eq!(max_encoded_len(255), 260);
    }

    #[test]
    fn max_decoded_len_ok() {
        assert_eq!(max_decoded_len(0)  , 0);
        assert_eq!(max_decoded_len(1)  , 1);
        assert_eq!(max_decoded_len(2)  , 2);
        assert_eq!(max_decoded_len(3)  , 3);
        assert_eq!(max_decoded_len(255), 255);
    }

    fn codec() -> Codec {
        Config::default().to_codec()
    }

    // A test payload.
    const PAYLOAD: [u8; PAYLOAD_LEN] = [0, 1, 2, 3];
    const PAYLOAD_LEN: usize = 4;

    const ENCODED_LEN: usize = 9;

    /// Returns an encoded frame with payload PAYLOAD.
    fn encoded_payload(buf: &mut [u8; ENCODED_LEN]) -> &[u8] {
        let len = codec().encode_to_slice(&PAYLOAD, buf).unwrap();
        &buf[0..len]
    }

    fn assert_payload_eq(encoded: &Encoded, payload: &Payload) {
        #[cfg(feature = "use_std")] {
            println!("assert_payload_eq \n\
                      -  encoded = {encoded:?}\n\
                      -  payload = {payload:?}");
        }
        let mut decoded_buf = [0; 100];
        let len = codec().decode_to_slice(encoded, &mut decoded_buf).unwrap();
        let decoded = &decoded_buf[0..len];
        assert_eq!(decoded, payload);
    }

    #[test]
    #[should_panic]
    fn encode_to_slice_dest_too_small() {
        let mut encoded_buf = [0u8; PAYLOAD_LEN];
        let _ = codec().encode_to_slice(&PAYLOAD, &mut encoded_buf);
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn encode_to_slice_ok_dynamic_dest() {
        let mut encoded_buf = vec![0u8; max_encoded_len(PAYLOAD.len())];
        let len = codec().encode_to_slice(&PAYLOAD, &mut encoded_buf).unwrap();
        let encoded = &encoded_buf[0..len];

        assert_payload_eq(encoded, &PAYLOAD);
    }

    // use_nightly required for statically allocated buffer
    #[test]
    #[cfg(feature = "use_nightly")]
    fn encode_to_slice_ok_static_dest() {
        let mut encoded_buf = [0u8; max_encoded_len(PAYLOAD_LEN)];
        let len = codec().encode_to_slice(&PAYLOAD, &mut encoded_buf).unwrap();
        let encoded = &encoded_buf[0..len];

        assert_payload_eq(encoded, &PAYLOAD);
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn encode_to_writer_ok() {
        let mut encoded = vec![];
        codec().encode_to_writer(&PAYLOAD, &mut encoded).unwrap();
        assert_payload_eq(&encoded, &PAYLOAD);
    }

    #[test]
    #[should_panic]
    fn decode_to_slice_dest_too_small() {
        let mut buf = [0; ENCODED_LEN];
        let encoded = encoded_payload(&mut buf);
        let mut decoded_buf = [0u8; PAYLOAD_LEN - 1];
        let _ = codec().decode_to_slice(encoded, &mut decoded_buf);
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn decode_to_slice_ok_dynamic_dest() {
        let encoded = codec().encode_to_box(&PAYLOAD).unwrap();
        let mut decoded_buf = vec![0u8; max_decoded_len(encoded.len())];
        let len = codec().decode_to_slice(&encoded, &mut decoded_buf).unwrap();
        let decoded = &decoded_buf[0..len];

        assert_eq!(&PAYLOAD, decoded);
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn decode_to_slice_no_end_symbol() {
        let encoded = vec![FRAME_END_SYMBOL + 1; max_encoded_len(0)];
        let mut decoded_buf = [];
        let res = codec().decode_to_slice(&encoded, &mut decoded_buf);

        match res {
            Err(Error::EofDuringFrame) => (),
            _ => panic!("Bad output: {:?}", res),
        }
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn decode_to_slice_encoded_too_short() {
        let mut c = codec();
        let encoded = vec![FRAME_END_SYMBOL; c.min_frame_len() - 1];
        let mut decoded_buf = [];
        let res = c.decode_to_slice(&encoded, &mut decoded_buf);

        match res {
            Err(Error::EofDuringFrame) => (),
            _ => panic!("Bad output: {:?}", res),
        }
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn decode_to_slice_encoded_empty() {
        let encoded = vec![];
        let mut decoded_buf = [];
        let res = codec().decode_to_slice(&encoded, &mut decoded_buf);

        match res {
            Err(Error::EofBeforeFrame) => (),
            _ => panic!("Bad output: {:?}", res),
        }
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn decode_to_slice_bad_checksum() {
        let mut c = codec();
        let encoded = c.encode_to_box(&PAYLOAD).unwrap();
        let mut encoded = Vec::from(&*encoded);
        let checksum_offset = encoded.len() - c.checksum().len() - 1;

        {
            let checksum =
                &mut encoded[checksum_offset..
                             (checksum_offset + c.config.checksum.len())];
            checksum[0] = checksum[0].wrapping_add(1);
            checksum[1] = checksum[1].wrapping_add(2);
        }

        let mut decoded_buf = vec![0u8; max_decoded_len(encoded.len())];
        let res = codec().decode_to_slice(&encoded, &mut decoded_buf);

        match res {
            Err(Error::ChecksumError) => (),
            _ => panic!("Bad output: {:?}", res),
        }
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn decode_to_slice_missing_bytes() {
        let encoded = codec().encode_to_box(&PAYLOAD).unwrap();
        let encoded = &encoded[1..encoded.len()];

        let mut decoded_buf = vec![0u8; max_decoded_len(encoded.len())];
        let res = codec().decode_to_slice(encoded, &mut decoded_buf);

        match res {
            Err(Error::ChecksumError) => (),
            _ => panic!("Bad output: {:?}", res),
        }
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn decode_to_box_ok() {
        let encoded = codec().encode_to_box(&PAYLOAD).unwrap();
        let decoded = codec().decode_to_box(&encoded).unwrap();

        assert_eq!(&PAYLOAD, &*decoded);
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn decode_from_reader_ok() {
        let mut c = codec();
        let encoded = c.encode_to_box(&PAYLOAD).unwrap();
        let mut reader = Cursor::new(&*encoded);
        let decoded = c.decode_from_reader::<Cursor<&[u8]>>(&mut reader).unwrap();
        assert_eq!(&*decoded, &PAYLOAD);
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn roundtrip_default_config() {
        roundtrip_case(&mut Config::default()
                                   .to_codec(),
                       &PAYLOAD);
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn roundtrip_no_checksum() {
        roundtrip_case(&mut Config::default()
                               .set_checksum(Checksum::None)
                               .to_codec(),
                       &PAYLOAD);
    }

    #[test]
    #[cfg(feature = "use_std")]
    fn roundtrip_empty_payload() {
        roundtrip_case(&mut Config::default()
                                   .to_codec(),
                       &[]);
    }

    #[cfg(feature = "use_std")]
    fn roundtrip_case(c: &mut Codec, payload: &Payload) {
        let encoded = c.encode_to_box(payload);
        println!("encoded: {encoded:?}");
        let encoded = encoded.unwrap();
        let decoded = c.decode_to_box(&encoded);
        println!("decoded: {decoded:?}");
        let decoded = decoded.unwrap();
        assert_eq!(&*decoded, payload);
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
        println!("r1: {r1:?}\n\
                  r2: {r2:?}");

        assert_eq!(*r1, s1);
        assert_eq!(*r2, s2);
    }

    #[test]
    fn empty_input() {
        let (mut _tx, mut rx) = pair();
        match rx.recv() {
            Err(Error::EofBeforeFrame) => (),
            e => panic!("Bad value: {:?}", e)
        }
    }

    #[test]
    fn partial_input() {
        let chan = Channel::new();
        let mut rx = config().to_receiver(chan.reader());
        let mut tx_raw = chan.writer();
        tx_raw.write(&[0x01]).unwrap();
        match rx.recv() {
            Err(Error::EofDuringFrame) => (),
            e => panic!("Bad value: {:?}", e)
        }
    }

    fn config() -> Config {
        Config::default()
    }

    fn pair() -> (Sender<Box<dyn Write>>, Receiver<Box<dyn Read>>) {
        let chan = Channel::new();
        let c = config();
        let tx = c.clone().to_sender(Box::new(chan.writer()) as Box<dyn Write>);
        let rx = c.clone().to_receiver(Box::new(chan.reader()) as Box<dyn Read>);
        (tx, rx)
    }
}
