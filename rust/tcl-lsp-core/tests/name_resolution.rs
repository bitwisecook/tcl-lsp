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

//! TP/FP/TN/FN matrix for issues #954-#958 (Rust LSP), unit level.
//!
//! Each issue is exercised across the confusion matrix:
//!   * **TP** — the feature fires where it should (the call *is* a reference,
//!     the token *is* a parameter, the read *is* suppressed).
//!   * **FP guard** — a look-alike that must *not* fire (a different class /
//!     method / function, a non-`pkgIndex` file, a `$lambda` variable).
//!   * **TN** — nothing relevant present, nothing emitted.
//!   * **FN** (the regression) — the exact shape each issue reported, which
//!     used to be missed and now resolves.
//!
//! `references`/`semantic_tokens` come from `tcl-lsp-core`; W210 diagnostics
//! from the `tcl-compiler` analyser directly.

#![allow(clippy::cast_possible_truncation)]

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::references::references;
use tcl_lsp_core::semantic_tokens::{full, legend_token_types};
use tcl_registry::registry_for_dialect;

fn analyse(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl8.6").clone()
}

/// Sorted, de-duplicated 0-based line numbers of a reference result.
fn ref_lines(ranges: &[LspRange]) -> Vec<u32> {
    let mut v: Vec<u32> = ranges.iter().map(|r| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Reference lines for the cursor at (`line`, `col`), declaration included.
fn refs_at(src: &str, line: u32, col: u32) -> Vec<u32> {
    let analysis = analyse(src);
    ref_lines(&references(src, tcl_dialect::DialectProfile::by_name("tcl"), line, col, &analysis, true))
}

fn refs_at_dialect(src: &str, dialect: &str, line: u32, col: u32) -> Vec<u32> {
    let analysis = Analyser::new().analyse(src, dialect).clone();
    ref_lines(&references(src, tcl_dialect::DialectProfile::by_name(dialect), line, col, &analysis, true))
}

/// W210 (read-before-set) diagnostic codes for `src` analysed as `path`.
fn has_w210(src: &str, path: Option<&str>) -> bool {
    let mut a = Analyser::new().with_file_path(path.map(str::to_string));
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W210)
}

/// The semantic-token *type name* covering the first byte of the first
/// **whole-word** occurrence of `needle` in `src` (via the public legend),
/// or `None`.  Word-bounded so a needle like `a` matches the standalone
/// parameter, not the `a` inside `apply`.
fn kind_of(src: &str, dialect: &str, needle: &str) -> Option<String> {
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b':';
    let bytes = src.as_bytes();
    let byte = (0..=src.len().saturating_sub(needle.len())).find(|&p| {
        src[p..].starts_with(needle)
            && (p == 0 || !is_word(bytes[p - 1]))
            && (p + needle.len() == src.len() || !is_word(bytes[p + needle.len()]))
    })?;
    let before = &src[..byte];
    let line = before.matches('\n').count() as u32;
    let col = (byte - before.rfind('\n').map_or(0, |n| n + 1)) as u32;
    let registry = registry_for_dialect(dialect);
    let st = full(src, tcl_dialect::DialectProfile::by_name(dialect), registry);
    let legend = legend_token_types();
    let (mut l, mut c) = (0u32, 0u32);
    for chunk in st.data.chunks(5) {
        let (dl, dc, len, ty) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        if dl > 0 {
            l += dl;
            c = dc;
        } else {
            c += dc;
        }
        if l == line && c <= col && col < c + len {
            return Some(legend[ty as usize].to_string());
        }
    }
    None
}

// ───────────────────────── #956 — `$obj method` references ────────────────

mod obj_method_dispatch {
    use super::*;

    /// FN→TP: the reported shape — `$b get` dispatches to `Bar956::get`, so the
    /// call site is a reference to the method declaration.
    #[test]
    fn tp_obj_dispatch_is_a_reference() {
        let src = "oo::class create Bar956 {\n    method get {key} { return $key }\n}\nset b [Bar956 new]\nputs [$b get foo]\n";
        // cursor on the `get` declaration name (line 1, col 11).
        assert_eq!(refs_at(src, 1, 11), vec![1, 4], "decl + $b dispatch");
    }

    /// TP: `CLASS create NAME` binds an object *command*; the bare `NAME method`
    /// dispatch is a reference too.
    #[test]
    fn tp_object_command_dispatch_is_a_reference() {
        let src = "oo::class create Dog {\n    method bark {} { return woof }\n}\nDog create rex\nrex bark\n";
        assert_eq!(refs_at(src, 1, 11), vec![1, 4], "decl + `rex bark`");
    }

    /// TP: `$b get` embedded in a quoted / compound word
    /// (`"result: [$b get foo]"`) is still a reference — the merged word's
    /// substitution is recovered by re-lexing its slice.
    #[test]
    fn tp_obj_dispatch_in_quoted_word() {
        let src = "oo::class create Bar {\n    method get {key} { return $key }\n}\nset b [Bar new]\nputs \"result: [$b get foo]\"\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 4],
            "decl + `$b` dispatch in a quote"
        );
    }

    /// FP guard: a same-named method on an *unrelated* class must not be pulled
    /// into `Bar`'s reference set — `$b get` resolves to `Bar`, never `Foo`.
    #[test]
    fn fp_unrelated_class_same_method_excluded() {
        let src = "oo::class create Bar {\n    method get {} { return 1 }\n}\noo::class create Foo {\n    method get {} { return 2 }\n}\nset b [Bar new]\nputs [$b get]\n";
        // cursor on Bar::get (line 1). References must be Bar's decl + the `$b
        // get` site (line 7), never Foo's decl (line 4).
        let got = refs_at(src, 1, 11);
        assert!(
            got.contains(&1) && got.contains(&7),
            "Bar decl + dispatch: {got:?}"
        );
        assert!(!got.contains(&4), "Foo::get must be excluded: {got:?}");
    }

    /// TN: a declared-but-never-dispatched method has only its declaration.
    #[test]
    fn tn_no_dispatch_only_declaration() {
        let src = "oo::class create Solo {\n    method ping {} { return pong }\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1], "declaration only");
    }

    /// FN→TP (issue #957's general form): an external `$obj method` /
    /// `NAME method` dispatch nested inside `if` / `foreach` at the
    /// top level (not inside any proc/method) is a reference too — the
    /// top-level scan region gets the same `Plain`-`BodyKind` recursion
    /// as every proc / method body scan.
    #[test]
    fn tp_obj_dispatch_nested_in_top_level_control_flow() {
        let src = "oo::class create Bar {\n    method get {key} { return $key }\n}\nset b [Bar new]\nif {1} {\n    $b get foo\n}\nforeach x {1 2} {\n    $b get foo\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 5, 8],
            "decl + if-nested + foreach-nested dispatch"
        );
    }

    /// FP guard: a control-flow-nested dispatch of a *different* method on
    /// the same class must not count toward `get`.
    #[test]
    fn fp_obj_dispatch_other_method_excluded_when_control_flow_nested() {
        let src = "oo::class create Bar {\n    method get {key} { return $key }\n    method other {key} { return $key }\n}\nset b [Bar new]\nif {1} {\n    $b other foo\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1], "no `other` leakage");
    }

    /// FN→TP regression for the *exact* issue #956 repro: a `variable` and
    /// `constructor` declared before the `method`, with the method body
    /// reading the instance variable.  The previous investigation of #956
    /// tested a simplified `Bar956` fixture without these members and
    /// concluded the count was already correct — true for the count, but
    /// the codeLens *command* was still empty (fixed separately in
    /// `tcl-lsp-core::code_lens` / `tcl-lsp-server`); this locks in that
    /// find-references itself was never affected by the extra members.
    #[test]
    fn fn_to_tp_exact_issue_956_repro_with_constructor_and_variable() {
        let src = "oo::class create Bar {\n   variable _options\n    constructor {args} {\n         set _options $args\n    }\n\n    method get {key} {\n        return [dict get $_options $key]\n    }\n\n}\nset b [Bar new]\nputs [$b get foo]\n";
        // cursor on the `get` declaration name (line 6, col 11).
        assert_eq!(
            refs_at(src, 6, 11),
            vec![6, 12],
            "decl + `puts [$b get foo]`"
        );
    }
}

