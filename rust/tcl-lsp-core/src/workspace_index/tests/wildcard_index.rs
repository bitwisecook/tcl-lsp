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

//! The precomputed import index: settlement cost, export gates, the registry-builtin gate and the admission index.

use super::*;

// The settlement walk's cost must not scale with
// (invocations x in-scope imports x export rows).
//
// Asked per call, that product is a 26 s `textDocument/references` on a
// 113-file workspace.  The shape that produces it is entirely ordinary and
// is reproduced below: many load-level `namespace import NS::*`, a source
// namespace with many `namespace export` patterns, and a great many bare
// calls that resolve to no workspace command at all (every `set`, `if`,
// `expr` in the project), each of which falls through to the wildcard
// import tier.  Each such call would ask, per in-scope import, per export
// row, "had this export run when that import ran?" — a `RunOrder` walk
// over `HashMap`s keyed by owned URI strings.
//
// Nothing about an import's position depends on the call being resolved,
// so that whole question is decided once per recorded import at
// index-build time (`ExportGate`), leaving a hash probe and a glob match
// on the per-call path.

/// A workspace of that shape, sized so a per-call gate costs seconds and
/// the indexed one costs milliseconds.
fn wildcard_import_heavy_workspace(
    importers: usize,
    exports: usize,
    calls_per_file: usize,
) -> Vec<(String, AnalysisResult)> {
    use std::fmt::Write as _;
    let patterns: Vec<String> = (0..exports).map(|i| format!("Exported{i}")).collect();
    let lib = format!(
        "namespace eval ::Lib {{\n    namespace export {}\n    proc helper {{}} {{}}\n}}\n",
        patterns.join(" "),
    );
    let mut docs = vec![("file:///lib.tcl".to_owned(), analyse(&lib))];
    for f in 0..importers {
        // A load-level glob import, then bare calls that settle to no
        // workspace command — the ones that reach the wildcard tier.
        let mut src = String::from("namespace import ::Lib::*\n");
        for c in 0..calls_per_file {
            writeln!(src, "noSuchCommand{c} a b").expect("writing to a String cannot fail");
        }
        docs.push((format!("file:///f{f}.tcl"), analyse(&src)));
    }
    docs
}

#[test]
fn settling_invocations_does_not_scale_with_imports_times_exports() {
    let docs = wildcard_import_heavy_workspace(60, 40, 60);
    let index = WorkspaceIndex::from_documents(docs.iter().map(|(uri, a)| (uri.as_str(), a)));
    let started = std::time::Instant::now();
    // The first call builds the settled-target view for the whole
    // workspace, which is where the cost lived.
    let hits = index.linked_invocations_of("::Lib::helper", "");
    let elapsed = started.elapsed();
    assert!(
        hits.is_empty(),
        "no call names ::Lib::helper, so this is the empty answer the walk had to work for",
    );
    // Measured on this shape, debug build: 81 ms with the gate hoisted,
    // 7.59 s with `import_hop` restored to asking `exports_name_at` per
    // call.  The bound sits between them with a 25x margin below and a
    // 3.8x margin above, so it cannot flake on a loaded CI box and still
    // fails outright if the per-call export walk comes back.
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "settling {} documents took {elapsed:?} — the per-call export walk is back (#1297)",
        docs.len(),
    );
}

/// The same workspace, asserting the *memo* rather than the wall clock:
/// a second reading of an unchanged index must serve the cached view, so
/// the cost above is paid once per generation and not once per request.
/// (`code_lenses` asks once per proc *and* once per class.)
#[test]
fn the_settled_target_view_is_served_from_cache_within_a_generation() {
    let docs = wildcard_import_heavy_workspace(8, 6, 4);
    let index = WorkspaceIndex::from_documents(docs.iter().map(|(uri, a)| (uri.as_str(), a)));
    let _ = index.linked_invocations_of("::Lib::missing", "");
    let first = index.settled_invocations[1].settled_documents();
    let _ = index.linked_invocations_of("::Lib::missing", "");
    assert!(
        first == index.settled_invocations[1].settled_documents(),
        "an unchanged index must serve the settled-target cache, not rebuild it",
    );
}

