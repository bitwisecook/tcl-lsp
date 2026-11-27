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

//! A script of statements, its command-resolution namespace, the binding sites kept beside it and the statement visitors.

use super::command_tokens::CommandTokens;
use super::statement::Statement;
use super::with_structured_ir_stack;
use tcl_lexer::Span;

// IR node types
//
// IR node kinds are modelled as a flat enum with struct variants.
// Each variant carries a `Span` for position tracking, plus
// variant-specific fields.

/// Command-resolution context for an executable script root or nested body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutionNamespace {
    /// The selected frame's defining namespace is statically known.
    Exact(String),
    /// Retained native or reached source-allocation identity and exact geometry.
    SourceContext(tcl_names::namespace::SourceNamespaceKey),
    /// A receiver- or caller-selected namespace is known only at runtime.
    RuntimeSelected,
}

impl ExecutionNamespace {
    /// Construct an exact command-resolution namespace.
    #[must_use]
    pub fn exact(namespace: impl Into<String>) -> Self {
        Self::Exact(namespace.into())
    }

    /// Select an identity-aware lookup context without parsing a display name.
    #[must_use]
    pub fn for_head_context(
        &self,
        head: &str,
    ) -> Option<std::borrow::Cow<'_, tcl_names::namespace::SourceNamespaceKey>> {
        use tcl_names::namespace::SourceNamespaceKey;
        match self {
            Self::SourceContext(context) => Some(std::borrow::Cow::Borrowed(context)),
            Self::Exact(namespace) => Some(std::borrow::Cow::Owned(SourceNamespaceKey::authored(
                if tcl_syntax::naming::is_absolute(head) {
                    "::"
                } else {
                    namespace
                },
            ))),
            Self::RuntimeSelected if tcl_syntax::naming::is_absolute(head) => {
                Some(std::borrow::Cow::Owned(SourceNamespaceKey::authored("::")))
            }
            Self::RuntimeSelected => None,
        }
    }

    /// Use the original evaluated head with the same identity-aware selector.
    #[must_use]
    pub fn for_invocation_context(
        &self,
        head: &str,
        tokens: Option<&CommandTokens>,
    ) -> Option<std::borrow::Cow<'_, tcl_names::namespace::SourceNamespaceKey>> {
        let evaluated = tokens
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(crate::command_binding::SourceInvocationBinding::evaluated_command_word);
        self.for_head_context(evaluated.unwrap_or(head))
    }

    /// Select the lookup route from the captured evaluated head. A lowered
    /// implementation name does not replace an absolute original invocation.
    #[must_use]
    pub fn for_invocation(&self, head: &str, tokens: Option<&CommandTokens>) -> Option<&str> {
        let evaluated = tokens
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(crate::command_binding::SourceInvocationBinding::evaluated_command_word);
        self.for_head(evaluated.unwrap_or(head))
    }

    /// Resolve one command head. Absolute names remain exact even in a
    /// runtime-selected frame; relative names do not.
    #[must_use]
    pub fn for_head(&self, head: &str) -> Option<&str> {
        if matches!(self, Self::SourceContext(_)) {
            return None;
        }
        if tcl_syntax::naming::is_absolute(head) {
            Some("::")
        } else {
            match self {
                Self::Exact(namespace) => Some(namespace),
                Self::SourceContext(_) | Self::RuntimeSelected => None,
            }
        }
    }

    pub(crate) fn selected_frame(&self, absolute: bool, frame_shift: i32) -> Self {
        match (absolute, frame_shift) {
            (true, 0) => Self::exact("::"),
            (false, 0) => self.clone(),
            _ => Self::RuntimeSelected,
        }
    }
}

