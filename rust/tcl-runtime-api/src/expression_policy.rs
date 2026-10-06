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

//! Expression evaluation policy, independent of physical compiler registration.

use tcl_dialect::{DialectProfileKey, model::DialectPoint};
use tcl_syntax::expr::parser::ExprParseContext;
use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;

/// Issuer of the retained expression grammar and evaluation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpressionEvaluationOrigin {
    /// An independently selected actual engine; its function registrations
    /// remain separate prerequisites.
    Native(DialectPoint),
    /// Explicit authored Tcl84 parser simulation for F5 source. This issues
    /// no native compiler, fixed-function table or physical result header.
    AuthoredTcl84Parser,
}

/// Independently installed authored function implementation, without native
/// registration, compiler, object-header or appliance authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthoredMathFunctionProvider {
    /// Tcl 8.4 core functions with an independent Park–Miller stream initially
    /// seeded with one and a separate precision context initially set to twelve.
    /// These initial values are reproducible simulation choices.
    Tcl84Core,
}

/// Original expression policy captured inside one runtime activation.
/// Parser and numeric capabilities do not create a math-function capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpressionEvaluationPolicy {
    /// Logical profile selected at capture, including an explicit host override.
    pub profile: DialectProfileKey,
    /// Independently authenticated policy issuer.
    pub origin: ExpressionEvaluationOrigin,
    /// Original checked parser axes; physical compiler parsing stays separate.
    pub context: ExprParseContext,
    /// Explicit numeric simulation, granting no native getter or cache authority.
    pub numeric_simulation: Option<AuthoredLogicalNumericSimulation>,
    /// Explicit authored fixed-function capability; parsing and numbers alone
    /// leave it absent. Native function prerequisites never consume this field.
    pub authored_functions: Option<AuthoredMathFunctionProvider>,
}
