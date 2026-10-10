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

//! `expr` math functions as a **registry query** — the bare-name ⇄ qualified
//! `::tcl::mathfunc::NAME` mapping, plus the two availability axes a consumer
//! has to keep apart.
//!
//! Inside an `expr` expression a function-call word `NAME(` is *not* an
//! ordinary command lookup: C Tcl compiles it to a dispatch on
//! `tcl::mathfunc::NAME` resolved **relative to the calling namespace, then
//! globally** (`tclCompExpr.c`'s `CompileMathFuncCall` / `tclExecute.c`'s
//! `INST_INVOKE` path; TIP 232). Verified against tclsh 8.6.16 and 9.0.4:
//!
//! ```tcl
//! namespace eval ::tcl::mathfunc { proc g {x} { expr {$x + 1000} } }
//! namespace eval ::foo {
//!     namespace eval tcl::mathfunc { proc f {x} { expr {$x * 100} } }
//!     proc use  {} { expr {f(2)} }   ;# 200  — ::foo::tcl::mathfunc::f
//!     proc useg {} { expr {g(2)} }   ;# 1002 — ::tcl::mathfunc::g (global)
//! }
//! proc f {x} { return GLOBAL-PROC-f }
//! ```
//!
//! A plain global `proc f` never enters that resolution, and conversely a
//! `proc` living in a `tcl::mathfunc` namespace is *not* reachable as a bare
//! command (`li {10 20 30} 1` → `invalid command name "li"` on both oracles) —
//! [`is_in_mathfunc_namespace`] is what lets a consumer keep the two apart
//! without naming a namespace itself.
//!
//! # The two availability axes
//!
//! | axis | question | helper |
//! |---|---|---|
//! | `expr` grammar | is `NAME(…)` a built-in function here? | [`available_in_expr`] |
//! | command table | does the command `::tcl::mathfunc::NAME` exist here? | [`command_wrappers_available`] + the spec's own `dialects` |
//!
//! They genuinely differ: `expr {abs(1)}` works in 8.4, but the *command*
//! `::tcl::mathfunc::abs` only exists from 8.5 (TIP 232 created the table).
//! So the [`CommandSpec`](crate::spec::CommandSpec) gate that
//! `mathfunc_generated.rs` writes (never looser than
//! [`SpecSurface::TCL85_PLUS`]) is the right gate
//! for the *qualified command* spelling and the **wrong** one for the bare
//! in-`expr` spelling — [`CommandRegistry::math_function_spec`] exists so a
//! consumer rendering the in-`expr` spelling gets the registry's hover data
//! under the `expr`-grammar gate instead of the command-table gate.
//!
//! [`tcl_syntax::expr::mathfunc`] stays the layer-1 fact table (which names
//! exist, and since when); this module is the single place the *registry*
//! answers mathfunc questions, so no consumer re-derives the
//! `tcl::mathfunc` prefix or the version ceiling for itself.

use tcl_dialect::{DialectProfile, TclVersion};
use tcl_syntax::expr::mathfunc::MathFuncSince;

use crate::registry::CommandRegistry;
use crate::spec::CommandSpec;

/// The absolute namespace every `expr` math function dispatches through.
pub const MATHFUNC_NAMESPACE: &str = "::tcl::mathfunc";

/// Selected native `info functions` implementation, independent of a reporting
/// namespace or a caller's desired expression grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeInfoFunctionsRecipe {
    /// Tcl 8.4 scans its interpreter's fixed math function table.
    FixedTable84,
    /// Later C releases evaluate a native-created script in the caller context.
    NamespaceScript(TclVersion),
}

impl NativeInfoFunctionsRecipe {
    /// Select the reached C implementation. Hosted and Jim surfaces cannot
    /// borrow this recipe from a Tcl-shaped profile.
    #[must_use]
    pub fn select(dialect: crate::InvocationDialect) -> Option<Self> {
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
            return None;
        }
        match dialect.native_name_protocol()? {
            tcl_syntax::naming::NativeNameProtocol::C(TclVersion::V8_4) => Some(Self::FixedTable84),
            tcl_syntax::naming::NativeNameProtocol::C(version) => {
                Some(Self::NamespaceScript(version))
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => None,
        }
    }

