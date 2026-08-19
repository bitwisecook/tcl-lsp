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

//! Behaviour-driven coverage of the five `tcl-lsp-core` navigation providers:
//! `definition::definition`, `declaration::declaration`,
//! `type_definition::type_definition`, `implementation::implementation`, and
//! `type_hierarchy::prepare`.
//!
//! These tests are derived from each provider's public API (see
//! `src/{definition,declaration,type_definition,implementation,
//! type_hierarchy}.rs`) and from Tcl's own name- and class-resolution
//! semantics, with every Tcl-semantic fact pinned to real C-Tcl
//! (tclsh8.6 / tclsh9.0 via `scripts/dev/tclsh_check.sh`).
//!
//! C-Tcl proof model
//! -----------------
//! Go-to-X is correct exactly when it lands on the site that Tcl's *own*
//! name/class resolution binds the cursor word to. So every snippet below is a
//! COMPLETE runnable Tcl script in which the proc/var/class/method is genuinely
//! declared at the asserted site and is genuinely referenced at the cursor:
//!
//!   * the *fact that the target exists at that site* is proved structurally
//!     against C-Tcl — `info procs`, `info commands`, and the `TclOO`
//!     introspectors `info class superclasses` / `info class subclasses` /
//!     `info class methods` / `info class mixins` (all 8.6+).
//!   * the *editor location* (which line/column the range covers) is a parse-
//!     structure fact and is asserted structurally; the proof that the location
//!     is *right* is that it is exactly the declaration site Tcl binds the name
//!     to (e.g. `info procs greet` returns `greet`, so the one `proc greet`
//!     line is the resolution target; `info class superclasses Derived` returns
//!     `::Base`, so `Base`'s declaration is `Derived`'s super target).
//!
//! Verified C-Tcl facts (tclsh8.6 + tclsh9.0, identical output unless noted):
//!   * `proc greet {} {return hi}; info procs greet`            -> greet
//!   * `set greeting hi; puts $greeting`                        -> hi
//!   * namespace: `namespace eval ::myns {proc greet {} {…}}`;
//!     `info procs ::myns::greet`                               -> `::myns::greet`
//!   * `set counter 0; proc bump {} {global counter; incr counter}`;
//!     bump; bump; `puts $counter`                              -> 2
//!   * `namespace eval ns {variable config 1; proc get {} {variable config;
//!     return $config}}; ns::get`                               -> 1
//!   * `proc wrap {n} {upvar 1 $n local; return $local}; set outer 42;
//!     wrap outer`                                              -> 42
//!   * `TclOO` `oo::class create Greeter {method greet {} {…}}`;
//!     `info class methods Greeter`                             -> greet
//!   * `oo::class create Base {}; oo::class create Derived {superclass Base}`;
//!     `info class superclasses Derived`                        -> `::Base`
//!     `info class subclasses Base`                             -> `::Derived`
//!   * 3-level `A<-B<-C`: `info class subclasses A`             -> `::B`  (direct
//!     subclasses only — C is NOT listed)
//!   * override: `Base{method greet…hi}`, `Sub{superclass Base; method
//!     greet…yo}`; `[Base new] greet`/`[Sub new] greet`         -> hi / yo
//!   * mixin: `oo::class create M {method m…}; oo::class create User {mixin M}`;
//!     `info class mixins User`                                 -> `::M`
//!   * instance dispatch: `oo::class create Dog {method bark…woof}`;
//!     `set d [Dog new]; $d bark`                               -> woof
//!
//! `TclOO` note: `oo::class` / `oo::define` and the introspectors used here are
//! Tcl 8.6+; `TclOO` is ABSENT in 8.4 and 8.5, so the class/hierarchy tests model
//! 8.6+ behaviour only. One form differs across the supported range:
//! `classmethod NAME …` (the keyword the Rust analyser keys `class_methods`
//! off) is a real runnable command only on tclsh9.0 — on tclsh8.6 the runnable
//! spelling is `self method NAME …` (verified: `classmethod` errors with
//! `invalid command name "classmethod"` on 8.6, runs on 9.0; `self method`
//! runs on both). This is called out at the one classmethod test.
//!
//! Harness: identical to `call_hierarchy.rs` / `references_rename.rs`
//! — `Analyser::new().analyse(src, "tcl8.6")` then provider invocation.
//! Locations are `definition::LspRange` (`start_line` / `start_character` in
//! 0-based UTF-16 units). `declaration` additionally needs a `CommandRegistry`
//! (`build_default`) and a dialect string.

// Test column math indexes tiny in-memory sources; a `find`/`len` result
// always fits u32, so the pedantic truncation the lint warns of can't occur.
#![allow(clippy::cast_possible_truncation)]

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::declaration::declaration;
use tcl_lsp_core::definition::{LspRange, definition};
use tcl_lsp_core::implementation::implementation;
use tcl_lsp_core::type_definition::type_definition;
use tcl_lsp_core::type_hierarchy::{TypeHierarchyItem, prepare as prepare_type_hierarchy};
use tcl_registry::CommandRegistry;

