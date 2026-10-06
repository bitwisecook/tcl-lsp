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

//! Structured source-word facts for target-neutral registry resolution.
//!
//! A Tcl source word is not necessarily the value eventually presented to a
//! command: substitution can replace it, and `{*}` expansion can change the
//! number of argv entries altogether.  This module makes that distinction
//! explicit before registry matching reaches literal-sensitive metadata such
//! as subcommand tables and argument-dependent effect resolvers.

/// Contents alternatives for one actual ordinary argv position. These are
/// value-analysis facts, not a literal, object or execution binding proof.
#[derive(Debug, Clone, Copy)]
pub enum RoleOperandValues<'a> {
    /// Every possible value is retained; the set must be nonempty and bounded.
    Closed(&'a [String]),
    /// An opaque or missing alternative remains possible.
    Unknown,
}

/// Closed value information for role/layout consensus, preserving argc.
#[derive(Debug, Clone, Copy)]
pub struct RoleOperandAlternatives<'a> {
    /// Evaluated argv index after the command head, not a source word offset.
    pub argument: usize,
    /// Contents possibilities with an explicit unknown residual.
    pub values: RoleOperandValues<'a>,
}

/// Native control protocol for a scheduled procedure replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTailcallProtocol {
    /// C returns `TCL_RETURN` and an empty tailcall terminates its procedure.
    Tcl,
    /// Jim returns code seven; an empty tailcall completes normally.
    Jim084,
}

impl NativeTailcallProtocol {
    /// Completion emitted while retaining the replacement on its issuer frame.
    #[must_use]
    pub const fn pending_code(self) -> i32 {
        match self {
            Self::Tcl => 2,
            Self::Jim084 => 7,
        }
    }

    /// Whether a tailcall without a target schedules procedure termination.
    #[must_use]
    pub const fn schedules_empty(self) -> bool {
        matches!(self, Self::Tcl)
    }
}

/// Native script parser admission, separate from bytecode compiler traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScriptParseTiming {
    /// C parses a command before substituting that command's arguments.
    BeforeEachCommand,
    /// Jim validates the script object before installing its evaluation frame.
    BeforeScript,
}

/// The source-level knowledge available for one Tcl invocation word.
///
/// Only [`Self::Literal`] exposes a string.  The remaining variants
/// deliberately retain no raw spelling: treating source text such as `$kind`
/// as its runtime value would make a registry decision unsound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationWord<'w> {
    /// A word whose Tcl value is known to be exactly this literal string.
    Literal(&'w str),
    /// One non-expanded argv word whose value is produced by substitution.
    Dynamic,
    /// One substituted argv word whose non-empty literal prefix proves its
    /// evaluated value cannot begin with `-` and therefore cannot be a
    /// leading option. For example, `"prefix-$value"` remains dynamic as a
    /// value, but it cannot select a command option before the known
    /// positional layout.
    DynamicNonOption,
    /// A single evaluated variable-name value with a proved literal array root
    /// and an unknown element. No complete string or element value is exposed.
    ArrayElementName {
        /// Literal root before the first opening parenthesis in the value.
        root: &'w str,
    },
    /// A `{*}` word whose evaluated Tcl list contributes an unknown number of
    /// argv entries.
    Expanded,
    /// A source region whose word and argv shape are unavailable to this
    /// resolver.
    Opaque,
}

/// The non-value classification of an [`InvocationWord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationWordKind {
    /// The word has a known literal value.
    Literal,
    /// The word's value is computed by substitution.
    Dynamic,
    /// The word expands a runtime Tcl list into argv entries.
    Expanded,
    /// The source region has no reliable word or argv representation.
    Opaque,
}

impl<'w> InvocationWord<'w> {
    /// Return this word's known Tcl value, if and only if it is literal.
    #[must_use]
    pub const fn literal(self) -> Option<&'w str> {
        match self {
            Self::Literal(value) => Some(value),
            Self::Dynamic
            | Self::DynamicNonOption
            | Self::ArrayElementName { .. }
            | Self::Expanded
            | Self::Opaque => None,
        }
    }

    /// Return the source knowledge category without exposing a non-literal
    /// spelling as a runtime value.
    #[must_use]
    pub const fn kind(self) -> InvocationWordKind {
        match self {
            Self::Literal(_) => InvocationWordKind::Literal,
            Self::Dynamic | Self::DynamicNonOption | Self::ArrayElementName { .. } => {
                InvocationWordKind::Dynamic
            }
            Self::Expanded => InvocationWordKind::Expanded,
            Self::Opaque => InvocationWordKind::Opaque,
        }
    }

    /// Whether this source word is guaranteed to contribute exactly one argv
    /// entry after evaluation.
    #[must_use]
    pub const fn has_exactly_one_argv_entry(self) -> bool {
        matches!(
            self,
            Self::Literal(_)
                | Self::Dynamic
                | Self::DynamicNonOption
                | Self::ArrayElementName { .. }
        )
    }

    /// Whether the evaluated word has a stable first character other than an option dash.
    #[must_use]
    pub fn proves_non_option(self) -> bool {
        match self {
            Self::DynamicNonOption => true,
            Self::ArrayElementName { root } => !root.starts_with('-'),
            _ => false,
        }
    }
}

/// Native `error` argument interpretation, independent of assistance metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeErrorArguments {
    /// C Tcl's optional error-info and error-code operands.
    Tcl,
    /// Jim's optional raw stack-trace operand; error code remains NONE.
    JimStackTrace,
}

/// Native usage-prefix rendering, independent of alias or ensemble rewriting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArgumentUsageHeader {
    /// Join the actual words with spaces without quoting their contents.
    RawWords,
    /// C Tcl 8.4 appends each original word as a NUL-terminated `CString`.
    RawCStringWords,
    /// Canonical C Tcl 8.5/8.6 leave the first word raw and quote later words.
    RawFirstListWords,
    /// Encode the actual words as Tcl list elements, preserving value bytes.
    ListWords,
}

impl NativeErrorArguments {
    /// Whether the post-head argument count is accepted.
    #[must_use]
    pub const fn accepts_len(self, count: usize) -> bool {
        count >= 1
            && count
                <= match self {
                    Self::Tcl => 3,
                    Self::JimStackTrace => 2,
                }
    }

    /// C error episodes publish compatibility globals; Jim retains its native
    /// result and stack trace without writing Tcl's errorInfo/errorCode cells.
    #[must_use]
    pub const fn publishes_tcl_error_globals(self) -> bool {
        matches!(self, Self::Tcl)
    }

    /// Actual native wrong-argument synopsis.
    #[must_use]
    pub const fn synopsis(self) -> &'static str {
        match self {
            Self::Tcl => "error message ?errorInfo? ?errorCode?",
            Self::JimStackTrace => "error message ?stacktrace?",
        }
    }
}

