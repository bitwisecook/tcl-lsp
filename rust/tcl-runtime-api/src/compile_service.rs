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

//! The runtime compile service: compile targets, how a release evaluates and binds commands, script command plans and `CompileService`.

use super::native_compilation::NativeCompilationEntry;
use super::native_compilation_error::NativeCompilationAdmissionError;
use super::native_procedure_compilation::OriginalProcedureCompileRequest;
use super::native_substitution;
use tcl_lexer::SourceImage;

// -- Compile service (the EVAL_STK / dynamic-code injection point) --

/// A compilation failure surfaced by [`CompileService`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// Ordinary source validation diagnostic from the compile service.
    Message(String),
    /// The compile service cannot honour the requested execution contract.
    /// This operational refusal cannot become a Tcl completion.
    Unsupported(String),
    /// The native compiler provider has not admitted the compilation boundary.
    /// This is a host refusal and cannot become a Tcl completion.
    NativeCompilationAdmission(NativeCompilationAdmissionError),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) | Self::Unsupported(message) => formatter.write_str(message),
            Self::NativeCompilationAdmission(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for CompileError {}

/// The source-level context required to compile a Tcl procedure body.
///
/// Procedure bodies are not scripts: unqualified variables live in a local
/// variable table seeded by the formal parameter names, `return` terminates a
/// procedure activation, and command resolution starts in the procedure's
/// namespace. Keeping this target typed prevents a runtime compiler from
/// accidentally compiling a body through the top-level script entry point.
#[derive(Debug, Clone, Copy)]
pub struct ProcedureCompileTarget<'a> {
    /// Body source, with offsets relative to the body itself.
    pub source: &'a str,
    /// Formal parameter names in declaration order (defaults are a runtime
    /// binding concern and do not affect bytecode generation).
    pub parameters: &'a [String],
    /// Canonical unrooted constructed namespace key in which the procedure
    /// body resolves commands. The empty string denotes the global namespace;
    /// this is an identity, not a written Tcl name, so a literal `:` segment is
    /// retained verbatim.
    pub namespace: &'a str,
}

/// Which source bodies a compilation request prepares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SourceCompilationScope {
    /// Authored analysis and AOT artifacts include deferred declaration bodies.
    #[default]
    WholeModule,
    /// Prepare the entered source; declaration bodies remain original values
    /// until their own activation requests compilation.
    EnteredSource,
}

/// A runtime script together with the namespace in which its command words
/// resolve.
///
/// Dynamic `eval`/command-substitution bodies are script frames, not procedure
/// frames, but their compiler provenance still needs the exact constructed
/// namespace. Keeping this target typed prevents a compiler from silently
/// stamping global binding assumptions onto a namespaced evaluation.
#[derive(Debug, Clone, Copy)]
pub struct ScriptCompileTarget<'a> {
    /// Script source, with offsets relative to this string.
    pub source: &'a str,
    /// Canonical unrooted constructed namespace key. Empty denotes global.
    pub namespace: &'a str,
    /// How the emulated release evaluates the script's own commands.
    pub evaluation: CommandEvaluation,
}

/// How the emulated release evaluates a command.
///
/// `tclsh` compiles every body a command evaluates, every script `source`
/// reads and every command its stdin feeds it, but evaluates the commands of
/// the script file it is given as `Tcl_EvalEx` does, one at a time without
/// compiling them. A release whose compiler reads a literal word apart from
/// its command (8.6's `TclIndexEncode`) reads such a command's words as the
/// command does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CommandEvaluation {
    /// Compiled.
    #[default]
    Compiled,
    /// Parsed and invoked without compiling it.
    Direct,
}

/// Whether the emulated release binds a command when it compiles the
/// enclosing script (its `compileProc` emits the command inline) or looks it
/// up when the command runs. Observable only in a release whose running
/// bytecode keeps the commands it compiled
/// (`TclVersion::revalidates_running_bytecode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReleaseBinding {
    /// Looked up each time the command runs.
    Invoked,
    /// Compiled inline wherever it appears.
    Compiled,
    /// Compiled inline only in a procedure body, whose local variable table
    /// its `compileProc` needs; looked up elsewhere.
    CompiledInProcedure,
    /// Compiled inline only in a procedure body and outside the script body
    /// of a `catch` compiled into it (8.4's `return`).
    CompiledInProcedureOutsideCatch,
}

