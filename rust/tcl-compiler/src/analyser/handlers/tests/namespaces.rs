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

//! `namespace eval`, `inscope`, `code`, `path`, `import` and `export`.

use super::*;

// handle_namespace_export_command

#[test]
fn handle_namespace_export_records_pattern() {
    let mut a = Analyser::new();
    a.handle_namespace_export_command(
        &["export".to_string(), "bar".to_string()],
        &[esc_tok(span(0, 6)), esc_tok(span(7, 10))],
        &[],
    );
    assert_eq!(a.result.namespace_exports.len(), 1);
    assert_eq!(a.result.namespace_exports[0].ns, "::");
    assert_eq!(a.result.namespace_exports[0].pattern, "bar");
}

#[test]
fn handle_namespace_export_records_multiple_patterns() {
    let mut a = Analyser::new();
    a.handle_namespace_export_command(
        &["export".to_string(), "bar".to_string(), "b*".to_string()],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 10)),
            esc_tok(span(11, 13)),
        ],
        &[],
    );
    let patterns: Vec<&str> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| e.pattern.as_str())
        .collect();
    assert_eq!(patterns, vec!["bar", "b*"]);
}

/// `NamespaceExportCmd` calls `Tcl_Export` per pattern and returns on the
/// first failure, so a qualified pattern aborts the loop — the patterns
/// *before* it are committed, the ones after are never exported at all.
/// Oracle rows (identical on tclsh 8.6.16 and 9.0.4, `namespace export`
/// read back from the namespace afterwards):
///
/// | script | exports |
/// |---|---|
/// | `namespace export ::bad ok` | nothing |
/// | `namespace export ok ::bad` | `ok` |
/// | `namespace export one ::bad three` | `one` |
#[test]
fn handle_namespace_export_stops_at_the_first_qualified_pattern() {
    let recorded = |words: &[&str]| -> Vec<String> {
        let mut a = Analyser::new();
        let args: Vec<String> = std::iter::once("export".to_string())
            .chain(words.iter().map(|w| (*w).to_string()))
            .collect();
        let tokens: Vec<Token> = (0..args.len())
            .map(|i| {
                let start = u32::try_from(i).expect("small index") * 10;
                esc_tok(span(start, start + 5))
            })
            .collect();
        a.handle_namespace_export_command(&args, &tokens, &[]);
        a.result
            .namespace_exports
            .iter()
            .map(|e| e.pattern.clone())
            .collect()
    };

    // The bad pattern is first: nothing is exported.
    assert_eq!(recorded(&["::bad", "ok"]), [] as [std::string::String; 0]);
    // …and after a valid one: only the valid one survives.
    assert_eq!(recorded(&["ok", "::bad"]), vec!["ok".to_string()]);
    assert_eq!(
        recorded(&["one", "::bad", "three"]),
        vec!["one".to_string()]
    );
    // A *relative* qualifier is equally invalid (oracle: `namespace
    // export sub::pat` errors), so it aborts too.
    assert_eq!(
        recorded(&["sub::pat", "ok"]),
        [] as [std::string::String; 0]
    );
    // A lone colon is not a separator, so `a:b` is a perfectly valid
    // pattern and must not abort (oracle: exports `a:b`).
    assert_eq!(
        recorded(&["a:b", "ok"]),
        vec!["a:b".to_string(), "ok".to_string()]
    );
    // The empty pattern is valid too (oracle: exports `{} ok`).
    assert_eq!(recorded(&["", "ok"]), vec![String::new(), "ok".to_string()]);
}

#[test]
fn handle_namespace_export_clear_commits_before_an_invalid_pattern() {
    // C spends a whole `Tcl_Export(…, "::", 1)` call on `-clear`, which
    // resets the list before any pattern is looked at, so a later bad
    // pattern cannot undo it. Oracle: a namespace exporting `keep`, then
    // `namespace export -clear ::bad`, exports nothing afterwards.
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &[
            "export".to_string(),
            "-clear".to_string(),
            "::bad".to_string(),
        ],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 13)),
            esc_tok(span(14, 19)),
        ],
        &[],
    );
    let events: Vec<(&str, bool)> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| (e.pattern.as_str(), e.clears))
        .collect();
    assert_eq!(events, vec![("", true)]);
}

