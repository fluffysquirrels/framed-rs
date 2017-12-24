//! Sending and receiving structs serialized with serde.

use error::{Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
// use ssmarshal;
use std::io::{Read, Write};
use std::marker::PhantomData;


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
    pub fn queue(&mut self, _v: &T) -> Result<usize> {
        unimplemented!();
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
        unimplemented!()
    }
}