    /// Assemble only the selected native script and its original optional
    /// pattern. This grants no evaluation, namespace, helper or Normal result.
    #[must_use]
    pub fn script(self, pattern: Option<&[u8]>) -> Option<Vec<u8>> {
        let Self::NamespaceScript(version) = self else {
            return None;
        };
        let mut script = INFO_FUNCTIONS_SCRIPT.to_vec();
        if let Some(pattern) = pattern {
            script.extend(
                tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
                    tcl_syntax::native_string::NativeStringProtocol::for_tcl_version(version),
                )
                .render(&[pattern]),
            );
        }
        Some(script)
    }
}

// Native InfoFunctionsCmd creates this script, then Tcl_EvalObjEx(flags=0).
// Both global and caller-local command inventories and every named helper are
// live execution inputs. Direct table enumeration cannot replace this body.
const INFO_FUNCTIONS_SCRIPT: &[u8] = b"\t    ::apply [::list {{pattern *}} {\n\t\t::set cmds {}\n\t\t::foreach cmd [::info commands ::tcl::mathfunc::$pattern] {\n\t\t    ::lappend cmds [::namespace tail $cmd]\n\t\t}\n\t\t::foreach cmd [::info commands tcl::mathfunc::$pattern] {\n\t\t    ::set cmd [::namespace tail $cmd]\n\t\t    ::if {$cmd ni $cmds} {\n\t\t\t::lappend cmds $cmd\n\t\t    }\n\t\t}\n\t\t::return $cmds\n\t    } [::namespace current]] ";

/// The relative spelling of [`MATHFUNC_NAMESPACE`] — what a namespace-local
/// override is written as (`namespace eval tcl::mathfunc { proc f … }`) and
/// the suffix a caller-relative candidate carries.
pub const MATHFUNC_NAMESPACE_RELATIVE: &str = "tcl::mathfunc";

/// A command-name value constructed from one checked original expression
/// identifier under the selected modern C function lookup purpose. This is
/// neither a written command word nor a function registration or dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExpressionFunctionCommandName {
    source: String,
    context: tcl_syntax::expr::parser::ExprParseContext,
    ordinal: usize,
    offset: u32,
    function: String,
    bytes: Vec<u8>,
    protocol: tcl_syntax::naming::NativeNameProtocol,
}

impl NativeExpressionFunctionCommandName {
    pub(crate) fn from_original_expression(
        expression: &crate::conditional_expression::ConditionalExpressionEvaluation,
        source: &str,
        ordinal: usize,
        dialect: crate::InvocationDialect,
    ) -> Option<Self> {
        // Proof: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        if native_function_dispatch(dialect) != Some(NativeMathFunctionDispatch::CommandTable)
            || expression.context() != &dialect.expression_parse_context(None)
            || crate::conditional_expression::ConditionalExpressionEvaluation::prepare(
                source,
                expression.context(),
            )
            .as_ref()
                != Some(expression)
        {
            return None;
        }
        let (function, offset, _) = expression
            .tree()
            .function_calls()
            .into_iter()
            .nth(ordinal)?;
        let begin = usize::try_from(offset).ok()?;
        let end = begin.checked_add(function.len())?;
        if !function.is_ascii() || source.as_bytes().get(begin..end) != Some(function.as_bytes()) {
            return None;
        }
        let mut bytes = MATHFUNC_NAMESPACE_RELATIVE.as_bytes().to_vec();
        bytes.extend_from_slice(b"::");
        bytes.extend_from_slice(function.as_bytes());
        Some(Self {
            source: source.to_owned(),
            context: *expression.context(),
            ordinal,
            offset,
            function: function.to_owned(),
            bytes,
            protocol: dialect.native_name_protocol()?,
        })
    }

    /// Exact original identifier bytes; no reporting string is reparsed.
    #[must_use]
    pub fn function_bytes(&self) -> &[u8] {
        self.function.as_bytes()
    }

    /// Original expression bytes retaining the checked identifier.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Complete checked expression grammar retained by this name producer.
    #[must_use]
    pub const fn context(&self) -> &tcl_syntax::expr::parser::ExprParseContext {
        &self.context
    }

    /// Original tree ordinal, independent of command argv ordinals.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// Identifier byte offset within the complete original expression.
    #[must_use]
    pub const fn offset(&self) -> u32 {
        self.offset
    }

    /// Relative command-name bytes constructed by the function lookup recipe.
    #[must_use]
    pub fn command_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Selected command naming purpose; no authority is promoted by equality.
    #[must_use]
    pub const fn protocol(&self) -> tcl_syntax::naming::NativeNameProtocol {
        self.protocol
    }
}

/// The absolute command name a bare `expr` function word `bare` dispatches to
/// when it resolves globally.
#[must_use]
pub fn qualified_name(bare: &str) -> String {
    format!("{MATHFUNC_NAMESPACE}::{bare}")
}