#[test]
fn handle_namespace_export_clear_records_an_ordered_tombstone() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &["export".to_string(), "bar".to_string()],
        &[esc_tok(span(0, 6)), esc_tok(span(7, 10))],
        &[],
    );
    a.handle_namespace_export_command(
        &[
            "export".to_string(),
            "-clear".to_string(),
            "baz".to_string(),
        ],
        &[
            esc_tok(span(11, 17)),
            esc_tok(span(18, 24)),
            esc_tok(span(25, 28)),
        ],
        &[],
    );
    // `-clear` is an ordered event, not an eager delete: the earlier
    // `bar` entry stays on the log so a `namespace import` that ran
    // *before* the `-clear` can still see it.
    let events: Vec<(&str, bool, u32)> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| (e.pattern.as_str(), e.clears, e.range.start()))
        .collect();
    assert_eq!(
        events,
        vec![("bar", false, 7), ("", true, 18), ("baz", false, 25)]
    );
}

#[test]
fn handle_namespace_export_consumes_only_one_clear_flag() {
    // `NamespaceExportCmd` compares `objv[1]`
    // against `-clear` once, so a *second* `-clear` is an ordinary export
    // pattern. Oracle (tclsh 8.6.14 / 9.0.4): `namespace export -clear
    // -clear p` leaves exactly `-clear p` exported, and a command really
    // named `-clear` is then importable through `namespace import
    // ::src::*`. Consuming every matching word instead records two
    // tombstones and silently drops the `-clear` export.
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &[
            "export".to_string(),
            "-clear".to_string(),
            "-clear".to_string(),
            "p".to_string(),
        ],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 13)),
            esc_tok(span(14, 20)),
            esc_tok(span(21, 22)),
        ],
        &[],
    );
    let events: Vec<(&str, bool)> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| (e.pattern.as_str(), e.clears))
        .collect();
    assert_eq!(events, vec![("", true), ("-clear", false), ("p", false)]);
}

#[test]
fn handle_namespace_export_flag_after_a_pattern_is_a_pattern() {
    // The flag is only ever the *first* word: oracle `namespace export a
    // -clear` → `namespace export` returns `a -clear`.
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &["export".to_string(), "a".to_string(), "-clear".to_string()],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 8)),
            esc_tok(span(9, 15)),
        ],
        &[],
    );
    let events: Vec<(&str, bool)> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| (e.pattern.as_str(), e.clears))
        .collect();
    assert_eq!(events, vec![("a", false), ("-clear", false)]);
}

#[test]
fn handle_namespace_import_consumes_only_one_force_flag() {
    // `namespace import -force -force ::src::*` reads the second `-force`
    // as an import pattern and aborts with `no namespace specified in
    // import pattern "-force"` before `::src::*` is imported (tclsh
    // 8.6.18/9.0.4: `info commands ::a` stays empty). The command raises,
    // so it records no import.
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_import_command(
        &[
            "import".to_string(),
            "-force".to_string(),
            "-force".to_string(),
            "::src::*".to_string(),
        ],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 13)),
            esc_tok(span(14, 20)),
            esc_tok(span(21, 29)),
        ],
        &[],
    );
    let patterns: Vec<&str> = a
        .result
        .namespace_imports
        .iter()
        .map(|i| i.pattern.as_str())
        .collect();
    assert_eq!(patterns, Vec::<&str>::new());
}

#[test]
fn handle_namespace_export_bare_clear_records_a_tombstone() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &["export".to_string(), "-clear".to_string()],
        &[esc_tok(span(0, 6)), esc_tok(span(7, 13))],
        &[],
    );
    assert_eq!(a.result.namespace_exports.len(), 1);
    assert!(a.result.namespace_exports[0].clears);
    assert_eq!(a.result.namespace_exports[0].pattern, "");
}

#[test]
fn handle_namespace_export_skips_dynamic_pattern() {
    let mut a = Analyser::new();
    a.handle_namespace_export_command(
        &["export".to_string(), "$dyn".to_string()],
        &[esc_tok(span(0, 6)), esc_tok(span(7, 11))],
        &[],
    );
    assert_eq!(
        a.result.namespace_exports.len(),
        0,
        "a dynamic pattern can't be statically recorded"
    );
}

