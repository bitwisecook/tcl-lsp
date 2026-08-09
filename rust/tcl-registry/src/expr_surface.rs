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

//! Registry-owned view of the versioned `expr` surface.
//!
//! `expr` contains both a closed grammar (operators) and an open command
//! namespace (`tcl::mathfunc`). The grammar's concrete metadata lives at the
//! syntax layer so the lexer can use it without depending on this crate; this
//! module is the one place engines ask whether that metadata is visible for a
//! profile, and where they obtain its Tcl-compatible diagnostic shape. Math
//! functions are resolved through [`CommandRegistry`], so a function cannot
//! become executable in an engine unless it has registry command data too.
//!
//! Consumers should build a [`RuntimeExprSurface`] from the emulated
//! [`TclVersion`] rather than compare releases or name individual operators
//! and functions themselves.

use tcl_dialect::{DialectProfile, TclVersion};
use tcl_syntax::expr::ast::{BinOp, ExprNode};

use crate::cache::registry_for_profile;
use crate::mathfunc::MATHFUNC_NAMESPACE;
use crate::mathfunc::available_in_expr;
use crate::registry::CommandRegistry;
use crate::spec::CommandSpec;

/// A versioned `expr` grammar rejection expressed in C Tcl's terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprSurfaceError {
    /// An operator token is not part of this profile's `expr` grammar.
    ///
    /// C Tcl calls this a bareword rather than a missing command. The spelling
    /// comes from the operator descriptor, never from an engine-local table.
    UnsupportedOperator {
        /// The rejected operator's canonical spelling.
        spelling: &'static str,
    },
}

impl ExprSurfaceError {
    /// Render the exact C Tcl 8.x bareword diagnostic for `source`.
    #[must_use]
    pub fn message(self, source: &str) -> String {
        match self {
            Self::UnsupportedOperator { spelling } => format!(
                "invalid bareword \"{spelling}\"\nin expression \"{source}\";\nshould be \"${spelling}\" or \"{{{spelling}}}\" or \"{spelling}(...)\" or ..."
            ),
        }
    }

    /// The Tcl `-errorcode` assigned to this grammar rejection.
    #[must_use]
    pub const fn error_code(self) -> &'static str {
        match self {
            Self::UnsupportedOperator { .. } => "TCL PARSE EXPR BAREWORD",
        }
    }
}

/// The expression grammar and builtin-math-function surface for one runtime.
///
/// It binds an emulated Tcl release to the cached command registry built for
/// that release's plain-Tcl [`DialectProfile`]. The registry is intentionally
/// retained here even for the closed operator grammar: it makes the runtime
/// profile, registry command surface, and expression checks one query rather
/// than three independently-maintained version comparisons.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeExprSurface {
    profile: &'static DialectProfile,
    registry: &'static CommandRegistry,
}

impl RuntimeExprSurface {
    /// Get the registry-backed `expr` surface for `version`.
    #[must_use]
    pub fn for_tcl_version(version: TclVersion) -> Self {
        let profile = DialectProfile::by_name(version.dialect_profile_name());
        Self {
            profile,
            registry: registry_for_profile(profile),
        }
    }

