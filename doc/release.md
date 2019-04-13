# Release process

## `framed` crate

1. Push changes to [GitHub][github].
1. Check build locally with `bin/build_local`.
1. Check [Travis build][travis]: [![Build Status](https://travis-ci.org/fluffysquirrels/framed-rs.svg)][travis]

   [travis]: https://travis-ci.org/fluffysquirrels/framed-rs
1. Increment version number in Cargo.toml (major version if breaking changes).
1. `cargo update` to update framed version in Cargo.lock.
1. Commit to update the version number.
1. Add a git tag for the new version number and push it to [GitHub][github]:

    `git tag vX.Y.Z && git push --tags`

1. Publish with `bin/publish_lib`.
1. Check new version appears on
   [![Crate](https://img.shields.io/crates/v/framed.svg)][crates]
   and
   [![Documentation](https://docs.rs/framed/badge.svg)][docs]

   [github]: https://github.com/fluffysquirrels/framed-rs
   [crates]: https://crates.io/crates/framed
   [docs]: https://docs.rs/framed