/// TP/TN on the hoisted export gate itself, through the public answer: an
/// exported name really does resolve through a wildcard import, and an
/// unexported sibling really does not.  This is the behaviour the
/// `ExportGate` precomputation must preserve exactly — the perf tests
/// above would happily pass on a gate that answered "no" to everything.
#[test]
fn the_precomputed_export_gate_still_admits_exactly_what_is_exported() {
    let lib = analyse(
        "namespace eval ::Lib {\n    proc shown {} {}\n    proc hidden {} {}\n    namespace export shown\n}\n",
    );
    let app = analyse("namespace import ::Lib::*\nshown\nhidden\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    // TP: `shown` is exported, so the bare call reaches it through the import.
    assert_eq!(
        index.linked_invocations_of("::Lib::shown", "").len(),
        1,
        "an exported name must still resolve through the wildcard import",
    );
    // TN: `hidden` is not exported, so the bare call reaches nothing —
    // tclsh 9.0.4 answers `invalid command name` for it.
    assert!(
        index.linked_invocations_of("::Lib::hidden", "").is_empty(),
        "an unexported sibling must not be reachable through the import",
    );
}

/// FN guard for the literal/glob split inside [`ExportGate`]: a *glob*
/// export pattern must keep working, since only metacharacter-free
/// patterns may take the hash-set path.
#[test]
fn a_glob_export_pattern_still_covers_the_names_it_matches() {
    let lib = analyse(
        "namespace eval ::Lib {\n    proc getThing {} {}\n    proc setThing {} {}\n    namespace export get*\n}\n",
    );
    let app = analyse("namespace import ::Lib::*\ngetThing\nsetThing\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    assert_eq!(index.linked_invocations_of("::Lib::getThing", "").len(), 1);
    assert!(
        index
            .linked_invocations_of("::Lib::setThing", "")
            .is_empty(),
        "`get*` does not cover `setThing`",
    );
}

// The import-conflict rule's "already exists" side must
// include the commands the *registry* declares, not only the procs and
// classes the workspace declares.
//
// Every expectation below is oracle-verified on tclsh 9.0.4 (built from
// core-9-0-4) and 8.6.14, byte-identical:
//
//   namespace eval ::Foo { proc set {a b} {…}; namespace export set }
//   namespace import ::Foo::*   -> can't import command "set": already exists
//                                  namespace origin ::set  == ::set
//   namespace import ::Foo::set -> same error (the exact-pattern twin)
//   namespace import -force ::Foo::*  -> namespace origin ::set == ::Foo::set
//   namespace eval ::Bar { namespace import ::Foo::* }
//                               -> namespace origin ::Bar::set == ::Foo::set
//   # …and `+` is NOT a global command, so it does not conflict:
//   info commands ::+           -> {}      (+ 1 2 -> invalid command name)

/// TP: the defect itself.  `::Foo` exports `set`; real Tcl refuses the
/// import, so the bare `set x 1` reaches the builtin and is **not** a
/// reference to `::Foo::set`.
#[test]
fn a_wildcard_import_does_not_bind_a_name_the_registry_already_declares() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import ::Foo::*\nset x 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index.linked_invocations_of("::Foo::set", "").is_empty(),
        "the import raised `already exists` and installed nothing, so the \
             bare call reaches the builtin",
    );
}

/// TN: `-force` is the one import that *does* replace a command the
/// target namespace already holds, so the call really is a reference.
#[test]
fn a_forced_wildcard_import_still_shadows_a_registry_builtin() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import -force ::Foo::*\nset x 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index.linked_invocations_of("::Foo::set", "").len(),
        1,
        "`namespace origin ::set` is `::Foo::set` after a -force import",
    );
}