// handle_namespace_path_command

/// The two-word literal set form records entries under the declaring
/// namespace; a later declaration replaces the whole path (C Tcl
/// semantics), and entries are kept as written (rooting happens in the
/// shared candidate builder).
#[test]
fn handle_namespace_path_records_and_replaces() {
    let mut a = Analyser::new();
    a.handle_namespace_path_command(&["path".to_string(), "::u ::v".to_string()], &[], &[]);
    assert_eq!(
        a.namespace_paths.get("::").map(Vec::as_slice),
        Some(&["::u".to_string(), "::v".to_string()][..]),
    );
    a.handle_namespace_path_command(&["path".to_string(), "inner".to_string()], &[], &[]);
    assert_eq!(
        a.namespace_paths.get("::").map(Vec::as_slice),
        Some(&["inner".to_string()][..]),
        "each namespace path declaration replaces the whole path",
    );
}

/// A **braced** path word is literal, not dynamic. The word
/// arrives with braces stripped, so a text-only dynamism scan sees the
/// `$` and abstains from the whole command; the token kind is the
/// authority.
///
/// tclsh-proof: tclsh8.6.18/9.0.4 — `namespace path {::$ns ::a}` alone
/// raises `namespace "::$ns" not found`; with both namespaces created
/// first, `namespace eval n { namespace path {::$ns ::a} }` then
/// `namespace eval n { namespace path }` answers `{::$ns} ::a` — the
/// entry is a namespace literally *named* `::$ns`.
#[test]
fn handle_namespace_path_records_a_braced_dollar_bearing_word() {
    let r = Analyser::new().analyse(
        "namespace eval {::$ns} {}\nnamespace eval ::a {}\nnamespace eval n { namespace path {::$ns ::a} }\n",
        "tcl8.6",
    );
    assert_eq!(
        r.namespace_paths.get("::n").map(Vec::as_slice),
        Some(&["::$ns".to_string(), "::a".to_string()][..]),
        "a braced word never substitutes, so its entries are literal names",
    );
}

/// TN control — the same text **unbraced** really is a substitution, so
/// the command still abstains.
#[test]
fn handle_namespace_path_still_skips_an_unbraced_dollar_word() {
    let mut a = Analyser::new();
    a.handle_namespace_path_command(
        &["path".to_string(), "$entries".to_string()],
        &[esc_tok(span(0, 4)), Token::new(TokenType::Var, span(5, 13))],
        &[],
    );
    assert_eq!(a.namespace_paths.len(), 0);
}

/// The query form (no list) and a dynamic list (`$var` / `[cmd]`)
/// record nothing — the conservative empty path stands.
#[test]
fn handle_namespace_path_skips_query_and_dynamic_forms() {
    let mut a = Analyser::new();
    a.handle_namespace_path_command(&["path".to_string()], &[], &[]);
    a.handle_namespace_path_command(&["path".to_string(), "$entries".to_string()], &[], &[]);
    a.handle_namespace_path_command(
        &["path".to_string(), "[current_path]".to_string()],
        &[],
        &[],
    );
    assert_eq!(a.namespace_paths.len(), 0);
}

/// A declaration inside a namespace scope keys to that namespace's
/// accumulated name, so settlement applies it to the right call sites.
#[test]
fn handle_namespace_path_keys_to_declaring_namespace() {
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(crate::analyser::types::Scope::new(
            crate::analyser::types::ScopeKind::Namespace,
            "outer",
        ));
    a.handle_namespace_path_command(&["path".to_string(), "::helpers".to_string()], &[], &[0]);
    assert_eq!(
        a.namespace_paths.get("::outer").map(Vec::as_slice),
        Some(&["::helpers".to_string()][..]),
    );
}

