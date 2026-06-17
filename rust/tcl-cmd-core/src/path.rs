//! Pure path-string helpers (`file tail`/`dirname`/`extension`/`rootname`).
//!
//! These are P0 — pure character manipulation, no host access — so they take and
//! return plain `&str`/`String` and need neither [`ValueOps`] nor a [`Host`].
//! The actual filesystem touches (`file exists`, `glob`, `open`) live in
//! [`crate::platform`]. Semantics follow `tclsh`'s `tclFileName.c`; the exact
//! edge cases are pinned against the C Tcl 9 suite as the family fills in.
//!
//! [`ValueOps`]: tcl_syntax::value::ValueOps
//! [`Host`]: tcl_platform::Host

/// `file tail path` — the last component (everything after the final `/`).
#[must_use]
pub fn tail(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// `file dirname path` — everything before the last component. Mirrors `tclsh`:
/// no slash → `.`; a leading-slash-only parent → `/`; trailing slashes are not
/// treated as a component.
#[must_use]
pub fn dirname(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        // Parent is the filesystem root.
        Some(0) => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
        None => ".".to_string(),
    }
}

/// `file extension path` — the final `.` of the tail through the end (including
/// the dot), or `""` when the tail has no interior dot. A leading-dot tail
/// (`.bashrc`) has no extension, matching `tclsh`.
#[must_use]
pub fn extension(path: &str) -> &str {
    let t = tail(path);
    match t.rfind('.') {
        Some(0) | None => "",
        Some(i) => &t[i..],
    }
}

/// `file rootname path` — `path` with [`extension`] removed.
#[must_use]
pub fn rootname(path: &str) -> String {
    let ext = extension(path);
    if ext.is_empty() {
        path.to_string()
    } else {
        path[..path.len() - ext.len()].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_cases() {
        assert_eq!(tail("/a/b/c"), "c");
        assert_eq!(tail("foo"), "foo");
        assert_eq!(tail("a/b.txt"), "b.txt");
    }

    #[test]
    fn dirname_cases() {
        assert_eq!(dirname("/a/b/c"), "/a/b");
        assert_eq!(dirname("/foo"), "/");
        assert_eq!(dirname("foo"), ".");
        assert_eq!(dirname("a/b"), "a");
    }

    #[test]
    fn extension_and_rootname() {
        assert_eq!(extension("a/b.txt"), ".txt");
        assert_eq!(extension(".bashrc"), "");
        assert_eq!(extension("noext"), "");
        assert_eq!(rootname("a/b.txt"), "a/b");
        assert_eq!(rootname("noext"), "noext");
    }
}
