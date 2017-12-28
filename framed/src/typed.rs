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
//!
//!     let mut encoded = vec![];
//!     {
//!         let mut sender = Sender::<_, Test>::new(&mut encoded);
//!         sender.send(&payload).expect("send ok");
//!     }
//!
//!     // `encoded` now contains the encoded value.
//!
//!     let mut receiver = Receiver::<_, Test>::new(Cursor::new(encoded));
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
//! #
//!     let mut ser_buf = [0u8; max_serialize_buf_len::<Test>()];
//!     let mut encoded_buf = [0u8; max_encoded_len::<Test>()];
//!     let encoded_len = encode_to_slice::<Test>(
//!         &payload,
//!         &mut ser_buf,
//!         &mut encoded_buf
//!     ).expect("encode ok");
//!     let encoded = &encoded_buf[0..encoded_len];
//!
//!     // `encoded` now contains the complete encoded frame.
//!
//!     let mut de_buf = [0u8; max_serialize_buf_len::<Test>()];
//!     let decoded = decode_from_slice(encoded, &mut de_buf)
//!                       .expect("decode ok");
//!
//!     assert_eq!(payload, decoded);
//! # }
//! ```

use ::{Encoded, TempBuffer};
use ::error::{Result};
use core::marker::PhantomData;
use core::mem::size_of;
use serde::Serialize;
use serde::de::DeserializeOwned;
use ssmarshal;
#[cfg(feature = "use_std")]
use std::io::{Read, Write};

/// Serializes and encodes the supplied value `v` into destination
/// buffer `dest`, using `ser_buf` as a temporary serialization buffer.
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
/// See the [no_std usage example][ex] in the `typed` module documentation.
/// [ex]: index.html#example-usage-from-a-no_std-crate
pub fn encode_to_slice<T: DeserializeOwned + Serialize>(
    v: &T,
    ser_buf: &mut TempBuffer,
    dest: &mut Encoded,
) -> Result<usize> {
    assert!(ser_buf.len() >= max_serialize_buf_len::<T>());
    assert!(dest.len() >= max_encoded_len::<T>());

    let ser_len = ssmarshal::serialize(ser_buf, v)?;
    let ser = &ser_buf[0..ser_len];
    ::encode_to_slice(ser, dest)
}

/// Decodes the supplied encoded frame `e`, then deserializes its
/// payload as a value of type `T`, using `de_buf` as a temporary
/// deserialization buffer.
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
/// See the [no_std usage example][ex] in the `typed` module documentation.
/// [ex]: index.html#example-usage-from-a-no_std-crate
pub fn decode_from_slice<T: DeserializeOwned + Serialize>(
    e: &Encoded,
    de_buf: &mut TempBuffer
) -> Result<T> {
    assert!(de_buf.len() >= max_serialize_buf_len::<T>());

    let de_len = ::decode_to_slice(e, de_buf)?;
    let payload = &de_buf[0..de_len];
    let (v, _len) = ssmarshal::deserialize(payload)?;
    Ok(v)
}

/// Returns an upper bound for the encoded length of a frame with a
/// serialized `T` value as its payload.
///
/// Useful for calculating an appropriate buffer length.
pub const fn max_encoded_len<T: DeserializeOwned + Serialize>() -> usize {
    super::max_encoded_len(max_serialize_buf_len::<T>())
}

/// Returns an upper bound for the temporary serialization buffer
/// length needed by `encode_to_slice` and `decode_from_slice` when
/// serializing or deserializing a value of type `T`.
pub const fn max_serialize_buf_len<T: DeserializeOwned + Serialize>() -> usize {
    super::max_encoded_len(size_of::<T>())
}


/// Sends encoded structs of type `T` over an inner `io::Write` instance.
///
/// ## Examples
///
/// See the [std usage example][ex] in the `typed` module documentation.
/// [ex]: index.html#example-usage-from-a-std-crate
#[cfg(feature = "use_std")]
pub struct Sender<W: Write, T: Serialize> {
    w: W,
    _t: PhantomData<T>,
}

