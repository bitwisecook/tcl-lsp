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
use tcl_dialect::model::surface_admits;

/// The dispatch mechanism an `expr` math-function call uses in this profile.
///
/// Tcl 8.4 predates TIP 232. Its builtins live in a fixed C function table,
/// whereas Tcl 8.5 and later resolve calls through the open
/// `tcl::mathfunc::*` command table. Keeping the distinction in the registry
/// surface prevents an engine from deriving it from a command spelling or a
/// release comparison of its own.
#[derive(Debug, Clone, Copy)]
pub enum MathFunctionCallTarget {
    /// An 8.4 fixed-table builtin. The attached spec supplies the canonical
    /// name, arity, hover metadata, and version facts for the implementation.
    FixedBuiltin(&'static CommandSpec),
    /// The open `tcl::mathfunc::*` command table (TIP 232 and later).
    CommandTable,
    /// A name absent from Tcl 8.4's fixed table. C Tcl reports
    /// `unknown math function`, rather than an invalid command.
    FixedTableMiss,
}

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
        Self::for_profile(
            crate::model::ingress::resolve_environment(version.dialect_profile_name())
                .analyser_profile(),
        )
    }

    /// Get the registry-backed `expr` surface for `profile`.
    ///
    /// The profile-keyed entry point. A consumer that already holds the
    /// compilation's [`DialectProfile`] — the compiler's codegen, whose
    /// target dialect is a whole-module fact — must not round-trip it
    /// through a [`TclVersion`]: that step maps every dialect onto its
    /// plain-Tcl release profile and so drops the dialect-identity half of
    /// [`Self::supports_operator`]'s gate, hiding the iRules word operators
    /// from an iRules compile.
    #[must_use]
    pub fn for_profile(profile: &'static DialectProfile) -> Self {
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
    /// dialect-only operator is instead gated by the descriptor's dialect set
    /// — **or** by the profile's F5-family core [`ExprGrammar`] word table:
    /// the word-form operators are an `f5-tcl` *trunk* fact, measured valid in
    /// tmsh and iApp `expr` too, not iRules-only
    /// (`docs/design/bigip-irule-parser-measurements.md` §4a), so any
    /// F5Tcl-cored profile accepts them by reading the family table directly
    /// rather than duplicating rows (ledger C12/B6). This contains no
    /// spelling-specific logic: every fact comes from the syntax descriptor or
    /// the family grammar, while the profile contributes the grammar base and
    /// availability point.
    ///
    /// [`ExprGrammar`]: tcl_dialect::model::ExprGrammar
    #[must_use]
    pub fn supports_operator(self, op: BinOp) -> bool {
        let spec = op.spec();
        let release_visible = spec.expr_grammar_min_version.is_none_or(|floor| {
            self.profile
                .expr_grammar_base
                .is_none_or(|base| base >= floor)
        });
        let dialect_visible = spec
            .surface
            .is_none_or(|rows| surface_admits(rows, Some(&self.profile.surface_query())))
            || self
                .profile
                .f5_core_expr_grammar()
                .is_some_and(|grammar| grammar.has_word_operator(spec.spelling));
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

    /// Whether this release exposes the open `::tcl::mathfunc::*` command
    /// table, as opposed to Tcl 8.4's fixed `expr` function table.
    #[must_use]
    pub fn has_math_function_command_table(self) -> bool {
        crate::mathfunc::command_wrappers_available(self.profile)
    }

    /// The registry-owned dispatch target for `expr` function-call word
    /// `bare`.
    ///
    /// Tcl 8.4 only permits entries from its fixed built-in table. Later
    /// releases use the command table for every word, including names which
    /// are not registry builtins, so procedures, aliases, renames, and
    /// extension commands retain their normal Tcl indirection behaviour.
    #[must_use]
    pub fn math_function_call_target(self, bare: &str) -> MathFunctionCallTarget {
        if self.has_math_function_command_table() {
            return MathFunctionCallTarget::CommandTable;
        }
        self.builtin_math_function(bare).map_or(
            MathFunctionCallTarget::FixedTableMiss,
            MathFunctionCallTarget::FixedBuiltin,
        )
    }

    /// Whether `qualified_name` denotes a registry-provided math-function
    /// builtin, and if so whether this release exposes it as a literal command.
    ///
    /// `None` means the name is not one of the registry's builtin entries. An
    /// engine must leave such names to ordinary Tcl command resolution: they
    /// may be user-defined math functions, aliases, or renamed commands.
    #[must_use]
    pub fn builtin_math_function_command_visible(self, qualified_name: &str) -> Option<bool> {
        let bare = crate::mathfunc::global_command_bare_name(qualified_name)?;
        let registry_name = format!("{MATHFUNC_NAMESPACE}::{bare}");
        self.registry.get(&registry_name)?;
        Some(self.has_math_function_command_table() && self.builtin_math_function(bare).is_some())
    }

    /// Whether a builtin command may be enumerated or invoked on this runtime
    /// surface. Commands outside the registry's builtin math-function table
    /// remain visible: they may be user-defined replacements or extensions.
    #[must_use]
    pub fn permits_builtin_math_function_command(self, qualified_name: &str) -> bool {
        self.builtin_math_function_command_visible(qualified_name)
            .is_none_or(|visible| visible)
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

    /// The word-form operators are an `f5-tcl` **trunk** fact — measured
    /// valid in tmsh and iApp `expr` too, not iRules-only
    /// (`docs/design/bigip-irule-parser-measurements.md` §4a). The surface
    /// reads the acceptance off the family's `ExprGrammar` word table, so
    /// every F5Tcl-cored profile admits them while plain Tcl stays
    /// byte-identical.
    #[test]
    fn f5_word_operators_follow_the_family_expr_grammar() {
        let word_ops = [
            BinOp::WordAnd,
            BinOp::WordOr,
            BinOp::Contains,
            BinOp::StartsWith,
            BinOp::EndsWith,
            BinOp::StrEquals,
            BinOp::Matches,
            BinOp::MatchesGlob,
            BinOp::MatchesRegex,
        ];
        for name in ["f5-irules", "f5-tmsh", "f5-iapps"] {
            let surface = RuntimeExprSurface::for_profile(
                crate::model::ingress::resolve_environment(name).analyser_profile(),
            );
            for op in word_ops {
                assert!(surface.supports_operator(op), "{name}: {op:?}");
            }
            // The shared 8.4-core word operators remain valid too.
            assert!(surface.supports_operator(BinOp::StrEq), "{name}: eq");
        }
        // Plain-Tcl parity: no release ever admits the F5 word forms.
        for version in [TclVersion::V8_4, TclVersion::V8_6, TclVersion::V9_1] {
            let surface = RuntimeExprSurface::for_tcl_version(version);
            for op in word_ops {
                assert!(!surface.supports_operator(op), "{version:?}: {op:?}");
            }
        }
        // The config-schema `f5-bigip` identity has no Tcl expr surface of
        // its own and is deliberately excluded from the family derivation.
        assert!(
            crate::model::ingress::resolve_environment("f5-bigip")
                .analyser_profile()
                .f5_core_expr_grammar()
                .is_none()
        );
    }

    #[test]
    fn tip521_math_functions_are_registry_backed_and_versioned() {
        let fixed = RuntimeExprSurface::for_tcl_version(TclVersion::V8_4);
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

        // TP/TN: Tcl 8.4's pre-TIP-232 function table is closed, while its
        // recognised builtins still use the registry spec rather than a
        // consumer-local function list.
        assert!(matches!(
            fixed.math_function_call_target("sqrt"),
            MathFunctionCallTarget::FixedBuiltin(spec) if spec.name == "::tcl::mathfunc::sqrt"
        ));
        assert!(matches!(
            fixed.math_function_call_target("user_supplied"),
            MathFunctionCallTarget::FixedTableMiss
        ));
        assert!(matches!(
            old.math_function_call_target("user_supplied"),
            MathFunctionCallTarget::CommandTable
        ));
        assert!(!fixed.has_math_function_command_table());
        assert!(old.has_math_function_command_table());

        assert_eq!(
            fixed.builtin_math_function_command_visible("tcl::mathfunc::sqrt"),
            Some(false)
        );
        assert_eq!(
            old.builtin_math_function_command_visible("tcl::mathfunc::isfinite"),
            Some(false)
        );
        assert_eq!(
            modern.builtin_math_function_command_visible("::tcl::mathfunc::isfinite"),
            Some(true)
        );
        assert_eq!(
            old.builtin_math_function_command_visible("tcl::mathfunc::custom"),
            None
        );
        // TP/FN/TN/FP: consumers may ask one registry question when filtering
        // an enumeration without hiding a user-provided replacement.
        assert!(modern.permits_builtin_math_function_command("::tcl::mathfunc::isfinite"));
        assert!(!old.permits_builtin_math_function_command("::tcl::mathfunc::isfinite"));
        assert!(old.permits_builtin_math_function_command("::tcl::mathfunc::sqrt"));
        assert!(old.permits_builtin_math_function_command("::tcl::mathfunc::custom"));
    }
}
