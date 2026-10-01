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

//! A reference body, run as the declared implementation of the command it
//! backs.
//!
//! A command a pack backs with a Tcl body says what runs when it is called. When
//! the registry's scan reads that body to the end and finds only commands the
//! bounded host runs ([`tcl_registry::value_transfer::reference_body`]), the
//! host can run it too, and the analyser asks it the answer to a call whose
//! arguments it knows. The derivation is an `evaluate -implementation` the pack
//! did not write: the same declaration, the same hook body on the same host, and
//! nothing for the author to maintain beside the body.
//!
//! It runs on the merged commands, after the capability gate has taken a body
//! the package may not declare and after the load has read the files a package
//! source names, so it sees exactly the bodies that are in force. A command whose
//! author stated its evaluation — a `semantics` row, an `evaluate` statement or
//! `semantics none` — is left as written.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use tcl_registry::arity::Arity;
use tcl_registry::pack_hooks::{HookInput, HookInputs};
use tcl_registry::spec::CommandSpec;
use tcl_registry::value_transfer::SemanticsDeclaration;
use tcl_registry::value_transfer::reference_body::{self, Inexpressible};
use tcl_registry::{BodySource, RuntimeBacking};

use super::semantics::EVALUATE_FIELD;
use super::{HookDecl, HookFamily, HookOwner, HookSource, PackCommand};
use crate::pack::{PackNotice, Severity};

/// Give each command of `commands` whose reference body the sandbox can run a
/// declared implementation and the hook body that is its text, and say on the
/// command's row why one the sandbox cannot run has none.
pub(crate) fn derive_implementations(commands: &mut [PackCommand], notices: &mut Vec<PackNotice>) {
    for command in commands {
        if let Some(why) = derive(command) {
            notices.push(PackNotice {
                path: command.file.clone(),
                line: command.line,
                context: format!("command {}", command.spec.name),
                message: format!(
                    "the reference body is not run as an implementation at analysis time: {why}"
                ),
                severity: Severity::Information,
            });
        }
    }
}

/// The reason `command` has no derived implementation, when it is one the
/// author can act on: nothing is said of a command with no body in hand or one
/// whose evaluation the author stated.
fn derive(command: &mut PackCommand) -> Option<String> {
    let spec = command.spec;
    let text = match spec.runtime_backing {
        RuntimeBacking::TclBody {
            source: BodySource::PackText { text },
        } => text,
        RuntimeBacking::TclBody {
            source: BodySource::PackageSource { .. },
        } => command.reference_text.as_deref()?,
        _ => return None,
    };
    // Whatever the author said of the command's evaluation — a `semantics` row,
    // an `evaluate` statement, `semantics none` — is the command's declaration.
    if !matches!(spec.semantics, SemanticsDeclaration::Inherited) {
        return None;
    }
    if !spec.subcommands.is_empty() || !spec.command_forms.is_empty() {
        return Some(
            "the command has subcommands or forms, which a body written for the whole command \
             does not index"
                .to_owned(),
        );
    }
    let (implementation, hook) = match reference_body::derive(spec.name, text) {
        Ok(derived) => derived,
        // A text that is not one `proc` is not a body at all, and nothing reads it
        // as code either.
        Err(Inexpressible::NotOneProc) => return None,
        Err(why) => return Some(why.to_string()),
    };
    let parameters = u16::try_from(hook.params.len()).unwrap_or(u16::MAX);
    if spec.arity != Arity::exact(parameters) || !spec.arity_windows.is_empty() {
        return Some(format!(
            "the body takes {parameters} parameter(s) and the command declares another arity: a \
             call the declaration admits would be answered by a body that was never given its \
             arguments"
        ));
    }
    command.spec = with_semantics(
        spec,
        SemanticsDeclaration::Declared(implementation.declared),
    );
    command.hooks.push(HookDecl {
        owner: HookOwner::Command,
        field: EVALUATE_FIELD,
        family: HookFamily::Evaluate,
        source: HookSource::Body {
            params: hook.params,
            body: hook.body,
            // The body reads its declared inputs, which arrive as the call's
            // words, as an `evaluate -implementation` body does.
            inputs: HookInputs::declared([HookInput::Words]),
        },
        line: command.line,
    });
    None
}

