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

//! The execution policies an invocation is selected under: the native
//! family, release and per-axis protocols a runtime and a compile recipe read.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// C Tcl 8.6's clock ensemble has deferred script bootstrap members.
    Clock,
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
    /// Tcl's encoding became an ensemble with private workers in C Tcl 8.6.
    Encoding,
}

/// Initial ensemble geometry, independently of registered callback tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEnsembleBootstrap {
    /// Actual namespace that owns the ensemble.
    pub namespace: &'static str,
    /// Mapped targets whose commands the distribution library creates later.
    pub deferred_members: &'static [&'static [u8]],
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

    /// This dialect's expression parse context with an explicitly installed
    /// logical parser recipe. The recipe supplies only the native syntax, and
    /// only when it validates against `profile` for this dialect's family and
    /// release; every other axis stays this dialect's own.
    #[must_use]
    pub fn expression_parse_context_with(
        self,
        provider: Option<LogicalExpressionParseProvider>,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> tcl_syntax::expr::parser::ExprParseContext {
        let mut context = self.expression_parse_context(profile);
        if let Some(syntax) = provider
            .zip(profile)
            .and_then(|(provider, profile)| self.logical_expression_syntax(provider, profile))
        {
            context.native_syntax = syntax;
        }
        context
    }

    /// The native syntax an explicit logical parser recipe supplies for this
    /// dialect's family and release under `profile`, when it applies.
    #[must_use]
    pub fn logical_expression_syntax(
        self,
        provider: LogicalExpressionParseProvider,
        profile: &tcl_dialect::DialectProfile,
    ) -> Option<tcl_syntax::expr::parser::NativeExprSyntax> {
        let canonical = Self::of_profile(profile);
        (canonical.native_family == self.native_family
            && canonical.tcl_version == self.tcl_version)
            .then_some(())?;
        canonical
            .logical_expression_parse_context(provider, profile)
            .map(|context| context.native_syntax)
    }

    /// Template variable grammar supplied by an explicit logical parser recipe.
    /// Availability or a compatible release alone cannot install this policy.
    #[must_use]
    pub fn logical_expression_template_policy(
        self,
        provider: LogicalExpressionParseProvider,
        profile: &tcl_dialect::DialectProfile,
    ) -> Option<TemplateParseErrors> {
        self.logical_expression_parse_context(provider, profile)
            .map(|_| TemplateParseErrors::Rejected)
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
            (EnsembleImplementationFamily::Clock, _) => self
                .clock_bootstrap_ensemble()
                .map(|bootstrap| bootstrap.namespace),
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
            (EnsembleImplementationFamily::Encoding, Some(version))
                if version >= TclVersion::V8_6 =>
            {
                Some("::tcl::encoding")
            }
            _ => None,
        }
    }

    /// Construct counted input for an independently selected private ensemble worker.
    /// The authored implementation namespace supplies exact constructed components;
    /// the selected tail remains byte data. This supplies no lookup, command identity,
    /// native compilation admission or original object representation.
    #[must_use]
    pub fn ensemble_implementation_command_address(
        self,
        family: EnsembleImplementationFamily,
        tail: &[u8],
    ) -> Option<Vec<u8>> {
        let namespace = self.ensemble_implementation_namespace(family)?;
        let path = tcl_core_types::ByteNamespacePath::from_segments(
            tcl_syntax::naming::key_segments(namespace),
        );
        let slot =
            tcl_core_types::ByteCommandSlot::new(path, tcl_core_types::NameBytes::from(tail));
        Some(tcl_syntax::naming::constructed_command_address_bytes(&slot))
    }

    /// Authored initial map entries that do not publish a worker command.
    #[must_use]
    pub fn ensemble_bootstrap_deferred_members(
        self,
        family: EnsembleImplementationFamily,
    ) -> &'static [&'static [u8]] {
        match family {
            EnsembleImplementationFamily::Clock => self
                .clock_bootstrap_ensemble()
                .map_or(&[], |bootstrap| bootstrap.deferred_members),
            _ => &[],
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
    /// Decode a Unicode list operand with the selected receiver's list and
    /// escape rules. Native object-byte projection is a separate operation.
    pub fn read_list(
        self,
        text: &str,
    ) -> Result<Vec<std::borrow::Cow<'_, str>>, tcl_syntax::list::ListError> {
        tcl_syntax::list::split_list_with_syntax(
            text,
            self.word_values.list,
            self.lexer_grammar.escapes,
        )
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
            packages: tcl_dialect::model::PackageFloors::Borrowed(&[]),
        })
    }

    /// [`Self::of_profile`] for an interned profile, memoised per thread by
    /// the profile's address: an interned profile is immutable and never
    /// freed, so its projection cannot change.
    #[must_use]
    pub fn of_static_profile(profile: &'static tcl_dialect::DialectProfile) -> Self {
        thread_local! {
            static LAST: std::cell::Cell<Option<(usize, InvocationDialect)>> =
                const { std::cell::Cell::new(None) };
            static PROJECTED: std::cell::RefCell<Vec<(usize, InvocationDialect)>> =
                const { std::cell::RefCell::new(Vec::new()) };
        }
        let address = std::ptr::from_ref(profile).addr();
        if let Some((seen, dialect)) = LAST.get()
            && seen == address
        {
            return dialect;
        }
        let dialect = PROJECTED.with_borrow_mut(|projected| {
            if let Some((_, dialect)) = projected.iter().find(|(seen, _)| *seen == address) {
                return *dialect;
            }
            let dialect = Self::of_profile(profile);
            projected.push((address, dialect));
            dialect
        });
        LAST.set(Some((address, dialect)));
        dialect
    }

    /// Project independently owned policies from the selected profile.
    #[must_use]
    pub fn of_profile(profile: &tcl_dialect::DialectProfile) -> Self {
        Self {
            native_family: profile.runtime_core_family(),
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

    /// Compare complete invocation policies after resolving their independently
    /// selected execution points. Static C profiles may store a typed release
    /// where a retained runtime stores its equivalent canonical point. Every
    /// other policy field must still match; foreign, conflicting or unknown
    /// points are not interchangeable. This comparison supplies no runtime
    /// entry, compiler admission, original object or cache capability.
    #[must_use]
    pub fn has_same_execution_policy(mut self, mut other: Self) -> bool {
        let (Some(left), Some(right)) = (self.execution_point(), other.execution_point()) else {
            return false;
        };
        self.core_point = Some(left);
        other.core_point = Some(right);
        self == other
    }
}

/// Scope of the currently shared native template parser's rejection proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateParseErrors {
    /// C Tcl rejects malformed variable/bracket components incrementally.
    Rejected,
    /// Jim 0.84's independent template variable acceptance; other parser errors
    /// remain unresolved rather than becoming guest errors.
    Jim084,
}

impl TemplateParseErrors {
    /// Reached native template policy from the independently retained engine.
    #[must_use]
    pub fn for_dialect(dialect: crate::InvocationDialect) -> Option<Self> {
        match dialect.execution_point()?.family() {
            tcl_dialect::model::Family::Tcl => Some(Self::Rejected),
            tcl_dialect::model::Family::Jim
                if dialect.execution_point()?.release()
                    == tcl_dialect::model::Release::JIM_0_84 =>
            {
                Some(Self::Jim084)
            }
            _ => None,
        }
    }

    /// Variable grammar of this selected template, independent of written words.
    #[must_use]
    pub const fn variable_syntax(self) -> tcl_lexer::word_parts::TemplateVariableSyntax {
        match self {
            Self::Rejected => tcl_lexer::word_parts::TemplateVariableSyntax::CTcl,
            Self::Jim084 => tcl_lexer::word_parts::TemplateVariableSyntax::Jim084,
        }
    }
}

#[cfg(test)]
mod tests;