/// What a release's compile of a unit can see where a command stands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct UnitKind {
    /// The unit is a procedure body.
    pub in_procedure: bool,
    /// The command stands in the script body of a `catch` the release
    /// compiled into the unit.
    pub in_catch_body: bool,
}

impl ReleaseBinding {
    /// Whether the command is bound at compile time where a unit of `kind`
    /// holds it.
    #[must_use]
    pub fn binds(self, kind: UnitKind) -> bool {
        match self {
            Self::Invoked => false,
            Self::Compiled => true,
            Self::CompiledInProcedure => kind.in_procedure,
            Self::CompiledInProcedureOutsideCatch => kind.in_procedure && !kind.in_catch_body,
        }
    }

    /// The binding that holds where both `self` and `other` hold. Each
    /// binding holds wherever the next one in `Invoked`,
    /// `CompiledInProcedureOutsideCatch`, `CompiledInProcedure`, `Compiled`
    /// does.
    #[must_use]
    pub fn meet(self, other: Self) -> Self {
        let rank = |binding: Self| match binding {
            Self::Invoked => 0,
            Self::CompiledInProcedureOutsideCatch => 1,
            Self::CompiledInProcedure => 2,
            Self::Compiled => 3,
        };
        if rank(self) <= rank(other) {
            self
        } else {
            other
        }
    }
}

/// A native script in its exact constructed byte namespace.
///
/// The namespace contains already selected segments. No display spelling is
/// reparsed, and the source image retains original bytes and channel policy.
#[derive(Debug, Clone, Copy)]
pub struct ScriptCompileTargetBytes<'a> {
    /// Immutable original script source, relative to its first byte.
    pub source: &'a SourceImage,
    /// Constructed command-resolution namespace; an empty path is root.
    pub namespace: &'a tcl_core_types::ByteNamespacePath,
}

/// A native procedure body with the actual bound formal storage keys.
#[derive(Debug, Clone, Copy)]
pub struct ProcedureCompileTargetBytes<'a> {
    /// Immutable original body bytes, without a Unicode or escaped surrogate.
    pub source: &'a SourceImage,
    /// Exact local storage keys in declaration order; defaults stay runtime values.
    pub parameters: &'a [tcl_core_types::NameBytes],
    /// Constructed command-resolution namespace, never recovered from display.
    pub namespace: &'a tcl_core_types::ByteNamespacePath,
}

/// Command-dispatch form requested for a procedure-body compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcedureDispatch {
    /// Registry-specialised bytecode guarded by its command-binding summary.
    Optimised,
    /// Ordinary runtime dispatch for every command.
    Plain,
}

/// Compiler-owned command-at-a-time parse plan for a runtime script.
///
/// Tcl parses scripts one command at a time.  A malformed command therefore
/// does not prevent earlier complete commands from running; after that prefix
/// completes normally, the malformed tail raises its parse error without
/// performing substitutions.  The compiler owns the lexer grammar needed to
/// find that boundary, while runtimes own execution, so this small value is
/// the seam between them.
#[derive(Debug, Clone)]
pub struct ScriptCommandPlan {
    /// Byte length of the complete-command prefix in the original source.
    /// Native byte source requires no Unicode character boundary.
    pub complete_prefix_len: usize,
    /// How many complete commands that prefix holds.
    ///
    /// Not derivable from `complete_prefix_len`: the prefix of a script whose
    /// *first* command is malformed still spans any leading whitespace and
    /// comments, so a nonzero length can carry **no** command at all.  A
    /// runtime distinguishing "ran nothing" from "ran something" must test
    /// this rather than the byte length.
    pub complete_prefix_commands: usize,
    /// Parse error raised if the complete prefix finishes normally.
    pub fatal_tail: Option<FatalTail>,
}

/// The parse error a [`ScriptCommandPlan`]'s malformed tail raises, with the
/// context C logs the `while executing` frame from.
///
/// The message alone is not enough: native error contexts retain the original
/// command extent and its line. Compilation and runtime reporting can select
/// different extents from the same parser geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FatalTail {
    /// The Tcl parse message (`missing "`, `missing close-brace`, …).
    pub message: String,
    /// Original source extent selected by C's runtime parse-error logger.
    ///
    /// C slices `source[commandStart ..= parsePtr->term]` — through the
    /// character that opened the unterminated construct, **not** to the end
    /// of the source. Truncation to 150 bytes is the logger's job, not this
    /// value's.
    pub command_text: Vec<u8>,
    /// Independently proved C8.4 compilation context. Its full remaining-source
    /// extent differs from the runtime logger's inclusive-terminator extent.
    /// Absence grants no compiler presentation; an empty proved extent is valid.
    pub compilation_command_text: Option<Vec<u8>>,
    /// One-based line of the malformed command's first byte, for the
    /// enclosing `(procedure …)` / `("eval" body line N)` frames.
    pub line: u32,
    /// The `while compiling` frames of a script the release refused to
    /// compile whole ([`CompileFailure`]), which run nothing at all and
    /// replace the `while executing` frame `command_text` would log.
    pub compile_info: Option<String>,
    /// The `-errorcode` the error carries, where it carries one.
    pub error_code: Option<String>,
}

