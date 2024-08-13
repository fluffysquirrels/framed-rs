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
//! See the `bytes` or `typed` modules to send and receive raw slices
//! of bytes or serialized structs respectively.
//!
//! ## Encoding
//!
//! Currently the encoding is:
//!
//! * Frame [COBS]-encoded to remove bytes equal to zero
//!   * Payload: supplied by user
//!   * Footer:
//!     * A checksum computed over the payload. The type of checksum depends
//!       on the configuration in `framed::bytes::Config`.
//! * A terminating zero byte.
//!
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
//! `use_std`: Use standard library. Enabled by default, disable for `no_std`.
//!
//! `use_nightly`:  Enables unstable features that only work on nightly rust.
//!                 Required for practical `no_std` use.
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

#![warn(missing_docs)]

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
extern crate byteorder;
extern crate cobs;

#[cfg(feature = "use_std")]
extern crate core;

extern crate crc16;
extern crate ref_slice;
extern crate serde;

#[macro_use]
#[cfg(test)]
extern crate serde_derive;

extern crate ssmarshal;


// ## Sub-modules
pub mod bytes;

#[cfg(all(test, feature = "use_std"))]
mod channel;

mod checksum;

pub mod error;
pub use error::{Error, Result};

pub mod typed;

// ## use statements
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
        &self.0
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
        &self.0
    }
}

/// The frame ends with (and includes) this byte.
///
/// Consumers can read encoded data into a buffer until they encounter
/// this value and then use one of the `decode_*` functions to decode
/// the frame's payload.
pub const FRAME_END_SYMBOL: u8 = 0;
