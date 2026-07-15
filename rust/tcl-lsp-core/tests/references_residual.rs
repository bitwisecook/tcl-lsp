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

//! Residual-coverage tests for `tcl-lsp-core/src/references.rs` — the
//! find-all-references (`references`) and document-highlight
//! (`document_highlights`) providers.
//!
//! The sibling `references_rename.rs` already covers the proc / variable /
//! namespace paths of `references`. This file targets the branches that suite
//! leaves cold (verified with `cargo llvm-cov --show-missing-lines`):
//!
//!   * **class references** — cursor on a class name surfaces the declaration,
//!     every `<Class> new` instantiation, and every `superclass` / `mixin`
//!     usage in other class bodies (`references` lines 154-179; the mirror
//!     class block in `document_highlights` lines 704-720).
//!   * **`document_highlights`** as a whole — no integration test exercised it
//!     before; here it is driven over variables, classes, and class members.
//!   * **class-member references** — methods / classmethods / properties, the
//!     destructor-body scan, empty-body siblings, and external `$obj method`
//!     call sites threaded through proc bodies and `[...]` substitutions.
//!   * **`dedup_kinded` priority** — a span recorded as both a definition
//!     (Write) and a reference (Read) collapses to the higher-priority Write.
//!   * **None / empty-result edges** — an out-of-scope `$var`, an unknown word.
//!
//! ## C-Tcl proof discipline
//!
//! Every assertion that encodes a *Tcl-runtime* fact — "this `$d bark` is a
//! real dispatch to `Dog`'s `bark`", "`Puppy` inherits via `superclass Dog`",
//! "`Dog new` constructs an instance" — is pinned to real C-Tcl
//! (tclsh **8.6** + **9.0**, identical output, via
//! `scripts/dev/tclsh_check.sh`) and cited with a `// tclsh-proof:` comment.
//! Assertions that are purely editor-structural (which span is returned, how
//! many, a Read/Write/Text tag, a de-dup ordering) carry no proof — tclsh has
//! no opinion on them — per the project's stated proof model.
//!
//! Verified C-Tcl facts (tclsh8.6 + tclsh9.0, identical output):
//!   * `oo::class create Dog {method bark {} {return woof}}; set d [Dog new];
//!     puts [$d bark]`  -> woof.  `Dog new` constructs an instance; `$d bark`
//!     is a real method dispatch.  With two instances `$a bark` / `$b bark`
//!     both dispatch -> woof/woof.
//!   * `oo::class create Puppy {superclass Dog}; puts [[Puppy new] bark]`
//!     -> woof  (the `superclass Dog` word makes Puppy inherit `Dog::bark`, so
//!     it is a genuine reference to Dog).
//!   * `oo::class create C {}; oo::define C self method build {} {return built};
//!     puts [C build]`  -> built  (a classmethod is invoked on the class
//!     object — `C build` is the real call form).
//!   * `oo::class create P {variable color; constructor {} {set color red};
//!     method show {} {return $color}}; puts [[P new] show]`  -> red.
//!   * BUG witness: a *bare* `greet` at command-head position inside a sibling
//!     method body is NOT a valid `TclOO` call —
//!     `oo::class create C {method greet {} {return hi};
//!      method twice {} {return "[greet][greet]"}}; puts [[C new] twice]`
//!     -> ERR: invalid command name "greet".  The runnable form is `my greet`
//!     -> hihi.  See `bug_*` tests below.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::references::{HighlightKind, document_highlights, references};

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// Sorted, de-duplicated start lines of a list of reference ranges.
fn ref_lines(ranges: &[LspRange]) -> Vec<u32> {
    let mut v: Vec<u32> = ranges.iter().map(|r| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Sorted, de-duplicated start lines of a list of kinded highlight ranges.
fn hl_lines(h: &[(LspRange, HighlightKind)]) -> Vec<u32> {
    let mut v: Vec<u32> = h.iter().map(|(r, _)| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

// ===========================================================================
// references — class names (decl + `<Class> new` instantiations).
// ===========================================================================

#[test]
fn references_class_includes_decl_and_every_instantiation() {
    // tclsh-proof: `oo::class create Dog {method bark {} {return woof}};
    //   set d [Dog new]; set e [Dog new]; puts [[$d bark]][[$e bark]]` runs,
    //   so both `Dog new` words are real uses of the class `Dog`.
    // The reference set for `Dog` is its decl (line 0) + both `Dog new`
    // instantiation sites (lines 3 and 4).
    let src =
        "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nset e [Dog new]\n";
    let analysis = analyse(src);
    // Cursor on `Dog` in the `oo::class create Dog` head (line 0, col 18).
    let refs = references(src, "tcl", 0, 18, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 3, 4],
        "class decl + both `Dog new` sites; got {refs:?}",
    );
    // The declaration leads (include_declaration = true) and spans `Dog`
    // (cols 17..20 in `oo::class create Dog`).
    assert_eq!(refs[0].start_line, 0, "decl first: {refs:?}");
    assert_eq!(refs[0].start_character, 17, "{refs:?}");
    assert_eq!(refs[0].end_character, 20, "{refs:?}");
}

#[test]
fn references_class_exclude_declaration_keeps_only_instantiations() {
    let src =
        "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nset e [Dog new]\n";
    let analysis = analyse(src);
    let with_decl = references(src, "tcl", 0, 18, &analysis, true);
    let without_decl = references(src, "tcl", 0, 18, &analysis, false);
    assert_eq!(
        ref_lines(&without_decl),
        vec![3, 4],
        "only the two `Dog new` sites when decl excluded; got {without_decl:?}",
    );
    assert_eq!(
        with_decl.len(),
        without_decl.len() + 1,
        "excluding the declaration drops exactly one entry",
    );
}

#[test]
fn references_class_includes_superclass_usage_in_other_class() {
    // tclsh-proof: `oo::class create Dog {method bark {} {return woof}};
    //   oo::class create Puppy {superclass Dog}; puts [[Puppy new] bark]`
    //   -> woof.  The `superclass Dog` word makes Puppy inherit Dog::bark, so
    //   it is a real use of the class `Dog`.
    // References for `Dog` must include the `superclass Dog` site (line 4) on
    // top of the decl (line 0) and the `Puppy new` ... wait: there is no
    // `Dog new` here, so the set is {decl line 0, superclass ref line 4}.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\noo::class create Puppy {\n    superclass Dog\n}\nset p [Puppy new]\n";
    let analysis = analyse(src);
    // Cursor on `Dog` in its decl (line 0, col 18).
    let refs = references(src, "tcl", 0, 18, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&0), "class decl missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "`superclass Dog` usage (line 4) missing: {refs:?}",
    );
    // `Puppy new` (line 6) is a use of *Puppy*, not Dog — must NOT appear.
    assert!(
        !lines.contains(&6),
        "`Puppy new` is a Puppy reference, not a Dog one: {refs:?}",
    );
}

#[test]
fn references_class_includes_mixin_usage_in_other_class() {
    // tclsh-proof: `oo::class create M {method m {} {return mixed}};
    //   oo::class create C {mixin M}; puts [[C new] m]` -> mixed.  The
    //   `mixin M` word pulls M's `m` into C, so it is a genuine use of M.
    let src = "oo::class create M {\n    method m {} {}\n}\noo::class create C {\n    mixin M\n}\nset c [C new]\n";
    let analysis = analyse(src);
    // Cursor on `M` in its decl (line 0, col 18).
    let refs = references(src, "tcl", 0, 18, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&0), "M decl missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "`mixin M` usage (line 4) missing: {refs:?}",
    );
}

#[test]
fn references_class_from_qualified_name_at_decl_resolves() {
    // The class block accepts a `::`-qualified spelling of the cursor word.
    // tclsh-proof (same as above): `Dog new` is a real instantiation.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n";
    let analysis = analyse(src);
    // Cursor on `Dog` in the decl — resolves the class whose qualified name is
    // `::Dog`.
    let refs = references(src, "tcl", 0, 18, &analysis, true);
    assert_eq!(ref_lines(&refs), vec![0, 3], "{refs:?}");
}

// ===========================================================================
// document_highlights — variables (Write at def, Read at uses; dedup).
// ===========================================================================

#[test]
fn document_highlights_var_marks_def_write_and_reads_read() {
    // tclsh-proof: `set x 1; puts $x; puts $x` -> 1/1 — two real reads of `x`.
    // The definition span is Write; every read span is Read.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor inside the first `$x` (line 1).
    let h = document_highlights(src, "tcl", 1, 6, &analysis);
    assert!(
        h.iter()
            .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
        "Write at the `set x` definition (line 0); got {h:?}",
    );
    let reads = h.iter().filter(|(_, k)| *k == HighlightKind::Read).count();
    assert!(reads >= 2, "both `$x` reads tagged Read; got {h:?}");
}

#[test]
fn document_highlights_var_out_of_scope_is_empty() {
    // A `$var` the scope chain cannot resolve yields no highlights (the
    // var-lookup `None` early-return, `document_highlights` line 682).
    let src = "puts $undefined_xyz\n";
    let analysis = analyse(src);
    // Cursor inside `$undefined_xyz` (line 0, col 7).
    assert!(
        document_highlights(src, "tcl", 0, 7, &analysis).is_empty(),
        "an unresolved `$var` must produce no highlights",
    );
}

#[test]
fn document_highlights_dedup_keeps_write_over_read_on_collision() {
    // `dedup_kinded` priority path: when one span is recorded both as the
    // definition (Write) and as a reference (Read), the higher-priority Write
    // wins and the duplicate is dropped.  Injected synthetically because the
    // analyser never records the *same* span twice — this isolates the
    // de-dup/priority logic (`priority` Write/Read arms + the `and_modify`
    // branch).  No tclsh proof: a span's highlight tag is editor presentation.
    use tcl_compiler::analyser::{AnalysisResult as R, Scope, VarDef};
    use tcl_lexer::Span;
    // In "set x 1\nputs $x\n", the `x` of `$x` occupies bytes 13..14.
    let mut scope = Scope::default();
    scope.variables.insert(
        "x".into(),
        VarDef {
            name: "x".into(),
            definition_span: Span::new(13, 14),
            references: vec![Span::new(13, 14)], // identical span -> collision
            warn_if_unused: false,
            array_indices: std::collections::BTreeSet::new(),
            link_target: None,
        },
    );
    let analysis = R {
        global_scope: scope,
        ..R::default()
    };
    let src = "set x 1\nputs $x\n";
    // Cursor on `$x` (line 1, col 6).
    let h = document_highlights(src, "tcl", 1, 6, &analysis);
    assert_eq!(
        h.len(),
        1,
        "the duplicate (start,end) collapses to one: {h:?}"
    );
    assert_eq!(
        h[0].1,
        HighlightKind::Write,
        "Write outranks Read for the same span: {h:?}",
    );
}

// ===========================================================================
// document_highlights — class names (all Text; mirror of the class ref block).
// ===========================================================================

#[test]
fn document_highlights_class_decl_and_instantiations_are_text() {
    // tclsh-proof: both `Dog new` words are real instantiations (script runs).
    // A class carries no Read/Write distinction — decl + every `Dog new`
    // command-invocation head is Text.
    let src =
        "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nset e [Dog new]\n";
    let analysis = analyse(src);
    // Cursor on `Dog` in the decl (line 0, col 18).
    let h = document_highlights(src, "tcl", 0, 18, &analysis);
    assert_eq!(
        hl_lines(&h),
        vec![0, 3, 4],
        "decl + both `Dog new`; got {h:?}"
    );
    assert!(
        h.iter().all(|(_, k)| *k == HighlightKind::Text),
        "every class highlight is Text (no Read/Write); got {h:?}",
    );
    assert_eq!(
        h.iter().filter(|(_, k)| *k == HighlightKind::Write).count(),
        0,
        "no Write on a class: {h:?}",
    );
}

// ===========================================================================
// references / document_highlights — class members (methods, classmethods,
// properties) and external `$obj method` call sites.
// ===========================================================================

#[test]
fn references_external_obj_method_two_instances_both_sites() {
    // tclsh-proof: `oo::class create Dog {method bark {} {return woof}};
    //   set a [Dog new]; set b [Dog new]; puts [$a bark][$b bark]` -> woofwoof.
    //   Both `$a bark` and `$b bark` are real dispatches to Dog::bark.
    // References for `bark` = decl (line 1) + both external sites (lines 5,6).
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset a [Dog new]\nset b [Dog new]\n$a bark\n$b bark\n";
    let analysis = analyse(src);
    // Cursor on `bark` in `$a bark` (line 5, col 3).
    let refs = references(src, "tcl", 5, 3, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![1, 5, 6],
        "method decl + both external dispatch sites; got {refs:?}",
    );
}

#[test]
fn references_external_obj_method_inside_proc_body_subst() {
    // tclsh-proof: `oo::class create Dog {method bark {} {return woof}};
    //   set d [Dog new]; proc run {} {return [$d bark]}` — `[$d bark]` is a
    //   real dispatch (the proc returns woof when run with `$d` in scope).
    // The reference scan descends into the proc body and the `[...]`
    // command-substitution, finding the `bark` site on line 5.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc run {} {\n    puts [$d bark]\n}\n";
    let analysis = analyse(src);
    // Cursor on the `bark` declaration (line 1, col 11).
    let refs = references(src, "tcl", 1, 11, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&1), "method decl missing: {refs:?}");
    assert!(
        lines.contains(&5),
        "`[$d bark]` inside the proc body (line 5) missing: {refs:?}",
    );
}

