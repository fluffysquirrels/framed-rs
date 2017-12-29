# `framed_decode`

A command line utility to decode data encoded with the `framed` crate.

Clone this repository then run `framed-rs/bin/decode_typed` to use the tool.

## framed-rs/bin/decode_typed

This command decodes a serialized struct encoded with the
[`framed::typed`][typed] module. It requires the struct to implement
the `serde::Deserialize` and `std::fmt::Debug` traits.

[typed]: https://docs.rs/framed/*/framed/typed/index.html

Here's an example. The main binary in crate `framed_test_type` writes
a serialized and encoded `framed_test_type::Test` struct to stdout;
first save that to a file:

```text
~/framed-rs % cargo run -p framed_test_type > test_data

# The Test value is printed to stderr:
test_type/main.rs: Sending sample value: Test {
    a: 1,
    b: 2
}
```

Set environment variables to configure the type to deserialize and the
crate it's in, then pipe the test data to decode_typed. Note that the
serialized data is not self describing, so if you configure the wrong
type you will get garbage data out.

```text
~/framed-rs % export DECODE_TYPE_CRATE_DIR=test_type;
~/framed-rs % export DECODE_TYPE_NAME=framed_test_type::Test;
~/framed-rs % < test_data  bin/decode_typed
# Some build output:
   Compiling framed_test_type v0.1.0 (file:///home/user/framed-rs/test_type)
   Compiling framed_decode v0.1.0 (file:///home/user/framed-rs/decode)

# decode logs each received value to stdout:
decode/main.rs: received value of type framed_test_type::Test: Test {
    a: 1,
    b: 2
}
```

`decode_typed` first builds the type crate with `cargo rustc` then
builds and runs `framed-rs/decode` linked to it. To configure the type
crate build pass cargo arguments in the environment variable
`DECODE_TYPE_CARGO_FLAGS` and rustc arguments in
`DECODE_TYPE_RUSTC_FLAGS`. To configure the decode tool build pass
cargo arguments in `DECODE_BIN_CARGO_FLAGS` and rustc arguments in
`DECODE_BIN_RUSTC_FLAGS`.

`decode_typed` passes command line arguments to the `framed_decode`
crate's binary; run `decode_typed --help` to see all the possible arguments:

```text
~/framed-rs % bin/decode_typed --help

framed_decode 0.1.0
Alex Helfet <alex.helfet@gmail.com>
`framed` sends and receives data over lossy streams of bytes.

`framed_decode` is a command line tool to decode data encoded by the `framed` library.

USAGE:
    decode-5aa7aaf9ea94963a [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --out-format <out-format>
            Output format type used to write data to stdout. [default: Debug]  [values:
            Csv, Debug, Json]
```

Note: you must install [`jq`][jq] and have it on your path.

On Ubuntu you can install it with: `sudo apt install jq`.

[jq]: https://stedolan.github.io/jq/
