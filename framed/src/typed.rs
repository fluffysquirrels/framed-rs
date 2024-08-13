//! Sending and receiving serde-serialized structs.
//!
//! Currently structs are serialized with [`ssmarshal`][ssmarshal],
//! but this is an implementation detail that may change without
//! warning. `ssmarshal` uses a straightforward, compact serialization
//! format that doesn't support compatibility between versions or
//! dynamic length types (arrays, maps). Its lack of versioning fits
//! with the design goals for the frame encoding in this crate:
//! unsuitable for long-term storage or transmission between different
//! versions of an application, but space-efficient.
//!
//! [ssmarshal]: https://crates.io/crates/ssmarshal
//!
//! ## Example usage from a `std` crate
//!
//! The `Sender` struct writes serialized and encoded values to an
//! inner `std::io::Write` instance, and the `Receiver` struct reads
//! and decodes values from an inner `std::io::Read` instance.
//!
//! ```rust
//! # extern crate framed;
//! # use framed::typed::*;
//! # extern crate serde;
//! # #[macro_use]
//! # extern crate serde_derive;
//! #
//! # use std::io::Cursor;
//! #
//! # fn main() {
//!     #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
//!     struct Test {
//!         a: u8,
//!         b: u16,
//!     }
//!
//!     let payload = Test { a: 1, b: 2 };
//!     let mut config = framed::bytes::Config::default().typed::<Test>();
//!
//!     let mut encoded = vec![];
//!     {
//!         let mut sender = config.clone().to_sender(&mut encoded);
//!         sender.send(&payload).expect("send ok");
//!     }
//!
//!     // `encoded` now contains the encoded value.
//!
//!     let mut receiver = config.clone().to_receiver(Cursor::new(encoded));
//!     let decoded = receiver.recv().expect("recv ok");
//!
//!     assert_eq!(payload, decoded);
//! # }
//! ```
//!
//! ## Example usage from a `no_std` crate
//!
//! The `encode_to_slice` and `decode_from_slice` functions offer an
//! API for `no_std` crates that do not have a heap allocator and
//! cannot use `std::io::Read` or `std::io::Write`.
//!
//! ```rust
//! # extern crate framed;
//! # use framed::bytes;
//! # use framed::typed::*;
//! # extern crate serde;
//! # #[macro_use]
//! # extern crate serde_derive;
//! #
//! # fn main() {
//! #   #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
//! #   struct Test {
//! #       a: u8,
//! #       b: u16,
//! #   }
//! #
//! #   let payload = Test { a: 1, b: 2 };
//! #   let mut config = bytes::Config::default().typed::<Test>();
//! #
//!     let mut codec = config.to_codec();
//!     let mut ser_buf = [0u8; max_serialize_buf_len::<Test>()];
//!     let mut encoded_buf = [0u8; max_encoded_len::<Test>()];
//!     let encoded_len = codec.encode_to_slice(
//!         &payload,
//!         &mut ser_buf,
//!         &mut encoded_buf
//!     ).expect("encode ok");
//!     let encoded = &encoded_buf[0..encoded_len];
//!
//!     // `encoded` now contains the complete encoded frame.
//!
//!     let mut de_buf = [0u8; max_serialize_buf_len::<Test>()];
//!     let decoded = codec.decode_from_slice(encoded, &mut de_buf)
//!                        .expect("decode ok");
//!
//!     assert_eq!(payload, decoded);
//! # }
//! ```

use ::{Encoded, TempBuffer};
use ::bytes;
use ::error::{Result};
use core::marker::PhantomData;
use core::mem::size_of;
use serde::Serialize;
use serde::de::DeserializeOwned;
use ssmarshal;
#[cfg(feature = "use_std")]
use std::io::{Read, Write};

/// Contains methods for encoding and decoding a serializable type `T`
/// with a specific configuration.
///
/// Construct an instance from a `Config` instance with the
/// `Config::to_codec` method.
pub struct Codec<T: DeserializeOwned + Serialize> {
    bytes_codec: bytes::Codec,
    _phantom: PhantomData<T>,
}

/// Configurable options for encoding and decoding a serializable type
/// `T` using a builder pattern.
///
/// Construct an instance with the method `framed::bytes::Config::typed<T>()`.
pub struct Config<T: DeserializeOwned + Serialize> {
    bytes_config: bytes::Config,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned + Serialize> Clone for Config<T> {
    fn clone(&self) -> Config<T> {
        Config::<T> {
            bytes_config: self.bytes_config.clone(),
            _phantom: PhantomData::<T>,
        }
    }
}

impl<T: DeserializeOwned + Serialize> Config<T> {
    pub(crate) fn new(bytes_config: &bytes::Config) -> Config<T> {
        Config::<T> {
            bytes_config: bytes_config.clone(),
            _phantom: PhantomData::<T>,
        }
    }

    /// Construct a `Codec` instance with this configuration.
    pub fn to_codec(&mut self) -> Codec<T> {
        Codec::<T> {
            bytes_codec: self.bytes_config.to_codec(),
            _phantom: PhantomData::<T>,
        }
    }

