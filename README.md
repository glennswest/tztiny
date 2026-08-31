# tztiny

A tiny, single-binary IANA timezone database installer for small Linux images.

A full `/usr/share/zoneinfo` tree is ~4.2 MB across hundreds of files — often
one of the largest things in a minimal image. `tztiny` embeds the entire
database in one small executable: at build time it deduplicates identical zone
files (most zones are hard links of each other) and compresses the unique data
with deflate. At runtime it writes out only the zones you actually need — or
just a single `/etc/localtime`, which needs no zoneinfo tree at all.

## Usage

```bash
# Set the system timezone with one file (no zoneinfo directory needed):
tztiny set America/Chicago            # writes /etc/localtime + /etc/timezone

# Install specific zones (for software that resolves TZ=Name lookups):
tztiny install America/Chicago Europe/London UTC

# Install a whole subtree, or everything, somewhere else:
tztiny install -o /usr/share/zoneinfo America
tztiny install -o /tmp/zi --all

# Explore:
tztiny list America/Argentina
tztiny cat Asia/Tokyo > /etc/localtime
tztiny version
```

Zone arguments are exact names (`America/Chicago`) or prefixes (`America`)
selecting a whole subtree.

## Building

The embedded database is read from the **build host's** `/usr/share/zoneinfo`
(override the path with `TZTINY_ZONEINFO=/path cargo build`), so build on a
Linux host with the tzdata version you want to ship. The `right/` and `posix/`
duplicate trees are skipped.

```bash
cargo build --release
```

The release profile is tuned for size (`opt-level = "z"`, LTO, stripped,
`panic = "abort"`). The only dependency is `miniz_oxide` (pure Rust deflate).

## Size

Built against tzdata 2025b (x86_64 Linux, glibc):

| what | size |
|---|---|
| full `/usr/share/zoneinfo` tree | 4.2 MB on disk, 599 zones |
| `tztiny` binary (all 599 zones included) | **463 KB**, 1 file |
| embedded database (341 unique blobs, deflated) | 103 KB of that |
| `tztiny set` output (`/etc/localtime`) | one TZif file (~1–3 KB) |

Extracted zones are byte-identical to the originals (`tztiny install --all`
reproduces the whole tree).

## How it works

- `build.rs` walks zoneinfo, keeps files with the `TZif` magic, collapses
  identical contents to unique blobs, concatenates and deflate-compresses
  them, and generates a sorted `name → blob` index compiled into the binary.
- The tzdata release version is read from `tzdata.zi` and reported by
  `tztiny version`.
- At runtime the stream is decompressed once and the selected blobs are
  written with their original directory structure.
