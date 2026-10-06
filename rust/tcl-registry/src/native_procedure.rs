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

//! Native procedure definition argv and persistent static storage grammar.

use crate::InvocationArguments;

/// Actual procedure activation ordering, separate from definition grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeProcedureActivationProtocol {
    /// Real C procedure compilation and formal binding for the selected release.
    C(tcl_dialect::TclVersion),
    /// Pinned Jim original-object Script execution.
    Jim084,
}

/// The actual native procedure compilation entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeProcedureCompilationPurpose {
    /// The command invocation may replace its registered procedure declaration.
    CommandBody,
    /// A code fragment is compiled in a procedure without replacing the command.
    CodeFragment,
}

/// Declaration ownership at a reached native body recompilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeProcedureRecompileAction {
    /// Recompile the original native declaration without replacing its identity.
    RetainDeclaration,
    /// Replace a genuinely shared legacy declaration before compiling its body.
    ReplaceSharedDeclaration,
}

impl NativeProcedureActivationProtocol {
    /// Whether definition must copy a shared body into a new plain string.
    /// Unshared bodies retain their original representation in every engine.
    #[must_use]
    pub fn copies_shared_definition_body(self) -> bool {
        use crate::native_procedure_body::{
            NativeProcedureBodyAction, NativeProcedureBodyCreationProtocol,
        };
        let strings = match self {
            Self::C(version) => tcl_syntax::native_string::NativeStringProtocol::C(version),
            Self::Jim084 => tcl_syntax::native_string::NativeStringProtocol::Jim084,
        };
        NativeProcedureBodyCreationProtocol::for_string_recipe(strings).action(true)
            == NativeProcedureBodyAction::CopyCountedString
    }

    /// Whether definition converts the outer formal list before pinning its body.
    #[must_use]
    pub fn formal_list_precedes_body_capture(self) -> bool {
        matches!(self, Self::Jim084)
    }

    /// Whether an invalid invocation must leave the body unprepared.
    #[must_use]
    pub fn validates_arguments_before_body(self) -> bool {
        matches!(self, Self::Jim084)
    }

    /// Whether a valid empty body returns without installing a call frame.
    #[must_use]
    pub fn empty_body_skips_activation(self) -> bool {
        matches!(self, Self::Jim084)
    }

    /// Whether the definition owns a reusable admitted body execution unit.
    #[must_use]
    pub fn retains_prepared_body(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// Select declaration replacement from actual native roles. Transport
    /// references and cached body presence do not supply this inventory.
    #[must_use]
    pub fn recompilation_action(
        self,
        purpose: NativeProcedureCompilationPurpose,
        native_references: usize,
    ) -> Option<NativeProcedureRecompileAction> {
        use tcl_dialect::TclVersion;
        let Self::C(version) = self else {
            return None;
        };
        if native_references == 0 {
            return None;
        }
        Some(
            if matches!(version, TclVersion::V8_4 | TclVersion::V8_5)
                && purpose == NativeProcedureCompilationPurpose::CommandBody
                && native_references > 1
            {
                NativeProcedureRecompileAction::ReplaceSharedDeclaration
            } else {
                NativeProcedureRecompileAction::RetainDeclaration
            },
        )
    }

    /// Whether failed body compilation prevents formal argument validation.
    #[must_use]
    pub fn parse_failure_precedes_arguments(self) -> bool {
        matches!(self, Self::C(tcl_dialect::TclVersion::V8_4))
    }

    /// Exact C8.4 parse-failure contexts before the ordinary invocation logger.
    /// The source producer must supply a proved command and body-relative line.
    #[must_use]
    pub fn parse_failure_information(
        self,
        message: &[u8],
        command: &[u8],
        line: u32,
        invoked: &[u8],
    ) -> Option<Vec<u8>> {
        let Self::C(tcl_dialect::TclVersion::V8_4) = self else {
            return None;
        };
        if line == 0 {
            return None;
        }
        let recipe =
            tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(tcl_dialect::TclVersion::V8_4);
        let (command, command_ellipsis) = recipe.command_log_excerpt(command, command.len())?;
        let name = tcl_syntax::naming::native_procedure_compilation_name_input(
            tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4),
            invoked,
        )
        .ok()?;
        let (name_excerpt, name_boundary_ellipsis) =
            recipe.command_log_excerpt(name, name.len().min(50))?;
        let mut information = message.to_vec();
        information.extend_from_slice(b"\n    while compiling\n\"");
        information.extend_from_slice(command);
        if command_ellipsis {
            information.extend_from_slice(b"...");
        }
        information.extend_from_slice(b"\"\n    (compiling body of proc \"");
        information.extend_from_slice(name_excerpt);
        if name.len() > 50 || name_boundary_ellipsis {
            information.extend_from_slice(b"...");
        }
        information.extend_from_slice(format!("\", line {line})").as_bytes());
        Some(information)
    }
}

/// Select procedure activation only from the actual native core issuer.
#[must_use]
pub fn procedure_activation_protocol(
    dialect: crate::InvocationDialect,
) -> Option<NativeProcedureActivationProtocol> {
    use tcl_dialect::model::{Family, Release};
    match dialect.family()? {
        Family::Tcl => Some(NativeProcedureActivationProtocol::C(dialect.tcl_version?)),
        Family::Jim if dialect.core_point?.release() == Release::JIM_0_84 => {
            Some(NativeProcedureActivationProtocol::Jim084)
        }
        _ => None,
    }
}

/// How an actual procedure definition publishes its command name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcedureNamePublication {
    /// Render the fully qualified name, then resolve it as a written command name.
    RenderedNameLookup,
    /// Publish the local tail directly in the selected namespace object.
    NamespaceOwner,
}

