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

//! `retired-api-gate` — the zero-reference gate for the P1-G retirements
//! (`docs/design/dialect-and-package-registry-centralisation.md` §3), plus
//! the P1a ledger-C4 retirement (the `head_identity` binding table, now
//! the realm command-binding state in `tcl_compiler::realm`).
//!
//! P1-G deleted the old dialect-name validators
//! (`DialectProfile::by_name` / `by_opt_name` / `resolve_known` /
//! `availability_for_name`) and the string-keyed registry doors
//! (`tcl_registry::registry_for_dialect` / `registry_handle_for_dialect`),
//! and made the profile-keyed cache doors and the `ProfileQueries` trait
//! `pub(crate)` inside `tcl-registry`. The compiler enforces the deletion
//! for the names that no longer exist; this gate additionally fails on any
//! **textual** reintroduction — a same-named public twin, a revived
//! import, a copy-pasted call — anywhere in the Rust tree, so the retired
//! spelling cannot come back under a fresh definition either.
//!
//! Comment lines are exempt (the ledger and the seam docs cite the retired
//! names as history), as are the crate-internal survivors inside
//! `rust/tcl-registry/src/` (`ProfileQueries` and the cache doors live on
//! there behind `pub(crate)` — the compile-level half of the gate). A
//! deliberate, reviewed exception carries a `// retired-api-ok: <reason>`
//! waiver on the flagged line or one of the four lines above it, and must
//! be recorded in the centralisation ledger.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// One retired spelling: the needle, whether the following character may
/// continue an identifier (it may not — `registry_for_dialect_profile` and
/// `resolve_known_environment` are live seam names), and whether the
/// pattern is scoped to code **outside** `rust/tcl-registry/src/` (the
/// crate-internal survivors).
struct RetiredPattern {
    needle: &'static str,
    outside_registry_only: bool,
}

/// The retired spellings. Every entry names a mechanism the §3 ledger
/// retired in P1-G; the seam replacements are
/// `tcl_registry::model::ingress::{resolve_environment,
/// resolve_known_environment, static_context_for, static_context_for_profile,
/// static_document_context_for, static_document_context_for_profile}` and
/// `ResolvedContext`'s query surface.
const RETIRED: &[RetiredPattern] = &[
    RetiredPattern {
        needle: "DialectProfile::by_name",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "DialectProfile::by_opt_name",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "DialectProfile::resolve_known",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "DialectProfile::availability_for_name",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "availability_for_name(",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "tcl_registry::registry_for_dialect",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "tcl_registry::cache::registry_for_dialect",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "registry_handle_for_dialect",
        outside_registry_only: true,
    },
    RetiredPattern {
        needle: "tcl_registry::registry_for_profile",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "tcl_registry::cache::registry_for_profile",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "registry_handle_for_profile",
        outside_registry_only: true,
    },
    RetiredPattern {
        needle: "ProfileQueries",
        outside_registry_only: true,
    },
    RetiredPattern {
        needle: "special_vars::resolve_dialect",
        outside_registry_only: false,
    },
    // Ledger C4 (P1a): the parallel offset-keyed head-identity binding
    // table is retired wholesale onto the realm command-binding state
    // (`tcl_compiler::realm`, answering `BindingKnowledge`).
    RetiredPattern {
        needle: "head_identity",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "HeadIdentityMap",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "HeadIdentity",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "command_head_identities",
        outside_registry_only: false,
    },
    // Ledger O2 (redesign §11.1, owner ruling 2026-08-27): the M9 dead axes.
    // Each was declared-and-unpopulated model surface — a word no data used,
    // inviting packs to guess at semantics the engine never implemented.
    // Principle P-C: anything genuinely needed later comes back *with* its
    // consumer, under whatever name that consumer wants.
    //
    // `ProfileSpec::conflicts` is deliberately NOT here: it is the one axis of
    // the six with a live consumer (`tcl-bigip`'s BIGIP6039 profile-graph
    // check), so it was kept.
    RetiredPattern {
        needle: "PASSWORD_OPTION",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "PasswordOption",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "IRULES_DATA_GETTER",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "IrulesDataGetter",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "xc_operation",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "arg_rows",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "VersionedArgRow",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "ProjectedArgs",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "ArgTables",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "arg_tables_at",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "project_arg_rows",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "arg_indices_for_role_at",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "command_prefixes_at",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "init_only",
        outside_registry_only: false,
    },
    // The `EventRequires` capability axis. Scoped to a needle that cannot
    // collide with the live `CapabilitySet` / `BuildCapability` vocabulary:
    // the retired word was a bare `capability` **field** on that struct.
    RetiredPattern {
        needle: "requires.capability",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "OptionConstraint",
        outside_registry_only: false,
    },
];

