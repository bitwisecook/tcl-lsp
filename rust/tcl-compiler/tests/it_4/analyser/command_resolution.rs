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

//! How a command word resolves: namespaces, `package require`, qualified `dict` specs, scoped command reports and pack-declared transitions.

// Namespace scopes + qualified procs.
mod namespace_analysis {
    use super::super::*;

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

// package require / provide records.
mod package_require {
    use super::super::*;

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
        assert_eq!(
            Analyser::new()
                .analyse("package provide mylib 1.0", D)
                .package_requires,
            [] as [tcl_compiler::signature_scan::types::SignaturePackageRequire; 0]
        );
        assert_eq!(
            Analyser::new().analyse("set x 42", D).package_requires,
            [] as [tcl_compiler::signature_scan::types::SignaturePackageRequire; 0]
        );
    }
}

// `::tcl::dict::*` standalone spellings must carry the same analysis
// contract as the `dict` subcommands they mirror, not a
// `CommandSpec::DEFAULT` stub.
mod dict_qualified_specs {
    use super::super::*;

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

// report::defstyle scoped command environment.
//
// The style script exposes the report configuration methods (`top`, `data`,
// `columns`, …) as commands available only inside the body.  The registry-
// driven scoped environment resolves them there (no W123 / arity false
// positives) while still catching genuine typos and misuse (TP), and keeps
// them unknown outside the body (correct scoping).  Organised as a
// TP / FP / TN / FN matrix.
mod report_scoped_commands {
    use super::super::*;

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

    // TN: valid scoped usage draws no unknown-command / arity error.