/// Native publication protocol, independently of later command lookup.
/// C Tcl 8.4/8.5 and pinned Jim render the name before command creation;
/// C Tcl 8.6 and later retain the selected namespace object.
#[must_use]
pub fn procedure_name_publication(
    dialect: crate::InvocationDialect,
) -> Option<ProcedureNamePublication> {
    use tcl_dialect::model::{Family, Release};
    match dialect.family()? {
        Family::Tcl => Some(if dialect.tcl_version? <= tcl_dialect::TclVersion::V8_5 {
            ProcedureNamePublication::RenderedNameLookup
        } else {
            ProcedureNamePublication::NamespaceOwner
        }),
        Family::Jim if dialect.core_point?.release() == Release::JIM_0_84 => {
            Some(ProcedureNamePublication::RenderedNameLookup)
        }
        _ => None,
    }
}

/// Publish an already constructed procedure key without interpreting its
/// namespace owner as written lookup syntax on owner-preserving engines.
/// Unknown engines can retain a key only when both protocols agree.
#[must_use]
pub fn published_procedure_key(
    constructed: String,
    dialect: Option<crate::InvocationDialect>,
) -> Option<String> {
    let rendered = tcl_syntax::naming::canonical_written_command(&constructed);
    if rendered == constructed {
        return Some(constructed);
    }
    match dialect.and_then(procedure_name_publication)? {
        ProcedureNamePublication::RenderedNameLookup => Some(rendered),
        ProcedureNamePublication::NamespaceOwner => Some(constructed),
    }
}

/// Procedure body syntax accepted by the selected definition entry.
/// The rule loader's declaration syntax is independent of runtime Tcl's
/// evaluated-value grammar and grants no procedure compiler/header proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcedureDefinitionBodyPolicy {
    /// A native procedure handler accepts an evaluated script value.
    EvaluatedValue,
    /// The platform rule loader requires a closed original braced body word.
    OriginalBracedLiteral,
}

impl ProcedureDefinitionBodyPolicy {
    /// Check the original argument formation separately from definition arity.
    /// An expanded or parser-recovery operand retains explicit uncertainty.
    #[must_use]
    pub fn accepts(
        self,
        shape: crate::native_compilation::NativeCompilationWordShape,
    ) -> Option<bool> {
        use crate::native_compilation::NativeCompilationWordShape as Shape;
        match (self, shape) {
            (_, Shape::Expanded | Shape::Opaque) => None,
            (Self::OriginalBracedLiteral, Shape::BracedLiteral) | (Self::EvaluatedValue, _) => {
                Some(true)
            }
            (Self::OriginalBracedLiteral, _) => Some(false),
        }
    }
}

/// Select the actual definition entry's body syntax without assuming a native
/// compiler implementation for the platform or inferring phase from source origin.
#[must_use]
pub fn procedure_definition_body_policy(
    dialect: crate::InvocationDialect,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<ProcedureDefinitionBodyPolicy> {
    use tcl_dialect::model::{Family, InvocationRealm};
    match (dialect.family()?, realm) {
        (Family::F5Irules, InvocationRealm::RuleLoader) => {
            Some(ProcedureDefinitionBodyPolicy::OriginalBracedLiteral)
        }
        (Family::Tcl | Family::Jim | Family::F5Irules, _) => {
            Some(ProcedureDefinitionBodyPolicy::EvaluatedValue)
        }
        _ => None,
    }
}

/// Actual interpreter mutation relevant to native bytecode cache admission.
/// Lookup-reference invalidation is independent of compiler invalidation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCompilerCacheMutation {
    /// Retiring, renaming, hiding, or exposing an actual command token.
    CommandToken {
        /// The actual token's independently captured compiler-hook presence.
        hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence,
    },
    /// A namespace was created/deleted or its command lookup path changed.
    NamespaceLookup,
    /// The namespace's command path was changed (a namespace resolver epoch).
    NamespacePath,
    /// A newly published local/qualified command shadows a global counterpart.
    NamespaceCommandShadow {
        /// Actual compiler hook on the command that was shadowed.
        hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence,
    },
    /// An existing ensemble's map, subcommands, or unknown handler changed.
    EnsembleConfiguration {
        /// The ensemble's actual native compiler-hook presence.
        hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence,
    },
    /// First execution trace added, or last removed, on an actual token.
    ExecutionTrace {
        /// The traced token's independently captured compiler-hook presence.
        hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence,
    },
    /// An interpreter-wide trace changed whether commands may compile inline.
    InlineTracing,
    /// A legacy fixed math implementation/signature invalidated compiled calls.
    FixedMath,
}

/// Whether a measured native mutation invalidates cached compilation.
/// Installing a procedure's `NoOp` header after command creation is deliberately
/// not a mutation here: C Tcl does not advance `compileEpoch` for that operation.
#[must_use]
pub fn native_compiler_cache_invalidated(
    dialect: crate::InvocationDialect,
    mutation: NativeCompilerCacheMutation,
) -> Option<bool> {
    use tcl_dialect::model::Family;
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
    match dialect.family()? {
        Family::Jim => Some(false),
        Family::Tcl if dialect.tcl_version.is_some() => match mutation {
            NativeCompilerCacheMutation::NamespaceLookup => Some(false),
            NativeCompilerCacheMutation::NamespacePath
            | NativeCompilerCacheMutation::InlineTracing => Some(true),
            NativeCompilerCacheMutation::FixedMath => {
                Some(dialect.tcl_version == Some(tcl_dialect::TclVersion::V8_4))
            }
            NativeCompilerCacheMutation::CommandToken { hook }
            | NativeCompilerCacheMutation::NamespaceCommandShadow { hook }
            | NativeCompilerCacheMutation::EnsembleConfiguration { hook }
            | NativeCompilerCacheMutation::ExecutionTrace { hook } => match hook {
                Hook::Present => Some(true),
                Hook::Absent => Some(false),
                Hook::Unknown => None,
            },
        },
        _ => None,
    }
}