/// FP guard: the rule is about the **target** namespace's command table,
/// and the builtins live in `::`.  Importing into `::Bar` binds
/// `::Bar::set` with no conflict at all, so a call from `::Bar` must
/// still resolve — the gate must not fire on the bare name alone.
#[test]
fn importing_a_builtin_name_into_a_sub_namespace_still_binds() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace eval ::Bar {\n    namespace import ::Foo::*\n    proc go {} { set x 1 }\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index.linked_invocations_of("::Foo::set", "").len(),
        1,
        "`::Bar::set` is not a builtin, so the import installs and the \
             call from ::Bar reaches it",
    );
}

/// TN: an ordinary, non-builtin exported name is unaffected — the gate
/// must not have made the whole wildcard tier answer "no".
#[test]
fn a_wildcard_import_of_a_non_builtin_name_is_unaffected() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc mything {} {}\n    namespace export mything\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import ::Foo::*\nmything\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(index.linked_invocations_of("::Foo::mything", "").len(), 1);
}

/// FN guard for the operator spellings, which is where a naive
/// "`registry.get(name).is_some()`" gate would break a *working* import:
/// `+` is not a command in `::` at all (`info commands ::+` is empty in a
/// fresh tclsh 9.0.4), so exporting `+` and importing it into `::`
/// genuinely binds — `namespace origin ::+` answers `::Ops::+`.
#[test]
fn importing_an_operator_name_into_the_global_namespace_still_binds() {
    let a = analyse_as(
        "namespace eval ::Ops {\n    proc + {a b} { return OPS }\n    namespace export +\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import ::Ops::*\n+ 1 2\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        index.linked_invocations_of("::Ops::+", "").len(),
        1,
        "a bare operator spelling is the post-`namespace import \
             ::tcl::mathop::*` form, not a command a fresh interpreter holds",
    );
}

/// The exact-pattern twin: `namespace import ::Foo::set` is refused for
/// the identical reason, so the link it would introduce is not live.
#[test]
fn an_exact_import_of_a_registry_builtin_installs_no_link() {
    let a = analyse_as(
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        "namespace import ::Foo::set\nset x 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index.linked_invocations_of("::Foo::set", "").is_empty(),
        "an exact import of an already-existing name installs nothing either",
    );
}