// ─────────────────── classmethod dispatch on the class's own command ──────

mod classmethod_dispatch {
    use super::*;

    /// FN→TP: a `classmethod` dispatches on the *class's own* command
    /// (`Factory make`) — never on an instance.  Before this fix,
    /// `find_obj_method_call_sites` only tracked instance handles, so a
    /// classmethod's reference count and codeLens were always "0
    /// references" regardless of how many times it was actually called —
    /// the common (in fact only) classmethod dispatch shape was invisible.
    #[test]
    fn tp_bare_class_command_dispatch_is_a_reference() {
        let src = "oo::class create Factory {\n    classmethod make {} {\n        return [Factory new]\n    }\n}\nFactory make\n";
        // cursor on the `make` declaration name (line 1, col 16); the bare
        // `Factory make` dispatch is on line 5.
        assert_eq!(refs_at(src, 1, 16), vec![1, 5], "decl + `Factory make`");
    }

    /// FN→TP: the reverse direction of the previous test — cursor on the
    /// *call site* itself (`Factory make`, line 5), not the declaration.
    /// Previously resolved to nothing at all (Codex review on #971, P2):
    /// `$obj`/`my` resolution doesn't match a bare two-word receiver, and
    /// the declaration-side resolver requires the cursor inside the class
    /// body, so Find References / Rename triggered from the actual dispatch
    /// site silently did nothing.
    #[test]
    fn tp_bare_class_command_dispatch_resolves_from_the_call_site_itself() {
        let src = "oo::class create Factory {\n    classmethod make {} {\n        return [Factory new]\n    }\n}\nFactory make\n";
        // cursor on `make` in `Factory make` (line 5, col 8).
        assert_eq!(refs_at(src, 5, 8), vec![1, 5], "decl + `Factory make`");
    }

    /// TP: a subclass that inherits (does not override) the classmethod
    /// dispatches on its *own* command and still counts as a reference to
    /// the ancestor's declaration — mirrors the existing instance-method
    /// inheritance handling.
    #[test]
    fn tp_inheriting_subclass_own_command_dispatch_is_a_reference() {
        let src = "oo::class create Factory {\n    classmethod make {} { return [Factory new] }\n}\noo::class create SubFactory {\n    superclass Factory\n}\nSubFactory make\n";
        assert_eq!(
            refs_at(src, 1, 16),
            vec![1, 6],
            "decl + inheriting subclass's own-command dispatch"
        );
    }

    /// FP guard: an *overriding* subclass's own dispatch must attribute to
    /// its own declaration, not the ancestor's — the ancestor's reference
    /// set must not include it.
    #[test]
    fn fp_overriding_subclass_dispatch_excluded_from_ancestor() {
        let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n}\noo::class create SubFactory {\n    superclass Factory\n    classmethod make {} { return 2 }\n}\nSubFactory make\n";
        // cursor on Factory::make (line 1). References must be its
        // declaration only — `SubFactory make` resolves to SubFactory's own
        // override, never Factory's.
        assert_eq!(refs_at(src, 1, 16), vec![1], "override excluded");
    }

    /// FP guard: a regular *instance* method must never gain bare
    /// class-command dispatch — `Factory get` (no `$obj`/instance) is not a
    /// valid `TclOO` call for an instance method, so it must not be
    /// synthesised as a reference just because the class's own command
    /// shares a name-set entry point with classmethod dispatch.
    #[test]
    fn fp_instance_method_gets_no_bare_class_command_dispatch() {
        let src = "oo::class create Factory {\n    method get {} { return 1 }\n}\nFactory get\n";
        // `Factory get` is not a `$obj`/instance dispatch, so it must not be
        // pulled in — declaration only.
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1],
            "no phantom class-command dispatch"
        );
    }

    /// TN: a declared-but-never-dispatched classmethod has only its
    /// declaration.
    #[test]
    fn tn_uncalled_classmethod_only_declaration() {
        let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n}\n";
        assert_eq!(refs_at(src, 1, 16), vec![1], "declaration only");
    }

    /// FN→TP: snit's `typemethod` is snit's equivalent of `TclOO`'s
    /// `classmethod` — dispatched the same way, on the type's own command
    /// (`Factory make`), never an instance. The definition-body grammar maps
    /// both to the same `class_methods` map (a registry-driven fact, not a
    /// TclOO-specific hardcoded check), so this generalises for free with no
    /// snit-specific code in `find_obj_method_call_sites`.
    #[test]
    fn tp_snit_typemethod_bare_dispatch_is_a_reference() {
        let src = "snit::type Factory {\n    typemethod make {} {\n        return [Factory create x]\n    }\n}\nFactory make\n";
        assert_eq!(
            refs_at(src, 1, 15),
            vec![1, 5],
            "decl + `Factory make` (snit typemethod)"
        );
    }

    /// FN→TP (issue #990): [incr Tcl]'s class-scoped `proc` maps to the same
    /// `class_methods` bucket as `classmethod`/`typemethod`, but itcl
    /// dispatches it as a single `::`-qualified identifier
    /// (`Factory::make`), never the two-word `Factory make` form the
    /// `cmd_set` scan matches — so that call site used to be invisible.  It
    /// is now resolved through the qualified-name path instead, keyed on the
    /// definer family, and the two-word scan still never matches it.
    #[test]
    fn fn_to_tp_itcl_class_proc_colon_dispatch_is_a_reference() {
        let src = "itcl::class Factory {\n    proc make {} {\n        return 1\n    }\n}\nFactory::make\n";
        assert_eq!(
            refs_at(src, 1, 9),
            vec![1, 5],
            "decl + the `Factory::make` class-proc dispatch"
        );
    }

    /// FP guard: a bare two-word `Factory make` must not be treated as a
    /// class-proc dispatch either — itcl's real syntax for creating (and
    /// naming) a new instance is `ClassName instanceName`, so `Factory make`
    /// in real itcl code names a *new object* called `make`, never a call to
    /// the class-scoped `proc make`.  Folding every `class_methods` entry
    /// into the bare two-word `cmd_set` regardless of definer family would
    /// wrongly count (and rewrite, under rename) this unrelated construct.
    #[test]
    fn fp_itcl_bare_two_word_text_is_not_confused_with_class_proc_dispatch() {
        let src =
            "itcl::class Factory {\n    proc make {} {\n        return 1\n    }\n}\nFactory make\n";
        assert_eq!(
            refs_at(src, 1, 9),
            vec![1],
            "declaration only — `Factory make` is itcl instance creation, not a class-proc dispatch"
        );
    }
}