/// The execution policies selected at invocation ingress.
/// A missing Tcl release remains missing for a reimplementation such as Jim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvocationDialect {
    /// Native runtime family, independently of a pinned release or catalogue.
    pub native_family: Option<tcl_dialect::model::Family>,
    /// Exact runtime release and build point when resolved at environment ingress.
    pub core_point: Option<tcl_dialect::model::DialectPoint>,
    /// Complete source grammar retained when the catalogue has no profile.
    pub lexer_grammar: tcl_dialect::LexerGrammar,
    /// Selected embedding expression operators, independent of native compiler support.
    pub expression_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
    /// Shared braced-value and list-parser rules, independently of core version.
    pub word_values: tcl_syntax::word_rules::WordValueRules,
    /// Numeral grammar used to parse a frame selector.
    pub numbers: tcl_dialect::NumberSyntax,
    /// String indexing units of the actual engine or embedding contract.
    /// Unknown engines retain no character-model proof.
    pub characters: Option<tcl_dialect::StringCharacterModel>,
    /// Optional-level grammar used by `upvar`.
    pub upvar_level_presence: Option<tcl_dialect::FrameLevelPresence>,
    /// Optional-level grammar used by `uplevel`.
    pub uplevel_level_presence: Option<tcl_dialect::FrameLevelPresence>,
    /// C Tcl release, when this interpreter follows one.
    pub tcl_version: Option<tcl_dialect::TclVersion>,
    /// Package discovery protocol of this execution engine.
    pub package_protocol: Option<tcl_dialect::PackageProtocol>,
    /// Variable naming and activation ownership policy.
    pub variable_lookup_policy: Option<tcl_dialect::VariableLookupPolicy>,
    /// Whether aliases retain cells or selected-frame names across rebinding.
    pub variable_link_binding: Option<tcl_dialect::VariableLinkBinding>,
    /// Selected root-variable and array-element storage protocol.
    pub variable_container_model: Option<tcl_dialect::VariableContainerModel>,
    /// Whether a missing relative qualified variable cell can resolve globally.
    pub namespace_var_global_fallback: Option<bool>,
    /// Exit-status integer conversion independent of numeral grammar.
    pub process_exit_conversion: Option<tcl_dialect::ProcessExitConversion>,
    /// Child interpreter factory and handle protocol.
    pub interpreter_protocol: Option<tcl_dialect::InterpreterProtocol>,
    /// Whether namespace imports retain tokens or resolve source names later.
    pub namespace_import_binding: Option<tcl_dialect::NamespaceImportBinding>,
}

/// The original quoted operand settlement of a selected expression engine,
/// independently of the `subst` command's control-code policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionQuoteControl {
    /// C Tcl preserves abrupt bracket completions and their original options.
    Propagate,
    /// Current Jim consumes return values/levels, while break, continue and
    /// custom completion codes become errors in the quoted operand activation.
    Jim084,
}

/// An explicitly authored logical quote provider, distinct from native engine
/// evidence. A simulation provider cannot certify vendor hardware semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalExpressionQuoteProvider {
    /// The surrounding host explicitly simulates the documented Tcl84 core
    /// quote contract while retaining F5 source/operator grammar.
    Tcl84CoreSimulation,
}

/// An embedding host's explicit expression parser and template capability.
/// It authorizes an authored logical recipe, not native vendor semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalExpressionParseProvider {
    /// Tcl 8.4 core parsing with the retained F5 operator and word grammar.
    Tcl84CoreSimulation,
}

/// An embedding host's independently installed source-word byte recipe.
/// It grants source escape materialisation, without physical object cache authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalSourceWordProvider {
    /// Authored Tcl 8.4 core source bytes for the retained F5 source grammar.
    Tcl84CoreSimulation,
}

/// Native completion grammar, independently of the commands an embedding host exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionOptionsPolicy {
    /// Tcl 8.4: catch has one result variable and return has the original options.
    Legacy,
    /// Tcl 8.5 and later: catch can capture options and return can restore them.
    Options,
}

/// Native private namespace used by an ensemble implementation rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnsembleImplementationFamily {
    /// Tcl's info ensemble has private implementations from 8.5.
    Info,
    /// Tcl's namespace ensemble became privately callable in 8.6.
    Namespace,
    /// Tcl's dict ensemble has private implementations from 8.5.
    Dict,
    /// Tcl's string ensemble has private implementations from 8.5.
    String,
    /// Tcl's array ensemble became privately callable in 8.6.
    Array,
    /// Tcl's binary and nested codec ensembles became privately callable in 8.6.
    Binary,
    /// Tcl's file ensemble has private workers from C Tcl 8.6.
    File,
}