/// Live seam names a retired needle is a prefix of — a hit whose
/// identifier continues past the needle into one of these is the
/// replacement itself, not a reintroduction.
fn identifier_continues(text: &str, hit_end: usize) -> bool {
    text[hit_end..]
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Scan the workspace's Rust sources; exit non-zero listing any offending
/// site. `check` is accepted for CLI symmetry with the other gates — the
/// gate never rewrites anything, so both modes verify.
pub fn run(_check: bool) -> ExitCode {
    let root = crate::util::repo_root();
    let mut files = Vec::new();
    collect_rs_files(&root.join("rust"), &mut files);
    collect_rs_files(&root.join("runtime/rust/src"), &mut files);

    let mut report = String::new();
    let mut hits = 0usize;
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // The gate's own docs and fixtures spell the patterns out.
        if rel == "rust/xtask/src/retired_api_gate.rs" {
            continue;
        }
        let inside_registry = rel.starts_with("rust/tcl-registry/src/");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (line_no, needle) in scan(&text, inside_registry) {
            if is_waived(&text, line_no) {
                continue;
            }
            hits += 1;
            let _ = writeln!(report, "  {rel}:{line_no}: `{needle}`");
        }
    }

    if hits == 0 {
        println!("retired-api-gate: OK (no retired dialect/registry API spellings in code)");
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "retired-api-gate: {hits} use(s) of retired P1-G API spellings — resolve \
         through `tcl_registry::model::ingress` (the one dialect-name seam) or \
         `ResolvedContext`'s queries instead, or mark a reviewed exception with \
         `// retired-api-ok: <reason>` and a ledger entry \
         (docs/design/dialect-and-package-registry-centralisation.md §3):\n{report}"
    );
    ExitCode::FAILURE
}

/// Recursively collect `.rs` files under `dir` (skipping `target/`).
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Yield `(1-based line, matched needle)` for every retired spelling on a
/// non-comment line. Comment lines (`//`, `///`, `//!`) may cite the
/// retired names as history — the ledger and the seam docs do — so only
/// code lines count.
fn scan(text: &str, inside_registry: bool) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        for pattern in RETIRED {
            if pattern.outside_registry_only && inside_registry {
                continue;
            }
            let mut from = 0;
            while let Some(off) = line[from..].find(pattern.needle) {
                let start = from + off;
                let end = start + pattern.needle.len();
                from = end;
                if identifier_continues(line, end) {
                    continue;
                }
                out.push((idx + 1, pattern.needle));
                break;
            }
        }
    }
    out
}

