# Temporary SQLx SQLite dependency patch

This directory contains the published `sqlx-sqlite` 0.8.6 crate from crates.io,
checksum `c2d12fe70b2c1b4401038055f90f151b78208de1f9f89a7dbfd41587a10c3eea`.
It is licensed under either Apache-2.0 or MIT; the upstream license files are
included here. The sole functional change is in the normalized `Cargo.toml`:
`libsqlite3-sys` is pinned to 0.39.1 instead of 0.30.1. That version bundles
SQLite 3.54.1, which addresses RUSTSEC-2026-0459. `Cargo.toml.orig` records the
upstream manifest and is not used by Cargo. Three documentation lines have
trailing whitespace removed; there are no Rust logic changes.

This downstream compatibility patch is not an upstream SQLx-supported version
combination. Revalidate all platforms and database behavior before release;
replace it with a maintained upstream release that supports a patched SQLite as
soon as available. Do not remove the root Cargo patch without rechecking the
selected `libsqlite3-sys` version and advisory status.

Verify the vendored tree with `bash scripts/test sqlx-patch`. It downloads the
checksum-pinned published archive; use `--archive /path/to/sqlx-sqlite-0.8.6.crate`
for offline verification. This checks provenance and local changes, not ABI
compatibility or correctness.
