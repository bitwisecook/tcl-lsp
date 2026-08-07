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

//! `TclOO` method-resolution-order parity: go-to-definition, hover, and
//! find-references must give the **same** answer at the **same** cursor
//! (issue #923 idx 28 / 34 / 35 / 37).
//!
//! Each test drives all three providers at one position and asserts them
//! together, so the three cannot drift apart again: the audit's finding was
//! precisely that they had — definition walked the full linearisation
//! (superclasses *and* mixins), while hover and references looked the
//! method up only on the receiver's own class and abstained on a miss.
//!
//! C-Tcl proof model: each snippet is a complete, runnable script whose
//! dispatch was pinned against tclsh 9.0.4.
//!
//! ```text
//! % oo::class create DuplChecker { method duplListCheck {lst} { return ok } }
//! % oo::class create Optimization { mixin DuplChecker
//!     method addPars {} { return [my duplListCheck {}] } }
//! % oo::class create Mpfit { superclass Optimization }
//! % info class mixins Optimization        -> ::DuplChecker
//! % [Mpfit new] addPars                   -> ok
//! ```
//!
//! and for the constructor chain (issue #923 idx 37):
//!
//! ```text
//! % oo::class create Base { constructor {n} { puts "base $n" } }
//! % oo::class create Derived { superclass Base
//!     constructor {n} { next $n } }
//! % Derived new 1                         -> base 1
//! ```

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::definition::{LspRange, definition};
use tcl_lsp_core::hover::hover;
use tcl_lsp_core::references::references;
use tcl_registry::CommandRegistry;

const D: &str = "tcl9.0";

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, D).clone()
}

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

fn start_lines(ranges: &[LspRange]) -> Vec<u32> {
    let mut v: Vec<u32> = ranges.iter().map(|r| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The 0-based `(line, character)` of the first occurrence of `needle`,
/// offset into the middle of the word so the cursor sits on it.
fn cursor_at(source: &str, needle: &str) -> (u32, u32) {
    for (idx, text) in source.split('\n').enumerate() {
        if let Some(col) = text.find(needle) {
            let line = u32::try_from(idx).expect("line fits u32");
            let col = u32::try_from(col + needle.len() / 2).expect("col fits u32");
            return (line, col);
        }
    }
    panic!("{needle:?} not found in source");
}

/// [`cursor_at`] for the `n`-th (0-based) line that contains `needle`.
fn cursor_at_line(source: &str, line_needle: &str, needle: &str) -> (u32, u32) {
    for (idx, text) in source.split('\n').enumerate() {
        if text.contains(line_needle)
            && let Some(col) = text.find(needle)
        {
            let line = u32::try_from(idx).expect("line fits u32");
            let col = u32::try_from(col + needle.len() / 2).expect("col fits u32");
            return (line, col);
        }
    }
    panic!("{needle:?} not found on a line containing {line_needle:?}");
}

/// Drive definition, hover, and find-references at one cursor.
struct Trio {
    definition: Vec<LspRange>,
    hover: Option<String>,
    references: Vec<LspRange>,
}

fn trio(source: &str, needle: &str) -> Trio {
    trio_at(source, cursor_at(source, needle))
}

fn trio_at(source: &str, (line, character): (u32, u32)) -> Trio {
    let analysis = analyse(source);
    let registry = reg();
    Trio {
        definition: definition(source, line, character, &analysis),
        hover: hover(source, line, character, &analysis, Some(&registry)).map(|h| h.value.clone()),
        references: references(source, D, line, character, &analysis, true),
    }
}

/// `my m` where `m` comes from a `mixin` two MRO hops away — the exact
/// shape of `tclopt.tcl`'s `Optimization::addPars` calling
/// `my duplListCheck` (issue #923 idx 34 / 35, and the second half of idx
/// 28, whose `ArgsPreprocess` is the same mixin-then-superclass chain).
const MIXIN_CHAIN: &str = "\
oo::class create DuplChecker {
    method duplListCheck {lst} { return ok }
}
oo::class create Optimization {
    mixin DuplChecker
    method addPars {} { return [my duplListCheck {}] }
}
oo::class create Mpfit {
    superclass Optimization
}
";

#[test]
fn my_call_to_a_mixin_provided_method_resolves_in_all_three_providers() {
    let t = trio(MIXIN_CHAIN, "duplListCheck {}");
    // The declaration is on line 1 (`method duplListCheck {lst} …`).
    assert_eq!(
        start_lines(&t.definition),
        vec![1],
        "definition already walked the MRO and must keep doing so",
    );
    let hover_text = t
        .hover
        .expect("hover must resolve the same mixin-provided method definition reaches");
    assert!(
        hover_text.contains("duplListCheck"),
        "hover names the resolved method: {hover_text}",
    );
    assert!(
        hover_text.contains("DuplChecker"),
        "hover names the providing class, not the receiver: {hover_text}",
    );
    // References must find both the declaration (line 1) and the `my` call
    // site (line 5).
    assert_eq!(
        start_lines(&t.references),
        vec![1, 5],
        "references must reach the mixin's declaration and the call site",
    );
}

#[test]
fn declaration_side_query_agrees_with_the_call_site_query() {
    // The audit's asymmetry: querying from the declaration already found
    // the call site, while querying from the call site found nothing.
    let from_decl = trio(MIXIN_CHAIN, "duplListCheck {lst}");
    let from_call = trio(MIXIN_CHAIN, "duplListCheck {}");
    assert_eq!(
        start_lines(&from_decl.references),
        start_lines(&from_call.references),
        "find-references must be direction-independent",
    );
}

/// A superclass-provided (not mixed-in) method: the same parity must hold
/// on the plain inheritance axis.
const SUPERCLASS_CHAIN: &str = "\
oo::class create Base {
    method describe {} { return base }
}
oo::class create Derived {
    superclass Base
    method report {} { return [my describe] }
}
";