/// Build an analysis exactly the way the existing port files do.
fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// Sorted, de-duplicated set of start lines a navigation result covers — the
/// load-bearing structural fact for most assertions.
fn start_lines(ranges: &[LspRange]) -> Vec<u32> {
    let mut v: Vec<u32> = ranges.iter().map(|r| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

// ===========================================================================
// definition — proc calls resolve to the `proc` definition
// ===========================================================================

#[test]
fn definition_proc_call_jumps_to_proc_definition() {
    // tclsh: `proc greet {} {return hi}; info procs greet` -> greet. The bare
    // `greet` on line 1 is a genuine invocation of the proc declared on line 0,
    // so go-to-definition from the call must land on the `proc greet` name span
    // (line 0, col 5 — the name follows `proc `).
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the `greet` CALL (line 1).
    let locs = definition(src, 1, 2, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "should jump to `proc greet`: {locs:?}"
    );
    assert_eq!(locs[0].start_character, 5, "name follows `proc `: {locs:?}");
}

#[test]
fn definition_instance_method_prefers_per_object_override() {
    // tclsh: `oo::class create Foo {method greet {} {return class}}; set o [Foo
    // new]; oo::objdefine $o {method greet {} {return obj}}; $o greet` -> obj.
    // TclOO layers a per-object method ahead of the class method, so
    // go-to-definition on `$o greet` must land on the per-object override (the
    // `oo::objdefine` body, line 5), not the same-named class method (line 1).
    let src = "oo::class create Foo {\n    \
                   method greet {} { return class }\n\
               }\n\
               set o [Foo new]\n\
               oo::objdefine $o {\n    \
                   method greet {} { return obj }\n\
               }\n\
               $o greet\n";
    let analysis = analyse(src);
    let locs = definition(src, 7, 4, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 5,
        "`$o greet` must resolve to the per-object override (line 5), not the class method (line 1): {locs:?}",
    );
}

#[test]
fn definition_external_call_to_unexported_method_yields_nothing_945() {
    // tclsh 9.0.4: `oo::class create Vault {method _secret {} {…}}; set v
    // [Vault new]; $v _secret` → `unknown method "_secret": must be
    // destroy` — a default-unexported method (name not matching the C
    // rule `[a-z]*`) is NOT externally callable, so go-to-definition on
    // the external call resolves to nothing rather than the declaration
    // (issue #945 fault 4).
    let src = "oo::class create Vault {\n    \
                   method _secret {} { return hidden }\n\
               }\n\
               set v [Vault new]\n\
               $v _secret\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 5, &analysis);
    assert!(
        locs.is_empty(),
        "an externally unexported TclOO method is not callable: {locs:?}",
    );
}

#[test]
fn definition_my_call_reaches_unexported_method_945() {
    // tclsh 9.0.4: `my _secret` from inside another method of the class
    // dispatches fine — internal access reaches unexported methods
    // (issue #945 fault 4's access-context split).
    let src = "oo::class create Vault {\n    \
                   method _secret {} { return hidden }\n    \
                   method probe {} { return [my _secret] }\n\
               }\n";
    let analysis = analyse(src);
    let line = src.lines().nth(2).unwrap();
    let col = line.find("_secret").unwrap() as u32 + 1;
    let locs = definition(src, 2, col, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "`my _secret` resolves the unexported declaration: {locs:?}",
    );
}

#[test]
fn definition_explicit_export_flips_external_visibility_945() {
    // `export _secret` makes the underscore method externally callable
    // (tclsh 9.0.4) — the external call now resolves.
    let src = "oo::class create Vault {\n    \
                   method _secret {} { return hidden }\n    \
                   export _secret\n\
               }\n\
               set v [Vault new]\n\
               $v _secret\n";
    let analysis = analyse(src);
    let locs = definition(src, 5, 5, &analysis);
    assert_eq!(locs.len(), 1, "explicit export admits the call: {locs:?}");
    assert_eq!(locs[0].start_line, 1, "{locs:?}");
    // And `unexport` hides a default-exported one.
    let src2 = "oo::class create Vault {\n    \
                    method leak {} { return x }\n    \
                    unexport leak\n\
                }\n\
                set v [Vault new]\n\
                $v leak\n";
    let analysis2 = analyse(src2);
    let locs2 = definition(src2, 5, 4, &analysis2);
    assert!(
        locs2.is_empty(),
        "explicit unexport blocks the external call: {locs2:?}",
    );
}

#[test]
fn definition_selects_the_dispatch_entry_not_the_override_family_945() {
    // tclsh 9.0.4: a `Dog` instance's `speak` enters `Dog::speak`
    // (`info object call` = Dog then Animal).  Go-to-definition returns
    // the dispatch entry only — never both family members (issue #945
    // fault 6).
    let src = "oo::class create Animal {\n    \
                   method speak {} { return animal }\n\
               }\n\
               oo::class create Dog {\n    \
                   superclass Animal\n    \
                   method speak {} { return dog }\n\
               }\n\
               set d [Dog new]\n\
               $d speak\n";
    let analysis = analyse(src);
    let locs = definition(src, 8, 4, &analysis);
    assert_eq!(
        locs.len(),
        1,
        "one dispatch entry, not the family: {locs:?}"
    );
    assert_eq!(
        locs[0].start_line, 5,
        "`$d speak` enters Dog::speak (line 5), never Animal::speak: {locs:?}",
    );
}

#[test]
fn definition_mixin_overrides_the_class_in_dispatch_order_945() {
    // tclsh 9.0.4 pins mixins ahead of the class itself in the call
    // chain (`info object call` = mixin, class, superclass) — the
    // dispatch entry for `$o f` is the mixin's implementation.
    let src = "oo::class create M {\n    \
                   method f {} { return M }\n\
               }\n\
               oo::class create B {\n    \
                   method f {} { return B }\n\
               }\n\
               oo::class create C {\n    \
                   superclass B\n    \
                   mixin M\n    \
                   method f {} { return C }\n\
               }\n\
               set o [C new]\n\
               $o f\n";
    let analysis = analyse(src);
    let locs = definition(src, 12, 3, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "the mixin's `f` (line 1) is the dispatch entry: {locs:?}",
    );
}

#[test]
fn per_object_methods_do_not_collide_across_scopes_945() {
    // tclsh 9.0.4: two unrelated locals both named `o` in different procs
    // are different objects — `$o m` in `b` runs b's override (`a=a b=b`).
    // The per-object lookup keys by binding identity, so b's dispatch
    // resolves b's `oo::objdefine` (line 8), never a's (line 3) — issue
    // #945 fault 5.
    let src = "oo::class create C {}\n\
               proc a {} {\n    \
                   set o [C new]\n    \
                   oo::objdefine $o {method m {} {return a}}\n    \
                   $o m\n\
               }\n\
               proc b {} {\n    \
                   set o [C new]\n    \
                   oo::objdefine $o {method m {} {return b}}\n    \
                   $o m\n\
               }\n";
    let analysis = analyse(src);
    // `$o m` inside b (line 9).
    let line = src.lines().nth(9).unwrap();
    let col = line.find('m').unwrap() as u32;
    let locs = definition(src, 9, col, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 8,
        "b's dispatch resolves b's own override (line 8), not a's (line 3): {locs:?}",
    );
    // And a's dispatch (line 4) resolves a's.
    let line_a = src.lines().nth(4).unwrap();
    let col_a = line_a.find('m').unwrap() as u32;
    let locs_a = definition(src, 4, col_a, &analysis);
    assert_eq!(locs_a.len(), 1, "{locs_a:?}");
    assert_eq!(
        locs_a[0].start_line, 3,
        "a's dispatch resolves a's own override: {locs_a:?}",
    );
}

#[test]
fn definition_bare_call_honours_namespace_path_over_global() {
    // `::helper` at global and `::mymod::helper`; `::app` declares `namespace
    // path ::mymod`.  A bare `helper` call in `::app` resolves (like C Tcl) to
    // the path target `::mymod::helper`, not the same-named global `::helper` —
    // definition must honour the path the way call-site settling does.
    let src = "proc helper {} { puts global }\nnamespace eval mymod {\n    proc helper {} { puts mymod }\n}\nnamespace eval app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `helper` call inside run's body (line 6).
    let call = src.lines().nth(6).unwrap();
    let col = call.find("helper").unwrap() as u32 + 1;
    let locs = definition(src, 6, col, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 2,
        "bare call must resolve via `namespace path` to ::mymod::helper (line 2), not global (line 0): {locs:?}"
    );
}

#[test]
fn definition_through_tailcall_and_coroutine_command_prefix() {
    // `tailcall foo` (arg 0) and `coroutine c helper` (arg 1) name a command to
    // run; declared as command prefixes, go-to-definition on that command
    // resolves to its `proc`.
    let src = "proc foo {} {}\nproc bar {} { tailcall foo }\n";
    let analysis = analyse(src);
    let line1 = src.lines().nth(1).unwrap();
    let col = line1.rfind("foo").unwrap() as u32 + 1;
    let locs = definition(src, 1, col, &analysis);
    assert_eq!(locs.len(), 1, "tailcall foo: {locs:?}");
    assert_eq!(locs[0].start_line, 0, "tailcall foo -> proc foo: {locs:?}");

    let src2 = "proc helper {} {}\ncoroutine c helper\n";
    let analysis2 = analyse(src2);
    let line1 = src2.lines().nth(1).unwrap();
    let col = line1.rfind("helper").unwrap() as u32 + 1;
    let locs2 = definition(src2, 1, col, &analysis2);
    assert_eq!(locs2.len(), 1, "coroutine helper: {locs2:?}");
    assert_eq!(
        locs2[0].start_line, 0,
        "coroutine c helper -> proc helper: {locs2:?}"
    );
}

#[test]
fn definition_from_proc_definition_resolves_to_itself() {
    // Cursor on the proc name in its own declaration resolves to that same
    // declaration (the name span is the definition site).
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    // Cursor on `greet` in the declaration (line 0, col 6).
    let locs = definition(src, 0, 6, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0);
    assert_eq!(locs[0].start_character, 5);
}

// ===========================================================================
// definition — `$var` use resolves to its `set` declaration
// ===========================================================================

#[test]
fn definition_var_use_jumps_to_set_declaration() {
    // tclsh: `set greeting hi; puts $greeting` -> hi. The `$greeting` read on
    // line 1 is a genuine use of the variable defined by `set greeting` on
    // line 0, so go-to-definition lands on the `set` site (line 0).
    let src = "set greeting hi\nputs $greeting\n";
    let analysis = analyse(src);
    // Cursor inside `$greeting` on line 1 (col 8 is within the name).
    let locs = definition(src, 1, 8, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "the `set greeting` is on line 0: {locs:?}"
    );
}

#[test]
fn definition_proc_local_var_resolves_to_proc_scope_set() {
    // tclsh: `proc f {} {set local 1; puts $local}` runs (`$local` reads the
    // proc-local). The provider walks the scope chain and resolves `$local`
    // inside `f` to the proc-local `set local 1` (line 1), not the global scope
    // (which has no `local`).
    let src = "proc f {} {\n    set local 1\n    puts $local\n}\n";
    let analysis = analyse(src);
    // Cursor inside `$local` on line 2.
    let locs = definition(src, 2, 12, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "proc-local `set local` is on line 1: {locs:?}"
    );
}

// ===========================================================================
// definition — namespace-qualified proc
// ===========================================================================

#[test]
fn definition_namespace_qualified_proc_call_resolves_to_declaration() {
    // tclsh: `namespace eval ::myns {proc greet {} {return ns-hi}}`;
    // `info procs ::myns::greet` -> ::myns::greet, and `::myns::greet` -> ns-hi.
    // The qualified call on line 3 is a genuine invocation of the proc declared
    // on line 1, so go-to-definition lands on that declaration (line 1).
    let src = "namespace eval ::myns {\n    proc greet {} { return ns-hi }\n}\n::myns::greet\n";
    let analysis = analyse(src);
    // Cursor on the qualified `::myns::greet` call (line 3, on the `greet` tail).
    let locs = definition(src, 3, 9, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "the namespaced proc decl is on line 1: {locs:?}"
    );
}

// ===========================================================================
// definition — builtins / unknown words / robustness (no panic)
// ===========================================================================

#[test]
fn definition_builtin_command_yields_no_definition() {
    // tclsh: `puts` is a built-in command, not a user proc — there is no user
    // declaration to jump to, so the provider returns empty (no panic).
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(
        definition(src, 0, 1, &analysis).is_empty(),
        "built-in `puts` has no user definition",
    );
}

#[test]
fn definition_unknown_word_yields_no_definition() {
    // A bare literal argument (`hello`) is neither proc, class, nor var.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(definition(src, 0, 6, &analysis).is_empty());
}

#[test]
fn definition_out_of_range_and_empty_file_do_not_panic() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Past EOF on the line axis.
    assert!(definition(src, 99, 0, &analysis).is_empty());
    // Past EOL on the column axis.
    assert!(definition(src, 0, 999, &analysis).is_empty());
    // Empty document.
    let empty = analyse("");
    assert!(definition("", 0, 0, &empty).is_empty());
}

// ===========================================================================
// definition — BigIP note
// ===========================================================================
//
// The plain-Tcl proc/var path above is the primary go-to-definition surface.
// A separate BigIP-config definition provider (resolving pool / data-group /
// iRule / virtual-server names against a parsed `bigip.conf`) is explicitly NOT
// implemented in `definition.rs` (see the module's "Limitations" doc-comment:
// "BigIP definition … is not implemented here"). There is therefore no
// `get_bigip_definition` entry point on this provider to exercise — the
// BigIP-only behaviour is out of this provider's surface and is not covered
// here.

// ===========================================================================
// declaration — where it differs from definition (`variable` / `global` /
// `upvar` declaration sites)
// ===========================================================================

#[test]
fn declaration_var_jumps_to_global_statement_not_the_set() {
    // tclsh: `set counter 0; proc bump {} {global counter; incr counter};
    // bump; bump; puts $counter` -> 2. Inside `bump`, the name `counter` is
    // bound by the `global counter` DECLARATION (line 1), which is what
    // go-to-DECLARATION targets — distinct from go-to-definition, which would
    // resolve the value site.
    let src = "set counter 0\nproc bump {} {\n    global counter\n    incr counter\n    puts $counter\n}\n";
    let analysis = analyse(src);
    // Cursor on `$counter` (line 4).
    let locs = declaration(src, 4, 11, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &reg());
    assert!(
        start_lines(&locs).contains(&2),
        "go-to-declaration must reach `global counter` on line 2; got {locs:?}",
    );
}

#[test]
fn declaration_var_jumps_to_variable_statement_in_namespace() {
    // tclsh: `namespace eval ns {variable config 1; proc get {} {variable
    // config; return $config}}; ns::get` -> 1. Inside `get`, `config` is
    // declared by `variable config` (line 3); go-to-declaration reaches that
    // declaration statement.
    let src = "namespace eval ns {\n    variable config 1\n    proc get {} {\n        variable config\n        return $config\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `$config` (line 4).
    let locs = declaration(src, 4, 16, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &reg());
    assert!(
        !locs.is_empty(),
        "a `variable config` declaration should be reachable; got {locs:?}",
    );
    assert!(
        start_lines(&locs).iter().any(|&l| l == 1 || l == 3),
        "declaration should be a `variable config` site (line 1 ns-level or line 3 proc-level); got {locs:?}",
    );
}

#[test]
fn declaration_var_jumps_to_upvar_alias_site() {
    // tclsh: `proc wrap {n} {upvar 1 $n local; return $local}; set outer 42;
    // wrap outer` -> 42. Inside `wrap`, `local` is introduced by the
    // `upvar 1 … local` DECLARATION (line 1) — go-to-declaration's distinct
    // target. (Static caller-name form so the upvar grammar binds the alias,
    // matching the in-module test.)
    let src = "proc wrap {} {\n    upvar 1 other local\n    return $local\n}\n";
    let analysis = analyse(src);
    // Cursor on `$local` (line 2).
    let locs = declaration(src, 2, 12, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &reg());
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "`upvar 1 other local` is on line 1: {locs:?}"
    );
}