/// The bare function name when `qualified` addresses a direct command in the
/// global [`MATHFUNC_NAMESPACE`], or `None` for every other command spelling.
///
/// This keeps the global command-table shape owned by the registry, so runtime
/// enumerators never need to reconstruct the namespace prefix themselves.
#[must_use]
pub fn global_command_bare_name(qualified: &str) -> Option<&str> {
    let bare = qualified
        .trim_start_matches("::")
        .strip_prefix(MATHFUNC_NAMESPACE_RELATIVE)?
        .strip_prefix("::")?;
    (!bare.is_empty() && !bare.contains("::")).then_some(bare)
}

/// Whether `qualified` names a command inside *a* `tcl::mathfunc` namespace —
/// the global `::tcl::mathfunc::sin` or a namespace-local override
/// `::foo::tcl::mathfunc::f`.
///
/// A command there is reachable only through `expr`'s function-call
/// production or its own fully-qualified name, never as a bare command word
/// (oracle in the module docs), so a bareword-resolution fallback must not
/// reach into it.
#[must_use]
pub fn is_in_mathfunc_namespace(qualified: &str) -> bool {
    let Some((ns, _tail)) = qualified.rsplit_once("::") else {
        return false;
    };
    ns == MATHFUNC_NAMESPACE
        || ns.ends_with(&format!("::{MATHFUNC_NAMESPACE_RELATIVE}"))
        || ns == MATHFUNC_NAMESPACE_RELATIVE
}

/// The newest math-function release `profile`'s `expr` grammar exposes, or
/// `None` for a profile with no version ceiling (the permissive fallback).
///
/// Gates on the profile's *`expr` grammar* base version — the same axis the
/// relational operators (`in` / `lt` / …) use — so a vendor shell running on
/// an 8.5 core has the 8.5 set even though its dialect tag is not a plain Tcl
/// version.
#[must_use]
pub fn expr_grammar_ceiling(profile: &'static DialectProfile) -> Option<MathFuncSince> {
    Some(match profile.expr_grammar_base? {
        TclVersion::V8_4 => MathFuncSince::Tcl84,
        // TIP 232 landed in 8.5; 8.6 added no math functions over 8.5.
        TclVersion::V8_5 | TclVersion::V8_6 => MathFuncSince::Tcl85,
        TclVersion::V9_0 => MathFuncSince::Tcl90,
        TclVersion::V9_1 => MathFuncSince::Tcl91,
    })
}

/// Whether `bare` is a genuine built-in `expr` math function available under
/// `profile` — the axis that governs the *in-`expr`* spelling `bare(…)`.
#[must_use]
pub fn available_in_expr(bare: &str, profile: &'static DialectProfile) -> bool {
    let dialect = crate::InvocationDialect::of_profile(profile);
    if dialect.family() == Some(tcl_dialect::model::Family::Jim) {
        return jim_fixed_math_function_names(dialect).is_some_and(|names| names.contains(&bare));
    }
    let Some(since) = tcl_syntax::expr::mathfunc::added_in(bare) else {
        return false;
    };
    expr_grammar_ceiling(profile).is_none_or(|ceiling| since <= ceiling)
}

/// Whether `profile` exposes math functions as literal `::tcl::mathfunc::*`
/// **commands** — TIP 232's wrapper mechanism, landed in 8.5.
///
/// Coarser than [`available_in_expr`]: it does not vary per name, because
/// every wrapper command — even one backing an 8.4-vintage function like
/// `sin` — only exists from 8.5 onward.
#[must_use]
pub fn command_wrappers_available(profile: &'static DialectProfile) -> bool {
    if crate::InvocationDialect::of_profile(profile).family()
        == Some(tcl_dialect::model::Family::Jim)
    {
        return false;
    }
    expr_grammar_ceiling(profile).is_none_or(|ceiling| ceiling >= MathFuncSince::Tcl85)
}

/// Fixed native functions measured in the current math-enabled Jim build.
/// An unmeasured Jim release never inherits C Tcl's open math command table.
#[must_use]
pub fn jim_fixed_math_function_names(
    dialect: crate::InvocationDialect,
) -> Option<&'static [&'static str]> {
    dialect
        .core_point
        .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
        .then_some(tcl_syntax::expr::mathfunc::jim_fixed_function_names())
}

