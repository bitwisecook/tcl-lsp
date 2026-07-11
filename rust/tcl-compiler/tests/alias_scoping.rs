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

//! Alias-tracking + var-scoping suites for `tcl-compiler`.
//!
//! Two areas:
//!   * Cross-tool variable-aliasing tracking
//!     (the memory-SSA alias lattice, the lifecycle W211/W220 escape
//!     suppression, taint flow through aliases, and the LSP references/rename
//!     providers).
//!   * The shared var-scoping detection helpers
//!     (`global` / `variable` / `upvar` / `namespace upvar` declaration-index
//!     parsing).
//!
//! The relevant modules are
//! [`tcl_compiler::var_scoping`] (the declaration-index grammar) and
//! [`tcl_compiler::memory_ssa`] (the
//! alias lattice — `alias_sets`, `may_alias`, the union-find merge — driven off
//! the same `var_scoping` helpers). The alias-escape lifecycle is observed
//! through the analyser + `run_all_checks` diagnostic surface, exactly as
//! `analyser.rs` / `checks.rs` drive it.
//!
//! ## Structural (compiler-internal) vs. C-Tcl-observable proof split
//!
//! Two kinds of fact appear in these tests:
//!
//!   * **Structural / compiler-internal.** The alias *lattice* (how many alias
//!     sets a proc has, which names a set merges, what indices the declaration
//!     grammar returns) is a property of the compiler's memory-SSA + var-scoping
//!     passes, NOT of any Tcl runtime value. tclsh has no observable notion of
//!     "alias set" or "declaration index", so those assertions are structural
//!     and carry no `tclsh:` citation.
//!
//!   * **C-Tcl-observable.** The *reason* the lattice merges `a` and `b` for
//!     `upvar 1 x a; upvar 1 x b`, suppresses the dead/unused warnings on an
//!     aliased write, and parses each scope command the way it does, is that it
//!     mirrors real Tcl name binding: an `upvar`/`global`/`variable`/`namespace
//!     upvar` alias makes two *names* refer to one storage cell, and a write
//!     through one name is visible through the other; a bare `set` inside a proc
//!     is a frame-local that does not escape. Those binding semantics were
//!     confirmed against real `tclsh8.6` and `tclsh9.0` via
//!     `scripts/dev/tclsh_check.sh` and are cited inline with `// tclsh:`
//!     (identical output on 8.6 and 9.0 unless noted). Each such script sets a
//!     cell via one name and reads it back via the alias, so the observable
//!     value pins the binding the compiler models.
//!
//! ## Divergences / out-of-scope
//!
//!   * **Taint through aliases (2 of 3 cases).** A whole-source taint
//!     solver wired to the intra-procedural alias lattice would observe taint
//!     written through `a` at an `eval $b` sink. But
//!     `tcl_compiler::taint::find_taint_warnings` is a *per-function* sink
//!     scan taking a pre-solved taint map; the user-facing single-document
//!     surface (`run_all_checks`, the same one `analyser.rs` uses) does NOT
//!     thread the alias lattice into that solve, so `T100` does not propagate
//!     across the alias here. The *direct* sink case (no aliasing) is covered and
//!     passes; the two alias-dependent cases are documented below rather than
//!     asserted against the wrong surface. This is a surface limitation, not a
//!     resolution bug — the constant-control case correctly produces no `T100`.
//!
//!   * **LSP references / rename (5 cases).** These drive references / rename.
//!     The equivalents (`get_references` / `prepare_rename` / `get_rename_edits`)
//!     live in the **`tcl-lsp-core`** crate, not `tcl-compiler`, so they are out
//!     of scope for this `tcl-compiler` integration test and are not covered
//!     here.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::cfg::Function;
use tcl_compiler::cfg_builder::build_cfg;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::memory_ssa::{MemorySsaFunction, build_memory_ssa};
use tcl_compiler::ssa::build_ssa;
use tcl_compiler::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};
use tcl_registry::{CommandRegistry, registry_for_dialect};

/// Default dialect for reproducers that are not dialect-sensitive.
const D: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    registry_for_dialect(D)
}

