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

//! Semantic-analyser tests.
//!
//! These drive `analyser.analyse(src)` and assert over the
//! returned result's `.diagnostics` (W###/E###/I###/H### codes) *and* its
//! structured semantic model (`global_scope`, `all_procs`, `all_classes`,
//! `regex_patterns`, `package_requires`, `command_aliases`, `source_targets`,
//! `unknown_proc_info`). [`tcl_compiler::analyser::AnalysisResult`]
//! exposes all these fields.
//!
//! ## Diagnostic surface
//!
//! Diagnostic assertions go through [`codes`]/[`fires`], copied verbatim from
//! the in-crate harness `src/analyser/diagnostics/fp/mod.rs`: they merge the
//! analyser pass with the `run_all_checks` compiler-checks pass (shimmer /
//! taint / dead-store) and drop optimisation codes, exactly mirroring the
//! user-facing `tcl diag` surface.
//!
//! ## C-Tcl ground truth
//!
//! Diagnostics that mirror a real tclsh error are pinned to ground truth from
//! `scripts/dev/tclsh_check.sh` (tclsh8.6 + tclsh9.0 on PATH). The headline
//! correspondences, verified while authoring this file:
//!   * `puts $x`            → `can't read "x": no such variable`      (W210)
//!   * `set`                → `wrong # args: should be "set …"`       (E002)
//!   * `break extra`        → `wrong # args: should be "break"`       (E003)
//!   * `regsub a b c d e`   → `wrong # args: should be "regsub …"`    (E003)
//!   * `string bogus hello` → `unknown … subcommand "bogus"`          (W001)
//!   * `namespace`          → `wrong # args: should be "namespace …"` (E001)
//!   * `unset x` (unset)    → `can't unset "x": no such variable`     (W213)
//!
//! Pure static-analysis heuristics with no direct runtime analogue (unused
//! params, dead stores, paste-error hints, etc.) are noted as such inline.
//!
//! ## Behavioural notes
//!
//! A handful of cases target features the single-document analyser +
//! `run_all_checks` surface does not reproduce (they live in the cross-file
//! workspace index). Each is asserted against the actual verdict with an
//! explanatory comment; where the behaviour is tracked elsewhere it is omitted.
//! See the per-section notes below.

use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::registry_for_dialect;

/// Default dialect for reproducers that are not dialect-sensitive.
const D: &str = "tcl8.6";

/// Every diagnostic code the full pipeline surfaces for `src` under `dialect`,
/// mirroring the user-facing `tcl diag` path: the analyser pass plus the
/// `run_all_checks` compiler-checks pass (shimmer / taint / dead-store), with
/// optimisation codes excluded exactly as `diag` excludes them.
///
/// Copied verbatim from `src/analyser/diagnostics/fp/mod.rs::codes`.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if `code` appears anywhere in the full diagnostic set for `src`.
fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// Count of how many times `code` appears in the merged diagnostic set.
fn count(src: &str, dialect: &str, code: &str) -> usize {
    codes(src, dialect)
        .iter()
        .filter(|c| c.as_str() == code)
        .count()
}

/// Full `(code, message)` pairs from the *analyser pass only* — used when a
/// test pins a message substring or message-scoped count (the analyser
/// pass owns every code those tests inspect: W210/W211/W213/W214/W215/W216/
/// W123/E001/E002/E003/W001).
fn analyser_diags(src: &str, dialect: &str) -> Vec<(String, String, Severity)> {
    Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| (d.code.to_string(), d.message.clone(), d.severity))
        .collect()
}

/// Count of W210 diagnostics whose message names `var`.
fn w210_for(src: &str, var: &str) -> usize {
    analyser_diags(src, D)
        .iter()
        .filter(|(c, m, _)| c == "W210" && m.contains(var))
        .count()
}

// ===========================================================================
// Proc records in the semantic model.
// ===========================================================================
mod proc_analysis {
    use super::*;

    #[test]
    fn simple_proc_records_name_and_params() {
        let r = Analyser::new().analyse("proc greet {name} { puts $name }", D);
        let proc = r.global_scope.procs.get("greet").expect("greet recorded");
        assert_eq!(proc.name, "greet");
        assert_eq!(proc.qualified_name, "::greet");
        assert_eq!(proc.params.len(), 1);
        assert_eq!(proc.params[0].name, "name");
    }