/// Select an independently observed procedure-header compiler. This operation
/// never confers stock command identity or executes a procedure body.
#[must_use]
pub fn select_procedure_header(
    header: tcl_dialect::NativeProcedureHeaderCompilation,
    dialect: crate::InvocationDialect,
    shapes: &[crate::native_compilation::NativeCompilationWordShape],
    context: crate::native_compilation::NativeCompilationContext,
) -> crate::native_compilation::NativeCompilationSelection {
    use crate::native_compilation::{
        NativeCompilationGuard as Guard, NativeCompilationMode as Mode,
        NativeCompilationSelection as Selection, NativeCompilationWordShape as Shape,
    };
    use tcl_dialect::{NativeProcedureHeaderCompilation as Header, TclVersion, model::Family};
    if context.mode == Mode::Direct
        || header == Header::Absent
        || dialect.family() == Some(Family::Jim)
    {
        return Selection::Generic;
    }
    if header != Header::NoOp
        || dialect.family() != Some(Family::Tcl)
        || context.mode != Mode::BytecodeObject
        || shapes.contains(&Shape::Opaque)
    {
        return Selection::Unknown;
    }
    let Some(version) = dialect.tcl_version else {
        return Selection::Unknown;
    };
    if shapes.contains(&Shape::Expanded) {
        return Selection::Generic;
    }
    Selection::Inline {
        operation: crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ProcedureNoOp),
        guard: if version == TclVersion::V8_4 {
            Guard::ChunkEntry
        } else {
            Guard::BeforeArguments
        },
    }
}

/// Select actual procedure-header compiler registration from retained
/// definition bytes and representation. C Tcl only admits bare `args` with
/// surrounding ASCII spaces and a non-precompiled whitespace body. C 8.5+
/// additionally recognises source backslash-newline whitespace.
#[must_use]
pub fn procedure_header_compilation(
    dialect: crate::InvocationDialect,
    parameters: Option<&str>,
    body: Option<&str>,
    precompiled_body: Option<bool>,
) -> tcl_dialect::NativeProcedureHeaderCompilation {
    procedure_header_compilation_bytes(
        dialect,
        parameters.map(str::as_bytes),
        body.map(str::as_bytes),
        precompiled_body,
    )
}

/// Select the actual native header from independently materialized definition
/// objects. Formal list storage remains separate: this header's parameter
/// comparison is a `CString` operation even when formal slots retain full bytes.
/// C8.4 tests a `CString` body prefix; C8.5+ examines the complete body bytes.
#[must_use]
pub fn procedure_header_compilation_bytes(
    dialect: crate::InvocationDialect,
    parameters: Option<&[u8]>,
    body: Option<&[u8]>,
    precompiled_body: Option<bool>,
) -> tcl_dialect::NativeProcedureHeaderCompilation {
    use tcl_dialect::{NativeProcedureHeaderCompilation as Header, TclVersion, model::Family};
    if dialect.family() == Some(Family::Jim) {
        return Header::Absent;
    }
    if dialect.family() != Some(Family::Tcl) {
        return Header::Unknown;
    }
    let Some(version) = dialect.tcl_version else {
        return Header::Unknown;
    };
    if precompiled_body == Some(true) {
        return Header::Absent;
    }
    let Some(parameters) = parameters else {
        return Header::Unknown;
    };
    let parameters = parameters
        .split(|byte| *byte == 0)
        .next()
        .unwrap_or_default();
    let start = parameters
        .iter()
        .position(|byte| *byte != b' ')
        .unwrap_or(parameters.len());
    let end = parameters
        .iter()
        .rposition(|byte| *byte != b' ')
        .map_or(start, |index| index + 1);
    if &parameters[start..end] != b"args" {
        return Header::Absent;
    }
    let (Some(body), Some(false)) = (body, precompiled_body) else {
        return Header::Unknown;
    };
    let bytes = if version == TclVersion::V8_4 {
        body.split(|byte| *byte == 0).next().unwrap_or_default()
    } else {
        body
    };
    let mut cursor = 0;
    while cursor < bytes.len() {
        if tcl_dialect::WordSeparators::Tcl.is_separator(bytes[cursor]) || bytes[cursor] == b'\n' {
            cursor += 1;
        } else if version >= TclVersion::V8_5 {
            let Some(end) = tcl_lexer::backslash_continuation_end(bytes, cursor) else {
                return Header::Absent;
            };
            cursor = end;
        } else {
            return Header::Absent;
        }
    }
    Header::NoOp
}

/// Authored native definition parser. Catalogue spelling cannot select it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeProcedureDefinitionSpec {
    /// C Tcl and Jim core procedure installation protocols.
    Core,
}

impl NativeProcedureDefinitionSpec {
    /// Select the grammar after the original arguments evaluated.
    #[must_use]
    pub fn select(self, arguments: InvocationArguments<'_>) -> NativeProcedureDefinitionSelection {
        select_native_procedure_definition(arguments)
    }

    /// Native definition usage without a command prefix inferred by consumers.
    #[must_use]
    pub fn usage(self, dialect: Option<crate::InvocationDialect>) -> Option<&'static str> {
        match dialect.and_then(crate::InvocationDialect::parameter_grammar) {
            Some(tcl_dialect::ParameterGrammar::Tcl) => Some("proc name args body"),
            Some(tcl_dialect::ParameterGrammar::Jim) => Some("proc name args ?statics? body"),
            None => None,
        }
    }

    /// Native operand count under an explicitly selected interpreter policy.
    #[must_use]
    pub fn arity(self, dialect: Option<crate::InvocationDialect>) -> crate::Arity {
        match dialect.and_then(crate::InvocationDialect::parameter_grammar) {
            Some(tcl_dialect::ParameterGrammar::Tcl) => crate::Arity::exact(3),
            Some(tcl_dialect::ParameterGrammar::Jim) => crate::Arity::new(3, 4),
            None => crate::Arity::any(),
        }
    }
}

