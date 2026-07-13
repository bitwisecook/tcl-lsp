// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CI gate for the generated Zed tree-sitter highlight queries.
//!
//! The queries are generated from `tcl-registry` (`cargo xtask gen-zed-queries`)
//! and byte-compared against the registry by that verb's `--check`. But that
//! guard is blind to the *other* coupling: the static template's tree-sitter
//! node names (`command`, `procedure`, `simple_word`, field `name:`, …). If the
//! pinned grammar is bumped and a node is renamed/removed — or the template is
//! hand-edited — highlighting silently breaks in Zed with nothing failing.
//!
//! These tests close that gap: every committed `highlights.scm` is compiled
//! against the grammar vendored in `vendor/` (pinned to the same rev as
//! `editors/zed/extension.toml`), and a `QueryError` fails the build. A separate
//! test asserts the vendored rev still matches `extension.toml`, so the vendored
//! parser can't silently drift from what Zed actually loads.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use tree_sitter::{Language, Query};

    extern "C" {
        fn tree_sitter_tcl() -> *const std::os::raw::c_void;
        fn tree_sitter_apl() -> *const std::os::raw::c_void;
        fn tree_sitter_bigip() -> *const std::os::raw::c_void;
    }

    /// The grammar each Zed language directory is backed by.
    ///
    /// Not every language is Tcl. `apl/` and `tmsh/` have their own grammars
    /// (issue #903) — validating their queries against the *Tcl* grammar would
    /// reject every one of them, and validating them against nothing at all,
    /// which is what happened before, let a query naming a node that does not
    /// exist ship silently and simply not highlight.
    fn language_for(dir: &str) -> Language {
        let raw = unsafe {
            match dir {
                "apl" => tree_sitter_apl(),
                "tmsh" => tree_sitter_bigip(),
                _ => tree_sitter_tcl(),
            }
        };
        unsafe { Language::from_raw(raw as *const _) }
    }

    /// Repo root — two levels up from this crate (`rust/zed-query-check`).
    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("canonicalize repo root")
    }

    /// Every committed `editors/zed/languages/*/{highlights,injections}.scm`.
    ///
    /// Injections are checked too: an injection query naming a node that does
    /// not exist does not error at runtime, it simply never injects — so the
    /// embedded Tcl inside an `ltm rule` body or an APL `[ … ]` expression
    /// would quietly stop being highlighted, with nothing to notice it.
    fn committed_queries() -> Vec<PathBuf> {
        let langs = repo_root().join("editors/zed/languages");
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&langs)
            .expect("read languages dir")
            .flatten()
        {
            for name in ["highlights.scm", "injections.scm"] {
                let scm = entry.path().join(name);
                if scm.is_file() {
                    out.push(scm);
                }
            }
        }
        out.sort();
        assert!(
            !out.is_empty(),
            "found no highlights.scm under {}",
            langs.display()
        );
        out
    }

    #[test]
    fn every_committed_query_compiles_against_the_grammar() {
        let mut failures = Vec::new();
        for path in committed_queries() {
            let dir = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            let language = language_for(&dir);
            let src = std::fs::read_to_string(&path).expect("read query");
            if let Err(e) = Query::new(&language, &src) {
                failures.push(format!("{}: {e:?}", path.display()));
            }
        }
        assert!(
            failures.is_empty(),
            "generated Zed highlight queries do not compile against the pinned \
             tree-sitter-tcl grammar:\n  {}",
            failures.join("\n  ")
        );
    }

    #[test]
    fn vendored_grammar_rev_matches_extension_toml() {
        let vendored =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/REV"))
                .expect("read vendor/REV")
                .trim()
                .to_owned();

        let ext = std::fs::read_to_string(repo_root().join("editors/zed/extension.toml"))
            .expect("read extension.toml");
        // Find the `rev = "..."` line inside the `[grammars.tcl]` block.
        let rev = ext
            .split_once("[grammars.tcl]")
            .and_then(|(_, rest)| rest.split_once("rev"))
            .and_then(|(_, rest)| rest.split_once('"'))
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(rev, _)| rev.to_owned())
            .expect("parse rev from extension.toml [grammars.tcl]");

        assert_eq!(
            vendored, rev,
            "vendored grammar rev (vendor/REV) is out of sync with \
             editors/zed/extension.toml [grammars.tcl] rev — re-vendor \
             parser.c/scanner.c from the new rev and update vendor/REV"
        );
    }
}