#[cfg(feature = "use_std")]
impl<W: Write, T: Serialize> Sender<W, T> {
    /// Construct a `Sender` that sends encoded structs over the supplied
    /// `io::Write`.
    pub fn new(w: W) -> Sender<W, T> {
        Sender::<W, T> {
            w: w,
            _t: PhantomData::default(),
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
        let mut ser_buf = vec![0u8; size_of::<T>()];

        let ser_len = ssmarshal::serialize(&mut ser_buf, v)?;
        let ser = &ser_buf[0..ser_len];
        ::encode_to_writer(&ser, &mut self.w)
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

/// Receives encoded structs of type `T` from an inner `io::Read` instance.
///
/// ## Examples
///
/// See the [std usage example][ex] in the `typed` module documentation.
/// [ex]: index.html#example-usage-from-a-std-crate
#[cfg(feature = "use_std")]
pub struct Receiver<R: Read, T: DeserializeOwned> {
    r: R,
    _t: PhantomData<T>,
}

#[cfg(feature = "use_std")]
impl<R: Read, T: DeserializeOwned> Receiver<R, T> {
    /// Construct a `Receiver` that receives encoded structs from the supplied
    /// `io::Read`.
    pub fn new(r: R) -> Receiver<R, T> {
        Receiver::<R, T> {
            r: r,
            _t: PhantomData::default(),
        }
    }

    /// Consume this `Receiver` and return the inner `io::Read`.
    pub fn into_inner(self) -> R {
        self.r
    }

    /// Receive an encoded frame from the inner `io::Read`, decode it
    /// and return the payload.
    pub fn recv(&mut self) -> Result<T> {
        let payload = super::decode_from_reader::<R>(&mut self.r)?;
        let (v, _len) = ssmarshal::deserialize(&*payload)?;
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
        println!("Test value: {:#?}", v);
        v
    }

    mod slice_tests {
        use super::*;

        #[test]
        fn roundtrip() {
            let input = test_val();
            let mut ser_buf = [0u8; max_serialize_buf_len::<Test>()];
            let mut enc_buf = [0u8; max_encoded_len::<Test>()];
            let len = super::encode_to_slice(
                &input, &mut ser_buf, &mut enc_buf
            ).unwrap();
            let enc = &enc_buf[0..len];

            let output = super::decode_from_slice(&enc, &mut ser_buf).unwrap();
            assert_eq!(input, output);
        }

        #[test]
        #[should_panic]
        fn bad_ser_buf_len() {
            let mut ser_buf = [0u8; 1];
            let mut enc     = [0u8; 100];
            super::encode_to_slice(
                &test_val(), &mut ser_buf, &mut enc
            ).unwrap();
        }

        #[test]
        #[should_panic]
        fn bad_dest_len() {
            let mut ser_buf = [0u8; 100];
            let mut enc     = [0u8; 1];
            super::encode_to_slice(
                &test_val(), &mut ser_buf, &mut enc
            ).unwrap();
        }
    }

    mod len_tests {
        use super::*;

        #[test]
        fn serialize_buf_len() {
            assert_eq!(max_serialize_buf_len::<Test>(), 42);
            assert_eq!(max_serialize_buf_len::<u8>(), 3);
            assert_eq!(max_serialize_buf_len::<u16>(), 4);
            assert_eq!(max_serialize_buf_len::<u32>(), 6);
            assert_eq!(max_serialize_buf_len::<u64>(), 10);
        }

        #[test]
        fn encoded_len() {
            assert_eq!(max_encoded_len::<Test>(), 44);
            assert_eq!(max_encoded_len::<u8>(), 5);
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
            println!("r: {:#?}", r);
            assert_eq!(v, r);
        }

        #[test]
        fn empty_input() {
            let (mut _tx, mut rx) = pair();
            match rx.recv() {
                Err(Error::EofBeforeFrame) => (),
                e @ _ => panic!("Bad value: {:?}", e)
            }
        }

        fn pair() -> (Sender<Box<Write>, Test>, Receiver<Box<Read>, Test>) {
            let c = Channel::new();
            let tx = Sender::new(Box::new(c.writer()) as Box<Write>);
            let rx = Receiver::new(Box::new(c.reader()) as Box<Read>);
            (tx, rx)
        }
    }
}
