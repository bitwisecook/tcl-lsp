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

//! A variable reference in a pack ends where Tcl's scanner ends it (review
//! naming-migration-review-20261022, N17).
//!
//! For `$a:b` C Tcl substitutes `a` and appends the literal `:b`; a lone colon
//! is literal and a run of two or more is a namespace separator. A pack's
//! template and its live hooks are Tcl evaluated by the VM, so the command a
//! template registers and the value a hook folds follow that rule. tclsh
//! 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 evaluate each template below to
//! `good`, under a stand-in `speclib`/`command` driver, in every form here.

use std::rc::Rc;

use tcl_registry::pack_hooks;
use tcl_spec_hooks::tclvm_host;
use tcl_spectcl::discovery::{Origin, PackFile, Tier};
use tcl_spectcl::hooks;
use tcl_spectcl::loader::{EvalOptions, evaluate_pack_with};

const SETUP: &str = "set a x\n        set a:b BAD\n        set {a:} LONE\n        \
    set arr(x:b) good\n        set arr(BAD) bad\n        set arr(x:) good\n        \
    set arr(LONE) bad";

/// A template naming its command through `reference`, directly in the
/// `speclib` body or returned by a helper `proc`.
fn template(dsl: &str, helper: bool, reference: &str) -> String {
    if helper {
        format!(
            "speclib edge {dsl} {{\n    proc pick {{}} {{\n        {SETUP}\n        \
             return {reference}\n    }}\n    command [pick] {{arity 0}}\n}}\n"
        )
    } else {
        format!("speclib edge {dsl} {{\n        {SETUP}\n    command {reference} {{arity 0}}\n}}\n")
    }
}

#[test]
fn a_template_names_its_command_where_the_reference_ends() {
    // `$a:b` is `$a` then `:b`; `${a}:b` is the braced control; `$a:` keeps
    // its lone colon literal.
    for reference in ["$arr($a:b)", "$arr(${a}:b)", "$arr($a:)"] {
        for dsl in ["1", "2.0"] {
            for helper in [false, true] {
                for fast in [true, false] {
                    let source = template(dsl, helper, reference);
                    let pack = evaluate_pack_with(
                        &source,
                        &EvalOptions {
                            static_fast_path: fast,
                            ..EvalOptions::default()
                        },
                    );
                    let names: Vec<&str> = pack
                        .commands
                        .iter()
                        .map(|command| command.spec.name)
                        .collect();
                    assert_eq!(
                        names,
                        ["good"],
                        "{reference} dsl={dsl} helper={helper} fast={fast}: {:?}\n{source}",
                        pack.load_error
                    );
                }
            }
        }
    }
}

/// A run of colons is a namespace separator, however long.
#[test]
fn a_colon_run_in_a_template_is_a_namespace_separator() {
    for reference in ["$arr($a::b)", "$arr($a:::b)"] {
        for dsl in ["1", "2.0"] {
            let source = format!(
                "speclib edge {dsl} {{\n    namespace eval a {{variable b NSB}}\n    \
                 set a x\n    set a:b BAD\n    set arr(NSB) good\n    set arr(x:b) bad\n    \
                 command {reference} {{arity 0}}\n}}\n"
            );
            let pack = evaluate_pack_with(&source, &EvalOptions::default());
            let names: Vec<&str> = pack
                .commands
                .iter()
                .map(|command| command.spec.name)
                .collect();
            assert_eq!(
                names,
                ["good"],
                "{reference} dsl={dsl}: {:?}",
                pack.load_error
            );
        }
    }
}