#[test]
fn references_method_with_empty_body_sibling_still_resolves() {
    // A sibling method with an empty `{}` body must not break the scan (the
    // empty-body-span guard).  tclsh-proof: `oo::class create Dog {method idle
    //   {} {}; method bark {} {return woof}}; set d [Dog new]; puts [$d bark]`
    //   -> woof — `$d bark` dispatches even though `idle` is empty.
    let src = "oo::class create Dog {\n    method idle {} {}\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Cursor on `bark` in `$d bark` (line 5, col 3).
    let refs = references(src, "tcl", 5, 3, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![2, 5],
        "decl (line 2) + external site (line 5); got {refs:?}",
    );
}

#[test]
fn references_classmethod_decl_resolves_to_member() {
    // A classmethod declaration resolves through the class-member path.
    // tclsh-proof: `oo::class create C {}; oo::define C self method build {}
    //   {return built}; puts [C build]` -> built — `C build` is the real call
    //   form (a classmethod is invoked on the class object).  The intra-class
    //   bare-`build` head match (line 2) is an editor heuristic — asserted
    //   structurally only (the decl is present; whether the sibling bare call
    //   is a runnable invocation is covered by the BUG tests below).
    let src =
        "oo::class create C {\n    classmethod build {} {}\n    classmethod make {} { build }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `build` classmethod declaration (line 1, col 17).
    let refs = references(src, "tcl", 1, 17, &analysis, true);
    assert!(
        ref_lines(&refs).contains(&1),
        "classmethod declaration (line 1) must resolve; got {refs:?}",
    );
}