/// A script: a sequence of IR statements.
///
/// Using `Vec` rather than a tuple because scripts are built incrementally
/// during lowering.
#[derive(Debug, Eq, Default)]
pub struct Script {
    /// Actual namespace owner for this retained executable body.
    pub namespace_context: Option<Box<tcl_names::namespace::SourceNamespaceKey>>,
    /// Exact evaluated source instance. Absence means no source mapping proof.
    pub executed_source: Option<std::sync::Arc<tcl_names::origin::ExecutedScriptSource>>,
    /// Original function occurrences, with reached proofs or conditional
    /// topology retaining an unmet actual binding-validation obligation.
    pub implicit_math_invocations: Vec<crate::command_binding::SourceMathInvocation>,
    /// Successful native expression preparation, before reached operand effects.
    pub expression_preparations: Vec<crate::command_binding::SourceExpressionPreparation>,
    /// Statements in execution order.
    pub statements: Vec<Statement>,
    /// Registry-resolved command heads consumed by typed lowering.
    ///
    /// Keeping these sites beside the script makes the dependency explicit in
    /// IR and lets CFG/codegen preserve it without re-parsing source fragments.
    pub command_binding_sites: CommandBindingSites,
    /// Exact user-procedure definitions whose bodies were copied into this
    /// script by an executable inlining transform.
    pub procedure_binding_requirements: ProcedureBindingRequirements,
    /// Rejection proved at this chunk's actual native compilation entry.
    /// An inventory of possible failures cannot populate this field.
    pub native_compilation_failure:
        Option<Box<crate::command_binding::SourceNativeCompilationFailure>>,
    /// Target-neutral chunk admission, preserving unresolved compiler paths.
    pub native_compilation_admission:
        Option<std::sync::Arc<crate::native_compilation_admission::NativeCompilationAdmission>>,
}

// A derived `Clone` or `PartialEq` of structured IR recurses several native
// frames per nesting level (`Script` -> `Vec<Statement>` -> `Statement` ->
// `IfClause` -> `Script`), which overflowed a 2 MiB thread at the
// structured-lowering depth cap. Each nested `Script` therefore continues on
// a fresh stack segment once a bounded number of levels share one stack.
impl Clone for Script {
    fn clone(&self) -> Self {
        with_structured_ir_stack(|| Self {
            namespace_context: self.namespace_context.clone(),
            executed_source: self.executed_source.clone(),
            implicit_math_invocations: self.implicit_math_invocations.clone(),
            expression_preparations: self.expression_preparations.clone(),
            statements: self.statements.clone(),
            command_binding_sites: self.command_binding_sites.clone(),
            procedure_binding_requirements: self.procedure_binding_requirements.clone(),
            native_compilation_failure: self.native_compilation_failure.clone(),
            native_compilation_admission: self.native_compilation_admission.clone(),
        })
    }
}

impl PartialEq for Script {
    fn eq(&self, other: &Self) -> bool {
        with_structured_ir_stack(|| {
            self.namespace_context == other.namespace_context
                && self.executed_source == other.executed_source
                && self.implicit_math_invocations == other.implicit_math_invocations
                && self.expression_preparations == other.expression_preparations
                && self.statements == other.statements
                && self.command_binding_sites == other.command_binding_sites
                && self.procedure_binding_requirements == other.procedure_binding_requirements
                && self.native_compilation_failure == other.native_compilation_failure
                && self.native_compilation_admission == other.native_compilation_admission
        })
    }
}

// Hashes exactly the fields `PartialEq` compares. A `Hasher` need not be
// `Send`, so this traversal stays on the caller's stack, as the derived
// implementation did.
impl std::hash::Hash for Script {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.namespace_context.hash(state);
        self.executed_source.hash(state);
        self.implicit_math_invocations.hash(state);
        self.expression_preparations.hash(state);
        self.statements.hash(state);
        self.command_binding_sites.hash(state);
        self.procedure_binding_requirements.hash(state);
        self.native_compilation_failure.hash(state);
        self.native_compilation_admission.hash(state);
    }
}

/// Compact storage for the uncommon typed-lowering dependency sidecar.
///
/// Most scripts contain no command whose head disappears during structured
/// lowering. A boxed slice omits `Vec`'s unused capacity word and avoids
/// inflating every recursively nested [`Script`] (and the [`Statement`]
/// variants which contain several of them).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CommandBindingSites(Box<[CommandBindingSite]>);

impl CommandBindingSites {
    /// Iterate over the retained typed-lowering dependencies.
    pub fn iter(&self) -> impl Iterator<Item = &CommandBindingSite> {
        self.0.iter()
    }