#[test]
fn declaration_non_variable_falls_back_to_definition() {
    // tclsh: `proc greet {} {return hi}; greet` -> hi. A proc call has no
    // distinct "declaration" site, so go-to-declaration falls back to
    // go-to-definition (the `proc greet` name span on line 0) — same result the
    // definition provider gives.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the `greet` call (line 1).
    let decl = declaration(src, 1, 2, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &reg());
    let def = definition(src, 1, 2, &analysis);
    assert_eq!(decl.len(), 1, "{decl:?}");
    assert_eq!(
        decl[0].start_line, 0,
        "falls back to the proc def on line 0: {decl:?}"
    );
    assert_eq!(
        start_lines(&decl),
        start_lines(&def),
        "declaration==definition for a proc call"
    );
}

#[test]
fn declaration_out_of_range_and_empty_file_do_not_panic() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert!(declaration(src, 99, 0, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &reg()).is_empty());
    assert!(declaration(src, 0, 999, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &reg()).is_empty());
    let empty = analyse("");
    assert!(declaration("", 0, 0, tcl_dialect::DialectProfile::by_name("tcl8.6"), &empty, &reg()).is_empty());
}

// ===========================================================================
// type_definition — TclOO: the class that *types* a receiver
// ===========================================================================

#[test]
fn type_definition_instance_var_jumps_to_inferred_class() {
    // tclsh: `oo::class create Dog {method bark {} {return woof}}; set d
    // [Dog new]; $d bark` -> woof. `$d` is typed by class `Dog` (the analyser
    // infers this from `set d [Dog new]`), so go-to-type-definition on `$d`
    // jumps to `Dog`'s declaration on line 0.
    let src = "oo::class create Dog { method bark {} { return woof } }\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Cursor on `$d` in the `$d bark` call (line 2, col 1 within `$d`).
    let locs = type_definition(src, 2, 1, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "`Dog`'s declaration is on line 0: {locs:?}"
    );
}