/// Presence in the original stock fixed-function registration roster of a
/// fresh native interpreter. The caller separately retains that fresh,
/// unchanged table contract; actual runtime tables override this advice.
/// This supplies no registration token, arity, result or compiler authority.
#[must_use]
pub fn fresh_fixed_function_presence(
    dialect: crate::InvocationDialect,
    name: &str,
) -> Option<bool> {
    if native_function_dispatch(dialect) != Some(NativeMathFunctionDispatch::FixedTable) {
        return None;
    }
    match dialect.family()? {
        // Tcl_CreateInterp registers tclBuiltinFuncTable (tclExecute.c)
        // through Tcl_CreateMathFunc before any source is evaluated.
        tcl_dialect::model::Family::Tcl => {
            Some(tcl_syntax::expr::mathfunc::added_in(name) == Some(MathFuncSince::Tcl84))
        }
        tcl_dialect::model::Family::Jim => {
            jim_fixed_math_function_names(dialect).map(|names| names.contains(&name))
        }
        _ => None,
    }
}

/// Native lookup protocol for reached expression function calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeMathFunctionDispatch {
    /// Actual interpreter-owned registration table; command names cannot replace it.
    FixedTable,
    /// Namespace-relative Tcl command lookup after evaluating function operands.
    CommandTable,
}

/// Select the native protocol from actual engine axes, independently of the
/// assistance catalogue. Unknown engines and unmeasured Jim releases abstain.
#[must_use]
pub fn native_function_dispatch(
    dialect: crate::InvocationDialect,
) -> Option<NativeMathFunctionDispatch> {
    use tcl_dialect::model::Family;
    match dialect.family()? {
        Family::Tcl => dialect.tcl_version.map(|version| {
            if version == TclVersion::V8_4 {
                NativeMathFunctionDispatch::FixedTable
            } else {
                NativeMathFunctionDispatch::CommandTable
            }
        }),
        Family::Jim => {
            jim_fixed_math_function_names(dialect).map(|_| NativeMathFunctionDispatch::FixedTable)
        }
        _ => None,
    }
}

/// Select the numeric implementation independently of function lookup.
/// Unknown native policies never silently acquire C math semantics.
#[must_use]
pub fn native_math_protocol(
    dialect: crate::InvocationDialect,
) -> Option<tcl_syntax::expr::mathfunc::NativeMathProtocol> {
    use tcl_syntax::expr::mathfunc::NativeMathProtocol;
    if dialect.family() == Some(tcl_dialect::model::Family::Jim) {
        return jim_fixed_math_function_names(dialect).map(|_| NativeMathProtocol::Jim084);
    }
    dialect.tcl_version.map(|_| NativeMathProtocol::Tcl)
}

/// Audited scalar native implementation, separate from its command spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeScalarMathOperation {
    /// Native square root converts one operand and returns a double object.
    Sqrt,
    /// C Tcl double converts one operand and returns a double object.
    Double,
}

/// Numeric subtype treatment at the selected scalar operand conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeMathNumericOperandPolicy {
    /// C native double access reads existing numeric representations directly.
    PreserveCategory,
    /// Jim may replace an integer representation with its coerced-double cache.
    /// The object remains numeric, but its earlier subtype is no longer proved.
    WeakenToNumeric,
}

/// Independent world obligation of the selected scalar operand conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScalarMathInputRequirement {
    /// An actual current native numeric object bypasses string callbacks.
    /// Otherwise its string access must be independently closed, including
    /// nested element updaters. Ordinary outer container shape is insufficient.
    NumericOrClosedStringAccess {
        /// Absolute post-head operand whose native conversion occurs.
        argument: usize,
    },
}

/// Selected scalar native result and operand protocol. Actual implementation
/// identity, normal completion and execution observers remain caller obligations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeScalarMathProtocol {
    dialect: crate::InvocationDialect,
    operand_at: usize,
}

