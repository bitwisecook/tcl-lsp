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

//! Per-domain walks over the subjects a transition names, and which subjects carry native identity.

use super::namespace_transition::{
    EnsembleCommandName, NamespaceTransition, NamespaceTransitionTarget,
};
use super::{
    CommandBindingTransition, InterpreterTransition, ObjectDispatchTarget,
    ObjectDispatchTransition, ObjectPrivateNamespace, TransitionSubject,
};
use crate::model::binding::PackageTransition;

fn visit_namespace_subject(
    target: &mut NamespaceTransitionTarget,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    if let NamespaceTransitionTarget::Named(subject) = target {
        visit(subject);
    }
}

fn visit_object_subject(
    target: &mut ObjectDispatchTarget,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    if let ObjectDispatchTarget::Named(subject) = target {
        visit(subject);
    }
}

fn visit_private_subject(
    target: &mut ObjectPrivateNamespace,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    if let ObjectPrivateNamespace::Named(subject) = target {
        visit(subject);
    }
}

pub(super) fn transition_commandbinding_identity(
    transition: &CommandBindingTransition,
    subject: &TransitionSubject,
) -> bool {
    match transition {
        CommandBindingTransition::Define { name, .. } => name == subject,
        CommandBindingTransition::Move { from, to } => from == subject || to == subject,
        CommandBindingTransition::Delete { interpreter, name } => {
            name == subject || interpreter.as_ref() == Some(subject)
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            ..
        } => [source_interpreter, alias, target_interpreter, target].contains(&subject),
        CommandBindingTransition::Unknown { .. } => false,
    }
}

pub(super) fn transition_interpreter_identity(
    transition: &InterpreterTransition,
    subject: &TransitionSubject,
) -> bool {
    match transition {
        InterpreterTransition::Create { interpreter, .. } => interpreter.as_ref() == Some(subject),
        InterpreterTransition::Delete { interpreter }
        | InterpreterTransition::MarkTrusted { interpreter }
        | InterpreterTransition::SetRecursionLimit { interpreter, .. }
        | InterpreterTransition::SetBackgroundError { interpreter, .. } => interpreter == subject,
        InterpreterTransition::Hide {
            interpreter,
            visible,
            hidden,
        }
        | InterpreterTransition::Expose {
            interpreter,
            hidden,
            visible,
        } => [interpreter, visible, hidden].contains(&subject),
    }
}

pub(super) fn transition_namespace_identity(
    transition: &NamespaceTransition,
    subject: &TransitionSubject,
) -> bool {
    let namespace = |target: &NamespaceTransitionTarget| matches!(target, NamespaceTransitionTarget::Named(name) if name == subject);
    match transition {
        NamespaceTransition::Ensure { namespace: target }
        | NamespaceTransition::Delete { namespace: target }
        | NamespaceTransition::Ensemble {
            namespace: target, ..
        }
        | NamespaceTransition::SetPath {
            namespace: target, ..
        }
        | NamespaceTransition::SetUnknown {
            namespace: target, ..
        } => namespace(target),
        NamespaceTransition::Export {
            namespace: target,
            patterns,
        }
        | NamespaceTransition::Import {
            namespace: target,
            patterns,
            ..
        }
        | NamespaceTransition::Forget {
            namespace: target,
            patterns,
        } => namespace(target) || patterns.contains(subject),
    }
}

pub(super) fn transition_objectdispatch_identity(
    transition: &ObjectDispatchTransition,
    subject: &TransitionSubject,
) -> bool {
    let object = |target: &ObjectDispatchTarget| matches!(target, ObjectDispatchTarget::Named(name) if name == subject);
    let private = |target: &ObjectPrivateNamespace| matches!(target, ObjectPrivateNamespace::Named(name) if name == subject);
    match transition {
        ObjectDispatchTransition::Configure { target, .. }
        | ObjectDispatchTransition::Destroy { target } => target == subject,
        ObjectDispatchTransition::Create {
            target,
            private_namespace,
            ..
        } => object(target) || private(private_namespace),
        ObjectDispatchTransition::Copy {
            source,
            target,
            private_namespace,
        } => source == subject || object(target) || private(private_namespace),
    }
}

pub(super) fn transition_package_identity(
    transition: &crate::model::binding::PackageTransition,
    subject: &TransitionSubject,
) -> bool {
    match transition {
        PackageTransition::Provide { package, .. }
        | PackageTransition::Require { package, .. }
        | PackageTransition::Ifneeded { package, .. } => package == subject,
        PackageTransition::Forget { packages } => packages.contains(subject),
        PackageTransition::DiscoveryDependencyChanged { .. }
        | PackageTransition::UnknownHandler { .. }
        | PackageTransition::Prefer { .. }
        | PackageTransition::IndexedAutoload { .. }
        | PackageTransition::SourceLoad { .. } => false,
    }
}