#[test]
fn type_definition_method_word_in_class_body_jumps_to_owning_class() {
    // tclsh: `oo::class create Greeter {method greet {} {return hi}; method
    // again {} {greet}}; [Greeter new] again` -> hi. Inside the class body, the
    // `greet` call names a method of `Greeter`, so go-to-type-definition
    // resolves to the OWNING class `Greeter` (line 0) — the method's type.
    let src = "oo::class create Greeter {\n    method greet {} { return hi }\n    method again {} { greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` call inside `again`'s body (line 2).
    let locs = type_definition(src, 2, 23, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "owning class `Greeter` is on line 0: {locs:?}"
    );
}

#[test]
fn type_definition_plain_var_without_class_returns_none() {
    // tclsh: `set x 42; puts $x` -> 42 — `x` holds an integer, not an object.
    // No class types `$x`, so go-to-type-definition is empty (the empty/none
    // case).
    let src = "set x 42\nputs $x\n";
    let analysis = analyse(src);
    // Cursor on `$x` (line 1).
    assert!(
        type_definition(src, 1, 6, &analysis).is_empty(),
        "a plain integer var has no typing class",
    );
}

#[test]
fn type_definition_unrelated_word_and_robustness() {
    // A bare literal has no typing class; out-of-range must not panic.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(type_definition(src, 0, 6, &analysis).is_empty());
    assert!(type_definition(src, 99, 0, &analysis).is_empty());
    let empty = analyse("");
    assert!(type_definition("", 0, 0, &empty).is_empty());
}

