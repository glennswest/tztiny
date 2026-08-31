// Scans the build host's zoneinfo tree, dedups identical TZif files,
// compresses the unique blobs into one deflate stream, and generates a
// sorted name -> blob index compiled into the binary.

use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|e| e.unwrap())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        // The right/ (leap-second) and posix/ trees duplicate every zone.
        if dir == base {
            let name = entry.file_name();
            if name == "right" || name == "posix" {
                continue;
            }
        }
        let meta = match fs::metadata(&path) {
            Ok(m) => m, // follows symlinks
            Err(_) => continue,
        };
        if meta.is_dir() {
            walk(&path, base, out);
        } else if meta.is_file() {
            let data = match fs::read(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            if data.len() >= 4 && &data[..4] == b"TZif" {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_str()
                    .expect("non-UTF8 zone name")
                    .to_string();
                out.push((rel, data));
            }
        }
    }
}

fn main() {
    let src = env::var("TZTINY_ZONEINFO").unwrap_or_else(|_| "/usr/share/zoneinfo".into());
    println!("cargo:rerun-if-env-changed=TZTINY_ZONEINFO");
    println!("cargo:rerun-if-changed={src}");
    let base = PathBuf::from(&src);

    let mut files = Vec::new();
    walk(&base, &base, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|a, b| a.0 == b.0);
    assert!(!files.is_empty(), "no TZif files found under {src}");

    // Dedup identical contents (hard-linked zones) into unique blobs.
    let mut seen: HashMap<Vec<u8>, u16> = HashMap::new();
    let mut blobs: Vec<(u32, u32)> = Vec::new();
    let mut raw: Vec<u8> = Vec::new();
    let mut names: Vec<(String, u16)> = Vec::new();
    for (name, data) in files {
        let idx = match seen.get(&data) {
            Some(&i) => i,
            None => {
                let i = u16::try_from(blobs.len()).expect("more than 65535 unique blobs");
                blobs.push((raw.len() as u32, data.len() as u32));
                raw.extend_from_slice(&data);
                seen.insert(data, i);
                i
            }
        };
        names.push((name, idx));
    }

    let compressed = miniz_oxide::deflate::compress_to_vec(&raw, 10);
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("tzdata.deflate"), &compressed).unwrap();

    // tzdata.zi begins with "# version 2025b" on most distros.
    let tz_version = fs::read_to_string(base.join("tzdata.zi"))
        .ok()
        .and_then(|s| {
            s.lines().next().map(|l| {
                l.trim_start_matches('#')
                    .trim()
                    .trim_start_matches("version")
                    .trim()
                    .to_string()
            })
        })
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "unknown".into());

    let mut idx = String::new();
    writeln!(idx, "pub const TZDATA_VERSION: &str = {tz_version:?};").unwrap();
    writeln!(idx, "pub const RAW_SIZE: usize = {};", raw.len()).unwrap();
    writeln!(idx, "pub static BLOBS: &[(u32, u32)] = &[").unwrap();
    for (off, len) in &blobs {
        writeln!(idx, "    ({off}, {len}),").unwrap();
    }
    writeln!(idx, "];").unwrap();
    writeln!(idx, "pub static NAMES: &[(&str, u16)] = &[").unwrap();
    for (name, blob) in &names {
        writeln!(idx, "    ({name:?}, {blob}),").unwrap();
    }
    writeln!(idx, "];").unwrap();
    fs::write(out_dir.join("index.rs"), idx).unwrap();
}
