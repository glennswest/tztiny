// tztiny — embeds the IANA timezone database (deduplicated, deflated) and
// writes out only the TZif entries a system needs.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::exit;

mod tzdata {
    include!(concat!(env!("OUT_DIR"), "/index.rs"));
    pub static DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tzdata.deflate"));
}

fn unpack() -> Vec<u8> {
    let raw = miniz_oxide::inflate::decompress_to_vec(tzdata::DATA)
        .unwrap_or_else(|_| die("corrupt embedded tzdata"));
    if raw.len() != tzdata::RAW_SIZE {
        die("embedded tzdata size mismatch");
    }
    raw
}

fn blob(raw: &[u8], idx: u16) -> &[u8] {
    let (off, len) = tzdata::BLOBS[idx as usize];
    &raw[off as usize..(off + len) as usize]
}

fn lookup(name: &str) -> Option<u16> {
    tzdata::NAMES
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|i| tzdata::NAMES[i].1)
}

// A pattern selects a zone by exact name, or a whole subtree by prefix
// ("America" or "America/" selects every America/* zone).
fn matches(pattern: &str, name: &str) -> bool {
    let p = pattern.trim_end_matches('/');
    name == p || (name.len() > p.len() && name.starts_with(p) && name.as_bytes()[p.len()] == b'/')
}

fn select(patterns: &[String]) -> Vec<(&'static str, u16)> {
    let mut out = Vec::new();
    for p in patterns {
        let mut hit = false;
        for &(name, idx) in tzdata::NAMES {
            if matches(p, name) {
                out.push((name, idx));
                hit = true;
            }
        }
        if !hit {
            die(&format!("unknown timezone: {p} (try: tztiny list)"));
        }
    }
    out.sort_by_key(|e| e.0);
    out.dedup_by_key(|e| e.0);
    out
}

fn write_file(path: &Path, data: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| die(&format!("mkdir {}: {e}", parent.display())));
    }
    fs::write(path, data).unwrap_or_else(|e| die(&format!("write {}: {e}", path.display())));
}

fn die(msg: &str) -> ! {
    eprintln!("tztiny: {msg}");
    exit(1);
}

fn usage() -> ! {
    eprint!(
        "tztiny {} (tzdata {}) — tiny timezone database installer\n\
         \n\
         usage:\n\
         \x20 tztiny list [PREFIX...]          list embedded zone names\n\
         \x20 tztiny install [-o DIR] ZONE...  write TZif files (default /usr/share/zoneinfo)\n\
         \x20 tztiny install [-o DIR] --all    write every zone\n\
         \x20 tztiny set [-o FILE] ZONE        write ZONE to /etc/localtime and /etc/timezone\n\
         \x20 tztiny cat ZONE                  write one TZif to stdout\n\
         \x20 tztiny version                   show versions and embedded size\n\
         \n\
         ZONE may be exact (America/Chicago) or a prefix (America) for a subtree.\n",
        env!("CARGO_PKG_VERSION"),
        tzdata::TZDATA_VERSION,
    );
    exit(2);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| usage());
    let rest: Vec<String> = args.collect();

    match cmd.as_str() {
        "list" => {
            for &(name, _) in tzdata::NAMES {
                if rest.is_empty() || rest.iter().any(|p| matches(p, name)) {
                    println!("{name}");
                }
            }
        }
        "install" => {
            let mut out_dir = PathBuf::from("/usr/share/zoneinfo");
            let mut patterns = Vec::new();
            let mut all = false;
            let mut it = rest.into_iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "-o" | "--output" => {
                        out_dir = PathBuf::from(it.next().unwrap_or_else(|| usage()))
                    }
                    "--all" => all = true,
                    _ if a.starts_with('-') => usage(),
                    _ => patterns.push(a),
                }
            }
            let zones: Vec<(&str, u16)> = if all {
                tzdata::NAMES.to_vec()
            } else if patterns.is_empty() {
                usage()
            } else {
                select(&patterns)
            };
            let raw = unpack();
            let mut bytes = 0usize;
            for &(name, idx) in &zones {
                let data = blob(&raw, idx);
                write_file(&out_dir.join(name), data);
                bytes += data.len();
            }
            eprintln!(
                "installed {} zone(s), {} bytes, into {}",
                zones.len(),
                bytes,
                out_dir.display()
            );
        }
        "set" => {
            let mut target = PathBuf::from("/etc/localtime");
            let mut zone = None;
            let mut it = rest.into_iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "-o" | "--output" => {
                        target = PathBuf::from(it.next().unwrap_or_else(|| usage()))
                    }
                    _ if a.starts_with('-') => usage(),
                    _ if zone.is_none() => zone = Some(a),
                    _ => usage(),
                }
            }
            let zone = zone.unwrap_or_else(|| usage());
            let idx = lookup(&zone)
                .unwrap_or_else(|| die(&format!("unknown timezone: {zone} (try: tztiny list)")));
            let raw = unpack();
            // /etc/localtime as a plain file: no zoneinfo tree needed at all.
            write_file(&target, blob(&raw, idx));
            if target == Path::new("/etc/localtime") {
                let mut tz = zone.clone().into_bytes();
                tz.push(b'\n');
                write_file(Path::new("/etc/timezone"), &tz);
            }
            eprintln!("set {} -> {zone}", target.display());
        }
        "cat" => {
            let zone = rest.first().cloned().unwrap_or_else(|| usage());
            let idx = lookup(&zone)
                .unwrap_or_else(|| die(&format!("unknown timezone: {zone} (try: tztiny list)")));
            let raw = unpack();
            std::io::stdout()
                .write_all(blob(&raw, idx))
                .unwrap_or_else(|e| die(&format!("stdout: {e}")));
        }
        "version" | "-V" | "--version" => {
            println!(
                "tztiny {} tzdata {} zones {} unique {} embedded {} bytes ({} raw)",
                env!("CARGO_PKG_VERSION"),
                tzdata::TZDATA_VERSION,
                tzdata::NAMES.len(),
                tzdata::BLOBS.len(),
                tzdata::DATA.len(),
                tzdata::RAW_SIZE,
            );
        }
        _ => usage(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_sorted_and_unique() {
        for w in tzdata::NAMES.windows(2) {
            assert!(w[0].0 < w[1].0, "{} !< {}", w[0].0, w[1].0);
        }
    }

    #[test]
    fn has_common_zones() {
        for z in ["UTC", "America/Chicago", "Europe/London", "Asia/Tokyo"] {
            assert!(lookup(z).is_some(), "missing {z}");
        }
        assert!(tzdata::NAMES.len() > 300);
    }

    #[test]
    fn blobs_are_tzif() {
        let raw = unpack();
        assert_eq!(raw.len(), tzdata::RAW_SIZE);
        for i in 0..tzdata::BLOBS.len() {
            assert_eq!(&blob(&raw, i as u16)[..4], b"TZif");
        }
    }

    #[test]
    fn prefix_matching() {
        assert!(matches("America", "America/Chicago"));
        assert!(matches("America/", "America/Chicago"));
        assert!(matches("America/Chicago", "America/Chicago"));
        assert!(!matches("America", "Americas/Nowhere"));
        // a pattern selects itself and its subtree
        assert!(matches("America/Chicago", "America/Chicago/Extra"));
        assert!(!matches("America/Chicago", "America/Chicag"));
        let sel = select(&["America/Argentina".to_string()]);
        assert!(sel.len() > 5);
        assert!(sel.iter().all(|(n, _)| n.starts_with("America/Argentina/")));
    }
}
