# tztiny — Project Context

Tiny timezone installer. A single small static binary that embeds the entire
IANA timezone database (deduplicated + deflate-compressed at build time) and
writes out only the TZif entries a system actually needs. Replaces shipping
the full `/usr/share/zoneinfo` tree (~4.2 MB, hundreds of inodes) on small
Linux images.

## Version

Current: **0.1.0**

Version locations:
- `Cargo.toml` — `version =`
- `CHANGELOG.md` — release headings

## Design

- `build.rs` scans the build host's `/usr/share/zoneinfo` (override with
  `TZTINY_ZONEINFO`), keeps only real TZif files (magic `TZif`), skips the
  `right/` and `posix/` duplicate trees, dedups identical zone files by
  content (hard-linked zones collapse to one blob), concatenates the unique
  blobs, and compresses the whole lot with deflate (miniz_oxide, pure Rust).
  It generates a sorted name→blob index as Rust source.
- The binary decompresses on demand and writes only what is requested.
- No clap, no heavy deps; release profile is `opt-level="z"`, LTO, stripped,
  `panic=abort` for minimal size.

## Commands

- `tztiny list [PREFIX...]` — list embedded zone names
- `tztiny install [-o DIR] ZONE|PREFIX...` — write TZif files (default
  `/usr/share/zoneinfo`); `--all` extracts everything
- `tztiny set [-o FILE] ZONE` — write a single TZif directly to
  `/etc/localtime` (a plain file, no zoneinfo tree needed) and record the
  name in `/etc/timezone`
- `tztiny cat ZONE` — TZif to stdout
- `tztiny version` — tool + embedded tzdata version

## Build

Per the cross-project rules: **all builds run on `root@dev.g8.lo`**, never on
the Mac (macOS zoneinfo differs from the target's; the embedded database must
come from a Linux host).

```bash
ssh root@dev.g8.lo
cd /root/tztiny && git pull
export CARGO_TARGET_DIR=/build/cargo/tztiny
cargo build --release && cargo test --release
```

## Work Plan

- [x] Project scaffolding (CLAUDE.md, CHANGELOG, README, .gitignore)
- [x] build.rs: scan, dedup, compress, generate index
- [x] main.rs: list / install / set / cat / version
- [x] Unit tests (index sorted, magic bytes, UTC present, round-trip size)
- [x] Build + test on dev.g8.lo, verify extracted zone matches system file
- [x] Record binary size vs zoneinfo size in README
- [ ] Future: optional musl static build for scratch containers