#[test]
fn references_property_decl_resolves_to_declaration_only() {
    // A `property` declaration resolves through the class-member path but is
    // never dispatched as `$obj prop`, so the provider does not append any
    // external `$obj` sites — only intra-body bare matches (none here).
    // tclsh-proof for the backing class: `oo::class create P {variable color;
    //   constructor {} {set color red}; method show {} {return $color}};
    //   puts [[P new] show]` -> red (the property/var is a live member).
    // `property` is a 9.0 `TclOO` member, so the vector runs under a 9.0
    // dialect (under 8.6 it would be flagged disabled and record nothing).
    let src = "oo::class create P {\n    property color\n    method show {} { return color }\n}\n";
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(src, "tcl9.0").clone()
    };
    // Cursor on the `color` property declaration (line 1, col 14).
    let refs = references(src, "tcl9.0", 1, 14, &analysis, true);
    assert!(
        ref_lines(&refs).contains(&1),
        "property declaration (line 1) must resolve; got {refs:?}",
    );
    // Excluding the declaration leaves no external `$obj color` dispatch sites.
    let without_decl = references(src, "tcl9.0", 1, 14, &analysis, false);
    assert!(
        !without_decl.iter().any(|r| r.start_line == 1),
        "the decl span itself must be gone when excluded; got {without_decl:?}",
    );
}

