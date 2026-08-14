// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Package source fetchers — tarball, git, and local path.
//!
//! Each fetcher downloads or
//! copies a package source into a target directory; the caller then hands the
//! directory to [`cas::store`](crate::cas::ContentAddressableStore::store) for
//! integrity hashing. Archive extraction is hardened against zip-slip,
//! absolute paths, symlinks, and decompression bombs.

// These compare URL suffixes (already lowercased) and the `.git` marker, not
// real filesystem paths, so `Path::extension` would be the wrong tool.
#![allow(clippy::case_sensitive_file_extension_comparisons)]

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use flate2::read::GzDecoder;
use tcl_sandbox::{Profile, SandboxPolicy};

use crate::errors::TclPkgError;

/// Build a sandbox profile for the `git` tool: confined to `cwd`, output
/// captured, with only the environment git legitimately needs (PATH, HOME, and
/// proxy/TLS settings). Credentials in the environment (SSH agents, tokens) are
/// stripped by the sandbox's sensitive-name denylist.
fn git_profile(label: &str, git: &Path, cwd: &Path) -> Profile {
    let mut p = Profile::new(label, git, cwd);
    for name in [
        "PATH",
        "HOME",
        "TMPDIR",
        "http_proxy",
        "https_proxy",
        "no_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "ALL_PROXY",
        "GIT_SSL_CAINFO",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "CURL_CA_BUNDLE",
    ] {
        p = p.pass_env(name);
    }
    p
}

/// Maximum total uncompressed size from a single archive (256 MiB). Protects
/// against decompression bombs.
const MAX_EXTRACT_BYTES: u64 = 256 * 1024 * 1024;

fn fetch_error(message: impl Into<String>) -> TclPkgError {
    TclPkgError::fetch(message)
}

/// Download and extract a tarball or zip from `url` into `dest`.
///
/// Supports gzip tarballs (`.tar.gz` / `.tgz`), plain `.tar`, and `.zip`. A
/// singular top-level directory is stripped so files land directly in `dest`.
pub fn fetch_tarball(url: &str, dest: &Path, timeout: u64) -> Result<(), TclPkgError> {
    std::fs::create_dir_all(dest)
        .map_err(|e| fetch_error(format!("cannot create {}: {e}", dest.display())))?;
    let bytes = download(url, timeout)?;
    extract_archive(&bytes, dest, url)
}

/// Extract an archive already in memory into `dest`, exactly as
/// [`fetch_tarball`] extracts a downloaded one.
///
/// `name_hint` is the archive's URL or filename; only its extension is read, to
/// choose the zip or tar path (a gzip magic sniff covers an extensionless tar).
/// Both extractors are the hardened ones — zip-slip rejection, symlink and
/// hardlink skipping, and the 256 MiB uncompressed cap — and a singular
/// top-level directory is stripped the same way, so a caller that already holds
/// the bytes (a local `.zip` snapshot, say) gets the identical guarantees
/// without a second implementation. Added for `tcl spec import`, which reads
/// release archives off disk as well as off the network.
pub fn extract_archive(bytes: &[u8], dest: &Path, name_hint: &str) -> Result<(), TclPkgError> {
    std::fs::create_dir_all(dest)
        .map_err(|e| fetch_error(format!("cannot create {}: {e}", dest.display())))?;

    let staging = dest.join(".staging");
    std::fs::create_dir_all(&staging)
        .map_err(|e| fetch_error(format!("cannot create staging dir: {e}")))?;

    let lower = name_hint.to_lowercase();
    if lower.ends_with(".zip") {
        extract_zip(bytes, &staging)?;
    } else {
        extract_tar(bytes, &staging, &lower)?;
    }

    strip_top_level(&staging, dest)
}

fn download(url: &str, timeout: u64) -> Result<Vec<u8>, TclPkgError> {
    fetch_bytes(url, timeout, &[])
}