#[test]
fn my_call_to_an_inherited_method_resolves_in_all_three_providers() {
    let t = trio(SUPERCLASS_CHAIN, "describe]");
    assert_eq!(start_lines(&t.definition), vec![1]);
    let hover_text = t.hover.expect("hover resolves the inherited method");
    assert!(hover_text.contains("describe"), "{hover_text}");
    assert!(
        hover_text.contains("Base"),
        "the provider is the superclass: {hover_text}",
    );
    assert_eq!(start_lines(&t.references), vec![1, 5]);
}

#[test]
fn a_locally_declared_method_is_unchanged_by_the_shared_walk() {
    // FP guard: the direct-hit case must render and resolve exactly as
    // before — the receiver's own class provides it, so no "inherited
    // from" note and no provider substitution.
    let src = "\
oo::class create Solo {
    method work {} { return 1 }
    method run {} { return [my work] }
}
";
    let t = trio(src, "work]");
    assert_eq!(start_lines(&t.definition), vec![1]);
    let hover_text = t.hover.expect("hover resolves a locally-declared method");
    assert!(hover_text.contains("Solo::work"), "{hover_text}");
    assert!(
        !hover_text.contains("inherited from"),
        "not inherited: {hover_text}",
    );
    assert_eq!(start_lines(&t.references), vec![1, 2]);
}

#[test]
fn an_undefined_method_stays_unresolved_in_all_three_providers() {
    // TN guard: `my` dispatch to a method no class on the MRO provides is
    // a runtime `unknown method` in Tcl, so all three must abstain rather
    // than guess at a same-named proc.
    let src = "\
proc noSuchMethod {} { return trap }
oo::class create Solo {
    method run {} { return [my noSuchMethod] }
}
";
    let t = trio(src, "noSuchMethod]");
    assert!(
        t.definition.is_empty(),
        "must not fall through to the same-named proc: {:?}",
        t.definition,
    );
    assert!(
        t.hover.is_none_or(|h| !h.contains("**method**")),
        "no method hover for an unknown method",
    );
}

/// `next` inside a `constructor` — the shape `::tclopt::ParameterMpfit`
/// uses to forward its options to `::tclopt::Parameter` (issue #923 idx
/// 37). Pinned: `Derived new 1` prints `base 1` under tclsh 9.0.4.
const CONSTRUCTOR_CHAIN: &str = "\
oo::class create Base {
    constructor {n} { set x $n }
}
oo::class create Derived {
    superclass Base
    constructor {n} { next $n }
}
";

#[test]
fn next_inside_a_constructor_resolves_in_all_three_providers() {
    let t = trio(CONSTRUCTOR_CHAIN, "next $n");
    assert_eq!(
        start_lines(&t.definition),
        vec![1],
        "`next` in a constructor chains to the superclass constructor",
    );
    // The identical shape via a plain method already worked; assert it
    // still does, so the constructor fix did not come at its expense.
    let via_method = trio(SUPERCLASS_CHAIN, "describe]");
    assert_eq!(start_lines(&via_method.definition), vec![1]);
}