/// Native result of successful command installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcedureDefinitionResult {
    /// C Tcl yields the empty string.
    Empty,
    /// Jim yields the evaluated name argument unchanged.
    NameArgument,
}

/// Positions selected after actual argv evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeProcedureDefinition {
    /// Command name operand.
    pub name_at: usize,
    /// Formal parameter list operand.
    pub parameters_at: usize,
    /// Optional persistent static-variable declaration operand.
    pub statics_at: Option<usize>,
    /// Deferred script body operand.
    pub body_at: usize,
    /// Successful definition result protocol.
    pub result: ProcedureDefinitionResult,
}

/// Native definition validity without interpreting dynamic values as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeProcedureDefinitionSelection {
    /// Valid native argv shape.
    Valid(NativeProcedureDefinition),
    /// Definition fails before creating a command.
    Invalid,
    /// Native protocol or final argv length is unresolved.
    Unknown,
}

/// Select the native operation's definition shape independently of its name.
#[must_use]
pub fn select_native_procedure_definition(
    arguments: InvocationArguments<'_>,
) -> NativeProcedureDefinitionSelection {
    use NativeProcedureDefinitionSelection as Selection;
    let Some(grammar) = arguments
        .dialect()
        .and_then(crate::InvocationDialect::parameter_grammar)
    else {
        return Selection::Unknown;
    };
    let Some(count) = arguments.exact_argv_len() else {
        return Selection::Unknown;
    };
    let (statics_at, body_at, result) = match (grammar, count) {
        (tcl_dialect::ParameterGrammar::Tcl, 3) => (None, 2, ProcedureDefinitionResult::Empty),
        (tcl_dialect::ParameterGrammar::Jim, 3) => {
            (None, 2, ProcedureDefinitionResult::NameArgument)
        }
        (tcl_dialect::ParameterGrammar::Jim, 4) => {
            (Some(2), 3, ProcedureDefinitionResult::NameArgument)
        }
        _ => return Selection::Invalid,
    };
    Selection::Valid(NativeProcedureDefinition {
        name_at: 0,
        parameters_at: 1,
        statics_at,
        body_at,
        result,
    })
}

/// Source of one independently persistent static cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticVariableInitialiser {
    /// A fresh private cell containing this literal value.
    Literal(String),
    /// A fresh private cell snapshotting the current variable's value.
    CopyCurrent(String),
    /// Retain the current physical variable cell, independently of name rebinding.
    CaptureCurrentCell(String),
}

/// Original-object initializer selected from a native Jim static specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticVariableValueInitialiser {
    /// Copy the current variable's retained value into a private cell.
    CopyCurrent,
    /// Retain the actual current cell across later name rebinding.
    CaptureCurrentCell,
    /// Retain the original second member object, without string conversion.
    LiteralValue,
}

/// A native static slot parsed independently of its initializer object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticVariableValueSpecifier {
    /// Actual counted slot name after the optional reference marker.
    pub name: Vec<u8>,
    /// Original-object initializer selected by the one/two-member grammar.
    pub initialiser: StaticVariableValueInitialiser,
}

/// Reached native static-specifier rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticVariableValueError {
    /// The original object list has neither one nor two members.
    Fields,
    /// A single-field initialization names a dictionary-sugar element.
    ArrayElement {
        /// Native reference capture rather than value copy.
        reference: bool,
    },
    /// The current static table already contains the selected slot.
    Duplicate,
    /// The current local receiver cannot supply an initializer.
    MissingCurrent,
}

impl StaticVariableValueError {
    /// Jim's actual `CString` diagnostic, separate from counted slot identity.
    #[must_use]
    pub fn message(self, original: &[u8]) -> Vec<u8> {
        let (prefix, suffix): (&[u8], &[u8]) = match self {
            Self::Fields => (b"too many fields in static specifier \"", b"\""),
            Self::ArrayElement { reference: true } => (b"Can't link to array element \"", b"\""),
            Self::ArrayElement { reference: false } => {
                (b"Can't initialise array element \"", b"\"")
            }
            Self::Duplicate => (b"static variable name \"", b"\" duplicated in statics list"),
            Self::MissingCurrent => (
                b"variable for initialization of static \"",
                b"\" not found in the local context",
            ),
        };
        let mut message = prefix.to_vec();
        message.extend_from_slice(original.split(|byte| *byte == 0).next().unwrap_or_default());
        message.extend_from_slice(suffix);
        message
    }
}

/// Select Jim's static declaration after original object list access. Literal
/// initializer objects are never decoded here. `None` means native authority
/// or the requested original name dependency is unavailable.
#[must_use]
pub fn static_variable_value_specifier(
    dialect: crate::InvocationDialect,
    field_count: usize,
    original_name: Option<&[u8]>,
) -> Option<Result<StaticVariableValueSpecifier, StaticVariableValueError>> {
    if !dialect.native_scalar_getter_protocol()?.is_jim084() {
        return None;
    }
    if !matches!(field_count, 1 | 2) {
        return Some(Err(StaticVariableValueError::Fields));
    }
    let original_name = original_name?;
    let reference = field_count == 1 && original_name.first() == Some(&b'&');
    let name = if reference {
        &original_name[1..]
    } else {
        original_name
    };
    if field_count == 1
        && tcl_syntax::naming::NativeNameProtocol::Jim084
            .combined_variable_input(name)
            .element()
            .is_some()
    {
        return Some(Err(StaticVariableValueError::ArrayElement { reference }));
    }
    let initialiser = if field_count == 2 {
        StaticVariableValueInitialiser::LiteralValue
    } else if reference {
        StaticVariableValueInitialiser::CaptureCurrentCell
    } else {
        StaticVariableValueInitialiser::CopyCurrent
    };
    Some(Ok(StaticVariableValueSpecifier {
        name: name.to_vec(),
        initialiser,
    }))
}