/// GET `url` and return the body, capped at the same 256 MiB an archive is.
///
/// The transport every fetch in this module shares: ureq with a global timeout,
/// HTTP status checked explicitly, and the proxy taken from the standard
/// environment variables ureq reads (`HTTPS_PROXY`, `NO_PROXY`, …). `headers`
/// adds request headers to the built-in `User-Agent` — an API that demands its
/// own agent string or an `Authorization:` needs them, and routing such a call
/// through here keeps it on the one hardened path.
pub fn fetch_bytes(
    url: &str,
    timeout: u64,
    headers: &[(&str, &str)],
) -> Result<Vec<u8>, TclPkgError> {
    let agent = ureq::config::Config::builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(timeout)))
        .build()
        .new_agent();
    let mut request = agent.get(url).header("User-Agent", "tclpkg/0.1");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut resp = request
        .call()
        .map_err(|e| fetch_error(format!("failed to fetch {url}: {e}")))?;
    if resp.status().as_u16() >= 400 {
        return Err(fetch_error(format!(
            "failed to fetch {url}: HTTP {}",
            resp.status().as_u16()
        )));
    }
    resp.body_mut()
        .with_config()
        .limit(MAX_EXTRACT_BYTES)
        .read_to_vec()
        .map_err(|e| fetch_error(format!("failed to read {url}: {e}")))
}

/// Validate that `name` resolves inside `dest_root`, returning the safe target.
fn safe_join(dest_root: &Path, name: &str) -> Result<PathBuf, TclPkgError> {
    let candidate = Path::new(name);
    if candidate.is_absolute() {
        return Err(fetch_error(format!(
            "archive member has absolute path: {name}"
        )));
    }
    if candidate
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(fetch_error(format!(
            "archive member escapes target directory: {name}"
        )));
    }
    Ok(dest_root.join(candidate))
}

fn extract_tar(bytes: &[u8], dest: &Path, lower: &str) -> Result<(), TclPkgError> {
    let is_gzip =
        lower.ends_with(".gz") || lower.ends_with(".tgz") || bytes.starts_with(&[0x1f, 0x8b]);
    if lower.ends_with(".bz2") || lower.ends_with(".xz") {
        return Err(fetch_error(
            "bzip2/xz tarballs are not supported by this build; use .tar.gz or .zip",
        ));
    }
    let reader: Box<dyn Read> = if is_gzip {
        Box::new(GzDecoder::new(Cursor::new(bytes)))
    } else {
        Box::new(Cursor::new(bytes))
    };
    let mut archive = tar::Archive::new(reader);
    let mut total: u64 = 0;
    for entry in archive
        .entries()
        .map_err(|e| fetch_error(format!("cannot read tar archive: {e}")))?
    {
        let mut entry = entry.map_err(|e| fetch_error(format!("corrupt tar entry: {e}")))?;
        let entry_type = entry.header().entry_type();
        // Skip symlinks and hardlinks.
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| fetch_error(format!("invalid tar path: {e}")))?
            .to_string_lossy()
            .into_owned();
        let target = safe_join(dest, &path)?;
        if entry_type.is_dir() {
            std::fs::create_dir_all(&target)
                .map_err(|e| fetch_error(format!("cannot create dir: {e}")))?;
            continue;
        }
        total = total.saturating_add(entry.header().size().unwrap_or(0));
        if total > MAX_EXTRACT_BYTES {
            return Err(fetch_error(
                "tar archive exceeds 256 MiB uncompressed limit (possible bomb)",
            ));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| fetch_error(format!("cannot create dir: {e}")))?;
        }
        let mut out = std::fs::File::create(&target)
            .map_err(|e| fetch_error(format!("cannot write {}: {e}", target.display())))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| fetch_error(format!("cannot extract {}: {e}", target.display())))?;
    }
    Ok(())
}