/// What a release that compiles a script whole compiles at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompileUnit {
    /// A procedure body, compiled when the procedure is first called.
    ProcedureBody,
    /// A script a command evaluates, or a chunk of commands `tclsh` reads
    /// from its stdin, compiled whole.
    Script,
}

/// Why a release that compiles a script whole refused it: the error, the
/// `-errorinfo` frames after the message (a `while compiling` frame per
/// script the failure is nested in, and the context line each compile
/// procedure adds), and the line of the failing command in the script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileFailure {
    /// The error message.
    pub message: String,
    /// The `-errorinfo` text that follows the message.
    pub error_info: String,
    /// One-based line of the outermost failing command (C's
    /// `iPtr->errorLine`).
    pub line: u32,
    /// The `-errorcode` the error carries, where it carries one (a numeral
    /// 8.4's expression parser cannot read).
    pub error_code: Option<String>,
}

impl FatalTail {
    /// A tail carrying only its message, for a caller with no source context.
    ///
    /// No source extent or compilation context is proved by this constructor.
    /// This differs from a proved, genuinely empty compilation extent.
    #[must_use]
    pub fn message_only(message: String) -> Self {
        Self {
            message,
            command_text: Vec::new(),
            compilation_command_text: None,
            line: 0,
            compile_info: None,
            error_code: None,
        }
    }

    /// The tail a refused compile raises before anything runs.
    #[must_use]
    pub fn compile_failure(failure: CompileFailure) -> Self {
        Self {
            message: failure.message,
            command_text: Vec::new(),
            compilation_command_text: None,
            line: failure.line,
            compile_info: Some(failure.error_info),
            error_code: failure.error_code,
        }
    }
}

impl ScriptCommandPlan {
    /// A clean script whose whole source is executable.
    ///
    /// `commands` is left unset (`usize::MAX` would be a lie and zero would
    /// claim nothing runs), so this constructor takes it explicitly where the
    /// count is known; [`Self::complete`] is for the no-cut case, where the
    /// distinction the count exists for cannot arise.
    #[must_use]
    pub fn complete(source_len: usize) -> Self {
        Self {
            complete_prefix_len: source_len,
            // No cut, so every command in the source is in the prefix. The
            // exact count is not needed: callers consult it only to tell an
            // empty prefix from a non-empty one before a fatal tail, and
            // there is no fatal tail here.
            complete_prefix_commands: usize::from(source_len > 0),
            fatal_tail: None,
        }
    }
}

/// Compiles a Tcl source string to a runtime-executable module at runtime.
///
/// `eval`/`uplevel`/dynamic command names compile a string while the program
/// runs, so a VM that supports them needs a compiler available during
/// execution. Injecting it as a trait keeps the VM crate lean and
/// compiler-optional: the embedder wires a real (`tcl-compiler`-backed)
/// implementation; a program that never hits `eval` can use a stub. Mirrors C
/// Tcl always carrying its bytecode compiler.
///
/// The produced module is an associated type (e.g. the VM sets it to
/// `tcl_bytecode::ModuleAsm`) so this contract crate stays free of any concrete
/// bytecode dependency — see the crate-level note.
pub trait CompileService {
    /// The runtime-executable artifact produced (the bytecode VM's `ModuleAsm`).
    type Module;

