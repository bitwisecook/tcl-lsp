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
//! It also holds the `one-loader` retirements: the CST pack-loader front
//! end (`load_pack`, `load_pack_cached`, `expand_includes`,
//! `report_extra_speclib_blocks`) and the second cache identity
//! (`LOADER_BUILD`, `eval_snapshot_memoised`), all replaced by design E's
//! evaluation loader behind one cache door.
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
//! It also holds the `one-vocabulary` lane's two retirements: the
//! `StubOverlay` per-document command overlay (gap ruling R1 — stubs are
//! provenance-tagged `SurfaceDeclaration`s now) and the second command-table
//! transition vocabulary (ledger C8 — `CommandRegistry::command_table_effect`
//! and `tcl_compiler::alias`'s argument destructuring, both replaced by
//! `CommandBindingTransition` facts).
//!
//! Comment lines are exempt (the ledger and the seam docs cite the retired
//! names as history), as are the crate-internal survivors inside
//! `rust/tcl-registry/src/` (`ProfileQueries` and the cache doors live on
//! there behind `pub(crate)` — the compile-level half of the gate). A
//! deliberate, reviewed exception carries a `// retired-api-ok: <reason>`
//! waiver on the flagged line or one of the four lines above it, and must
//! be recorded in the centralisation ledger.
//!
//! # The one-oracle gate (gap ruling R10)
//!
//! The second family this file carries is not about *deleted* spellings but
//! about **owned** ones: the answers the #1631 programme centralised — does
//! this command exist here, is it available here, what did this call do to
//! the command table — each of which a consumer could quietly grow a second
//! copy of. R10's answer is visibility narrowing where that is enough
//! (`Analyser::builtin_command_names` and
//! `model::declaration::DeclaredSurface::get` are `pub(crate)`;
//! `tcl-registry`'s cache doors and `ProfileQueries` were narrowed in P1-G)
//! **plus** this call-site sweep for the doors that cannot be narrowed
//! because legitimate spec-content readers share them.
//!
//! Each [`OwnedPattern`] names one such answer and the file prefixes that
//! own it. Writing it anywhere else fails the gate, and the escape hatch is
//! a `// one-oracle-ok: <reason>` waiver **plus** a row in the
//! centralisation ledger's §3 table — the same discipline the retired
//! family uses, for the same reason: an exception that nobody wrote down is
//! how a second oracle comes back.

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
    // Q13: the availability bitmask, and every projection that only made
    // sense as bits. Availability is stated as `SpecSurface` rows and asked
    // as a `SurfaceQuery` point.
    RetiredPattern {
        needle: "DialectSet",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "availability_mask",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "get_for_dialect(",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "load_dialect(",
        outside_registry_only: false,
    },
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
    // The `one-loader` lane (redesign §11, ledger row L1): `SpecTcl` had two
    // live implementations of "load a pack" — design E's evaluation loader
    // and the CST front end it was proved byte-identical to. The CST front
    // end is deleted; `tcl_spectcl::loader::evaluate_pack` (uncached) and
    // `tcl_spectcl::cache::evaluate_pack_cached` (the one production door)
    // are what remain. `PackTables`, `apply_pack_stmt` and the row readers
    // are deliberately NOT here: they are the one `SpecTcl` vocabulary, not
    // a second loader, and the evaluation loader replays through them.
    RetiredPattern {
        needle: "load_pack",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "expand_includes",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "report_extra_speclib_blocks",
        outside_registry_only: false,
    },
    // The parse-memo half of the two caching layers: one key
    // (`EvalSnapshotKey` stamped with the build) now serves both the
    // in-memory snapshot tier and the on-disk segmentation tier.
    RetiredPattern {
        needle: "LOADER_BUILD",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "eval_snapshot_memoised",
        outside_registry_only: false,
    },
    // The `one-vocabulary` lane, gap ruling R1 (redesign §11.2 D18): the
    // per-document `# tcl-lsp: stub` overlay and its parallel vocabulary.
    // Stubs ingest as provenance-tagged `SurfaceDeclaration`s now
    // (`tcl_registry::model::declaration`), read through the one
    // `DocumentCommandSurface` door.
    RetiredPattern {
        needle: "StubOverlay",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "stub_overlay",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "StubSig",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "StubSigFlags",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "build_stub_overlay",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "to_stub_sig",
        outside_registry_only: false,
    },
    // The `one-vocabulary` lane, ledger C8 (redesign §11.2 D9): the second
    // command-table transition vocabulary. `CommandTableEffect` survives as
    // the pack-authoring **selector** — `CommandSpec::command_table_effect`
    // is still a field a `SpecTcl` pack writes — but the consumer-facing
    // resolver is gone: the needle carries its call parenthesis so the
    // surviving field reads do not match it.
    RetiredPattern {
        needle: "command_table_effect(",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "command_table_effects",
        outside_registry_only: false,
    },
    // …and the per-consumer argument destructuring it forced. The layout
    // lives in `tcl_registry::state_transition::command_binding` now, once.
    RetiredPattern {
        needle: "detect_rename",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "detect_interp_alias",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "detect_interp_alias_delete",
        outside_registry_only: false,
    },
    RetiredPattern {
        needle: "is_interp_alias_shape",
        outside_registry_only: false,
    },
];

