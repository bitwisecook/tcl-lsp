// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Jim's own `info` table, independently of its inherited C command surface.

use crate::prelude::*;
use tcl_dialect::model::{Family, SpecSurface};

const NO_HOOK: crate::native_compilation::NativeCompilationSpec =
    crate::native_compilation::NativeCompilationSpec {
        grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
        operation: crate::SemanticOperationId::Invoke,
        body: crate::native_compilation::NativeBodyCompilation::Inherit,
    };

// Source proof: naming.info.jim-original-selector-table
// docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
// Jim_InfoCoreCommand owns these rows. No C ensemble worker, compiler hook,
// result-object class or successful-handler receipt is inherited here.
const fn member(
    name: &'static str,
    synopsis: &'static str,
    arity: Arity,
    detail: &'static str,
    roles: &'static [(u8, ArgRole)],
    return_type: Option<TclType>,
    traits: Traits,
) -> SubCommand {
    SubCommand {
        traits,
        name,
        synopsis,
        arity,
        detail,
        arg_roles: roles,
        return_type,
        native_compilation: Some(NO_HOOK),
        ..SubCommand::DEFAULT
    }
}

// Jim's INFO_EXISTS handler uses the selected variable lookup with no-error
// flags. The readonly name query shares its operation, not C compiler entry.
const fn existence(mut subcommand: SubCommand) -> SubCommand {
    let operation = crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoExists);
    subcommand.semantic_operation = Some(operation);
    subcommand.native_compilation = Some(crate::native_compilation::NativeCompilationSpec {
        operation,
        ..NO_HOOK
    });
    subcommand
}

const fn completeness(mut subcommand: SubCommand) -> SubCommand {
    subcommand.var_write_typing = crate::types::VarWriteTyping::Fixed(TclType::String);
    subcommand.traits = subcommand.traits.union(Traits::CONDITIONAL_VARIABLE_WRITE);
    subcommand
}