// ────────── #990 — [incr Tcl] `Factory::make` class-proc dispatch ──────────

/// itcl gives every class a real namespace of the same name and installs its
/// class-scoped `proc`s as ordinary commands inside it, so `Factory`'s `proc
/// make` is genuinely the command `::app::Factory::make`.
///
/// Oracle (tclsh 8.6.14 + Itcl 3.4), for a class declared in `::app`:
///
/// | spelling | from | result |
/// |---|---|---|
/// | `::app::Factory::make` | anywhere | dispatches |
/// | `app::Factory::make` | global / any namespace | dispatches |
/// | `Factory::make` | inside `namespace eval ::app` | dispatches |
/// | `Factory::make` | global or `::other` | `invalid command name` |
/// | `Factory::make` | inside the class's own body | dispatches (itcl's own class-namespace resolver) |
/// | `Other::omake` | inside `Factory`'s body | `invalid command name` |
/// | bare `make` | inside any of the class's bodies | dispatches |
/// | `::Factory::inst` (an *instance* method) | anywhere | `namespace "::" is not a class namespace` |
mod itcl_class_proc_dispatch {
    use super::*;

    fn itcl_src() -> String {
        concat!(
            "namespace eval ::app {\n",
            "    itcl::class Factory {\n",
            "        proc make {} { return 1 }\n",
            "        proc other {} { return [make] }\n",
            "        method inst {} { return [Factory::make] }\n",
            "    }\n",
            "    proc caller {} { return [Factory::make] }\n",
            "}\n",
            "::app::Factory::make\n",
            "app::Factory::make\n",
        )
        .to_owned()
    }

    /// FN→TP: every real dispatch spelling is a reference to the declaration.
    #[test]
    fn tp_every_dispatch_spelling_is_a_reference() {
        assert_eq!(
            refs_at(&itcl_src(), 2, 14),
            vec![2, 3, 4, 6, 8, 9],
            "decl + bare sibling + self-qualified + namespace-relative + absolute + global-relative"
        );
    }

    /// FN→TP: the reverse direction — the cursor on a call site finds the
    /// same set.
    #[test]
    fn tp_references_from_the_call_site_match_the_declaration() {
        let src = itcl_src();
        assert_eq!(refs_at(&src, 8, 18), refs_at(&src, 2, 14));
    }

    /// TN: the same text written where the class is *not* reachable resolves
    /// to nothing — real Tcl answers `invalid command name` there.
    #[test]
    fn tn_unreachable_spelling_is_not_a_reference() {
        let src = concat!(
            "namespace eval ::app {\n",
            "    itcl::class Factory {\n",
            "        proc make {} { return 1 }\n",
            "    }\n",
            "}\n",
            "Factory::make\n",
        );
        assert_eq!(refs_at(src, 2, 14), vec![2], "declaration only");
    }

    /// TN: an ordinary `::`-qualified proc call is untouched — `Factory` here
    /// is a plain namespace, not a class, so this stays a proc reference and
    /// never routes through the class-proc resolver.
    #[test]
    fn tn_plain_namespace_qualified_proc_call_is_still_a_proc_reference() {
        let src = "namespace eval ::Factory {\n    proc make {} { return 1 }\n}\nFactory::make\n";
        assert_eq!(refs_at(src, 1, 9), vec![1, 3], "decl + the proc call site");
    }

    /// FP guard: a *sibling* itcl class's simple name does not resolve from
    /// inside another class's body — itcl's class-namespace resolver covers
    /// the enclosing class only (`Other::omake` errors there).
    #[test]
    fn fp_sibling_class_simple_name_does_not_resolve_from_another_class_body() {
        let src = concat!(
            "namespace eval ::app {\n",
            "    itcl::class Other {\n",
            "        proc omake {} { return 1 }\n",
            "    }\n",
            "    itcl::class Factory {\n",
            "        proc probe {} { return [Other::omake] }\n",
            "    }\n",
            "}\n",
        );
        assert_eq!(refs_at(src, 2, 14), vec![2], "declaration only");
    }

    /// FP guard: an itcl *instance* method is not addressable as
    /// `Class::method` — only class-scoped `proc`s are — so that spelling
    /// resolves to nothing.
    #[test]
    fn fp_instance_method_is_not_addressable_as_a_qualified_name() {
        let src = concat!(
            "itcl::class Factory {\n",
            "    method inst {} { return 1 }\n",
            "}\n",
            "Factory::inst\n",
        );
        assert_eq!(refs_at(src, 1, 12), vec![1], "declaration only");
    }

