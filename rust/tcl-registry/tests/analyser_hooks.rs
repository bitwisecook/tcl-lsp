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

//! Drift tests for the analyser-hook and command-table-effect stamps.
//!
//! The analyser's central dispatch and the command-table consumers
//! (`command_binding`, the lowerer's alias table) retired their
//! per-command name guards in favour of these registry stamps.  These
//! tests pin the stamped sets to exactly the former guard lists, so a
//! stamp cannot silently appear, vanish, or move without this file
//! changing alongside it.

use std::collections::BTreeSet;

use tcl_registry::dialects::DialectSet;
use tcl_registry::hooks::AnalyserHookId;
use tcl_registry::{CommandRegistry, CommandTableEffect, Traits};

/// Every dialect name that loads a non-trivial command pack — the same
/// list `registry_sweep.rs` uses — so the sweep sees every spec,
/// including dialect twins (the iRules `proc`) and dialect-only
/// commands (the EDA `foreach_in_collection`).
const LOADABLE_DIALECTS: &[&str] = &[
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "f5-irules",
    "f5-iapps",
    "expect",
    "synopsys-eda-tcl",
    "cadence-eda-tcl",
    "xilinx-eda-tcl",
    "intel-quartus-eda-tcl",
    "mentor-eda-tcl",
    "bpf",
];

/// A registry with every loadable dialect pack merged in.
fn full_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::build_default();
    for name in LOADABLE_DIALECTS {
        let dialect = DialectSet::parse(name).expect("loadable dialect parses");
        reg.load_dialect(dialect);
    }
    reg
}

/// The analyser-hook stamps, pinned to the former per-handler
/// command-name guard list of
/// `tcl_compiler::analyser::commands::dispatch_command_handlers`.
/// Keys are `(command, subcommand)`; a command-level stamp has an
/// empty subcommand.
#[test]
fn analyser_hook_stamps_match_the_former_guard_list() {
    use AnalyserHookId as H;
    let expected: BTreeSet<(&str, &str, H)> = [
        // `if cmd_name != "set"` in handle_set_command (+ the
        // `set auto_path PATH` arm of handle_auto_path_command).
        ("set", "", H::Set),
        // `matches!(cmd_name, "variable" | "global")` in
        // handle_var_declaration_command.
        ("variable", "", H::Variable),
        ("global", "", H::Global),
        // `if cmd_name != "proc"` in handle_proc_command — the iRules
        // pack re-registers `proc`, so both specs carry the stamp.
        ("proc", "", H::Proc),
        // `if cmd_name != "apply"` in parse_apply_lambda_elements.
        ("apply", "", H::Apply),
        // `cmd_name != "uplevel" || args[0] != "#0"` — the level word
        // stays a handler shape check.
        ("uplevel", "", H::Uplevel),
        // The `namespace` guards distinguished subcommands, so the
        // stamps live on the SubCommand entries.
        ("namespace", "eval", H::NamespaceEval),
        ("namespace", "ensemble", H::NamespaceEnsemble),
        ("namespace", "import", H::NamespaceImport),
        ("namespace", "path", H::NamespacePath),
        ("namespace", "unknown", H::NamespaceUnknown),
        ("namespace", "upvar", H::NamespaceUpvar),
        // `matches!(cmd_name, "foreach" | "foreach_in_collection")` —
        // the EDA command shares the handler.
        ("foreach", "", H::Foreach),
        ("foreach_in_collection", "", H::Foreach),
        ("for", "", H::For),
        ("switch", "", H::Switch),
        ("catch", "", H::Catch),
        ("try", "", H::Try),
        ("upvar", "", H::Upvar),
        // handle_dict_var_command matched `args[0]` per subcommand.
        ("dict", "for", H::DictFor),
        ("dict", "update", H::DictUpdate),
        ("dict", "with", H::DictWith),
        // `cmd_name != "interp" || args[0] != "alias"` in
        // crate::alias's detectors, dispatched by handle_interp_alias.
        ("interp", "alias", H::InterpAlias),
        ("rename", "", H::Rename),
        ("oo::define", "", H::OoDefine),
        ("oo::objdefine", "", H::OoObjdefine),
        // handle_package_command matched `args[0]` require / provide.
        ("package", "require", H::PackageRequire),
        ("package", "provide", H::PackageProvide),
        ("source", "", H::Source),
        ("append", "", H::Append),
        // `lappend` additionally feeds the `lappend auto_path …` arm.
        ("lappend", "", H::Lappend),
        ("regexp", "", H::RegexPatternCapture),
        ("regsub", "", H::RegexPatternCapture),
        ("incr", "", H::Incr),
        // `cmd_name == "load"` in dispatch_command_handlers itself.
        ("load", "", H::Load),
    ]
    .into_iter()
    .collect();

    let reg = full_registry();
    let mut actual: BTreeSet<(&str, &str, AnalyserHookId)> = BTreeSet::new();
    for name in reg.command_names() {
        for spec in reg.specs(name) {
            if let Some(hook) = spec.analyser_hook {
                actual.insert((spec.name, "", hook));
            }
            for sub in spec.subcommands {
                if let Some(hook) = sub.analyser_hook {
                    actual.insert((spec.name, sub.name, hook));
                }
            }
        }
    }

    let missing: Vec<_> = expected.difference(&actual).collect();
    let extra: Vec<_> = actual.difference(&expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "analyser-hook stamps drifted from the pinned guard list\nmissing: {missing:?}\nextra: {extra:?}"
    );
}