/// `spec` with `declaration` as its semantics, leaked once per spec and
/// declaration.
///
/// Keyed by address, as the capability gate's clones are: every spec a
/// [`PackCommand`] holds is `'static`, so its address is its identity, and a load
/// that finds the same pack again finds the same clone.
fn with_semantics(
    spec: &'static CommandSpec,
    declaration: SemanticsDeclaration,
) -> &'static CommandSpec {
    type Memo = HashMap<(usize, usize), &'static CommandSpec>;
    static CLONES: OnceLock<Mutex<Memo>> = OnceLock::new();
    let SemanticsDeclaration::Declared(declared) = declaration else {
        unreachable!("only a declared implementation is derived");
    };
    let key = (
        std::ptr::from_ref(spec).addr(),
        std::ptr::from_ref(declared).cast::<()>().addr(),
    );
    let mut memo = CLONES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(done) = memo.get(&key) {
        return done;
    }
    let mut clone = spec.clone();
    clone.semantics = declaration;
    let leaked: &'static CommandSpec = Box::leak(Box::new(clone));
    memo.insert(key, leaked);
    leaked
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tcl_registry::value_transfer::{DeclaredEvaluation, DeclaredSemantics, reference_body};

    use super::*;
    use crate::discovery::{Origin, PackFile, Tier};

    fn load(rows: &str, body: &str) -> crate::pack::PackSet {
        let source = format!(
            "speclib vendor 2.0 {{\n    command vendor::f {{\n        {rows}\n        \
             runtime_backing tcl-body {{-pack-text {{{body}}}}}\n    }}\n}}\n"
        );
        crate::pack::load_in_memory(vec![(
            PackFile {
                tier: Tier::Workspace,
                path: PathBuf::from("vendor.tclspec"),
                origin: Origin::Setting,
                dependency_tier: None,
            },
            source,
        )])
    }

    fn says(set: &crate::pack::PackSet, needle: &str) -> bool {
        set.notices.iter().any(|notice| {
            notice.severity == Severity::Information && notice.message.contains(needle)
        })
    }

    #[test]
    fn a_body_the_sandbox_runs_is_the_commands_declared_implementation() {
        let set = load("arity 2", "proc vendor::f {a b} {string cat $a $b}");
        let command = &set.packs[0].commands[0];
        let SemanticsDeclaration::Declared(declared) = command.spec.semantics else {
            panic!("derived: {:?}", command.spec.semantics);
        };
        let declared = declared.as_declared().expect("a declared implementation");
        let DeclaredEvaluation::Implementation(implementation) = declared.evaluation else {
            panic!("an implementation");
        };
        assert_eq!(implementation.slot, None, "the host plan binds the slot");
        assert_eq!(implementation.capability.inputs.len(), 2);
        let [hook] = &command.hooks[..] else {
            panic!("one hook: {:?}", command.hooks);
        };
        assert_eq!(
            (hook.owner.clone(), hook.field),
            (HookOwner::Command, "evaluate")
        );
        assert_eq!(hook.family, HookFamily::Evaluate);
        assert_eq!(hook.line, command.line);
        let HookSource::Body { params, inputs, .. } = &hook.source else {
            panic!("a Tcl body: {:?}", hook.source);
        };
        assert_eq!(params, &["a".to_owned(), "b".to_owned()]);
        assert_eq!(inputs, &HookInputs::declared([HookInput::Words]));
        assert!(
            !says(&set, "not run as an implementation"),
            "{:?}",
            set.notices
        );
    }

    #[test]
    fn a_body_the_sandbox_cannot_run_is_said_on_the_commands_row_and_derives_nothing() {
        for (body, reason) in [
            ("proc vendor::f {x} {upvar 1 $x y; set y}", "`upvar`"),
            ("proc vendor::f {x} {clock seconds}", "`clock`"),
            ("proc vendor::f {x} {expr {rand()}}", "`rand`"),
            ("proc vendor::f {x} {set ::g $x}", "variable"),
            (
                "proc vendor::f {x} {if {$x} {return 1}; return 2}",
                "`return`",
            ),
            ("proc vendor::f {x {y 1}} {set x}", "parameter `y`"),
        ] {
            let set = load("arity 1", body);
            let command = &set.packs[0].commands[0];
            assert!(
                matches!(command.spec.semantics, SemanticsDeclaration::Inherited),
                "{body}"
            );
            assert!(command.hooks.is_empty(), "{body}: {:?}", command.hooks);
            assert!(says(&set, reason), "{body}: {:?}", set.notices);
            let row = set
                .notices
                .iter()
                .find(|notice| notice.message.contains("not run as an implementation"))
                .expect("said");
            assert_eq!(row.context, "command vendor::f");
            assert_eq!(row.line, command.line);
        }
    }

    #[test]
    fn a_text_that_is_not_one_proc_is_no_body_and_draws_no_notice_of_this_kind() {
        for body in [
            "return 1",
            "proc other::g {x} {set x}",
            "set x 1\nproc vendor::f {x} {set x}",
        ] {
            let set = load("arity 1", body);
            assert!(
                !says(&set, "not run as an implementation"),
                "{body}: {:?}",
                set.notices
            );
            assert!(set.packs[0].commands[0].hooks.is_empty());
        }
    }

    #[test]
    fn what_the_author_states_about_evaluation_is_left_as_written() {
        let body = "proc vendor::f {x} {expr {$x * 2}}";

        // `semantics none` abstains, and stays that way.
        let none = load("arity 1\n        semantics none", body);
        let command = &none.packs[0].commands[0];
        assert!(matches!(
            command.spec.semantics,
            SemanticsDeclaration::Declined
        ));
        assert!(command.hooks.is_empty(), "{:?}", command.hooks);

        // A `semantics` row without a route stays without one: nothing runs.
        let rows = load(
            "arity 1\n        semantics {effects {no_store_writes}}",
            body,
        );
        let command = &rows.packs[0].commands[0];
        let SemanticsDeclaration::Declared(declared) = command.spec.semantics else {
            panic!("the author's declaration: {:?}", command.spec.semantics);
        };
        let declared = declared.as_declared().expect("a declared semantics");
        assert!(
            declared.implementation().is_none(),
            "no implementation the author did not write: {declared:?}"
        );
        assert!(command.hooks.is_empty(), "{:?}", command.hooks);

        // An `evaluate` statement is the command's implementation, with its own body.
        let written = load(
            "arity 1\n        evaluate -implementation vendor.f.v1 -host bounded_tcl {\n            \
             inputs {arg 0 exact}\n            body {x} {fold [string cat mine $x]}\n        }",
            body,
        );
        let command = &written.packs[0].commands[0];
        let SemanticsDeclaration::Declared(declared) = command.spec.semantics else {
            panic!("the author's declaration: {:?}", command.spec.semantics);
        };
        let implementation = declared
            .as_declared()
            .and_then(DeclaredSemantics::implementation)
            .expect("the author's implementation");
        assert_eq!(implementation.capability.identity.id, "vendor.f.v1");
        let [hook] = &command.hooks[..] else {
            panic!("the author's one body: {:?}", command.hooks);
        };
        let HookSource::Body { body, .. } = &hook.source else {
            panic!("a body");
        };
        assert!(body.contains("mine"), "{body}");

        for set in [&none, &rows, &written] {
            assert!(
                !says(set, "not run as an implementation"),
                "{:?}",
                set.notices
            );
        }
    }

    #[test]
    fn the_declared_arity_must_be_the_bodys_parameters() {
        for rows in [
            "arity 1..2",
            "arity 0..",
            "arity 2",
            "arity 1\n        arity 1..2 -introduced 2.0",
        ] {
            let set = load(rows, "proc vendor::f {x} {expr {$x * 2}}");
            let command = &set.packs[0].commands[0];
            assert!(
                matches!(command.spec.semantics, SemanticsDeclaration::Inherited),
                "{rows}: derived against another arity"
            );
            assert!(says(&set, "another arity"), "{rows}: {:?}", set.notices);
        }
        let set = load("arity 1", "proc vendor::f {x} {expr {$x * 2}}");
        assert!(matches!(
            set.packs[0].commands[0].spec.semantics,
            SemanticsDeclaration::Declared(_)
        ));
    }

    #[test]
    fn a_command_with_subcommands_is_not_derived() {
        let set = load(
            "arity 1..\n        subcommand get {\n            arity 1\n        }",
            "proc vendor::f {x} {expr {$x * 2}}",
        );
        let command = &set.packs[0].commands[0];
        assert!(command.hooks.is_empty());
        assert!(says(&set, "subcommands or forms"), "{:?}", set.notices);
    }

    #[test]
    fn a_derivation_is_one_clone_per_spec_and_declaration() {
        let set = load("arity 1", "proc vendor::f {x} {expr {$x * 2}}");
        let spec = set.packs[0].commands[0].spec;
        let (implementation, _) =
            reference_body::derive("vendor::f", "proc vendor::f {x} {expr {$x * 3}}")
                .expect("expressible");
        let declare = || SemanticsDeclaration::Declared(implementation.declared);
        assert!(std::ptr::eq(
            with_semantics(spec, declare()),
            with_semantics(spec, declare())
        ));
        let (other, _) = reference_body::derive("vendor::f", "proc vendor::f {x} {expr {$x * 4}}")
            .expect("expressible");
        assert!(!std::ptr::eq(
            with_semantics(spec, declare()),
            with_semantics(spec, SemanticsDeclaration::Declared(other.declared))
        ));
    }

    /// The registry's scan, the hook host's sandbox and the loader's record of it
    /// name the same commands: a body the scan reads is one the host runs, and a
    /// command the host gains is a decision the scan's author makes.
    #[test]
    fn the_scans_whitelist_is_the_hosts() {
        assert_eq!(
            reference_body::SANDBOX_WORDS,
            tcl_spec_hooks::SANDBOX_COMMANDS
        );
        assert_eq!(
            super::super::SANDBOX_COMMANDS,
            tcl_spec_hooks::SANDBOX_COMMANDS
        );
    }
}
