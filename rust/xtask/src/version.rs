//! Project version.
//!
//! Prints the version setuptools-scm / hatch-vcs would produce for the
//! current tree (the version `uv build` stamps into the wheel name),
//! computed from `git describe`. Implements setuptools-scm's default
//! "guess-next-dev" version scheme with `local_scheme=no-local-version`:
//!
//! - exactly on a tag      → the tag, with any leading `v` stripped;
//! - N commits past a tag  → `guess_next(tag).devN` (last component + 1);
//! - no tags in history    → `0.0.0.dev0`, the Python script's stable
//!   fallback (matches `print_version.py`'s `LookupError` branch).
//!
//! The expected outputs were captured from real `setuptools_scm`
//! (`local_scheme="no-local-version"`) and are pinned by the unit tests.

use std::process::{Command, ExitCode};

use crate::util::repo_root;

/// Print the computed project version.
pub fn run() -> ExitCode {
    println!("{}", version_from_describe(git_describe().as_deref()));
    ExitCode::SUCCESS
}

/// `git describe --tags --long` for the repository, or `None` when the
/// command fails or the tree has no tags.
fn git_describe() -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root())
        .args(["describe", "--tags", "--long"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let described = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if described.is_empty() {
        None
    } else {
        Some(described)
    }
}

/// Map a `git describe --tags --long` string (`<tag>-<distance>-g<hash>`)
/// to the setuptools-scm version. `None` (no tags) yields the stable
/// `0.0.0.dev0` fallback.
fn version_from_describe(describe: Option<&str>) -> String {
    let Some(described) = describe else {
        return "0.0.0.dev0".to_owned();
    };
    // Peel the trailing `-g<hash>` and `-<distance>` fields off the right;
    // whatever remains is the tag (which may itself contain `-`).
    let Some((without_hash, _ghash)) = described.rsplit_once('-') else {
        return "0.0.0.dev0".to_owned();
    };
    let Some((tag, distance)) = without_hash.rsplit_once('-') else {
        return "0.0.0.dev0".to_owned();
    };
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    let distance: u32 = distance.parse().unwrap_or(0);
    if distance == 0 {
        tag.to_owned()
    } else {
        format!("{}.dev{distance}", guess_next(tag))
    }
}

/// setuptools-scm "guess-next-dev": bump the last dot-separated numeric
/// component (`1.2.3` → `1.2.4`). A non-numeric tail is left unchanged —
/// the project's release tags are simple `X.Y.Z` semver.
fn guess_next(tag: &str) -> String {
    let mut parts: Vec<String> = tag.split('.').map(String::from).collect();
    if let Some(last) = parts.last_mut()
        && let Ok(n) = last.parse::<u64>()
    {
        *last = (n + 1).to_string();
    }
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_setuptools_scm_outputs() {
        // Pinned against real `setuptools_scm.get_version(
        // local_scheme="no-local-version")` over temp git repos.
        assert_eq!(version_from_describe(None), "0.0.0.dev0");
        assert_eq!(version_from_describe(Some("v1.2.3-0-g21cedcf")), "1.2.3");
        assert_eq!(
            version_from_describe(Some("v1.2.3-3-g15e9b43")),
            "1.2.4.dev3"
        );
        assert_eq!(
            version_from_describe(Some("2.0.0-1-ga3e03bb")),
            "2.0.1.dev1"
        );
    }

    #[test]
    fn guess_next_bumps_last_numeric_component() {
        assert_eq!(guess_next("1.2.3"), "1.2.4");
        assert_eq!(guess_next("2.0.0"), "2.0.1");
        assert_eq!(guess_next("1.10.5"), "1.10.6");
    }
}