impl NativeScalarMathProtocol {
    pub(crate) fn for_invocation(
        operation: NativeScalarMathOperation,
        arguments: crate::InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<Self> {
        if argument_offset.checked_add(1) != arguments.exact_argv_len() {
            return None;
        }
        let dialect = arguments.dialect()?;
        native_function_dispatch(dialect)?;
        // Jim double selects its numeric-unary operator, including a separate
        // Boolean fallback, rather than C Tcl's selected double conversion.
        if operation == NativeScalarMathOperation::Double
            && dialect.family() != Some(tcl_dialect::model::Family::Tcl)
        {
            return None;
        }
        Some(Self {
            dialect,
            operand_at: argument_offset,
        })
    }

    /// Absolute post-head operand converted before the normal result is made.
    #[must_use]
    pub const fn operand_at(self) -> usize {
        self.operand_at
    }

    /// Operand-world obligation, independent of normal result and completion.
    #[must_use]
    pub const fn input_requirement(self) -> NativeScalarMathInputRequirement {
        NativeScalarMathInputRequirement::NumericOrClosedStringAccess {
            argument: self.operand_at,
        }
    }

    /// Numeric representation of the actual normal result, without value,
    /// allocation freshness, compiler admission or world-effect permission.
    #[must_use]
    pub const fn result_production(self) -> crate::native_result::NativeNumericResultProduction {
        crate::native_result::NativeNumericResultProduction::Double
    }

    /// Completion codes of the selected native implementation after operand
    /// evaluation. Command observers and replacement functions remain separate.
    #[must_use]
    pub const fn completion_route(self) -> crate::completion_route::InvocationCompletionRoute {
        crate::completion_route::InvocationCompletionRoute::TclAlternatives(&[
            crate::completion::CompletionCode::Ok,
            crate::completion::CompletionCode::Error,
        ])
    }

    /// Treatment of an independently proved current numeric operand subtype.
    /// Non-numeric objects still require their actual conversion footprint.
    #[must_use]
    pub fn numeric_operand_policy(self) -> NativeMathNumericOperandPolicy {
        if self.dialect.family() == Some(tcl_dialect::model::Family::Jim) {
            NativeMathNumericOperandPolicy::WeakenToNumeric
        } else {
            NativeMathNumericOperandPolicy::PreserveCategory
        }
    }
}

impl CommandRegistry {
    /// Protocol for an actual interpreter-owned fixed registration. The caller
    /// supplies the retained row's native registry identity and actual arity;
    /// a function name, stock presence or wrapper availability is insufficient.
    /// Wrapper command floors do not govern C 8.4 or Jim fixed-table dispatch.
    #[must_use]
    pub fn fixed_scalar_math_protocol(
        &self,
        dialect: crate::InvocationDialect,
        registry_identity: &str,
        registered_arity: Option<usize>,
        call_arity: usize,
    ) -> Option<NativeScalarMathProtocol> {
        if native_function_dispatch(dialect) != Some(NativeMathFunctionDispatch::FixedTable)
            || registered_arity != Some(call_arity)
            || call_arity != 1
        {
            return None;
        }
        let specs = self.specs(registry_identity);
        let contract = specs.first()?.native_result?;
        if !specs.iter().all(|spec| {
            spec.native_result == Some(contract)
                && spec.successful_handler
                    == Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
                && spec.arity == crate::arity::Arity::exact(1)
        }) {
            return None;
        }
        let crate::native_result::NativeResultContract::ScalarMath(operation) = contract else {
            return None;
        };
        // One retained native row argument remains one unknown value word.
        let words = [crate::InvocationWord::DynamicNonOption];
        NativeScalarMathProtocol::for_invocation(
            operation,
            crate::InvocationArguments::structured(&words).with_dialect(dialect),
            0,
        )
    }

    /// The registry [`CommandSpec`] backing the **in-`expr`** spelling of the
    /// bare math-function word `bare` under `profile`, or `None` when `bare`
    /// is not a built-in `expr` function this profile has.
    ///
    /// The mapping bare → `::tcl::mathfunc::bare` is data-driven (the spec is
    /// looked up by name; `mathfunc_generated.rs` owns the entries), and the
    /// gate is the *`expr` grammar* one ([`available_in_expr`]) rather than
    /// the spec's own command-table `dialects` — see the module docs for why
    /// the two differ. Consumers rendering the *qualified command* spelling
    /// keep using [`CommandRegistry::get_for_surface`] and get the tighter
    /// gate, as they should.
    #[must_use]
    pub fn math_function_spec(
        &self,
        bare: &str,
        profile: &'static DialectProfile,
    ) -> Option<&CommandSpec> {
        if !available_in_expr(bare, profile) {
            return None;
        }
        self.get(&qualified_name(bare))
    }

    /// Metadata for an independently selected expression-function identity.
    /// Fixed registrations use the function metadata namespace without
    /// requiring a command wrapper; command-table identities use their actual
    /// selected command. This query creates no lookup or registration receipt.
    #[must_use]
    pub fn selected_math_function_spec(
        &self,
        identity: &str,
        dispatch: NativeMathFunctionDispatch,
    ) -> Option<&CommandSpec> {
        match dispatch {
            NativeMathFunctionDispatch::CommandTable => self.get(identity),
            NativeMathFunctionDispatch::FixedTable => {
                let bare = global_command_bare_name(identity).or_else(|| {
                    (!identity.is_empty() && !identity.contains("::")).then_some(identity)
                })?;
                crate::registry::fixed_math_function_metadata(bare)
            }
        }
    }