/// Whether the flagged line (or one of the four lines above it — room for
/// a multi-line justification comment) carries a `retired-api-ok:` waiver.
fn is_waived(text: &str, line_no: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let idx = line_no.saturating_sub(1);
    (idx.saturating_sub(4)..=idx)
        .filter_map(|i| lines.get(i))
        .any(|l| l.contains("retired-api-ok:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate must fail on a seeded violation of every retired family.
    #[test]
    fn seeded_violations_are_flagged() {
        for seeded in [
            "let p = tcl_dialect::DialectProfile::by_name(\"tcl8.6\");",
            "let p = DialectProfile::by_opt_name(dialect);",
            "let p = DialectProfile::resolve_known(name);",
            "let mask = DialectProfile::availability_for_name(name);",
            "let r = tcl_registry::registry_for_dialect(\"tcl8.6\");",
            "let r = tcl_registry::cache::registry_for_dialect(dialect);",
            "let r = registry_handle_for_dialect(dialect);",
            "let r = tcl_registry::registry_for_profile(profile);",
            "let r = registry_handle_for_profile(profile);",
            "use tcl_registry::profile_queries::ProfileQueries;",
            "let mask = tcl_registry::special_vars::resolve_dialect(dialect);",
            "use tcl_compiler::head_identity::HeadWords;",
            "let map = HeadIdentityMap::none();",
            "let id = HeadIdentity::Rebound;",
            "let map = command_head_identities(source, dialect, registry);",
            // Ledger O2 — the M9 dead axes.
            "if spec.traits.contains(Traits::PASSWORD_OPTION) { }",
            "PasswordOption => PASSWORD_OPTION, Security, \"takes a password option\";",
            "if spec.traits.contains(Traits::IRULES_DATA_GETTER) { }",
            "IrulesDataGetter => IRULES_DATA_GETTER, Irules, \"a getter\";",
            "spec.xc_operation = Some(leak_str(&value));",
            "spec.arg_rows = rows;",
            "let row = VersionedArgRow { index: 0 };",
            "let out = ProjectedArgs::default();",
            "let tables = ArgTables::Stored { roles };",
            "let tables = spec.arg_tables_at(floor);",
            "let out = project_arg_rows(rows, None);",
            "let idx = registry.arg_indices_for_role_at(name, args, role, floor);",
            "let p = registry.command_prefixes_at(name, args, floor);",
            "requires.init_only = parse_flag(stmt.tail());",
            "if let Some(want) = requires.capability { }",
            "let c: &[OptionConstraint] = &[];",
        ] {
            assert_eq!(scan(seeded, false).len(), 1, "{seeded}");
        }
    }

    /// The live seam spellings a retired needle prefixes never match.
    #[test]
    fn seam_replacements_do_not_match() {
        for ok in [
            "let e = tcl_registry::model::ingress::resolve_known_environment(name);",
            "let r = crate::registry_for_dialect_profile(profile);",
            "let r = tcl_spectcl::install::registry_for_dialect_with_packs(d, &packs);",
            "let r = bundled::registry_for_dialect_from(dialect, &packs);",
            "let r = tcl_registry::registry_for_profile_with_overlay(p, key, |_| {});",
            "let g = registry_for_environment_if_built(&def, &id, &keyed, overlay);",
        ] {
            assert!(scan(ok, false).is_empty(), "{ok}");
        }
    }

    /// Comment lines citing the retired names as history are exempt; the
    /// same spelling in code is not.
    #[test]
    fn comment_citations_are_exempt() {
        let history = "/// The environment-model form of `DialectProfile::by_name(name)`.\n";
        assert!(scan(history, false).is_empty());
        let code = "let p = DialectProfile::by_name(name);\n";
        assert_eq!(scan(code, false).len(), 1);
    }

    /// The crate-internal survivors are exempt only inside
    /// `rust/tcl-registry/src/`.
    #[test]
    fn registry_internal_survivors_are_scoped() {
        let line = "use crate::profile_queries::ProfileQueries;\n";
        assert!(scan(line, true).is_empty());
        assert_eq!(scan(line, false).len(), 1);
    }

    /// The documented waiver suppresses a hit; an unwaived hit stands.
    #[test]
    fn waiver_comment_suppresses() {
        let src =
            "// retired-api-ok: ledger §3 entry, reviewed\nlet p = DialectProfile::by_name(n);\n";
        let hits = scan(src, false);
        assert_eq!(hits.len(), 1);
        assert!(is_waived(src, hits[0].0));
        let bare = "let p = DialectProfile::by_name(n);\n";
        let hits = scan(bare, false);
        assert_eq!(hits.len(), 1);
        assert!(!is_waived(bare, hits[0].0));
    }
}