    #[cfg(feature = "use_std")]
    /// Construct a `Receiver` instance with this configuration.
    pub fn to_receiver<R: Read>(&mut self, r: R) -> Receiver<R, T> {
        Receiver::<R, T> {
            codec: self.to_codec(),
            r,
        }
    }

    #[cfg(feature = "use_std")]
    /// Construct a `Sender` instance with this configuration.
    pub fn to_sender<W: Write>(&mut self, w: W) -> Sender<W, T> {
        Sender::<W, T> {
            codec: self.to_codec(),
            w,
        }
    }
}

impl<T: DeserializeOwned + Serialize> Codec<T> {
    /// Serializes and encodes the supplied value `v` into destination
    /// buffer `dest`, using `ser_buf` as a temporary serialization buffer.
    /// Available from `no_std` crates.
    ///
    /// Returns the number of bytes written to the beginning of `dest`.
    ///
    /// ## Panics
    ///
    /// This will panic if the supplied buffers are too small to serialize
    /// a value of `T`. Callers must ensure that:
    ///
    /// * `ser_buf.len() >= max_serialize_buf_len::<T>()` and
    /// * `dest.len() >= max_encoded_len::<T>()`.
    ///
    /// ## Examples
    ///
    /// See the [`no_std` usage example](index.html#example-usage-from-a-no_std-crate)
    /// in the `typed` module documentation.
    pub fn encode_to_slice(
        &mut self,
        v: &T,
        ser_buf: &mut TempBuffer,
        dest: &mut Encoded,
    ) -> Result<usize> {
        assert!(ser_buf.len() >= max_serialize_buf_len::<T>());
        assert!(dest.len() >= max_encoded_len::<T>());

        let ser_len = ssmarshal::serialize(ser_buf, v)?;
        let ser = &ser_buf[0..ser_len];
        self.bytes_codec.encode_to_slice(ser, dest)
    }

    /// Decodes the supplied encoded frame `e`, then deserializes its
    /// payload as a value of type `T`, using `de_buf` as a temporary
    /// deserialization buffer. Available from `no_std` crates.
    ///
    /// Returns the deserialized value.
    ///
    /// ## Panics
    ///
    /// This will panic if the supplied buffer is too small to deserialize
    /// a value of `T`. Callers must ensure that:
    ///
    /// * `de_buf.len() >= max_serialize_buf_len::<T>()`.
    ///
    /// ## Examples
    ///
    /// See the [`no_std` usage example](index.html#example-usage-from-a-no_std-crate)
    /// in the `typed` module documentation.
    pub fn decode_from_slice(
        &mut self,
        e: &Encoded,
        de_buf: &mut TempBuffer
    ) -> Result<T> {
        assert!(de_buf.len() >= max_serialize_buf_len::<T>());

        let de_len = self.bytes_codec.decode_to_slice(e, de_buf)?;
        let payload = &de_buf[0..de_len];
        let (v, _len) = ssmarshal::deserialize(payload)?;
        Ok(v)
    }
}

const_fn! {
    /// Returns an upper bound for the encoded length of a frame with a
    /// serialized `T` value as its payload.
    ///
    /// Useful for calculating an appropriate buffer length.
    pub fn max_encoded_len<T: Serialize>() -> usize {
        bytes::max_encoded_len(max_serialize_buf_len::<T>())
    }
}

const_fn! {
    /// Returns an upper bound for the temporary serialization buffer
    /// length needed by `encode_to_slice` and `decode_from_slice` when
    /// serializing or deserializing a value of type `T`.
    pub fn max_serialize_buf_len<T: Serialize>() -> usize {
        bytes::max_encoded_len(size_of::<T>())
    }
}


/// Sends encoded structs of type `T` over an inner `std::io::Write` instance.
///
/// Construct an instance using the method `Config::to_sender`
///
/// ## Examples
///
/// See the [std usage example](index.html#example-usage-from-a-std-crate)
/// in the `typed` module documentation.
#[cfg(feature = "use_std")]
pub struct Sender<W: Write, T: DeserializeOwned + Serialize> {
    codec: Codec<T>,
    w: W,
}

#[cfg(feature = "use_std")]
impl<W: Write, T: DeserializeOwned + Serialize> Sender<W, T> {
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
    pub fn queue(&mut self, v: &T) -> Result<usize> {
        // This uses a dynamically allocated buffer.
        //
        // I couldn't get a no_std version to compile with a stack-allocated
        // buffer as [0u8; size_of::<T>()] due to compiler errors like:
        // ```
        // error[E0401]: can't use type parameters from outer function; try using a local type parameter instead
        //   --> src/typed.rs:46:39
        //    |
        // 46 |         const _sbl: usize = size_of::<T>();
        //    |                                       ^ use of type variable from outer function
        // ```
        // I think this may require const generics
        // (rust-lang tracking issue:
        // https://github.com/rust-lang/rust/issues/44580).
        let mut ser_buf = vec![0u8; max_serialize_buf_len::<T>()];

        let ser_len = ssmarshal::serialize(&mut ser_buf, v)?;
        let ser = &ser_buf[0..ser_len];
        self.codec.bytes_codec.encode_to_writer(ser, &mut self.w)
    }