#[test]
fn next_inside_a_destructor_resolves_to_the_superclass_destructor() {
    let src = "\
oo::class create Base {
    destructor { set x 1 }
}
oo::class create Derived {
    superclass Base
    destructor { next }
}
";
    let t = trio(src, "next");
    assert_eq!(start_lines(&t.definition), vec![1]);
}

#[test]
fn next_in_a_constructor_with_no_superclass_constructor_abstains() {
    // TN — `TclOO` falls back to `oo::object`'s permissive default
    // constructor, which has no user-visible declaration to jump to.
    let src = "\
oo::class create Base {
}
oo::class create Derived {
    superclass Base
    constructor {n} { next $n }
}
";
    let t = trio(src, "next $n");
    assert!(t.definition.is_empty(), "{:?}", t.definition);
}

/// `self mixin -append X` — `TclOO`'s `self` wrapper forwards to
/// `oo::objdefine <thisclass> mixin -append X`, so `X` is genuinely
/// referenced there (issue #923 idx 15). Pinned against tclsh 9.0.4:
///
/// ```text
/// % oo::class create Marker { method marker {} { return m } }
/// % oo::class create ViaSelf  { self mixin -append Marker }
/// % info object mixins ViaSelf        -> ::Marker
/// ```
#[test]
fn self_wrapped_mixin_is_a_class_reference_in_all_three_providers() {
    let src = "\
oo::class create Marker {
    method marker {} { return m }
}
oo::class create ViaSelf {
    self mixin -append Marker
}
oo::class create ViaPlain {
    mixin Marker
}
";
    // From the declaration: both the `self mixin` and the plain `mixin`
    // usage must appear.
    let from_decl = trio(src, "Marker {");
    assert_eq!(
        start_lines(&from_decl.references),
        vec![0, 4, 7],
        "declaration + self-mixin usage + plain-mixin usage",
    );
    // From the `self mixin` usage itself.
    let from_use = trio_at(src, cursor_at_line(src, "self mixin", "Marker"));
    assert_eq!(start_lines(&from_use.definition), vec![0]);
    assert!(
        from_use.hover.is_some_and(|h| h.contains("Marker")),
        "the self-mixin target hovers as the class",
    );
    assert_eq!(start_lines(&from_use.references), vec![0, 4, 7]);
}

/// Two `oo::configurable` classes whose `initialize {}` blocks declare a
/// **same-named** class-scoped variable (issue #923 idx 36). Pinned against
/// tclsh 9.0.4 — the two are fully independent, each setter rejecting the
/// other's values, and each `initialize` body runs in its own class-object
/// namespace (`::oo::Obj20` / `::oo::Obj22`):
///
/// ```text
/// % f configure -strategy beta   -> {}
/// % f configure -strategy red    -> Foo: 'red' not in 'alpha beta gamma'
/// % b configure -mode red        -> {}
/// % b configure -mode beta       -> Bar: 'beta' not in 'red green blue'
/// ```
const SIBLING_INITIALIZE: &str = "\
oo::configurable create Foo {
    property strategy -set { classvariable available; if {$value in $available} { set -strategy $value } }
    initialize {
        variable available
        const available {alpha beta}
    }
}
oo::configurable create Bar {
    property mode -set { classvariable available; if {$value in $available} { set -mode $value } }
    initialize {
        variable available
        const available {red green}
    }
}
";

#[test]
fn initialize_block_variables_do_not_leak_between_sibling_classes() {
    // Foo's setter reads Foo's own declaration (line 3), Bar's reads Bar's
    // (line 10) — neither the first-class-wins definition nor the merged
    // 4-location reference set the shared enclosing scope produced.
    let foo = trio_at(
        SIBLING_INITIALIZE,
        cursor_at_line(SIBLING_INITIALIZE, "strategy", "available}"),
    );
    assert_eq!(start_lines(&foo.definition), vec![3]);
    assert_eq!(start_lines(&foo.references), vec![1, 3, 4]);

    let bar = trio_at(
        SIBLING_INITIALIZE,
        cursor_at_line(SIBLING_INITIALIZE, "mode", "available}"),
    );
    assert_eq!(
        start_lines(&bar.definition),
        vec![10],
        "must be Bar's own initialize declaration, not the file's first",
    );
    assert_eq!(
        start_lines(&bar.references),
        vec![8, 10, 11],
        "the two classes' declarations must not merge into one symbol",
    );
}

// ───────────── class factories and dynamically-installed members ─────────────
//
// Issue #923 idx 53 / 55 / 96 / 97 at the *LSP* surface.  The analyser-side
// mechanics have their own matrix (`tcl-compiler/tests/analyser.rs`,
// `mod class_factories`); these pin what the editor actually sees, which is
// what those findings reported.