pub(super) fn transition_commandbinding_visit(
    transition: &mut CommandBindingTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        CommandBindingTransition::Define { name, .. } => visit(name),
        CommandBindingTransition::Move { from, to } => {
            visit(from);
            visit(to);
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            if let Some(interpreter) = interpreter {
                visit(interpreter);
            }
            visit(name);
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            ..
        } => {
            visit(source_interpreter);
            visit(alias);
            visit(target_interpreter);
            visit(target);
            for argument in arguments {
                visit(argument);
            }
        }
        CommandBindingTransition::Unknown { operands } => {
            for operand in operands {
                visit(operand);
            }
        }
    }
}

pub(super) fn transition_interpreter_visit(
    transition: &mut InterpreterTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        InterpreterTransition::Create { interpreter, .. } => {
            if let Some(interpreter) = interpreter {
                visit(interpreter);
            }
        }
        InterpreterTransition::Delete { interpreter }
        | InterpreterTransition::MarkTrusted { interpreter } => visit(interpreter),
        InterpreterTransition::SetRecursionLimit { interpreter, limit } => {
            visit(interpreter);
            visit(limit);
        }
        InterpreterTransition::SetBackgroundError {
            interpreter,
            handler,
        } => {
            visit(interpreter);
            visit(handler);
        }
        InterpreterTransition::Hide {
            interpreter,
            visible,
            hidden,
        }
        | InterpreterTransition::Expose {
            interpreter,
            hidden,
            visible,
        } => {
            visit(interpreter);
            visit(visible);
            visit(hidden);
        }
    }
}

pub(super) fn transition_namespace_visit(
    transition: &mut NamespaceTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        NamespaceTransition::Ensure { namespace: target }
        | NamespaceTransition::Delete { namespace: target } => {
            visit_namespace_subject(target, visit);
        }
        NamespaceTransition::Ensemble {
            namespace: target,
            arguments,
            created,
            ..
        } => {
            visit_namespace_subject(target, visit);
            for argument in arguments {
                visit(argument);
            }
            if let Some(EnsembleCommandName::Named(subject)) = created {
                visit(subject);
            }
        }
        NamespaceTransition::Export {
            namespace: target,
            patterns,
        }
        | NamespaceTransition::Import {
            namespace: target,
            patterns,
            ..
        }
        | NamespaceTransition::Forget {
            namespace: target,
            patterns,
        } => {
            visit_namespace_subject(target, visit);
            for pattern in patterns {
                visit(pattern);
            }
        }
        NamespaceTransition::SetPath {
            namespace: target,
            path,
        } => {
            visit_namespace_subject(target, visit);
            visit(path);
        }
        NamespaceTransition::SetUnknown {
            namespace: target,
            handler,
        } => {
            visit_namespace_subject(target, visit);
            visit(handler);
        }
    }
}

pub(super) fn transition_objectdispatch_visit(
    transition: &mut ObjectDispatchTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        ObjectDispatchTransition::Configure { target, .. }
        | ObjectDispatchTransition::Destroy { target } => visit(target),
        ObjectDispatchTransition::Create {
            target,
            private_namespace,
            ..
        } => {
            visit_object_subject(target, visit);
            visit_private_subject(private_namespace, visit);
        }
        ObjectDispatchTransition::Copy {
            source,
            target,
            private_namespace,
        } => {
            visit(source);
            visit_object_subject(target, visit);
            visit_private_subject(private_namespace, visit);
        }
    }
}

pub(super) fn transition_package_visit(
    transition: &mut crate::model::binding::PackageTransition,
    visit: &mut impl FnMut(&mut TransitionSubject),
) {
    match transition {
        PackageTransition::DiscoveryDependencyChanged { .. } => {}
        PackageTransition::Provide { package, version } => {
            visit(package);
            if let Some(version) = version {
                visit(version);
            }
        }
        PackageTransition::Require {
            package,
            requirements,
            ..
        } => {
            visit(package);
            for requirement in requirements {
                visit(requirement);
            }
        }
        PackageTransition::Ifneeded {
            package,
            version,
            script,
            ..
        } => {
            visit(package);
            visit(version);
            if let Some(script) = script {
                visit(script);
            }
        }
        PackageTransition::Forget { packages } => {
            for package in packages {
                visit(package);
            }
        }
        PackageTransition::UnknownHandler { handler } => {
            if let Some(handler) = handler {
                visit(handler);
            }
        }
        PackageTransition::Prefer { mode } => {
            if let Some(mode) = mode {
                visit(mode);
            }
        }
        PackageTransition::IndexedAutoload { command } => visit(command),
        PackageTransition::SourceLoad { path } => visit(path),
    }
}
