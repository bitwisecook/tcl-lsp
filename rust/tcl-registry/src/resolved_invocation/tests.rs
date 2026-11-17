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

mod argument_count;
mod argument_roles;
mod case_clauses;
mod effects_and_transitions;
mod invocation_facts;
mod invocation_options;
mod operation_resolution;
mod source_definitions;
mod source_operands;
mod source_options;
mod source_scripts;

use super::*;
use crate::InvocationArguments;
use crate::world_effect::{
    CallbackKinds, EffectAccess, EffectAccessMode, EffectFootprint, StaticEffectAccess,
    StaticEffectFootprint, StaticInterpreterScope, StaticNamespaceScope, StaticSubjectScope,
    SubjectScope, WorldEffectComposition, WorldEffectDescriptor, WorldStateDomain,
};
use crate::{
    AbruptTransitionTransfer, CallerFrameSelection, CommandBindingDefinitionKind,
    CommandBindingTransition, CommandRegistry, CompletionCode, CompletionCodeDomain,
    CompletionDescriptor, DispatchDependencies, DispatchDependencyDescriptor,
    DispatchDependencyDomain, NamespaceTransition, NamespaceTransitionTarget, Reentrancy,
    StateTransition, StateTransitionCommit, StateTransitionDescriptor, StateTransitionDomain,
    StateTransitionKnowledge, TransitionSubject, VariableAliasTarget,
};
use tcl_dialect::model::{Family, SpecSurface, SurfaceQuery};

const COMMAND_CODES: &[CompletionCode] = &[CompletionCode::Error];
const SUBCOMMAND_CODES: &[CompletionCode] = &[CompletionCode::Break];
const FORM_CODES: &[CompletionCode] = &[CompletionCode::Other(71)];

const COMMAND_WORLD_ACCESSES: &[StaticEffectAccess] = &[StaticEffectAccess::new(
    WorldStateDomain::PackageState,
    EffectAccessMode::Read,
    StaticInterpreterScope::Current,
    StaticNamespaceScope::Current,
    StaticSubjectScope::Wildcard,
)];
const SUBCOMMAND_WORLD_ACCESSES: &[StaticEffectAccess] = &[StaticEffectAccess::new(
    WorldStateDomain::OoDispatch,
    EffectAccessMode::Read,
    StaticInterpreterScope::Current,
    StaticNamespaceScope::Current,
    StaticSubjectScope::Wildcard,
)];

const COMMAND_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: COMMAND_WORLD_ACCESSES,
        callback: crate::CallbackEffect::NONE,
    },
    resolver: None,
    source_resolver: None,
    dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};
const SUBCOMMAND_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: SUBCOMMAND_WORLD_ACCESSES,
        callback: crate::CallbackEffect::NONE,
    },
    resolver: None,
    source_resolver: None,
    dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

fn form_effect_resolver(args: InvocationArguments<'_>) -> EffectFootprint {
    let mut footprint = EffectFootprint::default();
    if let Some(subject) = args.literal_at(1) {
        footprint.add_access(EffectAccess::new(
            WorldStateDomain::VariableTraces,
            EffectAccessMode::Write,
            crate::InterpreterScope::Current,
            crate::NamespaceScope::Current,
            SubjectScope::named(subject),
        ));
    }
    footprint
}

const FORM_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(form_effect_resolver),
    source_resolver: None,
    dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};
const REFINING_FORM_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Replace,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(form_effect_resolver),
    source_resolver: None,
    dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

const WORLD_FORMS: &[CommandForm] = &[CommandForm {
    name: "argument-targeted",
    arity: Arity::exact(1),
    world_effects: crate::knowledge::Knowledge::Partial(FORM_WORLD),
    ..CommandForm::DEFAULT
}];
const WORLD_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "sub",
        arity: Arity::new(0, 1),
        subcommand_forms: WORLD_FORMS,
        world_effects: crate::knowledge::Knowledge::Partial(SUBCOMMAND_WORLD),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "refine",
        arity: Arity::exact(1),
        subcommand_forms: &[CommandForm {
            name: "precise-target",
            arity: Arity::exact(1),
            world_effects: crate::knowledge::Knowledge::Partial(REFINING_FORM_WORLD),
            ..CommandForm::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

const RUN_FORMS: &[CommandForm] = &[CommandForm {
    name: "custom-code-form",
    arity: Arity::exact(0),
    completion: Some(CompletionDescriptor::exact(FORM_CODES)),
    ..CommandForm::DEFAULT
}];

const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "run",
        arity: Arity::exact(0),
        completion: Some(CompletionDescriptor::exact(SUBCOMMAND_CODES)),
        subcommand_forms: RUN_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "plain",
        arity: Arity::exact(0),
        completion: Some(CompletionDescriptor::exact(SUBCOMMAND_CODES)),
        ..SubCommand::DEFAULT
    },
];

const SAFE_ON_UNINIT_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "narrow",
        arity: Arity::exact(0),
        safe_on_uninit: Some(SpecSurface::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inherit",
        arity: Arity::exact(0),
        ..SubCommand::DEFAULT
    },
];

const OBJECT_DEPENDENCY: DispatchDependencies =
    DispatchDependencies::one(DispatchDependencyDomain::ObjectDispatch);
const UNKNOWN_DEPENDENCY: DispatchDependencies =
    DispatchDependencies::one(DispatchDependencyDomain::UnknownHandling);
const DISPATCH_FORMS: &[CommandForm] = &[CommandForm {
    name: "guarded-form",
    arity: Arity::exact(0),
    dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(OBJECT_DEPENDENCY)),
    ..CommandForm::DEFAULT
}];
const DISPATCH_SUBCOMMANDS: &[SubCommand] = &[SubCommand {
    name: "run",
    arity: Arity::exact(0),
    subcommand_forms: DISPATCH_FORMS,
    dispatch_dependencies: Some(DispatchDependencyDescriptor::extend(UNKNOWN_DEPENDENCY)),
    ..SubCommand::DEFAULT
}];

fn registry_with_completion_fixture() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "completion-fixture",
        arity: Arity::any(),
        subcommands: SUBCOMMANDS,
        completion: Some(CompletionDescriptor::exact(COMMAND_CODES)),
        ..CommandSpec::DEFAULT
    });
    registry
}
