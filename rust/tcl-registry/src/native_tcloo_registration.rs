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

//! Audited native `TclOO` command registrations, independent of handler facts.
//!
//! The pinned C8.6 tclOO.c definition tables use `Tcl_CreateObjCommand`;
//! C9.0/C9.1 use `TclCreateObjCommandInNs`. Neither loop sets compileProc.
//! tclBasic.c initializes it to NULL. Definition slots in tclOODefineCmds.c
//! are object commands created by `AllocObject`, also without compileProc.
//! Helpers next/nextto/self explicitly install hooks and are excluded here.

use crate::native_compilation::{
    NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
};
use crate::{InvocationDialect, ObjectDispatchLayer, SemanticOperationId};
use tcl_dialect::{TclVersion, model::Family};

struct DefinitionRegistration {
    member: &'static str,
    class: Option<&'static str>,
    object: Option<&'static str>,
    first: TclVersion,
}

const DEFINITIONS: &[DefinitionRegistration] = &[
    DefinitionRegistration {
        member: "constructor",
        class: Some("oo::define::constructor"),
        object: None,
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "deletemethod",
        class: Some("oo::define::deletemethod"),
        object: Some("oo::objdefine::deletemethod"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "destructor",
        class: Some("oo::define::destructor"),
        object: None,
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "export",
        class: Some("oo::define::export"),
        object: Some("oo::objdefine::export"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "forward",
        class: Some("oo::define::forward"),
        object: Some("oo::objdefine::forward"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "method",
        class: Some("oo::define::method"),
        object: Some("oo::objdefine::method"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "renamemethod",
        class: Some("oo::define::renamemethod"),
        object: Some("oo::objdefine::renamemethod"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "self",
        class: Some("oo::define::self"),
        object: None,
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "unexport",
        class: Some("oo::define::unexport"),
        object: Some("oo::objdefine::unexport"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "class",
        class: None,
        object: Some("oo::objdefine::class"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "filter",
        class: Some("oo::define::filter"),
        object: Some("oo::objdefine::filter"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "mixin",
        class: Some("oo::define::mixin"),
        object: Some("oo::objdefine::mixin"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "superclass",
        class: Some("oo::define::superclass"),
        object: None,
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "variable",
        class: Some("oo::define::variable"),
        object: Some("oo::objdefine::variable"),
        first: TclVersion::V8_6,
    },
    DefinitionRegistration {
        member: "classmethod",
        class: Some("oo::define::classmethod"),
        object: None,
        first: TclVersion::V9_0,
    },
    DefinitionRegistration {
        member: "definitionnamespace",
        class: Some("oo::define::definitionnamespace"),
        object: None,
        first: TclVersion::V9_0,
    },
    DefinitionRegistration {
        member: "initialise",
        class: Some("oo::define::initialise"),
        object: None,
        first: TclVersion::V9_0,
    },
    DefinitionRegistration {
        member: "initialize",
        class: Some("oo::define::initialize"),
        object: None,
        first: TclVersion::V9_0,
    },
    DefinitionRegistration {
        member: "private",
        class: Some("oo::define::private"),
        object: Some("oo::objdefine::private"),
        first: TclVersion::V9_0,
    },
    DefinitionRegistration {
        member: "self",
        class: None,
        object: Some("oo::objdefine::self"),
        first: TclVersion::V9_0,
    },
];

fn version(dialect: InvocationDialect) -> Option<TclVersion> {
    (dialect.family() == Some(Family::Tcl))
        .then_some(dialect.tcl_version?)
        .filter(|version| *version >= TclVersion::V8_6)
}

/// Select the actual qualified worker for a proved active native definition
/// context. This metadata does not establish a live stock binding or authorize
/// resolving the bare member outside that context.
#[must_use]
pub fn definition_identity(
    identity: &str,
    layer: ObjectDispatchLayer,
    dialect: InvocationDialect,
) -> Option<&'static str> {
    let version = version(dialect)?;
    DEFINITIONS
        .iter()
        .filter(|row| row.member == identity && version >= row.first)
        .find_map(|row| match layer {
            ObjectDispatchLayer::Class => row.class,
            ObjectDispatchLayer::Object => row.object,
        })
}

/// An explicitly audited absent compiler hook at an exact native registration.
/// The issuer/version must be known; package replacements, bare context names,
/// real compiler-hook helpers and unlisted registrations retain uncertainty.
#[must_use]
pub fn compilation(identity: &str, dialect: InvocationDialect) -> Option<NativeCompilationSpec> {
    let version = version(dialect)?;
    let identity = identity.strip_prefix("::").unwrap_or(identity);
    let definition = DEFINITIONS.iter().any(|row| {
        version >= row.first && (row.class == Some(identity) || row.object == Some(identity))
    });
    let core_helper = version >= TclVersion::V9_0
        && matches!(
            identity,
            "oo::Helpers::callback"
                | "oo::Helpers::mymethod"
                | "oo::Helpers::classvariable"
                | "oo::Helpers::link"
                | "oo::DelegateName"
                | "oo::configuresupport::readableproperties"
                | "oo::configuresupport::writableproperties"
                | "oo::configuresupport::objreadableproperties"
                | "oo::configuresupport::objwritableproperties"
        );
    (definition || core_helper || identity == "oo::UnknownDefinition").then_some(
        NativeCompilationSpec {
            grammar: NativeCompilationGrammar::NoHook,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_registration_absence_requires_exact_native_context_and_release() {
        let registry = crate::CommandRegistry::build_default();
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let method = compilation("oo::define::method", dialect);
            assert_eq!(method.is_some(), version >= TclVersion::V8_6);
            assert_eq!(
                registry.native_compilation_for_registration("oo::define::method", dialect),
                method
            );
            if let Some(method) = method {
                assert_eq!(method.compiler_hook_presence(dialect), Some(false));
                assert_eq!(
                    definition_identity("method", ObjectDispatchLayer::Class, dialect),
                    Some("oo::define::method")
                );
                assert_eq!(
                    definition_identity("method", ObjectDispatchLayer::Object, dialect),
                    Some("oo::objdefine::method")
                );
            }
            assert_eq!(
                compilation("oo::UnknownDefinition", dialect).is_some(),
                version >= TclVersion::V8_6
            );
            assert_eq!(compilation("method", dialect), None);
            assert_eq!(compilation("oo::define::not_registered", dialect), None);
            for hook in [
                "oo::Helpers::next",
                "oo::Helpers::nextto",
                "oo::Helpers::self",
            ] {
                assert_eq!(compilation(hook, dialect), None);
            }
            assert_eq!(
                compilation("oo::define::classmethod", dialect).is_some(),
                version >= TclVersion::V9_0
            );
            assert_eq!(compilation("oo::objdefine::constructor", dialect), None);
        }
        for name in ["jim", "f5-irules"] {
            if let Some(profile) = tcl_dialect::DialectProfile::find(name) {
                assert_eq!(
                    compilation("oo::define::method", InvocationDialect::of_profile(profile)),
                    None
                );
                assert_eq!(
                    compilation(
                        "oo::UnknownDefinition",
                        InvocationDialect::of_profile(profile)
                    ),
                    None
                );
            }
        }
    }
}
