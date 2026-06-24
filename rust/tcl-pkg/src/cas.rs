//! Content-addressable store (CAS) and integrity hashing.
//!
//! The CAS lives at
//! `<cache_dir>/tclpkg/cas/sha256/<ab>/<full_hash>/tree/`; each entry is
//! immutable once written. The integrity string `sha256-<base64url-no-pad>` is
//! computed over a *canonicalised worktree* (sorted paths, stripped VCS dirs,
//! permission-masked, timestamps ignored) so a re-compressed archive hashes
//! identically.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

use crate::errors::TclPkgError;

const IGNORED_NAMES: &[&str] = &[".git", ".hg", ".svn", ".fossil", ".DS_Store", "Thumbs.db"];

fn is_ignored(name: &str, extra: &[String]) -> bool {
    IGNORED_NAMES.contains(&name) || extra.iter().any(|p| p == name)
}

fn load_tclpkgignore(root: &Path) -> Vec<String> {
    let ignore_file = root.join(".tclpkgignore");
    let Ok(text) = std::fs::read_to_string(&ignore_file) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToString::to_string)
        .collect()
}

/// Return sorted relative file paths under `root`, pruning ignored directories
/// and files. Directory symlinks are not followed.
fn walk_tree(root: &Path, extra_ignore: &[String]) -> Vec<PathBuf> {
    let mut result = Vec::new();
    collect_files(root, root, extra_ignore, &mut result);
    // Sort by POSIX path bytes for cross-machine determinism.
    result.sort_by_key(|a| posix(a).into_bytes());
    result
}