/// Actual C procedure-name rejection for an unavailable publication namespace.
/// The message uses a `CString` reporting operand, never the storage key.
#[must_use]
pub fn procedure_unknown_namespace_error(
    dialect: crate::InvocationDialect,
    original_name: &[u8],
) -> Option<(Vec<u8>, Option<Vec<u8>>)> {
    let version = dialect.native_scalar_getter_protocol()?.tcl_version()?;
    let mut message = b"can't create procedure \"".to_vec();
    message.extend_from_slice(
        original_name
            .split(|byte| *byte == 0)
            .next()
            .unwrap_or_default(),
    );
    message.extend_from_slice(b"\": unknown namespace");
    Some((
        message,
        (version >= tcl_dialect::TclVersion::V8_6).then(|| b"TCL VALUE COMMAND".to_vec()),
    ))
}

/// Validate the original selected holder and simple name before formal-list or
/// body access. Tcl 8.4/8.5 reject colon-prefixed simple names outside root;
/// modern Tcl and Jim permit them. Missing physical recipe remains unavailable.
#[must_use]
pub fn procedure_name_creation_error(
    dialect: crate::InvocationDialect,
    holder_is_global: bool,
    simple_name: &[u8],
) -> Option<Result<(), Vec<u8>>> {
    Some(procedure_creation_text_error(
        dialect.native_name_protocol()?,
        holder_is_global,
        simple_name,
    ))
}

/// Validate a selected procedure slot using its independently installed naming
/// policy. Authored policies establish only this textual creation rule, never
/// physical procedure headers, compiler hooks or namespace ownership.
///
/// # Errors
/// Rejects colon-prefixed local names under the selected old C naming recipe.
pub fn procedure_name_creation_error_for_policy(
    policy: tcl_syntax::naming::NamePolicyProtocol,
    holder_is_global: bool,
    simple_name: &[u8],
) -> Result<(), Vec<u8>> {
    procedure_creation_text_error(policy.recipe(), holder_is_global, simple_name)
}

fn procedure_creation_text_error(
    protocol: tcl_syntax::naming::NativeNameProtocol,
    holder_is_global: bool,
    simple_name: &[u8],
) -> Result<(), Vec<u8>> {
    if matches!(protocol, tcl_syntax::naming::NativeNameProtocol::C(version)
        if version <= tcl_dialect::TclVersion::V8_5)
        && !holder_is_global
        && simple_name.starts_with(b":")
    {
        let mut message = b"can't create procedure \"".to_vec();
        message.extend_from_slice(simple_name);
        message.extend_from_slice(b"\" in non-global namespace with name starting with \":\"");
        Err(message)
    } else {
        Ok(())
    }
}

/// One persistent declaration. Native activation resolves this static table
/// before installing formal values, so matching formals update its cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticVariableDeclaration {
    /// Name in the procedure's static storage table.
    pub name: String,
    /// Value/cell captured when the command is defined.
    pub initialiser: StaticVariableInitialiser,
}

/// Why the native static declaration cannot be installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticVariableError {
    /// A specifier has neither one nor two list fields.
    Fields(String),
    /// The post-reference-marker name repeats an earlier static slot.
    Duplicate(String),
    /// Single-field initialisation cannot capture an array element.
    ArrayElement {
        /// Name the initializer tried to resolve.
        name: String,
        /// Whether the operation attempted reference capture.
        reference: bool,
    },
}

impl StaticVariableError {
    /// Native diagnostic text for a rejected declaration.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Fields(specifier) => {
                format!("too many fields in static specifier \"{specifier}\"")
            }
            Self::Duplicate(name) => {
                format!("static variable name \"{name}\" duplicated in statics list")
            }
            Self::ArrayElement { name, reference } => format!(
                "Can't {} array element \"{name}\"",
                if *reference { "link to" } else { "initialise" }
            ),
        }
    }
}