    /// The profile whose grammar and command surface this object answers.
    #[must_use]
    pub const fn profile(self) -> &'static DialectProfile {
        self.profile
    }

    /// Whether the closed `expr` grammar admits `op`.
    ///
    /// An operator with no version floor is part of the base Tcl grammar. A
    /// dialect-only operator is instead gated by the descriptor's dialect set.
    /// This contains no spelling-specific logic: every fact comes from the
    /// syntax descriptor, while the profile contributes the grammar base and
    /// availability mask.
    #[must_use]
    pub fn supports_operator(self, op: BinOp) -> bool {
        let spec = op.spec();
        let release_visible = spec.expr_grammar_min_version.is_none_or(|floor| {
            self.profile
                .expr_grammar_base
                .is_none_or(|base| base >= floor)
        });
        let dialect_visible = spec
            .dialects
            .is_none_or(|dialects| dialects.intersects(self.profile.availability_mask));
        release_visible && dialect_visible
    }

    /// Validate the parsed expression's operators against this surface.
    ///
    /// Functions are intentionally not rejected here. Tcl's math-function
    /// namespace is open: a user procedure, alias, or renamed command may
    /// provide a name that is not a builtin for this release. Engines gate only
    /// their registry-backed builtin registrations, leaving normal command
    /// resolution to honour those indirections.
    pub fn validate(self, node: &ExprNode) -> Result<(), ExprSurfaceError> {
        match node {
            ExprNode::Binary { op, left, right } => {
                self.validate(left)?;
                if !self.supports_operator(*op) {
                    return Err(ExprSurfaceError::UnsupportedOperator {
                        spelling: op.spec().spelling,
                    });
                }
                self.validate(right)
            }
            ExprNode::Unary { operand, .. } => self.validate(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                self.validate(condition)?;
                self.validate(true_branch)?;
                self.validate(false_branch)
            }
            ExprNode::Call { args, .. } => {
                for arg in args {
                    self.validate(arg)?;
                }
                Ok(())
            }
            ExprNode::Literal { .. }
            | ExprNode::String { .. }
            | ExprNode::Var { .. }
            | ExprNode::Command { .. }
            | ExprNode::Raw { .. } => Ok(()),
        }
    }

    /// The registry command specification for a builtin `expr` math function
    /// under this surface, or `None` when the name is unknown or post-dates the
    /// emulated release.
    ///
    /// Callers use this only to decide whether their *builtin* is present.
    /// They must continue ordinary command resolution for a missing builtin so
    /// user-provided `tcl::mathfunc` commands remain valid.
    #[must_use]
    pub fn builtin_math_function(self, bare: &str) -> Option<&'static CommandSpec> {
        if !available_in_expr(bare, self.profile) {
            return None;
        }
        self.registry.math_function_spec(bare, self.profile)
    }

    /// The bare names of registry-backed builtin math functions visible here.
    #[must_use]
    pub fn builtin_math_function_names(self) -> Vec<&'static str> {
        self.registry.math_function_names(self.profile)
    }

    /// Whether `qualified_name` denotes a registry-provided math-function
    /// builtin, and if so whether this release exposes that builtin.
    ///
    /// `None` means the name is not one of the registry's builtin entries. An
    /// engine must leave such names to ordinary Tcl command resolution: they
    /// may be user-defined math functions, aliases, or renamed commands.
    #[must_use]
    pub fn builtin_math_function_visible(self, qualified_name: &str) -> Option<bool> {
        let bare = qualified_name
            .trim_start_matches("::")
            .strip_prefix("tcl::mathfunc::")?;
        if bare.is_empty() || bare.contains("::") {
            return None;
        }
        let registry_name = format!("{MATHFUNC_NAMESPACE}::{bare}");
        self.registry.get(&registry_name)?;
        Some(self.builtin_math_function(bare).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::expr::parser::parse_expr;

    #[test]
    fn tip461_operators_follow_the_profile_grammar_base() {
        let old = RuntimeExprSurface::for_tcl_version(TclVersion::V8_6);
        let modern = RuntimeExprSurface::for_tcl_version(TclVersion::V9_0);

        // TP: the 9.0 string-ordering operator is accepted at its floor.
        assert!(modern.supports_operator(BinOp::StrLt));
        // FN: the same operator is rejected before its floor.
        assert!(!old.supports_operator(BinOp::StrLt));
        // TN: a base operator remains valid in both releases.
        assert!(old.supports_operator(BinOp::Add));
        // FP guard: the failure has the C grammar classification and wording.
        let node = parse_expr("{a} lt {b}", None);
        let error = old.validate(&node).expect_err("8.6 must reject lt");
        assert_eq!(error.error_code(), "TCL PARSE EXPR BAREWORD");
        assert_eq!(
            error.message("{a} lt {b}"),
            "invalid bareword \"lt\"\nin expression \"{a} lt {b}\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ..."
        );
    }

    #[test]
    fn tip521_math_functions_are_registry_backed_and_versioned() {
        let old = RuntimeExprSurface::for_tcl_version(TclVersion::V8_6);
        let modern = RuntimeExprSurface::for_tcl_version(TclVersion::V9_0);

        // TP: a 9.0 builtin has its command spec at its floor.
        assert!(modern.builtin_math_function("isfinite").is_some());
        // FN: the same builtin is absent before its floor.
        assert!(old.builtin_math_function("isfinite").is_none());
        // TN: an arbitrary function name is never claimed as a builtin.
        assert!(modern.builtin_math_function("user_supplied").is_none());
        // FP guard: older functions remain present rather than applying a
        // blanket 9.0 gate to the namespace.
        assert!(old.builtin_math_function("sqrt").is_some());

        assert_eq!(
            old.builtin_math_function_visible("tcl::mathfunc::isfinite"),
            Some(false)
        );
        assert_eq!(
            modern.builtin_math_function_visible("::tcl::mathfunc::isfinite"),
            Some(true)
        );
        assert_eq!(
            old.builtin_math_function_visible("tcl::mathfunc::custom"),
            None
        );
    }
}