/// Dialect sensitivity: the gate must read the *document's own* dialect,
/// not a workspace-wide guess.  `HTTP::uri` is a command only the
/// `f5-irules` registry declares, so an import of it conflicts there and
/// binds under plain Tcl.
#[test]
fn the_builtin_gate_reads_each_documents_own_dialect() {
    let lib = "namespace eval ::Mine {\n    proc uri {} {}\n    namespace export uri\n}\n";
    let importer =
        "namespace eval ::HTTP {\n    namespace import ::Mine::*\n    proc go {} { uri }\n}\n";
    // Plain Tcl knows no `::HTTP::uri`, so the import installs.
    let a = analyse_as(
        lib,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let b = analyse_as(
        importer,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let plain = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(
        plain.linked_invocations_of("::Mine::uri", "").len(),
        1,
        "plain Tcl has no ::HTTP::uri to conflict with",
    );
    // The iRules registry does, so the same source conflicts.
    let a = analyse_as(lib, tcl_dialect::DialectProfile::irules());
    let b = analyse_as(importer, tcl_dialect::DialectProfile::irules());
    let irules = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        irules.linked_invocations_of("::Mine::uri", "").is_empty(),
        "f5-irules declares ::HTTP::uri, so the unforced import installs nothing",
    );
}

// `import_hop` must not scan a namespace's whole import list per call.
//
// Both of its first two filters — the tail pattern covers the word, and
// the export gate admits it — depend only on the word, so the admissible
// set is a build-time fact (`NamespaceImports`). On a large corpus such a
// scan is 28 369 calls over 3 987 739 rows, ~390 ms of the ~420 ms a
// `textDocument/references` takes after an edit.
//
// The risk in indexing it is order: `import_hop` breaks ties on the
// position *within the filtered sequence*, so a reordering — or a
// duplicate from a row carrying both literal and glob export patterns —
// would silently change which import wins. These pin that.

/// The admitted sequence must be exactly what a full scan would yield,
/// in the same order, over every shape the gate can take: literal-only,
/// glob-only, both at once (the duplicate hazard), and a tail pattern
/// that excludes a name the gate would otherwise admit.
#[test]
fn admitting_matches_a_full_scan_in_order() {
    let lib = analyse(
        "namespace eval ::Lib {\n    proc alpha {} {}\n    proc beta {} {}\n    proc gamma {} {}\n    namespace export alpha beta gamma\n}\n",
    );
    let globby = analyse(
        "namespace eval ::Glob {\n    proc alpha {} {}\n    proc beta {} {}\n    namespace export a*\n}\n",
    );
    let app = analyse(
        "namespace import ::Lib::*\nnamespace import ::Glob::*\nnamespace import ::Lib::b*\nalpha\nbeta\ngamma\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///lib.tcl", &lib),
        ("file:///glob.tcl", &globby),
        ("file:///app.tcl", &app),
    ]);
    let wci = WildcardImportIndex::build(&index);
    let imports = wci
        .imports_by_ns
        .get("::")
        .expect("the three imports are all at global level");
    for word in ["alpha", "beta", "gamma", "delta", "", "a", "*"] {
        let scanned: Vec<_> = imports
            .rows
            .iter()
            .filter(|row| tcl_syntax::glob::string_match(&row.imp.tail_pattern, word))
            .filter(|row| row.exported.covers(word))
            .map(|row| (row.imp.uri.as_str(), row.imp.at))
            .collect();
        let admitted: Vec<_> = imports
            .admitting(word)
            .map(|row| (row.imp.uri.as_str(), row.imp.at))
            .collect();
        assert_eq!(
            admitted, scanned,
            "admitting({word:?}) must equal the full scan, in order",
        );
    }
}

/// A row whose gate holds **both** a literal and a glob pattern is
/// reachable through both halves of the index and must still appear once.
/// A duplicate would shift every later row's tie-break position.
#[test]
fn a_row_indexed_twice_is_admitted_once() {
    let lib = analyse(
        "namespace eval ::Both {\n    proc alpha {} {}\n    namespace export alpha a*\n}\n",
    );
    let app = analyse("namespace import ::Both::*\nalpha\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    let wci = WildcardImportIndex::build(&index);
    let imports = wci.imports_by_ns.get("::").expect("one global import");
    assert_eq!(
        imports.admitting("alpha").count(),
        1,
        "`alpha` matches both the literal and the `a*` pattern of one gate",
    );
}

/// TP through the public answer: the indexed path still resolves a call
/// that only a wildcard import can reach. The perf guards below would be
/// happy with an index that admitted nothing at all.
#[test]
fn the_admission_index_still_resolves_a_wildcard_imported_call() {
    let lib =
        analyse("namespace eval ::Lib {\n    proc helper {} {}\n    namespace export helper\n}\n");
    let app = analyse("namespace import ::Lib::*\nhelper\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    assert_eq!(index.linked_invocations_of("::Lib::helper", "").len(), 1);
}

/// A gate that exports nothing admits nothing, so such rows drop out of
/// every lookup — the case that makes the index worth having, since a
/// workspace routinely imports namespaces it holds no exports for
/// (`::tcl::mathop`, `::tcltest`).
#[test]
fn an_import_of_an_unexporting_namespace_admits_no_word() {
    let app = analyse("namespace import ::tcl::mathop::*\nset x 1\n+ 1 2\n");
    let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
    let wci = WildcardImportIndex::build(&index);
    let imports = wci.imports_by_ns.get("::").expect("one global import");
    for word in ["set", "+", "puts", "anything"] {
        assert_eq!(
            imports.admitting(word).count(),
            0,
            "no export rows for ::tcl::mathop, so nothing is admissible",
        );
    }
}