impl InvocationDialect {
    /// Actual `TclOO` ensemble target installed into the native `info` map.
    /// This is bootstrap geometry, not authority to resolve a live command.
    #[must_use]
    pub fn info_oo_ensemble_namespace(self, member: &str) -> Option<&'static str> {
        if self.family()? != tcl_dialect::model::Family::Tcl
            || self.tcl_version? < tcl_dialect::TclVersion::V8_6
        {
            return None;
        }
        match member {
            "class" => Some("::oo::InfoClass"),
            "object" => Some("::oo::InfoObject"),
            _ => None,
        }
    }

    /// Resolve a deliberately installed logical numeric simulation capability.
    /// This grants no authentic primitive getter, compiler or cache authority.
    /// The physical host engine is a separate invocation contract.
    #[must_use]
    pub fn authored_logical_numeric_simulation(
        self,
        provider: tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation,
    ) -> Option<tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation> {
        use tcl_dialect::model::Family;
        use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
        let family = self.native_family.or(self
            .core_point
            .map(tcl_dialect::model::DialectPoint::family))?;
        if !matches!(family, Family::F5Tcl | Family::F5Irules)
            || self.tcl_version != Some(tcl_dialect::TclVersion::V8_4)
            || self.numbers != tcl_dialect::NumberSyntax::Tcl84
            || self.core_point.is_some_and(|point| {
                point.family() != family
                    || point.tcl_version() != Some(tcl_dialect::TclVersion::V8_4)
            })
        {
            return None;
        }
        match provider {
            AuthoredLogicalNumericSimulation::Tcl84Core => Some(provider),
        }
    }

    /// Actual scalar getter byte boundary. A retained C native family and
    /// version suffice even when no explicit core point is stored; a vendor
    /// compatibility version or numeral grammar supplies no engine proof.
    /// Object string materialization remains an independent caller obligation.
    #[must_use]
    pub fn scalar_numeric_input_policy(
        self,
    ) -> Option<tcl_syntax::number::NativeScalarNumericInputPolicy> {
        use tcl_syntax::number::NativeScalarNumericInputPolicy as Policy;
        self.native_scalar_getter_protocol().map(|protocol| {
            if protocol.is_jim084() {
                Policy::NulTerminatedJim084
            } else {
                Policy::LengthDelimited
            }
        })
    }

    /// Actual primitive getter engine, independently of source numeral/lexer
    /// overrides. Explicit point, family and native release must agree; a
    /// compatible vendor release cannot supply native C getter authority.
    #[must_use]
    pub fn native_scalar_getter_protocol(
        self,
    ) -> Option<tcl_syntax::scalar_getter::NativeScalarGetterProtocol> {
        use tcl_dialect::model::Family;
        use tcl_syntax::scalar_getter::NativeScalarGetterProtocol as Protocol;
        if let Some(point) = self.core_point {
            if self
                .native_family
                .is_some_and(|family| family != point.family())
                || self.tcl_version.is_some_and(|version| {
                    point.family() != Family::Tcl || point.tcl_version() != Some(version)
                })
            {
                return None;
            }
            return Protocol::for_point(point);
        }
        match self.native_family {
            Some(Family::Tcl) => self.tcl_version.map(Protocol::for_tcl_version),
            _ => None,
        }
    }

    /// Actual native list-object serialization, independently of list parsing
    /// and script quoting. Unknown or unaudited engines retain no byte proof.
    #[must_use]
    pub fn list_result_serialization(
        self,
    ) -> Option<tcl_syntax::list_result::NativeListResultSerialization> {
        self.native_string_protocol()
            .map(tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol)
    }

    /// Complete expression syntax and diagnostic axes at this native entry,
    /// retaining independent authoring operator and host-word grammar.
    #[must_use]
    pub fn expression_parse_context(
        self,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> tcl_syntax::expr::parser::ExprParseContext {
        use tcl_syntax::expr::parser::NativeExprSyntax;
        tcl_syntax::expr::parser::ExprParseContext {
            lexer_grammar: self.lexer_grammar,
            expr_grammar_base: profile
                .map_or(self.tcl_version, |profile| profile.expr_grammar_base),
            f5_word_grammar: self
                .expression_word_grammar
                .or_else(|| profile.and_then(tcl_dialect::DialectProfile::f5_core_expr_grammar)),
            native_syntax: match self.family() {
                Some(tcl_dialect::model::Family::Tcl) => self
                    .tcl_version
                    .map_or(NativeExprSyntax::Unknown, NativeExprSyntax::Tcl),
                Some(tcl_dialect::model::Family::Jim)
                    if self.core_point.is_some_and(|point| {
                        point.release() == tcl_dialect::model::Release::JIM_0_84
                    }) =>
                {
                    NativeExprSyntax::Jim084
                }
                _ => NativeExprSyntax::Unknown,
            },
        }
    }

    /// Select an explicitly authored logical parser recipe. The embedding
    /// host must independently retain its actual engine before installing it.
    /// This leaves the native invocation and compiler admission unchanged.
    #[must_use]
    pub fn logical_expression_parse_context(
        self,
        provider: LogicalExpressionParseProvider,
        profile: &tcl_dialect::DialectProfile,
    ) -> Option<tcl_syntax::expr::parser::ExprParseContext> {
        match provider {
            LogicalExpressionParseProvider::Tcl84CoreSimulation
                if self.native_family == Some(tcl_dialect::model::Family::F5Irules)
                    && self.tcl_version == Some(tcl_dialect::TclVersion::V8_4)
                    && crate::InvocationDialect::of_profile(profile) == self =>
            {
                let mut context = self.expression_parse_context(Some(profile));
                context.native_syntax =
                    tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4);
                Some(context)
            }
            LogicalExpressionParseProvider::Tcl84CoreSimulation => None,
        }
    }

    /// Template variable grammar supplied by an explicit logical parser recipe.
    /// Availability or a compatible release alone cannot install this policy.
    #[must_use]
    pub fn logical_expression_template_policy(
        self,
        provider: LogicalExpressionParseProvider,
        profile: &tcl_dialect::DialectProfile,
    ) -> Option<crate::substitution::TemplateParseErrors> {
        self.logical_expression_parse_context(provider, profile)
            .map(|_| crate::substitution::TemplateParseErrors::Rejected)
    }

    /// Actual source-word byte recipe with the original escape grammar retained.
    #[must_use]
    pub fn native_source_string_protocol(
        self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        let protocol = self.native_string_protocol()?;
        (self.lexer_grammar.escapes == protocol.escape_syntax()).then_some(protocol)
    }

    /// Source-word simulation is independently installed and does not borrow
    /// name, numeric, expression parser or host engine recipes.
    #[must_use]
    pub fn logical_source_string_protocol(
        self,
        provider: LogicalSourceWordProvider,
        profile: &tcl_dialect::DialectProfile,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        match provider {
            LogicalSourceWordProvider::Tcl84CoreSimulation
                if self.native_family == Some(tcl_dialect::model::Family::F5Irules)
                    && self.tcl_version == Some(tcl_dialect::TclVersion::V8_4)
                    && self.lexer_grammar.escapes == tcl_dialect::EscapeSyntax::Tcl84
                    && Self::of_profile(profile) == self =>
            {
                Some(tcl_syntax::native_string::NativeStringProtocol::C(
                    tcl_dialect::TclVersion::V8_4,
                ))
            }
            LogicalSourceWordProvider::Tcl84CoreSimulation => None,
        }
    }

    /// Select the actual native error grammar without inferring a C release.
    #[must_use]
    pub fn error_arguments(self) -> Option<NativeErrorArguments> {
        if self.tcl_version.is_some() {
            Some(NativeErrorArguments::Tcl)
        } else if self.core_point.is_some_and(|point| {
            point.family() == tcl_dialect::model::Family::Jim
                && point.release() == tcl_dialect::model::Release::JIM_0_84
        }) {
            Some(NativeErrorArguments::JimStackTrace)
        } else {
            None
        }
    }

    /// Actual selected engine's procedure replacement control protocol.
    #[must_use]
    pub fn tailcall_protocol(self) -> Option<NativeTailcallProtocol> {
        self.error_stack_protocol().map(|protocol| match protocol {
            tcl_runtime_api::jim_error_stack::NativeErrorStackProtocol::Tcl => {
                NativeTailcallProtocol::Tcl
            }
            tcl_runtime_api::jim_error_stack::NativeErrorStackProtocol::Jim084 => {
                NativeTailcallProtocol::Jim084
            }
        })
    }

    /// Actual native script object's delimiter-validation boundary.
    #[must_use]
    pub fn script_parse_timing(self) -> Option<NativeScriptParseTiming> {
        self.error_stack_protocol().map(|protocol| match protocol {
            tcl_runtime_api::jim_error_stack::NativeErrorStackProtocol::Tcl => {
                NativeScriptParseTiming::BeforeEachCommand
            }
            tcl_runtime_api::jim_error_stack::NativeErrorStackProtocol::Jim084 => {
                NativeScriptParseTiming::BeforeScript
            }
        })
    }

    /// Native automatic-error capture, separate from explicit error operands.
    #[must_use]
    pub fn error_stack_protocol(
        self,
    ) -> Option<tcl_runtime_api::jim_error_stack::NativeErrorStackProtocol> {
        use tcl_runtime_api::jim_error_stack::NativeErrorStackProtocol;
        self.error_arguments().map(|arguments| match arguments {
            NativeErrorArguments::Tcl => NativeErrorStackProtocol::Tcl,
            NativeErrorArguments::JimStackTrace => NativeErrorStackProtocol::Jim084,
        })
    }

    /// Whether native alias/ensemble dispatch rewrites original usage prefixes.
    /// Pinned C8.4 predates the alias rewrite; Jim retains canonical usage.
    #[must_use]
    pub fn argument_usage_rewriting(self) -> Option<bool> {
        match self.tcl_version {
            Some(tcl_dialect::TclVersion::V8_4) => Some(false),
            Some(_) => Some(true),
            None if self
                .core_point
                .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84) =>
            {
                Some(false)
            }
            None => None,
        }
    }

    /// Render actual usage-prefix words under the selected native presenter.
    /// C Tcl 8.4 uses `CString` words, 8.5/8.6 quote words after the first,
    /// Tcl 9 quotes every word, and Jim retains raw length-delimited words.
    /// This does not select or rewrite an invocation header.
    #[must_use]
    pub fn argument_usage_header(self, words: &[String]) -> Option<String> {
        let operands: Vec<_> = words
            .iter()
            .map(|word| crate::native_usage::NativeUsageWord::Original(word.as_bytes()))
            .collect();
        let bytes = self.native_usage_protocol()?.render_header(&operands)?;
        String::from_utf8(bytes).ok()
    }

    /// Select the actual usage-prefix encoder without converting value bytes
    /// into Unicode. Runtimes use their native byte-list encoder for list words.
    #[must_use]
    pub fn argument_usage_header_style(self) -> Option<NativeArgumentUsageHeader> {
        self.native_usage_protocol()
            .map(crate::native_usage::NativeUsageProtocol::style)
    }

    /// Native wrong-argument validation code, selected independently of text.
    #[must_use]
    pub fn wrong_arguments_error_code(self) -> Option<&'static str> {
        self.native_wrong_arguments_protocol()
            .map(crate::native_wrong_arguments::NativeWrongArgumentsProtocol::error_code)
    }

    /// Native expression argv validation and source-object composition.
    #[must_use]
    pub fn expression_arguments(self) -> Option<tcl_dialect::ExpressionArguments> {
        self.tcl_version
            .map(|_| tcl_dialect::ExpressionArguments::Concatenate)
            .or_else(|| {
                self.core_point
                    .filter(|point| {
                        point.family() == tcl_dialect::model::Family::Jim
                            && point.release() == tcl_dialect::model::Release::JIM_0_84
                    })
                    .map(|_| tcl_dialect::ExpressionArguments::Single)
            })
    }

    /// Native lazy double-string policy. Mutable precision is a separate
    /// interpreter-world dependency, not a source numeral property.
    #[must_use]
    pub fn double_string_policy(self) -> Option<tcl_dialect::DoubleStringPolicy> {
        self.tcl_version
            .map(tcl_dialect::DoubleStringPolicy::for_tcl_version)
            .or_else(|| {
                self.core_point
                    .filter(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
                    .map(|_| tcl_dialect::DoubleStringPolicy::JimTwelve)
            })
    }

    /// Native formal parameter grammar, independently of registry availability.
    #[must_use]
    pub const fn parameter_grammar(self) -> Option<tcl_dialect::ParameterGrammar> {
        match self.variable_lookup_policy {
            Some(tcl_dialect::VariableLookupPolicy::Tcl) => {
                Some(tcl_dialect::ParameterGrammar::Tcl)
            }
            Some(tcl_dialect::VariableLookupPolicy::Jim) => {
                Some(tcl_dialect::ParameterGrammar::Jim)
            }
            None => None,
        }
    }

    /// Native integer tower, selected independently of numeral grammar.
    /// An unknown engine has no arithmetic proof.
    #[must_use]
    pub fn arithmetic(self) -> Option<tcl_dialect::NativeArithmetic> {
        self.tcl_version
            .map(tcl_dialect::NativeArithmetic::for_tcl_version)
            .or_else(|| {
                self.core_point
                    .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
                    .then_some(tcl_dialect::NativeArithmetic::JimWide)
            })
    }
    /// Private implementation namespace proved for the selected native engine.
    /// Unknown engines and Jim must retain their original public invocation.
    #[must_use]
    pub fn ensemble_implementation_namespace(
        self,
        family: EnsembleImplementationFamily,
    ) -> Option<&'static str> {
        use tcl_dialect::TclVersion;
        match (family, self.tcl_version) {
            (EnsembleImplementationFamily::Info, Some(version)) if version >= TclVersion::V8_5 => {
                Some("::tcl::info")
            }
            (EnsembleImplementationFamily::Namespace, Some(version))
                if version >= TclVersion::V8_6 =>
            {
                Some("::tcl::namespace")
            }
            (EnsembleImplementationFamily::Dict, Some(version)) if version >= TclVersion::V8_5 => {
                Some("::tcl::dict")
            }
            (EnsembleImplementationFamily::String, Some(version))
                if version >= TclVersion::V8_5 =>
            {
                Some("::tcl::string")
            }
            (EnsembleImplementationFamily::Array, Some(version)) if version >= TclVersion::V8_6 => {
                Some("::tcl::array")
            }
            (EnsembleImplementationFamily::Binary, Some(version))
                if version >= TclVersion::V8_6 =>
            {
                Some("::tcl::binary")
            }
            (EnsembleImplementationFamily::File, Some(version)) if version >= TclVersion::V8_6 => {
                Some("::tcl::file")
            }
            _ => None,
        }
    }
    /// Concat protocol proved for this native engine. Jim requires representation
    /// evidence in addition to evaluated string operands.
    #[must_use]
    pub fn concat_policy(self) -> Option<tcl_dialect::ConcatPolicy> {
        if let Some(version) = self.tcl_version {
            Some(tcl_dialect::ConcatPolicy::Tcl(version))
        } else {
            self.core_point
                .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
                .then_some(tcl_dialect::ConcatPolicy::JimRepresentationSensitive)
        }
    }

    /// Complete index policy; an unselected engine abstains.
    #[must_use]
    pub fn index_syntax(self) -> Option<tcl_dialect::IndexSyntax> {
        self.tcl_version
            .map(tcl_dialect::IndexSyntax::for_version)
            .or_else(|| {
                self.core_point
                    .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
                    .then_some(tcl_dialect::IndexSyntax {
                        numbers: self.numbers,
                        grammar: tcl_dialect::IndexGrammar::Jim,
                        width: tcl_dialect::IndexIntegerWidth::Jim32,
                        end_abbreviations: false,
                    })
            })
    }

    /// Version-owned list replacement bounds; an unselected engine abstains.
    #[must_use]
    pub fn list_set_bounds(self) -> Option<tcl_dialect::ListSetBounds> {
        self.tcl_version
            .map(tcl_dialect::TclVersion::list_set_bounds)
            .or_else(|| {
                self.core_point
                    .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
                    .then_some(tcl_dialect::ListSetBounds::ExistingElement)
            })
    }

    /// Positional `catch` envelope, when the dialect has that grammar.
    /// Jim's leading completion-code switches require its invocation selector.
    #[must_use]
    pub fn catch_positional_arity(self) -> Option<crate::Arity> {
        match self.completion_options_policy()? {
            CompletionOptionsPolicy::Legacy => Some(crate::Arity::new(1, 2)),
            CompletionOptionsPolicy::Options => Some(crate::Arity::new(1, 3)),
        }
    }
    /// Native expression quote settlement; unknown/vendor engine axes abstain.
    #[must_use]
    pub fn expression_quote_control(self) -> Option<ExpressionQuoteControl> {
        match self.native_family {
            Some(tcl_dialect::model::Family::Tcl) if self.tcl_version.is_some() => {
                Some(ExpressionQuoteControl::Propagate)
            }
            Some(tcl_dialect::model::Family::Jim)
                if self.core_point.is_some_and(|point| {
                    point.release() == tcl_dialect::model::Release::JIM_0_84
                }) =>
            {
                Some(ExpressionQuoteControl::Jim084)
            }
            _ => None,
        }
    }

    /// Resolve an explicitly authored logical simulation quote contract.
    /// This carries no native engine, compiler hook or result-production proof.
    #[must_use]
    pub fn logical_expression_quote_control(
        self,
        provider: LogicalExpressionQuoteProvider,
    ) -> Option<ExpressionQuoteControl> {
        match provider {
            LogicalExpressionQuoteProvider::Tcl84CoreSimulation
                if self.native_family == Some(tcl_dialect::model::Family::F5Irules)
                    && self.tcl_version == Some(tcl_dialect::TclVersion::V8_4) =>
            {
                Some(ExpressionQuoteControl::Propagate)
            }
            LogicalExpressionQuoteProvider::Tcl84CoreSimulation => None,
        }
    }

    /// The completion grammar of the actual runtime, not its host command catalogue.
    #[must_use]
    pub fn completion_options_policy(self) -> Option<CompletionOptionsPolicy> {
        self.tcl_version.map(|version| {
            if version < tcl_dialect::TclVersion::V8_5 {
                CompletionOptionsPolicy::Legacy
            } else {
                CompletionOptionsPolicy::Options
            }
        })
    }
    /// Resolve the policies of a concrete C Tcl interpreter release.
    #[must_use]
    pub fn for_version(version: tcl_dialect::TclVersion) -> Self {
        Self {
            native_family: Some(tcl_dialect::model::Family::Tcl),
            core_point: None,
            expression_word_grammar: None,
            lexer_grammar: tcl_dialect::grammar_of_dialect_name(Some(
                version.dialect_profile_name(),
            )),
            word_values: tcl_syntax::word_rules::WordValueRules::TCL,
            numbers: version.number_syntax(),
            characters: Some(version.string_character_model()),
            upvar_level_presence: Some(version.upvar_level_presence()),
            uplevel_level_presence: Some(version.uplevel_level_presence()),
            tcl_version: Some(version),
            package_protocol: Some(tcl_dialect::PackageProtocol::Tcl),
            variable_lookup_policy: Some(tcl_dialect::VariableLookupPolicy::Tcl),
            variable_link_binding: Some(tcl_dialect::VariableLinkBinding::StableCell),
            variable_container_model: Some(tcl_dialect::VariableContainerModel::DistinctArray),
            namespace_var_global_fallback: Some(version.namespace_var_global_fallback()),
            process_exit_conversion: Some(if version < tcl_dialect::TclVersion::V9_0 {
                tcl_dialect::ProcessExitConversion::Narrow32
            } else {
                tcl_dialect::ProcessExitConversion::WholeInteger
            }),
            interpreter_protocol: Some(tcl_dialect::InterpreterProtocol::Tcl),
            namespace_import_binding: Some(tcl_dialect::NamespaceImportBinding::CommandToken),
        }
    }

    /// Resolve policies from an environment's explicit core point.
    #[must_use]
    pub fn of_point(point: tcl_dialect::model::DialectPoint) -> Self {
        let profile = tcl_dialect::DialectProfile::projected_from_point("", &[], "", point);
        Self {
            core_point: Some(point),
            ..Self::of_profile(&profile)
        }
    }

    /// Whether a quoted sole substitution retains its input object. The pinned
    /// Jim parser constructs a new string; C Tcl preserves the substituted value.
    /// Unknown engine points do not supply an object-identity guarantee.
    #[must_use]
    pub fn quoted_substitution_preserves_object(self) -> Option<bool> {
        if self.tcl_version.is_some() {
            return Some(true);
        }
        self.core_point
            .filter(|point| {
                point.family() == tcl_dialect::model::Family::Jim
                    && point.release() == tcl_dialect::model::Release::JIM_0_84
            })
            .map(|_| false)
    }

    /// Surface query for an explicitly selected runtime core, including its
    /// command-set ancestry anchor. No missing core becomes a C default.
    #[must_use]
    pub fn authoring_query(self) -> Option<tcl_dialect::model::SurfaceQuery<'static>> {
        use tcl_dialect::model::{CorePoints, SurfaceQuery};
        let point = self.core_point;
        let family = self.family()?;
        let release = point.map(|point| point.release().as_str()).or_else(|| {
            (family == tcl_dialect::model::Family::Tcl)
                .then_some(self.tcl_version)
                .flatten()
                .map(tcl_dialect::TclVersion::version_string)
        });
        let core = family.ancestry().map_or_else(
            || CorePoints::one(family, release),
            |ancestry| CorePoints::two((family, release), (ancestry.parent, Some(ancestry.anchor))),
        );
        Some(SurfaceQuery {
            realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            core,
            packages: &[],
        })
    }

    /// Project independently owned policies from the selected profile.
    #[must_use]
    pub fn of_profile(profile: &tcl_dialect::DialectProfile) -> Self {
        Self {
            native_family: profile
                .vendor_surface
                .into_iter()
                .chain(profile.grammar_union.iter().copied())
                .find_map(|provider| match provider {
                    tcl_dialect::model::SpecProvider::Core(family) => Some(family),
                    tcl_dialect::model::SpecProvider::Package(_) => None,
                }),
            core_point: profile.core_point,
            lexer_grammar: profile.grammar,
            expression_word_grammar: profile.f5_core_expr_grammar(),
            word_values: tcl_syntax::word_rules::WordValueRules::from_grammar(&profile.grammar),
            numbers: profile.grammar.numbers,
            characters: profile.character_model(),
            upvar_level_presence: profile.upvar_level_presence(),
            uplevel_level_presence: profile.uplevel_level_presence(),
            tcl_version: profile.runtime_version(),
            package_protocol: profile.package_protocol(),
            variable_lookup_policy: profile.variable_lookup_policy(),
            variable_link_binding: profile.variable_link_binding(),
            variable_container_model: profile.variable_container_model(),
            namespace_var_global_fallback: match profile.variable_lookup_policy() {
                Some(tcl_dialect::VariableLookupPolicy::Jim) => Some(false),
                Some(tcl_dialect::VariableLookupPolicy::Tcl) => profile
                    .runtime_version()
                    .map(tcl_dialect::TclVersion::namespace_var_global_fallback),
                None => None,
            },
            process_exit_conversion: profile.process_exit_conversion(),
            interpreter_protocol: profile.interpreter_protocol(),
            namespace_import_binding: profile.namespace_import_binding(),
        }
    }

    /// Runtime family, with an explicit point taking precedence over profile projection.
    #[must_use]
    pub fn family(self) -> Option<tcl_dialect::model::Family> {
        self.core_point
            .map(tcl_dialect::model::DialectPoint::family)
            .or(self.native_family)
    }

    /// Exact execution point for an actual interpreter invocation. Static C
    /// profiles retain a typed release rather than an explicit point. Only
    /// the C family can turn that release into its own canonical point;
    /// another family's compatible Tcl release grants no C implementation.
    #[must_use]
    pub fn execution_point(self) -> Option<tcl_dialect::model::DialectPoint> {
        use tcl_dialect::model::{DialectPoint, Family};
        if let Some(point) = self.core_point {
            if self
                .native_family
                .is_some_and(|family| family != point.family())
                || (point.family() == Family::Tcl
                    && self
                        .tcl_version
                        .is_some_and(|version| point.tcl_version() != Some(version)))
            {
                return None;
            }
            return Some(point);
        }
        (self.native_family == Some(Family::Tcl))
            .then_some(self.tcl_version)
            .flatten()
            .map(DialectPoint::for_tcl_version)
    }
}