fn extract_zip(bytes: &[u8], dest: &Path) -> Result<(), TclPkgError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| fetch_error(format!("cannot read zip archive: {e}")))?;
    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| fetch_error(format!("corrupt zip entry: {e}")))?;
        let Some(name) = file.enclosed_name() else {
            return Err(fetch_error(format!(
                "zip member escapes target directory: {}",
                file.name()
            )));
        };
        let name_str = name.to_string_lossy().into_owned();
        let target = safe_join(dest, &name_str)?;
        // Skip symlinks (Unix mode S_IFLNK in the upper bits).
        if let Some(mode) = file.unix_mode()
            && mode & 0o170_000 == 0o120_000
        {
            continue;
        }
        if file.is_dir() {
            std::fs::create_dir_all(&target)
                .map_err(|e| fetch_error(format!("cannot create dir: {e}")))?;
            continue;
        }
        total = total.saturating_add(file.size());
        if total > MAX_EXTRACT_BYTES {
            return Err(fetch_error(
                "zip archive exceeds 256 MiB uncompressed limit (possible zip bomb)",
            ));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| fetch_error(format!("cannot create dir: {e}")))?;
        }
        let mut out = std::fs::File::create(&target)
            .map_err(|e| fetch_error(format!("cannot write {}: {e}", target.display())))?;
        std::io::copy(&mut file, &mut out)
            .map_err(|e| fetch_error(format!("cannot extract {}: {e}", target.display())))?;
    }
    Ok(())
}

/// Move the contents of `staging` into `dest`, stripping a singular top-level
/// directory (the common archive convention).
fn strip_top_level(staging: &Path, dest: &Path) -> Result<(), TclPkgError> {
    let children: Vec<PathBuf> = std::fs::read_dir(staging)
        .map_err(|e| fetch_error(format!("cannot read staging dir: {e}")))?
        .flatten()
        .map(|e| e.path())
        .collect();
    let source = if children.len() == 1 && children[0].is_dir() {
        children[0].clone()
    } else {
        staging.to_path_buf()
    };
    for entry in std::fs::read_dir(&source)
        .map_err(|e| fetch_error(format!("cannot read extracted dir: {e}")))?
        .flatten()
    {
        let from = entry.path();
        let to = dest.join(entry.file_name());
        move_path(&from, &to)?;
    }
    let _ = std::fs::remove_dir_all(staging);
    Ok(())
}

fn move_path(from: &Path, to: &Path) -> Result<(), TclPkgError> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // Cross-device fallback: copy then remove.
    if from.is_dir() {
        copy_dir(from, to).map_err(|e| fetch_error(format!("cannot move dir: {e}")))?;
        let _ = std::fs::remove_dir_all(from);
    } else {
        std::fs::copy(from, to).map_err(|e| fetch_error(format!("cannot move file: {e}")))?;
        let _ = std::fs::remove_file(from);
    }
    Ok(())
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