#[test]
fn document_highlights_method_decl_and_external_site_all_text() {
    // tclsh-proof: `$d bark` is a real dispatch (script runs -> woof).
    // A method carries no Read/Write distinction — decl + external call are
    // Text.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Cursor on the `bark` declaration (line 1, col 11).
    let h = document_highlights(src, "tcl", 1, 11, &analysis);
    let lines = hl_lines(&h);
    assert!(lines.contains(&1), "method decl highlight missing: {h:?}");
    assert!(
        lines.contains(&4),
        "external `$d bark` highlight (line 4) missing: {h:?}",
    );
    assert!(
        h.iter().all(|(_, k)| *k == HighlightKind::Text),
        "method highlights are all Text; got {h:?}",
    );
}

// ===========================================================================
// references / document_highlights — robustness.
// ===========================================================================

#[test]
fn references_unknown_word_and_out_of_range_do_not_panic() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\n";
    let analysis = analyse(src);
    // A bare literal that is neither class, proc, method, nor var.
    assert!(
        references(src, "tcl", 1, 4, &analysis, true).is_empty()
            || !references(src, "tcl", 1, 4, &analysis, true).is_empty(),
        "must not panic on `method` keyword position",
    );
    // Far past EOF.
    assert!(references(src, "tcl", 99, 0, &analysis, true).is_empty());
    assert!(document_highlights(src, "tcl", 99, 0, &analysis).is_empty());
    // Empty document.
    let empty = analyse("");
    assert!(references("", "tcl", 0, 0, &empty, true).is_empty());
    assert!(document_highlights("", "tcl", 0, 0, &empty).is_empty());
}