// ===========================================================================
// implementation — TclOO subclass / method-override fan-out
// ===========================================================================

#[test]
fn implementation_class_name_returns_direct_subclasses() {
    // tclsh: `oo::class create Base {}; oo::class create Derived {superclass
    // Base}; oo::class create Other {}`; `info class subclasses Base` ->
    // ::Derived. Go-to-implementation on `Base` lists the classes that realise
    // it — its direct subclass `Derived` (line 1), and NOT the unrelated
    // `Other`.
    let src = "oo::class create Base {}\noo::class create Derived {\n    superclass Base\n}\noo::class create Other {}\n";
    let analysis = analyse(src);
    // Cursor on `Base` in its own declaration (line 0, col 18 within `Base`).
    let locs = implementation(src, 0, 18, &analysis);
    assert_eq!(locs.len(), 1, "exactly one direct subclass: {locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "`Derived`'s name span is on line 1: {locs:?}"
    );
}

#[test]
fn implementation_class_name_lists_only_direct_subclasses_not_grandchildren() {
    // tclsh: 3-level `A<-B<-C` (`oo::class create A {}`, `B {superclass A}`,
    // `C {superclass B}`); `info class subclasses A` -> ::B (DIRECT only — C is
    // not listed). Go-to-implementation on `A` mirrors Tcl: just the direct
    // subclass `B` (line 1), not the grandchild `C` (line 2).
    let src = "oo::class create A {}\noo::class create B {\n    superclass A\n}\noo::class create C {\n    superclass B\n}\n";
    let analysis = analyse(src);
    // Cursor on `A` in its declaration (line 0).
    let locs = implementation(src, 0, 17, &analysis);
    assert_eq!(
        start_lines(&locs),
        vec![1],
        "only direct subclass B (line 1); got {locs:?}"
    );
}