/// Parse Jim's static declarations with its shared native list/name owners.
/// Captured values and physical identities are resolved by the caller's cell
/// owner at definition time, rather than looked up again at invocation.
pub fn parse_static_variables(
    source: &str,
) -> Result<Vec<StaticVariableDeclaration>, StaticVariableError> {
    let mut declarations = Vec::new();
    let mut names = std::collections::HashSet::new();
    for specifier in tcl_syntax::list::split_list_jim(source) {
        let fields = tcl_syntax::list::split_list_jim(&specifier);
        let (name, initialiser) = match fields.as_slice() {
            [name] => {
                let (name, reference) = name
                    .strip_prefix('&')
                    .map_or((name.as_ref(), false), |name| (name, true));
                if tcl_syntax::naming::split_element_ref(name).is_some() {
                    return Err(StaticVariableError::ArrayElement {
                        name: name.to_owned(),
                        reference,
                    });
                }
                let initialiser = if reference {
                    StaticVariableInitialiser::CaptureCurrentCell(name.to_owned())
                } else {
                    StaticVariableInitialiser::CopyCurrent(name.to_owned())
                };
                (name.to_owned(), initialiser)
            }
            [name, value] => (
                name.to_string(),
                StaticVariableInitialiser::Literal(value.to_string()),
            ),
            _ => return Err(StaticVariableError::Fields(specifier.to_string())),
        };
        if !names.insert(name.clone()) {
            return Err(StaticVariableError::Duplicate(name));
        }
        declarations.push(StaticVariableDeclaration { name, initialiser });
    }
    Ok(declarations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedure_creation_name_policy_matches_18_native_slots_without_header_grants() {
        fn bytes(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut checked = 0;
        for row in include_str!("../tests/data/authored_procedure_names/native.tsv").lines() {
            let columns: Vec<_> = row.split('\t').collect();
            let environment = crate::model::ingress::resolve_environment(&if columns[0] == "jim" {
                "jim".into()
            } else {
                format!("tcl{}", columns[0])
            });
            let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
            let native = tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                dialect.execution_point().unwrap(),
            )
            .unwrap();
            let authored = match native.recipe() {
                tcl_syntax::naming::NativeNameProtocol::C(version) => {
                    tcl_syntax::naming::NamePolicyProtocol::authored_tcl(version)
                }
                tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                    tcl_syntax::naming::NamePolicyProtocol::authored_jim084()
                }
            };
            let name = bytes(columns[3]);
            let expected = if columns[4] == "0" {
                Ok(())
            } else {
                Err(bytes(columns[5]))
            };
            for policy in [native, authored] {
                assert_eq!(
                    procedure_name_creation_error_for_policy(policy, columns[2] == "1", &name),
                    expected,
                    "{row}"
                );
            }
            assert_ne!(native.authority(), authored.authority());
            checked += 1;
        }
        assert_eq!(checked, 18);
        let mut unknown =
            crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        unknown.core_point = None;
        assert!(procedure_name_creation_error(unknown, false, b":p").is_none());
    }

    #[test]
    fn c84_parse_context_matches_twenty_six_actual_native_parser_failures() {
        let decode = |input: &str| {
            input
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        let protocol = NativeProcedureActivationProtocol::C(tcl_dialect::TclVersion::V8_4);
        let mut compared = 0;
        for row in include_str!("../tests/data/native_c84_parse_context/8.4.20.tsv").lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 9);
            if fields[1] == "0" {
                // The native parser accepted this bracket suffix; its later
                // missing-command error supplies no compilation context.
                assert_eq!(fields[0], "10");
                continue;
            }
            assert_eq!(fields[1], "1");
            let source = decode(fields[6]);
            let start = fields[2].parse().unwrap();
            let size = fields[3].parse::<usize>().unwrap();
            let term = fields[4].parse().unwrap();
            assert_eq!(start + size, source.len());
            let command = tcl_syntax::native_parse_context::c84_compilation_command_extent(
                &source, start, term,
            )
            .unwrap();
            let result = decode(fields[7]);
            let line = u32::try_from(source[..start].split(|&byte| byte == b'\n').count()).unwrap();
            let mut information = protocol
                .parse_failure_information(&result, command, line, b"p")
                .unwrap();
            information.extend_from_slice(b"\n    invoked from within\n\"p\"");
            assert_eq!(information, decode(fields[8]), "{row}");
            compared += 1;
        }
        assert_eq!(compared, 26);
        assert!(
            tcl_syntax::native_parse_context::c84_compilation_command_extent(b"x", 1, 0).is_none()
        );
        assert!(
            tcl_syntax::native_parse_context::c84_compilation_command_extent(b"x", 0, 1).is_none()
        );
    }

    #[test]
    fn activation_order_keeps_native_compiler_and_script_owners_distinct() {
        use tcl_dialect::TclVersion;
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol =
                procedure_activation_protocol(crate::InvocationDialect::for_version(version))
                    .unwrap();
            assert!(protocol.retains_prepared_body());
            assert!(protocol.copies_shared_definition_body());
            assert!(!protocol.formal_list_precedes_body_capture());
            assert!(!protocol.validates_arguments_before_body());
            assert!(!protocol.empty_body_skips_activation());
            assert_eq!(
                protocol.parse_failure_precedes_arguments(),
                version == TclVersion::V8_4
            );
        }
        let protocol = procedure_activation_protocol(crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        ))
        .unwrap();
        assert!(!protocol.retains_prepared_body());
        assert!(!protocol.copies_shared_definition_body());
        assert!(protocol.formal_list_precedes_body_capture());
        assert!(protocol.validates_arguments_before_body());
        assert!(protocol.empty_body_skips_activation());
        assert!(!protocol.parse_failure_precedes_arguments());
    }

    #[test]
    fn byte_header_selection_matches_original_native_storage() {
        use tcl_dialect::NativeProcedureHeaderCompilation as Header;
        let fixtures = [
            (
                tcl_dialect::TclVersion::V8_4,
                include_str!("../../tcl-syntax/tests/data/native_procedure_headers/8.4.20.txt"),
            ),
            (
                tcl_dialect::TclVersion::V8_5,
                include_str!("../../tcl-syntax/tests/data/native_procedure_headers/8.5.19.txt"),
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                include_str!("../../tcl-syntax/tests/data/native_procedure_headers/8.6.18.txt"),
            ),
            (
                tcl_dialect::TclVersion::V9_0,
                include_str!("../../tcl-syntax/tests/data/native_procedure_headers/9.0.4.txt"),
            ),
            (
                tcl_dialect::TclVersion::V9_1,
                include_str!("../../tcl-syntax/tests/data/native_procedure_headers/9.1.0.txt"),
            ),
        ];
        let parameters: [&[u8]; 8] = [
            b"args",
            b"args\0X",
            b" args \0X",
            b"args",
            b"args",
            b"args",
            b"args",
            b"args",
        ];
        let bodies: [&[u8]; 8] = [
            b"",
            b"",
            b"",
            b" \0X",
            b"\xff",
            b"\\\n",
            b"\x0b\x0c\r\t\n",
            b"\0",
        ];
        let mut count = 0;
        for (version, fixture) in fixtures {
            for (case, row) in fixture.lines().enumerate() {
                assert!(row.contains("code=0"), "{version:?} {case}");
                let actual = procedure_header_compilation_bytes(
                    crate::InvocationDialect::for_version(version),
                    Some(parameters[case]),
                    Some(bodies[case]),
                    Some(false),
                );
                assert_eq!(
                    actual,
                    if row.ends_with("header=1") {
                        Header::NoOp
                    } else {
                        Header::Absent
                    },
                    "{version:?} {case}"
                );
                count += 1;
            }
        }
        assert_eq!(count, 40);
    }

    #[test]
    fn procedure_publication_retains_the_native_namespace_owner_axis() {
        for version in tcl_dialect::TclVersion::ALL {
            assert_eq!(
                procedure_name_publication(crate::InvocationDialect::for_version(version)),
                Some(if version <= tcl_dialect::TclVersion::V8_5 {
                    ProcedureNamePublication::RenderedNameLookup
                } else {
                    ProcedureNamePublication::NamespaceOwner
                })
            );
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            procedure_name_publication(jim),
            Some(ProcedureNamePublication::RenderedNameLookup)
        );
        let mut unknown = jim;
        unknown.core_point = None;
        assert_eq!(procedure_name_publication(unknown), None);
    }

    #[test]
    fn compiler_cache_mutations_retain_the_native_owner_and_hook_axes() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            assert_eq!(
                native_compiler_cache_invalidated(
                    dialect,
                    NativeCompilerCacheMutation::NamespaceLookup
                ),
                Some(false)
            );
            for mutation in [
                NativeCompilerCacheMutation::NamespacePath,
                NativeCompilerCacheMutation::InlineTracing,
            ] {
                assert_eq!(
                    native_compiler_cache_invalidated(dialect, mutation),
                    Some(true)
                );
            }
            for (hook, expected) in [
                (Hook::Absent, Some(false)),
                (Hook::Present, Some(true)),
                (Hook::Unknown, None),
            ] {
                for mutation in [
                    NativeCompilerCacheMutation::CommandToken { hook },
                    NativeCompilerCacheMutation::NamespaceCommandShadow { hook },
                    NativeCompilerCacheMutation::EnsembleConfiguration { hook },
                    NativeCompilerCacheMutation::ExecutionTrace { hook },
                ] {
                    assert_eq!(
                        native_compiler_cache_invalidated(dialect, mutation),
                        expected
                    );
                }
            }
            assert_eq!(
                native_compiler_cache_invalidated(dialect, NativeCompilerCacheMutation::FixedMath),
                Some(version == tcl_dialect::TclVersion::V8_4)
            );
        }
    }

    #[test]
    fn native_noop_selection_retains_versioned_token_capture_boundary() {
        use crate::native_compilation::{
            NativeCompilationContext, NativeCompilationGuard, NativeCompilationMode,
            NativeCompilationSelection, NativeCompilationWordShape,
        };
        use tcl_dialect::NativeProcedureHeaderCompilation as Header;
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            ..Default::default()
        };
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            assert_eq!(
                select_procedure_header(
                    Header::NoOp,
                    dialect,
                    &[NativeCompilationWordShape::Substituted],
                    context
                ),
                NativeCompilationSelection::Inline {
                    operation: crate::SemanticOperationId::Intrinsic(
                        crate::IntrinsicId::ProcedureNoOp
                    ),
                    guard: if version == tcl_dialect::TclVersion::V8_4 {
                        NativeCompilationGuard::ChunkEntry
                    } else {
                        NativeCompilationGuard::BeforeArguments
                    },
                }
            );
            assert_eq!(
                select_procedure_header(
                    Header::NoOp,
                    dialect,
                    &[NativeCompilationWordShape::Expanded],
                    context
                ),
                NativeCompilationSelection::Generic
            );
            assert_eq!(
                select_procedure_header(
                    Header::NoOp,
                    dialect,
                    &[NativeCompilationWordShape::Opaque],
                    context
                ),
                NativeCompilationSelection::Unknown
            );
            assert_eq!(
                select_procedure_header(
                    Header::NoOp,
                    dialect,
                    &[],
                    NativeCompilationContext {
                        mode: NativeCompilationMode::Direct,
                        ..context
                    }
                ),
                NativeCompilationSelection::Generic
            );
        }
    }

    #[test]
    fn native_header_policy_preserves_formal_bytes_and_whitespace_versions() {
        use tcl_dialect::NativeProcedureHeaderCompilation as Header;
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            for (parameters, body, expected) in [
                ("args", "", Header::NoOp),
                (" args ", " \t\n\u{b}\u{c}\r", Header::NoOp),
                ("{args}", "", Header::Absent),
                ("\targs", "", Header::Absent),
                ("args", "# comment", Header::Absent),
                ("value", "", Header::Absent),
                (
                    "args",
                    "\\\n",
                    if version == tcl_dialect::TclVersion::V8_4 {
                        Header::Absent
                    } else {
                        Header::NoOp
                    },
                ),
            ] {
                assert_eq!(
                    procedure_header_compilation(
                        dialect,
                        Some(parameters),
                        Some(body),
                        Some(false)
                    ),
                    expected,
                    "{version:?} {parameters:?} {body:?}"
                );
            }
            assert_eq!(
                procedure_header_compilation(dialect, Some("args"), Some(""), Some(true)),
                Header::Absent
            );
            assert_eq!(
                procedure_header_compilation(dialect, Some("args"), None, None),
                Header::Unknown
            );
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            procedure_header_compilation(jim, Some("args"), Some(""), Some(false)),
            Header::Absent
        );
    }

    #[test]
    fn evaluated_argv_shape_selects_native_body_and_result() {
        let words = ["p", "a", "{x 1}", "set x"];
        let c = InvocationArguments::literals(&words).with_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            NativeProcedureDefinitionSpec::Core.select(c),
            NativeProcedureDefinitionSelection::Invalid
        );
        let profile = crate::model::ingress::resolve_environment("jim").unit_profile();
        let jim = InvocationArguments::literals(&words).with_profile(Some(profile));
        assert_eq!(
            NativeProcedureDefinitionSpec::Core.select(jim),
            NativeProcedureDefinitionSelection::Valid(NativeProcedureDefinition {
                name_at: 0,
                parameters_at: 1,
                statics_at: Some(2),
                body_at: 3,
                result: ProcedureDefinitionResult::NameArgument,
            })
        );
        assert_eq!(
            NativeProcedureDefinitionSpec::Core.select(InvocationArguments::literals(&words)),
            NativeProcedureDefinitionSelection::Unknown
        );
    }

    #[test]
    fn static_initializers_keep_value_and_physical_cell_capture_distinct() {
        assert_eq!(
            parse_static_variables("x &y {z VALUE}").unwrap(),
            vec![
                StaticVariableDeclaration {
                    name: "x".into(),
                    initialiser: StaticVariableInitialiser::CopyCurrent("x".into())
                },
                StaticVariableDeclaration {
                    name: "y".into(),
                    initialiser: StaticVariableInitialiser::CaptureCurrentCell("y".into())
                },
                StaticVariableDeclaration {
                    name: "z".into(),
                    initialiser: StaticVariableInitialiser::Literal("VALUE".into())
                },
            ]
        );
        assert_eq!(
            parse_static_variables("x &x"),
            Err(StaticVariableError::Duplicate("x".into()))
        );
        assert!(matches!(
            parse_static_variables("&a(k)"),
            Err(StaticVariableError::ArrayElement {
                reference: true,
                ..
            })
        ));
        assert_eq!(
            parse_static_variables("{&x VALUE}").unwrap()[0].initialiser,
            StaticVariableInitialiser::Literal("VALUE".into())
        );
    }
}