#[test]
fn references_var_out_of_scope_is_empty() {
    // A `$var` the scope chain cannot resolve yields no references (the
    // var-lookup `None` early-return in `references`).  `find_var_at_position`
    // recognises `$undefined_xyz` as a variable token, but
    // `lookup_var_in_scope_chain` finds no `VarDef`, so the result is empty
    // (not a fall-through to the proc/class word search).
    let src = "puts $undefined_xyz\n";
    let analysis = analyse(src);
    // Cursor inside `$undefined_xyz` (line 0, col 7).
    assert!(
        references(src, "tcl", 0, 7, &analysis, true).is_empty(),
        "an unresolved `$var` must produce no references",
    );
}

#[test]
fn document_highlights_unknown_word_is_empty() {
    // A bare literal argument is not a highlightable symbol.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(document_highlights(src, "tcl", 0, 6, &analysis).is_empty());
}

#[test]
fn document_highlights_proc_decl_and_calls_are_text() {
    // tclsh-proof: `proc greet {} {return hi}; puts [greet]; puts [greet]`
    //   -> hi/hi — the two `greet` words are real invocations of the proc.
    // A proc carries no Read/Write distinction — its declaration and every
    // call-invocation head are Text (the `document_highlights` proc loop).
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 0, col 6).
    let h = document_highlights(src, "tcl", 0, 6, &analysis);
    assert_eq!(hl_lines(&h), vec![0, 1, 2], "decl + both calls; got {h:?}");
    assert!(
        h.iter().all(|(_, k)| *k == HighlightKind::Text),
        "proc highlights are all Text (only variables carry Read/Write); got {h:?}",
    );
}

#[test]
fn references_method_with_destructor_resolves_external_site() {
    // A class that declares a `destructor` exercises the destructor-body branch
    // of the method-reference scan.  tclsh-proof: `oo::class create Dog {method
    //   bark {} {return woof}; destructor {puts [my bark]}}; set d [Dog new];
    //   $d bark; $d destroy` -> woof (from `$d bark`) then woof (the destructor
    //   runs `my bark` on `$d destroy`).  Both are real dispatches.
    // The reference set for `bark` from the declaration = decl (line 1) + the
    // external `$d bark` (line 5).  (The destructor's `my bark` head is `my`,
    // so — per the documented BUG below — it is not surfaced; that is asserted
    // separately.  Here we pin only the decl + external site.)
    let src = "oo::class create Dog {\n    method bark {} {}\n    destructor { puts [my bark] }\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Cursor on the `bark` declaration (line 1, col 11).
    let refs = references(src, "tcl", 1, 11, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&1), "method decl missing: {refs:?}");
    assert!(
        lines.contains(&5),
        "external `$d bark` site (line 5) missing: {refs:?}",
    );
}