    #[test]
    fn multiple_params_in_order() {
        let r = Analyser::new().analyse("proc add {a b} { return [+ $a $b] }", D);
        let proc = &r.global_scope.procs["add"];
        assert_eq!(
            proc.params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn default_param_value_captured() {
        let r = Analyser::new().analyse("proc greet {{name World}} { puts $name }", D);
        let p = &r.global_scope.procs["greet"].params[0];
        assert_eq!(p.name, "name");
        assert!(p.has_default);
        assert_eq!(p.default_value.as_deref(), Some("World"));
    }

    #[test]
    fn no_params_is_empty() {
        let r = Analyser::new().analyse("proc noop {} { }", D);
        assert_eq!(r.global_scope.procs["noop"].params.len(), 0);
    }

    #[test]
    fn proc_in_all_procs_qualified() {
        let r = Analyser::new().analyse("proc foo {} {}", D);
        assert!(r.all_procs.contains_key("::foo"));
    }

    #[test]
    fn doc_harvested_from_preceding_comment() {
        let src = "# Adds two numbers\nproc add {a b} { return [+ $a $b] }\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.global_scope.procs["add"].doc.contains("Adds two numbers"));
    }

    #[test]
    fn multiple_procs_each_recorded() {
        let r = Analyser::new().analyse("proc foo {} {}\nproc bar {x} { return $x }\n", D);
        assert!(r.global_scope.procs.contains_key("foo"));
        assert!(r.global_scope.procs.contains_key("bar"));
    }

    #[test]
    fn proc_creates_child_scope_with_param_and_local() {
        let r = Analyser::new().analyse("proc foo {x} { set y 1 }", D);
        assert_eq!(r.global_scope.children.len(), 1);
        let s = &r.global_scope.children[0];
        assert_eq!(s.kind, tcl_compiler::analyser::ScopeKind::Proc);
        assert_eq!(s.name, "foo"); // proc-scope is keyed by the simple name
        assert!(s.variables.contains_key("x"));
        assert!(s.variables.contains_key("y"));
    }
}

// ===========================================================================
// `tcl::OptProc` — the `opt` package's automatic-option-parsing proc
// definer (issue #923 idx 90). Runtime mechanism (tclsh9.0/8.6-verified):
// installs `::proc $name args {...}` unconditionally — the real Tcl-level
// signature is always the single `args` catch-all, regardless of what
// `optlist` declares; `optlist`'s own descriptor words are bound as local
// variables in the body by `::tcl::OptKeyParse`, with a leading `-` on a
// flag descriptor stripped for the bound name.
// ===========================================================================
mod opt_proc_definer {
    use super::*;

    #[test]
    fn records_proc_with_real_args_only_arity() {
        // TP — the analyser must record the *real* Tcl-level signature,
        // never `optlist`'s own descriptor words.
        let r = Analyser::new().analyse(
            "tcl::OptProc greet {child -use -display} { return $child }",
            D,
        );
        let proc = r.all_procs.get("::greet").expect("greet recorded");
        assert_eq!(
            proc.params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["args"],
            "the real Tcl-level signature is always the single args catch-all"
        );
    }

    #[test]
    fn fully_qualified_spelling_also_registers() {
        // TP — real corpus code commonly writes this fully qualified;
        // `resolve_analyser_hook` must resolve it identically to the bare
        // spelling (issue #923 idx 90).
        let r = Analyser::new().analyse(
            "::tcl::OptProc greet {child -use -display} { return $child }",
            D,
        );
        assert!(r.all_procs.contains_key("::greet"), "{:?}", r.all_procs);
    }

    #[test]
    fn call_with_any_arity_draws_no_wrong_arg_count_diagnostic() {
        // TP — the finding's own headline claim: every real call
        // previously misreported "wrong number of arguments" because the
        // stub proc's `{}`-arity `ProcDef` was never overwritten.
        let src = "tcl::OptProc greet {child -use -display} { return $child }\ngreet a b c d\n";
        assert!(!fires(src, D, "E003"), "{:?}", codes(src, D));
    }

    #[test]
    fn optlist_flag_descriptor_dash_is_stripped_for_the_bound_local() {
        // TP — `::tcl::OptKeyParse` binds `-use`/`-display` as `use`/
        // `display`, never with the leading dash.
        let src =
            "tcl::OptProc greet {child -use -display} { return \"$use $display\" }\ngreet a\n";
        assert!(
            !fires(src, D, "W210"),
            "use/display must resolve as bound locals, not read-before-set: {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn args_catch_all_is_readable_in_the_body() {
        // TP — `$args` still holds the whole original argument list after
        // `OptKeyParse` runs, so a body reference to it (inspecting
        // leftovers) is legitimate, not a false read-before-set.
        let src = "tcl::OptProc greet {child} { return $args }\ngreet a\n";
        assert!(!fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    #[test]
    fn dash_stripped_local_does_not_collide_with_its_own_optlist_declaration() {
        // FN guard: a naive anchor for the synthetic `args` binding (this
        // idiom writes no literal `args` word anywhere) could collide with
        // another symbol's own span and silently hide it — every
        // optlist-derived local must still resolve to its own name, not
        // `args` (issue #923 idx 90 regression: the fix's first attempt
        // anchored `args` to the whole `optlist` word, which swallowed
        // every one of its own descriptor sub-spans).
        let r = Analyser::new().analyse(
            "tcl::OptProc greet {child -use -display} { return $child }",
            D,
        );
        let scope = &r.global_scope.children[0];
        assert!(
            scope.variables.contains_key("child"),
            "{:?}",
            scope.variables
        );
        assert!(scope.variables.contains_key("use"), "{:?}", scope.variables);
        assert!(
            scope.variables.contains_key("display"),
            "{:?}",
            scope.variables
        );
        assert!(
            scope.variables.contains_key("args"),
            "{:?}",
            scope.variables
        );
    }

    #[test]
    fn unrelated_proc_still_gets_real_arity_checked() {
        // FP guard: the fix must not loosen arity checking for ordinary
        // procs elsewhere in the same document.
        let src = "proc real {a b} { return [+ $a $b] }\nreal 1\n";
        assert!(fires(src, D, "E002"), "{:?}", codes(src, D));
    }

    #[test]
    fn plain_double_proc_redefinition_still_last_definition_wins() {
        // TN — control: an ordinary double-`proc` redefinition (unrelated
        // to `tcl::OptProc`) is unaffected by any of this fix's changes.
        let r = Analyser::new().analyse(
            "proc greet {} { return 1 }\nproc greet {x} { return $x }\n",
            D,
        );
        let proc = r.all_procs.get("::greet").expect("greet recorded");
        assert_eq!(
            proc.params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["x"],
            "last definition wins: {:?}",
            proc.params
        );
    }
}

// ===========================================================================
// Variable definitions across scopes.
// ===========================================================================
mod variable_analysis {
    use super::*;

    #[test]
    fn set_defines_var() {
        let r = Analyser::new().analyse("set x 42", D);
        assert!(r.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn multiple_sets_define_each() {
        let r = Analyser::new().analyse("set x 1\nset y 2\nset z 3", D);
        for v in ["x", "y", "z"] {
            assert!(r.global_scope.variables.contains_key(v), "{v} missing");
        }
    }

    #[test]
    fn incr_defines_var() {
        let r = Analyser::new().analyse("incr counter", D);
        assert!(r.global_scope.variables.contains_key("counter"));
    }

    #[test]
    fn array_set_defines_base_name() {
        let r = Analyser::new().analyse("set arr(key) value", D);
        assert!(r.global_scope.variables.contains_key("arr"));
    }

    #[test]
    fn proc_local_not_in_global_scope() {
        let r = Analyser::new().analyse("proc foo {} { set local 1 }", D);
        assert!(!r.global_scope.variables.contains_key("local"));
        assert!(r.global_scope.children[0].variables.contains_key("local"));
    }

    #[test]
    fn set_one_arg_is_read_only() {
        // `set x` (1 arg) is a *read*, not a definition. C-Tcl: `set x` on an
        // undefined var errors `can't read "x"`, confirming it is a read.
        let r = Analyser::new().analyse("set x", D);
        assert!(!r.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn variable_command_skips_value_word() {
        let r = Analyser::new().analyse("variable port 8080", D);
        assert!(r.global_scope.variables.contains_key("port"));
        assert!(!r.global_scope.variables.contains_key("8080"));
    }
}

// ===========================================================================
// Issue #1108 — a registry `VarRead`-role name word is a reference site.
//
// A variable is read by more than `$name`. Any command whose spec puts an
// argument in `ArgRole::VarRead` reads the cell that word names, and tclsh
// agrees (9.0.4 and 8.6.16, byte-identical):
//
//   proc f {} {set m 1; puts [set m]};        f   -> 1
//   proc f {} {set m 1; puts [info exists m]}; f  -> 1
//   proc f {} {set m 1; return [set m]};      f   -> 1
//
// Before the fix only `$m` and the *statement* form `set m` reached
// `VarDef::references`, so Find References / document-highlight / the
// minifier's rename pass under-reported every one of these sites.
// ===========================================================================
mod var_read_role_references {
    use super::*;

    /// The read spans recorded for `name` in the first proc scope, as the
    /// source text each one covers.
    fn read_texts(src: &str, name: &str) -> Vec<String> {
        fn walk(
            scope: &tcl_compiler::analyser::Scope,
            name: &str,
            src: &str,
        ) -> Option<Vec<String>> {
            if let Some(v) = scope.variables.get(name) {
                return Some(
                    v.references
                        .iter()
                        .map(|r| src[r.start() as usize..r.end() as usize].to_owned())
                        .collect(),
                );
            }
            scope.children.iter().find_map(|c| walk(c, name, src))
        }
        walk(&Analyser::new().analyse(src, D).global_scope, name, src).unwrap_or_default()
    }

    #[test]
    fn tp_a_substituted_set_read_is_recorded() {
        // FN before the fix: `refs` was empty.
        assert_eq!(
            read_texts("proc f {} {\n    set m 1\n    puts [set m]\n}\n", "m"),
            vec!["m".to_owned()],
            "`[set m]` is a read of `m`"
        );
    }

    #[test]
    fn tp_every_var_read_role_contributes_not_just_set() {
        // Registry-driven, so the fix is not about `set`: `info exists` and
        // `array get` declare `VarRead` positions too.
        for (src, name) in [
            ("proc f {} {\n    set m 1\n    info exists m\n}\n", "m"),
            (
                "proc f {} {\n    set m 1\n    puts [info exists m]\n}\n",
                "m",
            ),
            (
                "proc f {} {\n    array set a {k 1}\n    puts [array get a]\n}\n",
                "a",
            ),
        ] {
            assert_eq!(
                read_texts(src, name),
                vec![name.to_owned()],
                "a `VarRead`-role word must be a reference site: {src:?}"
            );
        }
    }

    #[test]
    fn tn_the_write_form_records_no_read() {
        // `set m 1` is a definition, not a read — the two-argument form has no
        // `VarRead` role at all, so nothing new appears.
        assert!(
            read_texts("proc f {} {\n    set m 1\n}\n", "m").is_empty(),
            "a plain write must not become its own reference"
        );
    }

    #[test]
    fn tn_a_computed_name_records_nothing() {
        // `set $n` reads whatever *value* `n` holds; which cell that is cannot
        // be known statically, so no read is attributed to `m`.
        assert!(
            read_texts(
                "proc f {} {\n    set m 1\n    set n m\n    puts [set $n]\n}\n",
                "m"
            )
            .is_empty(),
            "a `$`-computed name must not be credited to a same-spelled cell"
        );
    }

    #[test]
    fn fp_the_statement_form_is_recorded_exactly_once() {
        // `set m` is recorded by `set`'s own handler *and* by the generic
        // role pass; the sink dedupes by span, so the site appears once. A
        // duplicate would make hover's "N reference(s)" count read double.
        assert_eq!(
            read_texts("proc f {} {\n    set m 1\n    set m\n}\n", "m"),
            vec!["m".to_owned()],
            "one source location is one reference"
        );
    }

    #[test]
    fn tp_a_brace_quoted_name_word_reads_the_literal_cell() {
        // Issue #1078's cell, read through the role path: `{$n}` names the
        // variable *called* `$n`. tclsh 9.0.4 / 8.6.16: `set {$n} v; set {$n}`
        // -> v, while `info exists n` -> 0.
        let src = "proc f {} {\n    set {$n} 1\n    puts [set {$n}]\n}\n";
        // The recorded span is the word token's, which follows the inner-end
        // convention — the closing `}` sits one past its end, exactly as the
        // `{$n}` *declaration*'s span does.
        assert_eq!(
            read_texts(src, "$n"),
            vec!["{$n".to_owned()],
            "the literal cell owns the read"
        );
        assert!(
            read_texts(src, "n").is_empty(),
            "the unrelated plain cell must own nothing"
        );
    }
}

// ===========================================================================
// Issue #1138 — a script argument *built* with `list` is walked as the
// command it provably is.
//
// tclsh 9.0.4 and 8.6.16 agree that the three spellings are functionally
// identical:
//
//   proc f {} {upvar #0 g l; set l 1}
//   proc f {} {uplevel #0 {upvar #0 g l}; …}
//   proc f {} {uplevel #0 [list upvar #0 g l]; …}
//
// `list` packs its already-substituted arguments into exactly one command,
// so the built form is not dynamic — yet every pass keyed on a literal
// `{…}` body skipped it.
// ===========================================================================
mod list_quoted_script_arguments {
    use super::*;

    fn scope_named(
        scope: &tcl_compiler::analyser::Scope,
        kind: tcl_compiler::analyser::ScopeKind,
    ) -> Option<&tcl_compiler::analyser::Scope> {
        if scope.kind == kind {
            return Some(scope);
        }
        scope.children.iter().find_map(|c| scope_named(c, kind))
    }

    #[test]
    fn tp_a_list_built_uplevel_body_declares_in_the_uplevel_frame() {
        use tcl_compiler::analyser::ScopeKind;
        for src in [
            "proc f {} {\n    uplevel 1 [list set inner 1]\n}\n",
            // The braced control, which already worked — both must agree.
            "proc f {} {\n    uplevel 1 {set inner 1}\n}\n",
        ] {
            let r = Analyser::new().analyse(src, D);
            let up = scope_named(&r.global_scope, ScopeKind::Uplevel)
                .unwrap_or_else(|| panic!("an uplevel frame scope must open for {src:?}"));
            assert!(
                up.variables.contains_key("inner"),
                "`inner` belongs to the uplevel frame, not the proc: {src:?}"
            );
        }
    }

    #[test]
    fn tp_a_list_built_namespace_body_declares_in_the_namespace() {
        let src = "namespace eval ::ns [list set v 1]\n";
        let r = Analyser::new().analyse(src, D);
        let ns = &r.global_scope.children[0];
        assert!(
            ns.variables.contains_key("v"),
            "the built `set v 1` runs in `::ns`: {:?}",
            ns.variables.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn fp_a_list_built_uplevel_body_does_not_claim_the_substitution_bytes() {
        let src = "proc p {file} {\n    uplevel #0 [list source [file join $file]]\n}\n";
        let r = Analyser::new().analyse(src, D);
        let proc_scope = &r.global_scope.children[0];
        let file_var = proc_scope.variables.get("file").expect("param");
        let read_off = u32::try_from(src.find("$file").unwrap()).unwrap();
        assert!(
            file_var.references.iter().any(|s| s.start() == read_off),
            "the $file read at {read_off} must attach to the proc param; refs {:?}",
            file_var.references
        );
        let up = scope_named(&r.global_scope, tcl_compiler::analyser::ScopeKind::Uplevel)
            .expect("uplevel scope opens");
        assert!(
            up.body_span.is_none(),
            "a list-built uplevel body must not claim the substitution bytes: {:?}",
            up.body_span
        );
    }

    #[test]
    fn fp_a_rewalked_built_body_emits_no_duplicate_diagnostics() {
        let src = "proc p {} {\n    namespace eval :: [list puts [string index abc 99]]\n}\n";
        let r = Analyser::new().analyse(src, D);
        let mut spans: Vec<_> = r
            .diagnostics
            .iter()
            .map(|d| (d.code.as_str().to_owned(), d.span.start(), d.span.end()))
            .collect();
        let n = spans.len();
        spans.sort();
        spans.dedup();
        eprintln!(
            "diags: {:?}",
            r.diagnostics
                .iter()
                .map(|d| (d.code.as_str(), d.span, d.message.clone()))
                .collect::<Vec<_>>()
        );
        assert_eq!(n, spans.len(), "duplicate diagnostics from the re-walk");
    }

    #[test]
    fn fp_a_nested_subst_in_a_built_body_is_not_rerecorded_in_the_target_scope() {
        let src = "namespace eval ::app {\n    proc helper {} { return 1 }\n    proc user {} { namespace eval :: [list puts [helper]] }\n}\n";
        let r = Analyser::new().analyse(src, D);
        let inv: Vec<_> = r
            .command_invocations
            .iter()
            .filter(|i| i.name.contains("helper"))
            .collect();
        eprintln!("helper invocations: {inv:?}");
        assert!(inv.len() <= 1, "double-recorded invocation: {inv:?}");
    }

    #[test]
    fn tn_a_dynamic_build_stays_an_opaque_barrier() {
        // `[list $cb inner 1]` names no statically known command, and
        // `[$build …]` is not a list build at all — both keep today's
        // behaviour rather than guessing.
        use tcl_compiler::analyser::ScopeKind;
        for src in [
            "proc f {} {\n    uplevel 1 [list $cb inner 1]\n}\n",
            "proc f {} {\n    uplevel 1 [$build inner]\n}\n",
        ] {
            let r = Analyser::new().analyse(src, D);
            assert!(
                scope_named(&r.global_scope, ScopeKind::Uplevel).is_none(),
                "an unresolvable body must not be walked: {src:?}"
            );
        }
    }

    #[test]
    fn tp_a_substituted_read_in_a_built_body_belongs_to_the_building_frame() {
        // `::tk::SourceLibFile`'s real shape. `$file` is the proc's own
        // parameter, substituted in the proc frame *before* `namespace eval`
        // enters `::` — so the read is the proc's, and the namespace scope
        // must not claim those bytes (issue #1138 idx 102).
        let src = "proc ::tk::SourceLibFile {file} {\n    \
namespace eval :: [list source [file join $::tk_library $file.tcl]]\n\
}\n";
        let r = Analyser::new().analyse(src, D);
        let proc_scope = &r.global_scope.children[0];
        let file_var = &proc_scope.variables["file"];
        assert_eq!(
            file_var
                .references
                .iter()
                .map(|s| &src[s.start() as usize..s.end() as usize])
                .collect::<Vec<_>>(),
            vec!["$file"],
            "the parameter's read must be recorded"
        );
        let ns = proc_scope
            .children
            .iter()
            .find(|c| c.kind == tcl_compiler::analyser::ScopeKind::Namespace)
            .expect("the `namespace eval ::` scope exists");
        assert!(
            ns.body_span.is_none(),
            "a `[…]` body is evaluated in the calling frame, so the namespace \
scope must not own its bytes — owning them made the scope-chain lookup stop \
there and answer nothing for `$file`"
        );
    }

    #[test]
    fn tn_a_braced_namespace_body_still_owns_its_bytes() {
        // Control for the span above: a literal `{…}` body really does run in
        // the namespace frame, so it keeps its `body_span`.
        let src = "namespace eval ::ns {\n    variable v 1\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(
            r.global_scope.children[0].body_span.is_some(),
            "a literal braced body is the namespace's own"
        );
    }
}

// ===========================================================================
// Namespace scopes + qualified procs.
// ===========================================================================
mod namespace_analysis {
    use super::*;

    #[test]
    fn namespace_eval_creates_namespace_scope() {
        let src = "namespace eval myns {\n    proc helper {} { return 1 }\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert_eq!(r.global_scope.children.len(), 1);
        let ns = &r.global_scope.children[0];
        assert_eq!(ns.kind, tcl_compiler::analyser::ScopeKind::Namespace);
        assert_eq!(ns.name, "myns");
        assert!(ns.procs.contains_key("helper"));
    }

    #[test]
    fn namespace_qualified_proc_in_all_procs() {
        let src = "namespace eval math {\n    proc add {a b} { return [+ $a $b] }\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.all_procs.contains_key("::math::add"));
    }
}

// ===========================================================================
// Arity (E001/E002/E003), unknown subcommand (W001),
// read-before-set (W210), and the constant-branch family (I230).
// ===========================================================================
mod diagnostics {
    use super::*;

    // --- builtin arity (E001/E002/E003), all tclsh-observable `wrong # args` ---

    #[test]
    fn too_few_args_set_fires_e002() {
        // tclsh8.6/9.0: `set` → wrong # args: should be "set varName ?newValue?".
        let ds = analyser_diags("set", D);
        let errs: Vec<_> = ds
            .iter()
            .filter(|(_, _, s)| *s == Severity::Error)
            .collect();
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|(_, m, _)| m.contains("Too few")));
    }

    #[test]
    fn too_many_args_break_fires_e003() {
        // tclsh8.6/9.0: `break extra` → wrong # args: should be "break".
        let ds = analyser_diags("break extra", D);
        let errs: Vec<_> = ds
            .iter()
            .filter(|(_, _, s)| *s == Severity::Error)
            .collect();
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|(_, m, _)| m.contains("Too many")));
    }

    #[test]
    fn correct_arity_no_error() {
        // `set x 42` is valid in tclsh; no Error-severity diagnostic.
        let ds = analyser_diags("set x 42", D);
        assert!(!ds.iter().any(|(_, _, s)| *s == Severity::Error));
    }

    #[test]
    fn puts_arity_options_not_counted() {
        // `puts -nonewline stderr hello` is valid (the switch is not a
        // positional); `puts a b c` is over-arity. tclsh agrees.
        assert!(!fires("puts -nonewline stderr hello", D, "E003"));
        assert!(fires("puts a b c", D, "E003"));
    }

    #[test]
    fn declared_switches_not_counted_as_positional() {
        // Regression #455: declared option flags are skipped before counting.
        // These regsub switches exist in every supported dialect.
        for snippet in [
            "regsub -all -line {\\n} $args {} str",
            "regsub -all {a} $b {} c",
            "regsub -nocase -all -- $pat $s {} out",
        ] {
            assert!(
                !fires(snippet, D, "E003"),
                "unexpected E003 for {snippet:?}"
            );
        }
        // Genuine over-arity (5 positional, max 4) still fires.
        // tclsh: `regsub a b c d e` → wrong # args.
        assert!(fires("regsub a b c d e", D, "E003"));
    }

    #[test]
    fn while_too_few_args_is_error() {
        let ds = analyser_diags("while {1}", D);
        assert!(ds.iter().any(|(_, _, s)| *s == Severity::Error));
    }

    #[test]
    fn for_too_few_args_is_error() {
        let ds = analyser_diags("for {set i 0} {$i < 10}", D);
        assert!(ds.iter().any(|(_, _, s)| *s == Severity::Error));
    }

    #[test]
    fn missing_subcommand_namespace_fires_e001() {
        // tclsh: bare `namespace` → wrong # args: should be "namespace subcommand …".
        let ds = analyser_diags("namespace", D);
        let errs: Vec<_> = ds
            .iter()
            .filter(|(_, _, s)| *s == Severity::Error)
            .collect();
        assert!(!errs.is_empty());
        assert!(
            errs.iter()
                .any(|(_, m, _)| m.to_lowercase().contains("subcommand"))
        );
    }

    #[test]
    fn unknown_command_not_arity_checked() {
        // Unknown user commands get no arity error (W123 only, not E00x).
        let ds = analyser_diags("mycommand a b c d e", D);
        assert!(!ds.iter().any(|(_, _, s)| *s == Severity::Error));
    }

    // --- unknown subcommand (W001), tclsh-observable `unknown … subcommand` ---

    #[test]
    fn unknown_subcommand_warns_w001() {
        // tclsh: `string bogus hello` → unknown or ambiguous subcommand "bogus".
        let ds = analyser_diags("string bogus hello", D);
        assert!(
            ds.iter()
                .any(|(c, m, _)| c == "W001" && m.contains("Unknown subcommand"))
        );
    }

    #[test]
    fn widget_creation_pathname_is_not_a_subcommand_no_w001() {
        // A widget-creation command's first word is a window pathname
        // (`.e`), not a subcommand — even though the creator carries an
        // instance-command `subcommands` table. It must never trip W001.
        for src in [
            "entry .e -textvariable v",
            "canvas .c -background white",
            "menu .m -tearoff 0",
            "text .t -width 40",
            "listbox .lb -listvariable items",
            "ttk::treeview .tv",
            "ttk::notebook .nb",
        ] {
            assert!(
                !fires(src, D, "W001"),
                "widget creation `{src}` must not fire W001",
            );
        }
        // A genuine mistyped subcommand (no leading `.`) still fires.
        assert!(fires("string bogus hello", D, "W001"));
    }

    #[test]
    fn package_prefer_is_a_real_subcommand_no_w001() {
        // `package prefer` is real in every supported dialect — tclsh returns
        // "stable". Regression #109: must not be flagged "Unknown subcommand".
        let ds = analyser_diags("package prefer", D);
        assert!(!ds.iter().any(|(_, m, _)| m.contains("Unknown subcommand")));
    }

    #[test]
    fn package_files_is_disabled_under_tcl86_per_tclsh() {
        // Under tcl8.6 `package files` is NOT a subcommand — tclsh8.6 errors
        //   `bad option "files": must be forget, ifneeded, names, …`
        // (tclsh9.0 added `files`; this test pins 8.6.)  Because `files` *is* a
        // real subcommand in 9.0, this is W002 ("disabled in the active dialect
        // profile"), not W001 ("Unknown subcommand") which is reserved for a
        // name that exists in no dialect (issue #812).
        assert!(fires("package files mypackage", D, "W002"));
        assert!(!fires("package files mypackage", D, "W001"));
    }

    #[test]
    fn zipfs_is_a_9_0_ensemble_gated_out_of_earlier_dialects() {
        // `zipfs` ships in Tcl 9.0 (TIP 430); the 8.x profiles have no such
        // command.  Under 9.0 a real subcommand resolves cleanly, and a bogus
        // one is W001 (unknown subcommand) — proving the subcommand set is
        // modelled.  Under 8.6 the whole command is dialect-gated, so it is
        // W002 ("disabled in the active dialect profile"), which points the
        // user at the version rather than a bare "unknown command".
        assert!(!fires("zipfs mount archive.zip /mnt", "tcl9.0", "W123"));
        assert!(!fires("zipfs mount archive.zip /mnt", "tcl9.0", "W001"));
        assert!(!fires("zipfs mount archive.zip /mnt", "tcl9.0", "W002"));
        assert!(fires("zipfs bogus x", "tcl9.0", "W001"));
        assert!(fires("zipfs mount archive.zip /mnt", "tcl8.6", "W002"));
    }

    #[test]
    fn namespace_which_command_probe_does_not_flag_unknown() {
        // `namespace which -command foo` is an existence PROBE — it returns ""
        // for an unknown command rather than failing, so probing a name no
        // command defines must NOT draw W123.  (Navigation to a command that
        // *does* exist still works; that's a separate reference.)
        let src = "namespace which -command no_such_command_xyz\n";
        assert!(
            !fires(src, D, "W123"),
            "namespace which probe must not W123: {:?}",
            analyser_diags(src, D),
        );
    }

    #[test]
    fn proc_shadowing_ensemble_command_suppresses_w001() {
        // tclsh8.6: `proc string {op args} {...}` completely replaces the
        // builtin `string` ensemble at the call site — `string reverse x`
        // dispatches to the user proc, not the registry subcommand set, so
        // it must not be flagged "Unknown subcommand".  See FP-STY-17.
        let src = "proc string {op args} { return $op }\nstring reverse x\n";
        assert!(!fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn ensemble_shadowing_does_not_hide_a_genuinely_unknown_subcommand() {
        // The proc-shadow suppression above is scoped to `string` only — a
        // different, unshadowed ensemble in the same file still fires.
        let src = "proc string {op args} { return $op }\ninfo bogus\n";
        assert!(fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn dynamically_mapped_dict_ensemble_subcommand_suppresses_w001() {
        // FP — regression for issue #923 idx 105 Part B: the real tcllib
        // `dicttool.tcl` idiom patches the `dict` ensemble's `-map` at
        // runtime to add `getnull` (`namespace ensemble configure dict -map
        // [dict replace [namespace ensemble configure dict -map] getnull
        // ::tcl::dict::getnull]`), which a static `SUBCOMMANDS` table can't
        // reflect. Must not fire "Unknown subcommand 'getnull' for 'dict'"
        // once the backing proc exists at `::tcl::dict::getnull` — this was
        // previously inconsistent with hover/definition, which already
        // resolved the same call site correctly.
        let src = "proc ::tcl::dict::getnull {dictionary args} {\n\
             if {[exists $dictionary {*}$args]} { get $dictionary {*}$args }\n\
             }\n\
             namespace ensemble configure dict -map \
             [dict replace [namespace ensemble configure dict -map] \
             getnull ::tcl::dict::getnull]\n\
             proc demo {} {\n\
             set clay [dict create a 1 b 2]\n\
             return [dict getnull $clay a]\n\
             }\n";
        assert!(!fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn dynamically_mapped_dict_ensemble_fix_is_not_overfit_to_getnull() {
        // FP — a second, differently-named sibling from the same real
        // tcllib idiom (`is_dict`), proving the fix isn't hardcoded to one
        // subcommand name. This block never calls bare `exists`/`get`, so it
        // exercises Part B in isolation from Part A.
        let src = "proc ::tcl::dict::is_dict {d} {\n\
             if {[catch {dict size $d} err]} { return 0 }\n\
             return 1\n\
             }\n\
             namespace ensemble configure dict -map \
             [dict replace [namespace ensemble configure dict -map] \
             is_dict ::tcl::dict::is_dict]\n\
             dict is_dict [dict create a 1]\n";
        assert!(!fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn genuinely_unknown_dict_subcommand_still_fires_w001() {
        // TP — regression guard: a real, non-existent `dict` subcommand
        // (no proc named `zzzznotreal` anywhere, no ensemble patch) must
        // still fire — proves `dynamic_ensemble_subcommand_known` doesn't
        // over-suppress.
        let src = "proc demo {} {\n\
             set d [dict create a 1 b 2]\n\
             return [dict zzzznotreal $d a]\n\
             }\n";
        assert!(fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn dict_proc_at_conventional_location_without_ensemble_patch_is_an_accepted_false_negative() {
        // FN — deliberately ACCEPTED gap (idx 105 Part B's primary/simple
        // design, chosen over the more precise "observed an actual
        // `namespace ensemble configure -map` call" variant): a proc
        // defined at `::tcl::dict::<name>` is treated as "this ensemble
        // subcommand is known" even when the ensemble's `-map` was never
        // actually reconfigured to include it. Real tclsh would still error
        // `unknown or ambiguous subcommand "stray"` here. If this gap is
        // ever closed (tracking whether `-map` was truly reconfigured, see
        // idx 105's research plan for the harder variant), this test's
        // assertion flips and documents the improvement.
        let src = "proc ::tcl::dict::stray {d} { return $d }\n\
             dict stray [dict create a 1]\n";
        assert!(
            !fires(src, D, "W001"),
            "documents the accepted FN gap; got {:?}",
            analyser_diags(src, D),
        );
    }

    #[test]
    fn namespace_ensemble_configure_on_tk_suppresses_w001_for_systray_and_sysnotify() {
        // FP — regression for issue #923 idx 84: the real
        // `tk/library/systray.tcl` (and `print.tcl`, `fileicon.tcl`,
        // `accessibility.tcl`) idiom splices `systray`/`sysnotify` into the
        // *pre-existing, registry-builtin* `tk` ensemble via `namespace
        // ensemble configure tk -map [dict merge [namespace ensemble
        // configure tk -map] {systray ::tk::systray sysnotify
        // ::tk::sysnotify::sysnotify}]` — a `CONFIGURE`, not `CREATE`, on an
        // ensemble this file never itself created. tclsh9.0/8.6 both
        // confirm `tk systray create`/`tk sysnotify ...` are correct,
        // documented calls; must not fire "Unknown subcommand".
        let src = "proc ::tk::systray {args} {}\n\
             proc ::tk::sysnotify::sysnotify {a b} {}\n\
             namespace ensemble configure tk -map \
             [dict merge [namespace ensemble configure tk -map] \
             {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]\n\
             tk systray create -image book\n\
             tk sysnotify Alert message\n";
        assert!(!fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn namespace_ensemble_configure_genuinely_unknown_tk_subcommand_still_fires_w001() {
        // TP — regression guard: a real, non-existent `tk` subcommand (no
        // splice recorded for it) must still fire — proves
        // `statically_mapped_ensemble_subcommand_known` doesn't over-suppress
        // the whole `tk` ensemble once *any* subcommand has been spliced in.
        let src = "namespace ensemble configure tk -map \
             [dict merge [namespace ensemble configure tk -map] \
             {systray ::tk::systray}]\n\
             tk zzznotreal\n";
        assert!(fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn ensemble_without_implementation_namespace_is_unaffected_by_dynamic_map_check() {
        // TN — regression guard: `dynamic_ensemble_subcommand_known` is
        // inert for every ensemble whose `CommandSpec` doesn't set
        // `implementation_namespace` (only `dict`, currently) — a
        // same-named proc existing elsewhere must not suppress an unrelated
        // ensemble's genuinely unknown subcommand.
        let src = "proc bogus {} {}\nstring bogus hello\n";
        assert!(fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn error_diagnostics_have_a_span() {
        // The error anchors at the start of the script. The diagnostic carries
        // a byte `span`; assert it starts at offset 0 (the `set` token).
        let r = Analyser::new().analyse("set", D);
        assert!(!r.diagnostics.is_empty());
        assert_eq!(r.diagnostics[0].span.start(), 0);
    }

    #[test]
    fn multiline_diagnostic_anchors_on_offending_line() {
        // `break extra` sits on line 1; its span starts after the first line's
        // newline. We check the byte offset lands within the second line.
        let src = "set x 1\nbreak extra\nset y 2";
        let r = Analyser::new().analyse(src, D);
        let err = r
            .diagnostics
            .iter()
            .find(|d| d.severity == Severity::Error)
            .expect("an error");
        let line_start = src.find("break").unwrap();
        assert!(
            (err.span.start() as usize) >= line_start,
            "error span should be on the `break` line"
        );
    }

    // --- read-before-set (W210), tclsh `can't read "x"` ---

    #[test]
    fn read_before_set_warns_w210() {
        // tclsh8.6/9.0: `puts $x` → can't read "x": no such variable.
        assert_eq!(count("puts $x", D, "W210"), 1);
        let ds = analyser_diags("puts $x", D);
        assert!(
            ds.iter()
                .any(|(c, _, s)| c == "W210" && *s == Severity::Warning)
        );
    }

    #[test]
    fn read_before_set_in_expr_warns_w210() {
        // tclsh: `if {$x > 0} …` with x unset → can't read "x".
        assert_eq!(count("if {$x > 0} { puts yes }", D, "W210"), 1);
    }

    #[test]
    fn read_before_set_cleared_after_assignment() {
        assert!(!fires("set x 1\nputs $x", D, "W210"));
    }

    #[test]
    fn array_read_uses_base_variable_no_w210() {
        assert!(!fires("set arr(key) 1\nputs $arr(key)", D, "W210"));
    }

    // --- output-var writers suppress W210 on the written var ---
    // (regexp/scan/lassign write their target vars; reading them is safe.)

    #[test]
    fn regexp_capture_vars_not_read_before_set() {
        // The `email` read still fires (it is genuinely unset), but the capture
        // targets `user`/`domain` do not.
        assert_eq!(
            w210_for(
                "regexp {^(\\w+)@(\\w+)$} $email -> user domain\nputs $user",
                "user"
            ),
            0
        );
    }

    #[test]
    fn regexp_match_var_not_read_before_set() {
        assert_eq!(
            w210_for("regexp {\\d+} $text match\nputs $match", "match"),
            0
        );
    }

    #[test]
    fn scan_capture_vars_not_read_before_set() {
        assert_eq!(
            w210_for("scan \"42 hello\" \"%d %s\" num word\nputs $num", "num"),
            0
        );
    }

    #[test]
    fn lassign_vars_not_read_before_set() {
        assert_eq!(w210_for("lassign {a b c} x y z\nputs \"$x $y $z\"", "x"), 0);
    }

    // `regsub … result` writes its trailing `result` variable (recognised via
    // the registry's `VarWrite` arg-role, like `set`/`lassign`/`scan`), so a
    // later `$result` read is NOT read-before-set.
    // The `$text` argument is still unset and fires its own W210 (tclsh would
    // error on that first at runtime).
    #[test]
    fn regsub_result_var_suppresses_w210_at_top_level() {
        assert_eq!(
            w210_for("regsub {old} $text new result\nputs $result", "result"),
            0
        );
        // The unset input `$text` still reports read-before-set.
        assert_eq!(
            w210_for("regsub {old} $text new result\nputs $result", "text"),
            1
        );
    }

    // --- unused / dead-store / paste hints (pure static heuristics) ---

    #[test]
    fn unused_assigned_variable_hint_w211() {
        // Heuristic (no direct tclsh analogue): `set x 1` never read.
        let ds = analyser_diags("proc foo {} { set x 1 }", D);
        assert!(
            ds.iter()
                .any(|(c, _, s)| c == "W211" && *s == Severity::Hint)
        );
    }

    #[test]
    fn used_variable_no_unused_hint() {
        assert!(!fires("proc foo {} { set x 1; puts $x }", D, "W211"));
    }

    #[test]
    fn dead_assignment_detected_w220() {
        // Heuristic: `set x 1; set x 2; puts $x` — first store is dead.
        let ds = analyser_diags("proc foo {} { set x 1; set x 2; puts $x }", D);
        let dead: Vec<_> = ds.iter().filter(|(c, _, _)| c == "W220").collect();
        assert_eq!(dead.len(), 1);
        assert!(dead[0].1.contains('x'));
    }

    #[test]
    fn paste_error_hint_for_duplicate_static_assignment_h300() {
        // Heuristic: repeated assignment of the *same* literal value.
        let ds = analyser_diags("proc foo {} { set x 0; set x 0; puts $x }", D);
        let h: Vec<_> = ds.iter().filter(|(c, _, _)| c == "H300").collect();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].2, Severity::Hint);
        assert!(h[0].1.contains('x'));
    }

    #[test]
    fn paste_error_not_emitted_when_value_changes_or_dynamic() {
        assert!(!fires(
            "proc foo {} { set x 0; set x 1; puts $x }",
            D,
            "H300"
        ));
        assert!(!fires(
            "proc foo {} { set x $y; set x $y; puts $x }",
            D,
            "H300"
        ));
    }

    // --- constant-branch family (I230) ---

    #[test]
    fn constant_if_unreachable_branch_i230() {
        assert!(fires("if {1} { set x 1 } else { set y 2 }", D, "I230"));
    }

    #[test]
    fn constant_string_eq_and_double_equals_fold_i230() {
        // With x provably "foo", `$x == "foo"` / `$x eq "foo"` is always true →
        // alternate branch unreachable. tclsh: expr {"foo" == "foo"} → 1.
        for op in ["==", "eq"] {
            let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}");
            assert_eq!(count(&src, D, "I230"), 1, "expected I230 for {op:?}");
        }
    }

    #[test]
    fn constant_string_ne_and_bang_equals_fold_i230() {
        // `$x != "foo"` with x=="foo" is always false → then-branch unreachable.
        for op in ["!=", "ne"] {
            let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}");
            assert_eq!(count(&src, D, "I230"), 1, "expected I230 for {op:?}");
        }
    }

    #[test]
    fn infinite_loop_idiom_not_flagged_i230() {
        // `while 1` / `for … 1 …` are intentional infinite loops (exit via
        // break/return); a constant-true loop condition must not be I230.
        for src in [
            "proc f {} { while 1 { if {[g]} break } }",
            "proc f {} { while true { if {[g]} break } }",
            "proc f {} { for {set i 0} 1 {incr i} { if {$i > 9} break } }",
        ] {
            assert!(!fires(src, D, "I230"), "unexpected I230 for {src:?}");
        }
    }

    #[test]
    fn dead_while_zero_still_flagged() {
        // A constant-*false* loop condition means the body never runs.
        // This is reported as W240 (loop-never-executes) rather than I230;
        // the dead-loop fact is still surfaced.
        assert!(fires("proc f {} { while 0 { puts dead } }", D, "W240"));
    }

    // NOTE — no I231 for a constant switch: `switch 1 {1 {…} …}` does not fire
    // I231. The analyser + run_all_checks surface does not fold a constant
    // switch subject into an unreachable-arm diagnostic.
}

// ===========================================================================
// `set` dual-shape role resolution, observed through behaviour.
// ===========================================================================
mod set_dual_shape {
    use super::*;

    #[test]
    fn set_two_arg_is_write_one_arg_is_read() {
        // 2-arg `set x 1` is a write: defines `x`, no W210.
        assert!(!fires("set x 1\nputs $x", D, "W210"));
        assert!(
            Analyser::new()
                .analyse("set x 1", D)
                .global_scope
                .variables
                .contains_key("x")
        );
        // 1-arg `set x` is a read: does NOT define `x`.
        assert!(
            !Analyser::new()
                .analyse("set x", D)
                .global_scope
                .variables
                .contains_key("x")
        );
    }
}

// ===========================================================================
// `when` body recursion is dialect-gated (iRules-only builtin).
// ===========================================================================
mod when_dialect_gating {
    use super::*;

    #[test]
    fn when_body_not_analysed_under_plain_tcl() {
        // Under tcl8.6 `when` is an unknown command; its braced arg is opaque
        // data, NOT a handler script — so the body command `boguscmd` must not
        // get its own W123, and no spurious W210 on the body read.
        let src = "when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n";
        let ds = analyser_diags(src, D);
        assert!(
            !ds.iter()
                .any(|(c, m, _)| c == "W123" && m.contains("boguscmd")),
            "when body must not be analysed under tcl8.6"
        );
        assert!(
            !ds.iter().any(|(c, _, _)| c == "W210"),
            "no spurious W210 in opaque body"
        );
    }

    #[test]
    fn when_body_analysed_under_irules() {
        // Under f5-irules `when` IS a handler; the body is analysed and the
        // bogus inner command earns its own W123.
        let src = "when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n";
        let ds = analyser_diags(src, "f5-irules");
        assert_eq!(
            ds.iter()
                .filter(|(c, m, _)| c == "W123" && m.contains("boguscmd"))
                .count(),
            1,
            "when body IS analysed under f5-irules"
        );
    }
}

// ===========================================================================
// Bodies of if/while/for/foreach/dict-for are walked.
// ===========================================================================
mod control_flow {
    use super::*;

    fn vars(src: &str) -> std::collections::HashSet<String> {
        Analyser::new()
            .analyse(src, D)
            .global_scope
            .variables
            .keys()
            .cloned()
            .collect()
    }

    #[test]
    fn if_while_for_bodies_analysed() {
        assert!(vars("if {1} { set x 42 }").contains("x"));
        assert!(vars("while {1} { set x 42; break }").contains("x"));
        let v = vars("for {set i 0} {$i < 10} {incr i} { set x $i }");
        assert!(v.contains("i") && v.contains("x"));
    }

    #[test]
    fn if_expr_records_var_reference() {
        let r = Analyser::new().analyse("set x 1\nif {$x > 0} { set y 1 }", D);
        let xv = r.global_scope.variables.get("x").expect("x defined");
        assert!(!xv.references.is_empty());
    }

    #[test]
    fn foreach_defines_loop_var() {
        assert!(vars("foreach item {a b c} { puts $item }").contains("item"));
    }

    #[test]
    fn foreach_multi_var_each_defined_with_distinct_ranges() {
        let r = Analyser::new().analyse("foreach {a b c} {1 2 3} { puts $a }", D);
        for v in ["a", "b", "c"] {
            assert!(r.global_scope.variables.contains_key(v), "{v} missing");
        }
        // Each binding gets its own span, not the whole varList token's.
        let ra = r.global_scope.variables["a"].definition_span;
        let rb = r.global_scope.variables["b"].definition_span;
        let rc = r.global_scope.variables["c"].definition_span;
        assert_ne!(ra.start(), rb.start());
        assert_ne!(rb.start(), rc.start());
    }

    #[test]
    fn if_elseif_else_all_bodies_analysed() {
        let v = vars("if {$x} { set a 1 } elseif {$y} { set b 2 } else { set c 3 }");
        assert!(v.contains("a") && v.contains("b") && v.contains("c"));
    }

    #[test]
    fn dict_for_body_analysed() {
        assert!(vars("dict for {k v} $d { set seen 1 }").contains("seen"));
    }

    #[test]
    fn command_subst_inside_expr_records_outer_var() {
        // The analyser records the outer `n` but does not hoist the nested
        // `[set y 1]` write (inside the `[expr {…}]` command-substitution) into
        // `global_scope.variables`.
        let v = vars("set n [expr {[set y 1] + 2}]");
        assert!(v.contains("n"), "outer assignment `n` is recorded");
    }

    #[test]
    fn nested_proc_and_if_records_proc() {
        let src = "proc check {x} {\n    if {== $x 0} {\n        set result zero\n    } else {\n        set result nonzero\n    }\n    return $result\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.global_scope.procs.contains_key("check"));
    }

    #[test]
    fn tcloo_method_body_in_method_scope() {
        let src =
            "oo::class create Dog {\n    method bark {} {\n        set message woof\n    }\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.all_classes.contains_key("::Dog"));
        // The method scope is named `::Dog::bark`.
        let scope = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name.ends_with("Dog::bark"))
            .expect("bark method scope");
        assert!(scope.variables.contains_key("message"));
    }
}

// ===========================================================================
// Recorded regex literals.
// ===========================================================================
mod regex_patterns {
    use super::*;

    fn pats(src: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(src, D)
            .regex_patterns
            .iter()
            .map(|p| (p.pattern.clone(), p.command.clone()))
            .collect()
    }

    #[test]
    fn regexp_simple_records_pattern_and_command() {
        let p = pats("regexp {^[a-z]+$} $str");
        assert_eq!(p, [("^[a-z]+$".to_string(), "regexp".to_string())]);
    }

    #[test]
    fn regexp_with_options_and_terminator_and_start() {
        assert_eq!(pats("regexp -nocase -expanded {\\d+} $str")[0].0, "\\d+");
        assert_eq!(pats("regexp -nocase -- {^test} $str")[0].0, "^test");
        assert_eq!(pats("regexp -start 5 {pattern} $str")[0].0, "pattern");
    }

    #[test]
    fn regsub_simple_and_with_options() {
        let p = pats("regsub {\\d+} $str replacement result");
        assert_eq!(p, [("\\d+".to_string(), "regsub".to_string())]);
        assert_eq!(
            pats("regsub -all -nocase {foo} $str bar result")[0].0,
            "foo"
        );
    }

    #[test]
    fn switch_regexp_braced_and_inline_record_arms() {
        for src in [
            "switch -regexp $x { {^a} {puts a} {^b} {puts b} }",
            "switch -regexp $x {^a} {puts a} {^b} {puts b}",
        ] {
            let p = pats(src);
            assert_eq!(p.len(), 2, "for {src:?}");
            let pset: Vec<_> = p.iter().map(|(pat, _)| pat.as_str()).collect();
            assert!(pset.contains(&"^a") && pset.contains(&"^b"));
            assert!(p.iter().all(|(_, c)| c == "switch"));
        }
    }

    #[test]
    fn switch_regexp_excludes_default_arm() {
        let p = pats("switch -regexp $x { {^a} {puts a} default {puts other} }");
        assert_eq!(p, [("^a".to_string(), "switch".to_string())]);
    }

    #[test]
    fn switch_glob_exact_default_record_no_regex() {
        assert!(pats("switch -glob $x { a* {puts a} b* {puts b} }").is_empty());
        assert!(pats("switch -exact $x { hello {puts hi} }").is_empty());
        assert!(pats("switch $x { hello {puts hi} }").is_empty());
    }

    #[test]
    fn no_regex_in_other_commands() {
        assert!(pats("set x 1\nputs hello\nif {$x > 0} { puts yes }").is_empty());
    }

    #[test]
    fn multiple_regex_commands_in_one_file() {
        let p = pats("regexp {^a} $x\nregsub {^b} $y c result");
        assert_eq!(p.len(), 2);
        let cmds: std::collections::HashSet<_> = p.iter().map(|(_, c)| c.clone()).collect();
        assert_eq!(
            cmds,
            ["regexp".to_string(), "regsub".to_string()]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn regexp_inside_proc_recorded() {
        let p = pats("proc check {s} { regexp {^\\d+$} $s }");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].1, "regexp");
    }

    // --- variable propagation ---

    #[test]
    fn set_then_regexp_and_regsub_propagate_constant_pattern() {
        // `set pat {re}; regexp $pat …` records two patterns (def + use site).
        let p = pats("set pat {^\\d+$}\nregexp $pat $str");
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|(pat, _)| pat == "^\\d+$"));
        let p2 = pats("set pat {foo}\nregsub $pat $str bar result");
        assert_eq!(p2.len(), 2);
        assert!(p2.iter().all(|(pat, _)| pat == "foo"));
    }

    #[test]
    fn set_then_switch_regexp_propagates() {
        let p = pats("set pat {^hello}\nswitch -regexp $x $pat {puts matched}");
        assert!(p.iter().any(|(pat, _)| pat == "^hello"));
    }

    #[test]
    fn dynamic_or_interpolated_pattern_not_recorded() {
        assert!(pats("set pat $dynamic_value\nregexp $pat $str").is_empty());
        assert!(pats("set pat \"^$prefix\"\nregexp $pat $str").is_empty());
        // Reassigned to non-constant loses tracking.
        assert!(pats("set pat {^\\d+}\nset pat $other\nregexp $pat $str").is_empty());
    }

    #[test]
    fn variable_pattern_in_proc_scope() {
        let p = pats("proc check {s} { set pat {^\\w+$}; regexp $pat $s }");
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|(pat, _)| pat == "^\\w+$"));
    }

    #[test]
    fn literal_and_variable_patterns_mixed() {
        let p = pats("set pat {^a}\nregexp $pat $str\nregexp {^b} $str2");
        let pset: Vec<_> = p.iter().map(|(pat, _)| pat.as_str()).collect();
        assert!(pset.contains(&"^a") && pset.contains(&"^b"));
    }
}

// ===========================================================================
// package require / provide records.
// ===========================================================================
mod package_require {
    use super::*;

    #[test]
    fn simple_require_no_version() {
        let r = Analyser::new().analyse("package require http", D);
        assert_eq!(r.package_requires.len(), 1);
        assert_eq!(r.package_requires[0].name, "http");
        assert!(r.package_requires[0].version.is_none());
    }

    #[test]
    fn require_with_version_and_exact() {
        for src in [
            "package require http 2.9",
            "package require -exact http 2.9",
        ] {
            let r = Analyser::new().analyse(src, D);
            assert_eq!(r.package_requires.len(), 1, "for {src:?}");
            assert_eq!(r.package_requires[0].name, "http");
            assert_eq!(r.package_requires[0].version.as_deref(), Some("2.9"));
        }
    }

    #[test]
    fn multiple_requires() {
        let r = Analyser::new().analyse("package require http\npackage require tls", D);
        assert_eq!(r.package_requires.len(), 2);
        let names: std::collections::HashSet<_> =
            r.package_requires.iter().map(|p| p.name.clone()).collect();
        assert_eq!(
            names,
            ["http".to_string(), "tls".to_string()]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn provide_and_unrelated_not_recorded() {
        assert!(
            Analyser::new()
                .analyse("package provide mylib 1.0", D)
                .package_requires
                .is_empty()
        );
        assert!(
            Analyser::new()
                .analyse("set x 42", D)
                .package_requires
                .is_empty()
        );
    }
}

// ===========================================================================
// Unused proc parameters (W214) — pure static heuristic.
// ===========================================================================
mod unused_proc_parameters {
    use super::*;

    fn w214(src: &str) -> Vec<String> {
        analyser_diags(src, D)
            .into_iter()
            .filter(|(c, _, _)| c == "W214")
            .map(|(_, m, _)| m)
            .collect()
    }

    #[test]
    fn unused_param_detected() {
        let w = w214("proc foo {x y} { puts $x }");
        assert_eq!(w.len(), 1);
        assert!(w[0].contains('y'));
        // Severity is Hint.
        assert!(
            analyser_diags("proc foo {x y} { puts $x }", D)
                .iter()
                .any(|(c, _, s)| c == "W214" && *s == Severity::Hint)
        );
    }

    #[test]
    fn all_params_used_no_warning() {
        assert!(w214("proc foo {a b} { puts $a; puts $b }").is_empty());
    }

    #[test]
    fn args_param_not_flagged() {
        // `args` is the variadic catch-all and is never "unused".
        assert!(w214("proc foo {args} { puts hello }").is_empty());
    }

    #[test]
    fn underscore_prefixed_param_is_still_flagged_rust_behaviour() {
        // A leading `_` is not treated as a deliberate "unused" marker —
        // `_unused` still fires W214.
        let w = w214("proc foo {_unused x} { puts $x }");
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("_unused"));
    }

    #[test]
    fn multiple_unused_params_each_flagged() {
        assert_eq!(w214("proc foo {a b c} { puts hello }").len(), 3);
    }

    #[test]
    fn param_used_in_return_or_branch_condition() {
        assert!(w214("proc foo {x} { return $x }").is_empty());
        assert!(w214("proc foo {x} { if {$x > 0} { puts yes } }").is_empty());
    }

    #[test]
    fn param_used_in_expr_alias_counts_as_read() {
        // interp alias for expr — refs inside the aliased expr arg are reads (#42).
        let src = "interp alias {} = {} expr\nproc foo {x y} {\n    set result [= {$x + $y}]\n    return $result\n}\n";
        assert!(w214(src).is_empty());
    }

    #[test]
    fn param_used_in_dict_for_and_dict_map_bodies() {
        // dict for/map bodies are lowered into real CFG blocks in the analysis
        // build (#833), so a param read nested in the body is a first-class SSA
        // use — the deep body scan / text fallback (#236) is now belt-and-braces.
        let used = [
            "proc f {but} {\n    dict for {k v} $d {\n        if {$but ne \"\"} { puts $but }\n    }\n}\n",
            "proc f {scale} {\n    dict map {k v} $d { expr {$v * $scale} }\n}\n",
            "proc foo {targetKey} {\n    set d [dict create]\n    dict for {k v} $d {\n        if {[dict exists $v $targetKey]} { puts hi }\n    }\n}\n",
        ];
        for src in used {
            assert!(w214(src).is_empty(), "unexpected W214 for {src:?}");
        }
    }

    #[test]
    fn unused_param_alongside_used_in_dict_for_still_flagged() {
        let src = "proc f {{count 0} other} {\n    dict for {k v} $d {\n        if {$count>0} { incr count }\n    }\n}\n";
        let w = w214(src);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("other"));
    }

    #[test]
    fn param_read_inside_catch_body() {
        assert!(w214("proc f {x} {\n    catch { puts $x }\n}\n").is_empty());
    }

    #[test]
    fn param_used_only_by_dict_with_or_update() {
        // dict with / dict update mark the dict var VAR_READ+VAR_WRITE; the read
        // must survive the defs filter (#307).
        assert!(w214("proc f {pdata} {\n    dict with pdata {}\n}\n").is_empty());
        assert!(w214("proc f {d} {\n    dict update d k v {}\n}\n").is_empty());
    }

    #[test]
    fn pure_write_to_param_inside_opaque_body_still_flags() {
        let src = "proc f {x} {\n    dict for {k v} $d { set x 1 }\n}\n";
        let w = w214(src);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains('x'));
    }

    #[test]
    fn unrelated_namespaced_for_does_not_get_dict_body_fallback() {
        // Only ::tcl::dict::for|map get the synthetic body-arg fallback.
        let src = "proc f {x} {\n    ::my::for a b { set x 1 }\n}\n";
        let w = w214(src);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains('x'));
    }

    #[test]
    fn return_expr_and_cmd_subst_do_not_false_flag() {
        for src in [
            "proc foo {x} { return [expr {$x + 1}] }",
            "proc foo {x} { return [string length $x] }",
        ] {
            assert!(w214(src).is_empty(), "unexpected W214 for {src:?}");
        }
        // …but a genuinely-unused param still warns alongside.
        let w = w214("proc foo {x y} { return [expr {$x + 1}] }");
        assert_eq!(w.len(), 1);
        assert!(w[0].contains('y'));
    }

    #[test]
    fn dict_for_braced_data_word_use_divergence() {
        // `{$unused}` is a braced literal (braces inhibit substitution), so it
        // would not count as a use. The deep-body scan over the opaque
        // ::tcl::dict::for barrier does NOT flag `unused` here (it does not
        // exclude the braced data word), so no W214 fires.
        let src = "proc foo {used unused} {\n    set d [dict create]\n    dict for {k v} $d {\n        set msg {$unused}\n        puts $used\n    }\n}\n";
        assert!(
            w214(src).is_empty(),
            "Rust does not flag `unused` in this shape"
        );
    }

    // NOTE — trace-callback W214 (see `trace_callbacks` module): the analyser +
    // run_all_checks surface does NOT special-case trace callbacks, so W214
    // fires on a `trace add variable … watcher` callback's `name1 name2 op`
    // params. The control test `unrelated_proc_unused_param_still_flagged`
    // (which fires either way) covers this.
}

// ===========================================================================
// Trace callbacks.
// ===========================================================================
mod trace_callbacks {
    use super::*;

    #[test]
    fn unrelated_proc_unused_param_still_flagged() {
        // Whatever the trace-callback handling, an *unrelated* proc's unused
        // param must still fire W214.
        let src = "proc watcher {name1 name2 op} { puts hello }\nproc other {a b} { puts $a }\ntrace add variable x write watcher\n";
        // Message shape: `Parameter 'b' of proc '::other' is unused`.
        let w214: Vec<_> = analyser_diags(src, D)
            .into_iter()
            .filter(|(c, m, _)| c == "W214" && m.contains("'b'") && m.contains("::other"))
            .collect();
        assert_eq!(
            w214.len(),
            1,
            "unrelated `other`'s unused `b` must fire W214"
        );
    }
}

// ===========================================================================
// interp alias records in `command_aliases`.
// ===========================================================================
mod interp_alias {
    use super::*;

    fn alias(src: &str, name: &str) -> Option<(String, Vec<String>)> {
        Analyser::new()
            .analyse(src, D)
            .command_aliases
            .get(name)
            .map(|a| (a.target.clone(), a.extras.clone()))
    }

    #[test]
    fn alias_recorded_with_target() {
        assert_eq!(
            alias("interp alias {} = {} expr", "::="),
            Some(("expr".to_string(), vec![]))
        );
    }

    #[test]
    fn alias_with_prepended_args() {
        assert_eq!(
            alias("interp alias {} myput {} puts stdout", "::myput"),
            Some(("puts".to_string(), vec!["stdout".to_string()]))
        );
    }

    #[test]
    fn alias_non_current_interp_not_recorded() {
        assert!(alias("interp alias child = {} expr", "::=").is_none());
    }

    #[test]
    fn alias_qualified_name_normalised() {
        assert!(
            Analyser::new()
                .analyse("interp alias {} ::myexpr {} expr", D)
                .command_aliases
                .contains_key("::myexpr")
        );
    }

    #[test]
    fn alias_chain_not_resolved_transitively() {
        let src = "interp alias {} a {} b\ninterp alias {} b {} expr\n";
        assert_eq!(alias(src, "::a"), Some(("b".to_string(), vec![])));
        assert_eq!(alias(src, "::b"), Some(("expr".to_string(), vec![])));
    }

    #[test]
    fn alias_dynamic_name_not_recorded() {
        assert!(alias("set n \"=\"\ninterp alias {} $n {} expr", "::=").is_none());
    }

    #[test]
    fn alias_redefinition_overwrites() {
        let src = "interp alias {} myop {} expr\ninterp alias {} myop {} puts\n";
        assert_eq!(alias(src, "::myop"), Some(("puts".to_string(), vec![])));
    }

    #[test]
    fn alias_too_few_args_no_crash() {
        assert!(alias("interp alias {} = {}", "::=").is_none());
    }

    #[test]
    fn alias_target_in_child_interp_is_domain_qualified_945() {
        // A cross-domain alias deliberately crosses interpreter domains
        // (issue #945 fault 8): the current-interp `myexpr` runs the
        // *child's* `expr`, so the recorded target is qualified into the
        // child's `@interp@` domain — never treated as the parent's own
        // `expr` (the old model skipped the record entirely, losing the
        // cross-domain link).
        assert_eq!(
            alias("interp alias {} myexpr child expr", "::myexpr"),
            Some(("::@interp@child::expr".to_string(), vec![])),
        );
    }

    #[test]
    fn alias_query_form_no_crash() {
        assert!(alias("interp alias {} myexpr", "::myexpr").is_none());
    }

    #[test]
    fn alias_for_expr_and_body_taking_commands_no_w214() {
        // Aliases for expr / eval / foreach route the role analysis through the
        // alias target, so params used via the alias are not W214.
        let cases = [
            "interp alias {} = {} expr\nproc foo {x y} {\n    set result [= {$x + $y}]\n    return $result\n}\n",
            "interp alias {} myeval {} eval\nproc foo {x} {\n    myeval { set y 1 }\n    return $x\n}\n",
            "interp alias {} myforeach {} foreach\nproc foo {items} {\n    myforeach item $items { puts $item }\n    return $item\n}\n",
        ];
        for src in cases {
            assert!(!fires(src, D, "W214"), "unexpected W214 for {src:?}");
        }
    }

    #[test]
    fn alias_resolved_from_namespace_and_global_fallback_no_w214() {
        let cases = [
            "interp alias {} ::math::= {} expr\nnamespace eval math {\n    proc calc {x y} {\n        set result [= {$x + $y}]\n        return $result\n    }\n}\n",
            "interp alias {} = {} expr\nnamespace eval utils {\n    proc calc {x y} {\n        set result [= {$x + $y}]\n        return $result\n    }\n}\n",
            "interp alias {} = {} expr\nproc calc {x y} {\n    set result [::= {$x + $y}]\n    return $result\n}\n",
        ];
        for src in cases {
            assert!(!fires(src, D, "W214"), "unexpected W214 for {src:?}");
        }
    }

    #[test]
    fn alias_standalone_expr_call_has_no_error() {
        let src = "interp alias {} = {} expr\n= {1 + 2}\n";
        assert!(
            !analyser_diags(src, D)
                .iter()
                .any(|(_, _, s)| *s == Severity::Error)
        );
    }

    #[test]
    fn real_world_file_shortcuts_record_extras() {
        let src = "interp alias {} cp {} file copy -force\ninterp alias {} mkdir {} file mkdir\ninterp alias {} rm {} file delete -force\n";
        assert_eq!(
            alias(src, "::cp"),
            Some((
                "file".to_string(),
                vec!["copy".to_string(), "-force".to_string()]
            ))
        );
        assert_eq!(
            alias(src, "::mkdir"),
            Some(("file".to_string(), vec!["mkdir".to_string()]))
        );
        assert_eq!(
            alias(src, "::rm"),
            Some((
                "file".to_string(),
                vec!["delete".to_string(), "-force".to_string()]
            ))
        );
    }

    #[test]
    fn real_world_safe_and_cross_interp_aliases_945() {
        // A dynamic source path bound by a tracked `set i [interp create
        // ...]` (issue #923 idx 9) resolves through that binding rather
        // than aborting: `add` is defined *inside* `$i`'s domain (calling
        // `::api::add` back in the parent), so it homes under `$i`'s
        // synthetic `@interp@@autoname@<offset>` domain — never under the
        // bare `::add` a plain top-level alias would use.
        let safe = "set i [interp create -safe]\ninterp alias $i add {} ::api::add\nproc ::api::add {a b} {\n    return [expr {$a + $b}]\n}\n";
        assert!(alias(safe, "::add").is_none());
        assert_eq!(
            alias(safe, "::@interp@@autoname@6::add"),
            Some(("::api::add".to_string(), vec![])),
        );
        // A literal parent-side alias into a live child is tracked with a
        // domain-qualified target (issue #945 fault 8): `localGreet` runs
        // the child's `greet`, so navigation follows the alias link into
        // the child's `@interp@` domain, where `interp eval` homed the
        // proc's definition.
        let cross = "interp create child\ninterp eval child { proc greet {} { return hello } }\ninterp alias {} localGreet child greet\nlocalGreet\ninterp delete child\n";
        assert_eq!(
            alias(cross, "::localGreet"),
            Some(("::@interp@child::greet".to_string(), vec![])),
        );
    }

    #[test]
    fn real_world_dynamic_html_tag_aliases_not_tracked() {
        let src = "proc html_tag {tag args} { return \"<$tag>$args</$tag>\" }\nforeach tag {h1 h2 h3 p div span} {\n    interp alias {} $tag {} html_tag $tag\n}\n";
        assert!(alias(src, "::h1").is_none());
    }

    #[test]
    fn alias_deletion_form_keeps_prior_definition() {
        // `interp alias {} name {}` (4 args) deletes; not recorded as a new
        // alias, and the prior definition survives (we don't track deletions).
        let src = "interp alias {} myalias {} list\ninterp alias {} myalias {}\n";
        assert_eq!(alias(src, "::myalias"), Some(("list".to_string(), vec![])));
    }

    #[test]
    fn return_expr_alias_and_braced_no_w214() {
        let cases = [
            "proc foo {x} { return [expr {$x + 1}] }",
            "interp alias {} = {} expr\nproc pdPsCalc {width length} {\n    return [= {2*$width+2*$length}]\n}\n",
        ];
        for src in cases {
            assert!(!fires(src, D, "W214"), "unexpected W214 for {src:?}");
        }
    }

    #[test]
    fn issue_42_examples_no_w214() {
        let original = "interp alias {} = {} expr\nproc moveKeyInDict {dict key relPos} {\n    set keys [dkeys $dict]\n    set index [lsearch -exact $keys $key]\n    set newIndex [= {$index+$relPos}]\n    set prevKey [@ $keys $newIndex]\n    set newKeysOrder [lreplace [lreplace $keys $index $index $prevKey] $newIndex $newIndex $key]\n    foreach key $newKeysOrder {\n        dict append newDict $key [dget $dict $key]\n    }\n    return $newDict\n}\n";
        assert!(!fires(original, D, "W214"));
        let reopened = "interp alias {} = {} expr\nset scriptPath [file dirname [file normalize [info script]]]\nproc pdPsCalc {width length} {\n    return [= {2*$width+2*$length}]\n}\nset vSupply 2.0\nset inpFreq 850e6\n";
        assert!(!fires(reopened, D, "W214"));
    }
}

// ===========================================================================
// interp_value_flow — issue #923 idx 9: `set VAR [interp create ...]`
// binds VAR to an interpreter-domain key, so a later dynamic `$VAR` operand
// to `interp alias` / `interp eval` / the handle's own object command can
// resolve through the tracked binding instead of abstaining outright.
// ===========================================================================
mod interp_value_flow {
    use super::*;

    fn command_alias_targets(src: &str) -> std::collections::HashMap<String, String> {
        Analyser::new()
            .analyse(src, D)
            .command_aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.target.clone()))
            .collect()
    }

    #[test]
    fn primary_repro_cross_domain_alias_through_tracked_binding_resolves() {
        // TP — issue #923 idx 9 exact repro (tclsh9.0-verified: prints 42).
        // `set s [interp create -safe]` binds `s`; `interp alias $s greet {}
        // ::app::Helper` previously abstained outright because the source
        // path was dynamic text, leaving `greet` unresolved inside the
        // child's eval body (spurious W123 + 0 definition locations).
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset s [interp create -safe]\ninterp alias $s greet {} ::app::Helper\ninterp eval $s { greet }\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    #[test]
    fn object_command_handle_eval_form_resolves_through_tracked_binding() {
        // TP — the object-command spelling (`$mpip eval { ... }`) doctools.tcl
        // actually uses, exercising `handle_interp_handle_eval_command` rather
        // than the literal `interp eval PATH` form. `$mpip eval {...}` is
        // itself a non-literal command dispatch — `mpip` is tracked only as
        // an interpreter handle, not a known TclOO object
        // (`var_command.rs`'s separate, unrelated dispatch-suppression
        // system), so it legitimately still draws W307 the same as any
        // other untracked `$var subcommand` call. What this fix controls is
        // narrower: `greet` must resolve *inside* the eval body — no W123
        // (unknown command) and no W140 (never-created interpreter).
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset mpip [interp create -safe]\ninterp alias $mpip greet {} ::app::Helper\n$mpip eval { greet }\n";
        let diags = analyser_diags(src, D);
        assert!(
            diags.iter().all(|(c, _, _)| c != "W123" && c != "W140"),
            "{diags:?}"
        );
    }

    #[test]
    fn explicit_literal_name_variant_records_no_spurious_w140() {
        // TP — `set s [interp create -safe literalName]` now records
        // `literalName` in the interpreter map via the value-flow path, so a
        // later literal `interp eval literalName {...}` no longer sees an
        // apparently-uncreated interpreter, even though the path never
        // reaches `handle_interp_create_command` directly (it's nested
        // inside a `set`, not a bare top-level statement).
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset s [interp create -safe literalName]\ninterp alias literalName greet {} ::app::Helper\ninterp eval literalName { greet }\n";
        assert!(!fires(src, D, "W140"), "{:?}", codes(src, D));
    }

    #[test]
    fn two_procs_sharing_a_variable_name_never_collide() {
        // TP — cross-contamination guard (secondary issue #2 found during
        // this fix's research): each proc's `set s [interp create -safe]`
        // must get its own per-call-site `@autoname@<offset>` domain, not
        // one shared domain keyed off the raw variable text `s`. Before the
        // fix (verified live against the LSP binary) makeA's `helper` call
        // resolved into makeB's definition.
        let src = "proc makeA {} {\n    set s [interp create -safe]\n    interp eval $s {\n        proc helper {} { return A }\n        helper\n    }\n}\nproc makeB {} {\n    set s [interp create -safe]\n    interp eval $s {\n        proc helper {} { return B }\n        helper\n    }\n}\n";
        let r = Analyser::new().analyse(src, D);
        let helper_keys: Vec<&String> = r
            .all_procs
            .keys()
            .filter(|k| k.ends_with("::helper"))
            .collect();
        assert_eq!(
            helper_keys.len(),
            2,
            "expected 2 distinct helper domains: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        assert_ne!(helper_keys[0], helper_keys[1]);
    }

    #[test]
    fn untracked_dynamic_interp_path_stays_conservative() {
        // FP guard — a `$VAR` never bound by `set VAR [interp create ...]`
        // (here a bare proc parameter) must stay exactly as conservative as
        // before this fix: no crash, no spurious W140 (existence stays
        // unknowable for a genuinely untracked dynamic path).
        let src = "proc useEval {ip} {\n    interp eval $ip { someCmd }\n}\n";
        assert!(!fires(src, D, "W140"), "{:?}", codes(src, D));
    }

    #[test]
    fn reassigning_the_variable_clears_the_stale_interp_binding() {
        // FP guard — a later plain-string `set` of the same variable must
        // clear the interp binding, so a stale domain is never resolved and
        // the cross-domain alias falls through exactly as it does today for
        // a genuinely-unresolvable dynamic src_path.
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset mpip [interp create -safe]\nset mpip \"not an interpreter\"\ninterp alias $mpip greet {} ::app::Helper\n";
        assert!(
            command_alias_targets(src).is_empty(),
            "{:?}",
            command_alias_targets(src)
        );
    }

    #[test]
    fn existing_literal_top_level_behaviour_is_unchanged() {
        // TN — byte-for-byte unchanged control (scratchpad/audit/a): the
        // same shape as the primary repro, but with a literal path
        // throughout instead of a tracked variable binding.
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\ninterp create -safe s\ninterp alias s greet {} ::app::Helper\ninterp eval s { greet }\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    #[test]
    fn builtins_only_dynamic_interp_eval_body_stays_silent() {
        // TN — a tracked dynamic-path eval body containing only builtins
        // must stay diagnostic-free, confirming the fix doesn't newly
        // over-fire (scratchpad/audit/c).
        let src = "set s [interp create -safe]\ninterp eval $s {\n    set x 1\n    expr {$x + 1}\n    puts \"hi\"\n}\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    #[test]
    fn builtins_only_literal_interp_eval_body_stays_silent() {
        // TN — the same false-fire (W210 on `x`) reproduced with a *literal*
        // interpreter path, no dynamic tracking involved at all: `interp
        // eval`'s script argument is a `Plain`-body-kind `ArgRole::Body`
        // (unlike an `if`/`while`/`catch` body, never flattened into its
        // own CFG since the target interpreter is opaque to static
        // analysis), so it was scanned as ordinary value text with no
        // notion of its own `set x 1` write before the later `$x` read.
        let src = "interp create child\ninterp eval child {\n    set x 1\n    expr {$x + 1}\n}\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    /// The `@interp@<key>` synthetic namespaces a source's definitions
    /// land in — the observable projection of the interpreter-domain key.
    fn interp_domains(src: &str) -> Vec<String> {
        let mut keys: Vec<String> = Analyser::new()
            .analyse(src, D)
            .all_procs
            .keys()
            .filter_map(|k| k.split("::").find(|s| s.starts_with("@interp@")))
            .map(str::to_owned)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    #[test]
    fn braced_path_value_flow_key_matches_the_direct_handler_issue_1025() {
        // TP — `interp create {child}` names interpreter `child`
        // (tclsh8.6/9.0: `interp exists child` → 1, `interp slaves` →
        // `child`). The value-flow path used to `split_whitespace` the raw
        // substitution text and bind `$i` to `"{child}"`, a domain the
        // direct handler never records.
        let via_value = "set i [interp create {child}]\n$i eval { proc helper {} {} }\n";
        let direct = "interp create {child}\ninterp eval child { proc helper {} {} }\n";
        assert_eq!(
            interp_domains(via_value),
            interp_domains(direct),
            "value-flow and direct keys must agree"
        );
        assert_eq!(interp_domains(via_value), vec!["@interp@child".to_string()]);
    }

    #[test]
    fn nested_path_value_flow_key_matches_the_direct_handler_issue_1025() {
        // TP — `{parent child}` is one word: a descent path (tclsh8.6/9.0:
        // `interp create {parent child}` returns `parent child` and
        // `interp slaves parent` → `child`). `split_whitespace` used to
        // split it into `"{parent"` + `"child}"` and bind the first
        // fragment.
        let via_value = "interp create parent\nset i [interp create {parent child}]\n$i eval { proc helper {} {} }\n";
        let direct = "interp create parent\ninterp create {parent child}\ninterp eval {parent child} { proc helper {} {} }\n";
        assert_eq!(interp_domains(via_value), interp_domains(direct));
        assert!(
            interp_domains(via_value).contains(&"@interp@parent child".to_string()),
            "{:?}",
            interp_domains(via_value)
        );
    }

    #[test]
    fn braced_path_binding_links_a_later_literal_eval_issue_1025() {
        // TP — the linkage the diverging key broke: a proc defined through
        // the tracked `$i eval` handle must resolve for a later literal
        // `interp eval child` call, with no W123.
        let src = "set i [interp create {child}]\n$i eval { proc helper {} { return 1 } }\ninterp eval child { helper }\n";
        assert!(!fires(src, D, "W123"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "W140"), "{:?}", codes(src, D));
    }

    #[test]
    fn synthetic_autoname_key_used_for_pathless_interp_create() {
        // TP — pins the synthetic per-call-site key convention (mirrors
        // `handle_namespace_eval_dynamic_target_gets_a_synthetic_span_keyed_name`'s
        // `@dynns@15` pin): a pathless `interp create` inside `set` gets an
        // `@autoname@<offset-of-the-substitution's-opening-bracket>` key,
        // not the variable's raw text, so `foo`'s definition homes under a
        // synthetic, call-site-unique domain rather than one keyed on `s`.
        let src = "set s [interp create -safe]\ninterp eval $s { proc foo {} {} }\n";
        let r = Analyser::new().analyse(src, D);
        assert!(
            r.all_procs.keys().any(|k| k.contains("@autoname@6")),
            "{:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }
}

// ===========================================================================
// rename — issue #923 idx 3: constant-folding a dynamic-but-resolvable
// `rename OLD NEW` argument instead of unconditionally giving up.
// ===========================================================================
mod rename {
    use super::*;

    fn renamed_commands(src: &str) -> std::collections::HashMap<String, String> {
        Analyser::new().analyse(src, D).renamed_commands.clone()
    }

    #[test]
    fn static_rename_recorded_unchanged() {
        // TN — control check: a fully literal rename never even reaches
        // the new constant-folding resolver and must keep working
        // byte-for-byte as before.
        let src = "proc ::foo_impl {} { return impl }\nrename ::foo_impl ::foo\n";
        assert_eq!(
            renamed_commands(src).get("::foo"),
            Some(&"::foo_impl".to_string())
        );
    }

    #[test]
    fn single_var_old_name_resolves_via_constant_folding() {
        // TP — finding's own simplest repro #1: a single whole-word
        // `$var` holding a known-constant command name.
        let src = "proc ::foo_impl {} { return impl }\nset old ::foo_impl\nrename $old ::foo\n";
        assert_eq!(
            renamed_commands(src).get("::foo"),
            Some(&"::foo_impl".to_string())
        );
    }

    #[test]
    fn concatenated_var_new_name_resolves_via_constant_folding() {
        // TP — finding's own simplest repro #2: a literal-plus-`$var`
        // concatenation.
        let src = "proc ::foo_impl {} { return impl }\nset key impl\nrename ::foo_$key ::foo\n";
        assert_eq!(
            renamed_commands(src).get("::foo"),
            Some(&"::foo_impl".to_string())
        );
    }

    #[test]
    fn two_concatenated_vars_in_same_straight_line_scope_resolve() {
        // TP — both OLD and NEW built from resolvable variables in the
        // same straight-line proc scope, the closest Tier-1-fixable
        // approximation of the tcllib `json::SwitchTo` idiom (which
        // additionally needs a `foreach` var and a proc parameter — see
        // the two FN tests below, deliberately out of scope for Tier 1).
        let src = "namespace eval ::mypkg {}\n\
             proc ::mypkg::greet_tcl {name} { return \"hi $name\" }\n\
             proc ::mypkg::activate {} {\n    \
                 set c greet\n    \
                 set key tcl\n    \
                 rename ::mypkg::${c}_$key ::mypkg::${c}\n\
             }\n";
        assert_eq!(
            renamed_commands(src).get("::mypkg::greet"),
            Some(&"::mypkg::greet_tcl".to_string())
        );
    }

    #[test]
    fn genuinely_dynamic_value_stays_unresolved() {
        // TN — regression guard against over-eager folding: a value
        // that's never a compile-time constant (piped through `gets`)
        // must still be reported dynamic (no entry recorded).
        let src = "proc ::foo_impl {} { return impl }\nset old [gets stdin]\nrename $old ::foo\n";
        assert!(!renamed_commands(src).contains_key("::foo"));
    }

    #[test]
    fn foreach_loop_variable_over_a_literal_list_is_constant_folded() {
        // Was FN (documented at idx 3's landing) — closed by issue #923 idx
        // 86: a `foreach VAR {literal list} { ... }` loop over a fully
        // literal list now binds `VAR` to each element in turn before
        // simulating the body's own `rename`/`proc` sub-commands (the two
        // constant-fold-sensitive callers go-to-definition/references/
        // rename care about), rather than leaving the loop variable out of
        // the constant-string lattice entirely. `renamed_commands` is
        // populated the same as if `c` had been a plain top-level `set`
        // constant.
        let src =
            "proc ::foo_impl {} { return impl }\nforeach c {foo} { rename ::${c}_impl ::${c} }\n";
        assert_eq!(
            renamed_commands(src).get("::foo"),
            Some(&"::foo_impl".to_string())
        );
    }

    #[test]
    fn foreach_over_a_braced_list_element_is_not_mis_split_on_whitespace() {
        // Codex review (PR #1020): the literal-`foreach` simulation parsed
        // its value with `split_whitespace`, so a braced element `{bar baz}`
        // in `foreach c {a {bar baz}} { proc ::$c {} {} }` was mis-sliced
        // into `{bar` + `baz}` and the re-dispatched `proc` created bogus
        // commands `::{bar` / `::baz}`. Parsed as a real Tcl list, that value
        // is exactly two elements — `a` and `bar baz` — so no brace-fragment
        // proc is ever recorded.
        let procs = Analyser::new()
            .analyse("foreach c {a {bar baz}} { proc ::$c {} {} }\n", D)
            .all_procs;
        assert!(
            !procs.keys().any(|k| k.contains('{') || k.contains('}')),
            "no brace-fragment proc from a mis-split braced element: {:?}",
            procs.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn rename_target_set_only_in_a_conditional_branch_does_not_resolve() {
        // Codex review (PR #1020): a `rename`/`source` target read from the
        // last-write-wins const map is unsound across an `if` join. Here `t`
        // is `::foo` straight-line but reassigned `::bar` inside an `if`
        // body, so at the `rename` it could be either — the analyser must
        // abstain (leave the rename dynamic), never pin it to the branch
        // value `::bar` the lexical map happens to hold last.
        let renamed = renamed_commands(
            "proc ::foo_impl {} {}\nset t ::foo\nif {$cond} { set t ::bar }\nrename ::foo_impl $t\n",
        );
        assert!(
            !renamed.contains_key("::bar") && !renamed.contains_key("::foo"),
            "a branch-dependent rename target must not resolve to either branch: {renamed:?}"
        );
    }

    #[test]
    fn rename_target_set_straight_line_still_resolves() {
        // Control for the above: without the conditional, the single
        // straight-line `set` dominates the `rename`, so it still resolves
        // (issue #923 idx 3 behaviour unchanged).
        let renamed =
            renamed_commands("proc ::foo_impl {} {}\nset t ::foo\nrename ::foo_impl $t\n");
        assert_eq!(renamed.get("::foo"), Some(&"::foo_impl".to_string()));
    }

    #[test]
    fn proc_parameter_is_not_constant_folded() {
        // FN (expected, documented — explicitly out of scope per idx 3's
        // research plan: closing this needs interprocedural single-
        // call-site literal-argument propagation, a materially larger,
        // separate feature).
        let src = "proc ::foo_impl {} { return impl }\n\
             proc activate {key} { rename ::foo_$key ::foo }\n\
             activate impl\n";
        assert!(!renamed_commands(src).contains_key("::foo"));
    }

    #[test]
    fn resolvable_dynamic_rename_no_longer_widens_has_dynamic_providers() {
        // TN (bonus side effect) — before this fix, ANY dynamic-*looking*
        // rename set `has_dynamic_providers`, which blanket-suppresses
        // W123 for the whole file (diagnostics/unresolved.rs). Once the
        // rename resolves statically, that flag must stay false so W123
        // keeps firing on genuinely unknown commands elsewhere in the
        // file.
        let src = "proc ::foo_impl {} { return impl }\nset old ::foo_impl\nrename $old ::foo\n";
        assert!(!Analyser::new().analyse(src, D).has_dynamic_providers);
    }

    #[test]
    fn unresolvable_rename_still_widens_has_dynamic_providers() {
        // TN — control check for the above: a genuinely unresolvable
        // dynamic rename must still widen `has_dynamic_providers`,
        // unchanged from before this fix.
        let src = "proc ::foo_impl {} { return impl }\nset old [gets stdin]\nrename $old ::foo\n";
        assert!(Analyser::new().analyse(src, D).has_dynamic_providers);
    }
}

// ===========================================================================
// EvalUplevelIndirectDispatch — issue #923 idx 94: a bare `$var` body of an
// `ArgRole::Body`-marked argument (`eval $cmd`, `uplevel #0 $cmd …`)
// dynamically evaluates $var's value as a script at runtime, whose first
// word is the command actually dispatched — previously invisible to
// `command_invocations` (found by hover/go-to-definition via an independent
// cursor-token walk, missed by references/rename).
// ===========================================================================
mod eval_uplevel_indirect_dispatch {
    use super::*;

    fn invocations(
        src: &str,
    ) -> Vec<tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
        Analyser::new().analyse(src, D).command_invocations.clone()
    }

    #[test]
    fn eval_of_a_list_computed_var_resolves_the_real_head() {
        // TP — the finding's own minimal repro: `eval $cmdD` where `$cmdD`
        // is built via `[list greetD World]` — a computed value, not a
        // simple lexical literal the *lowering*-phase's simpler const-map
        // can fold (see `try_lower_eval_static`'s `const_map_lookup`), so
        // only the analyser's own flow-sensitive SSA-based
        // `settle_const_dispatches` can resolve it. Real tclsh9.0/8.6-
        // verified: `eval $cmdD` dispatches to `greetD World`.
        let src = "proc greetD {n} {puts \"D $n\"}\nset cmdD [list greetD World]\neval $cmdD\n";
        let invs = invocations(src);
        assert!(
            invs.iter()
                .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::greetD")),
            "expected an indirect invocation resolving to ::greetD: {invs:?}"
        );
        // The `greetD` word inside `[list greetD World]` is also a
        // separate, *direct*, rename-writable invocation at its own span
        // (byte 45..51, `&src[45..51] == "greetD"`) — the actual command
        // name as written, not just the `$cmdD` dispatch anchor.
        assert!(
            invs.iter().any(|i| !i.indirect
                && i.rename_safe
                && i.range == tcl_lexer::Span::new(45, 51)
                && i.resolved_qualified_name.as_deref() == Some("::greetD")),
            "expected a direct, rename-safe invocation anchored on greetD's own word: {invs:?}"
        );
    }

    #[test]
    fn uplevel_hash_zero_of_a_var_headed_script_resolves_the_real_head() {
        // TP — the real `tk/library/tearoff.tcl` `MenuDup`-analogue shape:
        // `uplevel #0 $cmd [list $w $newMenu]`. `$cmd`'s own value is the
        // command name; the trailing `[list …]` argument supplies its
        // arguments (uplevel concatenates all its post-level arguments into
        // one script). tclsh9.0/8.6-verified: dispatches to `target`.
        let src =
            "proc target {a b} {puts \"$a-$b\"}\nset cmd target\nuplevel #0 $cmd [list x y]\n";
        let invs = invocations(src);
        assert!(
            invs.iter()
                .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::target")),
            "expected an indirect invocation resolving to ::target: {invs:?}"
        );
    }

    #[test]
    fn plain_eval_of_a_braced_literal_body_is_unaffected() {
        // TN — control check: the ordinary literal-body `eval {…}` shape
        // (handled entirely by `analyse_body`'s existing `Str`-token path)
        // must keep resolving exactly as before; the new `Var`-token branch
        // is additive, not a replacement.
        let src = "proc target {} { return hi }\neval { target }\n";
        let invs = invocations(src);
        assert!(
            invs.iter()
                .any(|i| !i.indirect && i.resolved_qualified_name.as_deref() == Some("::target")),
            "expected a direct (non-indirect) invocation resolving to ::target: {invs:?}"
        );
    }

    #[test]
    fn genuinely_dynamic_eval_body_stays_unresolved() {
        // TN — regression guard: a value with no provable constant origin
        // (piped through `gets`) must still abstain — no invocation
        // recorded, matching the analyser's existing conservative default.
        let src = "proc target {} { return hi }\nset cmd [gets stdin]\neval $cmd\n";
        let invs = invocations(src);
        assert!(
            !invs
                .iter()
                .any(|i| i.resolved_qualified_name.as_deref() == Some("::target")),
            "a genuinely dynamic eval body must not resolve: {invs:?}"
        );
    }

    #[test]
    fn resolves_identically_inside_a_proc_body() {
        // TN (confirms a hypothesis in the finding's own root-cause hint is
        // moot): the auditor flagged "worth checking whether per-proc
        // FunctionUnits reach settle_one_site with usable SSA/def-use data
        // the same way the top-level unit does" as a second, compounding
        // gap. Empirically it isn't one — `function_unit_at`/
        // `const_contributors` have no top-level-vs-proc special-casing,
        // and this resolves identically to the top-level repro above.
        let src = "proc greetD {n} {puts \"D $n\"}\nproc caller {} {\n    set cmdD [list greetD World]\n    eval $cmdD\n}\n";
        let invs = invocations(src);
        assert!(
            invs.iter()
                .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::greetD")),
            "expected an indirect invocation resolving to ::greetD even inside a proc body: {invs:?}"
        );
    }
}

// ===========================================================================
// `apply [list {params} {body} ns]` — the list-constructor lambda idiom
// (issue #923 idx 116). Each list element must reach the body walk with its
// list delimiters removed, exactly as a literal braced lambda does.
// ===========================================================================
mod apply_list_lambda {
    use super::*;

    #[test]
    fn apply_list_constructor_body_element_is_delimiter_stripped() {
        // Codex review (PR #1020): `resolve_dynamic_apply_lambda`'s `[list
        // …]` path sliced each element's raw source span, keeping the braces
        // of a `{frobnicate arg}` body element, so the body re-segmented as a
        // single braced word and the real `frobnicate` call was never seen.
        // With the element text delimiter-stripped (zipped from the
        // segmenter's `texts`, the same shape the literal-lambda path uses),
        // the body walks as `frobnicate arg` and records `frobnicate` as its
        // own command invocation.
        let invs = Analyser::new()
            .analyse("apply [list {} {frobnicate arg} ::myns]\n", D)
            .command_invocations;
        assert!(
            invs.iter().any(|i| i.name == "frobnicate"),
            "apply [list ...] body must segment to the real `frobnicate` call: {:?}",
            invs.iter().map(|i| &i.name).collect::<Vec<_>>()
        );
    }
}

// ===========================================================================
// `::tcl::dict::*` standalone spellings (issue #923 idx 105) must carry the
// same analysis contract as the `dict` subcommands they mirror, not a
// `CommandSpec::DEFAULT` stub (Codex review, PR #1020).
// ===========================================================================
mod dict_qualified_specs {
    use super::*;

    #[test]
    fn qualified_dict_for_binds_loop_vars_and_walks_body() {
        // The standalone `::tcl::dict::for` must carry `dict for`'s arg-roles
        // (`LoopVarList` + `Body`) and its `DictFor` analyser hook, so its
        // body is analysed and `k`/`v` are bound — `CommandSpec::DEFAULT`
        // left it inert, so a call inside the body was never seen.
        let invs = Analyser::new()
            .analyse(
                "set d {a 1 b 2}\n::tcl::dict::for {k v} $d { frobnicate $k $v }\n",
                D,
            )
            .command_invocations;
        assert!(
            invs.iter().any(|i| i.name == "frobnicate"),
            "::tcl::dict::for body must be analysed like dict for: {:?}",
            invs.iter().map(|i| &i.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn qualified_dict_set_body_walks_a_nested_dict_for() {
        // A `::tcl::dict::for` nested inside another command's body still
        // carries its analysis contract, so the inner body's own call is
        // seen — a stronger check than a top-level call that the generic
        // walk would reach anyway.
        let invs = Analyser::new()
            .analyse(
                "proc p {d} { ::tcl::dict::for {k v} $d { frobnicate $k } }\n",
                D,
            )
            .command_invocations;
        assert!(
            invs.iter().any(|i| i.name == "frobnicate"),
            "nested ::tcl::dict::for body must be analysed: {:?}",
            invs.iter().map(|i| &i.name).collect::<Vec<_>>()
        );
    }
}

// ===========================================================================
// TestW123UnresolvedCommand — unknown-command detection.
// ===========================================================================
mod unresolved_command {
    use super::*;

    fn w123(src: &str) -> Vec<String> {
        analyser_diags(src, D)
            .into_iter()
            .filter(|(c, _, _)| c == "W123")
            .map(|(_, m, _)| m)
            .collect()
    }

    #[test]
    fn unknown_command_emits_w123() {
        let d = w123("mycommand a b c");
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("mycommand"));
    }

    #[test]
    fn known_builtin_and_user_proc_no_w123() {
        assert!(w123("set x 1").is_empty());
        assert!(w123("proc mycommand {x} { puts $x }\nmycommand hello").is_empty());
        // Forward-defined proc still suppresses.
        assert!(w123("mycommand hello\nproc mycommand {x} { puts $x }").is_empty());
    }

    #[test]
    fn namespace_qualified_and_variable_command_skipped() {
        assert!(w123("::myns::cmd arg1").is_empty());
        // `$cmd hello` — the head starts with `$`, not a literal command name.
        assert!(
            w123("set cmd puts\n$cmd hello")
                .iter()
                .all(|m| !m.contains("$cmd"))
        );
    }

    #[test]
    fn package_require_suppresses_w123() {
        // A `package require` may register the missing command at runtime →
        // suppressed (e.g. `package require Tk` provides `button`).
        assert!(w123("package require Tk\nbutton .b -text hi").is_empty());
    }

    #[test]
    fn dynamic_providers_suppress_w123() {
        // `has_dynamic_providers` gates W123 exactly as it gates W120: a
        // `load` / `auto_path` mutation / dynamic import may register
        // commands at runtime the analyser can't see, so the resolvable
        // command set is unknowable and W123 must abstain file-wide.
        for src in [
            "load mylib.so\nmycommand arg1",
            "lappend auto_path /opt/mylib\nmycmd arg",
            "namespace import ::foo::[computed]::*\nbar arg",
        ] {
            assert!(w123(src).is_empty(), "expected no W123 for {src:?}");
        }
    }

    #[test]
    fn glob_namespace_import_suppresses_matching_tail_only() {
        // A literal glob import provides every source command matching the
        // pattern's tail — `render_box` matches `render_*` (silent), while an
        // unrelated name matches nothing and still fires.
        let src = "namespace import ::acme::widgets::render_*\nrender_box 10 20\nfrobnicate 1\n";
        let d = w123(src);
        assert_eq!(d.len(), 1, "only the unrelated name fires; got {d:?}");
        assert!(d[0].contains("frobnicate"));
        // A full `*` tail conservatively resolves every bare name (the
        // imported namespace's export set is unknowable single-file).
        assert!(w123("namespace import ::foo::*\nbar arg").is_empty());
    }

    #[test]
    fn literal_namespace_import_suppresses_exactly_that_name() {
        let src =
            "namespace import ::acme::widgets::render_box\nrender_box 10 20\nrender_circle 5\n";
        let d = w123(src);
        assert_eq!(d.len(), 1, "only the un-imported name fires; got {d:?}");
        assert!(d[0].contains("render_circle"));
    }

    #[test]
    fn bare_dict_ensemble_builtin_resolves_from_inside_tcl_dict_namespace() {
        // FP — regression for issue #923 idx 105: `exists`/`get` are real,
        // separately-callable commands (`::tcl::dict::exists`,
        // `::tcl::dict::get`), backing the `dict` ensemble's own
        // subcommands (confirmed against tclsh9.0.4/8.6.14: `info commands
        // ::tcl::dict::*` lists them). A proc lexically defined *inside*
        // `::tcl::dict` (the real tcllib `dicttool.tcl` idiom) resolves a
        // bare call to them via ordinary current-namespace-then-global
        // lookup, so it must not fire W123 — isolated from any
        // `namespace ensemble configure` patching (idx 105 Part B, tested
        // separately) to prove this half is a pure namespace-resolution fact.
        let src = "proc ::tcl::dict::myhelper {d k} {\n\
             if {[exists $d $k]} { return [get $d $k] }\n\
             return MISSING\n\
             }\n\
             ::tcl::dict::myhelper [dict create a 1 b 2] a\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
    }

    #[test]
    fn bare_dict_ensemble_builtin_outside_tcl_dict_namespace_still_fires_w123() {
        // TP — regression guard: `exists`/`get` called bare *outside*
        // `::tcl::dict` must still fire (tclsh-confirmed real error:
        // "invalid command name \"exists\"") — proves the fix is properly
        // namespace-scoped, not a blanket allow-list for these two names.
        let d = w123("proc foo {} { return [exists bar] }\n");
        assert_eq!(d.len(), 1, "got {d:?}");
        assert!(d[0].contains("exists"));
    }

    #[test]
    fn fully_qualified_tcl_dict_builtin_calls_are_unaffected() {
        // TN — regression guard: the qualified spelling already worked via
        // the pre-existing `name.contains("::")` conservative skip; the new
        // resolution-candidate check must not change that.
        let src = "proc demo {} {\n\
             set d [dict create a 1 b 2]\n\
             if {[::tcl::dict::exists $d a]} { return [::tcl::dict::get $d a] }\n\
             }\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
    }

    #[test]
    fn ordinary_namespace_bare_name_resolution_is_unaffected_by_dict_fix() {
        // TN — regression guard: the common case (a bare call resolving to
        // an ordinary user proc via its own namespace) must be unaffected by
        // the new resolution-candidate check, which is a no-op there (the
        // bare-name check above it already resolves it).
        assert!(
            w123(
                "namespace eval ::foo {\n\
                 proc helper {} { return 42 }\n\
                 proc bar {} { return [helper] }\n\
                 }\n"
            )
            .is_empty()
        );
    }

    // issue #923 idx 3/4: tcllib `textutil::adjust` submodule commands were
    // registered under the wrong (umbrella-only) 2-segment name, so the
    // real 3-segment commands the common `package require textutil::adjust;
    // namespace import textutil::adjust::*` idiom (georgtree_argparse's
    // `argparse.tcl:380-382`) actually resolves to had no registry entry at
    // all. Ground truth (tclsh 9.0.4 + real tcllib-2.0): `package require
    // textutil::adjust` creates `::textutil::adjust::adjust` /
    // `::textutil::adjust::indent`, never a bare `::textutil::adjust`.
    mod textutil_adjust_idx3_idx4 {
        use super::*;

        #[test]
        fn qualified_submodule_calls_resolve_after_package_require() {
            // As phrased by the fix's own acceptance criterion: a call to
            // the real, canonical name resolves once the submodule package
            // is required. Note `package require` (any package, this file's
            // included) blanket-suppresses W123 file-wide (`emit_unresolved_
            // command_diagnostics`'s conservative "package may define
            // anything at runtime" gate) — so this pins that the call
            // resolves under the real-world idiom, while the two tests below
            // isolate the registry-data fact itself (no `package require` in
            // scope, so the file-wide suppression gate does not mask it).
            let src = "package require textutil::adjust\n\
                       namespace import textutil::adjust::*\n\
                       set a [adjust hello -length 20]\n\
                       set b [indent $a \"  \"]\n";
            assert!(w123(src).is_empty(), "got {:?}", w123(src));
        }

        #[test]
        fn qualified_submodule_names_are_directly_registry_known() {
            // TP, isolating the registry-data fact with no `package require`
            // in scope (so the blanket suppression gate above cannot mask
            // it): the literal, fully-qualified 3-segment names must be
            // directly known to W123's registry-name set.
            let src = "textutil::adjust::adjust hello -length 20\n\
                       textutil::adjust::indent hello \"  \"\n";
            assert!(w123(src).is_empty(), "got {:?}", w123(src));
        }

        #[test]
        fn a_bare_unimported_flat_name_still_unresolved() {
            // TN/FN contrast: with no `package require`, no `namespace
            // import`, and no umbrella alias in play, the *bare* flattened
            // names (`adjust`/`indent` with no qualification at all) are
            // correctly still unknown — the fix adds the submodule's real
            // names, it does not fabricate a global bare alias that only
            // the `textutil` umbrella actually provides.
            let d = w123("adjust hello -length 20\n");
            assert_eq!(d.len(), 1, "got {d:?}");
            assert!(d[0].contains("adjust"));
        }
    }

    #[test]
    fn coroutine_name_is_a_known_command() {
        // `coroutine NAME cmd ?arg …?` creates the command NAME
        // (`TclNRCoroutineObjCmd`) — calling it is not an unknown command,
        // while a typo'd name in the same file still fires.
        let src = "proc gen {} { while 1 { yield 1 } }\ncoroutine nextNum gen\nnextNum\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
        let typo = "proc gen {} { while 1 { yield 1 } }\ncoroutine nextNum gen\nnextNun\n";
        let d = w123(typo);
        assert_eq!(d.len(), 1, "the typo'd name fires; got {d:?}");
        assert!(d[0].contains("nextNun"));
    }

    #[test]
    fn interp_create_name_is_a_known_command() {
        // `interp create NAME` binds NAME as the child interpreter's
        // command — `NAME eval {…}` dispatches on it.
        assert!(w123("interp create child\nchild eval { puts hi }\n").is_empty());
        // The `-safe` flag at the name index is an option, never a name; a
        // missing name is auto-generated — nothing to record, nothing fires.
        assert!(w123("interp create -safe\n").is_empty());
        let r = Analyser::new().analyse("interp create -safe\n", D);
        assert!(
            !r.created_instance_commands.contains("-safe"),
            "an option flag must not be recorded as a created command"
        );
    }

    #[test]
    fn renamed_away_builtin_call_fires_w123() {
        // `rename puts {}` (or a rename-away) deletes the builtin — a later
        // call fails "invalid command name" (tclsh 9.0.4), so W123 fires.
        let d = w123("rename puts {}\nputs x\n");
        assert_eq!(d.len(), 1, "deleted builtin fires; got {d:?}");
        assert!(d[0].contains("'puts'"));
        assert_eq!(w123("rename puts myputs\nputs x\n").len(), 1);
        // The rename target stays known, and calls lexically BEFORE the
        // rename still resolve to the builtin.
        assert!(w123("rename puts myputs\nmyputs x\n").is_empty());
        assert!(w123("puts before\nrename puts myputs\n").is_empty());
        // A re-binding after the deletion makes the name callable again.
        assert!(w123("rename puts {}\nproc puts {args} {}\nputs x\n").is_empty());
        // A rename buried in a proc body is conditional — it runs only if
        // the proc is ever called — so the builtin stays known at top level.
        assert!(w123("proc hook {} { rename puts _p }\nputs hi\n").is_empty());
    }

    #[test]
    fn namespace_unknown_handler_suppresses_w123() {
        // Installing a per-namespace unknown handler (TIP 181) makes command
        // resolution unknowable — file-wide suppression, like a dynamic
        // `proc unknown`.
        let src = "proc handler {args} { puts $args }\nnamespace unknown handler\nmystery_cmd 1\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
        // The bare query form installs nothing; an empty handler resets to
        // the default — neither suppresses.
        assert_eq!(w123("namespace unknown\nmystery_cmd 1\n").len(), 1);
        assert_eq!(w123("namespace unknown {}\nmystery_cmd 1\n").len(), 1);
    }

    #[test]
    fn list_wrapped_namespace_unknown_installer_suppresses_w123() {
        // FP — issue #923 idx 110: `namespace eval $ns [list namespace
        // unknown $handler]` (tcllib's `namespacex::hook::Set` idiom) is
        // a `Cmd`-kind body — `analyse_body`'s literal-`{...}`-only gate
        // never walks it, and the generic nested-substitution scan
        // resolves the head to `list`, never `namespace unknown` — so
        // the installer was previously invisible to every path.
        let src = "proc handler {args} { puts $args }\nnamespace eval ::target [list namespace unknown handler]\nmystery_cmd 1\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
    }

    #[test]
    fn list_wrapped_namespace_unknown_full_tcllib_idiom_suppresses_w123() {
        // FP — the full attested shape (mirrors tcllib
        // modules/namespacex/namespacex.tcl:157-162's `hook::Set`, and
        // the finding's own repro): the installer is itself wrapped in
        // another proc, and the handler is installed from inside a
        // namespace body.
        let src = "proc ::hooklib::Set {ns handler} {\n    namespace eval $ns [list namespace unknown $handler]\n}\nnamespace eval ::target {\n    proc fallbackHandler {args} { return \"handled:$args\" }\n    ::hooklib::Set ::target [namespace code fallbackHandler]\n    proc run {} { return [mystery arg1 arg2] }\n}\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
    }

    #[test]
    fn list_wrapped_namespace_unknown_via_namespace_inscope_suppresses_w123() {
        // FP — `namespace inscope` shares the same
        // `AnalyserHookId::NamespaceEval` hook as `namespace eval`, so
        // one fix covers both call forms.
        let src = "proc handler {args} { puts $args }\nnamespace inscope ::target [list namespace unknown handler]\nmystery_cmd 1\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
    }

    #[test]
    fn list_wrapped_namespace_unknown_requires_the_exact_shape() {
        // TP — over-suppression guards: each of these must still fire
        // W123, proving the recogniser requires the literal `list
        // namespace unknown` head with a non-empty handler, not "any
        // list-wrapped namespace eval body".
        assert_eq!(
            w123("namespace eval ::target [list puts hello]\nmystery_cmd 1\n").len(),
            1,
            "an unrelated list-wrapped body must not suppress W123"
        );
        assert_eq!(
            w123("namespace eval ::target [list namespace export foo]\nmystery_cmd 1\n").len(),
            1,
            "a different namespace subcommand must not suppress W123"
        );
        assert_eq!(
            w123("namespace eval ::target [list namespace unknown]\nmystery_cmd 1\n").len(),
            1,
            "the bare query form installs nothing"
        );
        assert_eq!(
            w123("namespace eval ::target [list namespace unknown {}]\nmystery_cmd 1\n").len(),
            1,
            "an empty handler resets to the default, installs nothing"
        );
    }

    #[test]
    fn list_wrapped_namespace_unknown_does_not_blanket_suppress_the_namespace() {
        // TP — negative control: with no `namespace unknown` installer
        // anywhere, a genuinely unknown command inside the namespace
        // body must still be flagged.
        let src = "namespace eval ::target {\n    proc run {} { return [mystery arg1 arg2] }\n}\n";
        assert_eq!(w123(src).len(), 1);
    }

    #[test]
    fn list_wrapped_namespace_unknown_via_concat_is_a_known_remaining_gap() {
        // FP (documented, NOT fixed by this change) — the same idiom
        // built via `concat` instead of a literal `list` call is
        // intentionally out of scope (issue #923 idx 110's fix is
        // narrow to the exact attested `list namespace unknown` shape).
        // Pinned so nobody mistakes the narrow fix for a full
        // generalisation.
        let src = "proc handler {args} { puts $args }\nnamespace eval ::target [concat namespace unknown handler]\nmystery_cmd 1\n";
        assert_eq!(
            w123(src).len(),
            1,
            "concat-built installers are a documented, out-of-scope gap"
        );
    }

    #[test]
    fn static_rename_target_is_a_known_command() {
        // A static `rename OLD NEW` *defines* NEW (confirmed against tclsh
        // 9.0.4: `rename puts myputs; info commands myputs` lists it), so a
        // call to the renamed name must not draw W123 — while a genuinely
        // unknown name in the same file still does.
        assert!(w123("rename puts myputs\nmyputs hello").is_empty());
        assert_eq!(w123("rename puts myputs\nmyputz hello").len(), 1);
    }

    #[test]
    fn stub_directive_suppresses_w123() {
        let src = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub mycommand {arg1 arg2}\n# tcl-lsp: stubs-end\nmycommand x y\n";
        assert!(w123(src).is_empty());
    }

    #[test]
    fn alias_command_no_w123_and_alias_in_did_you_mean() {
        assert!(w123("interp alias {} = {} expr\n= {1 + 2}").is_empty());
        let d = w123("interp alias {} myput {} puts stdout\nmyptu hello");
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("myput"));
    }

    // --- did you mean? ---

    #[test]
    fn did_you_mean_suggestion_for_near_name() {
        // `putz` is one edit from `puts` (a real builtin).
        let r = Analyser::new().analyse("putz hello", D);
        let d: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W123")
            .collect();
        assert_eq!(d.len(), 1);
        assert!(d[0].message.contains("puts"));
        assert!(!d[0].fixes.is_empty());
        assert_eq!(d[0].fixes[0].new_text, "puts");
    }

    #[test]
    fn no_suggestion_for_distant_name() {
        let d = w123("xyzzyplugh arg1");
        assert_eq!(d.len(), 1);
        assert!(!d[0].contains("did you mean"));
    }

    // --- unknown-proc dispatch analysis ---

    #[test]
    fn unknown_proc_switch_dispatch_covers_targets() {
        let handled = "proc unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n        bar { puts bar }\n    }\n}\nfoo x\nbar y\n";
        assert!(w123(handled).is_empty());
        let unhandled = "proc unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n    }\n}\nfoo x\nbaz y\n";
        let d = w123(unhandled);
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("baz"));
    }

    #[test]
    fn unknown_proc_empty_stub_does_not_suppress() {
        let d = w123("proc unknown {args} {}\nmycommand arg1");
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("mycommand"));
    }

    #[test]
    fn unknown_proc_opaque_dispatch_suppresses() {
        // chains-original / auto_load / exec / glob-switch make dispatch opaque.
        for src in [
            "proc unknown {cmd args} {\n    _original_unknown $cmd {*}$args\n}\nmycommand arg1\n",
            "proc unknown {cmd args} {\n    auto_load $cmd\n}\nmycommand arg1\n",
            "proc unknown {cmd args} {\n    exec $cmd {*}$args\n}\nmycommand arg1\n",
            "proc unknown {cmd args} {\n    switch -glob $cmd {\n        fo* { puts foo }\n    }\n}\nfoobar x\n",
        ] {
            assert!(w123(src).is_empty(), "unexpected W123 for {src:?}");
        }
    }

    #[test]
    fn unknown_if_else_chains_original_suppresses() {
        let src = "proc unknown {cmd args} {\n    if {$cmd eq \"foo\"} {\n        puts foo\n    } else {\n        _original_unknown $cmd {*}$args\n    }\n}\nbaz x\n";
        assert!(w123(src).is_empty());
        let r = Analyser::new().analyse(src, D);
        let upi = r.unknown_proc_info.as_ref().expect("upi");
        assert!(upi.chains_original);
    }

    #[test]
    fn multiple_unknown_procs_last_wins() {
        let src = "proc unknown {cmd args} {\n    _original_unknown $cmd {*}$args\n}\nproc unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n    }\n}\nfoo x\nbaz y\n";
        let r = Analyser::new().analyse(src, D);
        let upi = r.unknown_proc_info.as_ref().expect("upi");
        assert!(!upi.chains_original);
        assert!(upi.dispatch_targets.contains("foo"));
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code.to_string() == "W123" && d.message.contains("baz"))
        );
    }

    // --- unknown_proc_info population ---

    #[test]
    fn unknown_proc_info_populated_for_switch_dispatch() {
        let src = "proc unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n        bar { puts bar }\n    }\n}\n";
        let r = Analyser::new().analyse(src, D);
        let upi = r.unknown_proc_info.as_ref().expect("upi");
        assert!(upi.dispatch_targets.contains("foo"));
        assert!(upi.dispatch_targets.contains("bar"));
        assert!(!upi.empty_stub);
        assert!(!upi.chains_original);
    }

    #[test]
    fn unknown_proc_info_empty_stub_flag() {
        let r = Analyser::new().analyse("proc unknown {args} {}", D);
        assert!(r.unknown_proc_info.as_ref().expect("upi").empty_stub);
    }

    #[test]
    fn no_unknown_proc_info_by_default() {
        assert!(
            Analyser::new()
                .analyse("set x 1", D)
                .unknown_proc_info
                .is_none()
        );
    }

    #[test]
    fn unknown_glob_switch_sets_pattern_dispatch_flag() {
        let src = "proc unknown {cmd args} {\n    switch -glob $cmd {\n        fo* { puts foo }\n    }\n}\nfoobar x\n";
        let r = Analyser::new().analyse(src, D);
        let upi = r.unknown_proc_info.as_ref().expect("upi");
        assert!(upi.has_pattern_dispatch);
        assert!(!r.diagnostics.iter().any(|d| d.code.to_string() == "W123"));
    }

    #[test]
    fn tcl_unknown_qualified_recognised() {
        let src = "proc ::tcl::unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n    }\n}\n";
        let r = Analyser::new().analyse(src, D);
        let upi = r.unknown_proc_info.as_ref().expect("upi");
        assert!(upi.dispatch_targets.contains("foo"));
    }

    // --- dead/live short-circuit arms ---
    // The analyser surfaces W123 for an unknown command like `[missingCommand]`
    // even inside a provably-dead `&&`/`||`/`?:` arm (which tclsh never
    // executes). The *live*-arm control — where tclsh genuinely errors — is
    // below.

    #[test]
    fn live_short_circuit_arm_command_still_w123() {
        // tclsh: `expr {1 && [missingCommand]}` runs the right arm and errors.
        assert_eq!(w123("expr {1 && [missingCommand]}").len(), 1);
        assert_eq!(w123("expr {[missingCommand] && 1}").len(), 1);
        assert_eq!(
            w123("proc f {c} { expr {$c && [missingCommand]} }").len(),
            1
        );
    }
}

// ===========================================================================
// `source` command extraction.
// ===========================================================================
mod source_targets {
    use super::*;

    #[test]
    fn literal_source_recorded() {
        let r = Analyser::new().analyse("source lib/utils.tcl", D);
        assert_eq!(r.source_targets.len(), 1);
        assert_eq!(r.source_targets[0].raw_path, "lib/utils.tcl");
        assert!(r.source_targets[0].is_literal);
    }

    #[test]
    fn variable_and_cmd_subst_sources_not_literal() {
        let r = Analyser::new().analyse("source $dir/utils.tcl", D);
        assert_eq!(r.source_targets.len(), 1);
        assert!(!r.source_targets[0].is_literal);
        let r2 = Analyser::new().analyse(
            "source [file join [file dirname [info script]] helper.tcl]",
            D,
        );
        assert_eq!(r2.source_targets.len(), 1);
        assert!(!r2.source_targets[0].is_literal);
    }

    #[test]
    fn source_with_encoding_option() {
        let r = Analyser::new().analyse("source -encoding utf-8 myfile.tcl", D);
        assert_eq!(r.source_targets.len(), 1);
        assert_eq!(r.source_targets[0].raw_path, "myfile.tcl");
        assert!(r.source_targets[0].is_literal);
    }

    #[test]
    fn multiple_sources_mixed_literal() {
        let r = Analyser::new().analyse("source a.tcl\nsource b.tcl\nsource $c", D);
        assert_eq!(r.source_targets.len(), 3);
        assert!(r.source_targets[0].is_literal);
        assert!(r.source_targets[1].is_literal);
        assert!(!r.source_targets[2].is_literal);
    }
}

// ===========================================================================
// TclOO class definition extraction.
// ===========================================================================
mod tcloo_classes {
    use super::*;
    use tcl_compiler::analyser::ClassDef;

    fn class(src: &str, qn: &str) -> ClassDef {
        class_in(src, qn, D)
    }

    /// [`class`] under an explicit dialect — for the members `TclOO` only
    /// gained in 9.0 (`classmethod`, `private`, `initialise`/`initialize`,
    /// `definitionnamespace`), which the 8.6 grammar rejects.
    fn class_in(src: &str, qn: &str, dialect: &str) -> ClassDef {
        Analyser::new()
            .analyse(src, dialect)
            .all_classes
            .get(qn)
            .cloned()
            .expect("class recorded")
    }

    #[test]
    fn class_create_basic() {
        let cd = class(
            "oo::class create Dog {\n    method bark {} { return \"woof\" }\n}\n",
            "::Dog",
        );
        assert_eq!(cd.name, "Dog");
        assert_eq!(cd.metaclass, "oo::class");
        assert!(cd.methods.contains_key("bark"));
        assert_eq!(cd.methods["bark"].kind, "method");
    }

    #[test]
    fn class_superclass() {
        let src = "oo::class create Animal {\n    method speak {} { error \"abstract\" }\n}\noo::class create Dog {\n    superclass Animal\n    method bark {} { return \"woof\" }\n}\n";
        assert_eq!(class(src, "::Dog").superclasses, ["Animal"]);
    }

    /// Integration (full analyse → resolved hierarchy): a bare
    /// `superclass Base` in a deeply-nested `::a::b::Sub`, where `Base` exists
    /// only in an *ancestor* namespace (`::a::Base`) and the tail is ambiguous
    /// (a second `::x::Base`), must NOT resolve — real Tcl errors there.  The
    /// former ancestor walk manufactured a `Sub ⊂ ::a::Base` edge.
    #[test]
    fn class_superclass_no_ancestor_namespace_walk() {
        let src = "namespace eval ::a {\n    oo::class create Base {}\n}\nnamespace eval ::x {\n    oo::class create Base {}\n}\nnamespace eval ::a::b {\n    oo::class create Sub {\n        superclass Base\n    }\n}\n";
        let analysis = Analyser::new().analyse(src, D).clone();
        let h = analysis.class_hierarchy();
        assert!(
            !h.is_subtype("::a::b::Sub", "::a::Base"),
            "must not manufacture an ancestor-namespace inheritance edge"
        );
    }

    /// TP: a base in the class's own namespace resolves, so
    /// the inheritance edge is real.
    #[test]
    fn class_superclass_same_namespace_resolves() {
        let src = "namespace eval ::a {\n    oo::class create Base {}\n    oo::class create Sub {\n        superclass Base\n    }\n}\n";
        let analysis = Analyser::new().analyse(src, D).clone();
        assert!(
            analysis
                .class_hierarchy()
                .is_subtype("::a::Sub", "::a::Base"),
            "a same-namespace superclass must resolve"
        );
    }

    #[test]
    fn class_variables_and_constructor() {
        let src = "oo::class create Dog {\n    variable name breed\n    constructor {n b} { set name $n; set breed $b }\n}\n";
        let cd = class(src, "::Dog");
        assert_eq!(cd.variables, ["name", "breed"]);
        assert_eq!(cd.constructors.len(), 1);
        assert_eq!(cd.constructors[0].kind, "constructor");
        assert_eq!(
            cd.constructors[0]
                .params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["n", "b"]
        );
    }

    #[test]
    fn class_method_params() {
        let src = "oo::class create Calc {\n    method add {a b} { expr {$a + $b} }\n}\n";
        let cd = class(src, "::Calc");
        assert_eq!(
            cd.methods["add"]
                .params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn class_destructor() {
        let cd = class(
            "oo::class create Conn {\n    destructor { close $fd }\n}\n",
            "::Conn",
        );
        let d = cd.destructor.as_ref().expect("destructor");
        assert_eq!(d.kind, "destructor");
    }

    #[test]
    fn class_mixins() {
        let cd = class(
            "oo::class create Dog {\n    mixin Serializable Comparable\n}\n",
            "::Dog",
        );
        assert_eq!(cd.mixins, ["Serializable", "Comparable"]);
    }

    #[test]
    fn class_forward() {
        let cd = class(
            "oo::class create Dog {\n    forward run ::dog::run_impl\n}\n",
            "::Dog",
        );
        assert_eq!(cd.methods["run"].kind, "forward");
    }

    #[test]
    fn class_filter_export_unexport() {
        let src = "oo::class create Dog {\n    method bark {} {}\n    method internal {} {}\n    filter myfilter\n    export bark\n    unexport internal\n}\n";
        let cd = class(src, "::Dog");
        assert_eq!(cd.filters, ["myfilter"]);
        assert!(cd.exports.contains("bark"));
        assert!(cd.unexports.contains("internal"));
    }

    #[test]
    fn oo_define_body_merges_methods() {
        // `classmethod` is 9.0-only, so this merge case runs under tcl9.0.
        let src = "oo::class create Dog {\n    method bark {} { return \"woof\" }\n}\noo::define Dog {\n    method fetch {item} { return $item }\n    classmethod count {} { return 0 }\n}\n";
        let cd = class_in(src, "::Dog", "tcl9.0");
        assert!(cd.methods.contains_key("bark"));
        assert!(cd.methods.contains_key("fetch"));
        assert!(cd.class_methods.contains_key("count"));
    }

    #[test]
    fn oo_define_inline_and_partial_class() {
        let cd = class(
            "oo::define Dog method sit {} { return \"sitting\" }",
            "::Dog",
        );
        assert!(cd.methods.contains_key("sit"));
        // oo::define on an unseen class creates a partial ClassDef.
        let cd2 = class(
            "oo::define MyClass {\n    method foo {} {}\n}\n",
            "::MyClass",
        );
        assert!(cd2.methods.contains_key("foo"));
    }

    #[test]
    fn metaclass_variants() {
        let cfg = class(
            "oo::configurable create Point {\n    property x y\n}\n",
            "::Point",
        );
        assert_eq!(cfg.metaclass, "oo::configurable");
        assert!(cfg.properties.contains_key("x") && cfg.properties.contains_key("y"));
        assert_eq!(
            class(
                "oo::abstract create Shape {\n    method area {} { error \"abstract\" }\n}\n",
                "::Shape"
            )
            .metaclass,
            "oo::abstract"
        );
        assert_eq!(
            class(
                "oo::singleton create Logger {\n    method log {msg} { puts $msg }\n}\n",
                "::Logger"
            )
            .metaclass,
            "oo::singleton"
        );
    }

    #[test]
    fn fully_qualified_oo_class_head_normalised() {
        let cd = class(
            "::oo::class create ::Dog {\n    method bark {} { return woof }\n}\n",
            "::Dog",
        );
        assert_eq!(cd.metaclass, "oo::class");
    }

    #[test]
    fn method_body_instance_variables_in_scope() {
        let src = "oo::class create Dog {\n    variable name\n    constructor {n} { set name $n }\n    method bark {} { return $name }\n}\n";
        let r = Analyser::new().analyse(src, D);
        let bark = r
            .global_scope
            .children
            .iter()
            .find(|s| s.name.ends_with("Dog::bark"))
            .expect("bark scope");
        assert!(bark.variables.contains_key("name"));
    }

    #[test]
    fn classmethod_and_private_method() {
        // Both members are 9.0-only (TIP 478); under 8.6 they draw W002
        // instead, which `oo_90_only_members_*` below pins.
        let cd = class_in(
            "oo::class create Counter {\n    classmethod instances {} { return 0 }\n}\n",
            "::Counter",
            "tcl9.0",
        );
        assert_eq!(cd.class_methods["instances"].kind, "classmethod");
        let cd2 = class_in(
            "oo::class create Foo {\n    private method helper {} { return 1 }\n}\n",
            "::Foo",
            "tcl9.0",
        );
        assert_eq!(cd2.methods["helper"].visibility, "private");
    }

    #[test]
    fn class_in_namespace_and_registered_on_scope() {
        let src = "namespace eval shapes {\n    oo::class create Circle {\n        variable radius\n        method area {} { expr {3.14 * $radius * $radius} }\n    }\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.all_classes.contains_key("::shapes::Circle"));
        let cd = &r.all_classes["::shapes::Circle"];
        assert_eq!(cd.variables, ["radius"]);
        assert!(cd.methods.contains_key("area"));
        // ClassDef registered on the enclosing scope by simple name.
        let r2 = Analyser::new().analyse("oo::class create Dog {\n    method bark {} {}\n}\n", D);
        assert!(r2.global_scope.classes.contains_key("Dog"));
    }

    #[test]
    fn self_method_body_is_walked_for_internal_diagnostics() {
        // TP — issue #923 idx 120 Part 1 bonus: before the fix, a
        // wrong-arity call inside a `self method`/`private method` body
        // drew no diagnostic at all (the body was never walked, only the
        // literal keywords "method"/"classmethod"/"constructor"/
        // "destructor" were recognised, "self"/"private" fell through
        // untouched). `string length` takes exactly one argument.
        let src = "oo::class create Widget {\n    self method make {n} {\n        string length a b c d\n        return \"made $n\"\n    }\n}\n";
        assert_eq!(
            count(src, D, "E003"),
            1,
            "the wrong-arity call inside a self-method body must now be walked: {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn private_method_body_is_also_walked_for_internal_diagnostics() {
        // TP — the sibling gap `private method` shared with `self method`
        // (both go through the same `unwrap_wrapper_member`-based
        // `collect_method_body`), confirming the fix isn't scoped to
        // `self` alone.
        // `private` is 9.0-only, so the body walk is exercised under tcl9.0.
        let src = "oo::class create Widget {\n    private method helper {} {\n        string length a b c d\n    }\n}\n";
        assert_eq!(
            count(src, "tcl9.0", "E003"),
            1,
            "the wrong-arity call inside a private-method body must now be walked: {:?}",
            codes(src, "tcl9.0")
        );
    }

    /// The `TclOO` definition members Tcl added in 9.0 — `classmethod`
    /// (TIP 478), `private`, `initialise`/`initialize`, and
    /// `definitionnamespace` (TIP 524) — do not exist in the 8.6 grammar.
    ///
    /// Oracle: each of the three call shapes below fails on real tclsh8.6
    /// with `invalid command name "<member>"` and succeeds on tclsh9.0
    /// (probes cm2 / cm3 / cm4). The registry's hover text already said
    /// "Tcl 9.0 also added the classmethod, private, initialise/initialize,
    /// and definitionnamespace subcommands (none exist under Tcl 8.6)"
    /// while the analyser accepted them silently.
    #[test]
    fn oo_90_only_members_are_flagged_under_86() {
        // Shape 1 — a member inside the metaclass's own `create` body.
        for member in [
            "classmethod count {} { return 0 }",
            "private { method secret {} { return s } }",
            "initialise { set n 1 }",
            "initialize { set n 1 }",
            "definitionnamespace ::mydefs",
        ] {
            let src =
                format!("oo::class create Dog {{\n    method bark {{}} {{}}\n    {member}\n}}\n");
            assert!(
                fires(&src, D, "W002"),
                "[create body] {member:?} must draw W002 under tcl8.6; got {:?}",
                codes(&src, D)
            );
            // Shape 2 — the same member inside an `oo::define Cls { … }` block.
            let block = format!(
                "oo::class create Dog {{ method bark {{}} {{}} }}\noo::define Dog {{\n    {member}\n}}\n"
            );
            assert!(
                fires(&block, D, "W002"),
                "[oo::define block] {member:?} must draw W002 under tcl8.6; got {:?}",
                codes(&block, D)
            );
            // Shape 3 — the single-command `oo::define Cls <member> …` form.
            let single = format!(
                "oo::class create Dog {{ method bark {{}} {{}} }}\noo::define Dog {member}\n"
            );
            assert!(
                fires(&single, D, "W002"),
                "[oo::define single] {member:?} must draw W002 under tcl8.6; got {:?}",
                codes(&single, D)
            );
        }
    }

    /// TN — the same three shapes are clean under tcl9.0, where the members
    /// exist.
    #[test]
    fn oo_90_only_members_are_clean_under_90() {
        for member in [
            "classmethod count {} { return 0 }",
            "private { method secret {} { return s } }",
            "initialise { set n 1 }",
            "definitionnamespace ::mydefs",
        ] {
            for src in [
                format!("oo::class create Dog {{\n    method bark {{}} {{}}\n    {member}\n}}\n"),
                format!(
                    "oo::class create Dog {{ method bark {{}} {{}} }}\noo::define Dog {{\n    {member}\n}}\n"
                ),
                format!(
                    "oo::class create Dog {{ method bark {{}} {{}} }}\noo::define Dog {member}\n"
                ),
            ] {
                assert!(
                    !fires(&src, "tcl9.0", "W002"),
                    "{member:?} exists in 9.0 and must be clean; got {:?}",
                    codes(&src, "tcl9.0")
                );
            }
        }
    }

    /// Reporting a 9.0-only member must not *erase* it: the analyser still
    /// records the member under 8.6, so go-to-definition, references,
    /// rename, document symbols, and code lenses keep working over the code
    /// the user actually wrote. Same contract as the whole-command W002,
    /// which reports a dialect-unavailable command while the analyser goes
    /// on modelling the call.
    #[test]
    fn a_gated_oo_member_is_reported_but_still_recorded() {
        let cd = class_in(
            "oo::class create Counter {\n    classmethod instances {} { return 0 }\n}\n",
            "::Counter",
            D,
        );
        assert!(
            cd.class_methods.contains_key("instances"),
            "the classmethod must still be recorded under tcl8.6; got {:?}",
            cd.class_methods.keys().collect::<Vec<_>>()
        );
        let cd2 = class_in(
            "oo::class create Dog { }\noo::define Dog classmethod count {} { return 0 }\n",
            "::Dog",
            D,
        );
        assert!(
            cd2.class_methods.contains_key("count"),
            "the single-command form's member must still be recorded; got {:?}",
            cd2.class_methods.keys().collect::<Vec<_>>()
        );
    }

    /// TN — the members that have existed since `TclOO` shipped in 8.6 are
    /// never gated, in any of the three shapes.
    #[test]
    fn always_available_oo_members_are_clean_under_86() {
        for member in [
            "method bark {} { return \"woof\" }",
            "constructor {n} { set n $n }",
            "destructor { return }",
            "variable name",
            "forward run puts",
        ] {
            for src in [
                format!("oo::class create Dog {{\n    {member}\n}}\n"),
                format!("oo::class create Dog {{ }}\noo::define Dog {{\n    {member}\n}}\n"),
                format!("oo::class create Dog {{ }}\noo::define Dog {member}\n"),
            ] {
                assert!(
                    !fires(&src, D, "W002"),
                    "{member:?} is 8.6 `TclOO` and must be clean; got {:?}",
                    codes(&src, D)
                );
            }
        }
    }

    #[test]
    fn self_method_declaration_site_recorded_as_a_classmethod() {
        // TP — proves Part 1 alone (record shape + span), independent of
        // the definition.rs-side call-site receiver-resolution bug Part 2
        // fixes.
        let cd = class(
            "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\n",
            "::Widget",
        );
        assert!(!cd.methods.contains_key("make"));
        let md = cd
            .class_methods
            .get("make")
            .expect("recorded as a classmethod");
        assert_eq!(md.kind, "classmethod");
        assert!(md.is_self_method);
    }
}

// ===========================================================================
// oo::Helpers::link — ClassDef::linked_members population (issue #923
// idx 113). Consumer-side (definition/hover resolution) is covered in
// tcl-lsp-core; this module is about `collect_oo_links` itself.
// ===========================================================================
mod oo_link {
    use super::*;
    use tcl_compiler::analyser::ClassDef;

    fn class(src: &str, qn: &str) -> ClassDef {
        Analyser::new()
            .analyse(src, D)
            .all_classes
            .get(qn)
            .cloned()
            .expect("class recorded")
    }

    #[test]
    fn single_name_link_records_alias_to_itself() {
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("foo").map(String::as_str),
            Some("foo")
        );
    }

    #[test]
    fn two_element_link_records_alias_to_a_different_target() {
        let cd = class(
            "oo::class create C {\n    constructor {} { link {shortcut realMethod} }\n    method realMethod {x} { return $x }\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("shortcut").map(String::as_str),
            Some("realMethod")
        );
    }

    #[test]
    fn link_called_from_destructor_is_recorded() {
        // TP — `collect_method_body` covers constructor/method/destructor
        // uniformly, and the link scan must too.
        let cd = class(
            "oo::class create C {\n    method foo {x} {return $x}\n    destructor { link foo }\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("foo").map(String::as_str),
            Some("foo")
        );
    }

    #[test]
    fn multiple_single_name_aliases_in_one_link_call_all_record() {
        // `link foo bar` is TWO independent one-element arguments, not a
        // single two-element pair — both must be recorded.
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo bar }\n    method foo {x} {return $x}\n    method bar {y} {return $y}\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("foo").map(String::as_str),
            Some("foo")
        );
        assert_eq!(
            cd.linked_members.get("bar").map(String::as_str),
            Some("bar")
        );
    }

    #[test]
    fn links_from_different_method_bodies_all_accumulate() {
        // Links recorded from different member bodies (not just the
        // constructor) must all accumulate onto the same ClassDef.
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo }\n    method setup {} { link bar }\n    method foo {x} {return $x}\n    method bar {y} {return $y}\n}\n",
            "::C",
        );
        assert_eq!(
            cd.linked_members.get("foo").map(String::as_str),
            Some("foo")
        );
        assert_eq!(
            cd.linked_members.get("bar").map(String::as_str),
            Some("bar")
        );
    }

    #[test]
    fn dynamic_link_target_is_not_recorded() {
        // TN — a dynamic `link $which` is skipped (mirrors
        // `detect_interp_alias`'s literal-only requirement): no
        // fabricated alias, conservative fallback to "not reachable".
        let cd = class(
            "oo::class create C {\n    variable which\n    constructor {w} { set which $w; link $which }\n    method foo {x} {return $x}\n}\n",
            "::C",
        );
        assert!(cd.linked_members.is_empty(), "{:?}", cd.linked_members);
    }

    #[test]
    fn link_for_one_name_does_not_blanket_legitimize_others() {
        // Precision guard — a link for a DIFFERENT name must not
        // blanket-legitimize every bareword in the class.
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo }\n    method foo {x} {return $x}\n    method other {y} {return $y}\n}\n",
            "::C",
        );
        assert!(cd.linked_members.contains_key("foo"));
        assert!(!cd.linked_members.contains_key("other"));
    }

    #[test]
    fn link_nested_inside_a_conditional_is_not_recorded() {
        // Deliberately shallow — only a top-level `link` call is
        // recognised, not one nested inside an `if`/`catch`/… body
        // argument (matches `scan_my_method_region`'s own accepted
        // scope boundary for this class of problem).
        let cd = class(
            "oo::class create C {\n    constructor {} { if {1} { link foo } }\n    method foo {x} {return $x}\n}\n",
            "::C",
        );
        assert!(cd.linked_members.is_empty(), "{:?}", cd.linked_members);
    }

    #[test]
    fn link_is_not_recorded_under_a_dialect_without_it() {
        // The head is recognised through the registry's
        // `TCLOO_BINDS_METHOD_ALIAS` trait, not by its spelling, so an 8.5
        // dialect — which has no TclOO at all, let alone `oo::Helpers::link`
        // — records nothing. (tclsh 8.6.14 without `ooutil` likewise:
        // `invalid command name "link"` even inside a method.)
        let cd = class(
            "oo::class create C {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n}\n",
            "::C",
        );
        assert!(cd.linked_members.contains_key("foo"));
        let cd85 = Analyser::new()
            .analyse(
                "oo::class create C {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n}\n",
                "tcl8.5",
            )
            .all_classes
            .get("::C")
            .cloned();
        assert!(
            cd85.is_none_or(|c| c.linked_members.is_empty()),
            "no `link` in the 8.5 registry, so no aliases",
        );
    }
}

// ===========================================================================
// The `oo::Helpers` family is method-context-scoped (issue #1026).
//
// tclsh 9.0.4, at the top level:
//     link foo          -> invalid command name "link"
//     my foo            -> invalid command name "my"
//     next / nextto     -> invalid command name "next" / "nextto"
//     self              -> invalid command name "self"
//     classvariable v   -> invalid command name "classvariable"
//     info commands ::link  -> {}   (empty)
// and inside `oo::class create C { method m {} { … } }`:
//     namespace current -> ::oo::Obj22       namespace path -> ::oo::Helpers
//     namespace which -command link -> ::oo::Helpers::link
//     namespace which -command my   -> ::oo::Obj22::my   (NOT a helper)
// while an `apply` lambda written inside that same method body loses the
// context entirely (`invalid command name "link"` / `"my"` / `"self"`).
//
// tclsh 8.6.14 agrees for the four members it has (`next`/`nextto`/`self`
// are `::oo::Helpers::*`, `my` is `::oo::ObjN::my`), and adding
// `::oo::Helpers::link` the way Tcllib's `ooutil` does still leaves the
// top-level bare `link` an `invalid command name`.
// ===========================================================================
mod oo_helpers_scoping {
    use super::*;

    /// Every family member, with the dialect that has it.
    const FAMILY: &[(&str, &str)] = &[
        ("link", "tcl9.0"),
        ("my", "tcl8.6"),
        ("next", "tcl8.6"),
        ("nextto", "tcl8.6"),
        ("self", "tcl8.6"),
        ("classvariable", "tcl9.0"),
    ];

    #[test]
    fn top_level_use_draws_w123() {
        for (word, dialect) in FAMILY {
            let src = format!("{word} foo\n");
            assert!(
                fires(&src, dialect, "W123"),
                "top-level `{word}` is `invalid command name` in real Tcl",
            );
        }
    }

    #[test]
    fn method_body_use_is_clean() {
        for (word, dialect) in FAMILY {
            let src = format!(
                "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    method m {{}} {{ {word} foo }}\n}}\n"
            );
            assert!(
                !fires(&src, dialect, "W123"),
                "`{word}` resolves inside a method body: {:?}",
                codes(&src, dialect),
            );
        }
    }

    #[test]
    fn constructor_destructor_and_class_side_bodies_are_clean() {
        // tclsh 9.0.4: `link`/`self`/`my` all work in a constructor, a
        // destructor, a `self method` body, and an `oo::objdefine method`
        // body.
        for (word, dialect) in FAMILY {
            for body in [
                format!(
                    "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    constructor {{}} {{ {word} foo }}\n}}\n"
                ),
                format!(
                    "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    destructor {{ {word} foo }}\n}}\n"
                ),
                format!(
                    "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    self method cm {{}} {{ {word} foo }}\n}}\n"
                ),
                format!(
                    "oo::class create C {{}}\nC create c1\noo::objdefine c1 {{\n    method om {{}} {{ {word} foo }}\n}}\n"
                ),
            ] {
                assert!(
                    !fires(&body, dialect, "W123"),
                    "`{word}` resolves in every method context: {:?}\n{body}",
                    codes(&body, dialect),
                );
            }
        }
    }

    #[test]
    fn apply_lambda_inside_a_method_body_draws_w123() {
        // TP — `apply` runs its body in the global namespace, so the object
        // context is gone (tclsh 9.0.4, inside a method:
        // `apply {{} { link Helper }}` -> `invalid command name "link"`).
        for (word, dialect) in FAMILY {
            let src = format!(
                "oo::class create C {{\n    method foo {{}} {{ return 1 }}\n    method m {{}} {{ apply {{{{}} {{ {word} foo }}}} }}\n}}\n"
            );
            assert!(
                fires(&src, dialect, "W123"),
                "an apply lambda loses the method context: {:?}",
                codes(&src, dialect),
            );
        }
    }

    #[test]
    fn a_linked_bareword_resolves_inside_the_objects_method_bodies() {
        // TN — `link foo` installs a real command `foo` in the object's own
        // namespace that dispatches `my foo` (tclsh 9.0.4: after `link
        // Helper` in a constructor, `namespace which -command Helper`
        // answers `::oo::ObjN::Helper` from every method body), so a later
        // bare `foo 1` is not an unknown command.
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n";
        assert!(!fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }

    #[test]
    fn an_unlinked_sibling_method_bareword_still_draws_w123() {
        // TP — `link` for one name must not blanket-legitimise every
        // bareword in the class: tclsh 9.0.4 raises `invalid command name
        // "other"` for an un-linked sibling method called bare.
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method other {} { return 2 }\n    method bar {} {\n        link foo\n        return [other]\n    }\n}\n";
        assert!(fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }

    #[test]
    fn a_linked_bareword_still_draws_w123_at_the_top_level() {
        // TP — the alias lives in the object's namespace, so it is not
        // callable from outside a method body.
        let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} { link foo }\n}\nfoo 1\n";
        assert!(fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }

    #[test]
    fn proc_body_draws_w123() {
        // TP — an ordinary proc is not a method context, however it is
        // nested.
        for (word, dialect) in FAMILY {
            let src = format!("proc helper {{}} {{ {word} foo }}\n");
            assert!(fires(&src, dialect, "W123"), "`{word}` in a plain proc");
        }
    }

    #[test]
    fn qualified_spelling_resolves_anywhere() {
        // TN — `::oo::Helpers::link` is a genuine global command
        // (`info commands ::oo::Helpers::link` answers it under tclsh
        // 9.0.4); calling it outside a method is a *runtime* error, not an
        // unknown command.
        for spelling in [
            "::oo::Helpers::link foo",
            "oo::Helpers::link foo",
            "::oo::Helpers::self",
            "::oo::Helpers::classvariable v",
        ] {
            let src = format!("{spelling}\n");
            assert!(
                !fires(&src, "tcl9.0", "W123"),
                "{spelling}: {:?}",
                codes(&src, "tcl9.0"),
            );
        }
    }

    #[test]
    fn a_user_proc_of_the_same_name_still_resolves_at_the_top_level() {
        // FP guard — the scope rule must not swallow a real, user-defined
        // global command that happens to share the name.
        let src = "proc link {args} { return $args }\nlink foo\n";
        assert!(!fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }

    /// A class-level `initialise` / `initialize` body, both spellings.
    fn init_body(word: &str, spelling: &str) -> String {
        format!(
            "oo::class create C {{\n    {spelling} {{\n        {word} foo\n    }}\n    method m {{}} {{ return 1 }}\n}}\n"
        )
    }

    /// **W123 keys on resolution, and the family genuinely resolves in a
    /// class `initialise` body** — so it must stay silent there (Codex
    /// review of PR #1084).
    ///
    /// tclsh 9.0.4, inside `oo::class create ::P { initialize { … } }`:
    /// `namespace current` is `::oo::Obj20`, `namespace path` is
    /// `::oo::Helpers ::oo`, and `namespace which -command link` answers
    /// `::oo::Helpers::link` — exactly as it does in a method body. Warning
    /// "unknown command" here would be a plain false positive.
    #[test]
    fn init_body_use_draws_no_w123() {
        for (word, _) in FAMILY {
            for spelling in ["initialize", "initialise"] {
                let src = init_body(word, spelling);
                assert!(
                    !fires(&src, "tcl9.0", "W123"),
                    "`{word}` resolves in an `{spelling}` body: {:?}",
                    codes(&src, "tcl9.0"),
                );
            }
        }
    }

    /// ...but **callability** is a different fact, and the two diverge in
    /// exactly this body.
    ///
    /// Same interpreter, same frame: every `::oo::Helpers` member raises
    /// `… may only be called from inside a method` when actually called,
    /// while `my` runs — it is `::oo::Obj20::my`, the *class object's* own
    /// dispatch command, and `my new` there really does make an instance.
    /// So the scope tree must mark the init frame as "resolves, but not a
    /// method frame".
    #[test]
    fn init_body_is_not_a_method_frame_but_still_reaches_oo_helpers() {
        use tcl_compiler::analyser::{
            innermost_scope_is_oo_method_frame, innermost_scope_reaches_oo_helpers,
        };
        let src = init_body("link", "initialize");
        let mut a = Analyser::new();
        let result = a.analyse(&src, "tcl9.0").clone();
        let init_off = u32::try_from(src.find("link foo").expect("the init-body call")).unwrap();
        let method_off = u32::try_from(src.find("return 1").expect("the method body")).unwrap();
        assert!(
            innermost_scope_reaches_oo_helpers(&result.global_scope, init_off),
            "an `initialize` body has ::oo::Helpers on its namespace path",
        );
        assert!(
            !innermost_scope_is_oo_method_frame(&result.global_scope, init_off),
            "an `initialize` body is not a method invocation",
        );
        // A real method body answers `true` to both, and the top level to
        // neither — the two predicates must not have collapsed into one.
        assert!(innermost_scope_reaches_oo_helpers(
            &result.global_scope,
            method_off
        ));
        assert!(innermost_scope_is_oo_method_frame(
            &result.global_scope,
            method_off
        ));
        assert!(!innermost_scope_reaches_oo_helpers(&result.global_scope, 0));
        assert!(!innermost_scope_is_oo_method_frame(&result.global_scope, 0));
    }

    /// The per-class `initialise` scoping that issue #923 idx 36 added must
    /// survive being marked "not a method frame": two sibling classes'
    /// same-named class variables stay independent.
    #[test]
    fn init_body_scoping_still_separates_sibling_classes() {
        let src = "oo::class create A {\n    initialize { variable Colours {red} }\n}\noo::class create B {\n    initialize { variable Colours {blue} }\n}\n";
        assert!(!fires(src, "tcl9.0", "W123"), "{:?}", codes(src, "tcl9.0"));
    }
}

// ===========================================================================
// Diagnostics keyed on command name must hit identically across bare /
// qualified / aliased spellings.
//
// Several of these equalities hold (W211 set, W215/W216, the W210
// variable/upvar/global suppressions); others do NOT — qualified `::unset` and
// aliased `unset` do not currently canonicalise to the W213 path. Each arm is
// asserted against the actual verdict, with the exceptions flagged inline.
// ===========================================================================
mod canonicalisation_matrix {
    use super::*;

    // --- W215 (unreachable variable name) ---

    #[test]
    fn w215_brace_in_var_name() {
        // `set "weird}name" 1` makes a var no $-form can read. Heuristic, but the
        // unreachability is verified against tclsh's ${...} parser semantics.
        let ds = analyser_diags("set \"weird}name\" 1", D);
        let w215: Vec<_> = ds.iter().filter(|(c, _, _)| c == "W215").collect();
        assert_eq!(w215.len(), 1);
        assert!(w215[0].1.contains("weird}name"));
        assert!(w215[0].1.contains('}'));
    }

    #[test]
    fn w215_trailing_backslash_in_var_name() {
        // `set "back\\" 1` creates `back\`. Under the Tcl 9 nesting rule
        // `${back\}` reads the `\}` as an inert pair and runs out of input
        // (missing close-brace) — unreachable, W215 fires (verified against
        // tclsh 9.0.3). Under the 8.x first-`}` rule there is no backslash
        // processing: `${back\}` simply names `back\` — reachable, silent.
        let ds = analyser_diags("set \"back\\\\\" 1", "tcl9.0");
        let w215: Vec<_> = ds.iter().filter(|(c, _, _)| c == "W215").collect();
        assert_eq!(w215.len(), 1);
        assert!(w215[0].1.contains("trailing") || w215[0].1.contains("missing close-brace"));

        assert!(
            !fires("set \"back\\\\\" 1", D, "W215"),
            "8.x reads ${{back\\}} as the name back\\ — reachable, no W215"
        );
    }

    #[test]
    fn w215_does_not_fire_when_brace_form_reaches_the_name() {
        // Names reachable via ${...} under BOTH `${…}` delimiting rules
        // (Tcl 9 nesting and the 8.x first-`}` scan): mid-name backslash
        // (no `}` involved) and ordinary names.
        for src in [
            "set \"back\\\\slash\" 1",
            "set \"foo-bar\" 1\nset normal 2\nset ::globalvar 3\nset arr(name) 4",
        ] {
            for dialect in [D, "tcl9.0"] {
                assert!(
                    !fires(src, dialect, "W215"),
                    "unexpected W215 for {src:?} under {dialect}"
                );
            }
        }
    }

    #[test]
    fn w215_balanced_inner_braces_are_dialect_dependent() {
        // A name with balanced inner braces is reachable only under the Tcl 9
        // nesting rule. tclsh8.6-verified: `set "a{b}c" 1; set ${a{b}c}` →
        // can't read "a{b" (the 8.x form ends at the FIRST `}`), while tclsh
        // 9.0.3 reads it fine.
        let src = "set \"a{b}c\" 1";
        assert!(
            !fires(src, "tcl9.0", "W215"),
            "Tcl 9 nesting reaches a{{b}}c — no W215"
        );
        assert!(
            fires(src, D, "W215"),
            "the 8.x first-close rule cannot reach a{{b}}c — W215 must fire"
        );
    }

    #[test]
    fn w215_close_paren_in_array_index() {
        let ds = analyser_diags("set \"arr(weird)stuff)\" 1", D);
        let w215: Vec<_> = ds.iter().filter(|(c, _, _)| c == "W215").collect();
        assert_eq!(w215.len(), 1);
        assert!(w215[0].1.contains("array element index contains ')'"));
    }

    // --- W216 (brace-then-paren array misuse) ---

    #[test]
    fn w216_brace_then_paren_with_fix() {
        let r = Analyser::new().analyse("set arr(name) hello\nputs ${arr}(name)", D);
        let w216: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W216")
            .collect();
        assert_eq!(w216.len(), 1);
        assert!(!w216[0].fixes.is_empty());
        assert_eq!(w216[0].fixes[0].new_text, "$arr(name)");
    }

    #[test]
    fn w216_brace_array_with_dollar_index() {
        let r = Analyser::new().analyse("set foo bar\nputs ${arr($foo)}", D);
        let w216: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W216")
            .collect();
        assert_eq!(w216.len(), 1);
        assert_eq!(w216[0].fixes[0].new_text, "$arr($foo)");
    }

    #[test]
    fn w216_funny_name_falls_back_to_set_indirection() {
        let r = Analyser::new().analyse("set \"funny name\" 1\nputs ${funny name($foo)}", D);
        let w216: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W216")
            .collect();
        assert_eq!(w216.len(), 1);
        assert_eq!(w216[0].fixes[0].new_text, "[set \"funny name($foo)\"]");
    }

    #[test]
    fn w216_does_not_fire_on_correct_forms() {
        for src in [
            "puts ${arr(name)}",
            "puts $arr($foo)",
            "puts ${arr}",
            "set foo bar\nputs $arr($foo)",
        ] {
            assert!(!fires(src, D, "W216"), "unexpected W216 for {src:?}");
        }
    }

    // --- W213 (unset of possibly-undefined var) ---

    #[test]
    fn w213_unset_bare_fires() {
        // tclsh: `unset x` on undefined x → can't unset "x": no such variable.
        assert_eq!(count("proc f {} { unset x }", D, "W213"), 1);
    }

    #[test]
    fn w213_nocomplain_silenced() {
        // `-nocomplain` opts out (tclsh suppresses the error too).
        assert_eq!(count("proc f {} { unset -nocomplain x }", D, "W213"), 0);
    }

    // Qualified `::unset x` and aliased `myunset x` do NOT fire W213 — the
    // qualified / aliased spelling is not canonicalised onto the W213 path on
    // this surface.
    #[test]
    fn w213_qualified_and_aliased_do_not_fire_rust_behaviour() {
        // The qualified / aliased spellings stay silent:
        assert_eq!(count("proc f {} { ::unset x }", D, "W213"), 0);
        assert_eq!(
            count(
                "interp alias {} myunset {} unset\nproc f {} { myunset x }",
                D,
                "W213"
            ),
            0
        );
        // The qualified+nocomplain combination is likewise silent.
        assert_eq!(count("proc f {} { ::unset -nocomplain x }", D, "W213"), 0);
    }

    // --- W211 (set but never used) canonicalisation ---

    #[test]
    fn w211_set_bare_and_qualified_both_fire() {
        // Both bare `set y 1` and qualified `::set y 1` canonicalise to the W211
        // path (this equality DOES hold in Rust).
        assert_eq!(count("proc f {} { set y 1 }", D, "W211"), 1);
        assert_eq!(count("proc f {} { ::set y 1 }", D, "W211"), 1);
    }

    // An aliased `s y 1` (→ set) does not fire W211 on this surface.
    #[test]
    fn w211_aliased_set_rust_behaviour() {
        let n = count("interp alias {} s {} set\nproc f {} { s y 1 }", D, "W211");
        // Record the actual count; the alias-rewrite-to-`set` W211 path is not
        // reproduced here.
        assert_eq!(
            n, 0,
            "aliased-set W211 not reproduced on this surface (divergence)"
        );
    }

    // --- W210 suppression by variable/upvar/global declarations ---

    #[test]
    fn w210_variable_decl_silences_bare_and_qualified() {
        // `variable v` introduces v from the namespace → reading it is not W210.
        for decl in ["variable v", "::variable v"] {
            let src =
                format!("namespace eval ns {{ variable v; proc f {{}} {{ {decl}; puts $v }} }}");
            assert_eq!(w210_for(&src, "'v'"), 0, "for {decl:?}");
        }
    }

    #[test]
    fn w210_upvar_decl_silences_bare_and_qualified() {
        // `upvar 1 caller_v local` aliases `local`; reads are not W210. Both the
        // bare and `::upvar` qualified spellings canonicalise onto this path.
        assert_eq!(
            w210_for(
                "proc f {} { upvar 1 caller_v local; puts $local }",
                "'local'"
            ),
            0
        );
        assert_eq!(
            w210_for(
                "proc f {} { ::upvar 1 caller_v local; puts $local }",
                "'local'"
            ),
            0
        );
    }

    #[test]
    fn w210_upvar_aliased_does_not_silence_rust_behaviour() {
        // An aliased `upvar` (`interp alias {} link {} upvar` then
        // `link 1 caller_v local`) is not recognised by the W210-suppression
        // path, so reading `local` DOES fire W210.
        assert_eq!(
            w210_for(
                "interp alias {} link {} upvar\nproc f {} { link 1 caller_v local; puts $local }",
                "'local'"
            ),
            1
        );
    }

    #[test]
    fn w210_global_decl_silences_bare_qualified_aliased() {
        // `global g` recognises g as a global alias → no W210 on g.
        for src in [
            "set g 0\nproc f {} { global g; set g 1 }",
            "set g 0\nproc f {} { ::global g; set g 1 }",
            "set g 0\ninterp alias {} mkglobal {} global\nproc f {} { mkglobal g; set g 1 }",
        ] {
            assert_eq!(w210_for(src, "'g'"), 0, "unexpected W210 'g' for {src:?}");
        }
    }

    // NOTE — IRULE4005 / W120 canonicalisation: these poke iRules-flow /
    // global-write internals. The IRULE4005 case asserts a *negative*
    // (bare/qualified/aliased `unset` does NOT fire IRULE4005); the W120 case
    // asserts the global-alias suppression already covered by
    // `w210_global_decl_silences_*` above. The IRULE4005 negative is covered
    // here for the bare spelling under the iRules dialect.

    #[test]
    fn irule4005_not_fired_by_bare_unset_under_irules() {
        // A bare `unset static::z` is not a "real write" → no IRULE4005.
        assert!(!fires(
            "when CLIENT_ACCEPTED { unset static::z }",
            "f5-irules",
            "IRULE4005"
        ));
    }
}

// ===========================================================================
// Issue #806 — report::defstyle scoped command environment.
//
// The style script exposes the report configuration methods (`top`, `data`,
// `columns`, …) as commands available only inside the body.  The registry-
// driven scoped environment resolves them there (no W123 / arity false
// positives) while still catching genuine typos and misuse (TP), and keeps
// them unknown outside the body (correct scoping).  Organised as a
// TP / FP / TN / FN matrix.
// ===========================================================================
mod report_scoped_commands {
    use super::*;

    fn codes_of(src: &str) -> Vec<String> {
        codes(src, D)
    }
    fn w123(src: &str) -> Vec<String> {
        analyser_diags(src, D)
            .into_iter()
            .filter(|(c, _, _)| c == "W123")
            .map(|(_, m, _)| m)
            .collect()
    }

    // ---- TN: valid scoped usage draws no unknown-command / arity error ----

    #[test]
    fn tn_valid_body_no_w123() {
        // The exact shape from the issue screenshot: line codes + operations.
        let src = "::report::defstyle simpletable {} {\n\
                   \x20 top set [split \"x\"]\n\
                   \x20 data set [split \"y\"]\n\
                   \x20 bottom enable\n\
                   \x20 topdatasep enable\n\
                   \x20 columns\n\
                   }\n";
        assert!(
            w123(src).is_empty(),
            "no W123 inside a valid style body: {:?}",
            w123(src)
        );
        assert!(!fires(src, D, "E001"));
        assert!(!fires(src, D, "E002"));
        assert!(!fires(src, D, "E003"));
        assert!(!fires(src, D, "W001"));
    }

    #[test]
    fn tn_nested_substitution_scoped_command_resolves() {
        // `[columns]` nested inside a `set` value is still a scoped command.
        let src = "::report::defstyle st {} {\n\
                   \x20 top set [string repeat \"= \" [columns]]\n\
                   }\n";
        assert!(
            w123(src).is_empty(),
            "columns in a substitution resolves: {:?}",
            w123(src)
        );
    }

    #[test]
    fn tn_sibling_style_resolves() {
        // A later style body may invoke a previously-defined style by name.
        let src = "::report::defstyle simpletable {} {\n\
                   \x20 top enable\n\
                   }\n\
                   ::report::defstyle captionedtable {n} {\n\
                   \x20 simpletable\n\
                   \x20 tcaption $n\n\
                   }\n";
        assert!(
            w123(src).is_empty(),
            "sibling style resolves: {:?}",
            w123(src)
        );
    }

    #[test]
    fn tn_config_methods_valid_arity() {
        let src = "::report::defstyle st {} {\n\
                   \x20 size 0 10\n\
                   \x20 size 1 dyn\n\
                   \x20 pad 0 both { }\n\
                   \x20 justify 0 center\n\
                   \x20 tcaption 1\n\
                   \x20 top get\n\
                   }\n";
        for code in ["W123", "E001", "E002", "E003", "W001"] {
            assert!(
                !fires(src, D, code),
                "{code} should not fire: {:?}",
                codes_of(src)
            );
        }
    }

    // ---- TP: genuine errors inside the body are still reported ----

    #[test]
    fn tp_typo_command_flagged() {
        // `toop` / `dataa` are not scoped commands → still W123.
        let src = "::report::defstyle st {} {\n  toop set x\n  dataa set y\n}\n";
        let w = w123(src);
        assert!(
            w.iter().any(|m| m.contains("toop")),
            "typo `toop` flagged: {w:?}"
        );
        assert!(
            w.iter().any(|m| m.contains("dataa")),
            "typo `dataa` flagged: {w:?}"
        );
    }

    #[test]
    fn tp_unknown_operation_flagged_w001() {
        let src = "::report::defstyle st {} {\n  top bogus\n}\n";
        assert!(
            fires(src, D, "W001"),
            "unknown op `top bogus` → W001: {:?}",
            codes_of(src)
        );
    }

    #[test]
    fn tp_bare_ensemble_requires_operation_e001() {
        let src = "::report::defstyle st {} {\n  top\n}\n";
        assert!(
            fires(src, D, "E001"),
            "bare `top` → E001: {:?}",
            codes_of(src)
        );
    }

    #[test]
    fn tp_operation_too_few_args_e002() {
        // `top set` needs the template value.
        let src = "::report::defstyle st {} {\n  top set\n}\n";
        assert!(
            fires(src, D, "E002"),
            "`top set` (no value) → E002: {:?}",
            codes_of(src)
        );
    }

    #[test]
    fn tp_plain_command_too_many_args_e003() {
        // `columns` takes no arguments.
        let src = "::report::defstyle st {} {\n  columns extra\n}\n";
        assert!(
            fires(src, D, "E003"),
            "`columns extra` → E003: {:?}",
            codes_of(src)
        );
    }

    // ---- FP guard: the scoped env must not wrongly suppress real code ----

    #[test]
    fn fp_core_commands_still_checked_in_body() {
        // Core commands inside the body keep their normal arity checks — a
        // scoped env must not swallow them.  `set` with one arg is fine; a
        // genuinely unknown core-looking head is still W123.
        let src = "::report::defstyle st {} {\n  set x 1\n  frobnicate a b\n}\n";
        assert!(
            w123(src).iter().any(|m| m.contains("frobnicate")),
            "unknown non-scoped head still W123: {:?}",
            w123(src)
        );
    }

    // ---- FN guard / scoping: scoped commands are unknown OUTSIDE the body ----

    #[test]
    fn fn_scoped_command_unknown_outside_body() {
        // `top` / `columns` at top level are not real commands.
        let src = "top set x\ncolumns\n";
        let w = w123(src);
        assert!(
            w.iter().any(|m| m.contains("top")),
            "`top` unknown outside body: {w:?}"
        );
        assert!(
            w.iter().any(|m| m.contains("columns")),
            "`columns` unknown outside body: {w:?}"
        );
    }

    // ---- report namespace + object commands ----

    #[test]
    fn report_namespace_commands_known() {
        // The dedicated specs make the introspection commands resolvable
        // (only W120 missing-require remains, which is correct).
        let src = "::report::styles\n::report::rmstyle foo\n::report::stylebody foo\n\
                   ::report::stylearguments foo\n";
        assert!(
            w123(src).is_empty(),
            "report namespace commands known: {:?}",
            w123(src)
        );
    }

    #[test]
    fn report_object_methods_resolve() {
        // `report::report` binds `r` as an object command; `r <method>` resolves
        // through the registry object class — no W123 on `r`.
        let src = "package require report\n::report::report r 3\nr data set x\nr printmatrix m\n";
        assert!(
            w123(src).is_empty(),
            "report object methods resolve: {:?}",
            w123(src)
        );
    }
}

// ===========================================================================
// Class factories and dynamically-installed members — issue #923 audit
// cluster C3 (idx 43/44/53/55/96/97).
//
// C-Tcl ground truth for every case below comes from tclsh 9.0.4 and
// tclsh 8.6.16, which agree on all of them:
//
// * `oo::class create Megawidget { superclass oo::class; self method create
//   {name superclasses body} { next $name [list superclass MegawidgetClass
//   {*}$superclasses]\;$body } }` then `Megawidget create SimpleWidget {}
//   {…}` / `Megawidget create FocusableWidget SimpleWidget {…}` /
//   `Megawidget create IconList FocusableWidget {…}` report
//   `info class superclasses` = `::MegawidgetClass`,
//   `::MegawidgetClass ::SimpleWidget`, and
//   `::MegawidgetClass ::FocusableWidget` respectively, and `my CreateHull`
//   from an `IconList` instance really runs `FocusableWidget`'s override.
// * `oo::define S { method {*}{foo {} {return foo-ran}} }` defines a real,
//   callable `foo`; so does the same `{*}` form inside an `oo::class create`
//   body.  `constructor {*}[info class constructor ::Base]` /
//   `method $m {*}[info class definition ::Base $m]` are equally real but
//   carry no statically-knowable element list.
// * `set ns tc; ${ns}::setdef …` dispatches to `::tc::setdef`.
// ===========================================================================
mod class_factories {
    use super::*;
    use tcl_compiler::analyser::{AnalysisResult, ClassDef};

    fn analysis(src: &str, dialect: &str) -> AnalysisResult {
        Analyser::new().analyse(src, dialect)
    }

    fn class(src: &str, qn: &str) -> ClassDef {
        analysis(src, "tcl9.0")
            .all_classes
            .get(qn)
            .cloned()
            .unwrap_or_else(|| panic!("class {qn} recorded"))
    }

    /// The Tk `library/megawidget.tcl` factory, reduced to its mechanism.
    const MEGAWIDGET: &str = concat!(
        "oo::class create Megawidget {\n",
        "    superclass oo::class\n",
        "    self method create {name superclasses body} {\n",
        "        next $name [list superclass MegawidgetClass {*}$superclasses]\\;$body\n",
        "    }\n",
        "}\n",
        "oo::class create MegawidgetClass {\n",
        "    method TraceOption {a b} { return traced }\n",
        "}\n",
        "Megawidget create SimpleWidget {} {\n",
        "    method CreateHull {} { my TraceOption a b }\n",
        "}\n",
        "Megawidget create FocusableWidget SimpleWidget {\n",
        "    method CreateHull {} { my TraceOption c d }\n",
        "}\n",
        "Megawidget create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
    );

    #[test]
    fn user_metaclass_creates_real_classes() {
        // TP — idx 96/97: a class whose own superclass chain reaches
        // `oo::class` is a class factory, so its `create` calls introduce
        // real classes.  Before this they never entered `all_classes` at
        // all: no outline entry, no references, no `next` resolution.
        let r = analysis(MEGAWIDGET, "tcl9.0");
        for name in ["::SimpleWidget", "::FocusableWidget", "::IconList"] {
            assert!(
                r.all_classes.contains_key(name),
                "{name} recorded: {:?}",
                r.all_classes.keys().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn user_metaclass_body_argument_is_read_from_the_create_override() {
        // TP — the override's signature is `{name superclasses body}`, so
        // the body is argument 3, not the builtin `create Name Body`
        // argument 2.  Reading the builtin position would walk the
        // superclass word as a script and lose every member.
        assert!(
            class(MEGAWIDGET, "::SimpleWidget")
                .methods
                .contains_key("CreateHull")
        );
        assert!(
            class(MEGAWIDGET, "::IconList")
                .methods
                .contains_key("GetSpecs")
        );
    }

    #[test]
    fn user_metaclass_splices_its_own_and_the_callers_superclasses() {
        // TP — matches `info class superclasses` on tclsh 9.0.4 / 8.6.16
        // exactly, including the implicit `MegawidgetClass` the factory
        // injects and the caller-supplied list spliced by `{*}$superclasses`.
        assert_eq!(
            class(MEGAWIDGET, "::SimpleWidget").superclasses,
            ["MegawidgetClass"]
        );
        assert_eq!(
            class(MEGAWIDGET, "::FocusableWidget").superclasses,
            ["MegawidgetClass", "SimpleWidget"]
        );
        assert_eq!(
            class(MEGAWIDGET, "::IconList").superclasses,
            ["MegawidgetClass", "FocusableWidget"]
        );
    }

    #[test]
    fn a_factory_made_class_with_a_readable_override_is_not_opaque() {
        // TN — inheritance is fully known here, so method checks stay live
        // rather than being blanket-suppressed.
        assert!(!class(MEGAWIDGET, "::FocusableWidget").inheritance_unknown);
    }

    #[test]
    fn an_unreadable_manufacturer_override_marks_inheritance_unknown() {
        // Abstention — the override builds its prologue from a runtime
        // value, so the superclass list cannot be known.  The class is
        // still recorded (it exists), but its inheritance is flagged
        // opaque so no method-existence check fires against a guess.
        let src = concat!(
            "oo::class create Meta {\n",
            "    superclass oo::class\n",
            "    self method create {name extra body} {\n",
            "        next $name [list superclass [pickBase $extra]]\\;$body\n",
            "    }\n",
            "}\n",
            "Meta create Widget somewhere {\n",
            "    method m {} { return 1 }\n",
            "}\n",
        );
        let cd = class(src, "::Widget");
        assert!(cd.inheritance_unknown, "{cd:?}");
        assert!(cd.superclasses.is_empty(), "{cd:?}");
        assert!(cd.methods.contains_key("m"), "{cd:?}");
    }

    /// A factory whose `create` override composes the definition body as a
    /// **string** — no nested command anywhere — yet really does inject a
    /// superclass.  tclsh 9.0.4 and 8.6.16 both report `StrWidget supers:
    /// ::StrBase` for this and run `[[StrWidget new] inherited]`.
    const STRING_PROLOGUE_META: &str = concat!(
        "oo::class create StrBase {\n",
        "    method inherited {} { return inherited-ran }\n",
        "}\n",
        "oo::class create StrMeta {\n",
        "    superclass oo::class\n",
        "    self method create {name body} {\n",
        "        next $name \"superclass StrBase\\n$body\"\n",
        "    }\n",
        "}\n",
        "StrMeta create StrWidget {\n",
        "    method own {} { return own-ran }\n",
        "}\n",
    );

    /// The same shape with the override passing `$body` straight through —
    /// nothing composed, nothing injected.  tclsh 9.0.4 / 8.6.16 report
    /// only the implicit `::oo::object` for it, i.e. a genuinely empty
    /// injection.
    const DIRECT_BODY_META: &str = concat!(
        "oo::class create PlainMeta {\n",
        "    superclass oo::class\n",
        "    self method create {name body} {\n",
        "        next $name $body\n",
        "    }\n",
        "}\n",
        "PlainMeta create PlainWidget {\n",
        "    method own {} { return plain-own }\n",
        "}\n",
    );

    #[test]
    fn a_string_built_prologue_is_opaque_not_empty() {
        // TP — the prologue injects `superclass StrBase` with no command
        // substitution at all, so the scan finds nothing to read.  Claiming
        // a *known-empty* injection would assert the class has no
        // superclass and let W308 fire on every inherited method; the only
        // sound answer is opaque.
        let cd = class(STRING_PROLOGUE_META, "::StrWidget");
        assert!(cd.inheritance_unknown, "{cd:?}");
        assert!(
            cd.methods.contains_key("own"),
            "the class is still real: {cd:?}"
        );
    }

    #[test]
    fn an_opaque_prologue_suppresses_w308_on_inherited_methods() {
        // TP (consumer) — `inherited` really runs on both interpreters, so
        // no unknown-method warning may fire for it.
        let src = concat!(
            "oo::class create StrBase {\n",
            "    method inherited {} { return inherited-ran }\n",
            "}\n",
            "oo::class create StrMeta {\n",
            "    superclass oo::class\n",
            "    self method create {name body} {\n",
            "        next $name \"superclass StrBase\\n$body\"\n",
            "    }\n",
            "}\n",
            "StrMeta create StrWidget {\n",
            "    method own {} { return own-ran }\n",
            "}\n",
            "set w [StrWidget new]\n",
            "$w inherited\n",
        );
        assert!(
            !fires(src, "tcl9.0", "W308"),
            "an unreadable prologue must abstain: {:?}",
            codes(src, "tcl9.0")
        );
    }

    #[test]
    fn a_provably_direct_body_still_yields_a_known_empty_injection() {
        // TN — `next $name $body` composes nothing, so the injection is
        // known-empty and the class's inheritance is fully known.  This is
        // the half that must *not* become opaque, or the abstention above
        // would cost every precise diagnostic on a factory-made class.
        let cd = class(DIRECT_BODY_META, "::PlainWidget");
        assert!(!cd.inheritance_unknown, "{cd:?}");
        assert!(cd.superclasses.is_empty(), "{cd:?}");
        assert!(cd.methods.contains_key("own"), "{cd:?}");
    }

    #[test]
    fn a_provably_direct_body_keeps_method_checks_live() {
        // FN guard — the known-empty case must stay precise: a method that
        // exists nowhere on the class still warns.
        let src = concat!(
            "oo::class create PlainMeta {\n",
            "    superclass oo::class\n",
            "    self method create {name body} {\n",
            "        next $name $body\n",
            "    }\n",
            "}\n",
            "PlainMeta create PlainWidget {\n",
            "    method own {} { return plain-own }\n",
            "}\n",
            "set w [PlainWidget new]\n",
            "$w nosuchmethod\n",
        );
        assert!(
            fires(src, "tcl9.0", "W308"),
            "a known-empty injection keeps method checks live: {:?}",
            codes(src, "tcl9.0")
        );
    }

    #[test]
    fn a_relative_superclass_reaches_the_metaclass_in_its_own_namespace() {
        // TP — `::n::DerivedMeta` declares `superclass Meta`, which Tcl
        // resolves in the declaring class's own namespace: tclsh 9.0.4 /
        // 8.6.16 both report `info class superclasses ::n::DerivedMeta` =
        // `::n::Meta`, and `::n::DerivedMeta create ::NsWidget {…}` really
        // makes a class.  Looking only at `Meta` / `::Meta` missed it.
        let src = concat!(
            "namespace eval ::n {\n",
            "    oo::class create Meta {\n",
            "        superclass oo::class\n",
            "        self method create {name body} { next $name $body }\n",
            "    }\n",
            "    oo::class create DerivedMeta {\n",
            "        superclass Meta\n",
            "    }\n",
            "}\n",
            "::n::DerivedMeta create ::NsWidget {\n",
            "    method nsown {} { return ns-own }\n",
            "}\n",
        );
        let cd = class(src, "::NsWidget");
        assert!(cd.methods.contains_key("nsown"), "{cd:?}");
    }

    #[test]
    fn a_relative_superclass_prefers_its_own_namespace_over_a_decoy() {
        // TN (cross-link guard, the #1063 precedent) — a same-tailed class
        // in an unrelated namespace must not be picked when the declaring
        // namespace has its own.  Here `::other::Meta` is a plain class and
        // `::n::Meta` is the real metaclass; picking the decoy would leave
        // `::NsWidget` unrecorded.
        let src = concat!(
            "namespace eval ::other {\n",
            "    oo::class create Meta {\n",
            "        method notafactory {} { return 1 }\n",
            "    }\n",
            "}\n",
            "namespace eval ::n {\n",
            "    oo::class create Meta {\n",
            "        superclass oo::class\n",
            "        self method create {name body} { next $name $body }\n",
            "    }\n",
            "    oo::class create DerivedMeta {\n",
            "        superclass Meta\n",
            "    }\n",
            "}\n",
            "::n::DerivedMeta create ::NsWidget {\n",
            "    method nsown {} { return ns-own }\n",
            "}\n",
        );
        assert!(class(src, "::NsWidget").methods.contains_key("nsown"));
    }

    #[test]
    fn a_relative_superclass_does_not_cross_link_to_another_namespace() {
        // TN — the declaring namespace's own `Meta` is an ordinary class,
        // so the chain ends there.  Reaching sideways into `::other::Meta`
        // (which *is* a metaclass) would manufacture a class creation real
        // Tcl never performs — `::n::Meta` shadows it from `::n`.
        let src = concat!(
            "namespace eval ::other {\n",
            "    oo::class create Meta {\n",
            "        superclass oo::class\n",
            "        self method create {name body} { next $name $body }\n",
            "    }\n",
            "}\n",
            "namespace eval ::n {\n",
            "    oo::class create Meta {\n",
            "        method notafactory {} { return 1 }\n",
            "    }\n",
            "    oo::class create DerivedMeta {\n",
            "        superclass Meta\n",
            "    }\n",
            "}\n",
            "::n::DerivedMeta create ::NsWidget {\n",
            "    method nsown {} { return ns-own }\n",
            "}\n",
        );
        assert!(
            !analysis(src, "tcl9.0")
                .all_classes
                .contains_key("::NsWidget"),
            "a non-factory chain must not manufacture a class: {:?}",
            analysis(src, "tcl9.0")
                .all_classes
                .keys()
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn an_ordinary_class_is_not_treated_as_a_factory() {
        // TN — `Dog` is a plain class, so `Dog create rex` makes an
        // *instance*, never a class.  Only a chain that reaches
        // `oo::class` manufactures classes.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} { return woof }\n",
            "}\n",
            "Dog create rex\n",
        );
        assert!(
            !analysis(src, "tcl9.0").all_classes.contains_key("::rex"),
            "an instance is not a class"
        );
    }

    #[test]
    fn a_factory_without_a_create_override_keeps_the_builtin_layout() {
        // TP — no override means the inherited `oo::class` manufacturer
        // runs, so `Meta create Name Body` has its body at argument 2 and
        // splices no extra superclass.
        let src = concat!(
            "oo::class create Meta {\n",
            "    superclass oo::class\n",
            "}\n",
            "Meta create Plain {\n",
            "    method m {} { return 1 }\n",
            "}\n",
        );
        let cd = class(src, "::Plain");
        assert!(cd.methods.contains_key("m"), "{cd:?}");
        assert!(cd.superclasses.is_empty(), "{cd:?}");
        assert!(!cd.inheritance_unknown, "{cd:?}");
    }

    #[test]
    fn oo_define_over_a_literal_foreach_list_extends_every_named_class() {
        // TP — idx 55: the ticklecharts `etsb.tcl` monkey-patch.  Each
        // literal element names a real class, so each gets the injected
        // method; nothing lands under a synthetic `@dynclass@` key.
        let src = concat!(
            "oo::class create chart { method Render {} { return 1 } }\n",
            "oo::class create timeline { method Render {} { return 1 } }\n",
            "foreach cls {chart timeline} {\n",
            "    oo::define $cls {\n",
            "        method RenderTsb {} { return tsb }\n",
            "    }\n",
            "}\n",
        );
        let r = analysis(src, "tcl9.0");
        for name in ["::chart", "::timeline"] {
            assert!(
                r.all_classes[name].methods.contains_key("RenderTsb"),
                "{name} gained RenderTsb: {:?}",
                r.all_classes[name].methods.keys().collect::<Vec<_>>()
            );
        }
        assert!(
            !r.all_classes.keys().any(|k| k.contains("@dynclass@")),
            "no synthetic class survives: {:?}",
            r.all_classes.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_foreach_simulation_does_not_duplicate_body_diagnostics() {
        // FP guard — the simulation re-dispatches only the named-definition
        // installer, once per element.  A diagnostic raised inside the
        // injected member's own body must still be reported exactly once
        // per class it lands on, never once per (class × iteration).
        let src = concat!(
            "oo::class create chart { method Render {} { return 1 } }\n",
            "oo::class create timeline { method Render {} { return 1 } }\n",
            "foreach cls {chart timeline} {\n",
            "    oo::define $cls {\n",
            "        method RenderTsb {} { return $undefinedVar }\n",
            "    }\n",
            "}\n",
        );
        let spans: Vec<_> = analysis(src, "tcl9.0")
            .diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.span))
            .collect();
        let mut seen = std::collections::HashSet::new();
        for entry in &spans {
            assert!(
                seen.insert(entry.clone()),
                "duplicate diagnostic {entry:?} in {spans:?}",
            );
        }
    }

    #[test]
    fn oo_define_over_a_dynamic_list_still_abstains() {
        // TN — the element list is a runtime value, so no class name is
        // knowable and the members stay on a per-call-site synthetic key
        // rather than being attributed to a guess.
        let src = concat!(
            "oo::class create chart { method Render {} { return 1 } }\n",
            "foreach cls $classes {\n",
            "    oo::define $cls {\n",
            "        method RenderTsb {} { return tsb }\n",
            "    }\n",
            "}\n",
        );
        let r = analysis(src, "tcl9.0");
        assert!(
            !r.all_classes["::chart"].methods.contains_key("RenderTsb"),
            "a runtime target must not be attributed to a literal class"
        );
        assert!(
            r.all_classes.keys().any(|k| k.contains("@dynclass@")),
            "the members land on a synthetic key: {:?}",
            r.all_classes.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn oo_define_resolves_a_dominating_constant_target() {
        // TP — the narrowest form of the same fact: a target bound by a
        // plain `set` that dominates the call extends the real class.
        let src = concat!(
            "oo::class create Widget {\n",
            "    method real {} { return 1 }\n",
            "}\n",
            "set cls Widget\n",
            "oo::define $cls method added {} { return 2 }\n",
        );
        let cd = class(src, "::Widget");
        assert!(cd.methods.contains_key("real"), "{cd:?}");
        assert!(cd.methods.contains_key("added"), "{cd:?}");
    }

    #[test]
    fn static_brace_expansion_splices_a_member_signature() {
        // TP — idx 53: `{*}` of a braced literal is spliced by the parser,
        // so `method {*}{foo {} {…}}` defines a real `foo` (verified on
        // tclsh 9.0.4 and 8.6.16, in both the `oo::class create` body and
        // an `oo::define` body).
        let src = concat!(
            "oo::class create S {\n",
            "    method {*}{foo {x} {return foo-$x}}\n",
            "    constructor {*}{{a b} {return ctor}}\n",
            "}\n",
        );
        let cd = class(src, "::S");
        assert!(cd.methods.contains_key("foo"), "{cd:?}");
        assert_eq!(cd.methods["foo"].params.len(), 1);
        assert_eq!(cd.constructors.len(), 1, "{cd:?}");
        assert_eq!(cd.constructors[0].params.len(), 2);
    }

    #[test]
    fn static_brace_expansion_splices_in_an_oo_define_body_too() {
        // TP — same mechanism through the `oo::define` body form.
        let src = concat!(
            "oo::class create S {}\n",
            "oo::define S {\n",
            "    method {*}{bar {} {return bar}}\n",
            "}\n",
        );
        assert!(class(src, "::S").methods.contains_key("bar"));
    }

    #[test]
    fn substituted_brace_expansion_still_abstains() {
        // TN — `{*}[info class definition …]` reflects a signature whose
        // words are only known at run time, so the member is left
        // unrecorded rather than invented with wrong parameters or a body
        // span pointing at the wrong text.
        let src = concat!(
            "oo::class create chart3D {\n",
            "    constructor {*}[info class constructor chart]\n",
            "    method options {*}[classDef chart options]\n",
            "    method getType {} { return chart3D }\n",
            "}\n",
        );
        let cd = class(src, "::chart3D");
        assert!(cd.methods.contains_key("getType"), "{cd:?}");
        assert!(!cd.methods.contains_key("options"), "{cd:?}");
        assert!(cd.constructors.is_empty(), "{cd:?}");
        // …and the abstention is *recorded*, so the tables read as a lower
        // bound rather than as the class's whole surface (issue #923 idx 53).
        assert!(cd.member_set_incomplete, "{cd:?}");
    }

    #[test]
    fn a_foreach_member_installer_marks_the_member_set_incomplete() {
        // TP — idx 53's other half: the ticklecharts `chart3D` installer
        // loop.  `foreach` is not a member word and carries a script the
        // member walk never descends into, so every `method` it installs is
        // invisible; the class must say so.  tclsh 9.0.4 / 8.6.16: `info
        // class methods ::C3` really lists `options` and `globalOptions`.
        let src = concat!(
            "oo::class create C3 {\n",
            "    foreach m {options globalOptions} {\n",
            "        method $m {} { return $m }\n",
            "    }\n",
            "    method getType {} { return c3 }\n",
            "}\n",
        );
        let cd = class(src, "::C3");
        assert!(cd.methods.contains_key("getType"), "{cd:?}");
        assert!(cd.member_set_incomplete, "{cd:?}");
    }

    #[test]
    fn an_ordinary_class_body_keeps_a_complete_member_set() {
        // TN — the flag is not a blanket "TclOO is hard" switch: a class
        // whose body is nothing but readable member declarations (including
        // the *statically spliceable* `{*}` form and a `self` wrapper block)
        // still reports a complete member set, so W308 keeps its teeth.
        let src = concat!(
            "oo::class create Plain {\n",
            "    superclass ::oo::object\n",
            "    variable a b\n",
            "    constructor {x} { set a $x }\n",
            "    method {*}{spliced {} {return 1}}\n",
            "    self { method make {} { return made } }\n",
            "    method run {} { return $a }\n",
            "    destructor { return }\n",
            "}\n",
        );
        assert!(!class(src, "::Plain").member_set_incomplete);
    }

    #[test]
    fn w308_abstains_on_a_class_whose_members_are_installed_reflectively() {
        // TP — idx 53's user-visible wrong answer: `$c3 options` drew
        // "Unknown method 'options'" on a call tclsh proves succeeds.  A
        // class whose member tables are a lower bound cannot support a
        // missing-method claim, so W308 must abstain — while the sibling
        // class in the same file, whose body *is* fully readable, keeps
        // being checked (the TN half, guarding against a blanket
        // suppression).
        let src = concat!(
            "oo::class create Donor {\n",
            "    method options {} { return 1 }\n",
            "}\n",
            "oo::class create C3 {\n",
            "    constructor {*}[info class constructor ::Donor]\n",
            "    foreach m {options} { method $m {*}[info class definition ::Donor $m] }\n",
            "}\n",
            "set c3 [C3 new]\n",
            "$c3 options\n",
            "set d [Donor new]\n",
            "$d nosuch\n",
        );
        let r = analysis(src, "tcl9.0");
        let codes: Vec<&str> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W308")
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(
            codes.len(),
            1,
            "only the readable class draws W308: {codes:?}"
        );
        assert!(codes[0].contains("nosuch"), "{codes:?}");
    }

    /// A user metaclass whose own definition is in **another file** cannot be
    /// recognised: the analyser is per-file, and nothing in
    /// `X create Name Supers Body` distinguishes a class factory from
    /// `interp create`, `image create`, or any ordinary proc taking a script
    /// argument.  Guessing would invent classes out of unrelated commands, so
    /// the LSP abstains — records no class, and emits no diagnostic about the
    /// members it therefore cannot see (issue #923 idx 97, the multi-file
    /// half; Tk's own `library/iconlist.tcl` is exactly this shape).
    ///
    /// The single-file case — the metaclass and its products in one file, as
    /// in Tk's `library/megawidget.tcl` — is fully resolved; see
    /// `user_metaclass_creates_real_classes` above.
    #[test]
    fn a_metaclass_defined_in_another_file_abstains_rather_than_guessing() {
        let src = concat!(
            "::Megawidget create IconList FocusableWidget {\n",
            "    method GetSpecs {} { return \"iconlist+[next]\" }\n",
            "}\n",
        );
        let r = analysis(src, "tcl9.0");
        assert!(
            !r.all_classes.contains_key("::IconList"),
            "no class may be invented from an unknown command: {:?}",
            r.all_classes.keys().collect::<Vec<_>>()
        );
        assert!(
            r.diagnostics
                .iter()
                .all(|d| d.code.to_string() != "W308" && d.code.to_string() != "W001"),
            "abstention must be silent, not a wrong answer: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn a_braced_variable_command_head_resolves_through_its_constant() {
        // TP — idx 44: the head's *variable* is `ns`, not the whole token
        // text `ns}::setdef` the lexer hands over for a braced composite
        // word.  Reading the true source bytes makes the call resolve to
        // the proc it really dispatches to (tclsh 9.0.4 / 8.6.16 both run
        // `::tc::setdef`).
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "set ns tc\n",
            "${ns}::setdef x y\n",
        );
        let resolved: Vec<String> = analysis(src, "tcl8.6")
            .command_invocations
            .iter()
            .filter(|inv| inv.name.contains("${ns}"))
            .filter_map(|inv| inv.resolved_qualified_name.clone())
            .collect();
        assert_eq!(resolved, ["::tc::setdef"]);
    }

    #[test]
    fn a_resolved_braced_variable_head_is_a_reference_not_a_rename_target() {
        // FP guard — the head's span is `${ns}::setdef`, which spells only
        // the tail.  Rewriting that span with a new name would splice it
        // over the substitution and corrupt the source (the idx 95 lesson),
        // so the invocation is marked `indirect`: references report it,
        // rename skips it.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "set ns tc\n",
            "${ns}::setdef x y\n",
        );
        let r = analysis(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|inv| inv.name.contains("${ns}"))
            .expect("the composite head is recorded");
        assert!(inv.indirect, "{inv:?}");
    }

    #[test]
    fn a_whole_word_variable_head_is_left_to_the_flow_sensitive_engine() {
        // TN — `$cmd` is M7's shape, settled from the CFG/SSA value model
        // rather than the walk's lexical map.  The walk must not
        // pre-resolve it from a last-write-wins constant, or the two
        // disagree about which sites are safe to rewrite.
        let src = "proc target {} { return hi }\nset cmd target\n$cmd\n";
        let r = analysis(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|inv| inv.name == "${cmd}")
            .expect("the variable head is recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::${cmd}"),
            "the walk leaves this head unfolded for M7 to settle: {inv:?}",
        );
    }

    #[test]
    fn a_braced_variable_command_head_abstains_on_a_branch_binding() {
        // TN — a head whose variable is written differently on two
        // branches has no single dominating value, so resolution must not
        // pin either one.
        let src = concat!(
            "namespace eval a { proc go {} { return 1 } }\n",
            "namespace eval b { proc go {} { return 1 } }\n",
            "set ns a\n",
            "if {$cond} { set ns b }\n",
            "${ns}::go\n",
        );
        let resolved: Vec<String> = analysis(src, "tcl8.6")
            .command_invocations
            .iter()
            .filter(|inv| inv.name.contains("${ns}"))
            .filter_map(|inv| inv.resolved_qualified_name.clone())
            .collect();
        assert_eq!(
            resolved,
            ["::${ns}::go"],
            "an unresolved head stays as written"
        );
    }

    #[test]
    fn foreach_installed_procs_are_enumerated_per_literal_element() {
        // Previously-fixed regression pin — idx 43 (the ticklecharts
        // `etypes.tcl` ensemble).  The `foreach`-literal simulation landed
        // for issue #923 idx 86 (PR #1020) and already covers `proc`, so
        // every element's proc is registered under its real qualified
        // name.  Pinned here so the idx 43 shape cannot regress.
        let src = concat!(
            "namespace eval ticklecharts {}\n",
            "foreach ptype {elist elist.n elist.s} {\n",
            "    proc ticklecharts::${ptype} {args} { return $args }\n",
            "}\n",
        );
        let r = analysis(src, "tcl8.6");
        for name in [
            "::ticklecharts::elist",
            "::ticklecharts::elist.n",
            "::ticklecharts::elist.s",
        ] {
            assert!(
                r.all_procs.contains_key(name),
                "{name} registered: {:?}",
                r.all_procs.keys().collect::<Vec<_>>()
            );
        }
        assert!(
            !r.all_procs.keys().any(|k| k.contains('$')),
            "no proc is filed under the unsubstituted template: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }
}

// ===========================================================================
// Constant command-substitution `set` RHS folding — issue #1132.
//
// The analyser's constant lattice folds `set VAR [cmd …]` through the
// registry `const_fold` / frame-fact engine (`crate::const_subst`), so the
// `${ns}::setdef` navigation chain resolves when `ns` was assigned a
// constant command substitution rather than a bare literal.
//
// C-Tcl ground truth (tclsh 9.0.4, run while authoring): `namespace
// qualifiers ::tc::X` → `::tc`; inside an instance method of `::tc::Chart`
// the full chain really dispatches (`set ns [namespace qualifiers [self
// class]]; ${ns}::setdef x y` → `::tc::setdef` runs); inside a `self
// method`, `self class` raises `method not defined by a class`; inside a
// `classmethod` it answers the internal `::oo::ObjN:: oo ::delegate` class,
// never the written one.
// ===========================================================================
mod const_cmd_subst_set_rhs {
    use super::*;
    use tcl_compiler::analyser::AnalysisResult;

    fn analysis(src: &str, dialect: &str) -> AnalysisResult {
        Analyser::new().analyse(src, dialect)
    }

    fn resolutions_of(r: &AnalysisResult, head_contains: &str) -> Vec<String> {
        r.command_invocations
            .iter()
            .filter(|inv| inv.name.contains(head_contains))
            .filter_map(|inv| inv.resolved_qualified_name.clone())
            .collect()
    }

    #[test]
    fn a_constant_namespace_qualifiers_rhs_folds_and_resolves_the_head() {
        // TP — the probe shape from issue #1132: zero OO involvement, a
        // plain proc, a constant `[namespace qualifiers …]` RHS.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::tc::setdef"],
            "the folded constant must drive head resolution"
        );
    }

    #[test]
    fn a_self_class_chain_folds_inside_an_instance_method() {
        // TP — the ticklecharts idiom one level removed: `set ns
        // [namespace qualifiers [self class]]` inside an instance method
        // of `::tc::Chart` (issue #923 idx 44's full mechanic).
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "oo::class create ::tc::Chart {\n",
            "    method go {} {\n",
            "        set ns [namespace qualifiers [self class]]\n",
            "        ${ns}::setdef x y\n",
            "    }\n",
            "}\n",
        );
        let r = analysis(src, "tcl9.0");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::tc::setdef"],
            "the frame-fact fold must feed the constant lattice"
        );
    }

    #[test]
    fn a_class_side_method_abstains_from_the_self_class_fold() {
        // FP guard (issue #1132 design constraint 2): `self class` never
        // answers the written class in a class-side frame (tclsh 9.0.4:
        // raises in a `self method`; answers the internal delegate class
        // in a `classmethod`) — folding it would invent a value. The head
        // must stay unresolved-as-written.
        for member in ["classmethod go {}", "self method go {}"] {
            let src = format!(
                concat!(
                    "namespace eval tc {{ proc setdef {{a b}} {{ return 1 }} }}\n",
                    "oo::class create ::tc::Chart {{\n",
                    "    {} {{\n",
                    "        set ns [namespace qualifiers [self class]]\n",
                    "        ${{ns}}::setdef x y\n",
                    "    }}\n",
                    "}}\n",
                ),
                member
            );
            let r = analysis(&src, "tcl9.0");
            assert_eq!(
                resolutions_of(&r, "${ns}"),
                ["::${ns}::setdef"],
                "{member}: a class-side frame must not fold [self class]"
            );
        }
    }

    #[test]
    fn a_later_rename_of_the_folding_head_blocks_the_fold() {
        // FP guard (issue #1132 design constraint 3): the trust oracle is
        // whole-module — a `rename` AFTER the `set`, buried inside a proc
        // body, still unbinds `namespace` from its builtin semantics
        // before some later call can run. The mid-walk `renamed_commands`
        // map cannot see it; the fold must still decline.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
            "proc sabotage {} { rename namespace nsx }\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::${ns}::setdef"],
            "a whole-module rename of the head must block the fold"
        );
    }

    #[test]
    fn a_shadowing_proc_definition_blocks_the_fold() {
        // FP guard — a user `proc namespace …` anywhere in the module
        // shadows the builtin, so `[namespace qualifiers …]` no longer has
        // builtin semantics (flow-insensitive: cross-proc call order is
        // not statically known).
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
            "proc namespace {args} { return ::evil }\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::${ns}::setdef"],
            "a shadowed builtin must not const-fold"
        );
    }

    #[test]
    fn a_branch_dependent_argument_abstains() {
        // TN — the substitution's `$q` argument has no dominating constant
        // (two branch values), so the fold must abstain rather than pick
        // the last-written branch.
        let src = concat!(
            "namespace eval a { proc go {} { return 1 } }\n",
            "namespace eval b { proc go {} { return 1 } }\n",
            "proc user {c} {\n",
            "    set q ::a::X\n",
            "    if {$c} { set q ::b::X }\n",
            "    set ns [namespace qualifiers $q]\n",
            "    ${ns}::go\n",
            "}\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::${ns}::go"],
            "a branch-dependent argument must not fold"
        );
    }

    #[test]
    fn per_item_path_folds_and_abstains_identically_to_the_whole_file_walk() {
        // The per-item (incremental) path analyses each proc body in
        // ISOLATION — it cannot see a `rename` elsewhere in the file, so
        // the shell attaches a whole-file trust snapshot to every deferred
        // body with a fold candidate (`DeferredBody::command_trust`).
        // Both directions must match the whole-file walk byte-for-byte:
        // the TP still folds, the FP still abstains.
        let tp = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
        );
        let fp = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
            "proc sabotage {} { rename namespace nsx }\n",
        );
        for (src, expected) in [(tp, "::tc::setdef"), (fp, "::${ns}::setdef")] {
            let whole = analysis(src, "tcl8.6");
            let per_item = Analyser::new().analyse_per_item(src, "tcl8.6");
            assert_eq!(
                resolutions_of(&whole, "${ns}"),
                [expected],
                "whole-file walk for {src:?}"
            );
            assert_eq!(
                resolutions_of(&per_item, "${ns}"),
                resolutions_of(&whole, "${ns}"),
                "per-item must match the whole-file walk for {src:?}"
            );
        }
    }

    #[test]
    fn a_dominating_constant_argument_folds_through_the_lattice() {
        // TP — the substitution reads a `$q` that IS a dominating
        // constant; the fold chains through the constant lattice.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set q ::tc::X\n",
            "    set ns [namespace qualifiers $q]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(resolutions_of(&r, "${ns}"), ["::tc::setdef"]);
    }
}

// ===========================================================================
// Leading byte-order mark (issue #1218).
// ===========================================================================
mod leading_bom {
    use super::{Analyser, analyser_diags, fires};

    /// A file whose first byte is a UTF-8 BOM: Tcl 9's `source` strips it
    /// before evaluating, so the first command is an ordinary `set` and must
    /// draw no unresolved-command diagnostic.
    #[test]
    fn tcl9_skips_a_leading_bom() {
        let src = "\u{FEFF}set x 1\nputs $x\n";
        assert!(
            !fires(src, "tcl9.0", "W123"),
            "diagnostics: {:?}",
            analyser_diags(src, "tcl9.0"),
        );
    }

    /// Tcl 8.x's `source` does **not** strip it — such a file really does fail
    /// with `invalid command name "<BOM>set"`, so the diagnostic stays.
    #[test]
    fn tcl86_still_reports_a_leading_bom() {
        let src = "\u{FEFF}set x 1\nputs $x\n";
        assert!(
            fires(src, "tcl8.6", "W123"),
            "diagnostics: {:?}",
            analyser_diags(src, "tcl8.6"),
        );
    }

    /// The mark is skipped by starting the scan past it, never by rewriting
    /// the buffer — so every span keeps its true byte offset and the first
    /// command's own span still points at the real name word.  This is the
    /// position-maths guarantee: a client that did send the BOM sees line 0
    /// columns that still land on the text it holds.
    #[test]
    fn skipping_the_bom_leaves_byte_offsets_untouched() {
        let src = "\u{FEFF}proc greet {} { return 1 }\n";
        let r = Analyser::new().analyse(src, "tcl9.0");
        let proc = r
            .global_scope
            .procs
            .values()
            .next()
            .expect("the proc must be recorded once the BOM is skipped");
        assert_eq!(
            proc.name_span.start() as usize,
            "\u{FEFF}proc ".len(),
            "name span must be in original-source offsets",
        );
        assert_eq!(
            &src[proc.name_span.start() as usize..proc.name_span.end() as usize],
            "greet",
        );
    }

    /// A BOM that is not at offset 0 is ordinary data in every dialect — the
    /// skip is a file-prologue rule, not a "U+FEFF is whitespace" rule.
    #[test]
    fn a_bom_mid_file_is_not_skipped() {
        let src = "set x 1\n\u{FEFF}set y 2\n";
        assert!(
            fires(src, "tcl9.0", "W123"),
            "diagnostics: {:?}",
            analyser_diags(src, "tcl9.0"),
        );
    }
}

// ===========================================================================
// A TclOO member body's implicit `namespace path` — issue #1137 idx 51.
//
// tclsh 8.6.16 and 9.0.4, inside `oo::class create C { method m {} { … } }`:
//     namespace current -> ::oo::ObjN        namespace path -> ::oo::Helpers
// so a bare word is looked up in the object namespace, then `::oo::Helpers`,
// then global. The documented "TclOO Tricks" idiom installs helpers directly
// (`proc ::oo::Helpers::callback {…} {…}`) and calls them bare from method
// bodies — which is what nico-robert/ticklecharts does, with the helper in
// `utils.tcl` and the calls in `esnap.tcl`.
//
// The candidate list is where that path belongs: `resolve_workspace_symbols`
// (the one cross-file resolver) walks it, so recording it once here fixes
// definition, references, rename, call hierarchy, and W123 together, across
// files, instead of each consumer growing its own lenient fallback.
// ===========================================================================
mod oo_helpers_namespace_path {
    use super::Analyser;

    fn candidates_for(src: &str, dialect: &str, name: &str) -> Vec<String> {
        Analyser::new()
            .analyse(src, dialect)
            .command_invocations
            .iter()
            .find(|inv| inv.name == name)
            .map(|inv| inv.resolution_candidates.clone())
            .unwrap_or_default()
    }

    fn resolution_of(src: &str, dialect: &str, name: &str) -> Option<String> {
        Analyser::new()
            .analyse(src, dialect)
            .command_invocations
            .iter()
            .find(|inv| inv.name == name)
            .and_then(|inv| inv.resolved_qualified_name.clone())
    }

    #[test]
    fn tp_a_method_body_call_carries_the_helpers_candidate_first() {
        // TP — `::oo::Helpers::callback` must be a candidate, and must rank
        // ahead of the global one: with both defined, real Tcl reaches the
        // helper.
        let src = "oo::class create C {\n    method Go {} {\n        callback Read\n    }\n}\n";
        assert_eq!(
            candidates_for(src, "tcl9.0", "callback"),
            ["::oo::Helpers::callback", "::callback"],
        );
    }

    #[test]
    fn tp_the_same_holds_for_a_constructor_body() {
        let src = "oo::class create C {\n    constructor {} {\n        callback Read\n    }\n}\n";
        assert_eq!(
            candidates_for(src, "tcl9.0", "callback"),
            ["::oo::Helpers::callback", "::callback"],
        );
    }

    #[test]
    fn tp_a_helper_declared_in_the_same_file_settles_the_call_onto_it() {
        // TP — with the helper present the call settles on it, not on a
        // phantom `::callback`.
        let src = "proc ::oo::Helpers::callback {m args} { return $m }\noo::class create C {\n    method Go {} {\n        callback Read\n    }\n}\n";
        assert_eq!(
            resolution_of(src, "tcl9.0", "callback").as_deref(),
            Some("::oo::Helpers::callback"),
        );
    }

    #[test]
    fn fp_a_top_level_call_gets_no_helpers_candidate() {
        // FP guard — `::oo::Helpers` is on a *method body's* namespace path
        // only. tclsh raises `invalid command name` for a bare `callback`
        // at the top level, so the candidate must not appear there.
        let src = "proc ::oo::Helpers::callback {m args} { return $m }\ncallback Read\n";
        assert_eq!(candidates_for(src, "tcl9.0", "callback"), ["::callback"]);
    }

    #[test]
    fn fp_an_apply_lambda_inside_a_method_loses_the_path() {
        // FP guard — `apply` runs its body in the global namespace, so the
        // object context (and its path) is gone. tclsh 9.0.4 raises
        // `invalid command name "link"` for exactly this shape.
        let src = "oo::class create C {\n    method Go {} {\n        apply {{} { callback Read }}\n    }\n}\n";
        assert_eq!(candidates_for(src, "tcl9.0", "callback"), ["::callback"]);
    }

    #[test]
    fn tn_a_snit_method_body_gets_no_helpers_candidate() {
        // TN — snit member bodies run in the type namespace with no
        // injected path; only the TclOO grammar declares one.
        let src = "snit::type T {\n    method Go {} {\n        callback Read\n    }\n}\n";
        let cands = candidates_for(src, "tcl9.0", "callback");
        assert!(
            !cands.iter().any(|c| c.starts_with("::oo::Helpers")),
            "snit gets no TclOO helper path: {cands:?}",
        );
    }

    #[test]
    fn tn_an_ordinary_builtin_still_settles_globally_from_a_method_body() {
        // TN — the extra candidate must not divert an ordinary call: `puts`
        // has no `::oo::Helpers` member, so it still reaches `::puts`.
        let src = "oo::class create C {\n    method Go {} {\n        puts hi\n    }\n}\n";
        assert_eq!(
            resolution_of(src, "tcl9.0", "puts").as_deref(),
            Some("::puts"),
        );
    }
}

// ===========================================================================
// A constant-dominated computed `namespace eval` target — issue #1113 item 3.
//
// `set ns ::app; namespace eval $ns { … }` creates `::app` on every run, so
// the block's procs really do home to `::app::…`.  The word is settled by the
// same identity-resolution helper the command head (idx 44), `source`,
// `rename`, and `oo::define`'s target already use, so its dominance rule —
// a branch-conditional binding proves nothing — applies here unchanged.
// Anything it cannot settle keeps the per-site `@dynns@` domain.
// ===========================================================================
mod const_dominated_namespace_eval {
    use super::Analyser;

    fn proc_names(src: &str) -> Vec<String> {
        let mut names: Vec<String> = Analyser::new()
            .analyse(src, "tcl9.0")
            .all_procs
            .keys()
            .cloned()
            .collect();
        names.sort();
        names
    }

    fn scope_names(src: &str) -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl9.0")
            .global_scope
            .children
            .iter()
            .map(|c| c.name.clone())
            .collect()
    }

    #[test]
    fn tp_a_constant_target_homes_the_blocks_procs_to_the_real_namespace() {
        let src = "set ns ::app\nnamespace eval $ns {\n    proc go {} { return 1 }\n}\n";
        assert_eq!(proc_names(src), ["::app::go"]);
        assert_eq!(scope_names(src), ["::app"]);
    }

    #[test]
    fn tp_the_word_is_recorded_as_a_declaring_occurrence() {
        // Navigation's half: the `$ns` word is where `::app` is created, so
        // it is the answer a `namespace children ::app` elsewhere jumps to.
        let src = "set ns ::app\nnamespace eval $ns { proc go {} { return 1 } }\n";
        let r = Analyser::new().analyse(src, "tcl9.0").clone();
        let decls: Vec<&str> = r
            .namespace_refs
            .iter()
            .filter(|n| n.declares && n.qualified_name == "::app")
            .map(|n| &src[n.span.start() as usize..n.span.end() as usize])
            .collect();
        assert_eq!(decls, ["$ns"], "{:?}", r.namespace_refs);
    }

    #[test]
    fn fp_a_branch_conditional_binding_keeps_the_synthetic_domain() {
        // FP guard — the dominance rule.  Which namespace this creates is a
        // run-time question, so claiming either would be a wrong answer.
        let src = "set ns ::a\nif {$c} { set ns ::b }\nnamespace eval $ns {\n    proc go {} { return 1 }\n}\n";
        assert!(
            proc_names(src).iter().all(|n| n.contains("@dynns@")),
            "{:?}",
            proc_names(src),
        );
    }

    #[test]
    fn tn_a_parameter_target_keeps_the_synthetic_domain() {
        // TN — the irc.tcl per-connection idiom the synthetic domain exists
        // for: nothing constant reaches `$n`, so each occurrence stays its
        // own scope.
        let src =
            "proc mk {n} {\n    namespace eval $n {\n        proc go {} { return 1 }\n    }\n}\n";
        let names = proc_names(src);
        assert!(names.iter().any(|n| n.contains("@dynns@")), "{names:?}");
    }

    #[test]
    fn tn_a_literal_target_is_unchanged() {
        let src = "namespace eval ::app {\n    proc go {} { return 1 }\n}\n";
        assert_eq!(proc_names(src), ["::app::go"]);
        assert_eq!(scope_names(src), ["::app"]);
    }
}

// ===========================================================================
// Issue #1252 — a brace-quoted word's *elements* are literal too.
//
// The IR/analyser word arrives with its braces already stripped, so scanning
// the element text alone reports "dynamic" for content Tcl never substitutes.
// #1245 fixed the whole-word question for `namespace path`; these are the two
// remaining places that ask it per element with the token in hand.
//
// tclsh-proof (8.6.16 / 9.0.4):
//   namespace eval x {}
//   namespace eval {::$ns} {}
//   namespace eval n { namespace path {::$ns ::x} ; namespace path }
//     ->  {::$ns} ::x        (an entry literally named `::$ns`, no var read)
//   foreach n {aa {$b} cc} { puts $n }
//     ->  aa / $b / cc       (three literal iterations, no read of `b`)
// ===========================================================================
mod braced_word_elements_are_literal {
    use super::*;

    /// Namespace-reference source texts recorded for `src`.
    fn ns_ref_texts(src: &str) -> Vec<String> {
        let r = Analyser::new().analyse(src, "tcl9.0").clone();
        let mut out: Vec<String> = r
            .namespace_refs
            .iter()
            .map(|n| src[n.span.start() as usize..n.span.end() as usize].to_owned())
            .collect();
        out.sort();
        out
    }

    /// Diagnostic codes emitted for `src`.
    fn diag_codes(src: &str) -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl9.0")
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect()
    }

    #[test]
    fn tp_namespace_path_records_a_braced_dollar_element_as_a_reference() {
        // The whole-word gate (#1245) already lets the path be *recorded*;
        // the element-reference walk skipped the same word, so the function
        // disagreed with itself. Both must now see `::$ns`.
        let src = "namespace eval n { namespace path {::$ns ::x} }\n";
        assert!(
            ns_ref_texts(src).iter().any(|t| t == "::$ns"),
            "the braced element is a namespace literally named `::$ns`; got {:?}",
            ns_ref_texts(src)
        );
    }

    #[test]
    fn fp_namespace_path_still_skips_a_substituting_element() {
        // FP guard: an unbraced path word does substitute, so its `$ns` names
        // nothing static and must stay unrecorded.
        let src = "namespace eval n { namespace path ::$ns }\n";
        assert!(
            !ns_ref_texts(src).iter().any(|t| t.contains('$')),
            "a substituting path word must record no element reference; got {:?}",
            ns_ref_texts(src)
        );
    }

    #[test]
    fn tn_namespace_path_literal_elements_unchanged() {
        let src = "namespace eval n { namespace path {::x ::y} }\n";
        let refs = ns_ref_texts(src);
        assert!(refs.iter().any(|t| t == "::x"), "{refs:?}");
        assert!(refs.iter().any(|t| t == "::y"), "{refs:?}");
    }

    #[test]
    fn tp_foreach_simulation_survives_a_braced_dollar_element() {
        // One odd element used to abstain from the whole simulation, so every
        // proc the loop installs went missing and each call raised W123.
        let src = "foreach n {aa {$b} cc} { proc $n {} {} }\naa\ncc\n";
        assert!(
            !diag_codes(src).iter().any(|c| c == "W123"),
            "the braced element must not abstain the whole simulation; got {:?}",
            diag_codes(src)
        );
    }

    #[test]
    fn fp_foreach_simulation_still_abstains_on_a_substituting_list() {
        // FP guard: a quoted list word *does* substitute, so its elements are
        // not knowable and the simulation must still decline.
        let src = "foreach n \"aa $b cc\" { proc $n {} {} }\naa\n";
        assert!(
            diag_codes(src).iter().any(|c| c == "W123"),
            "a substituting list word must still abstain; got {:?}",
            diag_codes(src)
        );
    }

    #[test]
    fn tn_foreach_simulation_literal_elements_unchanged() {
        let src = "foreach n {aa cc} { proc $n {} {} }\naa\ncc\n";
        assert!(
            !diag_codes(src).iter().any(|c| c == "W123"),
            "{:?}",
            diag_codes(src)
        );
    }
}