/// The live hook folds the value its Tcl computes: `good`, not the cell a
/// whole-name reading of `$a:b` would select; `${a}:b` is the braced control.
#[test]
fn a_live_hook_folds_the_value_its_substitution_computes() {
    for (dsl, reference) in [
        ("1", "$arr($a:b)"),
        ("2.0", "$arr($a:b)"),
        ("1", "$arr(${a}:b)"),
        ("2.0", "$arr(${a}:b)"),
    ] {
        let source = format!(
            "speclib edge {dsl} {{\n    command edge::test {{\n        arity 1\n        \
             arg 0 -role Value\n        const_fold -inputs {{words}} {{words ctx}} {{\n            \
             set a x\n            set a:b BAD\n            set arr(x:b) good\n            \
             set arr(BAD) bad\n            fold {reference}\n        }}\n    }}\n}}\n"
        );
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-variable-reference-{dsl}-{}-{}-{:?}",
            reference.len(),
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join("edge.tclspec");
        std::fs::write(&path, &source).expect("write pack");
        let packs = tcl_spectcl::pack::load(&[PackFile {
            tier: Tier::Workspace,
            path,
            origin: Origin::DotDir,
            dependency_tier: None,
        }]);
        let plan = hooks::plan_for(&packs);
        assert!(
            !plan.is_empty(),
            "dsl={dsl} {reference}: the hook is planned"
        );
        let host = Rc::new(tclvm_host());
        for programs in plan.packs() {
            for installed in host.install_pack_hooks(programs.clone()) {
                assert!(
                    installed.declined.is_none(),
                    "dsl={dsl}: {:?}",
                    installed.declined
                );
            }
        }
        pack_hooks::install_host(host);
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
        let folded: Vec<String> = tcl_compiler::optimiser::optimise_raw(
            "proc ::f {} {\n    set y [edge::test abcde]\n}\n",
            &registry,
            None,
        )
        .into_iter()
        .filter(|optimisation| optimisation.code.as_str() == "O129")
        .map(|optimisation| optimisation.replacement)
        .collect();
        pack_hooks::clear_host();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(folded, ["good"], "dsl={dsl} {reference}");
    }
}

#[test]
fn a_live_fold_keeps_declared_effects_and_unclosed_arguments() {
    for dsl in ["1", "2.0"] {
        let source = format!(
            "speclib edge {dsl} {{\n    command edge::constant {{\n        arity 1\n        \
             arg 0 -role Value\n        const_fold -inputs {{words}} {{words ctx}} {{fold good}}\n    }}\n    \
             command edge::abstain {{\n        arity 1\n        \
             const_fold -inputs {{words}} {{words ctx}} {{return}}\n    }}\n    \
             command edge::ordinary {{arity 1}}\n}}\n"
        );
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-fold-effects-{dsl}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join("edge.tclspec");
        std::fs::write(&path, &source).expect("write pack");
        let packs = tcl_spectcl::pack::load(&[PackFile {
            tier: Tier::Workspace,
            path,
            origin: Origin::DotDir,
            dependency_tier: None,
        }]);
        assert!(packs.notices.is_empty(), "dsl={dsl}: {:?}", packs.notices);
        let plan = hooks::plan_for(&packs);
        let host = Rc::new(tclvm_host());
        for programs in plan.packs() {
            for installed in host.install_pack_hooks(programs.clone()) {
                assert!(
                    installed.declined.is_none(),
                    "dsl={dsl}: {:?}",
                    installed.declined
                );
            }
        }
        pack_hooks::install_host(host);
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
        let replacements = |registry: &tcl_registry::CommandRegistry, call: &str| {
            tcl_compiler::optimiser::optimise_raw(
                &format!("proc ::f {{x}} {{\n    set y [{call}]\n}}\n"),
                registry,
                None,
            )
            .into_iter()
            .filter(|optimisation| optimisation.code.as_str() == "O129")
            .map(|optimisation| optimisation.replacement)
            .collect::<Vec<_>>()
        };
        let original = registry
            .get("edge::constant")
            .expect("pack command")
            .clone();
        assert!(original.world_effects.is_none());
        assert_eq!(replacements(&registry, "edge::constant abcde"), ["good"]);
        for call in [
            "edge::abstain abcde",
            "edge::ordinary abcde",
            "edge::constant $x",
            "edge::constant [edge::ordinary abcde]",
            "edge::constant [unknown abcde]",
        ] {
            assert!(
                replacements(&registry, call).is_empty(),
                "dsl={dsl}: {call}"
            );
        }
        let mut registry = registry.project_for_profile(registry.profile().expect("pack profile"));
        let mut declared = original.clone();
        declared.world_effects =
            Some(tcl_registry::world_effect::WorldEffectDescriptor::VARIABLE_WRITE);
        assert_eq!(
            declared.run_const_fold(&["abcde"], None),
            Some("good".into())
        );
        registry.insert(declared);
        assert!(
            replacements(&registry, "edge::constant abcde").is_empty(),
            "dsl={dsl}: the selected declared callback footprint prevents erasure"
        );
        let mut pure = original;
        pure.world_effects = Some(tcl_registry::world_effect::WorldEffectDescriptor::EMPTY);
        registry.insert(pure);
        assert_eq!(replacements(&registry, "edge::constant abcde"), ["good"]);
        pack_hooks::clear_host();
        assert!(replacements(&registry, "edge::constant abcde").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