static SUBCOMMANDS: &[SubCommand] = &[
    member(
        "alias",
        "info alias command",
        Arity::exact(1),
        "Returns the retained command prefix of an alias.",
        &[(0, ArgRole::CommandName)],
        Some(TclType::List),
        Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
    ),
    member(
        "aliases",
        "info aliases ?-all? ?pattern?",
        Arity::new(0, 2),
        "Lists alias commands, optionally including command names containing spaces and filtering by a pattern.",
        &[],
        Some(TclType::List),
        Traits::REFLECTS_COMMAND_NAMES,
    ),
    member(
        "args",
        "info args procname",
        Arity::exact(1),
        "Returns the procedure's original argument list, including default-value fields.",
        &[(0, ArgRole::CommandName)],
        Some(TclType::List),
        Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
    ),
    member(
        "body",
        "info body procname",
        Arity::exact(1),
        "Returns the procedure's original body object.",
        &[(0, ArgRole::CommandName)],
        Some(TclType::String),
        Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
    ),
    member(
        "channels",
        "info channels ?-all? ?pattern?",
        Arity::new(0, 2),
        "Lists channel commands when the selected Jim build provides aio.",
        &[],
        Some(TclType::List),
        Traits::REFLECTS_COMMAND_NAMES,
    ),
    member(
        "commands",
        "info commands ?-all? ?pattern?",
        Arity::new(0, 2),
        "Lists commands; namespace-aware enumeration can invoke the actual namespace info helper.",
        &[],
        Some(TclType::List),
        Traits::REFLECTS_COMMAND_NAMES,
    ),
    completeness(member(
        "complete",
        "info complete script ?missing?",
        Arity::new(1, 2),
        "Tests script completeness and optionally writes its missing delimiter to a variable.",
        &[(1, ArgRole::VarWrite)],
        Some(TclType::Boolean),
        Traits::empty(),
    )),
    existence(member(
        "exists",
        "info exists varName",
        Arity::exact(1),
        "Tests whether the original variable name resolves to a value.",
        &[(0, ArgRole::VarRead)],
        Some(TclType::Boolean),
        Traits::empty(),
    )),
    member(
        "frame",
        "info frame ?levelNum?",
        Arity::new(0, 1),
        "Reports Jim's procedure-frame depth or the selected frame's source information.",
        &[],
        None,
        Traits::CURRENT_FRAME_INTROSPECTION,
    ),
    member(
        "globals",
        "info globals ?pattern?",
        Arity::new(0, 1),
        "Lists global variables; namespace-aware enumeration can invoke namespace info.",
        &[],
        Some(TclType::List),
        Traits::INTROSPECTS_BY_NAME.union(Traits::CURRENT_FRAME_INTROSPECTION),
    ),
    member(
        "help",
        "info help command",
        Arity::exact(1),
        "Returns native command help or a report that help is unavailable.",
        &[(0, ArgRole::CommandName)],
        Some(TclType::String),
        Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
    ),
    member(
        "hostname",
        "info hostname",
        Arity::exact(0),
        "Evaluates the actual os.gethostname command.",
        &[],
        Some(TclType::String),
        Traits::empty(),
    ),
    member(
        "level",
        "info level ?levelNum?",
        Arity::new(0, 1),
        "Returns the active call depth or the selected call's original arguments.",
        &[],
        None,
        Traits::CURRENT_FRAME_INTROSPECTION,
    ),
    member(
        "locals",
        "info locals ?pattern?",
        Arity::new(0, 1),
        "Lists local variables; namespace-aware enumeration can invoke namespace info.",
        &[],
        Some(TclType::List),
        Traits::INTROSPECTS_BY_NAME.union(Traits::CURRENT_FRAME_INTROSPECTION),
    ),
    member(
        "nameofexecutable",
        "info nameofexecutable",
        Arity::exact(0),
        "Evaluates the actual command whose single name is info nameofexecutable.",
        &[],
        Some(TclType::String),
        Traits::empty(),
    ),
    member(
        "patchlevel",
        "info patchlevel",
        Arity::exact(0),
        "Returns the Jim build description when configured, otherwise its core version.",
        &[],
        Some(TclType::String),
        Traits::empty(),
    ),
    member(
        "procs",
        "info procs ?-all? ?pattern?",
        Arity::new(0, 2),
        "Lists procedure commands; namespace-aware enumeration can invoke namespace info.",
        &[],
        Some(TclType::List),
        Traits::REFLECTS_COMMAND_NAMES,
    ),
    member(
        "references",
        "info references",
        Arity::exact(0),
        "Lists references when the selected Jim build enables references.",
        &[],
        Some(TclType::List),
        Traits::empty(),
    ),
    member(
        "returncodes",
        "info returncodes ?code?",
        Arity::new(0, 1),
        "Returns Jim's completion-code name or its code-to-name dictionary.",
        &[],
        None,
        Traits::empty(),
    ),
    SubCommand {
        source_path_operation: Some(crate::source_path::SourcePathOperation::ScriptPath),
        ..member(
            "script",
            "info script ?filename?",
            Arity::new(0, 1),
            "Returns or replaces the interpreter's current filename object.",
            &[],
            Some(TclType::String),
            Traits::empty(),
        )
    },
    member(
        "source",
        "info source source ?filename line?",
        Arity::stepped(1, 3, 2),
        "Returns source information, or creates a string with the supplied filename and line.",
        &[],
        None,
        Traits::empty(),
    ),
    member(
        "stacktrace",
        "info stacktrace",
        Arity::exact(0),
        "Returns the current Jim stack-trace object.",
        &[],
        Some(TclType::List),
        Traits::empty(),
    ),
    member(
        "statics",
        "info statics procname",
        Arity::exact(1),
        "Returns the procedure's retained static variable names and values.",
        &[(0, ArgRole::CommandName)],
        Some(TclType::List),
        Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
    ),
    member(
        "tainted",
        "info tainted value",
        Arity::exact(1),
        "Reports the taint bit of the original value object.",
        &[],
        Some(TclType::Boolean),
        Traits::empty(),
    ),
    member(
        "usage",
        "info usage command",
        Arity::exact(1),
        "Returns usage for the selected native command or procedure.",
        &[(0, ArgRole::CommandName)],
        Some(TclType::String),
        Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
    ),
    member(
        "vars",
        "info vars ?pattern?",
        Arity::new(0, 1),
        "Lists visible variables; namespace-aware enumeration can invoke namespace info.",
        &[],
        Some(TclType::List),
        Traits::INTROSPECTS_BY_NAME.union(Traits::CURRENT_FRAME_INTROSPECTION),
    ),
    member(
        "version",
        "info version",
        Arity::exact(0),
        "Returns the Jim interpreter's major and minor core version.",
        &[],
        Some(TclType::String),
        Traits::empty(),
    ),
];