/// A borrowed, allocation-free view of post-head invocation words.
///
/// The literal representation keeps the historic `&[&str]` path zero-copy.
/// A structured representation is used when a source consumer knows that a
/// word was substituted, expanded, or opaque.  Query methods intentionally
/// expose values only through [`Self::literal_at`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationArguments<'w> {
    /// A compatibility view in which every word is known literal.
    Literals(&'w [&'w str]),
    /// A source-aware view containing individual word facts.
    Structured(&'w [InvocationWord<'w>]),
    /// Literal arguments carrying the ingress execution policies.
    ContextualLiterals(&'w [&'w str], InvocationDialect),
    /// Source-aware arguments carrying the ingress execution policies.
    ContextualStructured(&'w [InvocationWord<'w>], InvocationDialect),
}

/// Knowledge about one post-expansion argv position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationArgument<'w> {
    /// The position maps to one source word with the stated value knowledge.
    Word(InvocationWord<'w>),
    /// The source is known to contribute fewer argv entries than this index.
    Missing,
    /// An earlier expansion or opaque region makes this position unknowable.
    Indeterminate,
}

impl<'w> InvocationArguments<'w> {
    /// Construct an all-literal, zero-copy argument view.
    #[must_use]
    pub const fn literals(words: &'w [&'w str]) -> Self {
        Self::Literals(words)
    }

    /// Construct a source-aware argument view.
    #[must_use]
    pub const fn structured(words: &'w [InvocationWord<'w>]) -> Self {
        Self::Structured(words)
    }

    /// Stamp explicitly resolved policies, including non-catalogue environments.
    #[must_use]
    pub const fn with_dialect(self, dialect: InvocationDialect) -> Self {
        match self {
            Self::Literals(words) | Self::ContextualLiterals(words, _) => {
                Self::ContextualLiterals(words, dialect)
            }
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                Self::ContextualStructured(words, dialect)
            }
        }
    }

    /// Stamp the execution policies while retaining source-value knowledge.
    #[must_use]
    pub fn with_profile(self, profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        if self.dialect().is_some() {
            return self;
        }
        let Some(profile) = profile else { return self };
        self.with_dialect(InvocationDialect::of_profile(profile))
    }

    /// The selected execution policies, when the caller supplied a profile.
    #[must_use]
    pub const fn dialect(self) -> Option<InvocationDialect> {
        match self {
            Self::ContextualLiterals(_, dialect) | Self::ContextualStructured(_, dialect) => {
                Some(dialect)
            }
            Self::Literals(_) | Self::Structured(_) => None,
        }
    }

    /// Number of source words following the command head.
    ///
    /// This is not necessarily the final argv count; see
    /// [`Self::exact_argv_len`].
    #[must_use]
    pub const fn len(self) -> usize {
        match self {
            Self::Literals(words) | Self::ContextualLiterals(words, _) => words.len(),
            Self::Structured(words) | Self::ContextualStructured(words, _) => words.len(),
        }
    }

    /// Whether there are no post-head source words.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Return the word fact at `index`.
    #[must_use]
    pub fn get(self, index: usize) -> Option<InvocationWord<'w>> {
        match self {
            Self::Literals(words) | Self::ContextualLiterals(words, _) => {
                words.get(index).copied().map(InvocationWord::Literal)
            }
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                words.get(index).copied()
            }
        }
    }

    /// Project one final argv position while retaining precise source prefixes.
    ///
    /// A `{*}` or opaque word makes its own position and every following
    /// position indeterminate, but it does not erase exact words before it.
    #[must_use]
    pub fn argv_at(self, index: usize) -> InvocationArgument<'w> {
        match self {
            Self::Literals(words) | Self::ContextualLiterals(words, _) => words
                .get(index)
                .copied()
                .map_or(InvocationArgument::Missing, |word| {
                    InvocationArgument::Word(InvocationWord::Literal(word))
                }),
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                let mut argv_index = 0;
                for word in words {
                    match word {
                        InvocationWord::Expanded | InvocationWord::Opaque => {
                            return InvocationArgument::Indeterminate;
                        }
                        InvocationWord::Literal(_)
                        | InvocationWord::Dynamic
                        | InvocationWord::DynamicNonOption
                        | InvocationWord::ArrayElementName { .. } => {
                            if argv_index == index {
                                return InvocationArgument::Word(*word);
                            }
                            argv_index += 1;
                        }
                    }
                }
                InvocationArgument::Missing
            }
        }
    }

    /// Return a post-head value only when it is known literal.
    #[must_use]
    pub fn literal_at(self, index: usize) -> Option<&'w str> {
        self.get(index).and_then(InvocationWord::literal)
    }

    /// Proved array-root shape at one final argv position, excluding uncertain expansion offsets.
    #[must_use]
    pub fn array_element_root_at(self, index: usize) -> Option<&'w str> {
        match self.argv_at(index) {
            InvocationArgument::Word(InvocationWord::ArrayElementName { root }) => Some(root),
            _ => None,
        }
    }

    /// Whether every source word has a known literal Tcl value.
    #[must_use]
    pub fn are_all_literals(self) -> bool {
        match self {
            Self::Literals(_) | Self::ContextualLiterals(_, _) => true,
            Self::Structured(words) | Self::ContextualStructured(words, _) => words
                .iter()
                .all(|word| matches!(word, InvocationWord::Literal(_))),
        }
    }

    /// Whether every source word is guaranteed to contribute one argv entry.
    #[must_use]
    pub fn has_exact_argv_len(self) -> bool {
        match self {
            Self::Literals(_) | Self::ContextualLiterals(_, _) => true,
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                words.iter().all(|word| word.has_exactly_one_argv_entry())
            }
        }
    }

    /// Return the final argv count when expansion cannot change it.
    #[must_use]
    pub fn exact_argv_len(self) -> Option<usize> {
        self.has_exact_argv_len().then_some(self.len())
    }

    /// Whether any word is dynamic, expanded, or opaque.
    #[must_use]
    pub fn has_non_literal(self) -> bool {
        !self.are_all_literals()
    }

    /// Whether this view retained source word shape rather than accepting the
    /// legacy string-only compatibility projection.
    #[must_use]
    pub const fn is_source_aware(self) -> bool {
        matches!(self, Self::Structured(_) | Self::ContextualStructured(_, _))
    }

    /// Return every post-head Tcl value when all source words are literal.
    ///
    /// This is the explicit compatibility bridge for established registry
    /// resolvers whose input is `&[&str]`.  A dynamic, expanded, or opaque
    /// word makes the whole projection unavailable; callers must retain their
    /// typed unknown obligation instead of passing source spelling to the
    /// resolver.
    #[must_use]
    pub fn literal_values(self) -> Option<Vec<&'w str>> {
        match self {
            Self::Literals(words) | Self::ContextualLiterals(words, _) => Some(words.to_vec()),
            Self::Structured(words) | Self::ContextualStructured(words, _) => {
                words.iter().map(|word| word.literal()).collect()
            }
        }
    }

    /// Borrow the arguments from `start` onwards while preserving their
    /// source-knowledge representation.
    #[must_use]
    pub fn slice_from(self, start: usize) -> Self {
        match self {
            Self::Literals(words) => Self::Literals(words.get(start..).unwrap_or(&[])),
            Self::Structured(words) => Self::Structured(words.get(start..).unwrap_or(&[])),
            Self::ContextualLiterals(words, dialect) => {
                Self::ContextualLiterals(words.get(start..).unwrap_or(&[]), dialect)
            }
            Self::ContextualStructured(words, dialect) => {
                Self::ContextualStructured(words.get(start..).unwrap_or(&[]), dialect)
            }
        }
    }
}

