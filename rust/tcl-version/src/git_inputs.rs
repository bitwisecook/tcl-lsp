// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Git metadata inputs that can change the version embedded by `build.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve every repository input that can move the version stamp.
///
/// `../../.git/...` is correct only in the primary checkout. In a linked
/// worktree `.git` is a file pointing into the common repository, so those
/// literal children do not exist. Cargo treats a missing `rerun-if-changed`
/// path as perpetually dirty and rebuilds this crate (and every dependent
/// binary) before every command. `git rev-parse --git-path` knows both layouts.
pub(crate) fn dependency_paths(manifest_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // HEAD covers detached checkouts and changes between symbolic/detached
    // state. The exact symbolic ref covers ordinary commits, because HEAD's
    // own contents stay `ref: refs/heads/<branch>` while that branch advances.
    for name in ["HEAD", "refs/tags", "packed-refs"] {
        if let Some(path) = git_path(manifest_dir, name) {
            paths.push(path);
        }
    }

    if let Some(symbolic_ref) = git_output(manifest_dir, &["symbolic-ref", "-q", "HEAD"]) {
        // Do not watch the whole refs/heads directory: in a shared repository,
        // commits on unrelated worktree branches would then rebuild this crate.
        // The exact loose ref is the primary input; its reflog also exists for
        // ordinary non-bare repositories and covers a currently packed ref.
        for name in [&symbolic_ref, &format!("logs/{symbolic_ref}")] {
            if let Some(path) = git_path(manifest_dir, name) {
                paths.push(path);
            }
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

/// A deliberately absent Cargo input used to recompute unstamped provenance.
///
/// Cargo's change detector cannot represent every state that `git status`
/// observes (notably chmod-only changes and missing root-level files). Keeping
/// the probe under Cargo's private output directory avoids watching the source
/// or target directory recursively. The build script never creates it, so an
/// unstamped build always re-evaluates the working-tree state.
pub(crate) fn worktree_probe_path(out_dir: &Path) -> PathBuf {
    out_dir.join("tcl-version-worktree-state-probe")
}

fn git_path(manifest_dir: &Path, name: &str) -> Option<PathBuf> {
    let path = git_output_path(manifest_dir, &["rev-parse", "--git-path", name])?;
    let path = if path.is_absolute() {
        path
    } else {
        manifest_dir.join(path)
    };

    // Only emit live paths. Besides avoiding Cargo's perpetual-dirty behavior,
    // this lets a fresh repository legitimately omit packed-refs or refs/tags.
    path.canonicalize().ok()
}

fn git_output(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = git_output_bytes(manifest_dir, args)?;
    let output = String::from_utf8(output).ok()?;
    let output = output.trim();
    (!output.is_empty()).then(|| output.to_owned())
}

fn git_output_path(manifest_dir: &Path, args: &[&str]) -> Option<PathBuf> {
    git_output(manifest_dir, args).map(PathBuf::from)
}

fn git_output_bytes(manifest_dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .args(["-c", "core.quotePath=false"])
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::{dependency_paths, git_path, worktree_probe_path};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempRepo {
        root: PathBuf,
        primary: PathBuf,
        linked: PathBuf,
    }

    impl TempRepo {
        fn new() -> Self {
            let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock is after the Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "tcl-version-git-inputs-{}-{nanos}-{serial}",
                std::process::id()
            ));
            let primary = root.join("primary");
            let linked = root.join("linked");

            std::fs::create_dir_all(&root).expect("create temporary repository root");
            git(&root, &["init", "--initial-branch=base", path(&primary)]);
            git(&primary, &["config", "user.name", "tcl-version test"]);
            git(
                &primary,
                &["config", "user.email", "tcl-version@example.invalid"],
            );
            git(&primary, &["config", "commit.gpgsign", "false"]);
            let hooks = root.join("empty-hooks");
            std::fs::create_dir_all(&hooks).expect("create empty hooks directory");
            git(&primary, &["config", "core.hooksPath", path(&hooks)]);
            std::fs::write(primary.join("tracked"), "first\n").expect("write tracked file");
            git(&primary, &["add", "tracked"]);
            git(&primary, &["commit", "-m", "first"]);
            git(
                &primary,
                &["worktree", "add", "-b", "linked-test", path(&linked)],
            );

            Self {
                root,
                primary,
                linked,
            }
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn linked_worktree_dependencies_exist_and_follow_its_branch() {
        let repo = TempRepo::new();
        let manifest_dir = repo.linked.join("rust/tcl-version");
        std::fs::create_dir_all(&manifest_dir).expect("create nested manifest directory");

        let paths = dependency_paths(&manifest_dir);
        assert!(!paths.is_empty());
        assert!(
            paths.iter().all(|candidate| candidate.exists()),
            "all emitted Cargo dependencies must exist: {paths:#?}"
        );

        let head = git_path(&manifest_dir, "HEAD").expect("resolve worktree HEAD");
        let branch = git_path(&manifest_dir, "refs/heads/linked-test")
            .expect("resolve linked worktree branch");
        let branch_log = git_path(&manifest_dir, "logs/refs/heads/linked-test")
            .expect("resolve linked worktree branch reflog");
        let all_heads = git_path(&manifest_dir, "refs/heads").expect("resolve heads directory");
        assert!(paths.contains(&head), "missing worktree HEAD: {paths:#?}");
        assert!(
            paths.contains(&branch),
            "missing exact branch ref: {paths:#?}"
        );
        assert!(
            paths.contains(&branch_log),
            "missing exact branch reflog: {paths:#?}"
        );
        assert!(
            !paths.contains(&all_heads),
            "unrelated branches must not invalidate the build: {paths:#?}"
        );
        assert!(
            head.starts_with(repo.primary.join(".git/worktrees")),
            "linked HEAD must resolve through the common repository: {head:?}"
        );

        let before = std::fs::read_to_string(&branch).expect("read initial branch ref");
        let log_before = std::fs::read_to_string(&branch_log).expect("read initial branch reflog");
        std::fs::write(repo.linked.join("tracked"), "second\n").expect("update tracked file");
        git(&repo.linked, &["add", "tracked"]);
        git(&repo.linked, &["commit", "-m", "second"]);
        let after = std::fs::read_to_string(&branch).expect("read advanced branch ref");
        let log_after = std::fs::read_to_string(&branch_log).expect("read advanced branch reflog");
        assert_ne!(before, after, "the watched exact ref must move on commit");
        assert_ne!(
            log_before, log_after,
            "the watched exact reflog must move on commit"
        );
    }

    #[test]
    fn worktree_probe_is_absent_and_scoped_to_cargo_output() {
        let repo = TempRepo::new();
        let out_dir = repo.root.join("cargo-out");
        std::fs::create_dir(&out_dir).expect("create mock Cargo output directory");
        let probe = worktree_probe_path(&out_dir);
        assert_eq!(probe.parent(), Some(out_dir.as_path()));
        assert!(!probe.exists(), "the worktree probe must remain absent");
    }

    fn git(cwd: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("git output is UTF-8")
            .trim()
            .to_owned()
    }

    fn path(path: &Path) -> &str {
        path.to_str().expect("temporary path is UTF-8")
    }
}