    /// Every built-in `expr` math function available under `profile`, by bare
    /// name, sourced from the registry's own `::tcl::mathfunc::*` entries
    /// (so a name with no registry data never surfaces) and gated on the
    /// `expr`-grammar axis. Sorted, deduplicated.
    #[must_use]
    pub fn math_function_names(&self, profile: &'static DialectProfile) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .command_names()
            .filter_map(|name| name.strip_prefix(MATHFUNC_NAMESPACE)?.strip_prefix("::"))
            .filter(|bare| !bare.contains("::") && available_in_expr(bare, profile))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn selected_fixed_math_metadata_survives_command_surface_projection() {
        // Implementation contract: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        for name in ["tcl8.4", "jim"] {
            let profile = crate::model::ingress::resolve_environment(name).analyser_profile();
            let registry = crate::CommandRegistry::build_default().project_for_profile(profile);
            assert!(registry.get("::tcl::mathfunc::abs").is_none(), "{name}");
            assert!(
                registry
                    .selected_math_function_spec(
                        "abs",
                        super::NativeMathFunctionDispatch::FixedTable,
                    )
                    .is_some(),
                "{name}"
            );
            assert!(
                registry
                    .selected_math_function_spec(
                        "::tcl::mathfunc::abs",
                        super::NativeMathFunctionDispatch::CommandTable,
                    )
                    .is_none(),
                "{name}"
            );
            assert!(
                registry
                    .selected_math_function_spec(
                        "unknown_fixed_function",
                        super::NativeMathFunctionDispatch::FixedTable,
                    )
                    .is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn double_scalar_protocol_requires_selected_c_implementation_and_exact_operand() {
        let registry = crate::CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let values = [crate::InvocationWord::DynamicNonOption];
            if version == tcl_dialect::TclVersion::V8_4 {
                let protocol = registry
                    .fixed_scalar_math_protocol(dialect, "::tcl::mathfunc::double", Some(1), 1)
                    .unwrap();
                assert_eq!(protocol.operand_at(), 0);
                assert_eq!(
                    protocol.result_production(),
                    crate::native_result::NativeNumericResultProduction::Double
                );
            } else {
                let words = crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("::tcl::mathfunc::double"),
                    &values,
                )
                .with_dialect(dialect);
                let mut facts = registry
                    .resolve_structured_invocation(words, dialect.authoring_query())
                    .resolved()
                    .unwrap()
                    .facts();
                let protocol = facts
                    .normal_scalar_math_protocol(words.arguments())
                    .unwrap();
                assert_eq!(protocol.operand_at(), 0);
                assert_eq!(
                    protocol.numeric_operand_policy(),
                    super::NativeMathNumericOperandPolicy::PreserveCategory
                );
                facts.refine_scalar_math_input_effects(words.arguments(), false);
                assert!(facts.effects.requires_world_barrier());
                assert!(!facts.traits.contains(crate::Traits::PURE));
                facts.successful_handler = None;
                assert!(
                    facts
                        .normal_scalar_math_protocol(words.arguments())
                        .is_none()
                );
            }
            for values in [
                vec![],
                vec![crate::InvocationWord::Dynamic; 2],
                vec![crate::InvocationWord::Expanded],
            ] {
                assert!(
                    super::NativeScalarMathProtocol::for_invocation(
                        super::NativeScalarMathOperation::Double,
                        crate::InvocationArguments::structured(&values).with_dialect(dialect),
                        0,
                    )
                    .is_none()
                );
            }
        }
        let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert!(
            registry
                .fixed_scalar_math_protocol(jim, "::tcl::mathfunc::double", Some(1), 1)
                .is_none()
        );
        assert!(
            super::NativeScalarMathProtocol::for_invocation(
                super::NativeScalarMathOperation::Double,
                crate::InvocationArguments::literals(&["21"]),
                0,
            )
            .is_none()
        );
    }

    #[test]
    fn fixed_scalar_math_requires_native_registration_identity_and_arity() {
        let registry = crate::CommandRegistry::build_default();
        for dialect in [
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
            crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            )),
        ] {
            let protocol = registry
                .fixed_scalar_math_protocol(dialect, "::tcl::mathfunc::sqrt", Some(1), 1)
                .unwrap();
            assert_eq!(
                protocol.result_production(),
                crate::native_result::NativeNumericResultProduction::Double
            );
            assert_eq!(
                protocol.numeric_operand_policy(),
                if dialect.family() == Some(tcl_dialect::model::Family::Jim) {
                    super::NativeMathNumericOperandPolicy::WeakenToNumeric
                } else {
                    super::NativeMathNumericOperandPolicy::PreserveCategory
                }
            );
            assert!(
                registry
                    .fixed_scalar_math_protocol(dialect, "sqrt", Some(1), 1)
                    .is_none()
            );
            assert!(
                registry
                    .fixed_scalar_math_protocol(dialect, "::tcl::mathfunc::sqrt", None, 1)
                    .is_none()
            );
            assert!(
                registry
                    .fixed_scalar_math_protocol(dialect, "::tcl::mathfunc::sqrt", Some(1), 2)
                    .is_none()
            );
            assert!(
                registry
                    .fixed_scalar_math_protocol(dialect, "::tcl::mathfunc::abs", Some(1), 1)
                    .is_none()
            );
        }
        assert!(
            registry
                .fixed_scalar_math_protocol(
                    crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1),
                    "::tcl::mathfunc::sqrt",
                    Some(1),
                    1
                )
                .is_none()
        );
    }

    use super::*;

    fn profile(name: &str) -> &'static DialectProfile {
        crate::model::ingress::resolve_environment(name).analyser_profile()
    }

    #[test]
    fn original_function_command_names_keep_checked_expression_lineage() {
        // naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let context = dialect.expression_parse_context(None);
            let expression =
                crate::conditional_expression::ConditionalExpressionEvaluation::prepare(
                    "max(abs(1), Pi())",
                    &context,
                )
                .unwrap();
            let name = expression
                .original_function_command_name("max(abs(1), Pi())", 2, dialect)
                .unwrap();
            assert_eq!(name.function_bytes(), b"Pi");
            assert_eq!(name.command_bytes(), b"tcl::mathfunc::Pi");
            assert_eq!(name.ordinal(), 2);
            assert_eq!(name.offset(), 12);
            assert_eq!(name.source(), "max(abs(1), Pi())");
            assert_eq!(name.context(), &context);
            assert!(
                expression
                    .original_function_command_name("max(abs(2), Pi())", 2, dialect)
                    .is_none()
            );
            assert!(
                expression
                    .original_function_command_name("max(abs(1), Pi())", 3, dialect)
                    .is_none()
            );
            let mut foreign = dialect;
            foreign.numbers = tcl_dialect::NumberSyntax::Tcl84;
            foreign.lexer_grammar.numbers = tcl_dialect::NumberSyntax::Tcl84;
            assert!(
                expression
                    .original_function_command_name("max(abs(1), Pi())", 2, foreign)
                    .is_none()
            );
        }
        let dialect = crate::InvocationDialect::for_version(TclVersion::V8_4);
        let expression = crate::conditional_expression::ConditionalExpressionEvaluation::prepare(
            "Pi()",
            &dialect.expression_parse_context(None),
        )
        .unwrap();
        assert!(
            expression
                .original_function_command_name("Pi()", 0, dialect)
                .is_none()
        );
        let dialect = crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
        );
        let expression = crate::conditional_expression::ConditionalExpressionEvaluation::prepare(
            "Pi()",
            &dialect.expression_parse_context(None),
        )
        .unwrap();
        assert!(
            expression
                .original_function_command_name("Pi()", 0, dialect)
                .is_none()
        );
    }

    #[test]
    fn qualified_name_uses_the_absolute_namespace() {
        assert_eq!(qualified_name("sin"), "::tcl::mathfunc::sin");
    }

    #[test]
    fn global_command_bare_name_accepts_only_direct_global_members() {
        assert_eq!(
            global_command_bare_name("::tcl::mathfunc::sin"),
            Some("sin")
        );
        assert_eq!(global_command_bare_name("tcl::mathfunc::sin"), Some("sin"));
        // TN: a nested command and an unrelated namespace are not global
        // math-function command-table entries.
        assert_eq!(
            global_command_bare_name("::tcl::mathfunc::nested::sin"),
            None
        );
        assert_eq!(global_command_bare_name("::tcl::mathop::sin"), None);
    }

    #[test]
    fn mathfunc_namespace_membership_covers_global_and_namespace_local() {
        assert!(is_in_mathfunc_namespace("::tcl::mathfunc::sin"));
        assert!(is_in_mathfunc_namespace("::foo::tcl::mathfunc::f"));
        assert!(is_in_mathfunc_namespace("tcl::mathfunc::f"));
        // TN: an ordinary command, and the sibling operator namespace.
        assert!(!is_in_mathfunc_namespace("::sin"));
        assert!(!is_in_mathfunc_namespace("::tcl::mathop::+"));
        assert!(!is_in_mathfunc_namespace("::mathfunc::sin"));
        assert!(!is_in_mathfunc_namespace("sin"));
    }

    /// The `expr`-grammar axis, not the command-table one: `abs(…)` is an 8.4
    /// function even though the command `::tcl::mathfunc::abs` is 8.5+.
    #[test]
    fn expr_availability_follows_the_expr_grammar_axis() {
        assert!(available_in_expr("abs", profile("tcl8.4")));
        assert!(available_in_expr("sin", profile("tcl8.4")));
        // TIP 232 names are 8.5+.
        assert!(!available_in_expr("max", profile("tcl8.4")));
        assert!(available_in_expr("max", profile("tcl8.5")));
        // TIP 521 names are 9.0+.
        assert!(!available_in_expr("isnan", profile("tcl8.6")));
        assert!(available_in_expr("isnan", profile("tcl9.0")));
        // TIP 745 names are 9.1+.
        assert!(!available_in_expr("gamma", profile("tcl9.0")));
        assert!(available_in_expr("gamma", profile("tcl9.1")));
        // TN: not a math function at all.
        assert!(!available_in_expr("zzznope", profile("tcl9.1")));
    }

    #[test]
    fn command_wrappers_start_at_tcl85() {
        assert!(!command_wrappers_available(profile("tcl8.4")));
        assert!(command_wrappers_available(profile("tcl8.5")));
        assert!(command_wrappers_available(profile("tcl9.0")));
    }

    #[test]
    fn native_dispatch_uses_actual_engine_instead_of_assistance_profile() {
        for version in TclVersion::ALL {
            assert_eq!(
                native_function_dispatch(crate::InvocationDialect::for_version(version)),
                Some(if version == TclVersion::V8_4 {
                    NativeMathFunctionDispatch::FixedTable
                } else {
                    NativeMathFunctionDispatch::CommandTable
                })
            );
        }
        let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert!(command_wrappers_available(profile("tcl9.1")));
        assert_eq!(
            native_function_dispatch(jim),
            Some(NativeMathFunctionDispatch::FixedTable)
        );
        assert_eq!(
            native_function_dispatch(crate::InvocationDialect::of_profile(profile("tcl"))),
            None
        );
        assert_eq!(
            native_function_dispatch(crate::InvocationDialect::of_profile(profile("f5-irules"))),
            None
        );
    }

    #[test]
    fn math_function_spec_serves_registry_hover_data_under_the_expr_gate() {
        let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
        let spec = reg
            .math_function_spec("sin", profile("tcl8.6"))
            .expect("sin has registry data under 8.6");
        assert_eq!(spec.name, "::tcl::mathfunc::sin");
        assert!(spec.hover.is_some(), "hover data must come from the spec");
        // The 8.4 profile still gets `sin`'s spec for the in-expr spelling,
        // even though the spec's own command-table gate is TCL85_PLUS.
        assert!(reg.math_function_spec("sin", profile("tcl8.4")).is_some());
        // Version gate: `isnan` is 9.0+.
        let reg9 = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            reg9.math_function_spec("isnan", profile("tcl9.0"))
                .is_some()
        );
        assert!(reg.math_function_spec("isnan", profile("tcl8.6")).is_none());
        // TN: not a math function.
        assert!(reg.math_function_spec("puts", profile("tcl8.6")).is_none());
    }

    #[test]
    fn math_function_names_are_bare_sorted_and_gated() {
        let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
        let names = reg.math_function_names(profile("tcl9.0"));
        assert!(names.contains(&"sin"), "{names:?}");
        assert!(names.contains(&"max"), "{names:?}");
        assert!(names.contains(&"isnan"), "{names:?}");
        assert!(!names.contains(&"gamma"), "9.1-only: {names:?}");
        assert!(
            names.windows(2).all(|w| w[0] < w[1]),
            "sorted + deduped: {names:?}"
        );
        assert!(
            names.iter().all(|n| !n.contains("::")),
            "bare names only: {names:?}"
        );
        let names84 = reg.math_function_names(profile("tcl8.4"));
        assert!(names84.contains(&"sin"));
        assert!(!names84.contains(&"max"), "8.5-only: {names84:?}");
    }
}