fn collect_files(root: &Path, dir: &Path, extra: &[String], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_ignored(&name, extra) {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            // followlinks=False: a directory symlink is neither recursed nor
            // emitted as a file.
            collect_files(root, &path, extra, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
}

fn posix(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Compute the canonical SHA-256 integrity hash of a package worktree.
///
/// Returns `sha256-<base64url_no_pad>`, stable across machines: only permission
/// bits (with execute normalised) and file content matter.
#[must_use]
pub fn integrity_of_tree(root: &Path) -> String {
    let extra = load_tclpkgignore(root);
    let mut hasher = Sha256::new();
    for rel_path in walk_tree(root, &extra) {
        let abs_path = root.join(&rel_path);
        let Ok(meta) = std::fs::metadata(&abs_path) else {
            continue;
        };
        let mut mode = file_mode(&meta);
        if mode & 0o111 != 0 {
            mode |= 0o111;
        }
        let Ok(content) = std::fs::read(&abs_path) else {
            continue;
        };
        let file_hash = hex_digest(&content);
        hasher.update(posix(&rel_path).into_bytes());
        hasher.update(b"\n");
        hasher.update(format!("{mode:o}\n").into_bytes());
        hasher.update(format!("{}\n", content.len()).into_bytes());
        hasher.update(file_hash.into_bytes());
        hasher.update(b"\n");
        hasher.update(&content);
    }
    let digest = hasher.finalize();
    format!("sha256-{}", URL_SAFE_NO_PAD.encode(digest))
}

#[cfg(unix)]
fn file_mode(meta: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    meta.mode() & 0o7777
}

#[cfg(not(unix))]
fn file_mode(_meta: &std::fs::Metadata) -> u32 {
    // Non-Unix hosts have no POSIX mode; treat files as 0o644 for a stable
    // cross-platform hash (matches the common case on the canonical build host).
    0o644
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(s, "{byte:02x}");
    }
    s
}

fn hex_digest(data: &[u8]) -> String {
    to_hex(&Sha256::digest(data))
}

/// Return the total byte size of the files in a canonical worktree.
///
/// Walks the same file set as [`integrity_of_tree`] and sums file lengths, so
/// the lockfile `size` field is deterministic.
#[must_use]
pub fn tree_size(root: &Path) -> u64 {
    let extra = load_tclpkgignore(root);
    let mut total = 0u64;
    for rel_path in walk_tree(root, &extra) {
        if let Ok(meta) = std::fs::metadata(root.join(&rel_path)) {
            total = total.saturating_add(meta.len());
        }
    }
    total
}

/// Recompute the integrity hash and return whether it matches `expected`.
#[must_use]
pub fn verify_integrity(root: &Path, expected: &str) -> bool {
    integrity_of_tree(root) == expected
}

/// Manage the `sha256/<ab>/<hash>/tree/` CAS on disk.
pub struct ContentAddressableStore {
    base: PathBuf,
}

impl ContentAddressableStore {
    /// Create a store rooted at `cache_dir` (the `tclpkg/cas/sha256` subtree).
    #[must_use]
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            base: cache_dir.join("tclpkg").join("cas").join("sha256"),
        }
    }

    fn entry_dir(&self, integrity: &str) -> Result<PathBuf, TclPkgError> {
        let Some(b64) = integrity.strip_prefix("sha256-") else {
            return Err(TclPkgError::integrity(
                format!("unsupported integrity format: {integrity}"),
                None,
                None,
                None,
            ));
        };
        let raw = URL_SAFE_NO_PAD.decode(b64).map_err(|_| {
            TclPkgError::integrity(
                format!("malformed integrity hash: {integrity}"),
                None,
                None,
                Some(
                    "the lockfile may be corrupted; delete tclpkg.lock and re-run \
                     'tcl pkg install'"
                        .to_string(),
                ),
            )
        })?;
        let hex = to_hex(&raw);
        let shard = &hex[..2.min(hex.len())];
        Ok(self.base.join(shard).join(&hex))
    }

    /// Whether the CAS contains an entry for `integrity` (false on any decode
    /// error — callers guard against empty/malformed integrity strings).
    #[must_use]
    pub fn has(&self, integrity: &str) -> bool {
        self.entry_dir(integrity)
            .is_ok_and(|e| e.join("tree").is_dir())
    }

    /// Return the `tree/` directory for an existing CAS entry.
    pub fn tree_path(&self, integrity: &str) -> Result<PathBuf, TclPkgError> {
        let tree = self.entry_dir(integrity)?.join("tree");
        if tree.is_dir() {
            Ok(tree)
        } else {
            Err(TclPkgError::integrity(
                format!("CAS entry missing for {integrity}"),
                None,
                None,
                Some("run 'tcl pkg install' to populate the cache".to_string()),
            ))
        }
    }

    /// Copy a worktree into the CAS and return its integrity string.
    pub fn store(
        &self,
        source_dir: &Path,
        name: &str,
        version: &str,
        expected_integrity: Option<&str>,
    ) -> Result<String, TclPkgError> {
        let actual = integrity_of_tree(source_dir);
        if let Some(expected) = expected_integrity
            && actual != expected
        {
            return Err(TclPkgError::integrity(
                format!("integrity mismatch for {name}@{version}"),
                Some(expected),
                Some(&actual),
                Some("the download may be corrupted; delete the cache and retry".to_string()),
            ));
        }
        let entry = self.entry_dir(&actual)?;
        let tree = entry.join("tree");
        if tree.is_dir() {
            return Ok(actual); // already cached
        }
        std::fs::create_dir_all(&entry)
            .map_err(|e| TclPkgError::new(format!("cannot create CAS entry: {e}")))?;
        if !name.is_empty() {
            let _ = std::fs::write(entry.join("package.txt"), format!("{name} {version}\n"));
        }
        copy_dir(source_dir, &tree)
            .map_err(|e| TclPkgError::new(format!("cannot populate CAS: {e}")))?;
        Ok(actual)
    }

    /// Create `dest` as a symlink (or copy) pointing into the CAS.
    pub fn materialise(
        &self,
        integrity: &str,
        dest: &Path,
        symlink: bool,
    ) -> Result<(), TclPkgError> {
        let tree = self.tree_path(integrity)?;
        if dest.exists() || dest.is_symlink() {
            if dest.is_symlink() || dest.is_file() {
                let _ = std::fs::remove_file(dest);
            } else if dest.is_dir() {
                let _ = std::fs::remove_dir_all(dest);
            }
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                TclPkgError::new(format!("cannot create {}: {e}", parent.display()))
            })?;
        }
        if symlink && try_symlink(&tree, dest) {
            return Ok(());
        }
        copy_dir(&tree, dest)
            .map_err(|e| TclPkgError::new(format!("cannot materialise package: {e}")))
    }
}

#[cfg(unix)]
fn try_symlink(target: &Path, dest: &Path) -> bool {
    std::os::unix::fs::symlink(target, dest).is_ok()
}

