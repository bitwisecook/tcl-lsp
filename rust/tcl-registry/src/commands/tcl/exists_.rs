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

//! Jim's original `exists` selector grammar and variable-name roles.
use crate::prelude::*;

static OPTIONS: &[&str] = &["-command", "-proc", "-alias", "-channel", "-var"];

/// Exact command/variable classification selected by Jim's original enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimExistsKind {
    /// Any actual command record.
    Command,
    /// The actual command record's procedure flag.
    Procedure,
    /// The actual command record's alias flag.
    Alias,
    /// The actual command record's channel flag.
    Channel,
    /// A silent original variable-object read.
    Variable,
}

/// The pinned Jim core's `GetEnum` table, separate from lookup and record flags.
#[derive(Clone, Copy, Debug)]
pub struct NativeJimExistsProtocol;
impl NativeJimExistsProtocol {
    /// Original declaration identity for the shared same-object enum cache.
    #[must_use]
    pub fn table(self) -> crate::native_index_lookup::NativeStaticIndexTable {
        crate::native_index_lookup::NativeStaticIndexTable::supported_backend(OPTIONS)
    }
    /// Meaning of the ordinal returned by the actual original enum lookup.
    #[must_use]
    pub const fn selected(self, ordinal: usize) -> Option<NativeJimExistsKind> {
        match ordinal {
            0 => Some(NativeJimExistsKind::Command),
            1 => Some(NativeJimExistsKind::Procedure),
            2 => Some(NativeJimExistsKind::Alias),
            3 => Some(NativeJimExistsKind::Channel),
            4 => Some(NativeJimExistsKind::Variable),
            _ => None,
        }
    }
    /// Original registered argument usage, before any name lookup.
    #[must_use]
    pub const fn usage(self) -> &'static [u8] {
        b"?-command|-proc|-alias|-channel|-var? name"
    }
}
impl crate::InvocationDialect {
    /// Actual Jim enum backend selects the matching core existence worker.
    #[must_use]
    pub fn native_jim_exists_protocol(self) -> Option<NativeJimExistsProtocol> {
        self.native_jim_enum_protocol()
            .map(|_| NativeJimExistsProtocol)
    }
}

fn argument_roles(arguments: &[&str]) -> Vec<(u8, ArgRole)> {
    if arguments.len() == 1 {
        return vec![(0, ArgRole::VarRead)];
    }
    if let [option, _] = arguments {
        let protocol = NativeJimExistsProtocol;
        if let Ok(cache) = crate::native_jim_enum::NativeJimEnumProtocol.lookup(
            option.as_bytes(),
            &protocol.table(),
            crate::native_jim_enum::NativeJimEnumFlags::options(false),
            None,
        ) {
            let tcl_core_types::NativeJimOptionCache::Enum { entry, .. } = cache else {
                return Vec::new();
            };
            return vec![(
                1,
                if protocol.selected(entry.index()) == Some(NativeJimExistsKind::Variable) {
                    ArgRole::VarRead
                } else {
                    ArgRole::Name
                },
            )];
        }
    }
    Vec::new()
}

pub(super) fn spec() -> CommandSpec {
    CommandSpec {
        name: "exists",
        surface: Some(tcl_dialect::surface![
            tcl_dialect::model::SpecSurface::core_in(
                tcl_dialect::model::Family::Jim,
                &[("0.84", None)]
            )
        ]),
        arity: Arity::new(1, 2),
        arg_role_resolver: Some(argument_roles),
        return_type: Some(TclType::Boolean),
        traits: Traits::REFLECTS_COMMAND_NAMES,
        runtime_backing: RuntimeBacking::shipped("exists"),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Direct,
        }),
        forms: &[FormSpec {
            synopsis: "exists ?-command|-proc|-alias|-channel|-var? name",
            ..FormSpec::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