/// One **owned** spelling: an answer the programme centralised, and the
/// repository-relative path prefixes whose files may write it (ruling R10).
///
/// Unlike a [`RetiredPattern`] the spelling is alive and correct — in its
/// owner. What the gate forbids is a *second* home for it, which is how a
/// consumer grows its own existence oracle, availability rule, or binding
/// table without ever reintroducing a retired name.
struct OwnedPattern {
    needle: &'static str,
    owners: &'static [&'static str],
}

/// The owned answers (ruling R10). Each row is one question the
/// centralisation programme gave a single answer to.
const OWNED: &[OwnedPattern] = &[
    // **Does this command exist at this program point?** — the one oracle
    // (R-c, P1a). Its registry tier and its typed verdict live with it in
    // the analyser; nothing else may assemble either.
    OwnedPattern {
        needle: "CommandExistenceOracle",
        owners: &["rust/tcl-compiler/src/analyser/"],
    },
    OwnedPattern {
        needle: "command_existence_oracle",
        owners: &["rust/tcl-compiler/src/analyser/"],
    },
    OwnedPattern {
        needle: "builtin_command_names",
        owners: &["rust/tcl-compiler/src/analyser/"],
    },
    OwnedPattern {
        needle: "w123_registry_known_names",
        owners: &["rust/tcl-compiler/src/analyser/"],
    },
    // **Is this command available in this dialect?** — the registry's own
    // profile-visible surface. The compiler's constant folder is the one
    // consumer outside the registry that legitimately asks (issue #1427: a
    // fold skips the runtime availability gate, so it must); anything else
    // asking is a second availability rule.
    OwnedPattern {
        needle: "has_command_in_this_dialect",
        owners: &["rust/tcl-registry/src/", "rust/tcl-compiler/src/codegen/"],
    },
    OwnedPattern {
        needle: "all_dialect_command_names",
        owners: &["rust/tcl-registry/src/"],
    },
    // **What did this call do to the command table?** — ledger C8's one
    // vocabulary. The registry resolves the facts; `tcl_compiler::alias` is
    // the single bridge from reconstructed source words to that resolution.
    // A consumer building its own bridge is a second vocabulary.
    OwnedPattern {
        needle: "command_binding_transitions",
        owners: &["rust/tcl-registry/", "rust/tcl-compiler/src/alias.rs"],
    },
    OwnedPattern {
        needle: "command_table_transitions",
        owners: &[
            "rust/tcl-compiler/src/",
            "rust/tcl-lsp-core/src/tk_preview.rs",
        ],
    },
    // **Which command surface does this document analyse against?** — gap
    // ruling R1's one door. The per-document declaration set is assembled in
    // the analyser and read through `DocumentCommandSurface`; a consumer
    // holding the raw set would be rebuilding the overlay R1 retired.
    OwnedPattern {
        needle: "DeclaredSurface",
        owners: &[
            "rust/tcl-registry/src/",
            "rust/tcl-compiler/src/analyser/",
            "rust/tcl-lsp-db/src/",
        ],
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

/// Whether the character *before* a hit continues an identifier — i.e. the
/// needle matched a suffix of a longer name rather than a name of its own.
///
/// Without this, `arg_rows` matches inside `project_arg_rows` and a single
/// retired call is reported twice under two needles. A path separator is
/// not an identifier character, so a qualified spelling such as
/// `cache::registry_for_dialect` still matches on its bare needle.
fn identifier_precedes(text: &str, hit_start: usize) -> bool {
    text[..hit_start]
        .bytes()
        .next_back()
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
    let mut owned_report = String::new();
    let mut owned_hits = 0usize;
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
            if is_waived(&text, line_no, RETIRED_WAIVER) {
                continue;
            }
            hits += 1;
            let _ = writeln!(report, "  {rel}:{line_no}: `{needle}`");
        }
        for (line_no, needle) in scan_owned(&text, &rel) {
            if is_waived(&text, line_no, OWNED_WAIVER) {
                continue;
            }
            owned_hits += 1;
            let _ = writeln!(owned_report, "  {rel}:{line_no}: `{needle}`");
        }
    }

    if hits > 0 {
        eprintln!(
            "retired-api-gate: {hits} use(s) of retired P1-G API spellings — resolve \
             through `tcl_registry::model::ingress` (the one dialect-name seam) or \
             `ResolvedContext`'s queries instead, or mark a reviewed exception with \
             `// {RETIRED_WAIVER} <reason>` and a ledger entry \
             (docs/design/dialect-and-package-registry-centralisation.md §3):\n{report}"
        );
    }
    if owned_hits > 0 {
        eprintln!(
            "retired-api-gate (one-oracle, ruling R10): {owned_hits} use(s) of a \
             centralised answer outside the module that owns it — a second \
             existence oracle, availability rule, or binding table starts here. \
             Ask the owner instead, or mark a reviewed exception with \
             `// {OWNED_WAIVER} <reason>` **and** a row in the centralisation \
             ledger's §3 table \
             (docs/design/dialect-and-package-registry-centralisation.md §3):\n\
             {owned_report}"
        );
    }
    if hits == 0 && owned_hits == 0 {
        println!(
            "retired-api-gate: OK (no retired dialect/registry API spellings, and \
             every centralised answer stays with its owner)"
        );
        return ExitCode::SUCCESS;
    }
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
            if find_needle(line, pattern.needle) {
                out.push((idx + 1, pattern.needle));
            }
        }
    }
    out
}