#[cfg(not(unix))]
fn try_symlink(_target: &Path, _dest: &Path) -> bool {
    false
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let mut p = std::env::temp_dir();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            p.push(format!("tclpkg-cas-{tag}-{nanos}"));
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn make_tree(root: &Path, files: &[(&str, &str)]) -> PathBuf {
        let tree = root.join("tree");
        fs::create_dir_all(&tree).unwrap();
        for (rel, content) in files {
            let p = tree.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, content).unwrap();
        }
        tree
    }

    #[test]
    fn stable_and_prefixed() {
        let tmp = TempDir::new("stable");
        let root = make_tree(
            tmp.path(),
            &[("a.tcl", "set x 1\n"), ("b.tcl", "set y 2\n")],
        );
        let a = integrity_of_tree(&root);
        assert!(a.starts_with("sha256-"));
        assert_eq!(a, integrity_of_tree(&root));
    }

    #[test]
    fn content_sensitivity() {
        let t1 = TempDir::new("c1");
        let t2 = TempDir::new("c2");
        let r1 = make_tree(t1.path(), &[("a.tcl", "set x 1\n")]);
        let r2 = make_tree(t2.path(), &[("a.tcl", "set x 2\n")]);
        let r3 = make_tree(&t2.path().join("same"), &[("a.tcl", "set x 1\n")]);
        assert_ne!(integrity_of_tree(&r1), integrity_of_tree(&r2));
        assert_eq!(integrity_of_tree(&r1), integrity_of_tree(&r3));
    }

    #[test]
    fn ignores_vcs_and_ds_store() {
        let t1 = TempDir::new("i1");
        let root = make_tree(t1.path(), &[("a.tcl", "ok\n"), (".DS_Store", "binary")]);
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        let t2 = TempDir::new("i2");
        let clean = make_tree(t2.path(), &[("a.tcl", "ok\n")]);
        assert_eq!(integrity_of_tree(&root), integrity_of_tree(&clean));
    }

    #[test]
    fn tclpkgignore_honoured() {
        let t1 = TempDir::new("ti1");
        let root = make_tree(
            t1.path(),
            &[
                ("a.tcl", "ok\n"),
                ("build", "junk\n"),
                (".tclpkgignore", "build\n"),
            ],
        );
        let t2 = TempDir::new("ti2");
        let clean = make_tree(
            t2.path(),
            &[("a.tcl", "ok\n"), (".tclpkgignore", "build\n")],
        );
        assert_eq!(integrity_of_tree(&root), integrity_of_tree(&clean));
    }

    #[test]
    fn verify() {
        let tmp = TempDir::new("verify");
        let root = make_tree(tmp.path(), &[("a.tcl", "set x 1\n")]);
        let expected = integrity_of_tree(&root);
        assert!(verify_integrity(&root, &expected));
        assert!(!verify_integrity(&root, "sha256-WRONG"));
    }

    #[test]
    fn store_and_materialise() {
        let tmp = TempDir::new("store");
        let cas = ContentAddressableStore::new(&tmp.path().join("cache"));
        let root = make_tree(tmp.path(), &[("a.tcl", "hello\n")]);
        let integrity = cas.store(&root, "pkg", "1.0", None).unwrap();
        assert!(integrity.starts_with("sha256-"));
        assert!(cas.has(&integrity));
        // idempotent
        assert_eq!(integrity, cas.store(&root, "pkg", "1.0", None).unwrap());

        let dest = tmp.path().join("lib").join("pkg-1.0");
        cas.materialise(&integrity, &dest, false).unwrap();
        assert!(dest.is_dir());
        assert_eq!(fs::read_to_string(dest.join("a.tcl")).unwrap(), "hello\n");
    }

    #[test]
    fn store_rejects_wrong_expected() {
        let tmp = TempDir::new("expect");
        let cas = ContentAddressableStore::new(&tmp.path().join("cache"));
        let root = make_tree(tmp.path(), &[("a.tcl", "set x 1\n")]);
        let err = cas
            .store(&root, "a", "1.0", Some("sha256-WRONG"))
            .unwrap_err();
        assert!(err.to_string().contains("mismatch"));
    }

    #[test]
    fn tree_path_missing() {
        let tmp = TempDir::new("missing");
        let cas = ContentAddressableStore::new(&tmp.path().join("cache"));
        let err = cas
            .tree_path("sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
            .unwrap_err();
        assert!(err.to_string().contains("missing"));
    }
}