/// `Vec<String>` of arg texts — the shape the `var_scoping` helpers take.
fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

/// Build the memory-SSA for the first procedure whose qualified name contains
/// `proc`: `build_memory_ssa(build_ssa(fn))` where `fn` is taken from the CFG
/// module's `procedures` map.
fn mem(source: &str, proc: &str) -> MemorySsaFunction {
    let module = build_cfg(&lower_to_ir(source, registry()), false);
    let f: &Function = module
        .procedures
        .iter()
        .find(|(qname, _)| qname.contains(proc))
        .map_or_else(
            || {
                panic!(
                    "proc {proc:?} not found in {:?}",
                    module.procedures.keys().collect::<Vec<_>>()
                )
            },
            |(_, f)| f,
        );
    build_memory_ssa(&build_ssa(f, registry()))
}

/// Every diagnostic code the full pipeline surfaces for `src`, mirroring the
/// user-facing `tcl diag` path: the analyser pass plus the `run_all_checks`
/// compiler-checks pass (shimmer / taint / dead-store), with optimisation codes
/// excluded. Copied from `analyser.rs::codes`.
fn codes(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = registry();
    let cu = CompilationUnit::build_for(src, registry, false);
    for d in run_all_checks(&cu, registry, Some(D)) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if `code` appears anywhere in the merged diagnostic set for `src`.
fn fires(src: &str, code: &str) -> bool {
    codes(src).iter().any(|c| c == code)
}

// ===========================================================================
// The memory-SSA alias lattice.
//
// STRUCTURAL: "alias set" is a compiler-internal artefact (memory-SSA union-
// find). The *behaviour* it models — a write through one upvar name being
// visible through another name aliasing the same caller cell — is C-Tcl-
// observable and pinned once below in `aliased_write_visible_through_alias`.
// ===========================================================================
mod alias_lattice_merge {
    use super::*;

    #[test]
    fn two_upvars_to_same_caller_merge() {
        // Both `a` and `b` alias caller var `x` — a write through `a` is visible
        // through `b`, so they must be one alias set. Regression for the
        // caller-node-keyed-on-local bug that split them. (Structural; the
        // observable binding is proven in `aliased_write_visible_through_alias`.)
        let m = mem(
            "proc f {} { upvar 1 x a\n upvar 1 x b\n set a 1\n set b 2 }",
            "f",
        );
        assert_eq!(m.alias_sets.len(), 1);
        let names = m.alias_sets[0].names();
        for n in ["a", "b", "x"] {
            assert!(names.contains(n), "alias set must contain {n}: {names:?}");
        }
    }

    #[test]
    fn two_upvars_may_alias_each_other() {
        let m = mem("proc f {} { upvar 1 x a\n upvar 1 x b\n set a 1 }", "f");
        assert!(m.may_alias("a", "b"));
        assert!(m.may_alias("b", "a"));
    }

    #[test]
    fn upvar_pair_may_alias_is_symmetric() {
        let m = mem("proc f {} { upvar 1 caller local\n set local 1 }", "f");
        assert!(m.may_alias("caller", "local"));
        assert!(m.may_alias("local", "caller"));
    }

    #[test]
    fn independent_callers_stay_separate() {
        // `a` aliases `x`; `b` aliases `y` — different caller cells, so they must
        // NOT merge (the fix must not over-merge). tclsh confirms the cells are
        // independent: writing through `a` leaves `y` (read via `b`) untouched.
        // tclsh: `set x 0; set y 0; proc f {} { upvar 1 x a; upvar 1 y b;
        //         set a 1; return $b }; puts [f]` ⇒ `0`  (8.6 and 9.0).
        let m = mem(
            "proc f {} { upvar 1 x a\n upvar 1 y b\n set a 1\n set b 2 }",
            "f",
        );
        assert!(!m.may_alias("a", "b"));
        assert_eq!(m.alias_sets.len(), 2);
    }

    #[test]
    fn aliased_write_visible_through_alias() {
        // The observable binding the lattice models: two upvars onto the same
        // caller cell share storage, so a write through `a` is read back through
        // `b`. This is *why* `a` and `b` merge into one alias set above.
        // tclsh: `set caller 0; proc f {} { upvar 1 caller a; upvar 1 caller b;
        //         set a 42; return $b }; puts [f]` ⇒ `42`  (8.6 and 9.0).
        let m = mem(
            "proc f {} { upvar 1 caller a\n upvar 1 caller b\n set a 42\n return $b }",
            "f",
        );
        assert!(m.may_alias("a", "b"), "shared-caller upvars alias");
        assert_eq!(m.alias_sets.len(), 1);
    }
}

// ===========================================================================
// Alias-escape lifecycle — an aliased write escapes the frame, so it is
// neither a dead store (W220) nor a set-but-never-used local (W211).
//
// C-Tcl-observable: each suppression is justified by the alias binding it
// mirrors (an `upvar`/`global`/`variable`/`namespace upvar` write mutates a
// cell that outlives the local frame), pinned with tclsh below. The control
// (`plain_local_still_flagged`) shows a bare `set` is frame-local and DOES NOT
// escape — also tclsh-pinned.
// ===========================================================================
mod alias_escapes_lifecycle {
    use super::*;

    #[test]
    fn upvar_write_only_not_unused_or_dead() {
        // `set x 5` mutates the caller's variable — a real side effect, not a
        // dead store / unused local.
        // tclsh: `set caller 0; proc f {} { upvar 1 caller x; set x 5 }; f;
        //         puts $caller` ⇒ `5`  (8.6 and 9.0): the write escaped the frame.
        assert!(!fires("proc f {} { upvar 1 caller x\n set x 5 }", "W211"));
        assert!(!fires("proc f {} { upvar 1 caller x\n set x 5 }", "W220"));
    }

    #[test]
    fn global_write_only_not_unused_or_dead() {
        // tclsh: `set ::g 0; proc f {} { global g; set g 5 }; f; puts $::g` ⇒ `5`
        // (8.6 and 9.0): `global g` binds the namespace var, so the write escapes.
        assert!(!fires("proc f {} { global g\n set g 5 }", "W211"));
        assert!(!fires("proc f {} { global g\n set g 5 }", "W220"));
    }

    #[test]
    fn variable_write_only_not_unused_or_dead() {
        // tclsh: `namespace eval ns { variable v 0 }; proc ns::f {} { variable v;
        //         set v 5 }; ns::f; puts $::ns::v` ⇒ `5` (8.6 and 9.0): `variable v`
        //         binds `::ns::v`, so the write escapes the proc frame.
        assert!(!fires("proc f {} { variable v\n set v 5 }", "W211"));
        assert!(!fires("proc f {} { variable v\n set v 5 }", "W220"));
    }

    #[test]
    fn namespace_upvar_write_only_not_unused_or_dead() {
        // `namespace upvar` lowers to IR command `namespace` (not a name in the
        // alias-command set), so a naive command-name guard misses it; the
        // var_scoping grammar still recognises `dst` as an alias.
        // tclsh: `namespace eval ns { variable src 0 }; proc f {} {
        //         namespace upvar ::ns src dst; set dst 55 }; f; puts $::ns::src`
        //         ⇒ `55`  (8.6 and 9.0): the aliased write escapes to `::ns::src`.
        assert!(!fires(
            "proc f {} { namespace upvar ::ns src dst\n set dst 5 }",
            "W211"
        ));
        assert!(!fires(
            "proc f {} { namespace upvar ::ns src dst\n set dst 5 }",
            "W220"
        ));
    }

    #[test]
    fn plain_local_still_flagged() {
        // Control: the escape suppression must not silence a genuinely unused
        // local. A bare `set` inside a proc is frame-local and does NOT escape.
        // tclsh: `proc f {} { set y 5 }; f; puts [info exists ::y]` ⇒ `0`
        // (8.6 and 9.0): `y` never reached the caller, so it is genuinely unused.
        // The local is flagged W211 ("set but never used"); the co-located
        // dead-store W220 is deduped in favour of that single, clearer hint.
        assert!(fires("proc f {} { set y 5 }", "W211"));
        assert!(!fires("proc f {} { set y 5 }", "W220"));
    }

    #[test]
    fn aliased_overwrite_is_not_dead_store() {
        // `set a 1` is read through the aliased name `b` — not dead even though
        // there is no read of `a` itself.
        // tclsh: `set c 0; proc f {} { upvar 1 c a; upvar 1 c b; set a 1;
        //         return $b }; puts [f]` ⇒ `1`  (8.6 and 9.0): the store is live
        //         because it is observed through the alias `b`.
        assert!(!fires(
            "proc f {} { upvar 1 c a\n upvar 1 c b\n set a 1\n return $b }",
            "W220"
        ));
    }

    #[test]
    fn upvar_hash_zero_global_write_escapes() {
        // `upvar #0 g l` binds the absolute-frame (global) cell `g`; a write
        // through `l` escapes, so it is neither unused nor dead. (`#0` is the
        // `looks_like_level` `#<digits>` form — covered structurally below and
        // pinned here behaviourally.)
        // tclsh: `proc f {} { upvar #0 g l; set l 88 }; f; puts $::g` ⇒ `88`
        // (8.6 and 9.0).
        assert!(!fires("proc f {} { upvar #0 g l\n set l 5 }", "W211"));
        assert!(!fires("proc f {} { upvar #0 g l\n set l 5 }", "W220"));
    }
}

// ===========================================================================
// Taint through alias — only the direct (non-aliased) sink case maps to the
// `run_all_checks` surface; see the module-level DIVERGENCE note for the two
// alias-dependent cases.
// ===========================================================================
mod taint_through_alias {
    use super::*;

    #[test]
    fn direct_sink_still_warns() {
        // Baseline that does not depend on aliasing at all: a tainted source
        // flowing straight into an `eval` sink fires T100. (`HTTP::payload` is an
        // iRules taint source; T100 is the eval-of-tainted-data sink warning.)
        assert!(fires("set x [HTTP::payload]\neval $x", "T100"));
    }

    #[test]
    fn taint_through_expr_command_substitution_still_warns() {
        // RUST_ISSUE_021: a taint source nested in an `[expr {…}]` command
        // substitution must propagate into the assigned variable, so a later
        // `eval $x` fires T100. Storing the value through `expr` previously
        // laundered the taint (join_uses saw no `$var`), a false negative.
        assert!(fires("set x [expr {[HTTP::payload] + 1}]\neval $x", "T100"));
    }

    #[test]
    fn untainted_expr_does_not_warn() {
        // Control: an `[expr {…}]` with no taint source must not invent taint.
        assert!(!fires("set x [expr {1 + 2}]\neval $x", "T100"));
    }

    #[test]
    fn untainted_alias_does_not_warn() {
        // Control (holds on the `run_all_checks` surface): a constant flowing
        // through the alias must not raise a taint sink warning — the closure
        // must not invent taint. (The *positive* alias-propagation case is the
        // documented divergence; this negative control passes on either surface.)
        let src = "proc f {} {\n    upvar 1 c a\n    upvar 1 c b\n    set a \"constant\"\n    eval $b\n}\n";
        assert!(!fires(src, "T100"));
    }

    // DIVERGENCE — taint flowing between aliases of the same caller: T100 would
    // ideally fire for
    //   proc f {} { upvar 1 c a; upvar 1 c b; set a [HTTP::payload]; eval $b }
    // if a whole-source `find_taint_warnings` solver threaded the intra-
    // procedural alias lattice. The single-document
    // `run_all_checks` surface does not propagate taint across the alias, so no
    // T100 fires here. Not asserted against the wrong surface; see module note.
}

// ===========================================================================
// global_declaration_indices — the declaration-index grammar.
//
// STRUCTURAL: these pin the declaration-index *grammar* the memory-SSA alias
// detector and the LSP declaration provider share. The grammar exists to mirror
// which words `global` actually binds in Tcl; that binding is pinned
// behaviourally in `alias_escapes_lifecycle::global_write_only_*`.
// ===========================================================================
mod global_declaration_indices_tests {
    use super::*;

    #[test]
    fn multiple_names() {
        assert_eq!(
            global_declaration_indices(&v(&["a", "b", "c"])),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn skips_substituted_refs() {
        // `global $dynamic` is a runtime-resolved name; the compiler ignores it
        // as a static declaration, so we do too.
        assert_eq!(global_declaration_indices(&v(&["$dynamic", "b"])), vec![1]);
    }

    // NOTE: there is no token-vs-string variance to test here — the helper is
    // statically typed on `&[String]`, so the guarantee is simply that callers
    // pass arg *texts*, which every call site does.
}

// ===========================================================================
// variable_declaration_indices — the declaration-index grammar.
// STRUCTURAL grammar; the `variable` binding it models is tclsh-pinned in
// `alias_escapes_lifecycle::variable_write_only_*`.
// ===========================================================================
mod variable_declaration_indices_tests {
    use super::*;

    #[test]
    fn even_indices_only() {
        // variable name1 value1 name2 value2  → names at the even positions.
        assert_eq!(
            variable_declaration_indices(&v(&["counter", "0", "flag", "false"])),
            vec![0, 2]
        );
    }

    #[test]
    fn name_without_value() {
        // variable counter   (no initial value)
        assert_eq!(variable_declaration_indices(&v(&["counter"])), vec![0]);
    }

    #[test]
    fn skips_substituted_names() {
        assert!(variable_declaration_indices(&v(&["$dynamic", "0"])).is_empty());
    }
}

// ===========================================================================
// upvar_local_declaration_indices — the declaration-index grammar.
//
// STRUCTURAL: the index grammar across every level-word form (default / decimal
// / `#N`) and both `namespace upvar` lowerings. The grammar exists to mirror
// which token names the local alias in real Tcl; that alias binding is pinned
// behaviourally in `alias_escapes_lifecycle` (upvar + namespace-upvar writes
// escaping their caller / namespace cell).
// ===========================================================================
mod upvar_local_declaration_indices_tests {
    use super::*;

    #[test]
    fn default_level() {
        // upvar other local           → pair (other, local); local at index 1.
        assert_eq!(
            upvar_local_declaration_indices("upvar", &v(&["other", "local"])),
            vec![1]
        );
    }

    #[test]
    fn integer_level() {
        // upvar 1 other local         → pair (other, local); local at index 2.
        assert_eq!(
            upvar_local_declaration_indices("upvar", &v(&["1", "other", "local"])),
            vec![2]
        );
    }

    #[test]
    fn hash_level_arbitrary_digits() {
        // `upvar #3 other local` — arbitrary #N levels are recognised.
        assert_eq!(
            upvar_local_declaration_indices("upvar", &v(&["#3", "other", "local"])),
            vec![2]
        );
    }

    #[test]
    fn hash_zero_level() {
        assert_eq!(
            upvar_local_declaration_indices("upvar", &v(&["#0", "g", "l"])),
            vec![2]
        );
    }

    #[test]
    fn multiple_pairs() {
        // upvar 1 a A b B             → locals A (idx 2) and B (idx 4).
        assert_eq!(
            upvar_local_declaration_indices("upvar", &v(&["1", "a", "A", "b", "B"])),
            vec![2, 4]
        );
    }

    #[test]
    fn skips_substituted_names() {
        // Pair with a $-prefixed side is dropped.
        assert!(upvar_local_declaration_indices("upvar", &v(&["1", "$dyn", "local"])).is_empty());
    }

    #[test]
    fn namespace_upvar_lowered_form() {
        // Lowered `namespace upvar ns src dst` form: command="namespace",
        // args[0] == "upvar", args[1] == namespace, args[2:] are pairs.
        assert_eq!(
            upvar_local_declaration_indices("namespace", &v(&["upvar", "mymod", "src", "dst"])),
            vec![3]
        );
    }

    #[test]
    fn namespace_upvar_direct_form() {
        // command="namespace upvar", args[0] == namespace, args[1:] are pairs.
        assert_eq!(
            upvar_local_declaration_indices("namespace upvar", &v(&["mymod", "src", "dst"])),
            vec![2]
        );
    }

    #[test]
    fn empty_args() {
        assert!(upvar_local_declaration_indices("upvar", &[]).is_empty());
    }
}