impl crate::InvocationDialect {
    /// The stock Jim info table selected by its independently retained string
    /// recipe. This readonly table grants no installed command, native lookup
    /// cache, successful handler, helper execution or compiler preparation.
    /// C and unavailable/foreign Jim issuers keep their existing table owner.
    #[must_use]
    pub fn native_jim_info_member_names(self) -> Option<&'static [&'static str]> {
        use tcl_syntax::native_string::NativeStringProtocol;
        static NAMES: std::sync::OnceLock<Box<[&'static str]>> = std::sync::OnceLock::new();
        if self.native_string_protocol() != Some(NativeStringProtocol::Jim084) {
            return None;
        }
        Some(NAMES.get_or_init(|| SUBCOMMANDS.iter().map(|member| member.name).collect()))
    }

    /// Select Jim's original subcommand parser and reporting recipe. This does
    /// not issue its physical lookup cache or select any current helper command.
    #[must_use]
    pub fn native_jim_info_protocol(self) -> Option<NativeJimInfoProtocol> {
        Some(NativeJimInfoProtocol(
            self.native_jim_info_member_names()?,
            self.native_usage_protocol()?,
        ))
    }
}

/// Independently selected Jim info parser, with no object-cache authority.
#[derive(Clone, Copy, Debug)]
pub struct NativeJimInfoProtocol(
    &'static [&'static str],
    crate::native_usage::NativeUsageProtocol,
);

/// Whether the original Jim invocation allows current namespace forwarding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimInfoScope {
    /// Ordinary info dispatch may invoke the current multiword namespace helper.
    NamespaceAware,
    /// The original exact -nons option selects the direct core table operation.
    DirectCore,
}

pub use tcl_runtime_api::{NativeJimAliasLookupFailure, NativeJimCommandInventoryKind};

/// A selected inventory operation; no actual table or helper is retained here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimCommandInventory {
    /// Enumerate the actual flat table with the original optional pattern.
    Flat {
        /// Relative index in the original member operands, excluding the selector.
        pattern: Option<usize>,
        /// The exact -all option includes table names containing CString spaces.
        include_spaces: bool,
    },
    /// Invoke the actual current literal `namespace info` command.
    NamespaceHelper,
    /// The core handler rejects these operands before helper invocation.
    WrongArguments,
}

impl NativeJimInfoScope {
    /// Select the original inventory grammar and namespace-helper boundary.
    /// Current namespace bytes must come from the actual retained frame object.
    /// A rooted invocation head is independent of the exact -nons option.
    #[must_use]
    pub fn command_inventory(
        self,
        operand_count: usize,
        first_operand: Option<&[u8]>,
        actual_namespace: &[u8],
    ) -> NativeJimCommandInventory {
        let include_spaces =
            first_operand.is_some_and(|word| tcl_core_types::c_string_extent(word) == b"-all");
        let shift = usize::from(include_spaces);
        if operand_count > shift + 1 {
            return NativeJimCommandInventory::WrongArguments;
        }
        // Jim_InfoCoreCommand forwards the original member vector; the helper's
        // own formal grammar remains independent of this core option grammar.
        let rooted_single =
            operand_count == 1 && first_operand.is_some_and(|word| word.starts_with(b"::"));
        if self == Self::NamespaceAware && (!actual_namespace.is_empty() || rooted_single) {
            return NativeJimCommandInventory::NamespaceHelper;
        }
        NativeJimCommandInventory::Flat {
            pattern: (operand_count == shift + 1).then_some(shift),
            include_spaces,
        }
    }
}