/// The `namespace path` resolution tier is 8.5+: a bare call under a path
/// gains the path namespace as a candidate from 8.5 on, but under 8.4 it
/// must not (8.4 has no path tier, so the call never reaches it).
#[test]
fn bare_call_honours_namespace_path_only_from_8_5() {
    let src = "namespace eval ::app { namespace path ::mymod\n    proc run {} { helper } }\n";
    let candidates = |dialect: &str| {
        let mut a = Analyser::new();
        a.analyse(src, dialect)
            .command_invocations
            .iter()
            .find(|i| i.name == "helper")
            .map_or_default(|i| i.resolution_candidates.clone())
    };
    assert!(
        candidates("tcl8.6").iter().any(|c| c == "::mymod::helper"),
        "8.6 should add the path namespace as a candidate: {:?}",
        candidates("tcl8.6"),
    );
    assert!(
        !candidates("tcl8.4").iter().any(|c| c == "::mymod::helper"),
        "8.4 has no path tier, so it must not: {:?}",
        candidates("tcl8.4"),
    );
}

/// `namespace inscope ::x { proc foo }` runs the body in `::x`, so `foo`
/// homes to `::x::foo` — the same namespace frame as `namespace eval`, not
/// the caller's scope.
#[test]
fn namespace_inscope_runs_the_body_in_the_named_namespace() {
    let mut a = Analyser::new();
    let r = a.analyse("namespace inscope ::x { proc foo {} {} }\n", "tcl8.6");
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::x::foo"),
        "inscope body should home to the named namespace: {keys:?}",
    );
    assert!(
        !r.all_procs.contains_key("::foo"),
        "the body must not resolve in the caller's global scope: {keys:?}",
    );
}

/// `namespace code { proc foo }` returns a script and runs nothing
/// (tclsh8.6.18/9.0.4: it answers `::namespace inscope ::x { proc foo {} {} }`
/// and `info procs ::x::*` stays empty), so it defines no proc.
#[test]
fn namespace_code_returns_its_script_without_running_it() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::x { namespace code { proc foo {} {} } }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        !r.all_procs.contains_key("::x::foo") && !r.all_procs.contains_key("::foo"),
        "namespace code returns its script without running it: {keys:?}",
    );
}

#[test]
fn handle_namespace_eval_creates_child_scope() {
    let mut a = Analyser::new();
    let handled = a.handle_namespace_eval_command(
        &[
            "eval".to_string(),
            "ns1".to_string(),
            "proc inner {} {}".to_string(),
        ],
        &[
            esc_tok(span(10, 14)),
            esc_tok(span(15, 18)),
            str_tok(span(19, 35)),
        ],
        &[],
        &[],
    );
    assert!(handled);
    assert_eq!(a.result.global_scope.children.len(), 1);
    assert_eq!(a.result.global_scope.children[0].name, "ns1");
    assert_eq!(
        a.result.global_scope.children[0].kind,
        crate::analyser::types::ScopeKind::Namespace,
    );
}

#[test]
fn namespace_scope_geometry_retains_the_actual_point_operand() {
    // naming.source.original-point-operand-projection
    // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
    let source = "set target A; namespace eval $target {namespace eval B {set x 1}}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let outer = &result.global_scope.children[0];
    let inner = &outer.children[0];
    let outer_input = result
        .namespace_refs
        .iter()
        .find(|reference| {
            reference
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"A")
        })
        .unwrap()
        .original_name_input
        .as_ref()
        .unwrap();
    let inner_input = result
        .namespace_refs
        .iter()
        .find(|reference| {
            reference
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"B")
        })
        .unwrap()
        .original_name_input
        .as_ref()
        .unwrap();
    assert_eq!(
        outer.naming_scope,
        result
            .global_scope
            .naming_scope
            .as_ref()
            .unwrap()
            .child_from_input(outer_input)
    );
    assert!(outer.naming_scope.is_some());
    assert_eq!(
        inner.naming_scope,
        outer
            .naming_scope
            .as_ref()
            .unwrap()
            .child_from_input(inner_input)
    );
    assert!(inner.naming_scope.is_some());
    let unknown = Analyser::new().analyse("namespace eval $missing {set x 1}", "tcl8.6");
    assert!(unknown.global_scope.children[0].naming_scope.is_none());
}