/// Clone a git repository into `dest` and return the resolved commit SHA.
///
/// `rev` is a branch or tag (passed to `git clone --branch`). The `.git/`
/// directory is removed afterwards so the worktree hashes cleanly.
pub fn fetch_git(url: &str, dest: &Path, rev: Option<&str>) -> Result<String, TclPkgError> {
    std::fs::create_dir_all(dest)
        .map_err(|e| fetch_error(format!("cannot create {}: {e}", dest.display())))?;

    let mut url = url.to_string();
    let mut rev = rev.map(ToString::to_string);
    if let Some(stripped) = url.strip_prefix("git+") {
        url = stripped.to_string();
    }
    // Strip an @rev suffix when it follows a path segment or `.git`, but not an
    // `ssh user@host` prefix.
    if rev.is_none()
        && let Some(last_at) = url.rfind('@')
    {
        let prefix = &url[..last_at];
        if prefix.contains('/') || prefix.ends_with(".git") {
            rev = Some(url[last_at + 1..].to_string());
            url = prefix.to_string();
        }
    }

    // Resolve git's absolute path: the sandbox clears the environment, so a bare
    // `git` would not be found via PATH lookup at spawn time.
    let git = crate::venv::which("git").unwrap_or_else(|| std::path::PathBuf::from("git"));
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));

    let mut clone_args: Vec<String> = vec!["clone".into(), "--depth".into(), "1".into()];
    if let Some(r) = &rev {
        clone_args.push("--branch".into());
        clone_args.push(r.clone());
    }
    clone_args.push(url.clone());
    clone_args.push(dest.to_string_lossy().into_owned());

    // git fetches over the network, so this is the one trusted tool that runs
    // with network granted. The environment is still scrubbed of credentials
    // (SSH_*, *_TOKEN, …); only PATH, HOME and proxy/TLS settings pass through.
    let profile = git_profile("git-clone", &git, parent)
        .args(clone_args)
        .network(true);
    let outcome = crate::exec::execute(&profile, &SandboxPolicy::default())?;
    if !outcome.success {
        let stderr = String::from_utf8_lossy(&outcome.stderr).trim().to_string();
        return Err(fetch_error(format!("git clone failed: {stderr}")));
    }

    let rev_profile = git_profile("git-rev-parse", &git, dest)
        .args(["rev-parse".to_string(), "HEAD".to_string()]);
    let sha = crate::exec::execute(&rev_profile, &SandboxPolicy::default())
        .ok()
        .filter(|o| o.success)
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let git_dir = dest.join(".git");
    if git_dir.is_dir() {
        let _ = std::fs::remove_dir_all(&git_dir);
    }
    Ok(sha)
}

/// Copy a local directory into `dest` (used by `replace` directives pointing at
/// a local checkout).
pub fn fetch_path(source: &str, dest: &Path) -> Result<(), TclPkgError> {
    let src = Path::new(source);
    if !src.is_dir() {
        return Err(fetch_error(format!("local path not found: {source}")));
    }
    std::fs::create_dir_all(dest)
        .map_err(|e| fetch_error(format!("cannot create {}: {e}", dest.display())))?;
    copy_dir(src, dest).map_err(|e| fetch_error(format!("cannot copy local path: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_rejects_traversal() {
        let root = Path::new("/tmp/x");
        assert!(safe_join(root, "../evil").is_err());
        assert!(safe_join(root, "/etc/passwd").is_err());
        assert!(safe_join(root, "ok/file.tcl").is_ok());
    }

    #[test]
    fn path_fetch_copies() {
        let base = std::env::temp_dir().join(format!(
            "tclpkg-fetch-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.tcl"), "set x 1\n").unwrap();
        let dest = base.join("dest");
        fetch_path(src.to_str().unwrap(), &dest).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("a.tcl")).unwrap(),
            "set x 1\n"
        );
        assert!(fetch_path("/no/such/dir/xyz", &base.join("d2")).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn extract_tar_strips_top_level() {
        // Build a gzip tarball in memory with a singular top dir.
        use flate2::Compression;
        use flate2::write::GzEncoder;
        let mut tar_buf = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_buf);
            let data = b"hello\n";
            let mut header = tar::Header::new_gnu();
            header.set_path("pkg-1.0/a.tcl").unwrap();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, &data[..]).unwrap();
            builder.finish().unwrap();
        }
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        std::io::Write::write_all(&mut gz, &tar_buf).unwrap();
        let gz_bytes = gz.finish().unwrap();

        let base = std::env::temp_dir().join(format!(
            "tclpkg-tar-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let staging = base.join(".staging");
        std::fs::create_dir_all(&staging).unwrap();
        extract_tar(&gz_bytes, &staging, ".tar.gz").unwrap();
        strip_top_level(&staging, &base).unwrap();
        assert_eq!(
            std::fs::read_to_string(base.join("a.tcl")).unwrap(),
            "hello\n"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