impl NativeJimCommandInventory {
    /// The core inventory handler's own report, after the outer parser accepts
    /// the member. This does not substitute the invocation head for literal info.
    #[must_use]
    pub fn wrong_arguments_message(original_selector: &[u8]) -> Vec<u8> {
        let mut message = b"wrong # args: should be \"info ".to_vec();
        message.extend_from_slice(tcl_core_types::c_string_extent(original_selector));
        message.extend_from_slice(b" ?-all? ?pattern?\"");
        message
    }
}

/// Original-word positions retained by Jim's parser.
#[derive(Clone, Copy, Debug)]
pub enum NativeJimInfoDispatch {
    /// Invoke an actual selected member with the original remaining operands.
    Member {
        /// Static member selected from Jim's declaration.
        name: &'static str,
        /// Original command word, including the internal -nons shift.
        head: usize,
        /// First original member argument.
        arguments: usize,
        /// Independently selected -nons scope; rooting the head does not set it.
        scope: NativeJimInfoScope,
    },
    /// Publish parser usage, listing or diagnostic after any actual helper call.
    Report(NativeJimInfoReport),
}

/// Parser report whose helper/result boundary remains owned by the runtime.
#[derive(Clone, Copy, Debug)]
pub struct NativeJimInfoReport {
    head: usize,
    selector: usize,
    kind: JimInfoReportKind,
}

#[derive(Clone, Copy, Debug)]
enum JimInfoReportKind {
    /// Registered Jim_InfoCoreCommand arity, checked before Jim_ParseSubCmd.
    CoreWrongArity(crate::native_usage::NativeUsageProtocol),
    Missing,
    Commands,
    Usage,
    MemberUsage(usize),
    WrongArity(usize),
    Unknown,
    Ambiguous,
}

impl NativeJimInfoProtocol {
    /// Select the actual command flag for an already accepted canonical member.
    #[must_use]
    pub fn command_inventory_kind(self, member: &str) -> Option<NativeJimCommandInventoryKind> {
        match member {
            "commands" => Some(NativeJimCommandInventoryKind::Commands),
            "procs" => Some(NativeJimCommandInventoryKind::Procs),
            "aliases" => Some(NativeJimCommandInventoryKind::Aliases),
            _ => None,
        }
    }

    /// The original visible declaration passed to the actual current lsort.
    #[must_use]
    pub const fn names(self) -> &'static [&'static str] {
        self.0
    }

    /// Select original word positions in `Jim_InfoCoreCommand`/`Jim_ParseSubCmd`.
    /// Original strcmp equality precedes the counted-length strncmp scan. The callback
    /// supplies checked original bytes; this pure recipe installs no cache.
    pub fn dispatch<E>(
        self,
        word_count: usize,
        mut original: impl FnMut(usize) -> Result<std::rc::Rc<[u8]>, E>,
    ) -> Result<NativeJimInfoDispatch, E> {
        use JimInfoReportKind as K;
        // Jim_CoreCommandsTable registers this worker with minargs=1 and its
        // native usage. JimCallNative checks that arity before entering the
        // generic parser; the selected worker is independent of argv[0].
        if word_count < 2 {
            return Ok(NativeJimInfoDispatch::Report(NativeJimInfoReport {
                head: 0,
                selector: 1,
                kind: K::CoreWrongArity(self.1),
            }));
        }
        let mut head = 0;
        let mut selector = 1;
        if word_count > 2 && tcl_core_types::c_string_extent(&original(selector)?) == b"-nons" {
            head = 1;
            selector = 2;
        }
        let report = |kind, selector| {
            NativeJimInfoDispatch::Report(NativeJimInfoReport {
                head,
                selector,
                kind,
            })
        };
        if selector >= word_count {
            return Ok(report(K::Missing, selector));
        }
        let mut word = original(selector)?;
        let help = tcl_core_types::c_string_extent(&word) == b"-help";
        if help {
            selector += 1;
            if selector >= word_count {
                return Ok(report(K::Usage, selector));
            }
            word = original(selector)?;
        }
        if tcl_core_types::c_string_extent(&word) == b"-commands" {
            return Ok(report(K::Commands, selector));
        }
        let mut partial = None;
        let mut matched = None;
        for (index, name) in self.0.iter().enumerate() {
            if name.as_bytes() == tcl_core_types::c_string_extent(&word) {
                matched = Some(index);
                break;
            }
            // Table entries are ASCII C strings. A NUL in the counted key
            // matches only the same terminating NUL; strncmp then stops.
            let prefix = if word.contains(&0) {
                tcl_core_types::c_string_extent(&word) == name.as_bytes()
            } else {
                name.as_bytes().starts_with(&word)
            };
            if prefix {
                if partial.is_some() {
                    return Ok(report(if help { K::Usage } else { K::Ambiguous }, selector));
                }
                partial = Some(index);
            }
        }
        let Some(index) = matched.or(partial) else {
            return Ok(report(if help { K::Usage } else { K::Unknown }, selector));
        };
        if help {
            return Ok(report(K::MemberUsage(index), selector));
        }
        let arguments = selector + 1;
        if !u16::try_from(word_count - arguments)
            .is_ok_and(|count| SUBCOMMANDS[index].arity.accepts(count))
        {
            return Ok(report(K::WrongArity(index), selector));
        }
        Ok(NativeJimInfoDispatch::Member {
            name: self.0[index],
            head,
            arguments,
            scope: if head == 0 {
                NativeJimInfoScope::NamespaceAware
            } else {
                NativeJimInfoScope::DirectCore
            },
        })
    }
}

