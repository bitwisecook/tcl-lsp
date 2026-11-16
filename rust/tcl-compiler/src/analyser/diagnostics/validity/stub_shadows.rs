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

//! W116 and W117: a stub `proc` that shadows a built-in command or `expr` function.

use crate::analyser::state::Analyser;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SpecSurface;

impl Analyser {
    /// **W116 / W117.** Stub command / expression definition shadows a
    /// built-in. W116 uses this analysis generation's selected descriptor;
    /// W117 fires when a stub expr function/operator name
    /// collides with a built-in `expr` function or operator.
    pub(in crate::analyser) fn emit_w116_w117_stub_shadows(&mut self) {
        use super::super::types::Severity;

        if self.result.stub_commands.is_empty() && self.result.stub_expr_defs.is_empty() {
            return;
        }

        if !self.result.stub_commands.is_empty() {
            let context = self.analysis_context();
            let hits: Vec<(String, tcl_lexer::Span)> = self
                .result
                .stub_commands
                .iter()
                .filter(|stub| {
                    !stub.from_sidecar
                        && context
                            .context()
                            .resolve_spec(context.commands(), &stub.name)
                            .is_some()
                })
                .map(|s| (s.name.clone(), s.range))
                .collect();
            for (name, span) in hits {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W116,
                        span,
                        format!("Stub command '{name}' shadows built-in command."),
                        Severity::Warning,
                    ));
            }
        }

        // W117 — stub expr function/operator shadows a built-in.
        if !self.result.stub_expr_defs.is_empty() {
            let irules = self.profile.is_irules();
            let hits: Vec<(String, String, tcl_lexer::Span)> = self
                .result
                .stub_expr_defs
                .iter()
                .filter(|s| {
                    !s.from_sidecar
                        && (is_builtin_math_function(&s.name)
                            || is_builtin_expr_op(&s.name)
                            || (irules && is_irules_only_expr_op(&s.name)))
                })
                .map(|s| (s.name.clone(), s.kind.clone(), s.range))
                .collect();
            for (name, kind, span) in hits {
                let kind_label = if kind == "function" {
                    "function"
                } else {
                    "operator"
                };
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W117,
                        span,
                        format!(
                            "Stub expression {kind_label} '{name}' shadows built-in {kind_label}."
                        ),
                        Severity::Warning,
                    ));
            }
        }
    }
}

/// Whether `name` is a built-in `expr` math function (`sin`, `max`, the TIP
/// 745 C99 batch, …) in *any* dialect — used by the W117 stub-shadow check,
/// which doesn't have (and doesn't need) a specific dialect to check against
/// here since a stub shadowing a function that exists in some other dialect
/// is still worth flagging. Derived from
/// [`tcl_syntax::expr::mathfunc::added_in`] rather than a hand-typed list,
/// which drifts (a list claiming the TIP 745 batch before `dispatch()`
/// implements it).
pub(super) fn is_builtin_math_function(name: &str) -> bool {
    tcl_syntax::expr::mathfunc::added_in(name).is_some()
}

/// Whether `name` is a built-in `expr` operator spelling **not** gated to
/// iRules only (`+`, `in`, `**`, `eq`, …) — used by the W117 stub-shadow
/// check. Derived from [`tcl_syntax::expr::operators`] rather than a
/// hand-typed list: a `BinOp`/`UnaryOp` whose
/// `dialects` isn't exactly `Some(SpecSurface::IRULES)` is available outside
/// iRules (`None` = ungated, `Some(TCL90_PLUS)` etc. = version-gated but not
/// dialect-*identity*-gated — both count as "built-in" here; only the nine
/// iRules word operators are excluded).
pub(super) fn is_builtin_expr_op(name: &str) -> bool {
    tcl_syntax::expr::operators::ALL_BIN_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.surface != Some(SpecSurface::IRULES)
    }) || tcl_syntax::expr::operators::ALL_UNARY_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.surface != Some(SpecSurface::IRULES)
    })
}

/// Whether `name` is one of the nine iRules-only word operators (`and`,
/// `contains`, `not`, …) — see [`is_builtin_expr_op`]'s doc for the
/// derivation and why these are excluded there.
pub(super) fn is_irules_only_expr_op(name: &str) -> bool {
    tcl_syntax::expr::operators::ALL_BIN_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.surface == Some(SpecSurface::IRULES)
    }) || tcl_syntax::expr::operators::ALL_UNARY_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.surface == Some(SpecSurface::IRULES)
    })
}