    /// FN→TP: itcl's class-namespace resolver **pre-empts** the stock global
    /// fallback.  A global `::Factory::make` proc exists here, which stock
    /// resolution reaches from inside the class's own namespace — but itcl
    /// installs a custom command resolver on every class namespace and Tcl
    /// consults it first, so the class's own bodies still dispatch the class
    /// proc.
    ///
    /// Oracle (tclsh 8.6 + Itcl 3.4, probe `itcl2.tcl` — this script's exact
    /// shape):
    ///
    /// ```text
    /// A) from inside class proc  : ITCL-CLASSPROC
    /// B) from inside method      : ITCL-CLASSPROC
    /// C) from ::app              : ITCL-CLASSPROC
    /// D) from global             : GLOBAL-NS-PROC
    /// E) from ::other            : GLOBAL-NS-PROC
    /// ```
    #[test]
    fn fn_to_tp_the_class_resolver_pre_empts_a_global_namespace_proc() {
        let src = concat!(
            "namespace eval ::Factory { proc make {} { return \"GLOBAL-NS-PROC\" } }\n",
            "namespace eval ::app {\n",
            "    itcl::class Factory {\n",
            "        proc make {} { return \"ITCL-CLASSPROC\" }\n",
            "        proc probeSelf {} { return [Factory::make] }\n",
            "        method viaSelf {} { return [Factory::make] }\n",
            "    }\n",
            "}\n",
            "Factory::make\n",
        );
        let analysis = analyse(src);
        // Inside the class's own bodies (a class `proc` body and a `method`
        // body) the class proc wins, even though `::Factory::make` exists.
        for (line, character) in [(4, 37), (5, 39)] {
            assert_eq!(
                tcl_lsp_core::definition::itcl_class_proc_target_at(
                    src, tcl_dialect::DialectProfile::by_name("tcl8.6"), line, character, &analysis
                ),
                Some(("::app::Factory".to_owned(), "make".to_owned())),
                "line {line}: itcl's class resolver must pre-empt the global proc",
            );
            // Go-to-definition lands on the class proc's declaration (line 3),
            // not the global proc's (line 0).
            assert_eq!(
                ref_lines(&tcl_lsp_core::definition::definition(
                    src, line, character, &analysis
                )),
                vec![3],
                "line {line}: go-to-definition must reach the class proc",
            );
        }
        // From the global namespace the ordinary proc wins (oracle rows D/E).
        assert_eq!(
            tcl_lsp_core::definition::itcl_class_proc_target_at(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), 8, 2, &analysis),
            None,
            "from global, `Factory::make` is the plain ::Factory::make proc",
        );
        assert_eq!(
            ref_lines(&tcl_lsp_core::definition::definition(src, 8, 2, &analysis)),
            vec![0],
            "go-to-definition from global reaches the plain proc",
        );
    }

    /// FP guard: a real proc at a higher-priority resolution candidate
    /// shadows the class proc, exactly as it would at runtime — oracle
    /// (tclsh 8.6.14 + Itcl 3.4): this exact script prints
    /// `from ::app -> shadowing-proc` and `from global -> class-proc`.
    #[test]
    fn fp_a_shadowing_proc_wins_over_the_class_proc() {
        let src = concat!(
            "itcl::class Factory {\n",
            "    proc make {} { return \"class-proc\" }\n",
            "}\n",
            "namespace eval ::app {\n",
            "    namespace eval Factory {}\n",
            "    proc Factory::make {} { return \"shadowing-proc\" }\n",
            "    Factory::make\n",
            "}\n",
            "Factory::make\n",
        );
        assert_eq!(
            refs_at(src, 6, 14),
            vec![5, 6],
            "the shadowing proc's declaration + the `::app` call — not the class proc, \
             and not the global call that still reaches the class proc"
        );
    }
}

// ───────────────────────── #957 — `my method` references ──────────────────

mod my_method_dispatch {
    use super::*;

    /// FN→TP: the reported shape — `my getOptions` nested in `return [ … ]`.
    #[test]
    fn tp_my_dispatch_in_command_substitution() {
        let src = "oo::class create Bar957 {\n    method getOptions {key} { return $key }\n    method get {key} { return [my getOptions $key] }\n}\n";
        // cursor on `getOptions` declaration (line 1, col 11).
        assert_eq!(refs_at(src, 1, 11), vec![1, 2], "decl + `my` in [ … ]");
    }

