//! Sending and receiving structs serialized with serde and
//! [`ssmarshal`][ssmarshal].
//!
//! [ssmarshal]: https://crates.io/crates/ssmarshal
//!
//! `ssmarshal` uses a straightforward, compact serialization format
//! that doesn't support compatibility between versions or dynamic
//! length types (arrays, maps). Its lack of stability fits with the
//! frame encoding in this crate: unsuitable for long-term storage or
//! transmission between different versions of an application.
//!
//! This module currently requires `std`, the standard library.

use error::{Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use ssmarshal;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::mem::size_of;

/// Sends encoded structs of type `T` over an inner `io::Write` instance.
pub struct Sender<W: Write, T: Serialize> {
    w: W,
    _t: PhantomData<T>,
}

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
        //
        // When I need to write a no_std version I see a few easy options:
        // 1. Caller supplies a reference to a buffer, we can assert! that
        //    it's long enough. Annoying to use.
        // 2. Choose a reasonable length buffer and assert it's long enough.
        //    Won't work for large enough structs, may consume an inappropriate
        //    amount of memory for embedded use.
        // 3. Provide overloads for 1 and 2. For most cases the fixed size
        //    buffer will be fine, and if not you can provide your own.
        let mut ser_buf = vec![0u8; size_of::<T>()];

        let ser_len = ssmarshal::serialize(&mut ser_buf, v)?;
        let ser = &ser_buf[0..ser_len];
        #[cfg(feature = "trace")] {
            println!("framed: Serialized = {:?}", ser);
        }
        super::encode_to_writer(&ser, &mut self.w)
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
pub struct Receiver<R: Read, T: DeserializeOwned> {
    r: R,
    _t: PhantomData<T>,
}

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
    use channel::Channel;
    use error::Error;
    use std::io::{Read, Write};
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

    #[test]
    fn one() {
        let (mut tx, mut rx) = pair();
        let v = val();
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

    fn val() -> Test {
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

    fn pair() -> (Sender<Box<Write>, Test>, Receiver<Box<Read>, Test>) {
        let c = Channel::new();
        let tx = Sender::new(Box::new(c.writer()) as Box<Write>);
        let rx = Receiver::new(Box::new(c.reader()) as Box<Read>);
        (tx, rx)
    }
}