/// The waiver marker for a retired spelling.
const RETIRED_WAIVER: &str = "retired-api-ok:";

/// The waiver marker for an owned answer written outside its owner.
const OWNED_WAIVER: &str = "one-oracle-ok:";

/// Whether the flagged line (or one of the four lines above it — room for
/// a multi-line justification comment) carries a `marker` waiver.
fn is_waived(text: &str, line_no: usize, marker: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let idx = line_no.saturating_sub(1);
    (idx.saturating_sub(4)..=idx)
        .filter_map(|i| lines.get(i))
        .any(|l| l.contains(marker))
}

/// Yield `(1-based line, matched needle)` for every **owned** spelling
/// written in a file that does not own it (ruling R10). Comment lines are
/// exempt for the same reason the retired family exempts them: the ledger
/// and the module docs name the owned answers as prose.
fn scan_owned(text: &str, rel: &str) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        for pattern in OWNED {
            if pattern.owners.iter().any(|owner| rel.starts_with(owner)) {
                continue;
            }
            if find_needle(line, pattern.needle) {
                out.push((idx + 1, pattern.needle));
            }
        }
    }
    out
}

/// Whether `line` writes `needle` as a name of its own — an identifier
/// boundary on **both** sides, the property that keeps `arg_rows` from
/// matching inside `project_arg_rows`.
///
/// The boundary is only required at an end the needle *spells* with an
/// identifier character. A needle that already carries its own punctuation
/// — `availability_for_name(`, `command_table_effect(` — has its boundary
/// in the needle, and demanding another one outside it would make the
/// needle match nothing at all.
fn find_needle(line: &str, needle: &str) -> bool {
    let leading_boundary = needle
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_');
    let trailing_boundary = needle
        .bytes()
        .next_back()
        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_');
    let mut from = 0;
    while let Some(off) = line[from..].find(needle) {
        let start = from + off;
        let end = start + needle.len();
        from = end;
        if (leading_boundary && identifier_precedes(line, start))
            || (trailing_boundary && identifier_continues(line, end))
        {
            continue;
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate must fail on a seeded violation of every retired family.
    ///
    /// Each row is `(line, findings)`. Almost every seeded line carries one
    /// retired spelling and must be flagged exactly once — asserting the
    /// exact count is what catches a needle that has grown broad enough to
    /// match its own neighbours. A line that genuinely contains two retired
    /// spellings says so, rather than the assertion being relaxed for all of
    /// them.
    #[test]
    fn seeded_violations_are_flagged() {
        for (seeded, findings) in [
            (
                "let p = tcl_dialect::DialectProfile::by_name(\"tcl8.6\");",
                1,
            ),
            ("let p = DialectProfile::by_opt_name(dialect);", 1),
            ("let p = DialectProfile::resolve_known(name);", 1),
            // Two retired spellings on one line: the qualified path and the
            // bare call. The row says so rather than the assertion being
            // relaxed.
            ("let mask = DialectProfile::availability_for_name(name);", 2),
            ("let r = tcl_registry::registry_for_dialect(\"tcl8.6\");", 1),
            (
                "let r = tcl_registry::cache::registry_for_dialect(dialect);",
                1,
            ),
            ("let r = registry_handle_for_dialect(dialect);", 1),
            ("let r = tcl_registry::registry_for_profile(profile);", 1),
            ("let r = registry_handle_for_profile(profile);", 1),
            ("use tcl_registry::profile_queries::ProfileQueries;", 1),
            (
                "let mask = tcl_registry::special_vars::resolve_dialect(dialect);",
                1,
            ),
            ("use tcl_compiler::head_identity::HeadWords;", 1),
            ("let map = HeadIdentityMap::none();", 1),
            ("let id = HeadIdentity::Rebound;", 1),
            (
                "let map = command_head_identities(source, dialect, registry);",
                1,
            ),
            // Ledger O2 — the M9 dead axes.
            ("if spec.traits.contains(Traits::PASSWORD_OPTION) { }", 1),
            (
                "PasswordOption => PASSWORD_OPTION, Security, \"takes a password option\";",
                2,
            ),
            ("if spec.traits.contains(Traits::IRULES_DATA_GETTER) { }", 1),
            (
                "IrulesDataGetter => IRULES_DATA_GETTER, Irules, \"a getter\";",
                2,
            ),
            ("spec.xc_operation = Some(leak_str(&value));", 1),
            ("spec.arg_rows = rows;", 1),
            ("let row = VersionedArgRow { index: 0 };", 1),
            ("let out = ProjectedArgs::default();", 1),
            ("let tables = ArgTables::Stored { roles };", 1),
            ("let tables = spec.arg_tables_at(floor);", 1),
            ("let out = project_arg_rows(rows, None);", 1),
            (
                "let idx = registry.arg_indices_for_role_at(name, args, role, floor);",
                1,
            ),
            (
                "let p = registry.command_prefixes_at(name, args, floor);",
                1,
            ),
            ("requires.init_only = parse_flag(stmt.tail());", 1),
            ("if let Some(want) = requires.capability { }", 1),
            ("let c: &[OptionConstraint] = &[];", 1),
            // The `one-vocabulary` lane — gap ruling R1.
            ("use tcl_registry::stub_overlay::StubOverlay;", 2),
            ("let mut o = StubOverlay::new();", 1),
            ("let s: StubSig = def.to_stub_sig();", 2),
            ("flags: StubSigFlags::empty(),", 1),
            ("let o = build_stub_overlay(&defs);", 1),
            // …and ledger C8. The surviving pack-authoring **field** read is
            // not a hit — only the retired resolver call is, which is what
            // the needle's own parenthesis buys.
            ("let e = registry.command_table_effect(name, sub);", 1),
            ("footprint.legacy().command_table_effects.is_empty()", 1),
            ("if let Some((old, new)) = detect_rename(&args) { }", 1),
            ("let a = detect_interp_alias(&args);", 1),
            ("let d = detect_interp_alias_delete(&args);", 1),
            ("if !is_interp_alias_shape(args) { return; }", 1),
        ] {
            assert_eq!(scan(seeded, false).len(), findings, "{seeded}");
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

    /// The one-oracle gate (ruling R10) fails on a seeded second copy of
    /// every owned answer, exactly once per row.
    ///
    /// Asserting the exact count is what catches a needle that has grown
    /// broad enough to match its own neighbours — the same property the
    /// retired family's seeded test pins.
    #[test]
    fn seeded_second_oracles_are_flagged() {
        // A file that owns nothing: every owned answer is a violation there.
        let outsider = "rust/tcl-lsp-server/src/lib.rs";
        for (seeded, findings) in [
            ("let oracle = CommandExistenceOracle::default();", 1),
            ("let o = self.command_existence_oracle(registry);", 1),
            ("let names = self.builtin_command_names();", 1),
            ("let names = self.w123_registry_known_names(registry);", 1),
            ("if registry.has_command_in_this_dialect(head) { }", 1),
            ("let all = all_dialect_command_names();", 1),
            ("let t = registry.command_binding_transitions(words);", 1),
            (
                "let t = command_table_transitions(registry, head, args);",
                1,
            ),
            ("let declared = DeclaredSurface::new();", 1),
        ] {
            assert_eq!(scan_owned(seeded, outsider).len(), findings, "{seeded}");
        }
    }

    /// An owned answer is not a violation in the module that owns it, and
    /// the ownership test is a path **prefix** so a whole directory can own
    /// one.
    #[test]
    fn owned_answers_are_free_in_their_owner() {
        let line = "let names = self.builtin_command_names();\n";
        assert!(
            scan_owned(
                line,
                "rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs"
            )
            .is_empty()
        );
        assert_eq!(
            scan_owned(line, "rust/tcl-lsp-core/src/completion.rs").len(),
            1
        );

        // One answer may have several owners — the constant folder asks the
        // registry's availability question legitimately (issue #1427).
        let fold = "if registry.has_command_in_this_dialect(head) { }\n";
        assert!(scan_owned(fold, "rust/tcl-compiler/src/codegen/values.rs").is_empty());
        assert!(scan_owned(fold, "rust/tcl-registry/src/registry.rs").is_empty());
        assert_eq!(scan_owned(fold, "rust/tcl-vm/src/interp.rs").len(), 1);
    }

    /// A comment citing an owned answer is exempt; the same spelling in code
    /// is not.
    #[test]
    fn owned_answer_citations_are_exempt() {
        let prose = "/// Answered by `builtin_command_names`, the one oracle.\n";
        assert!(scan_owned(prose, "rust/tcl-lsp-core/src/completion.rs").is_empty());
        let code = "let names = self.builtin_command_names();\n";
        assert_eq!(
            scan_owned(code, "rust/tcl-lsp-core/src/completion.rs").len(),
            1
        );
    }

    /// The one-oracle waiver is its own marker: a `retired-api-ok:` comment
    /// does not license a second oracle, and vice versa.
    #[test]
    fn the_two_waivers_do_not_substitute_for_each_other() {
        let owned =
            "// one-oracle-ok: ledger §3 row, reviewed\nlet n = self.builtin_command_names();\n";
        let hits = scan_owned(owned, "rust/tcl-lsp-core/src/completion.rs");
        assert_eq!(hits.len(), 1);
        assert!(is_waived(owned, hits[0].0, OWNED_WAIVER));
        assert!(!is_waived(owned, hits[0].0, RETIRED_WAIVER));

        let wrong =
            "// retired-api-ok: not the right hatch\nlet n = self.builtin_command_names();\n";
        let hits = scan_owned(wrong, "rust/tcl-lsp-core/src/completion.rs");
        assert_eq!(hits.len(), 1);
        assert!(!is_waived(wrong, hits[0].0, OWNED_WAIVER));
    }

    /// The documented waiver suppresses a hit; an unwaived hit stands.
    #[test]
    fn waiver_comment_suppresses() {
        let src =
            "// retired-api-ok: ledger §3 entry, reviewed\nlet p = DialectProfile::by_name(n);\n";
        let hits = scan(src, false);
        assert_eq!(hits.len(), 1);
        assert!(is_waived(src, hits[0].0, RETIRED_WAIVER));
        let bare = "let p = DialectProfile::by_name(n);\n";
        let hits = scan(bare, false);
        assert_eq!(hits.len(), 1);
        assert!(!is_waived(bare, hits[0].0, RETIRED_WAIVER));
    }
}