#[cfg(test)]
mod definition_body_policy_tests {
    use super::*;
    use crate::native_compilation::NativeCompilationWordShape as Shape;
    use tcl_dialect::model::InvocationRealm;

    #[test]
    fn rule_loader_body_formation_is_independent_of_native_compilation() {
        let f5 = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        let loader = procedure_definition_body_policy(f5, InvocationRealm::RuleLoader).unwrap();
        let runtime =
            procedure_definition_body_policy(f5, InvocationRealm::InterpreterRuntime).unwrap();
        assert_eq!(loader.accepts(Shape::BracedLiteral), Some(true));
        for shape in [Shape::Literal, Shape::QuotedLiteral] {
            assert_eq!(loader.accepts(shape), Some(false));
            assert_eq!(runtime.accepts(shape), Some(true));
        }
        assert_eq!(loader.accepts(Shape::Opaque), None);
        assert_eq!(runtime.accepts(Shape::Expanded), None);
        for version in tcl_dialect::TclVersion::ALL {
            let native = procedure_definition_body_policy(
                crate::InvocationDialect::for_version(version),
                InvocationRealm::RuleLoader,
            )
            .unwrap();
            assert_eq!(native.accepts(Shape::Literal), Some(true));
        }
    }
}

#[cfg(test)]
mod recompilation_ownership_tests {
    use super::*;
    use tcl_dialect::TclVersion;