    /// Mutably iterate over the retained typed-lowering dependencies.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut CommandBindingSite> {
        self.0.iter_mut()
    }

    /// Unanimous original carrier of one actual statement. A synthetic marker
    /// owns its explicit phase carrier; consumed runtime-command sites at the
    /// same wrapper span cannot donate or contradict that different phase.
    #[doc(hidden)]
    pub fn unanimous_statement_source_tokens<'a>(
        sites: impl IntoIterator<Item = &'a CommandBindingSite>,
        statement: &'a Statement,
    ) -> Option<&'a CommandTokens> {
        if let Some(tokens) = statement.tokens()
            && tokens.synthetic.is_some()
        {
            return Some(tokens);
        }
        let mut candidates = statement.tokens().into_iter().chain(
            sites
                .into_iter()
                .filter(|site| site.span == statement.span())
                .filter_map(|site| site.source_tokens.as_deref()),
        );
        let first = candidates.next()?;
        candidates
            .all(|candidate| candidate == first)
            .then_some(first)
    }
}

impl CommandBindingSite {
    /// Relocate physical variable proof while retaining the consumed dispatch and lexical site.
    pub fn relocate_variable_proofs(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) {
        if let Some(context) = &self.variable_context {
            self.variable_context = Some(std::sync::Arc::new(context.relocated(relocation)));
        }
        if let Some(frame) = &self.variable_frame {
            self.variable_frame = Some(relocation.frame(frame));
        }
        if let Some(tokens) = &mut self.source_tokens {
            tokens.relocate_variable_proofs(relocation);
        }
    }
}

/// Compact storage for user-procedure dependencies introduced by inlining.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ProcedureBindingRequirements(Box<[tcl_runtime_api::ProcedureBindingIdentity]>);

impl ProcedureBindingRequirements {
    /// Iterate over the exact user-procedure dependencies.
    pub fn iter(&self) -> impl Iterator<Item = &tcl_runtime_api::ProcedureBindingIdentity> {
        self.0.iter()
    }
}

impl From<Vec<tcl_runtime_api::ProcedureBindingIdentity>> for ProcedureBindingRequirements {
    fn from(requirements: Vec<tcl_runtime_api::ProcedureBindingIdentity>) -> Self {
        Self(requirements.into_boxed_slice())
    }
}

impl From<Vec<CommandBindingSite>> for CommandBindingSites {
    fn from(sites: Vec<CommandBindingSite>) -> Self {
        Self(sites.into_boxed_slice())
    }
}

/// One command dependency retained after typed lowering consumes its head.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandBindingSite {
    /// Exact source span of the complete owning Tcl command.
    pub span: Span,
    /// Source spelling and registry-resolved implementation identity.
    pub binding: tcl_runtime_api::CommandBindingIdentity,
    /// Namespace objects proved to exist at this exact dispatch site. `None`
    /// denotes a compatibility node with no source-state query.
    pub known_namespaces: Option<std::sync::Arc<[String]>>,
    /// Actual variable activation proved at this dispatch site.
    pub variable_frame: Option<tcl_names::variable::VariableExecutionFrame>,
    /// Complete shared variable binding snapshot, including aliases and generations.
    pub variable_context: Option<std::sync::Arc<crate::var_resolve::ResolveContext>>,
    /// Namespace cells allocated on every reaching path at this site.
    pub existing_namespace_cells: Option<Vec<tcl_names::variable::VariableCellKey>>,
    /// Original source words and exact nested/read proofs consumed by typed IR.
    /// Generated dependencies have no source carrier and retain `None`.
    pub source_tokens: Option<Box<CommandTokens>>,
    /// The invocation's words after the command, as far as a release's
    /// choice to compile the command depends on them.
    pub shape: tcl_registry::release_compilation::InvocationShape,
}

/// Whether a command's normal result contributes to its enclosing script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementResultUse {
    /// A later real command discards the preceding normal result.
    Discarded,
    /// The final command supplies the enclosing completion's result.
    EnclosingCompletion,
    /// The requested command or its source ownership is unavailable.
    Unknown,
}

impl Script {
    /// Prefer a retained namespace owner over a presentation-derived default.
    #[must_use]
    pub fn execution_namespace(&self, fallback: ExecutionNamespace) -> ExecutionNamespace {
        self.namespace_context.as_deref().map_or(fallback, |key| {
            ExecutionNamespace::SourceContext(key.clone())
        })
    }

