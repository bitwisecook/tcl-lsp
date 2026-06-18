//! Pure path-string helpers (`file tail`/`dirname`/`extension`/`rootname`).
//!
//! Byte→byte operations on `/`-separated paths. Tcl uses `/` as the separator on
//! every platform, so these are deliberately platform-independent (the VM
//! previously used `std::path::Path`, whose separators are OS-dependent). Pure —
//! no [`ValueOps`], no host — so each runtime hands in the path bytes and builds
//! its own result from the returned slice. Semantics follow `tclsh`'s
//! `tclFileName.c`.
//!
//! [`ValueOps`]: tcl_syntax::value::ValueOps

/// Trim trailing `/` separators, keeping at least one byte (`///` → `/`), so a
/// trailing slash is never treated as a component.
fn trim_trailing(p: &[u8]) -> &[u8] {
    let mut end = p.len();
    while end > 1 && p[end - 1] == b'/' {
        end -= 1;
    }
    &p[..end]
}

/// `file tail path` — the last `/`-separated component (trailing slashes ignored,
/// so `/a/b/` → `b`).
#[must_use]
pub fn tail(path: &[u8]) -> &[u8] {
    let p = trim_trailing(path);
    match p.iter().rposition(|&c| c == b'/') {
        Some(i) => &p[i + 1..],
        None => p,
    }
}

/// `file dirname path` — everything before the last component: no slash → `.`;
/// the filesystem root's parent → `/`; a trailing slash is not a component.
#[must_use]
pub fn dirname(path: &[u8]) -> &[u8] {
    let p = trim_trailing(path);
    match p.iter().rposition(|&c| c == b'/') {
        Some(0) => b"/",
        Some(i) => &p[..i],
        None => b".",
    }
}

/// `file extension path` — the final `.` of the tail through the end (including
/// the dot), or `""` when the tail has no interior dot. A leading-dot tail
/// (`.bashrc`) has no extension, matching `tclsh`.
#[must_use]
pub fn extension(path: &[u8]) -> &[u8] {
    let t = tail(path);
    match t.iter().rposition(|&c| c == b'.') {
        Some(i) if i > 0 => &t[i..],
        _ => b"",
    }
}

/// `file rootname path` — `path` with its [`extension`] removed.
#[must_use]
pub fn rootname(path: &[u8]) -> &[u8] {
    let ext = extension(path);
    &path[..path.len() - ext.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_cases() {
        assert_eq!(tail(b"/a/b/c"), b"c");
        assert_eq!(tail(b"foo"), b"foo");
        assert_eq!(tail(b"a/b.txt"), b"b.txt");
        // Trailing slashes are ignored (the old str helper wrongly returned "").
        assert_eq!(tail(b"/a/b/"), b"b");
    }

    #[test]
    fn dirname_cases() {
        assert_eq!(dirname(b"/a/b/c"), b"/a/b");
        assert_eq!(dirname(b"/foo"), b"/");
        assert_eq!(dirname(b"foo"), b".");
        assert_eq!(dirname(b"a/b"), b"a");
        assert_eq!(dirname(b"/a/b/"), b"/a");
    }

    #[test]
    fn extension_and_rootname() {
        assert_eq!(extension(b"a/b.txt"), b".txt");
        assert_eq!(extension(b".bashrc"), b"");
        assert_eq!(extension(b"noext"), b"");
        assert_eq!(rootname(b"a/b.txt"), b"a/b");
        assert_eq!(rootname(b"noext"), b"noext");
    }
}
