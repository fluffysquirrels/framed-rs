//! Representations of errors returned by this crate.

use std::io;

error_chain! {
    foreign_links {
        Io(io::Error);
    }

    errors {
        CobsDecodeFailed {
            description("COBS decode failed"),
            display("COBS decode failed"),
        }
    }
}