    /// TP: a bare top-level `my getOptions` (not inside `[ … ]`) is also a ref.
    #[test]
    fn tp_my_dispatch_top_level() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method run {} { my getOptions x }\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1, 2], "decl + bare `my`");
    }

    /// TP: `my getOptions` embedded in a quoted / compound word
    /// (`"opts: [my getOptions $k]"`) is still a reference — the segmenter
    /// merges the whole word into one token, so the substitution is recovered
    /// by re-lexing the word slice.
    #[test]
    fn tp_my_dispatch_in_quoted_word() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} { return \"opts: [my getOptions $k]\" }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 2],
            "decl + `my` inside a quote"
        );
    }

    /// TP: `my getOptions` in a bareword concatenation (`[my getOptions]x`).
    #[test]
    fn tp_my_dispatch_in_compound_word() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} { return [my getOptions $k]-tail }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 2],
            "decl + `my` in a compound word"
        );
    }

    /// FP guard: `my other` must not count as a reference to `getOptions`.
    #[test]
    fn fp_my_other_method_excluded() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method other {k} { return $k }\n    method run {} { my other 1 }\n}\n";
        // references to `getOptions` (line 1) must be its declaration only.
        assert_eq!(refs_at(src, 1, 11), vec![1], "no `my other` leakage");
    }

    /// FP guard: a bare `getOptions` head (no `my`, no object) is *not* a call —
    /// a `TclOO` method is not a command in the body namespace.
    #[test]
    fn fp_bare_head_is_not_a_call() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method run {} { getOptions 1 }\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1], "bare head is not a dispatch");
    }

    /// FN→TP (issue #957's general form, reopened after the `[...]`-only
    /// fix): a `my method` dispatch nested inside `if` / `while` /
    /// `foreach` / `try` / `catch` / `eval` bodies is a reference.  The
    /// registry-driven `Plain`-`BodyKind` recursion (`plain_body_arg_indices`)
    /// covers every same-frame body generically — no per-command-name
    /// branch is needed for any of these.
    #[test]
    fn tp_my_dispatch_nested_in_if() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        if {1} {\n            my getOptions $k\n        }\n    }\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1, 4], "decl + `my` inside `if`");
    }

    #[test]
    fn tp_my_dispatch_nested_in_while() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        while {1} {\n            my getOptions $k\n            break\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 4],
            "decl + `my` inside `while`"
        );
    }

    #[test]
    fn tp_my_dispatch_nested_in_foreach() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        foreach x $k {\n            my getOptions $x\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 4],
            "decl + `my` inside `foreach`"
        );
    }

    #[test]
    fn tp_my_dispatch_nested_in_try_and_catch() {
        let try_src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        try {\n            my getOptions $k\n        } on error e {\n            puts $e\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(try_src, 1, 11),
            vec![1, 4],
            "decl + `my` inside `try`"
        );
        let catch_src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        catch {\n            my getOptions $k\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(catch_src, 1, 11),
            vec![1, 4],
            "decl + `my` inside `catch`"
        );
    }

    #[test]
    fn tp_my_dispatch_nested_in_eval() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        eval {my getOptions $k}\n    }\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1, 3], "decl + `my` inside `eval`");
    }

    /// TP: `switch`'s inline pattern/body-pairs form (form 1) — already
    /// standalone-body-per-pair, so this worked before the fix too, but is
    /// pinned here alongside form 2 for the full switch matrix.
    #[test]
    fn tp_my_dispatch_in_switch_inline_form() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        switch -- $k a {\n            my getOptions $k\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 4],
            "decl + `my` inside inline switch arm"
        );
    }

    /// FN→TP: `switch`'s single-braced clause-list form (form 2) — the
    /// registry's `switch_arg_roles` marks the *whole* clause list
    /// `ArgRole::Body`, so naively re-segmenting it as one script misreads
    /// `pattern { body }` as one bogus command and never reaches the arm
    /// body.  Splitting it via the registry's `case_list` vocabulary
    /// (never a hardcoded "switch" check) recovers each arm's own body.
    #[test]
    fn tp_my_dispatch_in_switch_braced_form() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        switch -- $k {\n            default {\n                my getOptions $k\n            }\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 5],
            "decl + `my` inside braced switch arm"
        );
    }

    /// Tcl 8.4 scans the first word of this two-argument call as an option,
    /// so the call has no valid subject and its clause-list word is not a
    /// body. Tcl 8.5+ stops option scanning at the reserved trailing words;
    /// the same source then reaches the nested procedure call. The release
    /// distinction comes from the registry's CaseListSpec/profile pair, not
    /// from this LSP consumer.
    #[test]
    fn two_argument_switch_body_follows_tcl_release() {
        for subject in ["-regexp", "--"] {
            let src = format!(
                "proc hit {{}} {{ return hit }}\nswitch {subject} {{\n    default {{\n        hit\n    }}\n}}\n"
            );
            assert_eq!(
                refs_at_dialect(&src, "tcl8.4", 0, 5),
                vec![0],
                "Tcl 8.4 must not descend the invalid two-argument switch word {subject:?}"
            );
            for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
                assert_eq!(
                    refs_at_dialect(&src, dialect, 0, 5),
                    vec![0, 3],
                    "{dialect} must descend the valid two-argument switch body with subject {subject:?}"
                );
            }
        }
    }

    #[test]
    fn irules_switch_glob_body_is_followed() {
        let src = "proc hit {} { return hit }\nswitch -glob $value {\n    default {\n        hit\n    }\n}\n";
        assert_eq!(
            refs_at_dialect(src, "f5-irules", 0, 5),
            vec![0, 3],
            "iRules switch -glob must descend its clause-list body"
        );
    }

    /// TP: arbitrarily-deep combinations of control flow, `[...]`
    /// substitution, and `eval` all compose — the recursion is generic,
    /// not a fixed nesting-depth allowance.
    #[test]
    fn tp_my_dispatch_deeply_nested_combination() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        foreach x $k {\n            if {$x} {\n                switch -- $x {\n                    default {\n                        return [my getOptions $x]\n                    }\n                }\n            }\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 7],
            "decl + `my` nested 4 levels deep"
        );
    }

    /// TP: the generic `Plain`-`BodyKind` mechanism covers *any* command
    /// whose registry spec declares a same-frame body — `dict for` here,
    /// not just the hand-enumerable control-flow keywords — proving the
    /// fix is registry-driven rather than a hardcoded command list.
    #[test]
    fn tp_my_dispatch_nested_in_dict_for() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {opts} {\n        dict for {k v} $opts {\n            my getOptions $k\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 4],
            "decl + `my` inside `dict for`"
        );
    }

    /// FP guard: `my other` nested arbitrarily deep in control flow must
    /// still not count as a reference to `getOptions` — the recursion must
    /// not blur distinct method names together just because it now looks
    /// inside more constructs.
    #[test]
    fn fp_my_other_method_excluded_when_control_flow_nested() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method other {k} { return $k }\n    method run {k} {\n        if {1} {\n            switch -- $k {\n                default {\n                    my other $k\n                }\n            }\n        }\n    }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1],
            "no `my other` leakage through nested control flow"
        );
    }

    /// TN / documented limitation: `uplevel 1 { my getOptions }` runs in a
    /// *different* call frame (the registry marks `uplevel`'s body
    /// `BodyKind::Structural` for exactly this reason — level `0` and
    /// level `1`+ can't be told apart from the static spec alone), and a
    /// bare `apply {{} { my getOptions }}` lambda likewise gets its own
    /// frame with no route back to the enclosing object's `my` unless the
    /// lambda is explicitly constructed with the object's namespace
    /// (confirmed Tcl semantics: `apply`'s body runs in the *global*
    /// namespace by default). Both are conservatively **not** followed —
    /// matching this codebase's "fall through rather than guess wrong"
    /// rule — rather than guessing whether the frame still resolves `my`.
    #[test]
    fn tn_uplevel_and_apply_bodies_conservatively_not_followed() {
        let uplevel_src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        uplevel 1 {my getOptions $k}\n    }\n}\n";
        assert_eq!(
            refs_at(uplevel_src, 1, 11),
            vec![1],
            "uplevel body not followed"
        );
        let apply_src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        apply {{} {my getOptions $k}}\n    }\n}\n";
        assert_eq!(
            refs_at(apply_src, 1, 11),
            vec![1],
            "apply lambda body not followed"
        );
    }
}

// ─────────────── #957 (general form) — `next` / `nextto` references ───────

mod next_dispatch {
    use super::*;

    /// FN→TP: `next` inside `Bar::getOptions`'s own body is a polymorphic
    /// reference to `Bar::getOptions` itself (the established attribution:
    /// a `next`/`nextto` site counts toward whichever method's body it is
    /// *written inside*, not the ancestor it dispatches to at runtime —
    /// `next` never mentions a method name for a rename to rewrite, so
    /// there is nothing there to attribute to the ancestor). This must be
    /// found however deeply the `next` call is nested in control flow.
    #[test]
    fn tp_next_dispatch_nested_in_if() {
        let src = "oo::class create Base {\n    method getOptions {key} { return $key }\n}\noo::class create Bar {\n    superclass Base\n    method getOptions {key} {\n        if {1} {\n            next $key\n        }\n    }\n}\n";
        // cursor on Bar's own `getOptions` declaration (line 5, col 11).
        assert_eq!(
            refs_at(src, 5, 11),
            vec![5, 7],
            "Bar decl + nested `next` site"
        );
    }