/// Dialect twins must agree: every spec sharing a stamped command name
/// carries the same command-level hook, so a dialect-scoped lookup
/// (`get` picks the last-registered spec) can never resolve a
/// different hook than the pinned one.
#[test]
fn analyser_hook_stamps_agree_across_dialect_twins() {
    let reg = full_registry();
    for name in reg.command_names() {
        let specs = reg.specs(name);
        let hooks: BTreeSet<_> = specs.iter().map(|s| s.analyser_hook).collect();
        assert!(
            hooks.len() <= 1,
            "specs named {name:?} disagree on analyser_hook: {hooks:?}"
        );
        let effects: BTreeSet<_> = specs.iter().map(|s| s.command_table_effect).collect();
        assert!(
            effects.len() <= 1,
            "specs named {name:?} disagree on command_table_effect: {effects:?}"
        );
    }
}

/// The analyser dispatch runs the definition-grammar-driven definer
/// handlers (`TclOO` metaclass / snit / itcl) only when no hook
/// matched, so a hook-stamped spec must never also satisfy one of
/// those handlers' registry conditions.  (`oo::define` carries a
/// `TclOO` *grammar* but not the metaclass trait, so it stays
/// disjoint.)
#[test]
fn analyser_hook_stamps_are_disjoint_from_definer_families() {
    use tcl_registry::definer::DefinerFamily;
    let reg = full_registry();
    for name in reg.command_names() {
        for spec in reg.specs(name) {
            if spec.analyser_hook.is_none() {
                continue;
            }
            let family = spec.definition_body.map(|g| g.family);
            let is_metaclass_definer = spec.traits.contains(Traits::IS_OO_METACLASS)
                && family == Some(DefinerFamily::TclOo);
            let is_snit_or_itcl = matches!(family, Some(DefinerFamily::Snit | DefinerFamily::Itcl));
            assert!(
                !is_metaclass_definer && !is_snit_or_itcl,
                "{name:?} carries an analyser hook AND a definer-family condition — \
                 the dispatch order would change"
            );
        }
    }
}

/// The command-table-effect stamps, pinned to the former name matches
/// in `tcl_compiler::command_binding::stmt_gen` (proc / rename /
/// interp) and `tcl_compiler::alias`'s detectors (interp alias /
/// rename).
#[test]
fn command_table_effect_stamps_match_the_former_name_matches() {
    use CommandTableEffect as E;
    let expected: BTreeSet<(&str, &str, E)> = [
        // `"proc" =>` in stmt_gen — Tcl_ProcObjCmd, tclProc.c.
        ("proc", "", E::DefinesProcedure),
        // `"rename" =>` in stmt_gen / `cmd_name != "rename"` in
        // detect_rename — Tcl_RenameObjCmd (tclCmdMZ.c) dispatching to
        // TclRenameCommand (tclBasic.c).
        ("rename", "", E::RenamesCommands),
        // `"interp" =>` + `args[0] == "alias"` — AliasCreate,
        // tclInterp.c; the effect rides the `alias` subcommand.
        ("interp", "alias", E::CreatesAliases),
    ]
    .into_iter()
    .collect();

    let reg = full_registry();
    let mut actual: BTreeSet<(&str, &str, E)> = BTreeSet::new();
    for name in reg.command_names() {
        for spec in reg.specs(name) {
            if let Some(effect) = spec.command_table_effect {
                actual.insert((spec.name, "", effect));
            }
            for sub in spec.subcommands {
                if let Some(effect) = sub.command_table_effect {
                    actual.insert((spec.name, sub.name, effect));
                }
            }
        }
    }

    let missing: Vec<_> = expected.difference(&actual).collect();
    let extra: Vec<_> = actual.difference(&expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "command-table-effect stamps drifted from the pinned list\nmissing: {missing:?}\nextra: {extra:?}"
    );

    // The resolver honours the subcommand-over-spec rule and the
    // exact-spelling contract the retired matches had.
    assert_eq!(
        reg.command_table_effect("proc", Some("x")),
        Some(E::DefinesProcedure)
    );
    assert_eq!(
        reg.command_table_effect("rename", Some("old")),
        Some(E::RenamesCommands)
    );
    assert_eq!(
        reg.command_table_effect("interp", Some("alias")),
        Some(E::CreatesAliases)
    );
    assert_eq!(reg.command_table_effect("interp", Some("aliases")), None);
    assert_eq!(reg.command_table_effect("interp", None), None);
    // A `::`-qualified head never matched the retired literals.
    assert_eq!(reg.command_table_effect("::rename", Some("old")), None);
}