/// Inputs supplied to a registry command-prefix resolver.
///
/// `spellings` preserves the compatibility view used by position-only
/// resolvers. `words` is the source-aware truth used by any resolver whose
/// appended arity depends on a literal argument value. Such a resolver must
/// use [`Self::literal_at`] and abstain when it returns `None`, never interpret
/// a dynamic word's source spelling as its runtime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPrefixArguments<'w> {
    spellings: &'w [&'w str],
    words: InvocationArguments<'w>,
}

impl<'w> CommandPrefixArguments<'w> {
    /// Construct a compatibility view in which every argument is literal.
    #[must_use]
    pub const fn literals(spellings: &'w [&'w str]) -> Self {
        Self {
            spellings,
            words: InvocationArguments::literals(spellings),
        }
    }

    /// Construct a source-aware view alongside reconstructed source
    /// spellings. Both slices must describe the same post-head words.
    #[must_use]
    pub fn structured(spellings: &'w [&'w str], words: &'w [InvocationWord<'w>]) -> Option<Self> {
        (spellings.len() == words.len()).then_some(Self {
            spellings,
            words: InvocationArguments::structured(words),
        })
    }

    /// Number of post-head source words.
    #[must_use]
    pub const fn len(self) -> usize {
        self.words.len()
    }

    /// Whether there are no post-head source words.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.words.is_empty()
    }

    /// Reconstructed source spellings for position-only compatibility
    /// resolvers. Do not use these to make a value-dependent decision.
    #[must_use]
    pub const fn spellings(self) -> &'w [&'w str] {
        self.spellings
    }

    /// Return an argument value only when source analysis proves it literal.
    #[must_use]
    pub fn literal_at(self, index: usize) -> Option<&'w str> {
        self.words.literal_at(index)
    }

    /// The source-aware view: values where proven literal, and the
    /// dynamic / expanded / opaque classification everywhere else. A resolver
    /// that must report *why* a word is not usable — the `SpecTcl` hook host
    /// builds the DSL's `kinds` from it — reads this rather than inferring
    /// "not literal" from [`Self::literal_at`] returning `None`.
    #[must_use]
    pub const fn words(self) -> InvocationArguments<'w> {
        self.words
    }

    /// Borrow the arguments from `start` onwards, preserving both views.
    #[must_use]
    pub fn slice_from(self, start: usize) -> Self {
        Self {
            spellings: self.spellings.get(start..).unwrap_or(&[]),
            words: self.words.slice_from(start),
        }
    }
}