    /// TP: same for `nextto CLASS`, and at the top level (no extra nesting).
    #[test]
    fn tp_nextto_dispatch_top_level() {
        let src = "oo::class create Base {\n    method getOptions {key} { return $key }\n}\noo::class create Bar {\n    superclass Base\n    method getOptions {key} {\n        nextto Base $key\n    }\n}\n";
        assert_eq!(refs_at(src, 5, 11), vec![5, 6], "Bar decl + `nextto` site");
    }

    /// FP guard: cursor on the *superclass*'s own declaration does not pick
    /// up a subclass override's `next` call — `Base::getOptions`'s own
    /// body has no `next`/`nextto` of its own.
    #[test]
    fn fp_superclass_declaration_excludes_subclass_next_site() {
        let src = "oo::class create Base {\n    method getOptions {key} { return $key }\n}\noo::class create Bar {\n    superclass Base\n    method getOptions {key} {\n        next $key\n    }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1],
            "Base decl only — Bar's `next` is not Base's own"
        );
    }
}

// ─────────────────── #958 — `::tcl::mathfunc` expr functions ──────────────

mod mathfunc_expr {
    use super::*;

    /// FN→TP: the reported shape — `Pi958()` inside `[expr { … }]` dispatches to
    /// the command `::tcl::mathfunc::Pi958`.
    #[test]
    fn tp_mathfunc_in_nested_expr_subst() {
        let src = "namespace eval ::tcl::mathfunc {\n    proc Pi958 {} { return 3.14 }\n}\nset x [expr {Pi958() / 2.0}]\n";
        // cursor on the `Pi958` proc name (line 1, col 9).
        assert_eq!(refs_at(src, 1, 9), vec![1, 3], "decl + expr-fn call");
    }

    /// TP: the same via a top-level `expr` command (not nested in `[ … ]`).
    #[test]
    fn tp_mathfunc_in_top_level_expr() {
        let src = "namespace eval ::tcl::mathfunc {\n    proc Half {x} { return [expr {$x / 2.0}] }\n}\nexpr {Half(4)}\n";
        let got = refs_at(src, 1, 9);
        assert!(got.contains(&1) && got.contains(&3), "decl + call: {got:?}");
    }

    /// FP guard: a *different* math function must not be pulled in — `Tau()`
    /// is not a reference to `Pi958`.
    #[test]
    fn fp_other_mathfunc_excluded() {
        let src = "namespace eval ::tcl::mathfunc {\n    proc Pi958 {} { return 3.14 }\n    proc Tau958 {} { return 6.28 }\n}\nset x [expr {Tau958()}]\n";
        assert_eq!(refs_at(src, 1, 9), vec![1], "only Pi958's declaration");
    }

    /// TN: a math-func proc that is never applied has only its declaration.
    #[test]
    fn tn_unused_mathfunc_only_declaration() {
        let src = "namespace eval ::tcl::mathfunc {\n    proc Unused {} { return 0 }\n}\n";
        assert_eq!(refs_at(src, 1, 9), vec![1], "declaration only");
    }
}

// ─────────────────────── #955 — pkgIndex `$dir` W210 ──────────────────────

mod pkgindex_dir {
    use super::*;

    const READ_DIR: &str = "set f [file join $dir pkg.tcl]\nsource $f\n";

    /// FN→(no diagnostic): reading `$dir` in a `pkgIndex.tcl` is not
    /// read-before-set — the loader sets it before the script runs.
    #[test]
    fn tp_dir_read_in_pkgindex_suppressed() {
        assert!(
            !has_w210(READ_DIR, Some("/proj/pkgIndex.tcl")),
            "$dir in pkgIndex.tcl must not draw W210"
        );
    }

    /// FP guard: the suppression is filename-scoped — the same `$dir` read in
    /// an ordinary file is a genuine read-before-set.
    #[test]
    fn fp_dir_read_in_ordinary_file_still_fires() {
        assert!(
            has_w210(READ_DIR, Some("/proj/other.tcl")),
            "$dir outside pkgIndex.tcl is still read-before-set"
        );
    }

    /// FP guard: only `dir` is implicit — an unrelated undefined read in a
    /// `pkgIndex.tcl` still draws W210.
    #[test]
    fn fp_other_undefined_var_in_pkgindex_still_fires() {
        let src = "set f [file join $dir $missing]\nsource $f\n";
        assert!(
            has_w210(src, Some("/proj/pkgIndex.tcl")),
            "an undefined `$missing` is still read-before-set in pkgIndex.tcl"
        );
    }

    /// FP guard: the gate matches the *basename* exactly, so a look-alike
    /// filename that merely ends in `pkgIndex.tcl` (`notpkgIndex.tcl`) is not
    /// a package index and its `$dir` read still fires.
    #[test]
    fn fp_lookalike_filename_still_fires() {
        assert!(
            has_w210(READ_DIR, Some("/proj/notpkgIndex.tcl")),
            "a file merely ending in `pkgIndex.tcl` is not a package index"
        );
    }

    /// TN: a `dir` the script sets itself reads cleanly everywhere.
    #[test]
    fn tn_locally_set_dir_is_clean() {
        let src = "set dir /tmp\nset f [file join $dir pkg.tcl]\n";
        assert!(
            !has_w210(src, Some("/proj/other.tcl")),
            "a locally-set var is defined before read"
        );
    }
}

// ──────────────────────── #954 — apply lambda tokens ──────────────────────

mod apply_lambda {
    use super::*;

    /// FN→TP: commands inside an `apply` lambda body are highlighted as a
    /// script (the reported bug), and the bare arg-list name is a parameter.
    #[test]
    fn tp_bare_arglist_param_and_body() {
        let src = "apply {dir {\n    puts $dir\n}} /tmp\n";
        assert_eq!(
            kind_of(src, "tcl", "dir").as_deref(),
            Some("parameter"),
            "bare arg-list `dir` is a parameter declaration"
        );
        assert_eq!(
            kind_of(src, "tcl", "puts").as_deref(),
            Some("function"),
            "the body command `puts` is highlighted as a function"
        );
    }

    /// TP: a braced arg-list keeps its parameter names.
    #[test]
    fn tp_braced_arglist_params() {
        let src = "apply {{a b} { expr {$a + $b} }} 1 2\n";
        assert_eq!(kind_of(src, "tcl", "a").as_deref(), Some("parameter"));
        assert_eq!(kind_of(src, "tcl", "b").as_deref(), Some("parameter"));
    }

