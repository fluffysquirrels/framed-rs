# `framed`

Rust crate to send and receive data over lossy streams of bytes.

[![Crate](https://img.shields.io/crates/v/framed.svg)](https://crates.io/crates/framed)

Documentation:
[![Documentation](https://docs.rs/framed/badge.svg)](https://docs.rs/framed)

This crate should build on the latest Rust stable, beta, and nightly
toolchains.  When the cargo feature `use_std` is disabled (requires
nightly toolchain), it should also build in embedded projects with
`no_std`.

## Development

Source code and issues on GitHub:
[![GitHub last commit](https://img.shields.io/github/last-commit/fluffysquirrels/framed-rs.svg)][github]

   [github]: https://github.com/fluffysquirrels/framed-rs

CI build on Travis CI: [![Build Status](https://travis-ci.org/fluffysquirrels/framed-rs.svg)](https://travis-ci.org/fluffysquirrels/framed-rs)

Pull requests welcome.

## Sub-crates

* `framed` in directory `./framed`:

    The core library.

* `framed_decode` in directory `./decode`:

    A command line tool to decode data encoded by the library.

* `framed_test_type` in directory `./test_type`:

    A crate for testing `framed`: a library with encodable types
    and a binary that outputs encoded data.

## License

Licensed under either of

- Apache License, Version 2.0 (see LICENSE-APACHE or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license (see LICENSE-MIT or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