    /// Encode the supplied payload as a frame, write it to the
    /// inner writer, then flush.
    ///
    /// Ensures the data has been transmitted before returning to the
    /// caller.
    ///
    /// See also: [`queue`](#method.queue)
    pub fn send(&mut self, v: &T) -> Result<usize> {
        let len = self.queue(v)?;
        self.flush()?;
        Ok(len)
    }
}

/// Receives encoded structs of type `T` from an inner `std::io::Read` instance.
///
/// Construct an instance using the method `Config::to_receiver`
///
/// ## Examples
///
/// See the [std usage example](index.html#example-usage-from-a-std-crate)
/// in the `typed` module documentation.
#[cfg(feature = "use_std")]
pub struct Receiver<R: Read, T: DeserializeOwned + Serialize> {
    codec: Codec<T>,
    r: R,
}

#[cfg(feature = "use_std")]
impl<R: Read, T: DeserializeOwned + Serialize> Receiver<R, T> {
    /// Consume this `Receiver` and return the inner `std::io::Read`.
    pub fn into_inner(self) -> R {
        self.r
    }

    /// Receive an encoded frame from the inner `std::io::Read`, decode it
    /// and return the payload.
    pub fn recv(&mut self) -> Result<T> {
        let payload =
            self.codec.bytes_codec.decode_from_reader::<R>(&mut self.r)?;
        let (v, _len) = ssmarshal::deserialize(&payload)?;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct Test {
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,

        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,

        some: Option<u8>,
        none: Option<u8>,

        a: [u8; 3],
    }

    fn test_val() -> Test {
        let v = Test {
            i8: 1,
            i16: 2,
            i32: 3,
            i64: 4,

            u8: 10,
            u16: 11,
            u32: 12,
            u64: 13,

            some: Some(17),
            none: None,

            a: [1, 2, 3],
        };
        #[cfg(feature = "use_std")] {
            println!("Test value: {v:#?}");
        }
        v
    }

    fn codec() -> Codec<Test> {
        bytes::Config::default().typed::<Test>().to_codec()
    }

    mod slice_tests {
        use super::*;

        #[test]
        #[cfg(feature = "use_nightly")]
        fn roundtrip() {
            let input = test_val();
            let mut c = codec();
            let mut ser_buf = [0u8; max_serialize_buf_len::<Test>()];
            let mut enc_buf = [0u8; max_encoded_len::<Test>()];
            let len = c.encode_to_slice(
                &input, &mut ser_buf, &mut enc_buf
            ).unwrap();
            let enc = &enc_buf[0..len];

            let output = c.decode_from_slice(&enc, &mut ser_buf).unwrap();
            assert_eq!(input, output);
        }

        #[test]
        #[should_panic]
        fn bad_ser_buf_len() {
            let mut ser_buf = [0u8; 1];
            let mut enc     = [0u8; 100];
            codec().encode_to_slice(
                &test_val(), &mut ser_buf, &mut enc
            ).unwrap();
        }

        #[test]
        #[should_panic]
        fn bad_dest_len() {
            let mut ser_buf = [0u8; 100];
            let mut enc     = [0u8; 1];
            codec().encode_to_slice(
                &test_val(), &mut ser_buf, &mut enc
            ).unwrap();
        }
    }

    mod len_tests {
        use super::*;

        #[test]
        fn serialize_buf_len() {
            assert_eq!(max_serialize_buf_len::<Test>(), 44);
            assert_eq!(max_serialize_buf_len::<u8>(), 5);
            assert_eq!(max_serialize_buf_len::<u16>(), 6);
            assert_eq!(max_serialize_buf_len::<u32>(), 8);
            assert_eq!(max_serialize_buf_len::<u64>(), 12);
        }

        #[test]
        fn encoded_len() {
            assert_eq!(max_encoded_len::<Test>(), 48);
            assert_eq!(max_encoded_len::<u8>(), 9);
        }
    }

    #[cfg(feature = "use_std")]
    mod rw_tests {
        use channel::Channel;
        use error::Error;
        use std::io::{Read, Write};
        use super::*;

        #[test]
        fn one() {
            let (mut tx, mut rx) = pair();
            let v = test_val();
            tx.send(&v).unwrap();
            let r = rx.recv().unwrap();
            println!("r: {r:#?}");
            assert_eq!(v, r);
        }

        #[test]
        fn empty_input() {
            let (mut _tx, mut rx) = pair();
            match rx.recv() {
                Err(Error::EofBeforeFrame) => (),
                e => panic!("Bad value: {:?}", e)
            }
        }

        fn pair() -> (Sender<Box<dyn Write>, Test>, Receiver<Box<dyn Read>, Test>) {
            let chan = Channel::new();
            let conf = bytes::Config::default().typed::<Test>();
            let tx = conf.clone()
                         .to_sender(Box::new(chan.writer()) as Box<dyn Write>);
            let rx = conf.clone()
                         .to_receiver(Box::new(chan.reader()) as Box<dyn Read>);
            (tx, rx)
        }
    }
}