    /// FP guard: `apply $lambda` is a runtime value, not a literal — the
    /// `$lambda` word stays a variable and is never split into a lambda.
    #[test]
    fn fp_variable_lambda_not_recursed() {
        let src = "apply $lambda a b\n";
        assert_eq!(
            kind_of(src, "tcl", "lambda").as_deref(),
            Some("variable"),
            "`$lambda` is a variable, not a lambda literal"
        );
    }
}

// ───────────── #1019 idx 16 — non-identifier method names ─────────────

/// Tcl puts no character restriction on a method name.  The cursor-word
/// rule used to stop at `-`, `<`, and `>`, so a hyphenated method and TIP
/// 558's generated property accessors (`<ReadProp-NAME>` /
/// `<WriteProp-NAME>`) were truncated to a name no class declares, and never
/// resolved from a call site.
///
/// Oracle (tclsh 8.6.14 + 9.0.4): `method with-dash`, `method a.b`, and
/// `method <ReadProp-x>` all define real, dispatchable methods, and
/// `oo::configurable`'s `property x` generates exactly `<ReadProp-x>` /
/// `<WriteProp-x>`.  Only `with-dash` / `a.b` are *exported* (`TclOO` exports
/// a method iff its name starts with an ASCII lowercase letter), so the
/// angle-bracketed pair is reachable through `my` only.
mod non_identifier_method_names {
    use super::*;

    /// FN→TP: a hyphenated method's external call site is a reference.
    #[test]
    fn tp_hyphenated_method_call_site_is_a_reference() {
        let src = "oo::class create C {\n    method with-dash {} { return 1 }\n}\nC create rex\nrex with-dash\n";
        assert_eq!(refs_at(src, 1, 13), vec![1, 4], "decl + `rex with-dash`");
    }

    /// FN→TP: the cursor *on the call site* resolves back to the declaration
    /// too — the reverse direction, which shares the same word rule.
    #[test]
    fn tp_hyphenated_method_references_from_the_call_site() {
        let src = "oo::class create C {\n    method with-dash {} { return 1 }\n}\nC create rex\nrex with-dash\n";
        assert_eq!(refs_at(src, 4, 7), vec![1, 4]);
    }

    /// FN→TP: the TIP 558 accessor shape, dispatched internally via `my`.
    #[test]
    fn tp_angle_bracketed_property_accessor_my_dispatch_is_a_reference() {
        let src = "oo::class create C {\n    method <ReadProp-x> {} { return 2 }\n    method probe {} { my <ReadProp-x> }\n}\n";
        assert_eq!(refs_at(src, 1, 13), vec![1, 2], "decl + `my <ReadProp-x>`");
    }

    /// TP: a dotted method name is one word too.
    #[test]
    fn tp_dotted_method_call_site_is_a_reference() {
        let src =
            "oo::class create C {\n    method a.b {} { return 1 }\n}\nC create rex\nrex a.b\n";
        assert_eq!(refs_at(src, 1, 12), vec![1, 4]);
    }

    /// FP guard: `$x-1` is arithmetic, not a `-1` method on `$x`, even when
    /// `x` really does hold an object whose class has methods.
    #[test]
    fn fp_subtraction_on_an_object_variable_is_not_a_method_call() {
        let src = "oo::class create C {\n    method with-dash {} { return 1 }\n}\nset x [C new]\nset y [expr {$x-1}]\n";
        assert_eq!(refs_at(src, 1, 13), vec![1], "declaration only");
    }

    /// TN: a hyphenated *option flag* written after an unrelated command is
    /// not a reference to a same-named method.
    #[test]
    fn tn_option_flag_is_not_a_method_reference() {
        let src = "oo::class create C {\n    method with-dash {} { return 1 }\n}\nputs -nonewline with-dash\n";
        assert_eq!(refs_at(src, 1, 13), vec![1], "declaration only");
    }
}

// ─────── #981 — namespace-aware bare class-command dispatch matching ───────

/// A bare `Factory make` is resolved the way Tcl resolves it — current
/// namespace first, then global — instead of by matching the class's simple
/// name as text.  Two classes sharing a tail name in different namespaces are
/// therefore no longer cross-linked, which matters because since #1047 this
/// one scanner feeds references, rename (including consumer-document edits),
/// the code lens count and click, and call hierarchy: a wrong match rewrites
/// real code.
///
/// Oracle, identical on tclsh 8.6.14 and 9.0.4, for classes in `::a` and
/// `::b` each declaring `make`:
///
/// | written | in | result |
/// |---|---|---|
/// | `Factory make` | `::b` | `b-made` — `::b::Factory`, never `::a::Factory` |
/// | `Factory make` | `::a` | `a-made` |
/// | `Factory make` | `::c`, with no `::c::Factory` and no `::Factory` | `invalid command name "Factory"` |
/// | `Factory make` | `::d`, with a global `::Factory` | the global class |
/// | `::a::Factory make` | global | `a-made` |
mod namespace_scoped_class_dispatch {
    use super::*;