#[test]
fn handle_namespace_eval_records_body_span() {
    let mut a = Analyser::new();
    a.handle_namespace_eval_command(
        &["eval".to_string(), "ns1".to_string(), String::new()],
        &[
            esc_tok(span(10, 14)),
            esc_tok(span(15, 18)),
            str_tok(span(19, 35)),
        ],
        &[],
        &[],
    );
    assert_eq!(
        a.result.global_scope.children[0].body_span,
        Some(span(19, 35))
    );
}

#[test]
fn handle_namespace_eval_dynamic_target_gets_a_synthetic_span_keyed_name() {
    // TP (corpus shape: irc.tcl's per-connection
    // `namespace eval $name { … }` idiom): a
    // dynamic target must never become the scope's `.name` verbatim (two
    // unrelated occurrences sharing the same variable name would then
    // collapse into one scope), so it's replaced with a synthetic name
    // keyed on this occurrence's own token offset.
    let mut a = Analyser::new();
    a.handle_namespace_eval_command(
        &[
            "eval".to_string(),
            "$name".to_string(),
            "proc inner {} {}".to_string(),
        ],
        &[
            esc_tok(span(10, 14)),
            esc_tok(span(15, 20)),
            str_tok(span(21, 37)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.global_scope.children[0].name, "@dynns@15");
}

#[test]
fn handle_namespace_eval_literal_target_keeps_its_written_name() {
    // FN guard — a literal (non-dynamic) target must still use its own
    // written text verbatim.
    let mut a = Analyser::new();
    a.handle_namespace_eval_command(
        &["eval".to_string(), "ns1".to_string(), String::new()],
        &[
            esc_tok(span(10, 14)),
            esc_tok(span(15, 18)),
            str_tok(span(19, 35)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.global_scope.children[0].name, "ns1");
}

#[test]
fn two_dynamic_namespace_eval_blocks_sharing_a_variable_name_never_collide() {
    // TP — the full-pipeline shape from irc.tcl: two lexically
    // unrelated procs each open `namespace eval $name { … }` with a
    // same-named local `name` (an unremarkable choice for this exact
    // idiom) and each define their own `helper`. Keying both blocks'
    // scope on the literal text "$name" would let the second `helper`
    // silently overwrite the first in the flat `all_procs` map.
    let mut a = Analyser::new();
    let r = a.analyse(
            "proc setupA {} {\n    set name connA\n    namespace eval $name {\n        proc helper {} { return A }\n    }\n}\n\
             proc setupB {} {\n    set name connB\n    namespace eval $name {\n        proc helper {} { return B }\n    }\n}\n",
            "tcl8.6",
        );
    // drift-ok: test assertion counting distinct entries by simple name
    // to prove no cross-occurrence merge, not a resolution decision.
    let helpers: Vec<_> = r
        .all_procs
        .iter()
        .filter(|(_, p)| p.name == "helper")
        .collect();
    assert_eq!(
        helpers.len(),
        2,
        "both blocks' helper must survive as distinct entries: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
    assert_ne!(
        helpers[0].0, helpers[1].0,
        "distinct call sites must get distinct qualified keys: {helpers:?}",
    );
    assert_ne!(
        helpers[0].1.body_span, helpers[1].1.body_span,
        "each is genuinely its own definition, not the same one twice: {helpers:?}",
    );
}

/// `namespace eval :: [list puts $file]` builds its script in the calling
/// frame, so `$file` reads the procedure's parameter before the namespace
/// is entered. tclsh 8.6.18 and 9.0.4: `proc ::p {file} {namespace eval ::
/// [list puts $file]}; p hello` prints `hello`.
#[test]
fn a_list_built_namespace_script_reads_its_operands_in_the_calling_frame() {
    let src = "proc ::p {file} {\n    namespace eval :: [list puts $file]\n}\np hello\n";
    let read = src.find("$file").unwrap();
    for dialect in ["tcl8.6", "tcl9.0"] {
        let r = Analyser::new().analyse(src, dialect);
        let procedure = r
            .global_scope
            .children
            .iter()
            .find(|scope| scope.kind == crate::analyser::types::ScopeKind::Proc)
            .expect("procedure scope");
        let references = &procedure.variables["file"].references;
        assert!(
            references
                .iter()
                .any(|span| span.start() as usize == read || span.start() as usize == read + 1),
            "{dialect}: {references:?}"
        );
    }
}