    #[test]
    fn tn_valid_body_no_w123() {
        // A representative shape: line codes + operations.
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

    // TP: genuine errors inside the body are still reported.

    #[test]
    fn tp_typo_command_flagged() {
        // `toop` / `dataa` are not scoped commands → still W123.
        // tcllib report is loaded: without it `::report::defstyle` raises
        // `invalid command name` and the style body never runs (tclsh 8.6.18).
        let src =
            "package require report\n::report::defstyle st {} {\n  toop set x\n  dataa set y\n}\n";
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
        // tcllib report is loaded: without it `::report::defstyle` raises
        // `invalid command name` and the style body never runs (tclsh 8.6.18).
        let src = "package require report\n::report::defstyle st {} {\n  top\n}\n";
        assert!(
            fires(src, D, "E001"),
            "bare `top` → E001: {:?}",
            codes_of(src)
        );
    }

    #[test]
    fn tp_operation_too_few_args_e002() {
        // `top set` needs the template value.
        // tcllib report is loaded: without it `::report::defstyle` raises
        // `invalid command name` and the style body never runs (tclsh 8.6.18).
        let src = "package require report\n::report::defstyle st {} {\n  top set\n}\n";
        assert!(
            fires(src, D, "E002"),
            "`top set` (no value) → E002: {:?}",
            codes_of(src)
        );
    }

    #[test]
    fn tp_plain_command_too_many_args_e003() {
        // `columns` takes no arguments.
        // tcllib report is loaded: without it `::report::defstyle` raises
        // `invalid command name` and the style body never runs (tclsh 8.6.18).
        let src = "package require report\n::report::defstyle st {} {\n  columns extra\n}\n";
        assert!(
            fires(src, D, "E003"),
            "`columns extra` → E003: {:?}",
            codes_of(src)
        );
    }

    // FP guard: the scoped env must not wrongly suppress real code.

    #[test]
    fn fp_core_commands_still_checked_in_body() {
        // Core commands inside the body keep their normal arity checks — a
        // scoped env must not swallow them.  `set` with one arg is fine; a
        // genuinely unknown core-looking head is still W123.
        // tcllib report is loaded: without it `::report::defstyle` raises
        // `invalid command name` and the style body never runs (tclsh 8.6.18).
        let src =
            "package require report\n::report::defstyle st {} {\n  set x 1\n  frobnicate a b\n}\n";
        assert!(
            w123(src).iter().any(|m| m.contains("frobnicate")),
            "unknown non-scoped head still W123: {:?}",
            w123(src)
        );
    }

    // FN guard / scoping: scoped commands are unknown OUTSIDE the body.

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

    // Report namespace + object commands.

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

// ── Pack-declared transitions and special variables ─────────────────────────

mod pack_declared_transitions {
    use super::super::*;

    /// Load `source` as a workspace pack and return an analyser bound to the
    /// registry generation it installs — the path the language server takes
    /// for a document under `.tcl-lsp/`.
    fn analyser_with_pack(source: &str) -> Analyser {
        let packs = tcl_spectcl::pack::load_in_memory(vec![(
            tcl_spectcl::PackFile {
                tier: tcl_spectcl::Tier::Workspace,
                path: std::path::PathBuf::from("/workspace/.tcl-lsp/scoped.tclspec"),
                origin: tcl_spectcl::discovery::Origin::DotDir,
                dependency_tier: None,
            },
            source.to_owned(),
        )]);
        let _registry = tcl_spectcl::install::registry_for_dialect_with_packs(D, &packs);
        Analyser::new().with_pack_overlay(packs.key)
    }

    /// A command no shipped registry knows, crossing frames exactly as
    /// `upvar` does: its `frame_effect` is `upvar`'s, and its
    /// `state_transitions` resolver derives the alias facts from it.
    const LINKER_PACK: &str = "speclib scoped 1.0 {\n\
        \x20   command scoped_link {\n\
        \x20       arity 2..\n\
        \x20       frame_effect -level-word ArityParity -layout AliasPairs\n\
        \x20       state_transitions {\n\
        \x20           argument_shape Positional\n\
        \x20           resolver from-frame-effect\n\
        \x20           widen -operands EveryArgument -domains {VariableCells VariableTraces}\n\
        \x20           commit MayCommitBeforeAbruptCompletion\n\
        \x20       }\n\
        \x20   }\n\
        }\n";

    /// The analyser binds a pack command's alias facts as it binds
    /// `upvar`'s — it reads the invocation's transitions, never the
    /// command's name: `scoped_link #0 counter c` defines `c` and links it to the
    /// global cell. With a computed level word the derived resolver has
    /// `upvar`'s abstentions (registry-consumer-contracts.md): tclsh8.6.18/
    /// 9.0.4 `upvar $lvl counter c` makes `c` a local link to an unknown
    /// frame, so `c` binds exactly as the `upvar` handler binds it.
    #[test]
    fn a_pack_declared_scope_alias_binds_its_local() {
        let bound = analyser_with_pack(LINKER_PACK).analyse(
            "proc p {} {\n    scoped_link #0 counter c\n    return $c\n}\n",
            D,
        );
        let proc_scope = &bound.global_scope.children[0];
        let c = proc_scope
            .variables
            .get("c")
            .unwrap_or_else(|| panic!("`c` is bound: {:?}", proc_scope.variables.keys()));
        assert_eq!(c.link_target.as_deref(), Some("::counter"));
        assert!(!c.warn_if_unused);

        let computed = analyser_with_pack(LINKER_PACK)
            .analyse("proc p {lvl} {\n    scoped_link $lvl counter c\n}\n", D);
        let shipped = analyser_with_pack(LINKER_PACK)
            .analyse("proc p {lvl} {\n    upvar $lvl counter c\n}\n", D);
        let derived = computed.global_scope.children[0]
            .variables
            .get("c")
            .expect("a computed level still binds `c`");
        let upvar = shipped.global_scope.children[0]
            .variables
            .get("c")
            .expect("`upvar` binds `c`");
        assert_eq!(derived.link_target, upvar.link_target);
    }

    /// A pack command whose grammar gives one clause two `LoopVarList`
    /// slots — the shipped `catch` shape — with no `arg` rows stating them
    /// again as flat roles, so the clause is the only place they are named.
    const TWO_LISTS_PACK: &str = "speclib twolists 2.1 {\n\
        \x20   command cmd {\n\
        \x20       arity 1..3\n\
        \x20       clause_grammar {\n\
        \x20           head {Body} -timing protected\n\
        \x20           tail {{LoopVarList optional} {LoopVarList optional}}\n\
        \x20       }\n\
        \x20   }\n\
        }\n";

    /// The generic tail binds every variable-list operand a clause fills,
    /// not only the first: `cmd {} a b` binds `a` and `b`. Negative: the
    /// optional second slot left unfilled binds nothing more.
    #[test]
    fn a_clause_with_two_variable_lists_binds_both() {
        let both = analyser_with_pack(TWO_LISTS_PACK).analyse("cmd {} a b\n", D);
        for name in ["a", "b"] {
            assert!(
                both.global_scope.variables.contains_key(name),
                "`{name}` is bound: {:?}",
                both.global_scope.variables.keys().collect::<Vec<_>>()
            );
        }
        let one = analyser_with_pack(TWO_LISTS_PACK).analyse("cmd {} a\n", D);
        assert!(one.global_scope.variables.contains_key("a"));
        assert_eq!(
            one.global_scope.variables.len(),
            1,
            "{:?}",
            one.global_scope.variables.keys().collect::<Vec<_>>()
        );
    }

    /// A pack declaring an interpreter-provided global bound at startup.
    const HOST_PACK: &str = "speclib hosted 2.1 {\n\
        \x20   special_var sim_home -kind Scalar -access ReadOnly -origin Dialect -startup Interpreter\n\
        \x20   special_var sim_opts -kind Array -access ReadWrite -origin Environment\n\
        }\n";

    /// A pack's `special_var` row reaches the registry generation the
    /// analyser walks under through the one door every reader asks
    /// (`CommandRegistry::special_vars`), with the facts the shipped table
    /// states: `sim_home` is readable before user code, `sim_opts` exists but
    /// is not. Negative: the generation without the pack knows neither.
    #[test]
    fn a_pack_declared_special_variable_is_readable_at_startup() {
        let packs = tcl_spectcl::pack::load_in_memory(vec![(
            tcl_spectcl::PackFile {
                tier: tcl_spectcl::Tier::Workspace,
                path: std::path::PathBuf::from("/workspace/.tcl-lsp/hosted.tclspec"),
                origin: tcl_spectcl::discovery::Origin::DotDir,
                dependency_tier: None,
            },
            HOST_PACK.to_owned(),
        )]);
        assert!(
            !packs.is_empty(),
            "a pack of special variables is not empty"
        );
        let overlaid = tcl_spectcl::install::registry_for_dialect_with_packs(D, &packs);
        let query = Some(tcl_dialect::model::SurfaceQuery::core(
            tcl_dialect::model::Family::Tcl,
            "8.6",
        ));
        assert!(overlaid.is_readable_at_startup("sim_home", query));
        assert!(overlaid.special_var_in_dialect("sim_opts", query).is_some());
        assert!(!overlaid.is_readable_at_startup("sim_opts", query));
        // The shipped rows still answer through the same door.
        assert!(overlaid.is_readable_at_startup("tcl_version", query));

        let plain = tcl_spectcl::install::registry_for_dialect_with_packs(
            D,
            &tcl_spectcl::PackSet::default(),
        );
        assert!(plain.special_var("sim_home").is_none());
        assert!(!plain.is_readable_at_startup("sim_home", query));
    }
}