    /// Two same-tailed classes, one dispatch each.
    fn two_namespaces() -> &'static str {
        concat!(
            "namespace eval ::a {\n",
            "    oo::class create Factory {\n",
            "        classmethod make {} { return 1 }\n",
            "    }\n",
            "    Factory make\n",
            "}\n",
            "namespace eval ::b {\n",
            "    oo::class create Factory {\n",
            "        classmethod make {} { return 2 }\n",
            "    }\n",
            "    Factory make\n",
            "}\n",
            "::a::Factory make\n",
        )
    }

    /// TN (the issue's own repro) + TP in one: `::a::Factory::make` counts
    /// its own namespace's bare dispatch and the absolute spelling, and does
    /// **not** count `::b`'s bare dispatch.
    #[test]
    fn tn_bare_dispatch_in_a_sibling_namespace_is_not_cross_linked() {
        assert_eq!(
            refs_at(two_namespaces(), 2, 20),
            vec![2, 4, 12],
            "decl + the `::a` bare dispatch + the absolute `::a::Factory make`"
        );
    }

    /// TN, the mirror direction: `::b::Factory::make` likewise keeps only its
    /// own sites.
    #[test]
    fn tn_the_mirror_direction_is_scoped_too() {
        assert_eq!(
            refs_at(two_namespaces(), 8, 20),
            vec![8, 10],
            "decl + the `::b` bare dispatch only"
        );
    }

    /// TP: the global-fallback case still matches — a bare `Factory make`
    /// inside a namespace that declares no `Factory` reaches the global class.
    #[test]
    fn tp_global_fallback_dispatch_still_matches() {
        let src = concat!(
            "oo::class create Factory {\n",
            "    classmethod make {} { return 1 }\n",
            "}\n",
            "namespace eval ::d {\n",
            "    Factory make\n",
            "}\n",
            "Factory make\n",
        );
        assert_eq!(
            refs_at(src, 1, 16),
            vec![1, 4, 6],
            "decl + the `::d` fallback dispatch + the top-level one"
        );
    }

    /// TN: where neither the current namespace nor the global namespace has a
    /// `Factory`, the call is `invalid command name` in real Tcl — so it is
    /// not a reference to the class in some *other* namespace.
    #[test]
    fn tn_dispatch_with_no_reachable_candidate_matches_nothing() {
        let src = concat!(
            "namespace eval ::a {\n",
            "    oo::class create Factory {\n",
            "        classmethod make {} { return 1 }\n",
            "    }\n",
            "}\n",
            "namespace eval ::c {\n",
            "    Factory make\n",
            "}\n",
        );
        assert_eq!(refs_at(src, 2, 20), vec![2], "declaration only");
    }

    /// FP guard: a same-named *proc* in the calling namespace shadows the
    /// class, so the bare call is not a class dispatch at all.
    #[test]
    fn fp_a_shadowing_proc_in_the_calling_namespace_wins() {
        let src = concat!(
            "oo::class create Factory {\n",
            "    classmethod make {} { return 1 }\n",
            "}\n",
            "namespace eval ::e {\n",
            "    proc Factory {args} { return 2 }\n",
            "    Factory make\n",
            "}\n",
        );
        assert_eq!(refs_at(src, 1, 16), vec![1], "declaration only");
    }

    /// TP: the object-command (`CLASS create NAME`) matcher keeps working —
    /// a bare `rex make` on an instance command still resolves.
    #[test]
    fn tp_object_command_dispatch_still_matches() {
        let src = concat!(
            "namespace eval ::a {\n",
            "    oo::class create Factory {\n",
            "        method make {} { return 1 }\n",
            "    }\n",
            "    Factory create rex\n",
            "    rex make\n",
            "}\n",
        );
        assert_eq!(refs_at(src, 2, 15), vec![2, 5], "decl + `rex make`");
    }

    /// TN + TP, the object-command half of #981 (closed by PR C3).
    ///
    /// `CLASS create NAME` binds `NAME` in the **creation site's** namespace,
    /// so `::a::Factory create rex` and `::b::Widget create rex` are two
    /// coexisting commands.  Oracle, identical on tclsh 9.0.4 and 8.6.16:
    ///
    /// ```text
    /// in ::a -> a-made
    /// in ::b -> b-made
    /// global a: a-made        ;# ::a::rex make
    /// global b: b-made        ;# ::b::rex make
    /// ```
    ///
    /// Before this fix `created_instance_commands` was a flat bare-name set:
    /// `::b::Widget::make` counted (and renamed) `::a`'s `rex make`, while
    /// `::a::Factory::make` lost its own call site entirely to
    /// last-write-wins.
    #[test]
    fn tn_object_command_dispatch_is_scoped_to_its_creation_namespace() {
        let src = concat!(
            "namespace eval ::a {\n",
            "    oo::class create Factory {\n",
            "        method make {} { return 1 }\n",
            "    }\n",
            "    Factory create rex\n",
            "    rex make\n",
            "}\n",
            "namespace eval ::b {\n",
            "    oo::class create Widget {\n",
            "        method make {} { return 2 }\n",
            "    }\n",
            "    Widget create rex\n",
            "    rex make\n",
            "}\n",
        );
        assert_eq!(
            refs_at(src, 2, 15),
            vec![2, 5],
            "::a::Factory::make: decl + its OWN `rex make`, never ::b's"
        );
        assert_eq!(
            refs_at(src, 9, 15),
            vec![9, 12],
            "::b::Widget::make: decl + its OWN `rex make`, never ::a's"
        );
    }

    /// FN→TP: an **explicit** `namespace import` creates a real command in the
    /// importing namespace, so a bare dispatch through the imported name is a
    /// reference to the source class's method.  Post-#1047 this one scanner
    /// also drives rename, so missing it left the call site stale.
    ///
    /// Oracle (tclsh 9.0.4, probe `ns981.tcl`):
    ///
    /// ```text
    /// 1) import+bare in ::b : A-MADE
    ///    info commands ::b::Factory = ::b::Factory
    /// ```
    #[test]
    fn fn_to_tp_an_explicit_namespace_import_is_followed() {
        let src = concat!(
            "namespace eval ::a {\n",
            "    oo::class create Factory {\n",
            "        classmethod make {} { return 1 }\n",
            "    }\n",
            "    namespace export Factory\n",
            "}\n",
            "namespace eval ::b {\n",
            "    namespace import ::a::Factory\n",
            "    Factory make\n",
            "}\n",
        );
        assert_eq!(
            refs_at(src, 2, 20),
            vec![2, 8],
            "decl + the bare dispatch through the imported name"
        );
    }

    /// TN (deliberate boundary): a **wildcard** import is not followed.
    /// Reproducing it needs the export-gated import snapshot of issue #1027 —
    /// which commands existed in `::a` when the import ran — and inventing
    /// aliases without it would match sites the runtime never dispatches.
    /// Documented in this module's limitations list.
    #[test]
    fn tn_a_wildcard_import_is_not_followed() {
        let src = concat!(
            "namespace eval ::a {\n",
            "    oo::class create Factory {\n",
            "        classmethod make {} { return 1 }\n",
            "    }\n",
            "    namespace export Factory\n",
            "}\n",
            "namespace eval ::b {\n",
            "    namespace import ::a::*\n",
            "    Factory make\n",
            "}\n",
        );
        assert_eq!(
            refs_at(src, 2, 20),
            vec![2],
            "declaration only — the wildcard-import model is out of scope (#1027)"
        );
    }

    /// FP guard: importing a *different* command does not make an unrelated
    /// bare `Factory make` a reference — the alias only maps the name it
    /// really imports.
    #[test]
    fn fp_an_unrelated_import_does_not_match() {
        let src = concat!(
            "namespace eval ::a {\n",
            "    oo::class create Factory {\n",
            "        classmethod make {} { return 1 }\n",
            "    }\n",
            "    proc helper {} { return 2 }\n",
            "    namespace export helper\n",
            "}\n",
            "namespace eval ::b {\n",
            "    namespace import ::a::helper\n",
            "    Factory make\n",
            "}\n",
        );
        assert_eq!(refs_at(src, 2, 20), vec![2], "declaration only");
    }
}