/// A structured source-word view for one invocation.
///
/// The command head is structured too.  Registry resolution can only select
/// a command descriptor when [`Self::head_literal`] is present; it never
/// treats a substituted head's source spelling as a command name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvocationWords<'w> {
    head: InvocationWord<'w>,
    arguments: InvocationArguments<'w>,
}

/// Registry-owned projection of the variable cells an invocation may write.
///
/// Literal names are effective Tcl cell names: an unqualified option value
/// whose registry descriptor declares [`VariableScope::Global`](crate::VariableScope)
/// is rooted with `::`. They remain useful even when another write target is
/// source-opaque. [`Self::opaque_variable_frame`] means substitution,
/// expansion, or a computed command head prevents the registry from
/// enumerating every possible target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VariableWriteProjection {
    /// Statically known literal variable names written by the invocation.
    pub literal_names: Vec<String>,
    /// The subset of [`Self::literal_names`] the invocation **reads before
    /// writing** ([`Traits::READS_BEFORE_WRITE`](crate::Traits::READS_BEFORE_WRITE)
    /// — `incr` / `append` / `lappend` / `lset` / `lpop` / `ledit`). An
    /// effect consumer needs the read as much as the write: the store that
    /// feeds `[incr n]` is observed by it, not overwritten by it.
    pub read_before_write_names: Vec<String>,
    /// Whether the invocation may write another, unnameable variable cell.
    pub opaque_variable_frame: bool,
}