    /// Compile an original native substitution template with its own cache and
    /// completion purpose. It is not a script containing a synthetic command.
    /// Services without the genuine template emitter refuse explicitly.
    fn compile_substitution_with_entry(
        &self,
        target: native_substitution::NativeSubstitutionTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, entry);
        Err(CompileError::Unsupported(
            "CompileService does not support original substitution templates".into(),
        ))
    }

    /// Compile original native script bytes in their constructed namespace.
    /// A service must implement byte ingress explicitly; decoding or escaping
    /// opaque source would change names and literal objects.
    fn compile_script_bytes_for_profile(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile);
        Err(CompileError::Unsupported(
            "CompileService does not support original byte scripts".into(),
        ))
    }

    /// Compile byte scripts with ordinary dispatch for every command.
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile);
        Err(CompileError::Unsupported(
            "CompileService does not support plain byte scripts".into(),
        ))
    }

    /// Compile byte scripts against the retained actual interpreter entry.
    /// The entry's physical compiler and logical policy remain independent.
    fn compile_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, entry);
        Err(CompileError::Unsupported(
            "CompileService does not support retained-entry byte scripts".into(),
        ))
    }

    /// Compile byte scripts with ordinary dispatch under the exact native entry.
    /// Services must preserve actual compiler prerequisites even when semantic
    /// command specializations are disabled.
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, entry);
        Err(CompileError::Unsupported(
            "CompileService does not support retained-entry plain byte scripts".into(),
        ))
    }

    /// Compile byte procedure bodies using actual formal storage identities.
    fn compile_procedure_bytes_for_profile(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, dispatch);
        Err(CompileError::Unsupported(
            "CompileService does not support byte procedure bodies".into(),
        ))
    }

    /// Compile a byte procedure against the retained actual interpreter entry.
    fn compile_procedure_bytes_with_entry(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, entry, dispatch);
        Err(CompileError::Unsupported(
            "CompileService does not support retained-entry byte procedures".into(),
        ))
    }

    /// Compile an actual retained declaration's original body. The request owns
    /// the exact prefix, formal keys and namespace; a separate target cannot be
    /// substituted. Existing services retain their ordinary procedure contract.
    fn compile_original_procedure_bytes_with_entry(
        &self,
        request: &OriginalProcedureCompileRequest,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        if !request.matches_entry(entry) {
            return Err(CompileError::Unsupported(
                "original procedure declaration is unavailable at this entry".into(),
            ));
        }
        self.compile_procedure_bytes_with_entry(request.target(), profile, entry, dispatch)
    }

    /// Locate a byte script's complete-command prefix before executing it.
    /// A service with no native byte parser refuses instead of reporting an
    /// encoding failure as an empty, successfully parsed script.
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        let _ = (source, profile);
        Err(CompileError::Unsupported(
            "CompileService does not expose its native byte parser".into(),
        ))
    }

    /// Locate complete byte commands using the actual compiler-entry grammar.
    /// A supplied entry cannot be replaced by a catalogue grammar guess.
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ScriptCommandPlan, CompileError> {
        let _ = (source, profile, entry);
        Err(CompileError::Unsupported(
            "CompileService does not expose entry-aware byte parsing".into(),
        ))
    }

    /// Compile `src` to a [`Module`](Self::Module), or report why it could not.
    fn compile(&self, src: &str) -> Result<Self::Module, CompileError>;

    /// Compile `src` for the dialect profile currently selected by the
    /// interpreter. A VM may change profile after it has cached dynamic
    /// bodies, so reusing a compiler constructed for an older registry/grammar
    /// is not sound: an unavailable command could already have been lowered to
    /// bytecode and bypass normal command dispatch.
    ///
    /// Profile-aware services must override this and select the profile's
    /// registry, lexer grammar, and expression dialect for this invocation.
    /// A fixed-profile compiler is permitted only for the permissive fallback;
    /// named-profile dynamic compilation is rejected rather than silently
    /// stamping older bytecode as current.
    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if profile.is_fallback() {
            self.compile(src)
        } else {
            Err(CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )))
        }
    }

    /// Compile a runtime script in its exact command-resolution namespace.
    ///
    /// The root-namespace default preserves existing compiler services. A
    /// non-root target fails closed because delegating to
    /// [`Self::compile_for_profile`] would attach incorrect global binding
    /// provenance to namespaced bytecode.
    fn compile_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if target.namespace.is_empty() {
            self.compile_for_profile(target.source, profile)
        } else {
            Err(CompileError::Unsupported(
                "CompileService does not support namespaced script compilation".to_string(),
            ))
        }
    }

    /// Compile `src` with every registry-driven inline/structured lowering
    /// hook suppressed: every command compiles to a plain dispatch, so
    /// execution traces — including `enterstep`/`leavestep` step traces —
    /// observe it (C Tcl's `DONT_COMPILE_CMDS_INLINE`, `tclTrace.c`; a step
    /// trace forces the traced proc "out of bytecode" so no inner command,
    /// including `set`/`incr`/`if`/`while`, is invisible to the trace).
    ///
    /// Used to recompile a proc's (or any dynamically-evaluated script's)
    /// body once a step-capable execution trace targets it, and reverted the
    /// same way once the last such trace is removed — the VM never leaves a
    /// proc permanently de-optimised. The default fails closed: a compiler
    /// service must explicitly implement plain dispatch before the runtime can
    /// safely recover from command mutation or expose step-visible execution.
    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        let _ = src;
        Err(CompileError::Unsupported(
            "CompileService does not support plain command dispatch".to_string(),
        ))
    }

    /// Profile-aware counterpart of [`Self::compile_traced`]. See
    /// [`Self::compile_for_profile`] for why dynamic recompilation must select
    /// the VM's current profile rather than a compiler's construction-time
    /// profile. The default follows the same fixed-profile rejection rule.
    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if profile.is_fallback() {
            self.compile_traced(src)
        } else {
            Err(CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )))
        }
    }

    /// Compile `src` with command invocations preserved as ordinary runtime
    /// dispatches for `profile`.
    ///
    /// This is the semantic name for the de-optimised form shared by two
    /// runtime conditions: step-capable execution traces must observe every
    /// command, and a command-table mutation may have replaced a builtin that
    /// an optimised unit would otherwise bypass. Both require exactly the
    /// same compiler contract, so the runtime selects one path rather than
    /// maintaining parallel trace and mutation compilers.
    ///
    /// The existing trace-aware method remains the implementation seam for
    /// compile services that already override it. Its default is deliberately
    /// fail-closed: an optimising compiler must explicitly provide this
    /// capability before the runtime may use it for invalidation recovery.
    fn compile_plain_dispatch_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.compile_traced_for_profile(src, profile)
    }

    /// Plain-dispatch counterpart of [`Self::compile_script_for_profile`].
    /// The same fail-closed namespace rule applies even though the returned
    /// artifact must have no specialised command dependencies: structured
    /// body lowering and nested definitions still consume the script context.
    fn compile_plain_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if target.namespace.is_empty() {
            self.compile_plain_dispatch_for_profile(target.source, profile)
        } else {
            Err(CompileError::Unsupported(
                "CompileService does not support namespaced plain script compilation".to_string(),
            ))
        }
    }

    /// Compile against actual live interpreter bindings. A service without
    /// this capability must use explicit plain dispatch, never infer a fresh
    /// stock table from catalogue availability.
    fn compile_script_with_entry(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        let _ = entry;
        self.compile_plain_script_for_profile(target, profile)
    }

    /// Compile a procedure against actual live interpreter entry facts.
    /// Unsupported entry-aware services retain only explicit plain dispatch.
    fn compile_procedure_with_entry(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let _ = (entry, dispatch);
        self.compile_procedure_for_profile(target, profile, ProcedureDispatch::Plain)
    }

    /// Locate the complete-command prefix and optional fatal parse tail of a
    /// runtime script under `profile`'s exact lexer grammar.
    ///
    /// The default preserves compatibility with compile services that do not
    /// expose their parser: the later whole-script compile remains their error
    /// boundary. Compiler-backed services should override this so `catch` and
    /// `try` can execute a valid prefix before observing a malformed tail.
    ///
    /// # Errors
    /// Returns unavailable lexical ownership or invalid original-source
    /// geometry separately from a guest parse tail in a successful plan.
    fn script_command_plan_for_profile(
        &self,
        src: &str,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        Ok(ScriptCommandPlan::complete(src.len()))
    }

    /// Why a release that compiles a script whole
    /// (`TclVersion::compile_errors_fail_scripts`) refuses `src`, compiled as
    /// `unit`; `None` when it compiles. The default knows of no refusal.
    fn compile_failure_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
        unit: CompileUnit,
    ) -> Option<CompileFailure> {
        let _ = (src, profile, unit);
        None
    }

    /// Compile a procedure body for an exact dialect profile and dispatch
    /// mode. The default deliberately fails closed: silently delegating to
    /// [`Self::compile_for_profile`] would create script-context bytecode with
    /// no parameter LVT and incorrect `return`/local-variable semantics.
    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, dispatch);
        Err(CompileError::Unsupported(
            "CompileService does not support procedure-body compilation".to_string(),
        ))
    }
}