impl NativeJimInfoReport {
    /// Original head needed by this report, without deriving a table name.
    #[must_use]
    pub const fn head(self) -> usize {
        self.head
    }
    /// Original selector needed only by an unknown/ambiguous diagnostic.
    #[must_use]
    pub const fn selector(self) -> Option<usize> {
        if matches!(
            self.kind,
            JimInfoReportKind::Unknown | JimInfoReportKind::Ambiguous
        ) {
            Some(self.selector)
        } else {
            None
        }
    }
    /// Join separator after actual lsort succeeds. Failed helpers retain their
    /// own result bytes; they do not undergo list conversion or joining.
    #[must_use]
    pub const fn choices_separator(self) -> Option<&'static [u8]> {
        match self.kind {
            JimInfoReportKind::Commands => Some(b" "),
            JimInfoReportKind::Usage
            | JimInfoReportKind::Unknown
            | JimInfoReportKind::Ambiguous => Some(b", "),
            _ => None,
        }
    }
    /// The original parser's completion, independently of the helper completion.
    #[must_use]
    pub const fn succeeds(self) -> bool {
        matches!(
            self.kind,
            JimInfoReportKind::Commands
                | JimInfoReportKind::Usage
                | JimInfoReportKind::MemberUsage(_)
        )
    }
    /// Selected Jim printf/AppendStrings reporting extents. The -commands
    /// result retains the full counted helper result; formatted fields use %s.
    #[must_use]
    pub fn message(self, head: &[u8], selector: &[u8], choices: &[u8]) -> Vec<u8> {
        use JimInfoReportKind as K;
        if let K::CoreWrongArity(usage) = self.kind {
            return usage.render_command_usage_message(head, b"subcommand ?arg ...?");
        }
        let head = tcl_core_types::c_string_extent(head);
        let mut result = Vec::new();
        match self.kind {
            K::CoreWrongArity(_) => unreachable!("registered arity rendered above"),
            K::Commands => return choices.to_vec(),
            K::Missing => {
                result.extend_from_slice(b"wrong # args: should be \"");
                result.extend_from_slice(head);
                result.extend_from_slice(b" command ...\"\nUse \"");
                result.extend_from_slice(head);
                result.extend_from_slice(b" -help ?command?\" for help");
            }
            K::Usage => {
                result.extend_from_slice(b"Usage: \"");
                result.extend_from_slice(head);
                result.extend_from_slice(b" command ... \", where command is one of: ");
                result.extend_from_slice(tcl_core_types::c_string_extent(choices));
            }
            K::MemberUsage(index) | K::WrongArity(index) => {
                result.extend_from_slice(if self.succeeds() {
                    b"Usage: "
                } else {
                    b"wrong # args: should be \""
                });
                result.extend_from_slice(head);
                result.push(b' ');
                result.extend_from_slice(
                    SUBCOMMANDS[index]
                        .synopsis
                        .strip_prefix("info ")
                        .expect("Jim info synopsis")
                        .as_bytes(),
                );
                if !self.succeeds() {
                    result.push(b'"');
                }
            }
            K::Unknown | K::Ambiguous => {
                result.extend_from_slice(head);
                result.extend_from_slice(if matches!(self.kind, K::Unknown) {
                    b", unknown command \""
                } else {
                    b", ambiguous command \""
                });
                result.extend_from_slice(tcl_core_types::c_string_extent(selector));
                result.extend_from_slice(b"\": should be ");
                result.extend_from_slice(tcl_core_types::c_string_extent(choices));
            }
        }
        result
    }
}