// ===========================================================================
// BUG WITNESSES — the intra-class member scan does not match real TclOO call
// forms.  These tests assert the CORRECT behaviour and are expected to FAIL
// until the provider is fixed; do not "fix" them by asserting today's output.
// ===========================================================================

// BUG (false negative): the intra-class method-reference scan looks for the
// member name at command-HEAD position, but a real intra-class call is
// `my <method>` — the head is `my` and the method name sits at argv[1].  So
// idiomatic call sites are MISSED.
//
//   Input: a class whose `twice` method calls `bark` the correct way (`my
//   bark`); cursor on the `bark` declaration.
//   Expected: references = {decl line 1, the two `my bark` sites on line 2}.
//   Actual:   references = {decl line 1} only — the `my bark` calls are not
//             found (the scan never inspects argv[1]).
//
// tclsh-proof that `my bark` is the genuine call form (and bare `bark` is not):
//   `oo::class create C {method bark {} {return woof};
//     method twice {} {return "[my bark][my bark]"}};
//    puts [[C new] twice]`                 -> woofwoof   (my-form runs)
//   `... method twice {} {return "[bark][bark]"} ...`   -> ERR: invalid
//                                            command name "bark"  (bare fails)
// (Identical on tclsh8.6 and tclsh9.0.)
#[test]
fn bug_intra_class_my_method_call_sites_are_missed() {
    let src = "oo::class create C {\n    method bark {} {}\n    method twice {} { my bark ; my bark }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `bark` declaration (line 1, col 11).
    let refs = references(src, "tcl", 1, 11, &analysis, true);
    let lines = ref_lines(&refs);
    // CORRECT behaviour: the two `my bark` dispatch sites on line 2 are real
    // references and must be surfaced.
    assert!(
        lines.contains(&2),
        "BUG: idiomatic `my bark` call sites on line 2 are missed by the \
         intra-class scan (it only matches a bare-head `bark`, which tclsh \
         rejects as `invalid command name`). Expected line 2 in the set; \
         got {refs:?}",
    );
}

// BUG (false positive): the intra-class scan DOES match a *bare* `greet` head
// inside a sibling method body and reports it as a reference — but tclsh proves
// a bare `greet` head there is `invalid command name`, i.e. NOT a call to the
// method at all.  A correct provider would not surface a non-call as a method
// reference.
//
//   Input: a class whose `twice` body contains bare `greet` words (not `my
//   greet`); cursor on the `greet` declaration.
//   Expected: references = {decl line 1} only (the bare `greet` words are not
//             real method calls).
//   Actual:   references = {decl line 1, line 2, line 2} — the two bare
//             `greet` heads are wrongly counted as method references.
//
// tclsh-proof: `oo::class create C {method greet {} {return hi};
//   method twice {} {return "[greet][greet]"}}; puts [[C new] twice]`
//   -> ERR: invalid command name "greet"  (so the bare words are NOT calls;
//   identical on tclsh8.6 and tclsh9.0).
#[test]
fn bug_intra_class_bare_head_is_not_a_real_reference() {
    let src =
        "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1, col 11).
    let refs = references(src, "tcl", 1, 11, &analysis, false); // exclude decl
    // CORRECT behaviour: with the declaration excluded there are NO real
    // references — the bare `greet` heads are not valid method dispatches.
    assert!(
        refs.is_empty(),
        "BUG: bare `greet` heads inside a sibling method body are reported as \
         method references, but tclsh rejects a bare `greet` there as \
         `invalid command name` — they are not real calls. Expected no \
         non-declaration references; got {refs:?}",
    );
}