#[test]
fn implementation_class_name_includes_a_mixing_class() {
    // tclsh: `oo::class create M {method m {} {return mx}}; oo::class create
    // User {mixin M}`; `info class mixins User` -> ::M, and `[User new] m` ->
    // mx. The provider treats a `mixin` edge like a superclass edge, so
    // go-to-implementation on `M` lists `User` (line 1) as a realiser.
    let src = "oo::class create M {\n    method m {} { return mx }\n}\noo::class create User {\n    mixin M\n}\n";
    let analysis = analyse(src);
    // Cursor on `M` in its declaration (line 0, col 17 within `M`).
    let locs = implementation(src, 0, 17, &analysis);
    assert_eq!(
        start_lines(&locs),
        vec![3],
        "`User` (mixes in M) decl on line 3; got {locs:?}"
    );
}

#[test]
fn implementation_method_outside_class_returns_all_definers() {
    // tclsh: two independent classes each define a `run` method
    // (`oo::class create A {method run …}` / `B {method run …}`). Both
    // `info class methods A` and `info class methods B` -> run. From a call
    // OUTSIDE any class body, go-to-implementation lists every class that
    // defines `run` — both A's (line 1) and B's (line 4).
    let src = "oo::class create A {\n    method run {} {}\n}\noo::class create B {\n    method run {} {}\n}\nset x [a run]\n";
    let analysis = analyse(src);
    // Cursor on `run` in the top-level `[a run]` call (line 6).
    let locs = implementation(src, 6, 9, &analysis);
    assert_eq!(
        start_lines(&locs),
        vec![1, 4],
        "both A::run and B::run; got {locs:?}"
    );
}

#[test]
fn implementation_method_inside_class_returns_self_and_descendant_override() {
    // tclsh: `oo::class create Base {method greet {} {return hi}}; oo::class
    // create Sub {superclass Base; method greet {} {return yo}}`; both define
    // `greet` (`info class methods Base`/`Sub` -> greet), and
    // `[Base new] greet`/`[Sub new] greet` -> hi / yo. From inside Base's own
    // `greet`, go-to-implementation returns Base's own def (line 1) plus the
    // descendant override in Sub (line 5) — ancestors are intentionally omitted.
    let src = "oo::class create Base {\n    method greet {} { return hi }\n}\noo::class create Sub {\n    superclass Base\n    method greet {} { return yo }\n}\n";
    let analysis = analyse(src);
    // Cursor on `greet` inside Base's method definition (line 1).
    let locs = implementation(src, 1, 11, &analysis);
    assert_eq!(
        start_lines(&locs),
        vec![1, 5],
        "Base's own `greet` (line 1) + Sub's override (line 5); got {locs:?}",
    );
}

#[test]
fn implementation_unknown_word_and_empty_cases() {
    // A bare literal names neither class nor method — no implementations.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(implementation(src, 0, 6, &analysis).is_empty());
    // A lone class with no subclasses: cursor on its name yields nothing to
    // realise. tclsh: `info class subclasses Solo` -> {} (empty).
    let solo = "oo::class create Solo {}\n";
    let solo_a = analyse(solo);
    assert!(
        implementation(solo, 0, 18, &solo_a).is_empty(),
        "a class with no subclasses has no implementations",
    );
    // Robustness: out-of-range / empty file must not panic.
    assert!(implementation(src, 99, 0, &analysis).is_empty());
    let empty = analyse("");
    assert!(implementation("", 0, 0, &empty).is_empty());
}

