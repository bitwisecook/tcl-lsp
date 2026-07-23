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
        Analyser::new()
            .analyse(src, D)
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
        let src = "oo::class create Dog {\n    method bark {} { return \"woof\" }\n}\noo::define Dog {\n    method fetch {item} { return $item }\n    classmethod count {} { return 0 }\n}\n";
        let cd = class(src, "::Dog");
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
        let cd = class(
            "oo::class create Counter {\n    classmethod instances {} { return 0 }\n}\n",
            "::Counter",
        );
        assert_eq!(cd.class_methods["instances"].kind, "classmethod");
        let cd2 = class(
            "oo::class create Foo {\n    private method helper {} { return 1 }\n}\n",
            "::Foo",
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
        let src = "oo::class create Widget {\n    private method helper {} {\n        string length a b c d\n    }\n}\n";
        assert_eq!(
            count(src, D, "E003"),
            1,
            "the wrong-arity call inside a private-method body must now be walked: {:?}",
            codes(src, D)
        );
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
