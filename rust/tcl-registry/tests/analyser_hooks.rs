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
use tcl_registry::hooks::AnalyserHookId;
use tcl_registry::{CommandRegistry, Traits};
use tcl_dialect::model::Family;

/// Every dialect name that loads a non-trivial *compiled-in* command pack —
/// the same list `registry_sweep.rs` uses — so the sweep sees every spec,
/// including dialect twins (the iRules `proc`).
const LOADABLE_DIALECTS: &[&str] = &[
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "f5-irules",
    "f5-iapps",
    "expect",
    "bpf",
];

/// The shipped `.tclspec` loadables, which is where the EDA vendor libraries
/// live now — `sdc_base` and the five vendor packs have no Rust modules at
/// all (`docs/design/spec-packs.md`), so a stamp on `foreach_in_collection`
/// reaches this sweep only by way of the pack loader.
///
/// The path is the repository's `specs/`, which is exactly what a release
/// lays down beside the executable; naming it rather than relying on
/// discovery's defaults keeps the test independent of where the test binary
/// happens to sit.
fn bundled_packs() -> tcl_spectcl::PackSet {
    tcl_spectcl::bundled::load_from(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs"),
    )
}

/// A registry with every loadable dialect pack merged in — compiled-in
/// dialects by their `SpecSurface` bit, the EDA vendor libraries through the
/// `SpecTcl` loader.
fn full_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::build_default();
    for name in LOADABLE_DIALECTS {
        let profile = tcl_dialect::DialectProfile::find(name).expect("a compiled-in dialect name");
        for &layer in profile.base_layers {
            reg.load_surface(layer);
        }
    }
    let packs = bundled_packs();
    assert!(
        !packs.is_empty(),
        "the shipped EDA loadables must be present for this sweep to see them"
    );
    for pack in &packs.packs {
        for command in &pack.commands {
            reg.insert(command.spec.clone());
        }
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
        ("namespace", "export", H::NamespaceExport),
        // The removal half of the import edge's lifecycle: `namespace forget`
        // takes an imported alias away again, so a bare call after it stops
        // resolving (issue #1103).
        ("namespace", "forget", H::NamespaceForget),
        // `inscope` shares the namespace-eval handler: same `[subcmd, ns,
        // body]` shape, body analysed in the named namespace's scope.
        ("namespace", "inscope", H::NamespaceEval),
        ("namespace", "path", H::NamespacePath),
        ("namespace", "unknown", H::NamespaceUnknown),
        ("namespace", "upvar", H::NamespaceUpvar),
        // `matches!(cmd_name, "foreach" | "foreach_in_collection")` —
        // the EDA command shares the handler.
        ("foreach", "", H::Foreach),
        ("foreach_in_collection", "", H::Foreach),
        ("for", "", H::For),
        ("switch", "", H::Switch),
        // The obsolete Tcl 8.x `case` shares `switch`'s clause-list handler:
        // one subject then either separate `patList body` pairs or a single
        // braced clause list, with the differences (no options, glob-only
        // matching, the optional `in` separator) carried by its own
        // `CaseListSpec::CASE` descriptor rather than by handler code.
        ("case", "", H::Switch),
        // Expect's clause-list commands share the switch analyser's generic
        // case-list body walker; their grammar is carried by `case_list`.
        ("expect", "", H::Switch),
        ("expect_after", "", H::Switch),
        ("expect_before", "", H::Switch),
        ("expect_user", "", H::Switch),
        ("expect_tty", "", H::Switch),
        ("expect_background", "", H::Switch),
        ("catch", "", H::Catch),
        ("try", "", H::Try),
        ("upvar", "", H::Upvar),
        // handle_dict_var_command matched `args[0]` per subcommand.
        ("dict", "for", H::DictFor),
        ("dict", "update", H::DictUpdate),
        ("dict", "with", H::DictWith),
        // The standalone `::tcl::dict::*` spellings (issue #923 idx 105) now
        // carry each subcommand's own analyser hook too (Codex review, PR
        // #1020), so `::tcl::dict::for {k v} $d {…}` is analysed like `dict
        // for` — landing as a *command-level* stamp on the qualified spec.
        ("::tcl::dict::for", "", H::DictFor),
        ("::tcl::dict::update", "", H::DictUpdate),
        ("::tcl::dict::with", "", H::DictWith),
        // `cmd_name != "interp" || args[0] != "alias"` in
        // crate::alias's detectors, dispatched by handle_interp_alias.
        ("interp", "alias", H::InterpAlias),
        // `interp eval CHILD SCRIPT` — the child interpreter's script is
        // analysed in an isolated scope (handle_interp_eval_command).
        ("interp", "eval", H::InterpEval),
        // The interpreter model (issue #945 fault 8): lifecycle and
        // command-visibility subcommands stamp their own hooks so the
        // analyser tracks child-interp existence, temporal identity
        // (delete/recreate epochs), and safe-interp hide/expose state
        // without matching the command name.
        ("interp", "create", H::InterpCreate),
        ("interp", "delete", H::InterpDelete),
        ("interp", "hide", H::InterpHide),
        ("interp", "expose", H::InterpExpose),
        ("rename", "", H::Rename),
        ("oo::define", "", H::OoDefine),
        ("oo::objdefine", "", H::OoObjdefine),
        // No former per-handler guard — `tcl::OptProc` (the `opt`
        // package's automatic-option-parsing proc definer) never had a
        // hook at all until issue #923 idx 90 added one, so `all_procs`
        // kept the stub's `{}`-arity `ProcDef` for every real
        // redefinition.
        ("tcl::OptProc", "", H::OptProc),
        // handle_package_command matched `args[0]` require / provide.
        ("package", "require", H::PackageRequire),
        ("package", "provide", H::PackageProvide),
        // `package ifneeded` registers an arbitrary load script; the
        // package-derived load order reads its presence as "the
        // statements this require runs are not static" (issue #1279).
        ("package", "ifneeded", H::PackageIfneeded),
        // Post-dates the former guard list: `package prefer latest` raises
        // the interpreter's selection mode, which provider selection reads
        // at the require's own offset (issue #1126).
        ("package", "prefer", H::PackagePrefer),
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

/// The command-table transition declarations, pinned to the former name
/// matches in `tcl_compiler::command_binding::stmt_gen` (proc / rename /
/// interp) and `tcl_compiler::alias`'s retired detectors (interp alias /
/// rename).
///
/// Centralisation ledger C8: `proc`, `rename` and `interp alias` name the
/// stock descriptors directly and no longer stamp the coarse
/// `command_table_effect` selector beside them; a pack-authored spec that
/// can only write the selector resolves to the same stock descriptor. Both
/// routes are pinned here through the one consumer door.
#[test]
fn command_table_transitions_match_the_former_name_matches() {
    use tcl_registry::{CommandBindingTransition, InvocationWord, InvocationWords};

    /// The binding facts `head args…` states, under the plain profile.
    fn bindings(reg: &CommandRegistry, head: &str, args: &[&str]) -> Vec<CommandBindingTransition> {
        let words: Vec<InvocationWord<'_>> =
            args.iter().map(|a| InvocationWord::Literal(a)).collect();
        reg.command_binding_transitions(InvocationWords::structured(
            InvocationWord::Literal(head),
            &words,
        ))
        .command_bindings()
        .cloned()
        .collect()
    }

    let reg = full_registry();

    // The four declaration sites, by the shape each really mutates.
    // `proc` — Tcl_ProcObjCmd, tclProc.c.
    assert!(matches!(
        bindings(&reg, "proc", &["greet", "", ""]).as_slice(),
        [CommandBindingTransition::Define { .. }]
    ));
    // `rename` — Tcl_RenameObjCmd (tclCmdMZ.c) dispatching to
    // TclRenameCommand (tclBasic.c).
    assert!(matches!(
        bindings(&reg, "rename", &["format", "origfmt"]).as_slice(),
        [CommandBindingTransition::Move { .. }]
    ));
    // `interp alias` — AliasCreate, tclInterp.c; the descriptor rides the
    // `alias` subcommand.
    assert!(matches!(
        bindings(&reg, "interp", &["alias", "", "myfmt", "", "format"]).as_slice(),
        [CommandBindingTransition::Alias { .. }]
    ));
    // No former name match — `tcl::OptProc` genuinely defines a procedure
    // (issue #923 idx 90), same as `proc` itself. It reaches the same
    // vocabulary through the `command_table_effect` selector.
    assert!(matches!(
        bindings(&reg, "tcl::OptProc", &["greet", "", ""]).as_slice(),
        [CommandBindingTransition::Define { .. }]
    ));

    // The subcommand-over-spec rule and the exact-subcommand-spelling
    // contract the retired matches had. `interp aliases` is its own
    // subcommand and mutates nothing; a bare `interp` selects no
    // subcommand at all.
    assert!(bindings(&reg, "interp", &["aliases"]).is_empty());
    assert!(bindings(&reg, "interp", &[]).is_empty());
    // A `::`-qualified head states the same transitions as the bare one
    // (issue #1185): C Tcl resolves the explicitly global spelling to the
    // same command, so `::rename format ::origfmt` / `::interp alias {}
    // myfmt {} format` / `::proc ::greet {} {…}` really do mutate the
    // command table — verified byte-identical on tclsh 9.0.4 and 8.6.16.
    assert!(matches!(
        bindings(&reg, "::rename", &["format", "::origfmt"]).as_slice(),
        [CommandBindingTransition::Move { .. }]
    ));
    assert!(matches!(
        bindings(&reg, "::interp", &["alias", "", "myfmt", "", "format"]).as_slice(),
        [CommandBindingTransition::Alias { .. }]
    ));
    assert!(matches!(
        bindings(&reg, "::proc", &["::greet", "", ""]).as_slice(),
        [CommandBindingTransition::Define { .. }]
    ));
    // Still nothing for an unrelated qualified name or a non-mutating
    // subcommand word.
    assert!(bindings(&reg, "::interp", &["aliases"]).is_empty());
    assert!(bindings(&reg, "::puts", &["x"]).is_empty());

    // A `rename` at any arity but two is `wrong # args`, which moves
    // nothing (ledger C8: the arity rule lives with the resolver now,
    // where every consumer sees it).
    assert!(bindings(&reg, "rename", &["old"]).is_empty());
    assert!(bindings(&reg, "rename", &["a", "b", "c"]).is_empty());
}
