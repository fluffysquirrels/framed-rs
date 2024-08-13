use ::Payload;
use byteorder::{self, ByteOrder};
use core::cmp::PartialEq;
use core::fmt::{self, Debug, Formatter};
use core::ops::Deref;
use crc16;

/// A checksum algorithm configuration to use when encoding data.
#[derive(Clone, Debug)]
#[derive(Default)]
pub enum Checksum {
    /// Use no checksum.
    None,

    /// CRC-16/CDMA2000 as [implemented in crate `crc16`][impl].
    /// This is the default checksum.
    ///
    /// [impl]: https://docs.rs/crc16/0.3.4/crc16/enum.CDMA2000.html
    #[default]
    Crc16Cdma2000,
}


pub(crate) const MAX_CHECKSUM_LEN: usize = 2;

#[derive(Eq)]
pub(crate) struct ChecksumValue {
    data: [u8; MAX_CHECKSUM_LEN],
    len: usize,
}

impl Checksum {
    pub(crate) fn len(&self) -> usize {
        match *self {
            Checksum::None => 0,
            Checksum::Crc16Cdma2000 => 2,
        }
    }

    pub(crate) fn calculate(&self, payload: &Payload) -> ChecksumValue {
        let mut v = ChecksumValue {
            data: [0u8; MAX_CHECKSUM_LEN],
            len: self.len(),
        };

        match *self {
            Checksum::None => (),
            Checksum::Crc16Cdma2000 => {
                let u16 = crc16::State::<crc16::CDMA2000>::calculate(payload);
                byteorder::NetworkEndian::write_u16(&mut v.data, u16);
            },
        };

        v
    }
}

impl Deref for ChecksumValue {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.data[0..self.len]
    }
}

impl PartialEq for ChecksumValue {
    fn eq(&self, other: &ChecksumValue) -> bool {
        **self == **other
    }
}

impl Debug for ChecksumValue {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", &**self)
    }
}