/// Flatten an outline to `(container, member)` pairs, one per nested symbol.
fn outline_members(source: &str) -> Vec<(String, String)> {
    fn walk(
        parent: &str,
        syms: &[tcl_lsp_core::document_symbols::DocumentSymbol],
        out: &mut Vec<(String, String)>,
    ) {
        for s in syms {
            if !parent.is_empty() {
                out.push((parent.to_string(), s.name.clone()));
            }
            walk(&s.name, &s.children, out);
        }
    }
    let mut out = Vec::new();
    walk(
        "",
        &tcl_lsp_core::document_symbols::document_symbols(source, D),
        &mut out,
    );
    out
}

/// Diagnostic codes the analyser emits for `source`.
fn diag_codes(source: &str) -> Vec<String> {
    analyse(source)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

/// idx 55 (TP): `foreach class {A B} { oo::define $class { method m … } }` —
/// the ticklecharts `etsb.tcl` monkey-patch.  The target names come from a
/// **literal** list, so each real class genuinely gains the method (tclsh
/// 9.0.4 / 8.6.16: `[mychart::chart new] RenderTsb` answers
/// `TSB:chart-json`).  The outline must hang it off the real classes — not a
/// synthetic `@dynclass@…` key — and navigation must reach its declaration.
#[test]
fn idx55_a_literal_foreach_oo_define_installer_reaches_the_real_classes() {
    let src = "\
namespace eval mychart {}
oo::class create mychart::chart {
    method toJSON {} { return chart-json }
}
oo::class create mychart::chart3D {
    method toJSON {} { return chart3d-json }
}
foreach class {mychart::chart mychart::chart3D} {
    oo::define $class {
        method RenderTsb {args} { return \"TSB:[my toJSON]\" }
        export RenderTsb
    }
}
set obj [mychart::chart new]
$obj RenderTsb
";
    let members = outline_members(src);
    for owner in ["chart", "chart3D"] {
        assert!(
            members.contains(&(owner.to_string(), "RenderTsb".to_string())),
            "{owner} must own the injected method: {members:?}",
        );
    }
    assert!(
        members
            .iter()
            .all(|(owner, _)| !owner.contains("@dynclass@")),
        "no synthetic key may leak into the outline: {members:?}",
    );
    let t = trio_at(src, cursor_at_line(src, "$obj RenderTsb", "RenderTsb"));
    assert_eq!(start_lines(&t.definition), vec![9], "{:?}", t.definition);
    assert!(t.hover.is_some_and(|h| h.contains("RenderTsb")));
    assert!(
        !diag_codes(src).contains(&"W308".to_string()),
        "a method the class really has must not warn: {:?}",
        diag_codes(src),
    );
}

/// idx 96 (TP): a class made by a **user-defined metaclass** (Tk's
/// `::tk::Megawidget`) is a real class, and the method it inherits from the
/// superclass that metaclass splices in resolves like any other.
///
/// tclsh 9.0.4 / 8.6.14: `info class superclasses ::tk::SimpleWidget` →
/// `::tk::MegawidgetClass`, and `my TraceOption` inside `CreateHull` really
/// runs `::tk::MegawidgetClass`'s implementation.
#[test]
fn idx96_a_metaclass_made_class_resolves_its_spliced_inherited_method() {
    let src = "\
namespace eval ::tk {}
oo::class create ::tk::Megawidget {
    superclass ::oo::class
    self method create {name superclasses body} {
        next $name [list superclass ::tk::MegawidgetClass {*}$superclasses]\\;$body
    }
}
oo::class create ::tk::MegawidgetClass {
    method TraceOption {option method args} { return traced }
}
::tk::Megawidget create ::tk::SimpleWidget {} {
    method CreateHull {} { my TraceOption -cursor UpdateCursorOption }
}
::tk::Megawidget create ::tk::FocusableWidget ::tk::SimpleWidget {
    method CreateHull {} { my TraceOption -takefocus UpdateTakefocus }
}
";
    let members = outline_members(src);
    for owner in ["SimpleWidget", "FocusableWidget"] {
        assert!(
            members.contains(&(owner.to_string(), "CreateHull".to_string())),
            "{owner} must be a class with its own members: {members:?}",
        );
    }
    let t = trio_at(src, cursor_at_line(src, "-cursor", "TraceOption"));
    assert_eq!(start_lines(&t.definition), vec![8], "{:?}", t.definition);
    assert!(
        t.hover
            .is_some_and(|h| h.contains("::tk::MegawidgetClass::TraceOption")),
        "hover must name the providing class",
    );
    // Both metaclass-made classes' call sites come back from the provider's
    // own declaration.
    assert_eq!(
        start_lines(&t.references),
        vec![8, 11, 14],
        "{:?}",
        t.references
    );
}

/// idx 97 (TP): `next` inside a metaclass-made class walks the same
/// linearisation the literal spelling would.
///
/// tclsh 9.0.4 / 8.6.14: `[IconList new] probe` → `GetSpecs
/// iconlist+focusable+simple+base`, i.e. `IconList::GetSpecs`'s `next`
/// reaches `FocusableWidget::GetSpecs`, skipping neither more nor less.
#[test]
fn idx97_next_resolves_through_a_metaclass_made_chain() {
    let src = "\
oo::class create Megawidget {
    superclass oo::class
    self method create {name superclasses body} {
        next $name [list superclass MegawidgetClass {*}$superclasses]\\;$body
    }
}
oo::class create MegawidgetClass {
    method GetSpecs {} { return base }
}
Megawidget create SimpleWidget {} {
    method GetSpecs {} { return \"simple+[next]\" }
}
Megawidget create FocusableWidget SimpleWidget {
    method GetSpecs {} { return \"focusable+[next]\" }
}
Megawidget create IconList FocusableWidget {
    method GetSpecs {} { return \"iconlist+[next]\" }
}
";
    let t = trio_at(src, cursor_at_line(src, "iconlist+", "next"));
    assert_eq!(
        start_lines(&t.definition),
        vec![13],
        "`next` must reach FocusableWidget::GetSpecs: {:?}",
        t.definition
    );
}

/// idx 53 (abstention) / issue #1277 (partial recovery): a class whose
/// members are installed by *reflection* keeps its member tables as a lower
/// bound, so W308 answers nothing rather than answering wrongly.
///
/// tclsh 9.0.4 / 8.6.16 both prove the members are real (`info class methods
/// ::C3` lists `options`; `[C3 new] options` runs). Since issue #1277, the
/// installer's `method $m …` names itself with exactly the loop variable
/// bound to a **literal** list (`foreach m {options} { … }`), so the walk
/// can honestly say where `options`'s name comes from — the list element
/// itself — even though it still knows nothing about the method's
/// parameter list (`{*}[info class definition ::Donor $m]` is itself
/// computed, so the signature stays `params_computed`). Definition/hover
/// now resolve to that honest source location instead of finding nothing;
/// what must still never happen is the W308 that used to fire here, or an
/// invented arity diagnostic on a call of any shape.
#[test]
fn idx53_reflectively_installed_members_abstain_instead_of_warning() {
    let src = "\
oo::class create Donor {
    method options {} { return 1 }
}
oo::class create C3 {
    constructor {*}[info class constructor ::Donor]
    foreach m {options} { method $m {*}[info class definition ::Donor $m] }
    method getType {} { return c3 }
}
set c3 [C3 new]
$c3 options
";
    assert!(
        !diag_codes(src).contains(&"W308".to_string()),
        "a reflectively-installed method is not a missing one: {:?}",
        diag_codes(src),
    );
    // The readable half of the class is still fully modelled.
    assert!(
        outline_members(src).contains(&("C3".to_string(), "getType".to_string())),
        "the abstention must not erase what *was* readable",
    );
    // The *name* is now honestly known (issue #1277) — go-to-definition
    // lands on the loop's own literal list, line 5 (`foreach m {options}
    // …`), not on a wrong or invented target.
    let t = trio_at(src, cursor_at_line(src, "$c3 options", "options"));
    assert_eq!(
        start_lines(&t.definition),
        vec![5],
        "the name's one real source location, not an invented one: {:?}",
        t.definition
    );
    // …but the *signature* is still unknown: a call of any arity must not
    // draw E002/E003.
    assert!(
        !diag_codes(src).iter().any(|c| c == "E002" || c == "E003"),
        "an unmodelled signature must never be checked: {:?}",
        diag_codes(src),
    );
}

/// idx 53 (FN guard): the abstention is scoped to the class that earns it.
/// A sibling class in the same file whose body is fully readable keeps every
/// method check.
#[test]
fn idx53_a_readable_sibling_class_keeps_its_method_checks() {
    let src = "\
oo::class create Donor {
    method options {} { return 1 }
}
oo::class create C3 {
    constructor {*}[info class constructor ::Donor]
}
set d [Donor new]
$d nosuch
";
    assert!(
        diag_codes(src).contains(&"W308".to_string()),
        "the readable class must still warn: {:?}",
        diag_codes(src),
    );
}