/// Registry-owned projection of the variable cells an invocation reads **by
/// name** — the read-only counterpart of [`VariableWriteProjection`].
///
/// An [`ArgRole::VarRead`](crate::ArgRole::VarRead) word names a cell the
/// command observes without storing to it: `info exists n`, `array exists a`,
/// `array size a`. The store feeding such a name is observed, not
/// overwritten, so an effect consumer that records only writes reports that
/// store dead (#2132).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VariableReadProjection {
    /// Statically known literal variable names read by the invocation.
    pub literal_names: Vec<String>,
    /// Whether the invocation may also read a cell that cannot be named — a
    /// substituted or `{*}`-expanded name word.
    pub opaque_variable_frame: bool,
}

impl<'w> InvocationWords<'w> {
    /// Construct an all-literal invocation without allocating.
    #[must_use]
    pub const fn literals(head: &'w str, arguments: &'w [&'w str]) -> Self {
        Self {
            head: InvocationWord::Literal(head),
            arguments: InvocationArguments::Literals(arguments),
        }
    }

    /// Construct an invocation from source-aware word facts.
    #[must_use]
    pub const fn structured(head: InvocationWord<'w>, arguments: &'w [InvocationWord<'w>]) -> Self {
        Self {
            head,
            arguments: InvocationArguments::Structured(arguments),
        }
    }

    /// Construct an invocation while retaining the argument view's exact
    /// source knowledge and execution policies.
    #[must_use]
    pub const fn from_arguments(
        head: InvocationWord<'w>,
        arguments: InvocationArguments<'w>,
    ) -> Self {
        Self { head, arguments }
    }

    /// Stamp explicitly resolved execution policies on post-head words.
    #[must_use]
    pub const fn with_dialect(mut self, dialect: InvocationDialect) -> Self {
        self.arguments = self.arguments.with_dialect(dialect);
        self
    }

    /// Stamp execution policies on the post-head words.
    #[must_use]
    pub fn with_profile(mut self, profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        self.arguments = self.arguments.with_profile(profile);
        self
    }

    /// Return the command-head word fact.
    #[must_use]
    pub const fn head(self) -> InvocationWord<'w> {
        self.head
    }

    /// Return the command head only when it is a known literal.
    #[must_use]
    pub const fn head_literal(self) -> Option<&'w str> {
        self.head.literal()
    }

