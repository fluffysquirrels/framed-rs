//! Vec-backed FIFO buffer of bytes for testing.
//!
//! TODO: Remove this, I have an easy workaround.

use std::cell::RefCell;
use std::io::{Read, Result, Write};
use std::rc::Rc;

/// Entry point for the module that can construct multiple `Reader`
/// and `Writer` endpoints using the same backing store.
pub struct Channel {
    inner: Rc<RefCell<Inner>>,
}

struct Inner {
    v: Vec<u8>,
    read_pos: usize,
}

/// Implements `io::Read`, returning data previously written to a
/// `Writer` instance from the same `Channel`.
pub struct Reader {
    inner: Rc<RefCell<Inner>>,
}

/// Implements `io::Write`, writing data to an underlying `Channel`.
pub struct Writer {
    inner: Rc<RefCell<Inner>>,
}

impl Channel {
    /// Construct a new `Channel`.
    pub fn new() -> Channel {
        Channel {
            inner: Rc::new(RefCell::new(Inner {
                v: Vec::new(),
                read_pos: 0
            }))
        }
    }

    /// Construct a new `Reader` for this `Channel`.
    pub fn reader(&self) -> Reader {
        Reader {
            inner: self.inner.clone(),
        }
    }

    /// Construct a new `Writer` for this `Channel`.
    pub fn writer(&self) -> Writer {
        Writer {
            inner: self.inner.clone(),
        }
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut inner = self.inner.borrow_mut();
        let pos = inner.read_pos;
        assert!(inner.v.len() >= pos);

        let left = inner.v.len() - pos;
        let len = buf.len().min(left);

        buf[0..len].copy_from_slice(&inner.v[pos..pos + len]);
        let new_pos = pos + len;
        assert!(inner.v.len() >= new_pos);
        inner.read_pos = new_pos;

        Ok(len)
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.borrow_mut().v.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use super::Channel;

    #[test]
    fn ok() {
        let c = Channel::new();
        let mut tx = c.writer();
        let mut rx = c.reader();

        let mut buf = [0u8; 5];

        assert_eq!(rx.read(&mut buf).unwrap(), 0);

        assert_eq!(tx.write(&[1, 2, 3]).unwrap(), 3);
        assert_eq!(rx.read(&mut buf[0..1]).unwrap(), 1);
        assert_eq!(buf, [1, 0, 0, 0, 0]);
        assert_eq!(rx.read(&mut buf[1..5]).unwrap(), 2);
        assert_eq!(buf, [1, 2, 3, 0, 0]);
    }
}