    /// Original invocation carrier for a statement owned by this script.
    /// Typed lowering can move that carrier into a consumed binding site.
    /// Conflicting retained carriers or synthetic boundaries provide no answer.
    #[must_use]
    pub fn retained_source_tokens_for_statement<'a>(
        &'a self,
        statement: &'a Statement,
    ) -> Option<&'a CommandTokens> {
        if !self
            .statements
            .iter()
            .any(|candidate| std::ptr::eq(candidate, statement))
        {
            return None;
        }
        CommandBindingSites::unanimous_statement_source_tokens(
            self.command_binding_sites.iter(),
            statement,
        )
        .filter(|tokens| tokens.synthetic.is_none())
    }

    /// Classify normal result use without confusing synthetic phase boundaries
    /// with real Tcl commands. Abrupt completion and effects remain separate.
    #[must_use]
    pub fn statement_result_use(&self, index: usize) -> StatementResultUse {
        let Some(statement) = self.statements.get(index) else {
            return StatementResultUse::Unknown;
        };
        if !self.is_authored_source() || statement.source_edit_span().is_none() {
            return StatementResultUse::Unknown;
        }
        if self.statements[index + 1..].iter().any(|statement| {
            statement.source_edit_span().is_some()
                && statement
                    .tokens()
                    .is_none_or(|tokens| tokens.synthetic.is_none())
        }) {
            StatementResultUse::Discarded
        } else {
            StatementResultUse::EnclosingCompletion
        }
    }

    /// Whether every byte belongs to an original authored source instance.
    #[must_use]
    pub fn is_authored_source(&self) -> bool {
        self.executed_source.as_ref().is_some_and(|source| {
            matches!(
                source.origin.kind(),
                tcl_names::origin::SourceOriginKind::Authored(_)
            )
        })
    }

    /// Whether this script and every retained child have authored mappings.
    /// Whole structured rewrites cannot insert a derived body's bytes by offset.
    #[must_use]
    pub fn is_fully_authored_source(&self) -> bool {
        let mut pending = vec![self];
        while let Some(script) = pending.pop() {
            if !script.is_authored_source() {
                return false;
            }
            for statement in &script.statements {
                pending.extend(statement.child_scripts());
            }
        }
        true
    }

    /// Project an edit only when this script has an authored source mapping.
    #[must_use]
    pub fn source_edit_span(&self, span: Span) -> Option<Span> {
        self.is_authored_source().then_some(span)
    }

    /// Create an empty script.
    #[must_use]
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
            command_binding_sites: CommandBindingSites::default(),
            procedure_binding_requirements: ProcedureBindingRequirements::default(),
            native_compilation_failure: None,
            native_compilation_admission: None,
            namespace_context: None,
            executed_source: None,
            implicit_math_invocations: Vec::new(),
            expression_preparations: Vec::new(),
        }
    }

    /// Create a script from a list of statements.
    #[must_use]
    pub fn from_statements(statements: Vec<Statement>) -> Self {
        Self {
            statements,
            command_binding_sites: CommandBindingSites::default(),
            procedure_binding_requirements: ProcedureBindingRequirements::default(),
            native_compilation_failure: None,
            native_compilation_admission: None,
            namespace_context: None,
            executed_source: None,
            implicit_math_invocations: Vec::new(),
            expression_preparations: Vec::new(),
        }
    }

    /// Create a lowered script with explicit typed-lowering dependencies.
    #[must_use]
    pub fn from_lowered_parts(
        statements: Vec<Statement>,
        command_binding_sites: Vec<CommandBindingSite>,
    ) -> Self {
        Self {
            statements,
            command_binding_sites: command_binding_sites.into(),
            procedure_binding_requirements: ProcedureBindingRequirements::default(),
            native_compilation_failure: None,
            native_compilation_admission: None,
            namespace_context: None,
            executed_source: None,
            implicit_math_invocations: Vec::new(),
            expression_preparations: Vec::new(),
        }
    }

    /// Create transformed executable IR with both registry and user-procedure
    /// binding dependencies retained explicitly.
    #[must_use]
    pub fn from_transformed_parts(
        statements: Vec<Statement>,
        command_binding_sites: Vec<CommandBindingSite>,
        procedure_binding_requirements: Vec<tcl_runtime_api::ProcedureBindingIdentity>,
    ) -> Self {
        Self {
            statements,
            command_binding_sites: command_binding_sites.into(),
            procedure_binding_requirements: procedure_binding_requirements.into(),
            native_compilation_failure: None,
            native_compilation_admission: None,
            namespace_context: None,
            executed_source: None,
            implicit_math_invocations: Vec::new(),
            expression_preparations: Vec::new(),
        }
    }

    /// Whether every statement compiles to a single fall-through block — no
    /// branch, join, loop or unwinding `return`. That is the shape the inline
    /// collecting-`lmap` codegen needs: it strips the body's trailing `POP` and
    /// appends the result via one `LMAP_COLLECT` on the fall-through tail, so a
    /// branch or join (an `if`, `while`, `switch` or nested loop) or a `return`
    /// that unwinds past the collect point would drop or mis-gather results.
    /// Lowering keeps such a body's `lmap` on the runtime builtin, and a pass
    /// that rewrites a body afterwards must not give it one.
    #[must_use]
    pub fn is_straight_line(&self) -> bool {
        self.statements.iter().all(|statement| {
            matches!(
                statement,
                Statement::Call { .. }
                    | Statement::AssignConst { .. }
                    | Statement::AssignExpr { .. }
                    | Statement::AssignValue { .. }
                    | Statement::Incr { .. }
                    | Statement::ExprEval { .. }
                    | Statement::Barrier { .. }
            )
        })
    }
}