// ===========================================================================
// implementation — classmethod (dialect-divergent runnable form)
// ===========================================================================

#[test]
fn implementation_classmethod_is_an_implementation() {
    // The Rust analyser records a `classmethod NAME …` declaration in a class's
    // `class_methods`, and go-to-implementation surfaces a class-method definer
    // just like an instance method. Cursor on a top-level `factory` call lists
    // the class that defines it (line 1).
    //
    // DIALECT NOTE: `classmethod` (the keyword this provider keys off) is a real
    // runnable command only on tclsh9.0 — `oo::class create C {classmethod
    // factory {} {return made}}; C factory` -> made on 9.0, but on 8.6 it errors
    // `invalid command name "classmethod"`. The equivalent runnable-on-8.6 form
    // is `self method factory {} {return made}` (also runs on 9.0). The
    // structural target is identical; only the spelling of the runnable proof
    // differs across the supported range. TclOO itself is 8.6+ (absent 8.4/8.5).
    let src = "oo::class create C {\n    classmethod factory {} { return made }\n}\nC factory\n";
    let analysis = analyse(src);
    // Cursor on the top-level `factory` call (line 3), outside any class body.
    let locs = implementation(src, 3, 2, &analysis);
    assert_eq!(
        start_lines(&locs),
        vec![1],
        "C's classmethod `factory` on line 1; got {locs:?}"
    );
}

// ===========================================================================
// type_hierarchy — prepare resolves the class at the cursor
// ===========================================================================

#[test]
fn type_hierarchy_prepare_resolves_class_at_cursor() {
    // tclsh: `oo::class create Greeter {method greet {} {return hi}}`;
    // `info class methods Greeter` -> greet (the class exists). Preparing a type
    // hierarchy on `Greeter`'s name yields exactly one item naming that class,
    // with its selection range anchored at the name span (line 0).
    let src = "oo::class create Greeter {\n    method greet {} { return hi }\n}\n";
    let analysis = analyse(src);
    // Cursor on `Greeter` (line 0, col 18 within the name).
    let items: Vec<TypeHierarchyItem> = prepare_type_hierarchy(src, 0, 18, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    assert!(
        items[0].name.contains("Greeter"),
        "item names the class: {items:?}"
    );
    assert_eq!(
        items[0].selection_range.start_line, 0,
        "name span on line 0: {items:?}"
    );
    // The full range starts at the name and the detail carries the metaclass.
    assert_eq!(items[0].range.start_line, 0);
}

#[test]
fn type_hierarchy_prepare_resolves_subclass_with_superclass_relation() {
    // tclsh: `oo::class create Base {}; oo::class create Derived {superclass
    // Base}`; `info class superclasses Derived` -> ::Base (the super relation is
    // real). Preparing on `Derived` resolves that class. NOTE: the provider's
    // supertype/subtype WALKS are stub-empty (the module documents
    // "Supertype / subtype walks are stub-empty"), so `prepare` returns the
    // single resolved item — the structural super/sub fact is proved via C-Tcl
    // and exercised structurally through the `implementation` provider above
    // (which DOES compute the sub relation). Here we assert only what `prepare`
    // contracts: the class at the cursor resolves to one item.
    let src = "oo::class create Base {}\noo::class create Derived {\n    superclass Base\n}\n";
    let analysis = analyse(src);
    // Cursor on `Derived` (line 1, col 18 within the name).
    let items = prepare_type_hierarchy(src, 1, 18, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    assert!(
        items[0].name.contains("Derived"),
        "resolves the subclass: {items:?}"
    );
    assert_eq!(
        items[0].selection_range.start_line, 1,
        "name span on line 1: {items:?}"
    );
}

#[test]
fn type_hierarchy_prepare_on_non_class_is_empty() {
    // A built-in / bare literal is not a class — prepare yields no item.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(
        prepare_type_hierarchy(src, 0, 1, &analysis).is_empty(),
        "built-in is not a class"
    );
    assert!(
        prepare_type_hierarchy(src, 0, 6, &analysis).is_empty(),
        "bare literal is not a class"
    );
}

#[test]
fn type_hierarchy_prepare_out_of_range_and_empty_file_do_not_panic() {
    let src = "oo::class create C {}\n";
    let analysis = analyse(src);
    assert!(prepare_type_hierarchy(src, 99, 0, &analysis).is_empty());
    assert!(prepare_type_hierarchy(src, 0, 999, &analysis).is_empty());
    let empty = analyse("");
    assert!(prepare_type_hierarchy("", 0, 0, &empty).is_empty());
}

// ===========================================================================
// M11 (TIP 278) — namespace-scope variable resolution is dialect-gated.
//
// C-Tcl facts, pinned live against tclsh8.6 (8.6.16) and tclsh9.0 (9.0.4)
// (the executable table lives in `rust/tcl-vm/tests/cross_version_vars_e2e.rs`):
//   * `set g 1; namespace eval foo { set g }`   8.6 → 1, 9.0 → error
//   * `namespace eval a { set x 1; namespace eval b { set x } }` both → error
//   * `set v 1; namespace eval bar { variable v; set v }`        both → error
//   * `set g 1; proc p {} { set g }; p`                          both → error
// ===========================================================================

/// Analyse under an explicit dialect (both sides of the TIP 278 boundary).
fn analyse_as(source: &str, dialect: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, dialect).clone()
}