/// Jim's own operation table wins over the C ancestry candidate.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "info",
        surface: Some(tcl_dialect::surface![SpecSurface::core_in(
            Family::Jim,
            &[("0.84", None)]
        )]),
        native_compilation: Some(NO_HOOK),
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        // Jim's listing/miss parser can call current lsort, and individual
        // operations can call namespace info, os.gethostname or a Tcl proc.
        // The table does not close those actual commands' effects.
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Information about the selected Jim interpreter",
            synopsis: &["info subcommand ?arg ...?"],
            snippet: "Jim exposes its own 27-operation table, independent of the C Tcl ancestry surface. Unique prefixes are accepted. It includes alias, aliases, channels, help, references, returncodes, source, stacktrace, statics, tainted, usage and version; C TclOO, coroutine, functions, loaded and tclversion operations are absent. Procedure args returns the original parameter list, including default fields. Namespace-aware listings and hostname/name-of-executable reporting can execute the actual selected helper commands.",
            source: "Jim_InfoCoreCommand and Jim_ParseSubCmd (Jim 0.84)",
            examples: "info args myproc\ninfo statics myproc\ninfo version",
            return_value: "Varies by subcommand; the operation table supplies no successful-handler or object-class receipt.",
        }),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_jim_inventory_keeps_core_options_and_namespace_forwarding_separate() {
        // Native proof: naming.info.original-root-and-explicit-nons-command-inventory
        // docs/design/analysis/name-resolution-proofs/info-original-root-and-explicit-nons-command-inventory.md
        // Pure selection is separate from actual table-key ownership and helper execution.
        let selected = |scope: NativeJimInfoScope, words: &[&[u8]], namespace: &[u8]| {
            scope.command_inventory(words.len(), words.first().copied(), namespace)
        };
        use NativeJimCommandInventory::{Flat, NamespaceHelper, WrongArguments};
        use NativeJimInfoScope::{DirectCore, NamespaceAware};
        assert_eq!(
            selected(NamespaceAware, &[b"-all", b"r2286*"], b""),
            Flat {
                pattern: Some(1),
                include_spaces: true
            }
        );
        assert_eq!(
            selected(NamespaceAware, &[b"-all", b"r2286*"], b"N"),
            NamespaceHelper
        );
        assert_eq!(
            selected(DirectCore, &[b"-all", b"r2286*"], b"N"),
            Flat {
                pattern: Some(1),
                include_spaces: true
            }
        );
        assert_eq!(selected(NamespaceAware, &[b"::N::*"], b""), NamespaceHelper);
        assert_eq!(
            selected(NamespaceAware, &[b"-all", b"::N::*"], b""),
            Flat {
                pattern: Some(1),
                include_spaces: true
            }
        );
        assert_eq!(
            selected(DirectCore, &[b"-al", b"r2286*"], b"N"),
            WrongArguments
        );
        assert!(matches!(
            parse(&[b"::info", b"commands", b"r2286*"]),
            NativeJimInfoDispatch::Member {
                scope: NamespaceAware,
                ..
            }
        ));
        assert!(matches!(
            parse(&[b"::info", b"-nons", b"commands", b"r2286*"]),
            NativeJimInfoDispatch::Member {
                scope: DirectCore,
                head: 1,
                arguments: 3,
                ..
            }
        ));
    }

    fn parse(words: &[&[u8]]) -> NativeJimInfoDispatch {
        let dialect = crate::InvocationDialect::of_profile(
            crate::model::resolve_environment("jim").unit_profile(),
        );
        dialect
            .native_jim_info_protocol()
            .unwrap()
            .dispatch(words.len(), |index| {
                Ok::<_, std::convert::Infallible>(std::rc::Rc::from(words[index]))
            })
            .unwrap()
    }

    #[test]
    fn original_jim_core_arity_precedes_generic_info_parser_reporting() {
        // naming.info.original-missing-and-empty-selector-dispatch
        // docs/design/analysis/name-resolution-proofs/info-original-missing-and-empty-selector-dispatch.md
        // Native292 measures bare info independently from empty selector input.
        // Pinned JimCallNative checks the genuine worker's minargs before
        // Jim_ParseSubCmd. The renamed/NUL cases below are renderer controls,
        // not additional public native invocations or command-table claims.
        let NativeJimInfoDispatch::Report(report) = parse(&[b"renamed info"]) else {
            panic!("selected core worker enforces registered minimum arity")
        };
        assert!(!report.succeeds());
        assert_eq!(report.head(), 0);
        assert_eq!(report.selector(), None);
        assert_eq!(report.choices_separator(), None);
        assert_eq!(
            report.message(b"renamed info", b"", b"UNUSED"),
            b"wrong # args: should be \"renamed info subcommand ?arg ...?\""
        );
        assert_eq!(
            report.message(b"renamed\0tail", b"", b"UNUSED"),
            b"wrong # args: should be \"renamed\""
        );
        let NativeJimInfoDispatch::Report(empty_selector) = parse(&[b"info", b""]) else {
            panic!("empty selector enters generic parser")
        };
        assert_eq!(empty_selector.selector(), Some(1));
        assert_eq!(empty_selector.choices_separator(), Some(b", ".as_slice()));
        assert_eq!(
            empty_selector.message(b"info", b"", b"CHOICES"),
            b"info, ambiguous command \"\": should be CHOICES"
        );
    }

    #[test]
    fn original_jim_existence_query_shares_operation_without_c_compiler_admission() {
        // naming.info.jim-original-selector-table
        // docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
        // Source: retained Jim_InfoCoreCommand INFO_EXISTS selects
        // Jim_GetVariable(interp, argv[2], 0), with no required contents read.
        let context = crate::model::ingress::static_context_for("jimtcl");
        let invocation = context
            .commands()
            .resolve_invocation(
                "info",
                &["exists", "u"],
                context.commands().own_surface_query(),
            )
            .unwrap();
        assert_eq!(
            invocation.semantics.operation,
            crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoExists)
        );
        let compiler = invocation.semantics.native_compilation.unwrap();
        assert_eq!(
            compiler.grammar,
            crate::native_compilation::NativeCompilationGrammar::NoHook
        );
        assert_eq!(compiler.operation, invocation.semantics.operation);
        assert_eq!(invocation.facts().arg_roles, [(0, ArgRole::VarRead)]);
    }

    #[test]
    fn original_jim_parser_keeps_immediate_and_counted_prefix_purposes_separate() {
        // Source proof: naming.info.jim-original-selector-table
        // docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
        assert!(matches!(
            parse(&[b"info", b"body\0\xff", b"p"]),
            NativeJimInfoDispatch::Member {
                name: "body",
                arguments: 2,
                ..
            }
        ));
        let NativeJimInfoDispatch::Report(encoded) = parse(&[b"info", b"body\xc0\x80"]) else {
            panic!("encoded zero is not CString termination")
        };
        assert!(!encoded.succeeds());
        assert_eq!(
            encoded.message(b"info", b"body\xc0\x80", b"CHOICES"),
            b"info, unknown command \"body\xc0\x80\": should be CHOICES"
        );
        let NativeJimInfoDispatch::Report(list) = parse(&[b"info", b"-commands\0tail", b"ignored"])
        else {
            panic!("immediate -commands comparison")
        };
        assert!(list.succeeds());
        assert_eq!(list.message(b"info", b"", b"A\0B"), b"A\0B");
        let NativeJimInfoDispatch::Report(help) = parse(&[b"info", b"-help", b"co"]) else {
            panic!("ambiguous help reports general usage")
        };
        assert!(help.succeeds());
        assert_eq!(help.choices_separator(), Some(b", ".as_slice()));
        let NativeJimInfoDispatch::Report(shifted) = parse(&[b"info", b"-nons", b"absent"]) else {
            panic!("internal namespace shift")
        };
        assert_eq!(shifted.head(), 1);
        assert_eq!(
            shifted.message(b"-nons", b"absent", b"CHOICES"),
            b"-nons, unknown command \"absent\": should be CHOICES"
        );
    }

    #[test]
    fn original_jim_info_inventory_and_arities_match_native_rows() {
        // Native proof: naming.info.jim-original-selector-table
        // docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
        let context = crate::model::static_context_for("jim");
        let selected = context.resolve_command("info").unwrap();
        assert_eq!(selected.subcommands.len(), 27);
        let captured = include_str!(
            "../../../tests/data/native_info_inventory_original/jim/inventory-jim-list/stdout.tsv"
        );
        let row = captured
            .lines()
            .find(|line| line.starts_with("INFO|0|"))
            .unwrap();
        let encoded = row.split('|').nth(3).unwrap();
        let bytes: Vec<_> = encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let names: Vec<_> = selected
            .subcommands
            .iter()
            .map(|member| member.name)
            .collect();
        assert_eq!(names.join(" ").as_bytes(), bytes);
        let actual = crate::InvocationDialect::of_profile(
            crate::model::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(actual.native_jim_info_member_names().unwrap(), names);
        let mut missing = actual;
        missing.core_point = None;
        assert_eq!(missing.native_jim_info_member_names(), None);
        let mut foreign = actual;
        foreign.native_family = Some(Family::Tcl);
        assert_eq!(foreign.native_jim_info_member_names(), None);
        for subcommand in selected.subcommands {
            let path = format!(
                "{}/tests/data/native_info_inventory_original/jim/exact-{}/stdout.tsv",
                env!("CARGO_MANIFEST_DIR"),
                subcommand.name
            );
            let captured = std::fs::read_to_string(path).unwrap();
            let row = captured
                .lines()
                .find(|line| line.starts_with("INFO|1|"))
                .unwrap();
            let encoded = row.split('|').nth(3).unwrap();
            let bytes: Vec<_> = encoded
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(
                bytes,
                format!("wrong # args: should be \"{}\"", subcommand.synopsis).as_bytes()
            );
            assert!(!subcommand.arity.accepts(10));
            // Software descriptor facts are separate from the public arity row.
            let expected_operation = if subcommand.name == "exists" {
                Some(crate::SemanticOperationId::Intrinsic(
                    crate::IntrinsicId::InfoExists,
                ))
            } else {
                None
            };
            assert_eq!(subcommand.semantic_operation, expected_operation);
            let expected = crate::native_compilation::NativeCompilationSpec {
                operation: expected_operation.unwrap_or(crate::SemanticOperationId::Invoke),
                ..NO_HOOK
            };
            assert_eq!(subcommand.native_compilation, Some(expected));
            assert!(subcommand.inline_codegen_hook.is_none());
            assert!(subcommand.successful_handler.is_none());
        }
        for absent in [
            "class",
            "object",
            "coroutine",
            "functions",
            "loaded",
            "default",
            "tclversion",
            "cmdcount",
        ] {
            assert!(!names.contains(&absent));
        }
        for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let c_context = crate::model::static_context_for(version);
            let c_info = c_context.resolve_command("info").unwrap();
            let actual = crate::InvocationDialect::of_profile(
                crate::model::resolve_environment(version).unit_profile(),
            );
            assert_eq!(actual.native_jim_info_member_names(), None);
            assert!(c_info.subcommands.iter().any(|sub| sub.name == "functions"));
            assert!(!c_info.subcommands.iter().any(|sub| sub.name == "statics"));
        }
        let complete = selected
            .subcommands
            .iter()
            .find(|subcommand| subcommand.name == "complete")
            .unwrap();
        assert_eq!(complete.arg_roles, &[(1, ArgRole::VarWrite)]);
        let source = selected
            .subcommands
            .iter()
            .find(|subcommand| subcommand.name == "source")
            .unwrap();
        assert!(source.arity.accepts(1));
        assert!(!source.arity.accepts(2));
        assert!(source.arity.accepts(3));
    }
}