    macro_rules! engine_cases {
        ($engine:literal) => {
            [
                include_str!(concat!(
                    "../../tcl-vm/tests/data/native_procedure_recompile/",
                    $engine,
                    "-0.txt"
                )),
                include_str!(concat!(
                    "../../tcl-vm/tests/data/native_procedure_recompile/",
                    $engine,
                    "-1.txt"
                )),
                include_str!(concat!(
                    "../../tcl-vm/tests/data/native_procedure_recompile/",
                    $engine,
                    "-2.txt"
                )),
                include_str!(concat!(
                    "../../tcl-vm/tests/data/native_procedure_recompile/",
                    $engine,
                    "-3.txt"
                )),
            ]
        };
    }

    #[test]
    fn recompilation_matches_all_20_native_declaration_identity_controls() {
        let engines = [
            (TclVersion::V8_4, engine_cases!("8.4.20")),
            (TclVersion::V8_5, engine_cases!("8.5.19")),
            (TclVersion::V8_6, engine_cases!("8.6.18")),
            (TclVersion::V9_0, engine_cases!("9.0.4")),
            (TclVersion::V9_1, engine_cases!("9.1.0")),
        ];
        let mut completed = 0;
        for (version, cases) in engines {
            let protocol =
                procedure_activation_protocol(crate::InvocationDialect::for_version(version))
                    .expect("actual C procedure issuer");
            for (case, source) in cases.into_iter().enumerate() {
                let rows: Vec<Vec<&str>> = source
                    .lines()
                    .filter(|line| line.starts_with("S|"))
                    .map(|line| line.split('|').collect())
                    .collect();
                let window = if case == 2 {
                    "active-after-invalidation"
                } else {
                    "invalidated"
                };
                let before = rows
                    .iter()
                    .find(|row| row[2] == window)
                    .expect("native recompilation frontier");
                let after = rows
                    .iter()
                    .find(|row| row[2] == "recompiled-return")
                    .expect("native completed invocation");
                let references = before[6].parse().expect("native procedure role count");
                let action = protocol
                    .recompilation_action(
                        NativeProcedureCompilationPurpose::CommandBody,
                        references,
                    )
                    .expect("actual C procedure is live");
                assert_eq!(
                    action == NativeProcedureRecompileAction::RetainDeclaration,
                    after[3] == "1",
                    "{version:?}/case {case}: {source}",
                );
                assert_eq!(
                    protocol.recompilation_action(
                        NativeProcedureCompilationPurpose::CodeFragment,
                        references
                    ),
                    Some(NativeProcedureRecompileAction::RetainDeclaration),
                    "code fragments do not replace the registered declaration",
                );
                completed += 1;
            }
        }
        assert_eq!(completed, 20);
    }

    #[test]
    fn absent_native_procedure_roles_and_jim_do_not_grant_c_recompilation() {
        for protocol in [
            NativeProcedureActivationProtocol::C(TclVersion::V8_4),
            NativeProcedureActivationProtocol::C(TclVersion::V8_5),
            NativeProcedureActivationProtocol::C(TclVersion::V8_6),
            NativeProcedureActivationProtocol::Jim084,
        ] {
            assert_eq!(
                protocol.recompilation_action(NativeProcedureCompilationPurpose::CommandBody, 0),
                None
            );
        }
        assert_eq!(
            NativeProcedureActivationProtocol::Jim084
                .recompilation_action(NativeProcedureCompilationPurpose::CommandBody, 2),
            None,
        );
    }
}