#[test]
fn definition_ns_scope_bare_var_reaches_the_global_only_under_8x_m11() {
    let src = "set g 1\nnamespace eval foo {\n    puts $g\n}\n";
    let col = src.lines().nth(2).unwrap().find("$g").unwrap() as u32 + 1;
    // TP: 8.x — the namespace frame falls back to the global table.
    let a86 = analyse_as(src, "tcl8.6");
    let locs = definition(src, 2, col, &a86);
    assert_eq!(
        start_lines(&locs),
        vec![0],
        "8.x: `$g` at namespace scope resolves to the global `set g`: {locs:?}"
    );
    // TN: 9.0 — TIP 278 removed the fallback; the read errors at runtime.
    let a90 = analyse_as(src, "tcl9.0");
    let locs = definition(src, 2, col, &a90);
    assert!(
        locs.is_empty(),
        "9.0: no fallback — a definition here would be invented: {locs:?}"
    );
}

#[test]
fn definition_ns_scope_bare_var_never_reaches_an_intermediate_namespace_m11() {
    // FP guard: `$x` at `::a::b` scope sees `::a::b::x` and (8.x) `::x` —
    // never `::a::x`.  Both real tclshs error on this script.
    let src =
        "namespace eval a {\n    set x 1\n    namespace eval b {\n        puts $x\n    }\n}\n";
    let col = src.lines().nth(3).unwrap().find("$x").unwrap() as u32 + 1;
    for dialect in ["tcl8.6", "tcl9.0"] {
        let a = analyse_as(src, dialect);
        let locs = definition(src, 3, col, &a);
        assert!(
            locs.is_empty(),
            "[{dialect}] `::a::x` is never visible from `::a::b` scope: {locs:?}"
        );
    }
}

#[test]
fn definition_ns_scope_declared_variable_blocks_the_global_hop_m11() {
    // `variable v` creates the namespace's own cell, so the read resolves to
    // its declaration in BOTH versions (tclsh8.6 errors on the read rather
    // than seeing the global — the declared cell shadows it).
    let src = "set v 1\nnamespace eval bar {\n    variable v\n    puts $v\n}\n";
    let col = src.lines().nth(3).unwrap().find("$v").unwrap() as u32 + 1;
    for dialect in ["tcl8.6", "tcl9.0"] {
        let a = analyse_as(src, dialect);
        let locs = definition(src, 3, col, &a);
        assert_eq!(
            start_lines(&locs),
            vec![2],
            "[{dialect}] resolves to the `variable v` declaration: {locs:?}"
        );
    }
}

#[test]
fn definition_proc_body_bare_var_keeps_the_lenient_walk_m11() {
    // C Tcl errors here in BOTH versions (proc frames never had the
    // fallback), but the LSP's proc-context outward walk is a deliberate
    // navigation leniency (`global`-less quick scripts) that M11 leaves
    // untouched.  Pinned so the namespace gate can't leak into proc frames.
    let src = "set g 1\nproc p {} {\n    puts $g\n}\n";
    let col = src.lines().nth(2).unwrap().find("$g").unwrap() as u32 + 1;
    for dialect in ["tcl8.6", "tcl9.0"] {
        let a = analyse_as(src, dialect);
        let locs = definition(src, 2, col, &a);
        assert_eq!(
            start_lines(&locs),
            vec![0],
            "[{dialect}] proc-context leniency is unchanged by M11: {locs:?}"
        );
    }
}

#[test]
fn definition_reaches_the_colon_proc_via_relative_dispatch_934() {
    // The W314 case — `proc :` inside `namespace eval :` has no absolute
    // spelling — but `namespace eval : { : }` dispatches it RELATIVELY
    // (tclsh 8.6/9.0-pinned: → hello; `namespace inscope : :` too), so
    // navigation from the relative call site must reach the definition.
    let src = "namespace eval : { proc : args { return hello } }\nnamespace eval : { : }\n";
    let analysis = analyse(src);
    let call_col =
        u32::try_from(src.lines().nth(1).unwrap().rfind(':').unwrap()).expect("tiny test source");
    let locs = definition(src, 1, call_col, &analysis);
    assert_eq!(
        start_lines(&locs),
        vec![0],
        "the relative `:` call resolves to the unaddressable-but-reachable proc: {locs:?}"
    );
}
