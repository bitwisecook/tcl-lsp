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

//! Conditional source class inventory for Tcllib Clay.

use crate::prelude::*;

const SCRIPT_FORM: CommandForm = CommandForm {
    name: "definition-script",
    arity: Arity::exact(2),
    arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
    ..CommandForm::DEFAULT
};

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clay::define",
        traits: Traits::CREATES_BARRIER
            | Traits::NEVER_INLINE_BODY
            | Traits::CREATES_DYNAMIC_BARRIER,
        arity: Arity::at_least(1),
        command_forms: &[SCRIPT_FORM],
        side_effects: SIDE_EFFECTS,
        body_kind: BodyKind::Structural,
        tcllib_package: Some("clay"),
        required_package: Some("clay"),
        definition_body: Some(&crate::definer::CLAY_GRAMMAR),
        runtime_backing: RuntimeBacking::None,
        forms: &[
            FormSpec {
                synopsis: "clay::define class definitionScript",
                ..FormSpec::DEFAULT
            },
            FormSpec {
                synopsis: "clay::define class subcommand ?arg ...?",
                ..FormSpec::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Create or extend a Clay class.",
            synopsis: &["clay::define class definitionScript"],
            snippet: "Creates or extends a Clay class using definition commands. The script may declare methods and configure Clay metadata.",
            source: "Tcllib 2.0 Clay 0.8.8",
            examples: "clay::define Server {method connect {sock ip port} {}}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use crate::{ArgRole, CommandRegistry};

    #[test]
    fn only_the_exact_script_form_supplies_class_header_roles() {
        let registry = CommandRegistry::build_default();
        let arguments = ["::Probe", "method connect {} {}"];
        let schema = registry
            .resolve_invocation("clay::define", &arguments, None)
            .unwrap();
        assert_eq!(
            schema.authored_source_argument_roles(),
            (vec![(0, ArgRole::Name), (1, ArgRole::Body)], true)
        );
        for arguments in [
            vec!["::Probe"],
            vec!["::Probe", "method", "connect", "{}", "{}"],
        ] {
            let schema = registry
                .resolve_invocation("clay::define", &arguments, None)
                .unwrap();
            assert!(schema.authored_source_argument_roles().0.is_empty());
        }
    }
}