    /// Return the structured post-head argument view.
    #[must_use]
    pub const fn arguments(self) -> InvocationArguments<'w> {
        self.arguments
    }

    /// Borrow the selected argv and its dialect snapshot while recursively
    /// interpreting native bodies. The owning invocation outlives the borrow.
    #[must_use]
    pub const fn arguments_ref(&self) -> &InvocationArguments<'w> {
        &self.arguments
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn logical_numeric_simulation_requires_explicit_consistent_f5_contract() {
        use tcl_dialect::model::{DialectPoint, Family, Release};
        use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
        let provider = AuthoredLogicalNumericSimulation::Tcl84Core;
        for release in [Release::F5_TCL_TMOS, Release::F5_IRULES_TMM] {
            let dialect = InvocationDialect::of_point(DialectPoint::canonical(release));
            assert_eq!(
                dialect.authored_logical_numeric_simulation(provider),
                Some(provider)
            );
            assert_eq!(dialect.native_scalar_getter_protocol(), None);
            assert_eq!(dialect.scalar_numeric_input_policy(), None);
            let mut conflict = dialect;
            conflict.core_point =
                Some(DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V9_0));
            assert_eq!(conflict.authored_logical_numeric_simulation(provider), None);
            conflict = dialect;
            conflict.tcl_version = Some(tcl_dialect::TclVersion::V9_0);
            assert_eq!(conflict.authored_logical_numeric_simulation(provider), None);
            conflict = dialect;
            conflict.numbers = tcl_dialect::NumberSyntax::Tcl90;
            assert_eq!(conflict.authored_logical_numeric_simulation(provider), None);
        }
        let mut unknown = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        assert_eq!(unknown.authored_logical_numeric_simulation(provider), None);
        unknown.native_family = None;
        assert_eq!(unknown.authored_logical_numeric_simulation(provider), None);
        unknown.native_family = Some(Family::F5Irules);
        assert_eq!(
            unknown.authored_logical_numeric_simulation(provider),
            Some(provider)
        );
        let jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        assert_eq!(jim.authored_logical_numeric_simulation(provider), None);
    }

    #[test]
    fn execution_point_retains_actual_release_without_borrowing_a_vendor_base() {
        use tcl_dialect::model::{DialectPoint, Family, Release};
        for version in tcl_dialect::TclVersion::ALL {
            let mut dialect = InvocationDialect::for_version(version);
            assert_eq!(
                dialect.execution_point(),
                Some(DialectPoint::for_tcl_version(version))
            );
            dialect.native_family = Some(Family::F5Irules);
            assert_eq!(dialect.execution_point(), None);
            dialect.native_family = None;
            assert_eq!(dialect.execution_point(), None);
        }
        let jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        assert_eq!(jim.execution_point(), jim.core_point);
        let mut conflict = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        conflict.core_point = Some(DialectPoint::canonical(Release::TCL_9_0));
        assert_eq!(conflict.execution_point(), None);
    }

    #[test]
    fn scalar_numeric_input_retains_actual_family_without_an_explicit_point() {
        use tcl_syntax::number::NativeScalarNumericInputPolicy as Policy;
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            assert!(dialect.core_point.is_none());
            assert_eq!(
                dialect.scalar_numeric_input_policy(),
                Some(Policy::LengthDelimited)
            );
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(
            jim.scalar_numeric_input_policy(),
            Some(Policy::NulTerminatedJim084)
        );
        for family in [
            tcl_dialect::model::Family::F5Tcl,
            tcl_dialect::model::Family::F5Irules,
        ] {
            let mut foreign = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
            foreign.native_family = Some(family);
            assert_eq!(foreign.scalar_numeric_input_policy(), None);
        }
        let mut unknown = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        unknown.native_family = None;
        assert_eq!(unknown.scalar_numeric_input_policy(), None);
    }

    #[test]
    fn primitive_getter_selection_rejects_conflicting_actual_axes() {
        use tcl_dialect::{
            TclVersion,
            model::{DialectPoint, Family, Release},
        };
        let mut dialect = InvocationDialect::for_version(TclVersion::V8_6);
        dialect.core_point = Some(DialectPoint::for_tcl_version(TclVersion::V9_1));
        assert_eq!(dialect.native_scalar_getter_protocol(), None);
        dialect.tcl_version = Some(TclVersion::V9_1);
        assert_eq!(
            dialect
                .native_scalar_getter_protocol()
                .unwrap()
                .tcl_version(),
            Some(TclVersion::V9_1)
        );
        dialect.native_family = Some(Family::F5Irules);
        assert_eq!(dialect.native_scalar_getter_protocol(), None);

        let mut jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        assert!(jim.native_scalar_getter_protocol().unwrap().is_jim084());
        jim.tcl_version = Some(TclVersion::V8_6);
        assert_eq!(jim.native_scalar_getter_protocol(), None);
        jim.tcl_version = None;
        jim.native_family = Some(Family::Tcl);
        assert_eq!(jim.native_scalar_getter_protocol(), None);
    }

    #[test]
    fn invocation_character_model_retains_embedding_policy() {
        let profile = tcl_dialect::DialectProfile::irules();
        assert_eq!(
            InvocationDialect::of_profile(profile).characters,
            Some(tcl_dialect::StringCharacterModel::Utf16CodeUnits)
        );
        for version in tcl_dialect::TclVersion::ALL {
            assert_eq!(
                InvocationDialect::for_version(version).characters,
                Some(version.string_character_model())
            );
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(
            jim.characters,
            Some(tcl_dialect::StringCharacterModel::Jim084Utf8)
        );
    }

    #[test]
    fn private_ensemble_implementations_follow_native_engine_versions() {
        use tcl_dialect::TclVersion;
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            assert_eq!(
                dialect
                    .ensemble_implementation_namespace(EnsembleImplementationFamily::Namespace)
                    .is_some(),
                version >= TclVersion::V8_6
            );
            assert_eq!(
                dialect
                    .ensemble_implementation_namespace(EnsembleImplementationFamily::Dict)
                    .is_some(),
                version >= TclVersion::V8_5
            );
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(
            jim.ensemble_implementation_namespace(EnsembleImplementationFamily::Namespace),
            None
        );
        assert_eq!(
            jim.ensemble_implementation_namespace(EnsembleImplementationFamily::Dict),
            None
        );
    }
    #[test]
    fn invocation_context_survives_argument_projection_and_expansion() {
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        let words = [InvocationWord::Literal("require"), InvocationWord::Expanded];
        let arguments = InvocationArguments::Structured(&words).with_dialect(dialect);
        assert_eq!(arguments.slice_from(1).dialect(), Some(dialect));
        assert_eq!(arguments.slice_from(1).exact_argv_len(), None);
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(jim.tcl_version, None);
        assert_eq!(
            jim.package_protocol,
            Some(tcl_dialect::PackageProtocol::Jim)
        );
        assert_eq!(
            jim.namespace_import_binding,
            Some(tcl_dialect::NamespaceImportBinding::SourceName)
        );
    }

    use super::*;

    #[test]
    fn structured_words_only_expose_known_literals() {
        let words = [
            InvocationWord::Literal("fixed"),
            InvocationWord::Dynamic,
            InvocationWord::Expanded,
            InvocationWord::Opaque,
        ];
        let arguments = InvocationArguments::structured(&words);

        assert_eq!(arguments.literal_at(0), Some("fixed"));
        assert_eq!(arguments.literal_at(1), None);
        assert_eq!(arguments.literal_at(2), None);
        assert_eq!(arguments.literal_at(3), None);
        assert!(!arguments.are_all_literals());
        assert!(!arguments.has_exact_argv_len());
        assert_eq!(arguments.exact_argv_len(), None);
        assert_eq!(arguments.literal_values(), None);
    }

    #[test]
    fn literal_values_preserve_complete_known_argv() {
        let structured = [
            InvocationWord::Literal("first"),
            InvocationWord::Literal("second"),
        ];
        assert_eq!(
            InvocationArguments::structured(&structured).literal_values(),
            Some(vec!["first", "second"])
        );
    }

    #[test]
    fn non_option_dynamic_keeps_argv_shape_without_exposing_a_value() {
        let arguments = InvocationArguments::structured(&[InvocationWord::DynamicNonOption]);
        assert!(arguments.has_exact_argv_len());
        assert_eq!(arguments.literal_at(0), None);
        assert_eq!(arguments.literal_values(), None);
        assert_eq!(
            arguments.get(0).map(InvocationWord::kind),
            Some(InvocationWordKind::Dynamic)
        );
    }

    #[test]
    fn source_word_materialisation_selects_actual_or_explicit_logical_recipes() {
        let provider = super::LogicalSourceWordProvider::Tcl84CoreSimulation;
        let logical = tcl_dialect::DialectProfile::irules();
        let dialect = crate::InvocationDialect::of_profile(logical);
        assert!(dialect.native_source_string_protocol().is_none());
        assert_eq!(
            dialect.logical_source_string_protocol(provider, logical),
            Some(tcl_syntax::native_string::NativeStringProtocol::C(
                tcl_dialect::TclVersion::V8_4
            ))
        );
        assert!(dialect.execution_point().is_none());
        for version in tcl_dialect::TclVersion::ALL {
            let actual = crate::InvocationDialect::for_version(version);
            assert_eq!(
                actual.native_source_string_protocol(),
                Some(tcl_syntax::native_string::NativeStringProtocol::C(version))
            );
            assert!(
                actual
                    .logical_source_string_protocol(provider, logical)
                    .is_none()
            );
            let mut changed = actual;
            changed.lexer_grammar.escapes = tcl_dialect::EscapeSyntax::Jim;
            assert!(changed.native_source_string_protocol().is_none());
        }
    }

    #[test]
    fn logical_parser_recipe_does_not_select_a_native_vendor_engine() {
        let profile = tcl_dialect::DialectProfile::irules();
        let dialect = crate::InvocationDialect::of_profile(profile);
        let provider = super::LogicalExpressionParseProvider::Tcl84CoreSimulation;
        assert_eq!(
            dialect
                .expression_parse_context(Some(profile))
                .native_syntax,
            tcl_syntax::expr::parser::NativeExprSyntax::Unknown
        );
        let context = dialect
            .logical_expression_parse_context(provider, profile)
            .expect("explicit authored F5 parser recipe");
        assert_eq!(context.lexer_grammar, profile.grammar);
        assert_eq!(context.f5_word_grammar, profile.f5_core_expr_grammar());
        assert_eq!(
            context.native_syntax,
            tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4)
        );
        assert!(dialect.execution_point().is_none());
        assert!(crate::substitution::TemplateParseErrors::for_dialect(dialect).is_none());
        assert!(
            dialect
                .logical_expression_template_policy(provider, profile)
                .is_some()
        );
        for version in tcl_dialect::TclVersion::ALL {
            let actual = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(actual)
                    .logical_expression_parse_context(provider, actual)
                    .is_none()
            );
        }
        assert!(
            crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl())
                .logical_expression_parse_context(provider, profile)
                .is_none()
        );
    }

    #[test]
    fn argv_projection_preserves_prefix_before_expansion() {
        let arguments = [
            InvocationWord::Literal("known"),
            InvocationWord::Expanded,
            InvocationWord::Literal("shifted"),
        ];
        let arguments = InvocationArguments::structured(&arguments);

        assert_eq!(
            arguments.argv_at(0),
            InvocationArgument::Word(InvocationWord::Literal("known"))
        );
        assert_eq!(arguments.argv_at(1), InvocationArgument::Indeterminate);
        assert_eq!(arguments.argv_at(2), InvocationArgument::Indeterminate);
    }
}