/// Depth cap for [`for_each_statement`]'s recursion over nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/`uplevel` bodies.
/// Transitively bounded via `MAX_LOWER_NEST_DEPTH` (every
/// `Script` this crate hands to `for_each_statement` was built by
/// [`crate::lowering`], which already caps its own construction at 256 and
/// emits a `Statement::Barrier` past that point), capped here independently
/// for defence-in-depth and consistency with every other full-tree walker in
/// this crate (`optimiser::MAX_OPTIMISER_WALK_DEPTH`,
/// `codegen::structured::MAX_STRUCTURED_DEPTH`), since this is a `pub`
/// utility any future caller could hand a `Script` built some other way.
const MAX_FOR_EACH_STATEMENT_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Recursively visit every statement in `script`, including nested bodies
/// (`if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`), in source order.
/// Each statement is visited before its own nested bodies (pre-order): a
/// compound statement (`If`, `For`, …) is passed to `visit` once for itself,
/// then its nested scripts are walked in turn.
///
/// Shared traversal for whole-module / whole-body scans that need "every
/// statement anywhere in this script" without each re-deriving the same
/// nested-body match arms (e.g. [`crate::var_observability::scan_module_global_names`]).
///
/// Nesting past [`MAX_FOR_EACH_STATEMENT_DEPTH`] (256) is silently not
/// descended into — `visit` still runs for every statement up to and
/// including that depth, matching this function's existing "best-effort
/// scan" contract (callers already tolerate this visitor skipping statements
/// it cannot reach, e.g. inside opaque `Statement::Barrier`s).
pub fn for_each_statement(script: &Script, visit: &mut impl FnMut(&Statement)) {
    for_each_statement_inner(script, visit, 0);
}

/// Visit every retained script, including nested bodies, without native-stack
/// recursion. The visitor may update metadata; it must retain the body tree.
pub fn for_each_script_mut(script: &mut Script, visit: &mut impl FnMut(&mut Script)) {
    let mut pending = vec![script];
    while let Some(script) = pending.pop() {
        visit(script);
        for statement in script.statements.iter_mut().rev() {
            pending.extend(statement.child_scripts_mut().into_iter().rev());
        }
    }
}

fn for_each_statement_inner(script: &Script, visit: &mut impl FnMut(&Statement), depth: u32) {
    if MAX_FOR_EACH_STATEMENT_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        visit(stmt);
        for body in stmt.child_scripts() {
            for_each_statement_inner(body, visit, depth + 1);
        }
    }
}
