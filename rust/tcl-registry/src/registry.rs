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

//! Command registry — lookup facade.
//!
//! Built once at startup from command spec modules, then queried by
//! every consumer. Supports dialect filtering and trait-membership
//! queries.

use std::collections::HashMap;
use std::sync::OnceLock;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::abbrev::{Keyword, KeywordTable, PrefixMatching};
use crate::arg_role::{AppendedArity, ArgRole};
use crate::arity::Arity;
use crate::body_kind::BodyKind;
use crate::events::{
    DataCollectionAction, DataCollectionOperation, DataCollectionProtocol, EventHandlerPriority,
};
use crate::forms::CommandForm;
use crate::hooks::{AnalyserHookId, CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::hover::CallbackTaintInput;
use crate::invocation_words::{CommandPrefixArguments, InvocationWord};
use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::resolved_invocation::{
    InvocationResolutionUnresolved, ResolvedInvocation, ResolvedSubcommand,
    StructuredInvocationResolution, SubcommandResolution,
};
use crate::side_effects::SideSwitchTarget;
use crate::spec::{BytePayloadSpec, CommandSpec, SubCommand};
use crate::state_transition::StateTransitions;
use crate::traits::Traits;
use crate::types::VarWriteTyping;
use crate::{InvocationArguments, InvocationWords};
use tcl_dialect::DialectProfile;
use tcl_dialect::model::Family;
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_admits;
use tcl_dialect::model::surface_breadth;
use tcl_dialect::model::{SpecProvider, SurfaceLayer, surface_provided_by};
use tcl_dialect::version_satisfies;

/// The trait union defining a **frame-sensitive** command — see
/// [`CommandRegistry::is_frame_sensitive`].
const FRAME_SENSITIVE_TRAITS: Traits = Traits::TERMINATES_BLOCK
    .union(Traits::TRANSFERS_CONTROL)
    .union(Traits::CREATES_SCOPE_ALIAS)
    .union(Traits::CREATES_BARRIER);

/// Resolved metadata for an iRules event — the result of
/// [`CommandRegistry::event_info`].
#[derive(Debug, Clone)]
pub struct EventInfo {
    /// The upper-cased event name as queried.
    pub event: String,
    /// Introduction / deprecation / retirement releases on the BIG-IP axis —
    /// explicit data, with an absent introducing release inheriting the axis
    /// baseline (15.0.0). Entirely unspecified for an unknown event.
    pub lifecycle: Lifecycle,
    /// The event's lifecycle state at the queried BIG-IP release.
    pub lifecycle_state: LifecycleState,
    /// Whether the event is a recognised iRules event.
    pub known: bool,
    /// `"init"` / `"once_per_connection"` / `"per_request"` / `"unknown"`.
    pub multiplicity: &'static str,
    /// Description prose, or `""` when none is recorded.
    pub description: String,
    /// Connection-side label, or `"unknown"` for an unrecognised event.
    pub side: &'static str,
    /// Transport string (`"tcp"`, `"tcp/udp"`, `""`), or `None` for an
    /// unrecognised event.
    pub transport: Option<String>,
    /// Profile types implied by the event, sorted.
    pub implied_profiles: Vec<&'static str>,
    /// Sorted names of every command valid in this event (empty when the
    /// event is unknown).
    pub valid_commands: Vec<String>,
}

/// How a registry-described control-flow body relates to its enclosing call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlArmSemantics {
    /// The body always runs once the enclosing command is reached.
    Always,
    /// Run-time selection decides whether the body runs.
    Selected,
    /// The body runs, but in a frame that cannot inherit caller locals.
    FrameBoundary,
    /// The body runs, but its completion is contained by the enclosing call.
    CompletionBoundary,
    /// The body is repeated over a **finite** collection the invocation
    /// itself supplies — `foreach` / `lmap` / the `foreach`-line family.
    ///
    /// Like [`Self::Uncertain`] it names no *selectable* execution: which
    /// iteration (or whether any at all) a body run belongs to is not
    /// statically decidable, so nothing may be read out of the body's
    /// environment. It differs in the one fact that *is* decidable — the loop
    /// runs a bounded number of times, so the construct terminates whenever
    /// its body does. A consumer asking "does control reach the next
    /// statement?" can therefore answer from the body alone, which it cannot
    /// do for [`Self::Uncertain`].
    BoundedIteration,
    /// The body is conditional or repeated in a way this descriptor does not
    /// statically select, and whose trip count it cannot bound either — a
    /// condition-driven `for` / `while`.
    Uncertain,
}

/// The completion effect of one concrete registry invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationCompletion {
    /// Execution may continue with the following statement.
    FallsThrough,
    /// A normal procedure result, optionally naming its result argument.
    ReturnsResult(Option<usize>),
    /// A non-normal or otherwise non-result completion ends this path.
    Terminates,
    /// Dynamic or invalid completion options prevent a sound classification.
    Unknown,
}

/// An exact control completion available from registry-declared semantics and
/// literal invocation options.
///
/// Unlike [`InvocationCompletion`], this retains the Tcl completion code a
/// surrounding `try` may match.  `ProcessExit` is intentionally separate:
/// `exit` does not produce a catchable Tcl completion and bypasses `finally`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactInvocationCompletion {
    /// A Tcl completion code propagated through enclosing control constructs.
    Tcl(crate::completion::CompletionCode),
    /// Immediate interpreter-process termination.
    ProcessExit,
}

/// Registry-owned completion knowledge for a source-aware invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationCompletionKnowledge {
    /// A fully determined Tcl completion or process exit.
    Exact(ExactInvocationCompletion),
    /// `return -code $value` can either propagate return or reject the code.
    DynamicReturnOrError,
    /// A dynamic `exit` status either terminates the process or raises
    /// catchable Tcl error; it never has a normal continuation.
    ExitOrError,
    /// The invocation has a runtime-dependent completion.
    Dynamic,
}

/// Parse the literal option subset of `return` that fixes the completion code
/// visible to an enclosing `try`.  With the default `-level 1`, `return`
/// itself propagates `TCL_RETURN` even when its eventual procedure result is
/// configured as `-code error`; `-level 0` exposes that configured code to the
/// immediately enclosing script instead.
/// The literal-return parser distinguishes an invocation whose outcome is
/// genuinely runtime-dependent from one Tcl will reject before it can return.
/// The latter is still a precise `TCL_ERROR`, so an enclosing `try on error`
/// must receive it.
enum ExactReturnCompletion {
    Completion(crate::completion::CompletionCode),
    StaticError,
    Dynamic,
}

/// Parse `exit ?returnCode?` after its descriptor has established the exact
/// one-word-or-omitted shape.  Tcl 8.x passes its argument through
/// `Tcl_GetIntFromObj`, which accepts `-UINT_MAX..=UINT_MAX` before the C cast;
/// Tcl 9.0+ instead uses `TclGetWideBitsFromObj`, accepting every integer
/// (including bignums) before truncating its low bits to the C exit status.
/// A literal rejected by its release's conversion is an ordinary catchable
/// error, not a process termination.
enum ExactProcessExitCompletion {
    ProcessExit,
    StaticError,
    Dynamic,
}

fn exact_process_exit_completion(
    args: crate::invocation_words::InvocationArguments<'_>,
    numbers: tcl_syntax::number::Numbers,
    version: Option<tcl_dialect::TclVersion>,
) -> ExactProcessExitCompletion {
    match args.get(0) {
        None => ExactProcessExitCompletion::ProcessExit,
        Some(_) => match args.literal_at(0) {
            // Tcl 9 intentionally asks its bit-preserving wide conversion,
            // not `Tcl_GetIntFromObj`: bignums are accepted and truncated to
            // a C `int` only at `Tcl_Exit`. Tcl 8.x's public conversion has
            // the fixed asymmetric `-UINT_MAX..=UINT_MAX` (32-bit) range.
            Some(value)
                if matches!(
                    version,
                    Some(tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1)
                ) && matches!(
                    numbers.parse_whole(value),
                    Some(
                        tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
                    )
                ) =>
            {
                ExactProcessExitCompletion::ProcessExit
            }
            Some(value)
                if numbers.parse_wide(value).is_some_and(|value| {
                    (-i64::from(u32::MAX)..=i64::from(u32::MAX)).contains(&value)
                }) =>
            {
                ExactProcessExitCompletion::ProcessExit
            }
            Some(_) => ExactProcessExitCompletion::StaticError,
            None => ExactProcessExitCompletion::Dynamic,
        },
    }
}

fn exact_return_completion(
    args: crate::invocation_words::InvocationArguments<'_>,
    numbers: tcl_syntax::number::Numbers,
) -> ExactReturnCompletion {
    use crate::completion::CompletionCode;

    let mut i = 0usize;
    let mut code = CompletionCode::Ok;
    let mut level = 1_i64;
    while let Some(word) = args.literal_at(i) {
        match word {
            "-code" => {
                let Some(value) = args.literal_at(i + 1) else {
                    if args.get(i + 1).is_some() {
                        return ExactReturnCompletion::Dynamic;
                    }
                    // `return` accepts one trailing result word.  Tcl only
                    // treats `-code` as an option when another argv word can
                    // supply its value, so a lone `return -code` returns the
                    // literal result `-code` with the default TCL_RETURN.
                    break;
                };
                code = match value {
                    "ok" | "0" => CompletionCode::Ok,
                    "error" | "1" => CompletionCode::Error,
                    "return" | "2" => CompletionCode::Return,
                    "break" | "3" => CompletionCode::Break,
                    "continue" | "4" => CompletionCode::Continue,
                    value => match crate::completion::canonical_completion_code(value, numbers) {
                        Some(value) => CompletionCode::from_int(value),
                        None if !args.is_source_aware()
                            && tcl_syntax::naming::is_dynamic_word(value) =>
                        {
                            return ExactReturnCompletion::Dynamic;
                        }
                        None => return ExactReturnCompletion::StaticError,
                    },
                };
                i += 2;
            }
            "-level" => {
                let Some(value) = args.literal_at(i + 1) else {
                    if args.get(i + 1).is_some() {
                        return ExactReturnCompletion::Dynamic;
                    }
                    break;
                };
                level = match numbers.parse_wide(value) {
                    Some(value) if value >= 0 => value,
                    None if !args.is_source_aware()
                        && tcl_syntax::naming::is_dynamic_word(value) =>
                    {
                        return ExactReturnCompletion::Dynamic;
                    }
                    _ => return ExactReturnCompletion::StaticError,
                };
                i += 2;
            }
            // These options do not alter the code, but `-options` may carry
            // a code/level override so remains deliberately opaque.
            "-options" => {
                if args.get(i + 1).is_none() {
                    break;
                }
                return ExactReturnCompletion::Dynamic;
            }
            // Return options are an extensible key/value dictionary.  An
            // unrecognised literal option cannot alter `-code`/`-level`, so
            // retain the concrete completion rather than dropping a valid
            // custom pair such as `-foo bar`.
            _ if word.starts_with('-') => {
                if args.get(i + 1).is_none() {
                    break;
                }
                i += 2;
            }
            _ => break,
        }
    }
    // A non-literal word followed by another word may evaluate to an
    // extensible return option (for example `$option` -> `-code`). It is not
    // sound to classify the same source shape as the literal two-result-word
    // arity error.
    if args.get(i).is_some() && args.literal_at(i).is_none() && args.get(i + 1).is_some() {
        return ExactReturnCompletion::Dynamic;
    }
    // `return` accepts one result word after its options; additional words are
    // an arity error, so no exact runtime completion is promised.
    if args
        .exact_argv_len()
        .is_some_and(|len| len.saturating_sub(i) > 1)
    {
        return ExactReturnCompletion::StaticError;
    }
    ExactReturnCompletion::Completion(if level == 0 {
        code
    } else {
        CompletionCode::Return
    })
}

fn try_control_arms(args: &[&str]) -> Option<Vec<(usize, ControlArmSemantics)>> {
    args.first()?;
    let mut arms = vec![(0usize, ControlArmSemantics::Always)];
    let mut trailing_fallthrough = false;
    let mut i = 1usize;
    while i < args.len() {
        match args.get(i).copied() {
            Some("finally") if i + 2 == args.len() && !trailing_fallthrough => {
                arms.push((i + 1, ControlArmSemantics::Always));
                i += 2;
            }
            Some("on" | "trap") if i + 3 < args.len() => {
                let clause = args[i];
                let selector = args[i + 1];
                if (clause == "on"
                    && !matches!(selector, "ok" | "error" | "return" | "break" | "continue")
                    && selector.parse::<i64>().is_err())
                    || (clause == "trap"
                        && (tcl_syntax::naming::is_dynamic_word(selector)
                            || tcl_syntax::list::split_list(selector).is_err()))
                {
                    return None;
                }
                if tcl_syntax::naming::is_dynamic_word(args[i + 2]) {
                    return None;
                }
                let variables = tcl_syntax::list::split_list(args[i + 2]).ok()?;
                if variables.len() > 2 {
                    return None;
                }
                trailing_fallthrough = crate::commands::tcl::try_body_is_fallthrough(args[i + 3]);
                if !trailing_fallthrough {
                    arms.push((i + 3, ControlArmSemantics::Selected));
                }
                i += 4;
            }
            _ => return None,
        }
    }
    (!trailing_fallthrough).then_some(arms)
}

impl EventInfo {
    /// Number of commands valid in this event.
    #[must_use]
    pub fn valid_command_count(&self) -> usize {
        self.valid_commands.len()
    }
}

/// Number of commands declaring a taint source — computed at compile
/// time so [`TAINT_SOURCE_INDEX`] can be a fixed-size `const` array.
const fn count_taint_sources(specs: &[CommandSpec]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < specs.len() {
        if specs[i].taint_source.is_some() {
            n += 1;
        }
        i += 1;
    }
    n
}

/// Build the taint-source index at compile time by scanning the const
/// [`crate::commands::irules::IRULES_SPECS`] array for every spec's
/// [`crate::CommandSpec::taint_source`].
const fn build_taint_source_index()
-> [(&'static str, crate::taint::TaintColour); TAINT_SOURCE_COUNT] {
    let specs = crate::commands::irules::IRULES_SPECS;
    let mut out = [("", crate::taint::TaintColour::empty()); TAINT_SOURCE_COUNT];
    let mut i = 0;
    let mut k = 0;
    while i < specs.len() {
        if let Some(colour) = specs[i].taint_source {
            out[k] = (specs[i].name, colour);
            k += 1;
        }
        i += 1;
    }
    out
}

const TAINT_SOURCE_COUNT: usize = count_taint_sources(crate::commands::irules::IRULES_SPECS);

/// The taint-source index: command name → getter-form source colour, a
/// **compile-time** table derived from every iRules spec's
/// [`crate::CommandSpec::taint_source`] — the data's single home is each
/// `CommandSpec`, so this never drifts from the spec definitions.
///
/// Independent of which dialects a registry has loaded: a `tcl8.6`
/// document still
/// sees `HTTP::path` as a source. (The core Tcl sources `gets` / `read` /
/// `exec` / … are classified by [`crate::Traits::TAINT_SOURCE`] instead,
/// so they carry no index entry.)
const TAINT_SOURCE_INDEX: [(&str, crate::taint::TaintColour); TAINT_SOURCE_COUNT] =
    build_taint_source_index();

/// Lookup facade over command specs.
///
/// The registry is built once from the command spec modules and then
/// queried read-only. All command-specific knowledge lives in the
/// specs — consumers never match on command name strings.
pub struct CommandRegistry {
    by_name: FxHashMap<&'static str, Vec<&'static CommandSpec>>,
    loaded_layers: Vec<SurfaceLayer>,
    /// The dialect profile this registry was built for, when it came from
    /// `registry_for_profile` / `registry_for_dialect`. `None` for
    /// hand-assembled registries (tests, ad-hoc tools), which fall back to
    /// the `loaded_layers`-derived behaviour answers.
    profile: Option<&'static tcl_dialect::DialectProfile>,
    /// Packages a `SpecTcl` pack declared **ambient** in this registry's
    /// dialect, with the version the runtime provides — `(package, version)`,
    /// in installation order.
    ///
    /// Empty for every compiled-in registry. A pack fills it through
    /// [`Self::insert_ambient_package`], which is how a package that comes
    /// *with* a dialect rather than being `package require`d gets a version
    /// floor at all (issue #1627).
    ///
    /// This is the pack-authored twin of [`tcl_dialect::LibraryPin`] with
    /// `ambient: true`. The profile axis is compiled in and describes the
    /// dialects this repository models; this one is authored outside it, which
    /// is the axis that has to exist before `tk` / `tcllib` / `iapps` can move
    /// to packs (issue #1631) — a package's own version floor must not depend
    /// on whether this crate happens to know the package's name.
    ambient_packages: Vec<(&'static str, &'static str)>,
}

/// The set of command names registered by *every* dialect, built once and
/// cached.  Backs [`CommandRegistry::known_in_any_dialect`] — the
/// dialect-agnostic existence check over every loaded dialect.
/// Built from the same spec functions [`CommandRegistry::build_default`]
/// and [`CommandRegistry::load_surface`] draw from, so it stays in lock-step
/// with the registry's command universe.
fn all_dialect_command_names() -> &'static FxHashSet<&'static str> {
    static NAMES: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut set: FxHashSet<&'static str> = FxHashSet::default();
        let mut add = |specs: Vec<CommandSpec>| {
            for spec in specs {
                // Normalise away a leading `::` so a spec registered only in
                // its fully-qualified spelling (e.g.
                // `::tcl::unsupported::corotype`, which has no separate bare
                // registration) still matches `known_in_any_dialect`'s
                // already-bare query — the caller strips a literal `::` head
                // from the source text before calling in, so the set must be
                // bare-normalised too or the two never agree.
                set.insert(spec.name.strip_prefix("::").unwrap_or(spec.name));
            }
        };
        add(crate::commands::bpf::bpf_command_specs());
        add(crate::commands::tcl::tcl_command_specs());
        add(crate::commands::stdlib::stdlib_command_specs());
        add(crate::commands::tcllib::tcllib_command_specs());
        add(crate::commands::argparse::argparse_command_specs());
        add(crate::commands::ticklecharts::ticklecharts_command_specs());
        add(crate::commands::itcl::itcl_command_specs());
        add(crate::commands::tk::tk_command_specs());
        add(crate::commands::irules::irules_command_specs());
        add(crate::commands::iapps::iapps_command_specs());
        add(crate::commands::expect::expect_command_specs());
        // The EDA vendor libraries are deliberately NOT added: they ship as
        // bundled `.tclspec` loadables (`docs/design/spec-packs.md`), so this
        // crate does not know their names at compile time and W002 reports an
        // EDA command outside an EDA profile as an ordinary unknown command
        // rather than as "exists, but not here". The pack is what knows.
        // SpecTcl is deliberately NOT added. This set answers "is this name a
        // command in *some* dialect", and W002 turns a `true` into "exists,
        // but not here". SpecTcl's statement words are ordinary English nouns
        // (`arity`, `traits`, `value`, `detail`) that mean nothing outside a
        // pack body, so claiming them here would rewrite an honest
        // unknown-command report on a user's `proc arity` call into a
        // misleading dialect-availability one — the exact opposite of the
        // context-sensitivity the SpecTcl grammars exist to provide.
        set
    })
}

/// Append the indices covered by `layouts` whose declared role equals `role`.
///
/// `tail_len` is the number of argument words the layouts index over (the
/// whole post-head list, or the words after a subcommand word), and `offset`
/// is added to each result to convert back into an absolute `args` index —
/// the same `+1`-for-the-subcommand-word convention every other role source
/// here uses.
fn push_repeated_roles(
    out: &mut Vec<usize>,
    layouts: &[crate::repeated::RepeatedArgLayout],
    tail_len: usize,
    offset: usize,
    role: ArgRole,
) {
    for layout in layouts.iter().filter(|l| l.role == role) {
        out.extend(layout.indices(tail_len).into_iter().map(|i| i + offset));
    }
}

/// Append the `args` indices consumed by value-taking options whose value role
/// (primary or secondary) equals `role`.
///
/// Walks `args` from `scan_start` (1 to skip a subcommand word, else 0),
/// resolving option names, aliases, and unique abbreviations through the
/// declared table — so `-1`, `$x`, `[cmd]`, and ambiguous abbreviations are
/// treated as positionals — and advancing past each recognised option and the
/// value word(s) it consumes ([`OptionSpec::value_indices`], which honours
/// arity and the `--` terminator). The emitted indices are absolute into `args`,
/// exactly like the positional roles, so consumers map them via `argv[idx + 1]`
/// unchanged. A two-way binding (`role: VarWrite, also_role: VarRead`) emits its
/// index for a query of either role — the multi-role convention, split across
/// queries.
fn push_option_value_roles(
    out: &mut Vec<usize>,
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    role: ArgRole,
) {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = crate::spec::resolve_option_prefix(options, args[i]) {
            let vals = opt.value_indices(args, i);
            if opt.value_role() == Some(role) || opt.value_also_role() == Some(role) {
                out.extend(vals.iter().copied());
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
}

/// Return the timing of the script-valued option that consumes `index`.
///
/// The scan deliberately mirrors [`push_option_value_roles`]: aliases and
/// unique abbreviations resolve through the same table, and value arity / `--`
/// handling comes from [`OptionSpec::value_indices`].  `None` means either that
/// `index` is not an option value or that the consuming value is not a script.
fn option_script_timing_at(
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
) -> Option<crate::hover::ScriptTiming> {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = crate::spec::resolve_option_prefix(options, args[i]) {
            let vals = opt.value_indices(args, i);
            if vals.contains(&index) {
                return opt.value_script_timing();
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
    None
}

/// Return the external callback substitutions declared by the option value
/// that consumes `index`. The empty slice is a meaningful answer: the option
/// is a callback, but it carries no user-controlled replacement value.
fn option_callback_taint_inputs_at(
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
) -> Option<&'static [CallbackTaintInput]> {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = crate::spec::resolve_option_prefix(options, args[i]) {
            let vals = opt.value_indices(args, i);
            if vals.contains(&index) {
                return Some(opt.value_callback_taint_inputs());
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
    None
}

/// Return the variable frame of the option value that consumes `index`.
///
/// The scan is identical to [`option_script_timing_at`] and
/// [`push_option_value_roles`], so aliases, unique prefixes, arity, and the
/// option terminator cannot disagree between role and scope consumers.
fn option_variable_scope_at(
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
) -> Option<crate::hover::VariableScope> {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = crate::spec::resolve_option_prefix(options, args[i]) {
            let vals = opt.value_indices(args, i);
            if vals.contains(&index) {
                return opt.value_variable_scope();
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
    None
}

/// Collect option values whose role is [`ArgRole::CommandPrefix`], paired with
/// the option's [`AppendedArity`] — the option-side companion to
/// [`push_option_value_roles`], used by [`CommandRegistry::command_prefixes`].
fn push_command_prefix_options(
    out: &mut Vec<(usize, AppendedArity)>,
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
) {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = crate::spec::resolve_option_prefix(options, args[i]) {
            let vals = opt.value_indices(args, i);
            if opt.value_role() == Some(ArgRole::CommandPrefix) {
                let arity = opt.value_appended_arity();
                out.extend(vals.iter().map(|&v| (v, arity)));
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
}

/// Whether source words prove a command's leading option layout.
///
/// A literal option consumes its descriptor-declared values before scanning
/// continues. A dynamic word while that scan is live might evaluate to a
/// value-taking option, so every resolver-derived positional role must
/// abstain. A literal invalid or ambiguous dash word is likewise not a
/// positional operand: treating it as one would describe an invocation Tcl
/// rejects. The profile-filtered descriptor table, its abbreviation policy,
/// and the command's reserved trailing operands are inputs, so every pattern
/// and format consumer makes this decision from the same owner data.
fn source_option_layout_is_proven(
    options: &[&crate::hover::OptionSpec],
    args: InvocationArguments<'_>,
    prefix_matching: PrefixMatching,
    reserved_trailing_words: usize,
) -> bool {
    if !args.has_exact_argv_len() {
        return false;
    }
    let spellings: Vec<_> = (0..args.len())
        .map(|index| args.literal_at(index).unwrap_or_default())
        .collect();
    let mut index = 0;
    let option_scan_end = args.len().saturating_sub(reserved_trailing_words);
    while index < option_scan_end {
        if matches!(args.get(index), Some(InvocationWord::DynamicNonOption)) {
            return true;
        }
        let Some(word) = args.literal_at(index) else {
            return false;
        };
        let Some(option) =
            crate::patterns::resolve_available_option_prefix_with(options, word, prefix_matching)
        else {
            return !word.starts_with('-');
        };
        // A hook-defined arity reads following values to decide how many
        // words it consumes. A substituted value would make that boundary
        // runtime-dependent, unlike an ordinary fixed-arity option such as
        // `-start $index`.
        if option.value_arity_hook().is_some()
            && (index + 1..args.len()).any(|value| args.literal_at(value).is_none())
        {
            return false;
        }
        index += 1 + option.value_word_count(&spellings, index);
        if option.name == "--" {
            return true;
        }
    }
    true
}

/// The shipped specs of one command group, built once and leaked.
///
/// Every group below is a `fn() -> Vec<CommandSpec>` over `const` data, so it
/// returns byte-identical specs however often it is called — and it was called
/// often: once per `build_default`, which the cache's own docs note runs per
/// CFG build on some paths, and once more per dialect layer per registry
/// generation. Building each group once and handing out `&'static` slices
/// makes a registry an index over shared data instead of an owner of a private
/// copy of it.
fn shared_specs(
    cell: &'static OnceLock<&'static [CommandSpec]>,
    build: fn() -> Vec<CommandSpec>,
) -> &'static [CommandSpec] {
    cell.get_or_init(|| &*Vec::leak(build()))
}

/// Declare a `fn() -> &'static [CommandSpec]` wrapping one group's builder in
/// its own `OnceLock`.
macro_rules! shared_group {
    ($name:ident, $build:expr) => {
        fn $name() -> &'static [CommandSpec] {
            static CELL: OnceLock<&'static [CommandSpec]> = OnceLock::new();
            shared_specs(&CELL, $build)
        }
    };
}

shared_group!(tcl_specs, crate::commands::tcl::tcl_command_specs);
shared_group!(stdlib_specs, crate::commands::stdlib::stdlib_command_specs);
shared_group!(tcllib_specs, crate::commands::tcllib::tcllib_command_specs);
shared_group!(
    argparse_specs,
    crate::commands::argparse::argparse_command_specs
);
shared_group!(
    ticklecharts_specs,
    crate::commands::ticklecharts::ticklecharts_command_specs
);
shared_group!(itcl_specs, crate::commands::itcl::itcl_command_specs);
shared_group!(tk_specs, crate::commands::tk::tk_command_specs);
shared_group!(bpf_specs, crate::commands::bpf::bpf_command_specs);
shared_group!(irules_specs, crate::commands::irules::irules_command_specs);
shared_group!(iapps_specs, crate::commands::iapps::iapps_command_specs);
shared_group!(tmsh_specs, crate::commands::iapps::tmsh_command_specs);
shared_group!(expect_specs, crate::commands::expect::expect_command_specs);
shared_group!(
    spectcl_specs,
    crate::commands::spectcl::spectcl_command_specs
);

impl CommandRegistry {
    /// Build the default registry with core Tcl + stdlib + tcllib commands.
    #[must_use]
    pub fn build_default() -> Self {
        let mut registry = Self {
            by_name: FxHashMap::default(),
            loaded_layers: Vec::new(),
            profile: None,
            ambient_packages: Vec::new(),
        };
        for spec in tcl_specs() {
            registry.insert_static(spec);
        }
        for spec in stdlib_specs() {
            registry.insert_static(spec);
        }
        for spec in tcllib_specs() {
            registry.insert_static(spec);
        }
        for spec in argparse_specs() {
            registry.insert_static(spec);
        }
        for spec in ticklecharts_specs() {
            registry.insert_static(spec);
        }
        for spec in itcl_specs() {
            registry.insert_static(spec);
        }
        // Tk geometry/widget commands (`grid` / `pack` / `wm` / `button` / …)
        // are part of the always-known command universe: a `.tcl` script may
        // `package require Tk` at runtime, and the diagnostics treat them as
        // recognised under every Tcl dialect, so Tk is folded into the base
        // registry.  Mark the layer loaded so a later `load_surface(Tk)` is
        // a no-op rather than a double-insert.
        for spec in tk_specs() {
            registry.insert_static(spec);
        }
        registry.loaded_layers.push(SurfaceLayer::Package("Tk"));
        registry
    }

    /// Load a surface's commands into the registry (idempotent).
    pub fn load_surface(&mut self, layer: SurfaceLayer) {
        if self.loaded_layers.contains(&layer) {
            return;
        }
        let specs: &'static [CommandSpec] = match layer {
            SurfaceLayer::Package("bpf") => bpf_specs(),
            SurfaceLayer::Core(Family::F5Irules, _) => irules_specs(),
            SurfaceLayer::Package("iapps") => iapps_specs(),
            // The tmsh shell's own pack: the `tmsh::` surface shared with
            // iApps, without the iApp-only commands (D8).
            SurfaceLayer::Package("tmsh") => tmsh_specs(),
            SurfaceLayer::Package("Tk") => tk_specs(),
            SurfaceLayer::Package("expect") => expect_specs(),
            // SpecTcl: the `.tclspec` DSL's own statement words. A pack file
            // is an ordinary Tcl script, so the base Tcl surface stays loaded
            // underneath (hook bodies are real Tcl); this layer adds the
            // declaration vocabulary on top of it.
            SurfaceLayer::Package("spectcl") => spectcl_specs(),
            // A core release brings no pack of its own — it records which
            // language the registry is. The EDA shells are such a release
            // plus `required_package`-gated command libraries, which ship as
            // bundled `.tclspec` loadables (`docs/design/spec-packs.md`).
            _ => &[],
        };
        for spec in specs {
            self.insert_static(spec);
        }
        self.loaded_layers.push(layer);
    }

    /// Load iRules dialect commands (convenience wrapper).
    pub fn load_irules(&mut self) {
        self.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    }

    /// Load BPF-Tcl dialect commands (convenience wrapper).
    pub fn load_bpf(&mut self) {
        self.load_surface(SurfaceLayer::Package("bpf"));
    }

    /// Whether this registry's dialect reads a bare leading-zero integer
    /// (`08`, `010`) as **octal**.
    ///
    /// Tcl 9.0 dropped the leading-zero octal rule (TIP 114): `08` parses as
    /// decimal 8 and `010` as decimal 10. Every earlier Tcl (8.4/8.5/8.6) and
    /// every 8.x-derived dialect (f5-irules ≈ 8.4, f5-iapps ≈ 8.5/8.6, the EDA
    /// dialects) keeps the octal rule, where `08`/`09` are *invalid* octal
    /// (treated as a string in `==`/`!=`) and `010` is 8.
    ///
    /// TIP 114 lands in tcl9.0 and stays in tcl9.1 (and any later 9.x), so the
    /// decimal rule applies to *every* Tcl 9 dialect, not tcl9.0 alone. The
    /// per-dialect registry built by `registry_for_dialect` records its Tcl
    /// version via [`Self::load_surface`], so a registry whose
    /// `loaded_layers` names a 9.x core release (tcl9.0, tcl9.1, …) is
    /// decimal; every other dialect (8.4/8.5/8.6 and the F5/EDA registries,
    /// which never load a Tcl-9 release row) reads leading zeros as octal.
    #[must_use]
    pub fn leading_zero_is_octal(&self) -> bool {
        self.octal_fold_policy().unwrap_or(true)
    }

    /// The release this registry's dialect runs, if it names one.
    ///
    /// The registry-level mirror of [`DialectProfile::runtime_version`], so a
    /// consumer holding a registry never spells out `profile().and_then(…)`.
    #[must_use]
    pub fn runtime_version(&self) -> Option<tcl_dialect::TclVersion> {
        self.profile()
            .and_then(tcl_dialect::DialectProfile::runtime_version)
    }

    /// The string/character model of the release this registry's dialect runs.
    #[must_use]
    pub fn character_model(&self) -> Option<tcl_dialect::StringCharacterModel> {
        self.profile()
            .and_then(tcl_dialect::DialectProfile::character_model)
    }

    /// The numeral grammar of the dialect this registry serves, or the
    /// permissive 9.x default when no profile is loaded.
    ///
    /// The single way a registry-holding consumer names its release for number
    /// parsing — the hand-written `profile().map_or(Tcl90, …)` had six copies.
    #[must_use]
    pub fn numbers(&self) -> tcl_dialect::NumberSyntax {
        tcl_dialect::NumberSyntax::of_profile(self.profile())
    }

    /// The three-valued leading-zero fold policy for this registry's
    /// dialect. A profile-built registry (`registry_for_profile` /
    /// `registry_for_dialect`) answers from the profile's runtime base:
    /// `Some(true)` = 8.x octal, `Some(false)` = 9.x decimal (`bpf`
    /// included, D7), `None` = abstain — no Tcl runtime to have an opinion
    /// (`f5-bigip`, the unknown-dialect fallback; §11.1 of the
    /// dialect-profile model). A hand-assembled registry keeps the
    /// historical `loaded_layers` derivation (a version pack records its
    /// release, so a 9.x load reads decimal).
    #[must_use]
    pub fn octal_fold_policy(&self) -> Option<bool> {
        match self.profile {
            Some(p) => p.leading_zero_is_octal.as_bool(),
            None => Some(!self.loaded_core_is_tcl9()),
        }
    }

    /// Stamp the dialect profile this registry serves. Called by the
    /// per-profile cache (`registry_for_profile`) so behaviour queries
    /// (`octal_policy`, future runtime projections) answer from the
    /// profile rather than re-deriving from loaded packs.
    pub(crate) fn set_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        self.profile = Some(profile);
    }

    /// The dialect profile this registry was built for, if any.
    #[must_use]
    pub fn profile(&self) -> Option<&'static tcl_dialect::DialectProfile> {
        self.profile
    }

    /// The point a consumer resolves hooks and calls at when honouring
    /// **this registry's own** dialect: the attached profile's, or `None`
    /// — surface-blind — for a profile-less registry.
    ///
    /// This is what lets a version-pinned compile pipeline (issues
    /// #1462/#1463) suppress a structured lowering or codegen hook for a
    /// command the emulated release does not have — `lmap` under a tcl8.4
    /// registry resolves to no spec, so the call reaches the runtime's
    /// availability gate as a generic dispatch instead of being inlined.
    #[must_use]
    pub fn own_surface_query(&self) -> Option<SurfaceQuery<'static>> {
        self.profile.map(DialectProfile::surface_query)
    }

    /// Whether a loaded core layer is a Tcl 9.x release — the derivation a
    /// profile-less registry falls back to for the octal question.
    fn loaded_core_is_tcl9(&self) -> bool {
        self.loaded_layers.iter().any(|layer| {
            matches!(layer, SurfaceLayer::Core(Family::Tcl, release)
                if version_satisfies(release, "9.0-"))
        })
    }

    /// Insert an owned command spec, **leaking it** for the process lifetime.
    ///
    /// The registry indexes `&'static CommandSpec`, so an owned spec has to be
    /// given somewhere permanent to live. That is the right trade for the two
    /// callers that need it — a test building a registry by hand, and a
    /// `.tclspec` pack whose specs are already leaked by the loader — and the
    /// wrong one for the hundreds of shipped specs, which is why
    /// [`Self::insert_static`] exists and every built-in path uses it.
    ///
    /// A caller in a loop over user input wants `insert_static` and an arena
    /// it controls, not this.
    pub fn insert(&mut self, spec: CommandSpec) {
        self.insert_static(Box::leak(Box::new(spec)));
    }

    /// Insert a spec the caller already owns permanently — no copy, no leak.
    ///
    /// This is what makes a per-pack-edit registry affordable. A registry is
    /// rebuilt from scratch for every distinct pack-set content the server
    /// sees, and a `CommandSpec` is 1,296 bytes: copying the ~2,400 shipped
    /// specs into each generation cost megabytes a keystroke. Sharing one
    /// leaked copy of every shipped spec reduces a generation to its index —
    /// the names, and one pointer each.
    pub fn insert_static(&mut self, spec: &'static CommandSpec) {
        self.by_name.entry(spec.name).or_default().push(spec);
    }

    /// Record that `package` is ambient in this registry's dialect at
    /// `version` — the `ambient_package` statement of a `SpecTcl` pack.
    pub fn insert_ambient_package(&mut self, package: &'static str, version: &'static str) {
        self.ambient_packages.push((package, version));
    }

    /// Whether `package` is **ambient** — provided by the runtime, with no
    /// `package require` needed to reach it.
    ///
    /// The union of the two things that can make that true: the dialect
    /// profile shipping it (an F5 surface is part of the runtime, §7.1 axis
    /// C), and a loaded `SpecTcl` pack declaring it with `ambient_package`.
    /// This is the one place that union is taken — every consumer that used
    /// to ask the profile alone must ask here instead, or a pack's ambient
    /// declaration registers a floor while the same package still draws a
    /// "needs a `package require`" diagnostic.
    #[must_use]
    pub fn is_ambient_package(&self, package: &str) -> bool {
        self.profile
            .is_some_and(|profile| profile.is_ambient_package(package))
            || self
                .ambient_packages
                .iter()
                .any(|(name, _)| *name == package)
    }

    /// The version floor a pack declared for `package` as ambient, if any.
    ///
    /// The **highest** declared version wins when several packs name the same
    /// package: each is a claim that the runtime provides at least that
    /// version, and the strongest such claim is the one that holds. This is
    /// the same max-composition an explicit `package require` goes through,
    /// and for the same reason — a floor is a lower bound, so combining two
    /// lower bounds takes the greater.
    #[must_use]
    pub fn ambient_package_floor(&self, package: &str) -> Option<&'static str> {
        self.ambient_packages
            .iter()
            .filter(|(name, _)| *name == package)
            .map(|(_, version)| *version)
            .max_by(|a, b| crate::version::compare(a, b))
    }

    /// Every `(package, declared floor)` row loaded packs registered with
    /// `ambient_package`, in registration order. The enumeration face of
    /// [`Self::is_ambient_package`] / [`Self::ambient_package_floor`], for
    /// the model layer's context assembly
    /// ([`crate::model::assembly::ContextRegistry`]) — the resolved context
    /// carries these rows so pack-ambient floors answer from the context
    /// rather than from a live registry borrow.
    #[must_use]
    pub fn ambient_package_rows(&self) -> &[(&'static str, &'static str)] {
        &self.ambient_packages
    }

    /// Whether `name` exists as a command in *any* dialect, independent of
    /// which dialects this registry instance loaded.
    ///
    /// Looks up the global by-name index across every dialect's specs.
    /// Like [`Self::taint_source`], this is deliberately dialect-agnostic:
    /// an iRules command such as `when` is "known" even when analysing a
    /// `tcl8.6` document whose registry never loaded the iRules specs — the
    /// W002 disabled-in-dialect check needs to distinguish "exists, but not
    /// in this dialect" (→ DISALLOWED) from "exists nowhere" (→ W123's
    /// concern).  A leading `::` falls back to the bare name, matching
    /// [`Self::get`].
    #[must_use]
    pub fn known_in_any_dialect(&self, name: &str) -> bool {
        let bare = name.strip_prefix("::").unwrap_or(name);
        all_dialect_command_names().contains(bare)
    }

    /// Look up a command spec by name (dialect-agnostic).
    ///
    /// A leading `::` (global qualifier) falls back to the bare name, so an
    /// explicitly-global call to a built-in (`::foreach`, `::for`, …)
    /// resolves to the same spec as its unqualified form.
    ///
    /// The return is `&'static` because the registry stores only interned
    /// static specs — the identity [`crate::model::SpecKey`] keys the realm
    /// binding layer by (P1a); the P2 generation work re-keys dynamic pack
    /// specs when non-`'static` specs join.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&'static CommandSpec> {
        self.by_name
            .get(name)
            .or_else(|| {
                name.strip_prefix("::")
                    .and_then(|bare| self.by_name.get(bare))
            })
            .and_then(|v| v.last().copied())
    }

    /// Exact spelling lookup; unlike [`Self::get`], does not apply rooted
    /// global-name fallback.
    #[must_use]
    pub fn get_exact(&self, name: &str) -> Option<&'static CommandSpec> {
        self.by_name.get(name).and_then(|v| v.last().copied())
    }

    /// Whether a **fresh interpreter** of this registry's dialect already holds
    /// a command at exactly `qualified_name` — the question `namespace
    /// import`'s "already exists" conflict asks, and the one `info commands
    /// ::x` answers.
    ///
    /// Deliberately *not* [`Self::get`], which resolves every **spelling** a
    /// call site may legally write, including spellings that only become
    /// callable after an explicit `namespace import`.  The bare operator forms
    /// are exactly that case, and conflating the two inverts a real answer:
    ///
    /// ```text
    /// # oracle, tclsh 9.0.4 and 8.6.14, byte-identical
    /// info commands ::+            ;# -> {}          (empty!)
    /// + 1 2                        ;# -> invalid command name "+"
    /// info commands ::set          ;# -> ::set
    ///
    /// namespace eval ::Ops { proc + {a b} {…}; namespace export + }
    /// namespace import ::Ops::*    ;# -> OK, namespace origin ::+ is ::Ops::+
    ///
    /// namespace eval ::Foo { proc set {a b} {…}; namespace export set }
    /// namespace import ::Foo::*    ;# -> can't import command "set": already exists
    /// ```
    ///
    /// So `set` blocks an unforced import and `+` does not, even though
    /// [`Self::get`] answers `Some` for both.  A bare
    /// [`Traits::OPERATOR_COMMAND`] spelling is the post-`namespace import
    /// ::tcl::mathop::*` form and is excluded here; the namespaced
    /// `::tcl::mathop::+` spelling is a genuine member of a fresh
    /// interpreter's command table and is kept.
    ///
    /// The name must also *be* the spec's own canonical name, so `get`'s
    /// leading-`::` fallback cannot make a namespaced command answer for a
    /// global one.
    ///
    /// # Known imprecision
    ///
    /// A command a *package* provides (`::csv::split`, `::math::statistics::mean`)
    /// is declared here whether or not the script `package require`s it, so
    /// asking about a package's own namespace can over-claim.  The registry
    /// carries no "needs a `package require`" marker to gate on yet.  Reaching
    /// that case means importing *into* a package's own namespace, which is
    /// why it is documented rather than worked around.
    #[must_use]
    pub fn declares_command_at(&self, qualified_name: &str) -> bool {
        let bare = qualified_name.trim_start_matches("::");
        self.get(bare).is_some_and(|spec| {
            // `get` resolves spellings; this asks about one exact name, so the
            // spec has to be the one that *is* that command.
            if spec.name.trim_start_matches("::") != bare {
                return false;
            }
            // The namespaced `::tcl::mathop::+` is in a fresh interpreter's
            // command table; the bare `+` it can be imported to is not.
            bare.contains("::") || !spec.traits.contains(Traits::OPERATOR_COMMAND)
        })
    }

    /// The typed BPF-Tcl lowering descriptor for `name`, when `name` is a
    /// BPF-dialect command (see [`crate::bpf_op`]).  The BPF-Tcl front-end
    /// dispatches on this — never on the command name.
    #[must_use]
    pub fn bpf_op(&self, name: &str) -> Option<&'static crate::bpf_op::BpfOpSpec> {
        self.get(name).and_then(|s| s.bpf_op)
    }

    /// Find the registered spelling for a typed BPF-Tcl operation.
    ///
    /// This is the reverse lookup companion to [`Self::bpf_op`].  Tools that
    /// generate BPF-Tcl source can select an operation by its registry-owned
    /// [`crate::bpf_op::BpfOpKind`] instead of embedding a command spelling.
    /// It deliberately returns the first registered spelling: aliases must
    /// share an operation descriptor, and the canonical command specs are
    /// registered before any compatibility aliases.
    #[must_use]
    pub fn bpf_command_for(&self, kind: crate::bpf_op::BpfOpKind) -> Option<&'static str> {
        self.by_name
            .values()
            .flat_map(|specs| specs.iter())
            .find_map(|spec| (spec.bpf_op.is_some_and(|op| op.kind == kind)).then_some(spec.name))
    }

    /// Look up a command spec filtered by dialect, picking the
    /// **most-specific** visible spec (`best_visible` — §5.3's single
    /// selection rule).
    ///
    /// As with [`Self::get`], a leading `::` falls back to the bare name.
    ///
    /// A registry built for a dialect profile additionally applies that
    /// profile's operator-head exclusion ([`Self::spec_visible`]) whenever
    /// the query is about the profile's own surface — so an f5-irules query
    /// on the f5-irules registry sees exactly the specs iRules ships, no
    /// matter which consumer asks (dialect-profile-model.md §9.2).
    #[must_use]
    pub fn get_for_surface(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static CommandSpec> {
        self.by_name
            .get(name)
            .or_else(|| {
                name.strip_prefix("::")
                    .and_then(|bare| self.by_name.get(bare))
            })
            .and_then(|specs| self.best_visible(specs, dialect))
    }

    /// Resolve `head` the way this registry's own availability rules
    /// resolve it: through [`Self::get_for_surface`] against the dialect
    /// profile this registry was built for, or through the dialect-agnostic
    /// [`Self::get`] when it has no profile.
    ///
    /// The single place the "same availability rules as `get`" promise made
    /// by the profile-aware behaviour queries is implemented, so a query
    /// added later cannot quietly answer for a command the profile's
    /// dialect does not have.
    fn spec_for_this_registry(&self, head: &str) -> Option<&CommandSpec> {
        match self.profile {
            Some(profile) => self.get_for_surface(head, Some(profile.surface_query())),
            None => self.get(head),
        }
    }

    /// Whether this registry's **own dialect** has a command spelled `head`.
    ///
    /// The public face of [`Self::spec_for_this_registry`], for callers that
    /// only need the yes/no: a constant-folder deciding whether it may
    /// pre-compute a call, say. Folding is a rewrite that skips the runtime's
    /// availability gate entirely, so a fold that does not ask this question
    /// silently *adds* the command to releases that never had it — a
    /// `tcl8.4` compile folded `[dict create a 1 a 2]` to a literal instead of
    /// raising `invalid command name "dict"` (issue #1427).
    ///
    /// A profile-less registry (`build_default`) answers dialect-agnostically,
    /// exactly as [`Self::get`] does.
    #[must_use]
    pub fn has_command_in_this_dialect(&self, head: &str) -> bool {
        self.spec_for_this_registry(head).is_some()
    }

    /// Which `TclOO` method-context keyword `head` is, if it is one.
    ///
    /// The registry-first replacement for the `head == "my"` /
    /// `matches!(head, "my" | "next" | "nextto")` literals consumers used to
    /// carry (issue #1050). Every consumer that needs "is this word a method
    /// dispatch or introspection keyword, and which kind" asks here, so a
    /// dialect that gains or loses one of them propagates through the specs
    /// rather than through a walker edit.
    ///
    /// A leading `::` resolves to the bare name (consumers previously
    /// matched `"my" | "::my"` by hand), matching [`Self::get`].
    ///
    /// Dialect-aware through the registry instance itself: a registry built
    /// by `registry_for_dialect` / `registry_for_profile` answers at that
    /// profile's point, so all four keywords — every one of them 8.6-and-
    /// later — return `None` from a `tcl8.4` or `tcl8.5` registry. A
    /// profile-less registry (`CommandRegistry::build_default`) answers
    /// dialect-agnostically, exactly as [`Self::get`] does.
    ///
    /// `link` is **not** a keyword here and must not be added: it creates
    /// per-class bareword commands rather than dispatching one (issue
    /// #1026). Nor is `self`'s definer-grammar homonym — the `self` word
    /// inside an `oo::define` body is a member-grammar wrapper, resolved
    /// through [`crate::definer`], not a command head.
    #[must_use]
    pub fn method_dispatch_keyword(&self, head: &str) -> Option<MethodDispatchKind> {
        let traits = self.spec_for_this_registry(head)?.traits;
        if traits.contains(Traits::TCLOO_SELF_DISPATCH) {
            Some(MethodDispatchKind::SelfDispatch)
        } else if traits.contains(Traits::TCLOO_NEXT_CHAIN) {
            Some(MethodDispatchKind::NextChain)
        } else if traits.contains(Traits::TCLOO_INTROSPECTION) {
            Some(MethodDispatchKind::Introspection)
        } else {
            None
        }
    }

    /// Whether a bracketed command substitution `[cmd ?arg?]` denotes the
    /// current `TclOO` receiving object — so a consumer resolving what a
    /// dispatch head (`[cmd ?arg?] method`) means should treat it exactly
    /// like `my method`: the enclosing class, not a structurally-inferred
    /// type. `arg` is the substitution's own first word, `None` for a bare
    /// call (`[self]` as opposed to `[self object]`).
    ///
    /// Registry data via [`CommandSpec::self_receiver_words`]
    /// (`self`/`object` today) rather than name-matching `cmd` — a
    /// `TCLOO_INTROSPECTION` command answers `Introspection` from
    /// [`Self::method_dispatch_keyword`] (its argument is a closed
    /// subcommand set, never a method name) for every *other* word, since
    /// this is a narrower, additional fact about specific words of that
    /// same closed set, not a fourth [`MethodDispatchKind`] axis: unlike
    /// `my`, the value only dispatches once *substituted* as a command
    /// head, and unlike plain introspection, this one specific word's
    /// result is the receiver itself.
    #[must_use]
    pub fn is_self_receiver_call(&self, cmd: &str, arg: Option<&str>) -> bool {
        let Some(spec) = self.spec_for_this_registry(cmd) else {
            return false;
        };
        if spec.self_receiver_words.is_empty() {
            return false;
        }
        match arg {
            None => spec.arity.min == 0,
            Some(word) => spec.self_receiver_words.contains(&word),
        }
    }

    /// Whether `head`'s **bare** spelling resolves only from inside a
    /// `TclOO` method context — the registry-side half of issue #1026's
    /// scoping rule; see [`Traits::TCLOO_METHOD_CONTEXT`] for the oracle
    /// transcripts.
    ///
    /// Consumers pair this with their own "is this call site inside a
    /// `TclOO` method body?" fact
    /// (`tcl_compiler::analyser::scope::innermost_scope_reaches_oo_helpers`)
    /// — the registry knows *which* commands are scoped, the call site
    /// knows *where* it is, and neither needs the other's command names.
    ///
    /// Dialect-aware exactly like [`Self::method_dispatch_keyword`]: a
    /// registry built for a profile answers at that profile's point, so a
    /// `tcl8.4` registry — which has no `TclOO` at all — answers `false`
    /// for every one of them.
    ///
    /// A qualified spelling (`oo::Helpers::link`, `::oo::Helpers::link`) is
    /// a separate, unscoped spec, so it answers `false`: the command really
    /// does exist globally, and calling it outside a method is a runtime
    /// error rather than an unknown command.
    #[must_use]
    pub fn resolves_only_in_method_context(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT))
    }

    /// Whether **calling** `head` needs a real `TclOO` method invocation,
    /// not merely a frame that can resolve it — see
    /// [`Traits::TCLOO_REQUIRES_METHOD_FRAME`] for the oracle transcript.
    ///
    /// The narrower companion to [`Self::resolves_only_in_method_context`],
    /// and the two answer differently in exactly one place: a Tcl 9 class
    /// `initialise` / `initialize` body, where the whole family resolves
    /// (so `W123` must stay silent) but only `my` actually runs (so
    /// completion and hover must offer only `my`).
    ///
    /// A consumer deciding "may I offer this word here / does it hover"
    /// wants **both**: the word must resolve at the call site *and*, when
    /// this answers `true`, the call site must be a method frame rather
    /// than a bare object frame. `tcl_lsp_core::oo_dispatch` pairs them
    /// once so no consumer re-derives the rule.
    ///
    /// Dialect-aware exactly like [`Self::resolves_only_in_method_context`].
    #[must_use]
    pub fn requires_oo_method_frame(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_REQUIRES_METHOD_FRAME))
    }

    /// Resolve `word` against the registry-declared universal object-command
    /// surface.
    ///
    /// `TclOO` receiver commands are runtime values, so their source head is not
    /// itself a static registry command.  This query gives common consumers
    /// the inherited method metadata without encoding the registry entry that
    /// owns that surface.  A dialect may supply at most one such surface; an
    /// accidental duplicate is treated conservatively as no unique answer.
    #[must_use]
    pub fn object_command_method(&self, word: &str) -> Option<&SubCommand> {
        let mut matches = self.by_name.values().filter_map(|specs| {
            let spec = specs.last()?;
            spec.traits
                .contains(Traits::OBJECT_COMMAND_SURFACE)
                .then_some(spec)?
                .resolve_subcommand(word)
        });
        let method = matches.next()?;
        matches.next().is_none().then_some(method)
    }

    /// Whether `word` is a destructive operation on the universal object
    /// command surface.
    #[must_use]
    pub fn is_destructive_object_method(&self, word: &str) -> bool {
        self.object_command_method(word)
            .is_some_and(|method| method.destructive)
    }

    /// Whether `word` is a **manufacturer method** of any definer family the
    /// registry models — `create` / `new` / `createWithNamespace` for
    /// `TclOO`, `create` for snit.
    ///
    /// The union over every definer grammar, for the one consumer that has a
    /// class name but not the family it belongs to: a *pure consumer*
    /// document holding `set w [Widget create x]` where `Widget` is declared
    /// in another file (issue #1303). A consumer that knows the family must
    /// ask its grammar
    /// ([`crate::definer::DefinitionBodyGrammar::manufacturer`]) instead —
    /// that answer is exact, this one is a union.
    #[must_use]
    pub fn is_manufacturer_method(&self, word: &str) -> bool {
        self.manufacturer_methods(word)
            .any(|method| method.visibility == crate::definer::MemberVisibility::Exported)
    }

    /// Every definer family's declaration of the manufacturer method `word`
    /// — the iterator behind [`Self::is_manufacturer_method`], for a consumer
    /// that needs the layout (which argument names the instance) and not just
    /// the yes/no.
    pub fn manufacturer_methods<'a>(
        &'a self,
        word: &'a str,
    ) -> impl Iterator<Item = &'static crate::definer::ManufacturerMethod> + 'a {
        self.by_name.values().filter_map(move |specs| {
            let spec = specs.last()?;
            spec.manufacturer_methods
                .iter()
                .find(|method| method.keyword == word)
                .or_else(|| {
                    spec.definition_body
                        .and_then(|grammar| grammar.manufacturer(word))
                })
        })
    }

    /// The exact manufacturer descriptor for registered command `head` and
    /// method `word`. This covers both definition-body commands and package
    /// class factories carrying standalone manufacturer data.
    #[must_use]
    pub fn manufacturer_method(
        &self,
        head: &str,
        word: &str,
    ) -> Option<&'static crate::definer::ManufacturerMethod> {
        let spec = self.get(head)?;
        if !spec.manufacturer_methods.is_empty() {
            return spec
                .manufacturer_methods
                .iter()
                .find(|method| method.keyword == word);
        }
        spec.definition_body
            .and_then(|grammar| grammar.manufacturer(word))
    }

    /// The exact manufacturer descriptor when ordinary external dispatch
    /// through registered command `head` may reach it.
    ///
    /// This is the call-classification counterpart of
    /// [`Self::manufacturer_method`]. Consumers examining a real command
    /// invocation should normally use this method; definition-body analysis
    /// that models `self export` / `self unexport` needs the unfiltered
    /// descriptor and applies the class-local visibility changes itself.
    #[must_use]
    pub fn exported_manufacturer_method(
        &self,
        head: &str,
        word: &str,
    ) -> Option<&'static crate::definer::ManufacturerMethod> {
        self.manufacturer_method(head, word)
            .filter(|method| method.visibility == crate::definer::MemberVisibility::Exported)
    }

    /// The first constructor-payload argument for manufacturer `word`, when
    /// every definer family declaring that word agrees on the layout.
    ///
    /// Consumers that know the exact family should query its grammar
    /// directly. Scope-blind flow analyses use this conservative union: a
    /// future family reusing a keyword with a different layout makes the
    /// answer `None`, so parameter flow abstains instead of skipping the
    /// wrong structural words.
    #[must_use]
    pub fn uniform_manufacturer_constructor_args_from(&self, word: &str) -> Option<usize> {
        let mut methods = self.manufacturer_methods(word);
        let first = usize::from(methods.next()?.constructor_args_from);
        methods
            .all(|method| usize::from(method.constructor_args_from) == first)
            .then_some(first)
    }

    /// The instance-name argument for manufacturer `word`, when every
    /// definer family declaring that word agrees on the layout.
    ///
    /// `None` is deliberately ambiguous: the method may generate its own
    /// name, or different families may use different layouts. Consumers
    /// that need to distinguish those cases should use
    /// [`Self::manufacturer_methods`] and require agreement themselves.
    #[must_use]
    pub fn uniform_manufacturer_names_instance_at(&self, word: &str) -> Option<usize> {
        let mut methods = self.manufacturer_methods(word);
        let first = methods.next()?.names_instance_at?;
        methods
            .all(|method| method.names_instance_at == Some(first))
            .then_some(usize::from(first))
    }

    /// Whether `word` can conservatively identify a constructor call when a
    /// consumer knows only that the head names some class, not which definer
    /// family created it.
    ///
    /// Named manufacturer methods and family-specific bare-word naming hints
    /// are both registry data. Exact class-aware consumers should use the
    /// class grammar instead of this union.
    #[must_use]
    pub fn is_possible_class_construction_word(&self, word: &str) -> bool {
        self.is_manufacturer_method(word)
            || self.by_name.values().any(|specs| {
                specs.last().is_some_and(|spec| {
                    spec.definition_body
                        .and_then(|grammar| grammar.bare_word_construction_hint)
                        .is_some_and(|recognises| recognises(word))
                })
            })
    }

    /// Whether `head` binds bareword aliases for methods of the current
    /// object — `TclOO`'s `link`; see [`Traits::TCLOO_BINDS_METHOD_ALIAS`].
    ///
    /// The registry-first replacement for the `texts[0] == "link"` literal
    /// the analyser's class-body walk used to carry (issue #1026), and
    /// dialect-aware for the same reason [`Self::method_dispatch_keyword`]
    /// is: `link` is 9.0-core / 8.6-via-`ooutil`, so an 8.5 registry answers
    /// `false`.
    #[must_use]
    pub fn binds_method_alias(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_BINDS_METHOD_ALIAS))
    }

    /// The single spec-selection rule (§5.3, D6): among the specs of one
    /// name visible under `dialect`, pick the **most specific** — a
    /// dialect-scoped spec beats a catch-all (`surface: None`), a narrower
    /// surface beats a wider one, and among equals the
    /// *last-registered* spec wins, so curated pack overrides keep beating
    /// the data they shadow. `get_for_surface`, the iRules event
    /// cross-product, and (via `ProfileQueries::resolve_command`) the CLI
    /// snapshot all resolve through this one rule.
    fn best_visible(
        &self,
        specs: &[&'static CommandSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static CommandSpec> {
        specs
            .iter()
            .enumerate()
            .filter(|(_, s)| self.spec_visible(s, dialect))
            .max_by_key(|&(index, s)| {
                let scope_tightness =
                    std::cmp::Reverse(s.surface.map_or(u32::MAX, surface_breadth));
                (s.surface.is_some(), scope_tightness, index)
            })
            .map(|(_, s)| *s)
    }

    /// Whether `spec` is visible to any of `providers` — the coarse,
    /// version-blind twin of [`Self::spec_visible`] the static grammars use.
    ///
    /// First-paint highlighting has no resolved release to ask about, so it
    /// asks which providers the profile's language spans. The operator-head
    /// exclusion still applies: math operators are not command heads under
    /// iRules at any release.
    #[must_use]
    pub fn spec_visible_to(&self, spec: &CommandSpec, providers: &[SpecProvider]) -> bool {
        spec.surface
            .is_none_or(|rows| surface_provided_by(rows, providers))
            && self.profile.is_none_or(|profile| {
                profile.operators_as_commands
                    || !spec
                        .traits
                        .contains(crate::traits::Traits::OPERATOR_COMMAND)
            })
    }

    /// The full availability test for a point query on this registry: the
    /// spec's own surface gate, plus — when this registry was built for a
    /// profile and the query is that profile's own point — the
    /// operator-command exclusion (§9), for a profile whose math operators
    /// are not command heads.
    ///
    /// There is no disable list: availability is fully explicit in each
    /// spec's surface rows, so a sandbox-banned command such as `exec`
    /// simply never names the iRules family.
    ///
    /// Public because generators projecting a command surface for an
    /// explicit point need the same exclusion semantics `get_for_surface`
    /// applies internally.
    #[must_use]
    pub fn spec_visible(&self, spec: &CommandSpec, dialect: Option<SurfaceQuery<'_>>) -> bool {
        if !spec.supports_dialect(dialect) {
            return false;
        }
        let Some(profile) = self.profile else {
            return true;
        };
        if dialect.is_none_or(|query| query != profile.surface_query()) {
            // The query is about some other surface's availability; this
            // profile's operator-exclusion does not apply to it.
            return true;
        }
        // iRules availability is stated positively by each spec, so there
        // is no subtractive ban list — the only profile-level exclusion left
        // is the operator-command one (math operators are not command heads
        // under iRules).
        profile.operators_as_commands
            || !spec
                .traits
                .contains(crate::traits::Traits::OPERATOR_COMMAND)
    }

    /// Return all registered command names.
    pub fn command_names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().copied()
    }

    /// Return command names whose command-level descriptor selects `operation`.
    ///
    /// This uses the same target-neutral descriptor precedence as structured
    /// invocation resolution. It is intended for whole-module trust proofs that
    /// need to quantify over every registry spelling of one semantic operation.
    pub fn command_names_for_semantic_operation(
        &self,
        operation: crate::SemanticOperationId,
    ) -> impl Iterator<Item = &str> {
        self.by_name.iter().filter_map(move |(name, specs)| {
            specs
                .iter()
                .any(|spec| {
                    crate::resolved_invocation::descriptor_operation(
                        spec.semantic_operation,
                        spec.lowering_hook,
                        spec.codegen_hook,
                        spec.inline_codegen_hook,
                    ) == Some(operation)
                })
                .then_some(*name)
        })
    }

    /// The [`ObjectClassSpec`] for a `TclOO` / megawidget class named
    /// `class_name`, or `None` when it is not a registry-modelled class.
    ///
    /// For a `TclOO` class the class name is the factory command name
    /// (`oo::class create Foo` binds command `Foo`), so this resolves through
    /// the ordinary command table — no separate index.  A leading `::` falls
    /// back to the bare name, as with [`Self::get`].
    #[must_use]
    pub fn object_class(&self, class_name: &str) -> Option<&crate::spec::ObjectClassSpec> {
        self.get(class_name).and_then(|s| s.object_class)
    }

    /// The version floor this registry itself guarantees for `spec`'s owning
    /// package. A profile supplies its pinned runtime library version; a
    /// `SpecTcl` ambient-package declaration supplies the pack equivalent.
    /// When both make a claim, their strongest floor wins.
    ///
    /// Request-time consumers that have `package require` facts can raise this
    /// floor through `DocumentFloor`; registry-only lookup deliberately keeps
    /// the static, resolved-profile answer. `None` remains permissive for an
    /// unpinned package or a hand-assembled registry with no ambient claim.
    fn package_floor_for_spec(&self, spec: &CommandSpec) -> Option<&'static str> {
        let package = spec.owning_package()?;
        let profile_floor = self
            .profile
            .and_then(|profile| profile.library_floor_default(package));
        let ambient_floor = self.ambient_package_floor(package);
        match (profile_floor, ambient_floor) {
            (Some(profile), Some(ambient)) => (crate::version::compare(profile, ambient).is_lt())
                .then_some(ambient)
                .or(Some(profile)),
            (profile, ambient) => profile.or(ambient),
        }
    }

    /// The stronger of the registry's own package floor and a caller's
    /// document-resolved floor. `package_version` is intentionally supplied
    /// by the caller rather than inferred from a command spelling: an explicit
    /// `package require` is document data, while the profile/ambient floor is
    /// registry data.
    fn package_floor_for_spec_at<'a>(
        &self,
        spec: &CommandSpec,
        package_version: Option<&'a str>,
    ) -> Option<&'a str> {
        match (self.package_floor_for_spec(spec), package_version) {
            (Some(registry), Some(document)) => {
                if crate::version::compare(registry, document).is_lt() {
                    Some(document)
                } else {
                    Some(registry)
                }
            }
            (Some(registry), None) => Some(registry),
            (None, document) => document,
        }
    }

    /// Assemble the visible instance-method table for `class_name`, walking
    /// declared superclasses breadth-first.
    ///
    /// The owning *class command* supplies the lifecycle axis for each row,
    /// rather than assuming every inherited method belongs to the receiver's
    /// package. This is the object-method counterpart of normal subcommand
    /// filtering: a Tk 9.1-only widget operation must not become a hover,
    /// semantic-token, taint, or completion fact in a profile that resolves
    /// Tk 9.0. The filter is entirely descriptor-driven, so the registry knows
    /// no widget or method spellings.
    #[must_use]
    pub fn instance_methods(&self, class_name: &str) -> Vec<&'static SubCommand> {
        self.instance_methods_at(
            class_name,
            None,
            self.profile.map(tcl_dialect::DialectProfile::surface_query),
        )
    }

    /// [`Self::instance_methods`] with a document-resolved floor for the
    /// owning package and an explicit surface point.
    ///
    /// A request-time caller obtains `package_version` from its document's
    /// package-require facts for the class command, which can only raise the
    /// profile/ambient floor. Passing `None` retains the registry's static
    /// profile answer. `dialect` similarly lets an unprofiled, hand-assembled
    /// registry honour a caller's resolved command surface.
    #[must_use]
    pub fn instance_methods_at(
        &self,
        class_name: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<&'static SubCommand> {
        // FIFO so the walk is genuinely breadth-first and visits siblings in
        // declaration order — `Vec::pop` would make this a reversed-sibling
        // depth-first search, contradicting the documented contract.
        let mut queue = std::collections::VecDeque::from([class_name.to_string()]);
        let mut seen = std::collections::HashSet::new();
        let mut method_names = std::collections::HashSet::new();
        let mut visible_methods = Vec::new();
        while let Some(cls) = queue.pop_front() {
            if !seen.insert(cls.clone()) {
                continue;
            }
            // Resolve the owning command through this registry's profile,
            // then gate each of its method descriptors on that command's
            // package axis. A class absent from this profile cannot donate
            // inherited methods either.
            let Some(class_spec) = self.spec_for_this_registry(&cls) else {
                continue;
            };
            let Some(class) = class_spec.object_class else {
                continue;
            };
            let package_floor = self.package_floor_for_spec_at(class_spec, package_version);
            for candidate in class.instance_methods {
                let method_dialects = candidate.surface.or(class_spec.surface);
                if !candidate.available_for_version(package_floor)
                    || !method_dialects.is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
                {
                    continue;
                }
                // Breadth-first order implements ordinary override lookup:
                // the first declaration of a canonical name is the visible
                // one. Prefix ambiguity is resolved only after the complete
                // visible method table has been assembled.
                if method_names.insert(candidate.name) {
                    visible_methods.push(candidate);
                }
            }
            for sup in class.superclasses {
                queue.push_back((*sup).to_string());
            }
        }
        visible_methods
    }

    /// Resolve an instance method `method` on class `class_name`, walking
    /// declared superclasses breadth-first. Lifecycle-gated methods are
    /// filtered against their owning class command's resolved package floor.
    /// Returns the owning class's [`SubCommand`] method spec, or `None` when
    /// unresolved.
    #[must_use]
    pub fn instance_method(
        &self,
        class_name: &str,
        method: &str,
    ) -> Option<&crate::spec::SubCommand> {
        self.instance_method_at(
            class_name,
            method,
            None,
            self.profile.map(tcl_dialect::DialectProfile::surface_query),
        )
    }

    /// [`Self::instance_method`] with a document-resolved owning-package
    /// floor and explicit surface point. See [`Self::instance_methods_at`]
    /// for the two axes' composition.
    #[must_use]
    pub fn instance_method_at(
        &self,
        class_name: &str,
        method: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&crate::spec::SubCommand> {
        let visible_methods = self.instance_methods_at(class_name, package_version, dialect);
        let method_prefix_matching = self
            .spec_for_this_registry(class_name)
            .and_then(|spec| spec.object_class)
            .map_or(crate::abbrev::PrefixMatching::Strict, |class| {
                class.method_prefix_matching
            });
        let table = KeywordTable::from_keywords(
            visible_methods.iter().map(|candidate| Keyword {
                name: candidate.name,
                min_abbrev: candidate.min_abbrev,
            }),
            method_prefix_matching,
        );
        let canonical = table.resolve(method).unique()?;
        visible_methods
            .into_iter()
            .find(|candidate| candidate.name == canonical)
    }

    /// Resolve a concrete object-instance invocation to the same
    /// target-neutral semantic projection used for ordinary commands.
    ///
    /// `args` starts with the method word and is followed by its arguments.
    /// The class/factory command supplies the registry-owned method table, but
    /// constructor-only command traits and effects are deliberately not
    /// inherited by the instance call. A matching [`SubCommandForm`] may
    /// replace the method row's coarse traits, mutator flag, and legacy side
    /// effects for the concrete argument shape.
    ///
    /// [`SubCommandForm`]: crate::forms::SubCommandForm
    #[must_use]
    pub fn resolve_instance_invocation<'r, 'w>(
        &'r self,
        class_name: &str,
        receiver: &'w str,
        args: &'w [&'w str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        self.resolve_structured_instance_invocation(
            class_name,
            InvocationWords::literals(receiver, args),
            dialect,
        )
    }

    /// Source-aware companion to [`Self::resolve_instance_invocation`].
    ///
    /// The receiver is the invocation head. Only a literal method word can
    /// select the class method table. Form-level literal selectors likewise
    /// inspect only known literal words; a substituted operation word keeps
    /// the conservative method-row semantics rather than guessing a form.
    #[must_use]
    pub fn resolve_structured_instance_invocation<'r, 'w>(
        &'r self,
        class_name: &str,
        words: InvocationWords<'w>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        let arguments = words.arguments();
        let method_spelling = arguments.literal_at(0)?;
        let class_spec = if dialect.is_none() {
            self.get(class_name)?
        } else {
            self.get_for_surface(class_name, dialect)?
        };
        class_spec.object_class?;
        let method = self.instance_method(class_name, method_spelling)?;
        let form = pick_form(method.subcommand_forms, arguments, 1, dialect);
        let resolved_method = ResolvedSubcommand {
            spelling: method_spelling,
            canonical_name: method.name,
        };
        let subcommand = if method_spelling == method.name {
            SubcommandResolution::Exact(resolved_method)
        } else {
            SubcommandResolution::UniquePrefix(resolved_method)
        };
        Some(ResolvedInvocation::new_instance(
            words, class_spec, method, form, subcommand,
        ))
    }

    /// Resolve the [`ArgRole::CommandPrefix`] positions and appended arities for
    /// an instance-method dispatch `$obj method method_args…`.
    ///
    /// `method_args` are the words *after* the method name.  Mirrors the
    /// subcommand arm of [`Self::command_prefixes`] (static
    /// [`SubCommand::command_prefixes`] table ∪ `command_prefix_resolver` ∪
    /// command-prefix options), but keyed on the object's class + method rather
    /// than a top-level command — so `$g walk … -command cb` (option value) and
    /// `$t walkproc … cb` (trailing positional, resolver) light up the same
    /// references / call-graph / W123 / arity substrate.  Returned indices are
    /// relative to `method_args` (0 = first word after the method name).
    ///
    /// [`SubCommand::command_prefixes`]: crate::spec::SubCommand::command_prefixes
    #[must_use]
    pub fn instance_method_command_prefixes(
        &self,
        class_name: &str,
        method: &str,
        method_args: &[&str],
    ) -> Vec<(usize, AppendedArity)> {
        self.instance_method_command_prefixes_with_arguments(
            class_name,
            method,
            CommandPrefixArguments::literals(method_args),
        )
    }

    /// Source-aware companion to [`Self::instance_method_command_prefixes`].
    /// Literal-sensitive resolvers can distinguish runtime substitutions from
    /// values proved at the call site.
    #[must_use]
    pub fn instance_method_command_prefixes_structured<'w>(
        &self,
        class_name: &str,
        method: &str,
        spellings: &'w [&'w str],
        words: &'w [InvocationWord<'w>],
    ) -> Vec<(usize, AppendedArity)> {
        let Some(arguments) = CommandPrefixArguments::structured(spellings, words) else {
            return Vec::new();
        };
        self.instance_method_command_prefixes_with_arguments(class_name, method, arguments)
    }

    fn instance_method_command_prefixes_with_arguments(
        &self,
        class_name: &str,
        method: &str,
        method_args: CommandPrefixArguments<'_>,
    ) -> Vec<(usize, AppendedArity)> {
        let Some(m) = self.instance_method(class_name, method) else {
            return Vec::new();
        };
        let n = method_args.len();
        let mut out: Vec<(usize, AppendedArity)> = Vec::new();
        if let Some(resolver) = m.command_prefix_resolver {
            out.extend(
                resolver(method_args)
                    .into_iter()
                    .map(|(i, a)| (i as usize, a)),
            );
        } else {
            out.extend(m.command_prefixes.iter().map(|(i, a)| (*i as usize, *a)));
        }
        push_command_prefix_options(&mut out, m.options, method_args.spellings(), 0);
        out.retain(|&(idx, _)| idx < n);
        out
    }

    /// Whether `pkg` is a package the registry knows about — i.e. at
    /// least one registered command declares it as its
    /// [`required_package`](crate::CommandSpec::required_package).
    ///
    /// Used by the W120 (missing-`package require`) check: a `package
    /// require` of an *unknown* third-party package may itself pull in
    /// arbitrary commands (e.g. a wrapper that `package require Tk`s
    /// internally), so the analyser cannot prove a Tk/extension command
    /// is unprovided and must suppress W120.
    #[must_use]
    pub fn provides_package(&self, pkg: &str) -> bool {
        self.by_name
            .values()
            .flat_map(|specs| specs.iter())
            .any(|spec| spec.required_package == Some(pkg))
    }

    /// Return every registered [`CommandSpec`] for `name` (all dialects),
    /// in registration order. Empty when the name is unknown.
    ///
    /// This is the raw data view — resolution (which spec *wins* under a
    /// dialect) goes through [`Self::get_for_surface`]'s most-specific
    /// rule.
    #[must_use]
    pub fn specs(&self, name: &str) -> &[&'static CommandSpec] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }

    /// The taint-source colour declared by `command`'s spec, or `None`
    /// when it is not a source.
    ///
    /// Reads the compile-time [`TAINT_SOURCE_INDEX`] — a table *derived*
    /// from every command spec's [`crate::CommandSpec::taint_source`],
    /// built at compile time. It is deliberately **dialect-agnostic and
    /// independent of which dialects are loaded into this registry**:
    /// an iRules
    /// getter such as `HTTP::path` is a known source even when analysing a
    /// `tcl8.6` document whose registry never loaded the iRules commands.
    #[must_use]
    pub fn taint_source(&self, command: &str) -> Option<crate::taint::TaintColour> {
        TAINT_SOURCE_INDEX
            .iter()
            .find(|(name, _)| *name == command)
            .map(|(_, colour)| *colour)
    }

    /// Return the names of `command`'s subcommands whose traits include
    /// `t` — the subcommand-level counterpart of [`Self::commands_with_trait`].
    /// Empty when the command is unknown or has no matching subcommand.
    #[must_use]
    pub fn subcommands_with_trait(&self, command: &str, t: Traits) -> Vec<&str> {
        self.get(command).map_or_else(Vec::new, |spec| {
            spec.subcommands
                .iter()
                .filter(|s| s.traits.contains(t))
                .map(|s| s.name)
                .collect()
        })
    }

    /// Return the sorted names of every f5-irules command valid in `event`.
    ///
    /// The valid-command set: a command
    /// is valid when it supports the iRules dialect, the event is not in its
    /// `excluded_events`, and either it carries no `event_requires` or the
    /// event's [`crate::events::EventProps`] satisfy them.  Returns an empty
    /// vector for an unknown event.
    #[must_use]
    pub fn valid_irules_commands_for_event<'a>(
        &'a self,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
        bigip_version: Option<&str>,
    ) -> Vec<&'a str> {
        let Some(props) = events.get_props(event) else {
            return Vec::new();
        };
        // The declared version range for an F5-surface spec: explicit
        // introduction/removal data, or the axis baseline (15.0) for a
        // spec with none. `bigip_version: None` keeps the pre-version
        // behaviour (no filtering) so digest-stable callers opt in.
        let version_ok = |spec: &CommandSpec| {
            let Some(version) = bigip_version else {
                return true;
            };
            spec.lifecycle
                .with_baseline(tcl_dialect::VersionKey::BigipVersion.baseline_version())
                .available_at(Some(version))
        };
        let mut names: Vec<&str> = self
            .by_name
            .iter()
            .filter_map(|(name, specs)| {
                // Best spec for the dialect — the §5.3 most-specific rule,
                // matching `get_for_surface`.
                let spec =
                    self.best_visible(specs, Some(SurfaceQuery::any_release(Family::F5Irules)))?;
                if !version_ok(spec) {
                    return None;
                }
                if spec.excluded_events.contains(&event) {
                    return None;
                }
                if let Some(req) = spec.event_requires.as_ref()
                    && !crate::events::event_satisfies(props, req, event, profiles)
                {
                    return None;
                }
                Some(*name)
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// Whether `command` is legal in iRules `event` — the O(1) legality-matrix
    /// test.
    ///
    /// A command is legal when the event is known, the command supports the
    /// iRules dialect, the event is not in the command's `excluded_events`,
    /// and the command's [`crate::events::EventRequires`] (if any) are
    /// satisfied by the event's [`crate::events::EventProps`].  An unknown
    /// event is illegal for every command — an event with no props has an
    /// empty valid-command set.
    #[must_use]
    pub fn is_irules_command_legal_in_event(
        &self,
        command: &str,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> bool {
        self.is_irules_call_legal_in_event(command, &[], event, events, profiles)
    }

    /// Whether a concrete iRules command call is legal in `event`.
    ///
    /// Unlike [`Self::is_irules_command_legal_in_event`], this resolves the
    /// command's registry-declared argument-prefix event forms before applying
    /// the common event matrix. Consumers pass the words after the command
    /// name; no consumer needs command-specific subcommand knowledge.
    #[must_use]
    pub fn is_irules_call_legal_in_event(
        &self,
        command: &str,
        args: &[&str],
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> bool {
        let Some(props) = events.get_props(event) else {
            return false;
        };
        let Some(spec) =
            self.get_for_surface(command, Some(SurfaceQuery::any_release(Family::F5Irules)))
        else {
            return false;
        };
        if spec.excluded_events.contains(&event) {
            return false;
        }
        let requirements = spec.event_requirements_for_args(args);
        if !requirements.only_in.is_empty() && !requirements.only_in.contains(&event) {
            return false;
        }
        requirements
            .requires
            .is_none_or(|req| crate::events::event_satisfies(props, req, event, profiles))
    }

    /// Sorted iRules events where `command` is legal — the inverse of
    /// [`Self::valid_irules_commands_for_event`].  Used by the IRULE1001
    /// "Available in: …" hint.
    #[must_use]
    pub fn irules_events_for_command<'a>(
        &self,
        command: &str,
        events: &'a crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> Vec<&'a str> {
        self.irules_events_for_call(command, &[], events, profiles)
    }

    /// Sorted iRules events where a concrete command call is legal. See
    /// [`Self::is_irules_call_legal_in_event`] for the argument-form-aware
    /// contract resolution.
    #[must_use]
    pub fn irules_events_for_call<'a>(
        &self,
        command: &str,
        args: &[&str],
        events: &'a crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> Vec<&'a str> {
        let mut names: Vec<&str> = events
            .all_event_names()
            .into_iter()
            .filter(|event| {
                self.is_irules_call_legal_in_event(command, args, event, events, profiles)
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// Resolve full metadata for an iRules `event` — the engine behind
    /// `f5 irule event-info`.
    ///
    /// The event name is upper-cased and trimmed. `known` /
    /// `valid_commands` come from this registry (the cross-product is the
    /// same as [`Self::valid_irules_commands_for_event`]); the side /
    /// transport / implied-profiles / description / multiplicity come from
    /// `events`. `deprecated` is always `false` — the `when`-argument-value
    /// detail path carries no "deprecated" markers (independent of
    /// [`crate::events::EventProps::deprecated`]).
    #[must_use]
    pub fn event_info(
        &self,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
        bigip_version: Option<&str>,
    ) -> EventInfo {
        let name = event.trim().to_uppercase();
        let target = bigip_version
            .or(tcl_dialect::VersionKey::BigipVersion.default_version())
            .unwrap_or("16.1.0");
        let known =
            !name.is_empty() && events.is_known(&name) && events.event_available_at(&name, target);
        let valid_commands: Vec<String> = if known {
            self.valid_irules_commands_for_event(&name, events, profiles, Some(target))
                .into_iter()
                .map(ToOwned::to_owned)
                .collect()
        } else {
            Vec::new()
        };
        let props = events.get_props(&name);
        let lifecycle = events.event_lifecycle(&name).unwrap_or_default();
        EventInfo {
            lifecycle,
            lifecycle_state: lifecycle.state_at(Some(target)),
            known,
            multiplicity: events.multiplicity(&name),
            description: events.description(&name).unwrap_or("").to_owned(),
            side: props.map_or("unknown", crate::events::EventProps::side_label),
            // "No transport" is modelled as `None`, not an empty string.
            transport: props.and_then(|p| (!p.transport.is_empty()).then(|| p.transport.join("/"))),
            implied_profiles: {
                let mut v: Vec<&'static str> = props
                    .map(|p| p.implied_profiles.to_vec())
                    .unwrap_or_default();
                v.sort_unstable();
                v
            },
            event: name,
            valid_commands,
        }
    }

    /// The symbol-definer descriptor for `name` in `dialect`, if the command
    /// binds a navigable definition name (a `tcltest::test` case, …).
    ///
    /// A leading `::` falls back to the bare name, as with [`Self::get`].  The
    /// analyser and signature scanner consult this to record outline symbols
    /// generically — the argument index and outline category come from the
    /// [`crate::symbol_def::SymbolDef`], never from a command-name check.
    #[must_use]
    pub fn defines_symbol(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&crate::symbol_def::SymbolDef> {
        self.get_for_surface(name, dialect)
            .and_then(|s| s.defines_symbol.as_ref())
    }

    /// Every command name that declares a [`crate::symbol_def::SymbolDef`] in
    /// any registered spec, for consumers (the signature scanner) that
    /// precompute a symbol-definer lookup set rather than querying per-call.
    #[must_use]
    pub fn commands_defining_symbols(&self) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .iter()
                    .any(|s| s.defines_symbol.is_some())
                    .then_some(*name)
            })
            .collect()
    }

    /// Return all command specs whose traits include `t`.
    #[must_use]
    pub fn commands_with_trait(&self, t: Traits) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs.last().filter(|s| s.traits.contains(t)).map(|_| *name)
            })
            .collect()
    }

    /// Every instance method some registered metaclass **generates** from a
    /// class's declared `property` members — the union of
    /// [`DefinitionBodyGrammar::property_accessor_methods`] over the
    /// metaclasses that configure by property
    /// ([`Traits::CONFIGURES_BY_PROPERTY`]).
    ///
    /// The dialect-wide answer, for a consumer holding a class whose own
    /// metaclass it could not resolve to a grammar — a user metaclass derived
    /// from `oo::configurable` — but which demonstrably declares `property`
    /// members. Prefer the class's own grammar
    /// (`CommandSpec::definition_body`) whenever it resolves; this is the
    /// fallback that keeps such a class from being told its generated
    /// accessor does not exist (issue #1362), without any consumer spelling
    /// `configure` itself.
    ///
    /// Sorted and deduplicated. Empty for a dialect with no such metaclass
    /// (every pre-9.0 Tcl, where `oo::configurable` does not exist).
    ///
    /// [`DefinitionBodyGrammar::property_accessor_methods`]: crate::definer::DefinitionBodyGrammar::property_accessor_methods
    #[must_use]
    pub fn property_accessor_methods(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = self
            .commands_with_trait(Traits::CONFIGURES_BY_PROPERTY)
            .into_iter()
            .filter_map(|name| self.get(name))
            .filter_map(|spec| spec.definition_body)
            .flat_map(|grammar| grammar.property_accessor_methods.iter().copied())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// How a call to `name` binds a variable to an **object handle**, when it
    /// does — [`CommandSpec::binds_handle`], resolved through [`Self::get`] so
    /// the explicitly global spelling (`::set`) answers identically to the
    /// bare one (issue #1185).
    ///
    /// The member-body-only installers a class system injects (snit's
    /// `install NAME using TYPE …`) are **not** here: they are not global
    /// commands, and live on
    /// [`crate::definer::DefinitionBodyGrammar::member_body_commands`] —
    /// enumerate them with [`Self::member_body_handle_bindings`].
    ///
    /// [`CommandSpec::binds_handle`]: crate::spec::CommandSpec::binds_handle
    #[must_use]
    pub fn handle_binding(
        &self,
        name: &str,
    ) -> Option<&'static crate::handle_binding::HandleBindingSpec> {
        self.get(name).and_then(|spec| spec.binds_handle)
    }

    /// Every member-body command that binds an object handle, as
    /// `(word, layout)` pairs collected from the definition-body grammars this
    /// registry's definers carry.
    ///
    /// These words exist only inside a class system's member bodies (snit's
    /// `install`), so they deliberately have no global `CommandSpec` — a
    /// consumer builds a small lookup from this list once per document instead
    /// of naming the keyword.  Deduplicated: the snit `type` and `widget`
    /// grammars share one member-body command set.
    #[must_use]
    pub fn member_body_handle_bindings(
        &self,
    ) -> Vec<(&'static str, crate::handle_binding::HandleBindingSpec)> {
        let mut out: Vec<_> = self
            .by_name
            .values()
            .filter_map(|specs| specs.last())
            .filter_map(|spec| spec.definition_body)
            .flat_map(|grammar| grammar.member_body_commands.iter())
            .filter_map(|cmd| cmd.binds_handle.map(|layout| (cmd.name, layout)))
            .collect();
        out.sort_unstable_by_key(|(name, _)| *name);
        out.dedup_by_key(|(name, _)| *name);
        out
    }

    /// Whether `name` **writes or modifies** the variable named by its
    /// first argument (`set` / `append` / `lappend` / `incr` / `lset`):
    /// [`Traits::FIRST_ARG_VARNAME`] minus the destroy-only `unset`
    /// ([`Traits::DESTROYS_VARIABLE`]).
    ///
    /// The single membership query for the write-command consumers —
    /// loop-bound checks, dead-store cancellation, embedded-script def
    /// collection, catch-body out-vars — which previously each kept a
    /// hardcoded (and mutually inconsistent) name set.
    #[must_use]
    pub fn writes_first_arg_variable(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FIRST_ARG_VARNAME)
                && !s.traits.contains(Traits::DESTROYS_VARIABLE)
        })
    }

    /// Whether `name` **read-modify-writes** the variable named by its
    /// first argument (`append` / `lappend` / `incr` / `lset` — not the
    /// whole-value `set`): [`Traits::FIRST_ARG_VARNAME`] ∧
    /// [`Traits::READS_BEFORE_WRITE`].
    ///
    /// Drives the minifier's RMW target protection: a name-compaction
    /// must not rename a variable whose current value an RMW command is
    /// about to fold into, while a plain `set` target is rename-safe.
    #[must_use]
    pub fn rmw_first_arg_variable(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FIRST_ARG_VARNAME)
                && s.traits.contains(Traits::READS_BEFORE_WRITE)
        })
    }

    /// Whether `name` is a core Tcl built-in carrying the
    /// [`Traits::BYTE_COMPILED`] trait — i.e. the minifier must not
    /// rewrite this command head to a `$var` alias.  Checks every
    /// registered spec for the name (not just the dialect-preferred
    /// one) so the core stamp is honoured even when a dialect layers
    /// an additional spec under the same name.
    #[must_use]
    pub fn is_byte_compiled(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::BYTE_COMPILED))
        })
    }

    /// Whether `name` is unsafe in sandboxed dialects — it allows
    /// context escalation (`uplevel`, `history`).  Drives the IRULE2003
    /// "unsafe iRules command" check.  Checks every spec registered
    /// under the name.
    #[must_use]
    pub fn is_unsafe(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.unsafe_command))
    }

    /// The [`crate::traits::UNIT_LINKAGE_TRAITS`] this concrete invocation
    /// carries — the registry's answer to "does this command widen the set
    /// of callers beyond the file it appears in?".
    ///
    /// `PROVIDES_PACKAGE` (`package provide` / `ifneeded`) and
    /// `EXPORTS_COMMAND` (`namespace export`, `namespace ensemble`) say the
    /// file publishes an API surface; `LOADS_EXTERNAL_UNIT` (`source`,
    /// `load`, `package require`, `auto_load`, `auto_import`, `namespace
    /// import`) says another unit's script runs in this interpreter and can
    /// call back in.  Any of the three sinks the "every caller of this
    /// file's procs is in this file" assumption that
    /// `tcl_compiler::unit_scope`'s interprocedural call-site seed rests on
    /// (issue #977).
    ///
    /// Resolved through [`Self::resolve_call`], so the subcommand word is
    /// honoured (`package provide` is a boundary, `package names` is not)
    /// and `spec.traits | sub.traits` composes exactly as
    /// [`crate::spec::SubCommand::traits`] documents. An unknown command
    /// carries no linkage — a user proc named `source` is a user proc.
    #[must_use]
    pub fn unit_linkage(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Traits {
        self.invocation_traits(name, args, dialect)
            .intersection(crate::traits::UNIT_LINKAGE_TRAITS)
    }

    /// Every trait this concrete invocation carries — `spec.traits |
    /// sub.traits`, composed exactly as [`crate::spec::SubCommand::traits`]
    /// documents.
    ///
    /// The invocation-level counterpart of `self.get(name).traits`: a
    /// subcommand's traits are *additive* over its parent's, so a consumer
    /// asking a trait question about a compound command must compose them.
    /// `namespace eval` / `namespace inscope` / `interp eval` carry the
    /// eval-family traits ([`Traits::EVALUATES_CODE`],
    /// [`Traits::SCRIPT_CONCATENATES_ARGS`]) on the **subcommand**, not on
    /// the `namespace` / `interp` spec, so a parent-only trait test silently
    /// misses them.
    ///
    /// Resolved through [`Self::resolve_call`], so the subcommand word is
    /// honoured; pass `None` to skip dialect gating (the plain
    /// [`Self::get`] lookup) when the question is "what shape is this
    /// command" rather than "is it available here". An unknown command
    /// carries no traits.
    #[must_use]
    pub fn invocation_traits(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Traits {
        let Some(resolved) = self.resolve_call(name, args, dialect) else {
            return Traits::empty();
        };
        resolved.spec.traits | resolved.sub.map_or_else(Traits::empty, |sub| sub.traits)
    }

    /// When the script at 0-based argument `index` runs relative to this
    /// concrete invocation.
    ///
    /// Script-valued options carry an exact per-value [`ScriptTiming`] fact, so
    /// one invocation may safely mix immediate, deferred, and reference-only
    /// executable text. A command or subcommand [`ScriptTimingResolver`]
    /// supplies the same exact fact for positional forms whose timing depends
    /// on their written arguments.
    /// Invocation-wide [`Traits::DEFERS_BODY`] remains the compatibility
    /// fallback. `None` means `index` is not a supplied executable position.
    ///
    /// [`ScriptTiming`]: crate::hover::ScriptTiming
    /// [`ScriptTimingResolver`]: crate::spec::ScriptTimingResolver
    #[must_use]
    pub fn script_timing(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::hover::ScriptTiming> {
        let resolved = self.resolve_invocation(name, args, dialect)?;
        let spec = if dialect.is_none() {
            self.get(resolved.canonical_command)?
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)?
        };
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        let is_executable = ArgRole::ALL
            .iter()
            .copied()
            .filter(|role| role.has_script_timing())
            .any(|role| self.arg_indices_for_role(name, args, role).contains(&index));
        if !is_executable {
            return None;
        }

        let (timing_resolver, offset) = sub.map_or((spec.script_timing_resolver, 0usize), |sub| {
            (sub.script_timing_resolver, 1usize)
        });
        if let Some(resolve) = timing_resolver
            && let Some(relative) = index.checked_sub(offset)
            && let Some((_, timing)) = resolve(args.get(offset..).unwrap_or(&[]))
                .into_iter()
                .find(|(position, _)| usize::from(*position) == relative)
        {
            return Some(timing);
        }

        // An invocation-sensitive resolver is the exact fact and therefore
        // precedes an option's static default. This matters for APIs such as
        // `bibtex::parse`: the same callback option runs synchronously for an
        // in-memory input but is stored when `-channel` selects background
        // parsing. Resolver silence deliberately falls through to the option.
        let options = resolved.semantics.options.base;
        let scan_start = resolved.semantics.argument_offset;
        if let Some(timing) = option_script_timing_at(options, args, scan_start, index) {
            return Some(timing);
        }

        Some(if resolved.semantics.traits.contains(Traits::DEFERS_BODY) {
            crate::hover::ScriptTiming::Deferred
        } else {
            crate::hover::ScriptTiming::SameInvocation
        })
    }

    /// User-controlled values that this concrete deferred callback host
    /// substitutes into argument `index` before Tcl evaluates it.
    ///
    /// This is deliberately a registry query rather than a Tk-specific
    /// compiler table. It unifies positional callback bodies (`bind`) and
    /// option values (`entry -validatecommand`), and only returns a non-empty
    /// list when the selected executable position is declared to receive an
    /// external value. Framework metadata is not inferred as taint.
    #[must_use]
    pub fn callback_taint_inputs(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let Some(resolved) = self.resolve_invocation(name, args, dialect) else {
            return &[];
        };
        let Some(spec) = (if dialect.is_none() {
            self.get(resolved.canonical_command)
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)
        }) else {
            return &[];
        };
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        let (table, offset) = sub.map_or((spec.callback_taint_inputs, 0usize), |sub| {
            (sub.callback_taint_inputs, 1usize)
        });
        let options = resolved.semantics.options.base;
        let scan_start = resolved.semantics.argument_offset;
        if let Some(inputs) = option_callback_taint_inputs_at(options, args, scan_start, index) {
            return if option_script_timing_at(options, args, scan_start, index)
                == Some(crate::hover::ScriptTiming::Deferred)
            {
                inputs
            } else {
                &[]
            };
        }
        if self
            .script_timing(name, args, index, dialect)
            .is_none_or(|timing| timing != crate::hover::ScriptTiming::Deferred)
        {
            return &[];
        }
        let Some(relative) = index.checked_sub(offset) else {
            return &[];
        };
        table
            .iter()
            .find_map(|(at, inputs)| (usize::from(*at) == relative).then_some(*inputs))
            .unwrap_or(&[])
    }

    /// User-controlled values that a resolved instance method substitutes
    /// into argument `index` before evaluating its deferred callback.
    ///
    /// `args` starts with the instance method word. The query is the
    /// object-dispatch counterpart of [`Self::callback_taint_inputs`]: it
    /// resolves the receiver class and method through the same registry table,
    /// then consults the method's positional callback table or its declared
    /// options. A method form carrying [`Traits::CONFIGURES_INSTANCE_OPTIONS`]
    /// inherits the owning class's option table, which is how widget
    /// `configure` setters expose constructor options without a command-name
    /// branch. Unknown/dynamic methods and non-deferred positions abstain.
    #[must_use]
    pub fn instance_callback_taint_inputs(
        &self,
        class_name: &str,
        receiver: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let Some(resolved) = self.resolve_instance_invocation(class_name, receiver, args, dialect)
        else {
            return &[];
        };
        let Some(method_spelling) = args.first() else {
            return &[];
        };
        let Some(method) = self.instance_method(class_name, method_spelling) else {
            return &[];
        };
        let Some(relative) = index.checked_sub(1) else {
            return &[];
        };

        let options = if resolved
            .semantics
            .traits
            .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
        {
            let class_spec = if dialect.is_none() {
                self.get(class_name)
            } else {
                self.get_for_surface(class_name, dialect)
            };
            let Some(class_spec) = class_spec else {
                return &[];
            };
            class_spec.options
        } else {
            method.options
        };
        let option_inputs = option_callback_taint_inputs_at(options, args, 1, index);
        if let Some(inputs) = option_inputs {
            return if option_script_timing_at(options, args, 1, index)
                == Some(crate::hover::ScriptTiming::Deferred)
            {
                inputs
            } else {
                &[]
            };
        }

        let timing = method
            .script_timing_resolver
            .and_then(|resolve| {
                resolve(args.get(1..).unwrap_or(&[]))
                    .into_iter()
                    .find_map(|(at, timing)| (usize::from(at) == relative).then_some(timing))
            })
            .unwrap_or_else(|| {
                if resolved.semantics.traits.contains(Traits::DEFERS_BODY) {
                    crate::hover::ScriptTiming::Deferred
                } else {
                    crate::hover::ScriptTiming::SameInvocation
                }
            });
        if timing != crate::hover::ScriptTiming::Deferred {
            return &[];
        }
        method
            .callback_taint_inputs
            .iter()
            .find_map(|(at, inputs)| (usize::from(*at) == relative).then_some(*inputs))
            .unwrap_or(&[])
    }

    /// Variable frame of the option value at 0-based argument `index`.
    ///
    /// `None` means that `index` is not a variable-name option value. The
    /// returned scope is registry data and therefore applies uniformly to
    /// lowering, SSA, and taint analysis without command-name branches.
    #[must_use]
    pub fn option_variable_scope(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::hover::VariableScope> {
        let resolved = self.resolve_call(name, args, dialect)?;
        let (options, scan_start) = resolved
            .sub
            .map_or((resolved.spec.options, 0), |sub| (sub.options, 1));
        option_variable_scope_at(options, args, scan_start, index)
    }

    /// Classify one body argument of a typed control-flow invocation.
    /// Dynamic clause grammar stays here beside the registry hook, so a
    /// consumer never recognises a command or structural keyword by name.
    #[must_use]
    pub fn control_arm_semantics(
        &self,
        name: &str,
        args: &[&str],
        body_index: usize,
    ) -> Option<ControlArmSemantics> {
        use crate::hooks::LoweringHookId;
        let resolved = self.resolve_call(name, args, None)?;
        match resolved.lowering_hook? {
            LoweringHookId::If => {
                if (resolved.spec.clause_shape_check?)(args).is_some() {
                    return None;
                }
                self.arg_indices_for_role(name, args, ArgRole::Body)
                    .contains(&body_index)
                    .then_some(ControlArmSemantics::Selected)
            }
            LoweringHookId::Switch => Some(ControlArmSemantics::Selected),
            LoweringHookId::NamespaceEval => Some(ControlArmSemantics::FrameBoundary),
            // `apply`'s lambda runs **now**, in a fresh procedure frame that
            // inherits none of the caller's locals — a frame boundary, and the
            // only [`ArgRole::LambdaLiteral`] position in the registry today
            // (issue #1656). The role is one level removed from a body (the
            // argument is the `{argList body ?ns?}` *list*), so the index is
            // gated on the role rather than assumed, exactly as the `if` arm
            // above gates on `ArgRole::Body`.
            //
            // The boundary is not symmetric with `namespace eval`'s on one
            // point, and a consumer reading completion out of this descriptor
            // should know it: a `return` inside the lambda completes the
            // *lambda*, so control does reach the statement after the `apply`,
            // whereas `namespace eval`'s script propagates its `return` to the
            // enclosing procedure (tclsh 8.6.16 / 9.0.4 agree on both). Errors
            // propagate from either. The compiler's fall-through walk does not
            // separate an early *normal* completion from an abnormal one — a
            // limit recorded on PR #1652 — so it reads a lambda that returns
            // as one it cannot walk past, which is conservative and sound.
            LoweringHookId::Apply => self
                .arg_indices_for_role(name, args, ArgRole::LambdaLiteral)
                .contains(&body_index)
                .then_some(ControlArmSemantics::FrameBoundary),
            LoweringHookId::Catch => Some(ControlArmSemantics::CompletionBoundary),
            // A collection loop iterates a value the invocation itself
            // supplies, so it runs a *finite* number of times: it terminates
            // whenever its body does. A condition loop's trip count depends on
            // state this descriptor cannot read, so it stays uncertain.
            LoweringHookId::Foreach | LoweringHookId::Lmap | LoweringHookId::ForeachLine => {
                Some(ControlArmSemantics::BoundedIteration)
            }
            LoweringHookId::For | LoweringHookId::While => Some(ControlArmSemantics::Uncertain),
            LoweringHookId::Try => try_control_arms(args)?
                .into_iter()
                .find_map(|(idx, semantics)| (idx == body_index).then_some(semantics)),
            _ => None,
        }
    }

    /// Whether a typed control invocation's complete grammar is valid.
    /// `None` means the command has no typed control grammar.
    #[must_use]
    pub fn control_invocation_valid(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<bool> {
        use crate::hooks::LoweringHookId;
        let resolved = self.resolve_call(name, args, dialect)?;
        let hook = resolved.lowering_hook?;
        match hook {
            LoweringHookId::If => Some((resolved.spec.clause_shape_check?)(args).is_none()),
            LoweringHookId::Switch => Some(self.case_invocation(name, args, dialect).is_some()),
            LoweringHookId::Try => Some(try_control_arms(args).is_some()),
            LoweringHookId::NamespaceEval
            | LoweringHookId::Catch
            | LoweringHookId::For
            | LoweringHookId::While
            | LoweringHookId::Foreach
            | LoweringHookId::Lmap
            | LoweringHookId::ForeachLine => Some(true),
            _ => None,
        }
    }

    /// Parse a case-list invocation using only options available in this
    /// registry profile. This is the dialect-aware entry point for consumers:
    /// the descriptor owns the layout, while [`crate::ProfileQueries`] owns
    /// option availability and value arity.
    #[must_use]
    pub fn case_invocation(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<(crate::spec::CaseListSpec, crate::spec::CaseInvocation)> {
        let resolved = self.resolve_call(name, args, dialect)?;
        let case = resolved.spec.case_list?;
        let options = self.profile().map_or_else(
            || resolved.spec.option_specs(dialect),
            |profile| {
                crate::profile_queries::ProfileQueries::available_option_specs(
                    profile,
                    resolved.spec,
                )
            },
        );
        let effective_dialect = self
            .profile()
            .map_or(dialect, |profile| Some(profile.surface_query()));
        Some((*case, case.invocation(args, &options, effective_dialect)?))
    }

    /// Classify whether a concrete invocation falls through, returns a normal
    /// procedure result, or terminates with another completion code.
    #[must_use]
    pub fn invocation_completion(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> InvocationCompletion {
        use crate::hooks::LoweringHookId;
        let Some(resolved) = self.resolve_call(name, args, dialect) else {
            return InvocationCompletion::Unknown;
        };
        let positional_len = args
            .len()
            .saturating_sub(usize::from(resolved.sub.is_some()));
        let invocation_arity = u16::try_from(positional_len).unwrap_or(u16::MAX);
        if !resolved.arity().accepts(invocation_arity) {
            return InvocationCompletion::Unknown;
        }
        if resolved.lowering_hook == Some(LoweringHookId::Return) {
            let mut i = 0usize;
            let mut code = Some(true);
            let mut level = Some(1_i64);
            while let Some(word) = args.get(i).copied() {
                match word {
                    "--" => {
                        i += 1;
                        break;
                    }
                    "-code" => {
                        let Some(value) = args.get(i + 1).copied() else {
                            break;
                        };
                        code = match value {
                            "ok" | "0" => Some(true),
                            "error" | "return" | "break" | "continue" | "1" | "2" | "3" | "4" => {
                                Some(false)
                            }
                            value if value.parse::<i64>().is_ok() => Some(false),
                            _ => None,
                        };
                        i += 2;
                    }
                    "-level" => {
                        let Some(value) = args.get(i + 1).copied() else {
                            break;
                        };
                        level = value.parse::<i64>().ok().filter(|value| *value >= 0);
                        i += 2;
                    }
                    "-options" | "-errorcode" | "-errorinfo" | "-errorstack" => {
                        if args.get(i + 1).is_none() {
                            break;
                        }
                        return InvocationCompletion::Unknown;
                    }
                    _ if word.starts_with('-') => {
                        if args.get(i + 1).is_none() {
                            break;
                        }
                        return InvocationCompletion::Unknown;
                    }
                    _ => break,
                }
            }
            if args.len().saturating_sub(i) > 1 {
                return InvocationCompletion::Unknown;
            }
            if code == Some(false) {
                return InvocationCompletion::Terminates;
            }
            if code.is_none() || level.is_none() {
                return InvocationCompletion::Unknown;
            }
            if level == Some(0) {
                return InvocationCompletion::FallsThrough;
            }
            let result = (i < args.len()).then_some(i);
            return InvocationCompletion::ReturnsResult(result);
        }

        let traits =
            resolved.spec.traits | resolved.sub.map_or_else(Traits::empty, |sub| sub.traits);
        if traits.intersects(
            Traits::TERMINATES_BLOCK
                | Traits::BREAKS_LOOP
                | Traits::CONTINUES_LOOP
                | Traits::REPLACES_FRAME,
        ) {
            InvocationCompletion::Terminates
        } else {
            InvocationCompletion::FallsThrough
        }
    }

    /// Return an exact Tcl completion code (or process exit) for a concrete,
    /// literal invocation when registry data and return options establish one.
    ///
    /// This is intentionally narrower than [`Self::invocation_completion`]:
    /// dynamic `return -options $dict`, dynamic `-code`/`-level` values, and
    /// ordinary calls with only a conservative completion descriptor return
    /// `None` rather than inventing a handler edge.
    #[must_use]
    pub fn exact_invocation_completion(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ExactInvocationCompletion> {
        self.exact_invocation_completion_words(
            name,
            crate::invocation_words::InvocationArguments::literals(args),
            dialect,
        )
    }

    /// Source-aware counterpart to [`Self::exact_invocation_completion`].
    /// Dynamic words remain dynamic; literal brace words retain their literal
    /// value rather than being mistaken for a variable spelling.
    #[must_use]
    pub fn exact_invocation_completion_words(
        &self,
        name: &str,
        args: crate::invocation_words::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ExactInvocationCompletion> {
        // An expansion can alter the final argv shape.  A descriptor for a
        // well-formed call (notably `exit`) must not override that unknown
        // arity, because an expanded call may instead raise TCL_ERROR.
        let argument_len = args.exact_argv_len()?;
        let literal_args = args.literal_values();
        // A non-expanded dynamic argument still has a known argv count.  It
        // cannot select a value-dependent subcommand, but a flat command such
        // as `error $message` retains its registry-owned exact completion.
        let resolved = self.resolve_call(name, literal_args.as_deref().unwrap_or(&[]), dialect)?;
        let positional_len = argument_len.saturating_sub(usize::from(resolved.sub.is_some()));
        if !resolved
            .arity()
            .accepts(u16::try_from(positional_len).unwrap_or(u16::MAX))
        {
            // Tcl validates command arity before executing its declared
            // effect.  A known call with an exact invalid shape is therefore
            // an exact, catchable TCL_ERROR, not an unknown normal path.
            return Some(ExactInvocationCompletion::Tcl(
                crate::completion::CompletionCode::Error,
            ));
        }
        if resolved.lowering_hook == Some(LoweringHookId::Return) {
            let profile = self.profile()?;
            return match exact_return_completion(
                args,
                tcl_syntax::number::Numbers::of_profile(Some(profile)),
            ) {
                ExactReturnCompletion::Completion(code) => {
                    Some(ExactInvocationCompletion::Tcl(code))
                }
                ExactReturnCompletion::StaticError => Some(ExactInvocationCompletion::Tcl(
                    crate::completion::CompletionCode::Error,
                )),
                ExactReturnCompletion::Dynamic => None,
            };
        }
        let descriptor = resolved
            .form
            .and_then(|form| form.completion)
            .or(resolved.sub.and_then(|sub| sub.completion))
            .or(resolved.spec.completion);
        if descriptor.is_some_and(|descriptor| {
            descriptor.value_semantics
                == crate::completion::CompletionValueSemantics::ProcessExitStatus
        }) {
            return match exact_process_exit_completion(
                args,
                tcl_syntax::number::Numbers::of_profile(self.profile()),
                self.runtime_version(),
            ) {
                ExactProcessExitCompletion::ProcessExit => {
                    Some(ExactInvocationCompletion::ProcessExit)
                }
                ExactProcessExitCompletion::StaticError => Some(ExactInvocationCompletion::Tcl(
                    crate::completion::CompletionCode::Error,
                )),
                ExactProcessExitCompletion::Dynamic => None,
            };
        }
        let traits =
            resolved.spec.traits | resolved.sub.map_or_else(Traits::empty, |sub| sub.traits);
        if traits.contains(Traits::TERMINATES_PROCESS) {
            return Some(ExactInvocationCompletion::ProcessExit);
        }
        let descriptor = descriptor?;
        let crate::completion::CompletionCodeDomain::Exact(codes) = descriptor.codes else {
            return None;
        };
        (codes.len() == 1).then_some(ExactInvocationCompletion::Tcl(codes[0]))
    }

    /// Return source-aware completion knowledge without exposing return-option
    /// parsing to a consumer such as the diagram renderer.
    #[must_use]
    pub fn invocation_completion_knowledge(
        &self,
        name: &str,
        args: crate::invocation_words::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<InvocationCompletionKnowledge> {
        if let Some(exact) = self.exact_invocation_completion_words(name, args, dialect) {
            return Some(InvocationCompletionKnowledge::Exact(exact));
        }
        let literals = args.literal_values().unwrap_or_default();
        let resolved = self.resolve_call(name, &literals, dialect)?;
        let descriptor = resolved
            .form
            .and_then(|form| form.completion)
            .or(resolved.sub.and_then(|sub| sub.completion))
            .or(resolved.spec.completion);
        if descriptor.is_some_and(|descriptor| {
            descriptor.value_semantics
                == crate::completion::CompletionValueSemantics::ProcessExitStatus
        }) {
            // The exact path above already returned omitted / literal-valid
            // process exits and literal-invalid TCL_ERROR. Anything left is
            // source-dynamic (including `{*}` with an unknown argv length):
            // Tcl can only either parse a status and exit or reject the
            // invocation; it can never fall through normally.
            return Some(InvocationCompletionKnowledge::ExitOrError);
        }
        if resolved.lowering_hook != Some(LoweringHookId::Return) {
            return None;
        }
        let dynamic_code = (0..args.len()).any(|index| {
            args.literal_at(index) == Some("-code") && args.literal_at(index + 1).is_none()
        });
        let has_level = (0..args.len()).any(|index| args.literal_at(index) == Some("-level"));
        let has_options = (0..args.len()).any(|index| args.literal_at(index) == Some("-options"));
        Some(if dynamic_code && !has_level && !has_options {
            InvocationCompletionKnowledge::DynamicReturnOrError
        } else {
            InvocationCompletionKnowledge::Dynamic
        })
    }

    /// Whether `name` is valid only at the top level of an iRule script
    /// (`when`, `proc`, `priority`, `timing`).  Drives the IRULE5006 /
    /// IRULE5007 placement checks ([`Traits::IRULES_TOP_LEVEL_ONLY`]).
    #[must_use]
    pub fn is_irules_top_level_only(&self, name: &str) -> bool {
        self.get(name)
            .is_some_and(|spec| spec.traits.contains(Traits::IRULES_TOP_LEVEL_ONLY))
    }

    /// Apply the iRules command-placement contract to one resolved head.
    ///
    /// The only legal file-level forms carry
    /// [`Traits::IRULES_TOP_LEVEL_ONLY`]. Every other invocation, including a
    /// user-defined command absent from this registry, is executable and must
    /// occur inside an event or procedure. An invalid nested top-level body is
    /// not cascaded: its enclosing executable command already carries the
    /// placement finding, while declaration forms nested there remain
    /// independently reportable.
    #[must_use]
    pub fn irules_command_placement(
        &self,
        name: &str,
        context: crate::events::IrulesExecutionContext,
    ) -> crate::events::IrulesCommandPlacement {
        use crate::events::{IrulesCommandPlacement, IrulesExecutionContext};

        if self.is_irules_top_level_only(name) {
            return if context == IrulesExecutionContext::TopLevel {
                IrulesCommandPlacement::Allowed
            } else {
                IrulesCommandPlacement::RequiresTopLevel
            };
        }
        match context {
            IrulesExecutionContext::TopLevel => IrulesCommandPlacement::RequiresEventOrProcedure,
            IrulesExecutionContext::EventBody
            | IrulesExecutionContext::ProcedureBody
            | IrulesExecutionContext::InvalidNestedBody => IrulesCommandPlacement::Allowed,
        }
    }

    /// Validate and classify one invocation on iRules' top-level declaration
    /// surface. Arity, argument layout, declaration traits, and known-event
    /// membership are decided here so compiler/tooling consumers cannot admit
    /// different executable regions.
    #[must_use]
    pub fn irules_top_level_declaration(
        &self,
        name: &str,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
        events: &crate::events::EventRegistry,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let declaration = self.irules_top_level_declaration_shape(name, arguments)?;
        match &declaration {
            Declaration::Event { event, .. } if !events.is_known(event) => None,
            Declaration::Event { .. }
            | Declaration::Procedure { .. }
            | Declaration::Priority { .. }
            | Declaration::Timing { .. } => Some(declaration),
        }
    }

    /// Validate an iRules file-level declaration without requiring an event
    /// selector to be known.
    ///
    /// This is the source-aware declaration owner used by lowering, symbols,
    /// diagnostics, and inventories. It deliberately accepts an unknown event
    /// with a valid declaration shape so diagnostics can report that selector;
    /// executable inventories use [`Self::irules_top_level_declaration`],
    /// which additionally requires the event registry membership.
    #[must_use]
    pub fn irules_top_level_declaration_shape(
        &self,
        name: &str,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let args = arguments.values();
        let spec = self.get(name)?;
        let argument_count = u16::try_from(args.len()).ok()?;
        if !spec.arity.accepts(argument_count)
            || self.irules_command_placement(name, crate::events::IrulesExecutionContext::TopLevel)
                != crate::events::IrulesCommandPlacement::Allowed
        {
            return None;
        }
        if let Some(effect) = self.irules_top_level_effect(name, args) {
            return Some(effect);
        }
        let bodies = self.arg_indices_for_role(name, args, crate::ArgRole::Body);
        let [body_index] = bodies.as_slice() else {
            return None;
        };
        if !arguments.is_braced_literal(*body_index) {
            return None;
        }
        if spec.traits.contains(crate::Traits::IS_EVENT_HANDLER) {
            return self.irules_event_declaration_shape(arguments);
        }
        if spec.traits.contains(crate::Traits::DEFINES_PROCEDURE) {
            let names = self.arg_indices_for_role(name, args, crate::ArgRole::Name);
            let [name_index] = names.as_slice() else {
                return None;
            };
            return Some(Declaration::Procedure {
                name_index: *name_index,
                body_index: *body_index,
            });
        }
        None
    }

    /// Parse a stateful iRules file-level declaration through its spec-owned
    /// effect descriptor.
    #[must_use]
    pub fn irules_top_level_effect(
        &self,
        name: &str,
        args: &[&str],
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let spec = self.get(name)?;
        let argument_count = u16::try_from(args.len()).ok()?;
        if !spec.arity.accepts(argument_count) {
            return None;
        }
        match spec.irules_top_level_effect {
            Some(crate::events::IrulesTopLevelEffect::Priority) => {
                let [value] = args else { return None };
                let value = value.parse::<u16>().ok()?;
                crate::events::BIGIP_EVENT_HANDLER_PRIORITY
                    .accepts(value)
                    .then_some(Declaration::Priority { value })
            }
            Some(crate::events::IrulesTopLevelEffect::Timing) => {
                let [value] = args else { return None };
                let enabled = match *value {
                    "on" | "enable" => true,
                    "off" | "disable" => false,
                    _ => return None,
                };
                Some(Declaration::Timing { enabled })
            }
            None => None,
        }
    }

    /// Validate the shared iRules event-handler argument grammar independent
    /// of its resolved registry spelling.
    #[must_use]
    pub fn irules_event_declaration(
        &self,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
        events: &crate::events::EventRegistry,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        let declaration = self.irules_event_declaration_shape(arguments)?;
        let crate::events::IrulesTopLevelDeclaration::Event { event, .. } = &declaration else {
            return None;
        };
        events.is_known(event).then_some(declaration)
    }

    /// Validate event-handler layout without requiring the event name to be
    /// known. Diagnostic consumers use this to report an otherwise valid
    /// top-level declaration whose event selector is unknown.
    #[must_use]
    pub fn irules_event_declaration_shape(
        &self,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;
        let args = arguments.values();
        let (body_index, priority) = match args {
            [_, _] => (1, None),
            [_, "priority", value, _] => {
                let priority = value.parse::<u16>().ok().filter(|priority| {
                    crate::events::BIGIP_EVENT_HANDLER_PRIORITY.accepts(*priority)
                })?;
                (3, Some(priority))
            }
            [_, "timing", value, _] => {
                matches!(*value, "on" | "off" | "enable" | "disable").then_some((3, None))?
            }
            [_, "priority", priority, "timing", timing, _] => {
                let priority = priority.parse::<u16>().ok().filter(|priority| {
                    crate::events::BIGIP_EVENT_HANDLER_PRIORITY.accepts(*priority)
                })?;
                matches!(*timing, "on" | "off" | "enable" | "disable")
                    .then_some((5, Some(priority)))?
            }
            _ => return None,
        };
        if !arguments.is_braced_literal(body_index) {
            return None;
        }
        let event = args.first()?.to_uppercase();
        Some(Declaration::Event {
            event,
            body_index,
            priority,
        })
    }

    /// Whether `name` should appear as a notable action node in a flow
    /// diagram ([`Traits::DIAGRAM_ACTION`]). Accepts both the bare
    /// (`HTTP::respond`) and the canonical (`::HTTP::respond`) spelling —
    /// the leading `::` stamped on `Statement::Call.canonical_command` by
    /// lowering is stripped to recover the bare registration form — and
    /// reflects the dialects loaded into this registry (the diagram-action
    /// set is part of the per-registry trait index, so a
    /// `--dialect f5-irules` registry recognises iRules actions).
    #[must_use]
    pub fn is_diagram_action(&self, name: &str) -> bool {
        let has = |n: &str| {
            self.by_name.get(n).is_some_and(|specs| {
                specs
                    .iter()
                    .any(|s| s.traits.contains(Traits::DIAGRAM_ACTION))
            })
        };
        has(name) || name.strip_prefix("::").is_some_and(has)
    }

    /// Whether `name` is explicitly marked **never** translatable to F5
    /// Distributed Cloud (XC) — any registered spec whose
    /// [`CommandSpec::xc_translatable`](crate::CommandSpec) is
    /// `Some(false)`. Consumed by the `f5-xc` iRule→XC translator.
    #[must_use]
    pub fn is_xc_never_translatable(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.xc_translatable == Some(false)))
    }

    /// Whether `name` is explicitly marked translatable to XC despite an
    /// otherwise-untranslatable namespace prefix — any registered spec
    /// whose [`CommandSpec::xc_translatable`](crate::CommandSpec) is
    /// `Some(true)`.
    #[must_use]
    pub fn is_xc_translatable_override(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.xc_translatable == Some(true)))
    }

    /// Whether `name` carries the [`Traits::NOT_PROC_FACTORY`] trait —
    /// a registered command head that incidentally matches the
    /// proc-factory token shape but is not a factory wrapper.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_not_proc_factory(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::NOT_PROC_FACTORY))
        })
    }

    /// Whether `name` is an iRules side-switch — `clientside`,
    /// `serverside`, or `peer` — i.e. a command that evaluates its
    /// nesting-script body under a different connection-side context
    /// than the surrounding event ([`Traits::IS_SIDE_SWITCH`]).
    /// Consulted by the iRules collect/release/payload flow check when
    /// it descends into a side-switch body.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_side_switch(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::IS_SIDE_SWITCH))
        })
    }

    /// Return the side context selected by `name`'s nesting-script body.
    ///
    /// This is more precise than [`Self::is_side_switch`]: callers that need
    /// to recurse into a body use the declared fixed/client/server/peer
    /// behaviour instead of matching a command spelling.
    #[must_use]
    pub fn side_switch_target(&self, name: &str) -> Option<SideSwitchTarget> {
        self.get(name)?.side_switch_target
    }

    /// Return the data-collection lifecycle fact declared by `name`.
    ///
    /// This is the only lookup consumers need for collect/release/payload
    /// analysis: command files opt in explicitly, so a similarly named custom
    /// command is never mistaken for an iRules buffer operation.
    #[must_use]
    pub fn data_collection_operation(&self, name: &str) -> Option<DataCollectionOperation> {
        self.get(name)?.data_collection
    }

    /// Return the declared protocol facts for `protocol`.
    ///
    /// A protocol can be known even when it has no `collect` command (UDP and
    /// ASM payloads are immediately available), so this scans all declared
    /// lifecycle operations rather than only collect commands.
    #[must_use]
    pub fn data_collection_protocol(&self, protocol: &str) -> Option<DataCollectionProtocol> {
        self.by_name
            .values()
            .flatten()
            .filter_map(|spec| spec.data_collection)
            .find(|operation| operation.protocol.name.eq_ignore_ascii_case(protocol))
            .map(|operation| operation.protocol)
    }

    /// Return the registered collect command for `protocol`, if it has one.
    ///
    /// Editor quick fixes use this to offer only commands that really exist;
    /// for example, UDP payload access deliberately returns `None` because
    /// BIG-IP provides each datagram without a `UDP::collect` command.
    #[must_use]
    pub fn data_collection_collect_command(&self, protocol: &str) -> Option<&CommandSpec> {
        self.by_name.values().flatten().copied().find(|spec| {
            spec.data_collection.is_some_and(|operation| {
                operation.action == DataCollectionAction::Collect
                    && operation.protocol.name.eq_ignore_ascii_case(protocol)
            })
        })
    }

    /// Return the event-handler priority policy declared by `name`.
    ///
    /// BIG-IP's `when` handler has a runtime default priority, so the policy
    /// records that omission is valid. Consumers can still honour a stricter
    /// policy for a dialect whose handler spec opts into one.
    #[must_use]
    pub fn event_handler_priority(&self, name: &str) -> Option<EventHandlerPriority> {
        self.get(name)?.event_handler_priority
    }

    /// Return the command spec that declares an event-handler priority grammar.
    ///
    /// Code generators and editor fixes use this when they need to create a
    /// handler rather than inspect an existing command. This keeps both the
    /// handler spelling and its priority grammar in the registry. Returns
    /// `None` if the dialect has no handler grammar or more than one, because
    /// choosing between multiple handlers requires additional registry data.
    #[must_use]
    pub fn event_handler_spec(&self) -> Option<&CommandSpec> {
        let mut handlers = self
            .by_name
            .values()
            .flatten()
            .filter(|spec| spec.event_handler_priority.is_some());
        let handler = handlers.next()?;
        handlers.next().is_none().then_some(handler)
    }

    /// Whether `name` is **frame-sensitive**: its meaning depends on the
    /// frame or scope it executes in, so moving a call across a proc
    /// boundary (inlining it into a caller) changes behaviour.  The union
    /// of the block-unwinding ([`Traits::TERMINATES_BLOCK`]),
    /// control-transferring ([`Traits::TRANSFERS_CONTROL`]),
    /// scope-aliasing ([`Traits::CREATES_SCOPE_ALIAS`]), and
    /// barrier-creating ([`Traits::CREATES_BARRIER`]) traits.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under the
    /// name.  Drives the inline-proc code action's safety decline and the
    /// negative half of [`Self::is_splice_safe`].
    #[must_use]
    pub fn is_frame_sensitive(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.intersects(FRAME_SENSITIVE_TRAITS))
        })
    }

    /// Every command name that is frame-sensitive ([`Self::is_frame_sensitive`]),
    /// for consumers that scan text for any member of the set (the
    /// inline-proc code action's body check) rather than querying one
    /// resolved head.
    #[must_use]
    pub fn frame_sensitive_commands(&self) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .iter()
                    .any(|s| s.traits.intersects(FRAME_SENSITIVE_TRAITS))
                    .then_some(*name)
            })
            .collect()
    }

    /// Whether `name` is **splice-safe**: a call to it can be lifted out of
    /// a wrapper proc and spliced into any caller's frame without changing
    /// observable behaviour.  A splice-safe command is fully lowered
    /// ([`Traits::FRAMELESS_RUNTIME`] — it never falls back to the
    /// interpreter, so it needs no frame) **and** frame-independent: not
    /// frame-sensitive ([`Self::is_frame_sensitive`]) and not operating on
    /// variables by name ([`Traits::FIRST_ARG_VARNAME`] writers,
    /// [`Traits::FRAME_HASH_BUILTIN`] hash-bucket binders, and
    /// [`Traits::DYNAMIC_EVAL_BODY`] body evaluators all observe the frame
    /// they run in).  Drives the inliner's verbatim-splice eligibility;
    /// membership is pinned by `splice_safe_membership` in
    /// `tests/registry_commands.rs`.
    #[must_use]
    pub fn is_splice_safe(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FRAMELESS_RUNTIME)
                && !s.traits.intersects(
                    FRAME_SENSITIVE_TRAITS
                        .union(Traits::FIRST_ARG_VARNAME)
                        .union(Traits::FRAME_HASH_BUILTIN)
                        .union(Traits::DYNAMIC_EVAL_BODY),
                )
        })
    }

    /// How *name* crosses stack frames, if it does — the registry's
    /// [`FrameEffectSpec`](crate::frame_effect::FrameEffectSpec).
    ///
    /// The single membership test for "is this a frame-crossing command?".
    /// A consumer reads the returned descriptor to find the level word and
    /// the affected arguments; it never names `upvar` / `uplevel` / `eval`
    /// itself.
    #[must_use]
    pub fn frame_effect(&self, name: &str) -> Option<crate::frame_effect::FrameEffectSpec> {
        self.get(name).and_then(|spec| spec.frame_effect)
    }

    /// The behavioural traits in force for one **concrete call** — the
    /// command's own set, unioned with those of the subcommand `args[0]`
    /// resolves to (exact or unique-prefix), when it has subcommands.
    ///
    /// The trait counterpart of [`Self::arg_indices_for_role`], and it takes
    /// the same `args` shape (subcommand first) for the same reason: a
    /// consumer asking "does *this* call declare a namespace / evaluate code"
    /// must not have to know whether the fact lives on the ensemble or on one
    /// of its subcommands. `namespace eval` carries
    /// [`Traits::DECLARES_NAMESPACE`] on the subcommand while `namespace`
    /// itself does not, and `namespace inscope` — identical argument layout,
    /// same analyser hook — does not carry it at all.
    #[must_use]
    pub fn call_traits(&self, name: &str, args: &[&str]) -> Traits {
        let Some(spec) = self.get(name) else {
            return Traits::empty();
        };
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        sub.map_or(spec.traits, |sub| spec.traits.union(sub.traits))
    }

    /// The declared [`ArgTypeHint`] for the 0-based argument `index` (after
    /// the command name) of a call to `name` with `args`.
    ///
    /// Resolves through the subcommand when the command is an ensemble and
    /// `args[0]` names one, so `dict for {k v} $d body` reports the `Dict`
    /// hint the `for` subcommand declares on its own argument 1.
    ///
    /// [`ArgTypeHint`]: crate::hooks::ArgTypeHint
    #[must_use]
    pub fn arg_type_hint(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
    ) -> Option<&'static crate::hooks::ArgTypeHint> {
        let spec = self.get(name)?;
        let find = |types: &'static [(u8, crate::hooks::ArgTypeHint)], idx: usize| {
            u8::try_from(idx)
                .ok()
                .and_then(|i| types.iter().find(|(at, _)| *at == i).map(|(_, h)| h))
        };
        if !spec.subcommands.is_empty()
            && !args.is_empty()
            && let Some(sub) = spec.resolve_subcommand(args[0])
            && index >= 1
        {
            return find(sub.arg_types, index - 1);
        }
        find(spec.arg_types, index)
    }

    /// Resolve argument indices for a given role.
    ///
    /// For subcommand-based commands (e.g. `dict create`), pass the
    /// subcommand as the first element of `args`.
    ///
    /// Three role sources feed this, in the order the registry contract
    /// documents: a dynamic `arg_role_resolver`, the static `arg_roles`
    /// table, and — for the unbounded regular tails a fixed table cannot
    /// express — the [`RepeatedArgLayout`]s of
    /// [`CommandSpec::repeated_args`] (issue #1185).  The repeated layouts
    /// are *additive*: a spec may pin its leading words with `arg_roles`
    /// (`namespace upvar`'s leading namespace word) and still declare the
    /// repeating pair tail.
    ///
    /// [`RepeatedArgLayout`]: crate::repeated::RepeatedArgLayout
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, args: &[&str], role: ArgRole) -> Vec<usize> {
        // `CommandPrefix` positions (with their appended arities) are owned by
        // [`Self::command_prefixes`]; delegate so highlighting, param-trait
        // inference, and the call-reference extractor all read one source.
        if role == ArgRole::CommandPrefix {
            return self
                .command_prefixes(name, args)
                .into_iter()
                .map(|(i, _)| i)
                .collect();
        }
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let n = args.len();
        let mut out: Vec<usize> = Vec::new();
        let case_body_roles_allowed = spec.case_list.is_none()
            || self
                .case_invocation(name, args, self.own_surface_query())
                .is_some();

        // Check subcommand (exact or unique-prefix abbreviation).
        if !spec.subcommands.is_empty()
            && !args.is_empty()
            && let Some(sub) = spec.resolve_subcommand(args[0])
        {
            // Positional roles, offset by +1 for the subcommand word.
            if let Some(resolver) = sub.arg_role_resolver {
                out.extend(
                    resolver(&args[1..])
                        .into_iter()
                        .filter(|(_, r)| *r == role)
                        .map(|(i, _)| i as usize + 1),
                );
            } else {
                out.extend(
                    sub.arg_roles
                        .iter()
                        .filter(|(_, r)| *r == role)
                        .map(|(i, _)| *i as usize + 1),
                );
            }
            // Repeated tails, over the words after the subcommand word.
            push_repeated_roles(&mut out, sub.repeated_args, n.saturating_sub(1), 1, role);
            // Value-taking options on the subcommand (scan past the sub word).
            push_option_value_roles(&mut out, sub.options, args, 1, role);
            out.retain(|&idx| idx < n);
            out.sort_unstable();
            out.dedup();
            return out;
        }

        // Option-selected pattern layouts are owned by the paired pattern
        // resolver.  Reusing that answer keeps role consumers aligned with
        // hover and semantic-token consumers, including profile-gated option
        // abbreviations and reserved positional suffixes.
        if role == ArgRole::Pattern && spec.pattern_arg_resolver.is_some() {
            out.extend(
                self.pattern_args(name, args)
                    .into_iter()
                    .map(|pattern| usize::from(pattern.index)),
            );
        // Top-level positional roles.
        } else if let Some(resolver) = spec.arg_role_resolver {
            out.extend(
                resolver(args)
                    .into_iter()
                    .filter(|(_, r)| {
                        *r == role && (role != ArgRole::Body || case_body_roles_allowed)
                    })
                    .map(|(i, _)| i as usize),
            );
        } else {
            out.extend(
                spec.arg_roles
                    .iter()
                    .filter(|(_, r)| {
                        *r == role && (role != ArgRole::Body || case_body_roles_allowed)
                    })
                    .map(|(i, _)| *i as usize),
            );
        }
        // Repeated tails (`global a b c`, `upvar ?level? o l o l`).
        push_repeated_roles(&mut out, spec.repeated_args, n, 0, role);
        // Value-taking options carry roles at their (dynamic) value positions.
        push_option_value_roles(&mut out, spec.options, args, 0, role);
        out.retain(|&idx| idx < n);
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether source words prove the option-dependent descriptor layout of
    /// `spec` (or its selected subcommand).  This is deliberately shared by
    /// pattern and format queries: both families otherwise risk interpreting
    /// `$mode` as a positional word before Tcl has decided whether it is an
    /// option.
    fn source_descriptor_layout_is_proven(
        &self,
        spec: &CommandSpec,
        sub: Option<&SubCommand>,
        args: InvocationArguments<'_>,
        effective_dialect: Option<SurfaceQuery<'_>>,
    ) -> bool {
        if !args.has_exact_argv_len() {
            return false;
        }
        // Static role positions and generic option-value roles remain stable
        // once expansion is excluded. Only a resolver can reinterpret a
        // source word as a positional operand, so a `clock format $time
        // -format %Y` time value does not suppress the independently-owned
        // `-format` value role.
        let resolver_depends_on_options = sub.map_or(
            spec.arg_role_resolver.is_some() || spec.pattern_arg_resolver.is_some(),
            |sub| sub.arg_role_resolver.is_some(),
        );
        if !resolver_depends_on_options {
            return true;
        }
        let (options, option_args, prefix_matching, reserved_trailing_words) = if let Some(sub) =
            sub
        {
            let options = self.profile().map_or_else(
                || {
                    sub.options
                        .iter()
                        .filter(|option| {
                            option.supports_dialect(effective_dialect, sub.surface.or(spec.surface))
                        })
                        .collect()
                },
                |profile| {
                    crate::profile_queries::ProfileQueries::available_sub_option_specs(
                        profile, spec, sub,
                    )
                },
            );
            (options, args.slice_from(1), sub.prefix_matching, 0)
        } else {
            let options = self.profile().map_or_else(
                || spec.option_specs(effective_dialect),
                |profile| {
                    crate::profile_queries::ProfileQueries::available_option_specs(profile, spec)
                },
            );
            (
                options,
                args,
                spec.prefix_matching,
                spec.reserved_trailing_words,
            )
        };
        options.is_empty()
            || source_option_layout_is_proven(
                &options,
                option_args,
                prefix_matching,
                reserved_trailing_words,
            )
    }

    /// Resolve a subcommand only when its source word has a known literal
    /// value. A dynamic selector cannot be projected into a selected
    /// descriptor's argument layout.
    fn source_selected_subcommand<'a>(
        spec: &'a CommandSpec,
        args: InvocationArguments<'_>,
    ) -> Option<&'a SubCommand> {
        (!spec.subcommands.is_empty())
            .then(|| {
                args.literal_at(0)
                    .and_then(|word| spec.resolve_subcommand(word))
            })
            .flatten()
    }

    /// The format-string words of **one concrete call**: which argument
    /// positions carry a conversion / field string, and which mini-language
    /// each is written in.
    ///
    /// The single registry answer to a question the LSP used to answer by
    /// matching command spellings — `match head { "format" => …, "scan" =>
    /// …, "clock" => …, "binary" => …, "regsub" => … }` in both the
    /// semantic-token walk and the inlay-hint collector, each with its own
    /// copy of the argument layout (issue #1185). It combines the two facts
    /// the registry already models:
    ///
    /// * **Where** — the [`ArgRole::FormatString`] / [`ArgRole::ScanFormat`]
    ///   positions of the call, resolved through
    ///   [`Self::arg_indices_for_role`], so a fixed index (`format`), a
    ///   resolver-computed one (`scan`, `regsub` past its switches), a
    ///   subcommand-relative one (`binary format`), and an **option value**
    ///   (`clock format … -format FMT`) all answer the same way.
    /// * **Which language** — [`CommandSpec::format_string_type`], overridden
    ///   by [`SubCommand::format_string_type`] when a subcommand dispatches
    ///   (`clock format` ⇒ `Clock`, `binary scan` ⇒ `Binary`).
    ///
    /// Because the head is resolved through [`Self::get`], the explicitly
    /// global spellings (`::format`, `::clock`, …) answer identically to the
    /// bare ones — the false negative the spelling tests had. A command with
    /// no declared family answers empty, so a same-named user proc, an
    /// unknown command, or a dynamic head is never misread.
    ///
    /// `args` is the post-head argument list, subcommand first, the same
    /// shape [`Self::arg_indices_for_role`] takes; returned indices are into
    /// that list.
    #[must_use]
    pub fn format_string_args(&self, name: &str, args: &[&str]) -> Vec<FormatStringArg> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        // A dispatching subcommand's family wins over the parent's.
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        let Some(kind) = sub
            .and_then(|s| s.format_string_type)
            .or(spec.format_string_type)
        else {
            return Vec::new();
        };
        let mut out: Vec<FormatStringArg> = Vec::new();
        for (role, scan) in [(ArgRole::FormatString, false), (ArgRole::ScanFormat, true)] {
            out.extend(
                self.arg_indices_for_role(name, args, role)
                    .into_iter()
                    .map(|index| FormatStringArg { index, kind, scan }),
            );
        }
        out.sort_by_key(|f| f.index);
        out
    }

    /// Source-aware counterpart to [`Self::format_string_args`].
    ///
    /// Unlike the compatibility string query, this method retains whether
    /// each source word is literal, dynamic, expanded, or opaque. The format
    /// family still comes entirely from the registry, but a dynamic leading
    /// word is never reinterpreted as a positional pattern or replacement
    /// before the profile-filtered option grammar has resolved it. This is
    /// the owner query for editors and other source-aware consumers.
    #[must_use]
    pub fn format_string_args_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
    ) -> Vec<FormatStringArg> {
        self.format_string_args_words_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::format_string_args_words`].
    #[must_use]
    pub fn format_string_args_words_for_dialect(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<FormatStringArg> {
        let effective_dialect = self
            .profile()
            .map_or(dialect, |profile| Some(profile.surface_query()));
        let spec = if effective_dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, effective_dialect)
        };
        let Some(spec) = spec else {
            return Vec::new();
        };
        // A dynamic subcommand cannot select a concrete format family.
        if !spec.subcommands.is_empty() && !args.is_empty() && args.literal_at(0).is_none() {
            return Vec::new();
        }
        let sub = Self::source_selected_subcommand(spec, args);
        let Some(kind) = sub
            .and_then(|sub| sub.format_string_type)
            .or(spec.format_string_type)
        else {
            return Vec::new();
        };
        if !self.source_descriptor_layout_is_proven(spec, sub, args, effective_dialect) {
            return Vec::new();
        }
        let spellings: Vec<_> = (0..args.len())
            .map(|index| args.literal_at(index).unwrap_or_default())
            .collect();
        let mut out = Vec::new();
        for (role, scan) in [(ArgRole::FormatString, false), (ArgRole::ScanFormat, true)] {
            out.extend(
                self.arg_indices_for_role(name, &spellings, role)
                    .into_iter()
                    .map(|index| FormatStringArg { index, kind, scan }),
            );
        }
        out.sort_by_key(|format| format.index);
        out
    }

    /// The pattern-bearing arguments of one concrete invocation.
    ///
    /// This pairs each declared position with its embedded language. A static
    /// command derives the positions from [`ArgRole::Pattern`]; an
    /// option-selected command supplies the paired facts through its registry
    /// resolver. Consumers therefore never need command-name or `-regexp`
    /// branches of their own.
    #[must_use]
    pub fn pattern_args(&self, name: &str, args: &[&str]) -> Vec<crate::patterns::PatternArg> {
        self.pattern_args_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::pattern_args`].
    ///
    /// A profile-bound registry always uses its attached profile.  A
    /// profile-less registry uses `dialect`, which is how editor consumers
    /// carrying one shared registry keep option-selected pattern layouts in
    /// sync with the current document's Tcl release.
    #[must_use]
    pub fn pattern_args_for_dialect(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<crate::patterns::PatternArg> {
        let effective_dialect = self
            .profile()
            .map_or(dialect, |profile| Some(profile.surface_query()));
        let spec = if effective_dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, effective_dialect)
        };
        let Some(spec) = spec else {
            return Vec::new();
        };
        if let Some(resolve) = spec.pattern_arg_resolver {
            let options = self.profile().map_or_else(
                || spec.option_specs(effective_dialect),
                |profile| {
                    crate::profile_queries::ProfileQueries::available_option_specs(profile, spec)
                },
            );
            return resolve(
                args,
                crate::patterns::PatternArgResolverContext {
                    options: &options,
                    reserved_trailing_words: spec.reserved_trailing_words,
                },
            );
        }
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        let Some(kind) = sub.and_then(|sub| sub.pattern_type).or(spec.pattern_type) else {
            return Vec::new();
        };
        self.arg_indices_for_role(name, args, ArgRole::Pattern)
            .into_iter()
            .filter_map(|index| u8::try_from(index).ok())
            .map(|index| crate::patterns::PatternArg { index, kind })
            .collect()
    }

    /// Source-aware counterpart to [`Self::pattern_args`].
    ///
    /// A pattern descriptor may use a dedicated pattern resolver, a dynamic
    /// argument-role resolver, or static roles. Every option-dependent form
    /// goes through the same source-layout proof before the compatibility
    /// resolver sees a spelling projection, so `regexp $mode …`,
    /// `glob $mode …`, and `string match $mode …` all abstain consistently.
    #[must_use]
    pub fn pattern_args_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
    ) -> Vec<crate::patterns::PatternArg> {
        self.pattern_args_words_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::pattern_args_words`].
    #[must_use]
    pub fn pattern_args_words_for_dialect(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<crate::patterns::PatternArg> {
        let effective_dialect = self
            .profile()
            .map_or(dialect, |profile| Some(profile.surface_query()));
        let spec = if effective_dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, effective_dialect)
        };
        let Some(spec) = spec else {
            return Vec::new();
        };
        if !spec.subcommands.is_empty() && !args.is_empty() && args.literal_at(0).is_none() {
            return Vec::new();
        }
        let sub = Self::source_selected_subcommand(spec, args);
        if !self.source_descriptor_layout_is_proven(spec, sub, args, effective_dialect) {
            return Vec::new();
        }
        let spellings: Vec<_> = (0..args.len())
            .map(|index| args.literal_at(index).unwrap_or_default())
            .collect();
        self.pattern_args_for_dialect(name, &spellings, effective_dialect)
    }

    /// How a formatter should **present** argument `index` of a call to
    /// `name` — the layout fact that refines the argument's semantic
    /// [`ArgRole`] (issue #1186).
    ///
    /// Returns the declared override, or [`ArgPresentation::BlockScript`]
    /// (the default) when the spec says nothing. `for`'s `start` and `next`
    /// scripts answer [`ArgPresentation::InlineScript`], which is how the
    /// formatting engine keeps them on the header line without a
    /// `name == "for"` branch.
    ///
    /// Resolves through [`Self::get`], so the explicitly-global spelling
    /// `::for` answers identically to the bare `for` — the false negative
    /// the formatter's literal name comparison had. A command the registry
    /// does not know (a user proc, a dynamic head) answers the default,
    /// which is what leaves such a call formatted like any other command.
    ///
    /// `args` is the post-name argument list, in the same shape
    /// [`Self::arg_indices_for_role`] takes (subcommand first), so a
    /// subcommand-dispatching call resolves against the subcommand's own
    /// table with the same `+1` offset.
    #[must_use]
    pub fn arg_presentation(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
    ) -> crate::presentation::ArgPresentation {
        use crate::presentation::ArgPresentation;
        let Some(spec) = self.get(name) else {
            return ArgPresentation::default();
        };
        // A dispatching subcommand owns the positions after its own word.
        if !spec.subcommands.is_empty()
            && let Some(sub) = args.first().and_then(|word| spec.resolve_subcommand(word))
            && index > 0
            && let Ok(sub_index) = u8::try_from(index - 1)
        {
            return sub
                .arg_presentation
                .iter()
                .find(|(i, _)| *i == sub_index)
                .map_or_else(ArgPresentation::default, |(_, p)| *p);
        }
        let Ok(idx) = u8::try_from(index) else {
            return ArgPresentation::default();
        };
        spec.arg_presentation
            .iter()
            .find(|(i, _)| *i == idx)
            .map_or_else(ArgPresentation::default, |(_, p)| *p)
    }

    /// The declared roles of the argument positions a call has **not**
    /// filled — the optional trailing words a caller could still append.
    ///
    /// [`Self::arg_indices_for_role`] answers "what role does each supplied
    /// argument play"; this answers the complementary question a
    /// splice-a-trailing-word quick-fix needs: "what would the *next* words
    /// mean if I added them".  Positions are returned in ascending order,
    /// starting at `args.len()` and stopping at the spec's arity ceiling, and
    /// only while the spec actually declares a role for the position — an
    /// unlimited-arity command with no declared tail role yields nothing
    /// rather than an unbounded run of `Value` slots.
    ///
    /// A subcommand-dispatching call resolves against the subcommand's own
    /// role table (offset by the subcommand word), exactly as
    /// [`Self::arg_indices_for_role`] does, so `chan gets`-shaped commands
    /// answer for the right spec.  Dynamic
    /// [`arg_role_resolver`](CommandSpec::arg_role_resolver) tables are
    /// consulted with the supplied argument list, which is what a resolver
    /// keyed on the *written* words can answer; positions the resolver does
    /// not name are simply absent from the result.
    #[must_use]
    pub fn unfilled_trailing_roles(&self, name: &str, args: &[&str]) -> Vec<(usize, ArgRole)> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        // Resolve against the subcommand when one dispatches, mirroring
        // `arg_indices_for_role`: `sub_offset` is the number of leading words
        // (the subcommand itself) the subcommand's own indices sit after.
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        let (static_roles, dynamic_roles, sub_offset) = match sub {
            Some(sub) => (sub.arg_roles, sub.arg_role_resolver, 1usize),
            None => (spec.arg_roles, spec.arg_role_resolver, 0usize),
        };
        let declared: Vec<(usize, ArgRole)> = match dynamic_roles {
            Some(resolve) => resolve(args.get(sub_offset..).unwrap_or(&[]))
                .into_iter()
                .map(|(idx, role)| (idx as usize + sub_offset, role))
                .collect(),
            None => static_roles
                .iter()
                .map(|(idx, role)| (*idx as usize + sub_offset, *role))
                .collect(),
        };
        let ceiling = usize::from(spec.arity.max);
        (args.len()..ceiling)
            .map_while(|position| {
                declared
                    .iter()
                    .find(|(idx, _)| *idx == position)
                    .map(|(_, role)| (position, *role))
            })
            .collect()
    }

    /// [`Self::arg_indices_for_role`]`(name, args, `[`ArgRole::Body`]`)`,
    /// filtered to the indices whose [`BodyKind`] is
    /// [`Plain`](BodyKind::Plain) — a body that runs in the caller's own
    /// frame (`if`/`while`/`for`/`foreach`/`switch`/`try`/`catch`/`eval`, …).
    ///
    /// `Structural` bodies (`proc`, `oo::class create`, `oo::define`,
    /// `snit::method`, `uplevel`, `namespace eval`, …) execute in a
    /// definition or different-frame dispatch context, so are excluded —
    /// a command found inside one is *not* still running in the enclosing
    /// call's scope.  This is the generic test for "is a dispatch nested in
    /// this body argument still the same lexical/dispatch context as the
    /// caller": intra-class `my`/`next`/`nextto` `TclOO` dispatch and `$obj`
    /// method dispatch recursion both need this to decide which nested
    /// script regions still belong to the same enclosing method.
    ///
    /// Respects a subcommand's own `body_kind` override (e.g. a compound
    /// command whose subcommand's body is structural even though the
    /// top-level spec defaults to `Plain`), the same resolution order
    /// [`Self::arg_indices_for_role`] uses for the subcommand's other roles.
    #[must_use]
    pub fn plain_body_arg_indices(&self, name: &str, args: &[&str]) -> Vec<usize> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let sub_body_kind = if spec.subcommands.is_empty() {
            None
        } else {
            args.first()
                .and_then(|first| spec.resolve_subcommand(first))
                .map(|sub| sub.body_kind)
        };
        let body_kind = sub_body_kind.unwrap_or(spec.body_kind);
        if body_kind != BodyKind::Plain {
            return Vec::new();
        }
        self.arg_indices_for_role(name, args, ArgRole::Body)
    }

    /// Resolve [`ArgRole::CommandPrefix`] argument positions and their
    /// [`AppendedArity`] for a concrete call.
    ///
    /// The single source of truth for command-prefix callbacks — unions the
    /// three declaration mechanisms (static [`CommandSpec::command_prefixes`]
    /// table, [`CommandSpec::command_prefix_resolver`], and `command_prefix`
    /// option values), for the top-level command or its resolved subcommand.
    /// Mirrors [`Self::arg_indices_for_role`]'s subcommand offset / `--`
    /// handling. Consumers: highlighting, find-references / call-hierarchy /
    /// call-graph recording, and the callback-arity check.
    ///
    /// For subcommand-based commands pass the subcommand as `args[0]`.
    #[must_use]
    pub fn command_prefixes(&self, name: &str, args: &[&str]) -> Vec<(usize, AppendedArity)> {
        self.command_prefixes_with_arguments(name, CommandPrefixArguments::literals(args))
    }

    /// Source-aware command-prefix resolution.
    ///
    /// `spellings` supports existing position-only declarations; `words`
    /// carries the proof boundary for literal-sensitive arity resolvers. The
    /// slices must describe the same post-head source words.
    #[must_use]
    pub fn command_prefixes_structured<'w>(
        &self,
        name: &str,
        spellings: &'w [&'w str],
        words: &'w [InvocationWord<'w>],
    ) -> Vec<(usize, AppendedArity)> {
        let Some(arguments) = CommandPrefixArguments::structured(spellings, words) else {
            return Vec::new();
        };
        self.command_prefixes_with_arguments(name, arguments)
    }

    fn command_prefixes_with_arguments(
        &self,
        name: &str,
        args: CommandPrefixArguments<'_>,
    ) -> Vec<(usize, AppendedArity)> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let n = args.len();
        let mut out: Vec<(usize, AppendedArity)> = Vec::new();

        if !spec.subcommands.is_empty()
            && !args.is_empty()
            && let Some(subcommand) = args.literal_at(0)
            && let Some(sub) = spec.resolve_subcommand(subcommand)
        {
            if let Some(resolver) = sub.command_prefix_resolver {
                out.extend(
                    resolver(args.slice_from(1))
                        .into_iter()
                        .map(|(i, a)| (i as usize + 1, a)),
                );
            } else {
                out.extend(
                    sub.command_prefixes
                        .iter()
                        .map(|(i, a)| (*i as usize + 1, *a)),
                );
            }
            push_command_prefix_options(&mut out, sub.options, args.spellings(), 1);
            out.retain(|&(idx, _)| idx < n);
            return out;
        }

        if let Some(resolver) = spec.command_prefix_resolver {
            out.extend(resolver(args).into_iter().map(|(i, a)| (i as usize, a)));
        } else {
            out.extend(spec.command_prefixes.iter().map(|(i, a)| (*i as usize, *a)));
        }
        push_command_prefix_options(&mut out, spec.options, args.spellings(), 0);
        out.retain(|&(idx, _)| idx < n);
        out
    }

    /// Resolve a concrete invocation to its target-neutral registry semantics.
    ///
    /// This is the common compiler entry point.  It retains the original head
    /// and argument spellings while selecting the registry command,
    /// subcommand, and form descriptors.  The returned projection contains no
    /// backend code-generation hook; target registries decide how to emit the
    /// already-resolved semantic operation later.
    ///
    /// Returns `None` when the command head is unknown or unavailable in
    /// `dialect`.
    #[must_use]
    pub fn resolve_invocation<'r, 'w>(
        &'r self,
        name: &'w str,
        args: &'w [&'w str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        self.resolve_structured_invocation(InvocationWords::literals(name, args), dialect)
            .resolved()
    }

    /// Resolve source-aware invocation words to target-neutral registry
    /// semantics.
    ///
    /// Only a literal command head can select a registry command. Likewise,
    /// only a literal first argument can select a subcommand. Expanded and
    /// opaque arguments make form arity indeterminate, so no form is matched.
    /// A substituted non-expanded argument still contributes exactly one argv
    /// entry and may therefore participate in an arity-only form selection;
    /// its spelling is never inspected as a value.
    ///
    /// The compatibility [`Self::resolve_invocation`] adapter constructs an
    /// all-literal view without allocation.
    #[must_use]
    pub fn resolve_structured_invocation<'r, 'w>(
        &'r self,
        words: InvocationWords<'w>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> StructuredInvocationResolution<'r, 'w> {
        let Some(name) = words.head_literal() else {
            return StructuredInvocationResolution::from_unresolved(
                InvocationResolutionUnresolved::ComputedHead {
                    word_kind: words.head().kind(),
                },
            );
        };
        let spec = if dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, dialect)
        };
        let Some(spec) = spec else {
            return StructuredInvocationResolution::from_unresolved(
                InvocationResolutionUnresolved::UnknownLiteralHead { spelling: name },
            );
        };
        let arguments = words.arguments();
        let (subcommand, sub) = resolve_semantic_subcommand(spec, arguments, dialect);
        let form = match (sub, spec.subcommands.is_empty(), arguments.is_empty()) {
            (Some(sub), _, _) => pick_form(sub.subcommand_forms, arguments, 1, dialect),
            (None, true, _) | (None, false, true) => {
                pick_form(spec.command_forms, arguments, 0, dialect)
            }
            (None, false, false) => None,
        };
        StructuredInvocationResolution {
            invocation: Some(ResolvedInvocation::new(words, spec, sub, form, subcommand)),
            unresolved: None,
        }
    }

    /// Resolve a concrete call to its registry-described form.
    ///
    /// Given the command head `name` and the literal argument words
    /// `args`, returns a [`ResolvedCall`] describing which
    /// [`CommandSpec`] / [`SubCommand`] / [`CommandForm`] the call
    /// matches and which lowering / codegen hook applies. The
    /// returned reference borrows from the registry, so callers must
    /// not retain it across registry mutation.
    ///
    /// Returns `None` when the command is unknown to the registry.
    #[must_use]
    pub fn resolve_call<'r>(
        &'r self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ResolvedCall<'r>> {
        let selection = self.resolve_legacy_call_selection(name, args, dialect)?;
        let spec = selection.spec;
        let sub = selection.sub;
        let form = selection.form;

        let mut resolved = ResolvedCall {
            spec,
            sub,
            form,
            lowering_hook: spec.lowering_hook,
            codegen_hook: spec.codegen_hook,
            inline_codegen_hook: spec.inline_codegen_hook,
            analyser_hook: spec.analyser_hook,
        };

        if let Some(sub) = sub {
            resolved.lowering_hook = form
                .and_then(|f| f.lowering_hook)
                .or(sub.lowering_hook)
                .or(spec.lowering_hook);
            resolved.codegen_hook = form
                .and_then(|f| f.codegen_hook)
                .or(sub.codegen_hook)
                .or(spec.codegen_hook);
            // Forms carry no inline hook — the inline emitters guard
            // their own applicability (arity / shape) at the dispatch
            // site, so subcommand-level wins over command-level.
            resolved.inline_codegen_hook = sub.inline_codegen_hook.or(spec.inline_codegen_hook);
            // Forms carry no analyser hook either — the analyser
            // handlers keep their own shape guards, so the
            // subcommand-level stamp wins over the command-level one.
            resolved.analyser_hook = sub.analyser_hook.or(spec.analyser_hook);
            return Some(resolved);
        }

        if let Some(f) = form {
            resolved.lowering_hook = f.lowering_hook.or(spec.lowering_hook);
            resolved.codegen_hook = f.codegen_hook.or(spec.codegen_hook);
            resolved.form = Some(f);
        }
        Some(resolved)
    }

    /// Select raw registry descriptors for the legacy call resolver.
    ///
    /// The exact subcommand lookup intentionally preserves the long-standing
    /// `resolve_call` compatibility behaviour.  The common
    /// [`ResolvedInvocation`] resolver instead uses
    /// [`CommandSpec::resolve_subcommand_word`] and records a typed outcome.
    fn resolve_legacy_call_selection<'r>(
        &'r self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<InvocationSelection<'r>> {
        let spec = if dialect.is_none() {
            self.get(name)?
        } else {
            self.get_for_surface(name, dialect)?
        };

        if !spec.subcommands.is_empty()
            && let Some(first) = args.first()
            && let Some(sub) = spec.subcommand(first)
        {
            return Some(InvocationSelection {
                spec,
                sub: Some(sub),
                form: pick_form(
                    sub.subcommand_forms,
                    InvocationArguments::literals(args),
                    1,
                    dialect,
                ),
            });
        }

        Some(InvocationSelection {
            spec,
            sub: None,
            form: pick_form(
                spec.command_forms,
                InvocationArguments::literals(args),
                0,
                dialect,
            ),
        })
    }

    /// Resolve the option-terminator profile for a command invocation.
    ///
    /// Matches the invocation's first argument against subcommands
    /// that declare an [`OptionSpec`](crate::hover::OptionSpec) with
    /// `name == "--"`, then falls back to form-level `--` declarations.
    /// Returns `None` when the command does not declare a `--`
    /// terminator at all (subcommand-scoped or form-scoped).
    ///
    /// Drives the W304 ("missing option terminator") diagnostic — the
    /// returned `scan_start` index, `subcommand` label, and
    /// `options` slice tell the caller where to start scanning for
    /// the first positional argument and which option specs to
    /// consult for value-consuming options (so they're not mistaken
    /// for positionals).  Returning a borrowed `&'static [OptionSpec]`
    /// rather than a freshly-allocated `HashSet` keeps the resolver
    /// allocation-free on the analyser hot path; per-command option
    /// counts are small (typically 1-3 value-consuming options per
    /// command), so a linear scan at the call site is cheaper than
    /// a `HashSet` build.
    ///
    #[must_use]
    pub fn resolve_option_terminator(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ResolvedTerminator> {
        let spec = if dialect.is_none() {
            self.get(name)?
        } else {
            self.get_for_surface(name, dialect)?
        };

        // Subcommand-scoped first. Resolve the subcommand word the same way
        // ensemble dispatch does — accepting a unique prefix abbreviation
        // (`string le` ⇒ `length`) — so an abbreviated subcommand keeps its
        // `--` terminator profile, matching `arg_indices_for_role`.
        if let Some(first) = args.first()
            && let Some(sub) = if dialect.is_none() {
                spec.resolve_subcommand(first)
            } else {
                spec.resolve_subcommand_for_dialect(first, dialect)
            }
            && sub.options.iter().any(|o| o.name == "--")
        {
            return Some(ResolvedTerminator {
                scan_start: 1,
                subcommand: Some(sub.name),
                options: sub.options,
                reserved_trailing_words: 0,
            });
        }

        // Form-level fallback — option specs live at the
        // `CommandSpec.options` level (a single set per spec), so we
        // consult that directly when no subcommand match was found.
        if spec.options.iter().any(|o| o.name == "--") {
            let effective_dialect = self
                .profile()
                .map_or(dialect, |profile| Some(profile.surface_query()));
            let reserved_trailing_words =
                spec.case_list.map_or(spec.reserved_trailing_words, |case| {
                    case.option_scan_reserved_trailing_words(
                        args,
                        effective_dialect,
                        spec.reserved_trailing_words,
                    )
                });
            return Some(ResolvedTerminator {
                scan_start: 0,
                subcommand: None,
                options: spec.options,
                reserved_trailing_words,
            });
        }

        None
    }

    /// The **command-table transitions** one invocation establishes — the
    /// one vocabulary for "this call changed the command table"
    /// (centralisation ledger C8, redesign §11.2 D9).
    ///
    /// Every consumer that models command-name bindings reads this: the
    /// flow-sensitive lattice, the lowerer's alias table, the analyser's
    /// rename / alias records, the realm scan, the taint walk. None of them
    /// dispatches on a coarse effect word and re-destructures the argument
    /// layout for itself any more — the layout lives with the registry
    /// resolver, once, and a dynamic operand becomes a typed
    /// [`TransitionSubject::Unknown`] or a domain widening rather than each
    /// consumer's own `is_dynamic_word` test.
    ///
    /// Selection is the ordinary
    /// [`Self::resolve_structured_invocation`] at **this registry's own
    /// point**: command-table mutations are executable behaviour, so
    /// they must use the profile-visible command surface. In particular,
    /// iRules intentionally has no `rename` or `interp`, even though the
    /// dialect-agnostic catalogue knows their Tcl specs. A profile-less
    /// registry (`build_default`) answers dialect-agnostically, exactly as
    /// [`Self::get`] does.
    ///
    /// Reusing that primitive means a **subcommand-shaped** mutator now
    /// resolves by the ordinary ensemble rule rather than by exact
    /// spelling, which is a deliberate behavioural change from the retired
    /// `command_table_effect`: `interp ali {} a {} b` really does create an
    /// alias in C Tcl (`interp` is an ensemble), and the retired resolver's
    /// exact-word lookup made it invisible to every binding consumer.
    /// `interp aliases` is its own subcommand and still states nothing.
    ///
    /// The explicitly global spellings answer
    /// identically to the bare ones — `::rename format ::origfmt`,
    /// `::interp alias {} myfmt {} ::origfmt` and `::proc ::greet {} {…}`
    /// all really do mutate the command table (tclsh 9.0.4 and 8.6.16,
    /// byte-identical; `namespace which -command ::rename` → `::rename`).
    ///
    /// [`TransitionSubject::Unknown`]: crate::TransitionSubject::Unknown
    #[must_use]
    pub fn command_binding_transitions(&self, words: InvocationWords<'_>) -> StateTransitions {
        let query = self.profile.map(DialectProfile::surface_query);
        self.resolve_structured_invocation(words, query)
            .resolved()
            .map_or_else(StateTransitions::default, |invocation| {
                invocation.state_transitions()
            })
    }

    /// Whether `name` (or the compound key `"cmd sub"`) produces a
    /// canonical Tcl list — a list whose elements are properly
    /// quoted so re-parsing by ``eval`` / ``uplevel`` /
    /// ``interp eval`` doesn't trigger unwanted substitution.
    ///
    /// The canonical-list-command set is
    /// derived from `return_type == TclType::List` on the command
    /// (or its subcommand entry), minus the explicit exclusion
    /// `concat` whose join-strip-grouping semantics can leave
    /// unquoted specials in the output.
    ///
    /// Drives the W101 (`eval` with string concatenation) safe-
    /// idiom suppression — `eval [list ...]`, `eval [linsert ...]`,
    /// `eval [split ...]`, etc. shouldn't fire because the inner
    /// command's output is a properly-quoted list.
    #[must_use]
    pub fn is_canonical_list_command(&self, name: &str) -> bool {
        // Exclusion: concat returns LIST but isn't canonical.
        if name == "concat" {
            return false;
        }
        // Compound form ``"cmd sub"`` — split into head + sub.
        if let Some((head, sub_name)) = name.split_once(' ') {
            if let Some(spec) = self.get(head)
                && let Some(sub) = spec.subcommand(sub_name)
            {
                return sub.return_type == Some(crate::types::TclType::List);
            }
            return false;
        }
        // Bare command name.
        self.get(name)
            .and_then(|spec| spec.return_type)
            .is_some_and(|t| t == crate::types::TclType::List)
    }

    /// `{command: BytePayloadSpec}` for every registered `<proto>::payload`
    /// byte-array command — the getter is a binary source and `<cmd> replace`
    /// a byte sink for the S110 byte-array-corruption check.
    ///
    /// Only commands actually loaded into this registry are returned, so the
    /// set is implicitly gated by the active dialect: a plain-Tcl registry
    /// (no iRules pack loaded) yields an empty map — the payload layouts
    /// are intersected with the active dialect.
    #[must_use]
    pub fn byte_array_payload_layouts(&self) -> HashMap<&'static str, BytePayloadSpec> {
        let mut out = HashMap::new();
        for specs in self.by_name.values() {
            for spec in specs {
                if let Some(layout) = spec.byte_array_payload {
                    out.insert(spec.name, layout);
                }
            }
        }
        out
    }

    /// Number of registered commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Resolved option-terminator profile for a command invocation.
///
/// Returned by [`CommandRegistry::resolve_option_terminator`].  Drives
/// the W304 ("missing option terminator") diagnostic.  Carries
/// borrowed references into the registry's static spec table; do
/// not retain across registry mutation.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTerminator {
    /// Index in the `args` slice where positional-argument scanning
    /// begins.  `0` for form-level matches; `1` for subcommand-scoped
    /// matches (the first arg is the subcommand keyword).
    pub scan_start: usize,
    /// Subcommand keyword that owns the `--` declaration, if the
    /// match was subcommand-scoped.  `None` for form-level matches.
    pub subcommand: Option<&'static str>,
    /// Borrowed slice of every option declared on the matched
    /// command (or subcommand).  Callers consult [`crate::hover::OptionSpec::takes_value`]
    /// on each entry to determine whether an option name consumes a
    /// following value argument — done at the call site to avoid the
    /// per-resolve `HashSet` allocation a precomputed name set would
    /// require.  Per-command counts are small; a linear scan is
    /// cheaper than a heap-allocated set on the analyser hot path.
    pub options: &'static [crate::hover::OptionSpec],
    /// Trailing words (after the command name) that are never scanned as
    /// option candidates — see [`crate::spec::CommandSpec::reserved_trailing_words`].
    /// `0` for every subcommand-scoped match (no subcommand currently
    /// needs the reservation) and for any form without one declared.
    pub reserved_trailing_words: usize,
}

/// One format-string word of a concrete call — the answer
/// [`CommandRegistry::format_string_args`] returns.
///
/// The two facts a consumer needs are deliberately separate: `kind` names the
/// mini-language (a `clock` field string and a `format` %-string share
/// neither syntax nor version gates), while `scan` says which *direction* it
/// is written in (`format` and `scan` share the `Sprintf` family but not its
/// conversion set — `%b` is 8.6+ in both, `%p` is a `format`-only Tcl 9
/// addition). Collapsing them would make a consumer guess one from the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormatStringArg {
    /// 0-based index into the post-head argument list.
    pub index: usize,
    /// Which mini-language the word is written in.
    pub kind: crate::patterns::FormatType,
    /// The word is a **scan**-direction spec ([`ArgRole::ScanFormat`]) rather
    /// than a format-direction one ([`ArgRole::FormatString`]).
    pub scan: bool,
}

/// Which `TclOO` method-context keyword a command word is — the answer
/// [`CommandRegistry::method_dispatch_keyword`] returns.
///
/// The three variants are three different axes, deliberately not merged: a
/// consumer that wants "does the next word name a method" wants
/// [`Self::SelfDispatch`] alone, and one that wants "is this a call into the
/// method chain" wants `SelfDispatch | NextChain` but never
/// [`Self::Introspection`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodDispatchKind {
    /// `my` — dispatches a method on the current object; the following word
    /// is a method name on the enclosing class
    /// ([`Traits::TCLOO_SELF_DISPATCH`]).
    SelfDispatch,
    /// `next` / `nextto` — invokes the next implementation of the
    /// *currently executing* method along the receiver's method resolution
    /// order, so no word names a method. `nextto`'s first word names the
    /// *class* to resume from, marked [`ArgRole::Name`] on the spec
    /// ([`Traits::TCLOO_NEXT_CHAIN`]).
    NextChain,
    /// `self` — introspects the current invocation and dispatches nothing;
    /// its argument is a closed subcommand set, never a method name
    /// ([`Traits::TCLOO_INTROSPECTION`]).
    Introspection,
}

/// Outcome of [`CommandRegistry::resolve_call`].
///
/// Carries borrowed references into the registry's spec table; the
/// resolved call describes the matched command spec, optionally a
/// matched subcommand and form, and the effective lowering / codegen
/// hook identifiers (form-level wins over subcommand-level wins
/// over command-level).
#[derive(Debug, Clone, Copy)]
pub struct ResolvedCall<'r> {
    /// The matched top-level command spec.
    pub spec: &'r CommandSpec,
    /// The matched subcommand, if the call has one.
    pub sub: Option<&'r SubCommand>,
    /// The matched form descriptor, if any.
    pub form: Option<&'r CommandForm>,
    /// Effective lowering hook identifier.
    pub lowering_hook: Option<LoweringHookId>,
    /// Effective codegen hook identifier.
    pub codegen_hook: Option<CodegenHookId>,
    /// Effective inline (value-position / catch-body) codegen hook
    /// identifier (subcommand-level wins over command-level).
    pub inline_codegen_hook: Option<InlineCodegenHookId>,
    /// Effective analyser handler-family hook identifier
    /// (subcommand-level wins over command-level).
    pub analyser_hook: Option<AnalyserHookId>,
}

/// Raw descriptor selection shared by the legacy and target-neutral
/// invocation-resolution facades.
#[derive(Debug, Clone, Copy)]
struct InvocationSelection<'r> {
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    form: Option<&'r CommandForm>,
}

impl ResolvedCall<'_> {
    /// Effective arity for this resolved call: the form arity if a
    /// form matched, otherwise the subcommand arity, otherwise the
    /// top-level [`CommandSpec`] arity.
    #[must_use]
    pub fn arity(&self) -> Arity {
        if let Some(f) = self.form {
            return f.arity;
        }
        if let Some(s) = self.sub {
            return s.arity;
        }
        self.spec.arity
    }

    /// Effective [`VarWriteTyping`] for this resolved call: the matched
    /// subcommand's when one matched (`binary scan` destructures where the
    /// bare `binary` does not), otherwise the top-level [`CommandSpec`]'s.
    ///
    /// The compiler's type-inference pass consults this to type the
    /// variables a command writes as a side effect, rather than assuming
    /// they receive the command's return value (issue #867).
    #[must_use]
    pub fn var_write_typing(&self) -> VarWriteTyping {
        self.sub
            .map_or(self.spec.var_write_typing, |s| s.var_write_typing)
    }

    /// The result↔element-structure fact for this call — the subcommand's
    /// when one matched, else the command's. See
    /// [`crate::types::ReturnElements`].
    #[must_use]
    pub fn return_elements(&self) -> Option<crate::types::ReturnElements> {
        self.sub
            .map_or(self.spec.return_elements, |s| s.return_elements)
    }

    /// The in-place element evolution of the written variable for this call —
    /// the subcommand's when one matched, else the command's. See
    /// [`crate::types::VarElementsEffect`].
    #[must_use]
    pub fn var_elements_effect(&self) -> Option<crate::types::VarElementsEffect> {
        self.sub
            .map_or(self.spec.var_elements_effect, |s| s.var_elements_effect)
    }
}

/// Select the [`CommandForm`] whose arity, dialect, and optional literal
/// argument-prefix selector match the invocation.
///
/// **No lifecycle filter, because there is nothing to filter on.**
/// [`CommandForm`] — the *semantic* form, which routes lowering / codegen
/// hooks and per-form descriptors — carries no [`crate::lifecycle::Lifecycle`]
/// field; only its documentation counterpart [`crate::hover::FormSpec`] does,
/// and that is what [`CommandSpec::primary_synopsis`] and
/// [`CommandSpec::optional_trailing_arg_names`] gate. If a semantic form ever
/// gains a lifecycle, this is where the same `package_version: Option<&str>`
/// parameter belongs — every caller here already threads a `dialect` through
/// and could thread a floor beside it.
fn pick_form<'r>(
    forms: &'r [CommandForm],
    arguments: InvocationArguments<'_>,
    argument_offset: usize,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<&'r CommandForm> {
    let argument_count = arguments.exact_argv_len()?.checked_sub(argument_offset)?;
    let n = u16::try_from(argument_count).unwrap_or(u16::MAX);
    let admits_dialect = |form: &CommandForm| !matches!(form.surface, Some(d) if dialect.is_some() && !surface_admits(d, dialect.as_ref()));
    // Resolve each selector word as its own Tcl keyword table. This matters
    // for nested operations: `tag c ...` is ambiguous between `cell` and
    // `configure` before a later word can be inspected, while `tag cell h`
    // first resolves `cell` exactly and then uniquely prefixes `has`.
    //
    // Keep walking after a selector completes: a sibling may extend it
    // (`tag` beside `tag bind`), and the longest statically matched selector
    // wins. A dynamic word while such an extension remains viable abstains;
    // it may turn out to be the longer selector at runtime.
    let mut active: Vec<&CommandForm> = forms
        .iter()
        .filter(|form| admits_dialect(form) && form.literal_argument_prefix.is_some())
        .collect();
    let mut selected = None;
    for selector_index in 0..argument_count {
        active.retain(|form| {
            form.literal_argument_prefix
                .is_some_and(|selector| selector.words.len() > selector_index)
        });
        if active.is_empty() {
            break;
        }
        let spelling = arguments.literal_at(argument_offset + selector_index)?;
        let mut has_exact_word = false;
        let mut abbreviated_word = None;
        let mut abbreviated_ambiguous = false;

        for form in &active {
            let selector = form
                .literal_argument_prefix
                .expect("active forms have literal selectors");
            let canonical = selector.words[selector_index];
            if spelling == canonical {
                has_exact_word = true;
            } else if selector.prefix_matching.accepts_prefixes()
                && !spelling.is_empty()
                && canonical.starts_with(spelling)
            {
                if abbreviated_word.is_some_and(|word| word != canonical) {
                    abbreviated_ambiguous = true;
                } else {
                    abbreviated_word = Some(canonical);
                }
            }
        }

        let canonical = if has_exact_word {
            spelling
        } else if abbreviated_ambiguous {
            return None;
        } else if let Some(abbreviated) = abbreviated_word {
            abbreviated
        } else {
            break;
        };
        active.retain(|form| {
            let selector = form
                .literal_argument_prefix
                .expect("active forms have literal selectors");
            selector.words[selector_index] == canonical
                && (spelling == canonical || selector.prefix_matching.accepts_prefixes())
        });
        if let Some(form) = active.iter().copied().find(|form| {
            form.arity.accepts(n)
                && form
                    .literal_argument_prefix
                    .is_some_and(|selector| selector.words.len() == selector_index + 1)
        }) {
            selected = Some(form);
        }
    }
    if selected.is_some() {
        return selected;
    }

    let selector_overlaps_arity = forms.iter().any(|form| {
        admits_dialect(form) && form.arity.accepts(n) && form.literal_argument_prefix.is_some()
    });
    if selector_overlaps_arity {
        return None;
    }
    forms.iter().find(|form| {
        admits_dialect(form) && form.arity.accepts(n) && form.literal_argument_prefix.is_none()
    })
}

/// Resolve a subcommand for the common semantic path.
///
/// The registry's keyword table determines prefix legality.  The legacy
/// [`ResolvedCall`] path intentionally remains exact-only because its callers
/// preserve historical hook-routing behaviour.
fn resolve_semantic_subcommand<'r, 'w>(
    spec: &'r CommandSpec,
    arguments: InvocationArguments<'w>,
    dialect: Option<SurfaceQuery<'_>>,
) -> (SubcommandResolution<'w>, Option<&'r SubCommand>) {
    if spec.subcommands.is_empty() || arguments.is_empty() {
        return (SubcommandResolution::NotApplicable, None);
    }

    let Some(spelling) = arguments.literal_at(0) else {
        let word_kind = arguments.get(0).map_or(
            crate::InvocationWordKind::Opaque,
            crate::InvocationWord::kind,
        );
        return (SubcommandResolution::Indeterminate { word_kind }, None);
    };
    let matched = spec.resolve_subcommand_word(spelling, dialect, None, None);
    match matched {
        crate::abbrev::KeywordMatch::Unique(canonical_name) => {
            // `resolve_subcommand_word` builds its table from `spec` itself,
            // so its canonical result is necessarily one of these entries.
            let sub = spec
                .subcommand(canonical_name)
                .expect("registry subcommand table and descriptor slice agree");
            let resolved = ResolvedSubcommand {
                spelling,
                canonical_name: sub.name,
            };
            let outcome = if spelling == sub.name {
                SubcommandResolution::Exact(resolved)
            } else {
                SubcommandResolution::UniquePrefix(resolved)
            };
            (outcome, Some(sub))
        }
        crate::abbrev::KeywordMatch::Ambiguous(_) => {
            (SubcommandResolution::Ambiguous { spelling }, None)
        }
        crate::abbrev::KeywordMatch::Unknown => (SubcommandResolution::Unknown { spelling }, None),
    }
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("commands", &self.by_name.len())
            .field("loaded_layers", &self.loaded_layers)
            .field("profile", &self.profile.map(|p| p.name))
            .field("ambient_packages", &self.ambient_packages)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::CommandRegistry;
    use crate::forms::LiteralArgumentPrefix;
    use tcl_dialect::model::SpecProvider;
    use tcl_dialect::model::{Family, SpecSurface, SurfaceLayer, SurfaceQuery};
    use tcl_dialect::surface;

    // -- ambient packages (issue #1627) -----------------------------------

    /// Two packs naming the same package are two claims that the runtime
    /// provides at least that version. The strongest claim is the one that
    /// holds, so the floor is the greatest — never the last declared, which
    /// would make the answer depend on pack load order.
    #[test]
    fn the_highest_declared_ambient_version_is_the_floor() {
        let mut registry = CommandRegistry::build_default();
        assert_eq!(
            registry.ambient_package_floor("Tk"),
            None,
            "a compiled-in registry declares none"
        );

        registry.insert_ambient_package("Tk", "8.6");
        registry.insert_ambient_package("Tk", "8.5");
        registry.insert_ambient_package("Itcl", "4.0");
        assert_eq!(registry.ambient_package_floor("Tk"), Some("8.6"));
        assert_eq!(registry.ambient_package_floor("Itcl"), Some("4.0"));
        assert_eq!(registry.ambient_package_floor("Expect"), None);

        // Order-independent: declaring the higher one last changes nothing.
        let mut reversed = CommandRegistry::build_default();
        reversed.insert_ambient_package("Tk", "8.5");
        reversed.insert_ambient_package("Tk", "8.6");
        assert_eq!(reversed.ambient_package_floor("Tk"), Some("8.6"));
    }

    // -- unfilled_trailing_roles (issue #1190) ----------------------------
    //
    // The complement of `arg_indices_for_role`: what the *next* words would
    // mean, which is the question a splice-a-trailing-argument quick fix asks.

    #[test]
    fn unfilled_trailing_roles_reports_the_optional_capture_variables() {
        let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
        // `catch {body}` leaves both `VarWrite` slots open.
        assert_eq!(
            reg.unfilled_trailing_roles("catch", &["{body}"]),
            vec![(1, ArgRole::VarWrite), (2, ArgRole::VarWrite)]
        );
        // One supplied leaves one.
        assert_eq!(
            reg.unfilled_trailing_roles("catch", &["{body}", "res"]),
            vec![(2, ArgRole::VarWrite)]
        );
        // Fully supplied leaves none.
        assert!(
            reg.unfilled_trailing_roles("catch", &["{body}", "res", "opts"])
                .is_empty()
        );
    }

    #[test]
    fn unfilled_trailing_roles_stops_at_the_arity_ceiling() {
        let reg = CommandRegistry::build_default();
        // Every position past `catch`'s maximum of three is absent, however
        // many are asked for.
        let roles = reg.unfilled_trailing_roles("catch", &["{body}"]);
        assert!(roles.iter().all(|(index, _)| *index < 3), "{roles:?}");
    }

    #[test]
    fn unfilled_trailing_roles_is_empty_for_an_unknown_command() {
        let reg = CommandRegistry::build_default();
        assert!(
            reg.unfilled_trailing_roles("no_such_command", &[])
                .is_empty()
        );
    }

    #[test]
    fn unfilled_trailing_roles_stops_at_an_undeclared_position() {
        let reg = CommandRegistry::build_default();
        // `puts` declares no trailing role a caller could fill with a
        // *known* meaning, so nothing is offered rather than an unbounded
        // run of `Value` slots.
        assert!(reg.unfilled_trailing_roles("puts", &["hello"]).is_empty());
    }

    /// `SAFE_INTERP_HIDDEN` and `TRANSFERS_CONTROL` were the same bit.
    ///
    /// Both were spelled `1 << 61`, so the 65th trait silently aliased the
    /// 61st. Because `FRAME_SENSITIVE_TRAITS` unions in `TRANSFERS_CONTROL`,
    /// every safe-interp-hidden command — `file`, `source`, `encoding`,
    /// `open`, … — read as frame-sensitive and had the inline-proc code
    /// action suppressed, while `break`/`continue`/`yield`/`yieldto`/
    /// `tailcall` read as safe-interp-hidden.
    ///
    /// The two are now separate enum variants, so the aliasing is
    /// unrepresentable rather than merely fixed. This pins the *behaviour*
    /// that was wrong, which a type-level guarantee alone does not cover.
    #[test]
    fn safe_interp_hidden_is_not_control_transfer() {
        let reg = CommandRegistry::build_default();
        assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);

        for name in ["break", "continue", "yield", "yieldto", "tailcall"] {
            let spec = reg.get(name).expect("control-transfer command");
            assert!(
                spec.traits.contains(Traits::TRANSFERS_CONTROL),
                "{name} must transfer control"
            );
            assert!(
                !spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
                "{name} is not hidden in a safe interpreter"
            );
            assert!(reg.is_frame_sensitive(name), "{name} is frame-sensitive");
        }

        // Safe-hidden commands carrying no *other* frame-sensitive trait.
        // These are the ones the alias was wrongly marking: each had the
        // inline-proc code action suppressed on it.
        for name in [
            "file", "encoding", "open", "socket", "exec", "cd", "pwd", "glob", "load", "unload",
        ] {
            let spec = reg.get(name).expect("safe-hidden command");
            assert!(
                spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
                "{name} is hidden in a safe interpreter"
            );
            assert!(
                !spec.traits.contains(Traits::TRANSFERS_CONTROL),
                "{name} does not transfer control"
            );
            assert!(
                !reg.is_frame_sensitive(name),
                "{name} must not be frame-sensitive — the alias suppressed the \
                 inline-proc code action on it"
            );
        }

        // The correction must not swing too far: `source` and `exit` are
        // frame-sensitive on their own merits (a barrier and a block
        // terminator respectively), and stay so.
        for name in ["source", "exit"] {
            assert!(
                reg.is_frame_sensitive(name),
                "{name} is frame-sensitive independently of the trait alias"
            );
        }
    }

    use super::*;

    #[test]
    fn get_for_dialect_picks_the_most_specific_visible_spec() {
        // §5.3/D6 — the single selection rule. Three specs of one name,
        // registered deliberately most-specific-FIRST so the old
        // last-match rule would pick the catch-all:
        //   1. a TCL86-scoped spec (tightest),
        //   2. a wider TCL86|TCL90 spec,
        //   3. a catch-all (`surface: None`).
        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: "d6_probe",
            surface: Some(SpecSurface::TCL86),
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_probe",
            surface: Some(surface![SpecSurface::core_in(
                Family::Tcl,
                &[("8.6", Some("9.1"))]
            )]),
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_probe",
            surface: None,
            ..CommandSpec::DEFAULT
        });

        // Scoped beats catch-all, tighter beats wider — even though the
        // catch-all was registered last.
        let under_86 =
            reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "8.6")));
        assert_eq!(under_86.and_then(|s| s.surface), Some(SpecSurface::TCL86));
        // Under 9.0 the tightest visible spec is the two-bit one.
        let under_90 =
            reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "9.0")));
        assert_eq!(
            under_90.and_then(|s| s.surface),
            Some(surface![SpecSurface::core_in(
                Family::Tcl,
                &[("8.6", Some("9.1"))]
            )])
        );
        // Where no scoped spec is visible, the catch-all still resolves.
        let under_84 =
            reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "8.4")));
        assert!(under_84.is_some_and(|s| s.surface.is_none()));
    }

    #[test]
    fn get_for_dialect_breaks_specificity_ties_by_last_registration() {
        // Curated pack overrides re-register a name at the same scope; the
        // later registration must keep winning (the old `.rev()` guarantee,
        // preserved as the D6 tie-break).
        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: "d6_tie",
            surface: Some(SpecSurface::TCL86),
            arity: Arity::exact(1), // base data
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_tie",
            surface: Some(SpecSurface::TCL86),
            arity: Arity::exact(2), // curated override
            ..CommandSpec::DEFAULT
        });
        let won = reg
            .get_for_surface("d6_tie", Some(SurfaceQuery::core(Family::Tcl, "8.6")))
            .expect("d6_tie resolves");
        assert_eq!(won.arity, Arity::exact(2), "later registration wins ties");
    }

    #[test]
    fn instance_method_walks_superclasses_breadth_first() {
        use crate::spec::{ObjectClassSpec, SubCommand};

        // Diamond hierarchy: Diamond → {Left, Right} → Base. Both Left and
        // Right (and Base) define `m`; a breadth-first walk in declaration
        // order must find Left's `m` first, whereas a `Vec::pop` LIFO walk
        // would reverse the siblings and return Right's.
        static M_LEFT: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "left",
            ..SubCommand::DEFAULT
        }];
        static M_RIGHT: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "right",
            ..SubCommand::DEFAULT
        }];
        static M_BASE: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "base",
            ..SubCommand::DEFAULT
        }];
        static DIAMOND: ObjectClassSpec = ObjectClassSpec {
            class_name: "Diamond",
            instance_methods: &[],
            superclasses: &["Left", "Right"],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };
        static LEFT: ObjectClassSpec = ObjectClassSpec {
            class_name: "Left",
            instance_methods: &M_LEFT,
            superclasses: &["Base"],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };
        static RIGHT: ObjectClassSpec = ObjectClassSpec {
            class_name: "Right",
            instance_methods: &M_RIGHT,
            superclasses: &["Base"],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };
        static BASE: ObjectClassSpec = ObjectClassSpec {
            class_name: "Base",
            instance_methods: &M_BASE,
            superclasses: &[],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };

        let mut reg = CommandRegistry::build_default();
        for oc in [&DIAMOND, &LEFT, &RIGHT, &BASE] {
            reg.insert(CommandSpec {
                name: oc.class_name,
                object_class: Some(oc),
                ..CommandSpec::DEFAULT
            });
        }

        let resolved = reg
            .instance_method("Diamond", "m")
            .expect("method resolves");
        assert_eq!(
            resolved.detail, "left",
            "breadth-first, declaration-ordered walk visits Left before Right"
        );
    }

    #[test]
    fn instance_method_at_honours_explicit_package_and_dialect_gates() {
        use crate::spec::ObjectClassSpec;

        static METHODS: [SubCommand; 1] = [SubCommand {
            name: "later",
            surface: Some(SpecSurface::TCL91),
            lifecycle: Lifecycle::introduced_in("9.1"),
            ..SubCommand::DEFAULT
        }];
        static CLASS: ObjectClassSpec = ObjectClassSpec {
            class_name: "VersionedClass",
            instance_methods: &METHODS,
            superclasses: &[],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };

        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: CLASS.class_name,
            required_package: Some("Demo"),
            object_class: Some(&CLASS),
            ..CommandSpec::DEFAULT
        });

        assert!(
            reg.instance_method_at(
                "VersionedClass",
                "later",
                Some("9.0"),
                Some(SurfaceQuery::core(Family::Tcl, "9.1"))
            )
            .is_none(),
            "the explicit owning-package floor rejects a future method"
        );
        assert!(
            reg.instance_method_at(
                "VersionedClass",
                "later",
                Some("9.1"),
                Some(SurfaceQuery::core(Family::Tcl, "9.0"))
            )
            .is_none(),
            "the method's own dialect gate remains independent of lifecycle"
        );
        assert!(
            reg.instance_method_at(
                "VersionedClass",
                "later",
                Some("9.1"),
                Some(SurfaceQuery::core(Family::Tcl, "9.1"))
            )
            .is_some(),
            "both registry-declared availability axes admit the method"
        );
    }

    #[test]
    fn build_default_has_commands() {
        let reg = CommandRegistry::build_default();
        assert!(!reg.is_empty());
        assert!(reg.get("for").is_some());
        assert!(reg.get("set").is_some());
        assert!(reg.get("nonexistent_command").is_none());
    }

    #[test]
    fn leading_zero_is_octal_tracks_tcl_version() {
        // Plain default registry (no Tcl release row) defaults to octal.
        assert!(CommandRegistry::build_default().leading_zero_is_octal());
        // tcl9.0 (TIP 114) reads leading zeros as decimal; everything else
        // (8.4/8.5/8.6 and the 8.x-derived F5 dialects) stays octal.
        let octal_cases = [
            SurfaceLayer::Core(Family::Tcl, "8.4"),
            SurfaceLayer::Core(Family::Tcl, "8.5"),
            SurfaceLayer::Core(Family::Tcl, "8.6"),
            SurfaceLayer::Core(Family::F5Irules, ""),
            SurfaceLayer::Package("iapps"),
        ];
        for layer in octal_cases {
            let mut reg = CommandRegistry::build_default();
            reg.load_surface(layer);
            assert!(reg.leading_zero_is_octal(), "{layer:?} should be octal");
        }
        let mut reg90 = CommandRegistry::build_default();
        reg90.load_surface(SurfaceLayer::Core(Family::Tcl, "9.0"));
        assert!(!reg90.leading_zero_is_octal(), "tcl9.0 should be decimal");
        // tcl9.1 keeps the TIP 114 decimal rule; a tcl9.1-only
        // registry (loads TCL91, not TCL90) must still read leading zeros as
        // decimal.
        let mut reg91 = CommandRegistry::build_default();
        reg91.load_surface(SurfaceLayer::Core(Family::Tcl, "9.1"));
        assert!(!reg91.leading_zero_is_octal(), "tcl9.1 should be decimal");
    }

    #[test]
    fn irules_command_legality_matrix() {
        use crate::events::EventRegistry;
        use crate::profiles::ProfileRegistry;
        let mut reg = CommandRegistry::build_default();
        reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        let events = EventRegistry::build();
        let profiles = ProfileRegistry::build();
        // HTTP::respond is satisfied in HTTP_REQUEST (HTTP profile implied) but
        // not in the L4 CLIENT_ACCEPTED event.
        assert!(reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "HTTP_REQUEST",
            &events,
            &profiles
        ));
        assert!(!reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "CLIENT_ACCEPTED",
            &events,
            &profiles
        ));
        // An unknown event is illegal for every command.
        assert!(!reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "NOT_AN_EVENT",
            &events,
            &profiles
        ));
        // HA::status is explicitly excluded from RULE_INIT.
        assert!(!reg.is_irules_command_legal_in_event(
            "HA::status",
            "RULE_INIT",
            &events,
            &profiles
        ));

        // The inverse list is sorted and reflects the same matrix.
        let evs = reg.irules_events_for_command("HTTP::respond", &events, &profiles);
        assert!(evs.contains(&"HTTP_REQUEST"));
        assert!(!evs.contains(&"CLIENT_ACCEPTED"));
        assert!(
            evs.windows(2).all(|w| w[0] <= w[1]),
            "events must be sorted"
        );
    }

    #[test]
    fn get_resolves_global_qualifier_to_builtin() {
        let reg = CommandRegistry::build_default();
        assert!(reg.get("::foreach").is_some());
        assert_eq!(
            reg.get("::for").map(|s| s.name),
            reg.get("for").map(|s| s.name)
        );
        assert!(reg.get("::nonexistent_command").is_none());
    }

    #[test]
    fn switch_names_is_dialect_filtered() {
        let reg = CommandRegistry::build_default();
        let regsub = reg.get("regsub").expect("regsub spec");
        // `-command` is Tcl 9.0+ (TIP 463); the always-available
        // switches appear in every dialect.
        let in_86 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
        assert!(in_86.contains(&"-all"), "{in_86:?}");
        assert!(in_86.contains(&"-nocase"), "{in_86:?}");
        assert!(
            !in_86.contains(&"-command"),
            "9.0-only -command leaked into 8.6: {in_86:?}",
        );
        let in_90 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
        assert!(
            in_90.contains(&"-command"),
            "-command missing under 9.0: {in_90:?}",
        );
        // No filter → every declared option, no duplicates.
        let all = regsub.switch_names(None);
        assert!(all.contains(&"-command"));
        let mut dedup = all.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(dedup.len(), all.len(), "switch_names returned duplicates");
    }

    #[test]
    fn tcl9_commands_from_pr_433_are_registered() {
        let reg = CommandRegistry::build_default();
        for name in [
            "foreachLine",
            "readFile",
            "writeFile",
            "lpop",
            "const",
            "tcl::idna",
            "::tcl::idna",
            "tcl::process",
            "::tcl::process",
        ] {
            assert!(
                reg.get(name).is_some(),
                "{name} not registered after SYNC-MAY19-tcl9-commands",
            );
        }
    }

    #[test]
    fn coroinject_coroprobe_registered() {
        // Verify these two commands remain registered and visible to
        // the LSP.
        let reg = CommandRegistry::build_default();
        assert!(reg.get("coroinject").is_some());
        assert!(reg.get("coroprobe").is_some());
    }

    #[test]
    fn tcl9_commands_gated_to_tcl90() {
        let reg = CommandRegistry::build_default();
        // `const` (Tcl 9.0, TIP 677) joins the list: it used to carry
        // `surface: None` as a workaround to reach iRules events, which
        // wrongly made it appear valid in 8.4/8.5/8.6 too. The registry-wide
        // explicit-dialect sweep corrected it to `TCL90_PLUS` — it does not
        // exist in iRules' embedded Tcl 8.4.6, so it is (correctly) neither
        // pre-9.0 nor iRules-visible.
        for name in ["foreachLine", "readFile", "writeFile", "lpop", "const"] {
            let spec = reg.get(name).expect("registered");
            // A 9.0 addition is available in 9.0 *and* 9.1 (a `.1` release is
            // additive — verified against C Tcl 9.1b0 doc/*.n), so it is gated
            // `TCL90_PLUS`, not `TCL90`-only.
            assert_eq!(
                spec.surface,
                Some(SpecSurface::TCL90_PLUS),
                "{name} should be Tcl 9.0+",
            );
            assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
            assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))));
            assert!(!spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.6"))));
            assert!(!spec.supports_dialect(Some(SurfaceQuery::any_release(Family::F5Irules))));
        }
    }

    #[test]
    fn every_command_has_hover_with_manpage_source() {
        // Every registered command must carry a hover snippet with a non-empty
        // summary and a manpage/source attribution. A short allowlist covers
        // internal pseudo-commands and dialect placeholders that have no user
        // documentation.
        // The four regex-quote spellings are internal idiom-recognition
        // entries for the taint analyser (T103), not real Tcl commands --
        // no manpage exists to cite (re_quote.html/.htm 404s on every
        // tcl-lang.org tree for 8.4-9.1 alike; see re_quote.rs's own doc
        // comment for the full explanation).
        const HOVERLESS_OK: &[&str] = &[
            "disabled_in_irules",
            "re_quote",
            "regex_quote",
            "regex::quote",
            "regexp::quote",
        ];
        let reg = CommandRegistry::build_default();
        let mut missing_hover = Vec::new();
        let mut missing_source = Vec::new();
        for name in reg.command_names() {
            if HOVERLESS_OK.contains(&name) {
                continue;
            }
            let spec = reg.get(name).expect("registered");
            match &spec.hover {
                None => missing_hover.push(name.to_string()),
                Some(h) => {
                    if h.summary.trim().is_empty() || h.source.trim().is_empty() {
                        missing_source.push(name.to_string());
                    }
                }
            }
        }
        missing_hover.sort();
        missing_source.sort();
        assert!(
            missing_hover.is_empty(),
            "commands without a hover snippet: {missing_hover:?}",
        );
        assert!(
            missing_source.is_empty(),
            "commands with an empty hover summary or manpage source: {missing_source:?}",
        );
    }

    #[test]
    fn timerate_registered_with_body_and_int_hint() {
        // `timerate` measures the rate of execution of a script.
        use crate::arg_role::ArgRole;
        use crate::side_effects::SideEffectTarget;
        use crate::types::TclType;
        let reg = CommandRegistry::build_default();
        let spec = reg.get("timerate").expect("timerate registered");
        assert_eq!(spec.name, "timerate");
        // BODY role on arg 0, INT type hint (shimmers) on arg 1.
        assert_eq!(spec.arg_role_at(0), Some(ArgRole::Body));
        assert_eq!(
            spec.arg_types
                .iter()
                .find(|(i, _)| *i == 1)
                .map(|(_, h)| h.expected),
            Some(Some(TclType::Int)),
        );
        // Unbounded arity: at least the command word, no upper bound.
        assert!(!spec.arity.accepts(0));
        assert!(spec.arity.accepts(1));
        assert!(spec.arity.accepts(6));
        assert_eq!(spec.return_type, Some(TclType::String));
        // The body runs arbitrary code → an UNKNOWN-target read+write effect.
        assert!(
            spec.side_effects
                .iter()
                .any(|e| e.target == SideEffectTarget::Unknown && e.reads && e.writes),
            "timerate should declare an UNKNOWN read+write side effect",
        );
    }

    #[test]
    fn lookup_for_command() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get("for").unwrap();
        assert_eq!(spec.name, "for");
        assert!(spec.traits.contains(Traits::CONTROL_FLOW));
        assert!(spec.traits.contains(Traits::HAS_LOOP_BODY));
        assert_eq!(spec.arity, crate::arity::Arity::exact(4));
    }

    #[test]
    fn arg_roles_for_static_command() {
        let reg = CommandRegistry::build_default();
        let bodies =
            reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Body);
        assert!(bodies.contains(&0)); // init
        assert!(bodies.contains(&2)); // next
        assert!(bodies.contains(&3)); // body
        assert!(!bodies.contains(&1)); // cond is Expr, not Body
    }

    #[test]
    fn arg_roles_for_expr() {
        let reg = CommandRegistry::build_default();
        let exprs =
            reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Expr);
        assert_eq!(exprs, vec![1]); // only the condition
    }

    #[test]
    fn uplevel_body_arg_role_skips_optional_level() {
        // Issue #837: `uplevel ?level? {body}` — the body word's index depends
        // on whether a leading `level` word is present. The registry resolver
        // is the single source of truth every body consumer (semantic tokens,
        // green-tree descent, SSA) queries.
        let reg = CommandRegistry::build_default();
        // Literal relative level → body at 1.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["1", "{set x 1}"], ArgRole::Body),
            vec![1]
        );
        // Absolute `#0` level → body at 1.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["#0", "{set x 1}"], ArgRole::Body),
            vec![1]
        );
        // No level → body at 0.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["{set x 1}"], ArgRole::Body),
            vec![0]
        );
        // Dynamic level followed by a script → body at 1.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["$lvl", "{set x 1}"], ArgRole::Body),
            vec![1]
        );
        // A lone dynamic word is the body itself (implicit level 1) → body at 0.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["$body"], ArgRole::Body),
            vec![0]
        );
        // Bodyless `uplevel 1` (a wrong-#args error) exposes no body word — the
        // literal level must not be mis-tagged as a script.
        assert!(
            reg.arg_indices_for_role("uplevel", &["1"], ArgRole::Body)
                .is_empty()
        );
    }

    #[test]
    fn if_marks_structural_keywords() {
        let reg = CommandRegistry::build_default();
        let args = [
            "1", "then", "{a}", "elseif", "2", "then", "{b}", "else", "{c}",
        ];
        let kw = reg.arg_indices_for_role("if", &args, ArgRole::Keyword);
        // then@1, elseif@3, then@5, else@7
        assert_eq!(kw, vec![1, 3, 5, 7], "{kw:?}");
        // The bodies and exprs still resolve too.
        let bodies = reg.arg_indices_for_role("if", &args, ArgRole::Body);
        assert!(bodies.contains(&2) && bodies.contains(&6) && bodies.contains(&8));
    }

    #[test]
    fn try_marks_structural_keywords() {
        let reg = CommandRegistry::build_default();
        let args = ["{body}", "on", "error", "{e}", "{h}", "finally", "{f}"];
        let kw = reg.arg_indices_for_role("try", &args, ArgRole::Keyword);
        // on@1, finally@5
        assert_eq!(kw, vec![1, 5], "{kw:?}");
    }

    /// Issue #1185 — the format families answer from registry data alone:
    /// the *position* from the `FormatString` / `ScanFormat` roles, the
    /// *family* from `format_string_type` (previously never populated).
    #[test]
    fn format_string_args_cover_every_family() {
        use crate::patterns::FormatType;
        let reg = CommandRegistry::build_default();
        let found = |name: &str, args: &[&str]| -> Vec<(usize, FormatType, bool)> {
            reg.format_string_args(name, args)
                .into_iter()
                .map(|f| (f.index, f.kind, f.scan))
                .collect()
        };
        // TP — a fixed argument index.
        assert_eq!(
            found("format", &["%d", "7"]),
            vec![(0, FormatType::Sprintf, false)]
        );
        // TP — a resolver-computed index, scan direction.
        assert_eq!(
            found("scan", &["$s", "%d", "v"]),
            vec![(1, FormatType::Sprintf, true)]
        );
        // TP — subcommand-relative indices.
        assert_eq!(
            found("binary", &["format", "c3", "$l"]),
            vec![(1, FormatType::Binary, false)]
        );
        assert_eq!(
            found("binary", &["scan", "$v", "c3", "out"]),
            vec![(2, FormatType::Binary, true)]
        );
        // TP — an *option value* position, both directions.
        assert_eq!(
            found("clock", &["format", "$t", "-format", "%Y"]),
            vec![(3, FormatType::Clock, false)]
        );
        assert_eq!(
            found("clock", &["scan", "$s", "-format", "%Y"]),
            vec![(3, FormatType::Clock, true)]
        );
        // TP — `regsub`'s replacement template, shifted past its switches.
        assert_eq!(
            found("regsub", &["-all", "--", "e", "$s", "X", "out"]),
            vec![(4, FormatType::Regsub, false)]
        );
        // FN guard — the explicitly global spellings resolve identically.
        assert_eq!(
            found("::format", &["%d", "7"]),
            found("format", &["%d", "7"])
        );
        assert_eq!(
            found("::clock", &["format", "$t", "-format", "%Y"]),
            found("clock", &["format", "$t", "-format", "%Y"])
        );
        // TN — a subcommand with no format string, and an unknown command.
        assert!(found("binary", &["encode", "hex", "$d"]).is_empty());
        assert!(found("clock", &["seconds"]).is_empty());
        assert!(found("no::such::command", &["%d"]).is_empty());
        assert!(found("puts", &["%d"]).is_empty());
        // TN — `regsub -command` makes that position a callback, not a
        // template, so it declares no replacement word.
        assert!(
            !found("regsub", &["-command", "--", "e", "$s", "cb"])
                .iter()
                .any(|(i, _, _)| *i == 3)
        );
    }

    #[test]
    fn source_aware_format_layouts_abstain_only_while_option_selection_is_dynamic() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::FormatType;

        let dynamic_prefix = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        let fixed_start_value = [
            InvocationWord::Literal("-start"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            assert!(
                registry
                    .format_string_args_words(
                        "regsub",
                        InvocationArguments::structured(&dynamic_prefix),
                    )
                    .is_empty(),
                "{dialect}: $mode could shift regsub's replacement position"
            );
            assert_eq!(
                registry.format_string_args_words(
                    "regsub",
                    InvocationArguments::structured(&fixed_start_value),
                ),
                vec![FormatStringArg {
                    index: 4,
                    kind: FormatType::Regsub,
                    scan: false,
                }],
                "{dialect}: a known -start still consumes exactly its dynamic value"
            );
        }

        let invalid_command_abbrev = [
            InvocationWord::Literal("-c"),
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        let exact_command = [
            InvocationWord::Literal("-command"),
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("callback"),
        ];
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            registry
                .format_string_args_words(
                    "regsub",
                    InvocationArguments::structured(&invalid_command_abbrev),
                )
                .is_empty(),
            "-c is not a Tcl regsub abbreviation and cannot claim a replacement template"
        );
        assert!(
            registry
                .format_string_args_words("regsub", InvocationArguments::structured(&exact_command))
                .is_empty(),
            "only exact Tcl 9 -command selects a callback rather than a replacement template"
        );

        let clock = [
            InvocationWord::Literal("format"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("-format"),
            InvocationWord::Literal("%Y"),
        ];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .format_string_args_words("clock", InvocationArguments::structured(&clock),),
            vec![FormatStringArg {
                index: 3,
                kind: FormatType::Clock,
                scan: false,
            }],
            "a fixed positional time value cannot hide an after-positional option value"
        );
    }

    #[test]
    fn source_aware_pattern_layouts_abstain_for_dynamic_leading_options() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};

        let dynamic_prefix = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a+}"),
            InvocationWord::Dynamic,
        ];
        for name in ["regexp", "regsub"] {
            assert!(
                crate::model::ingress::static_context_for("tcl9.0")
                    .commands()
                    .pattern_args_words(name, InvocationArguments::structured(&dynamic_prefix))
                    .is_empty(),
                "{name}: $mode could be a leading option"
            );
        }
        let glob_dynamic_prefix = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("/tmp"),
            InvocationWord::Literal("*.tcl"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "glob",
                    InvocationArguments::structured(&glob_dynamic_prefix)
                )
                .is_empty(),
            "glob $mode /tmp *.tcl cannot choose a pattern tail before $mode resolves"
        );
        let string_dynamic_prefix = [
            InvocationWord::Literal("match"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("a*"),
            InvocationWord::Literal("abc"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "string",
                    InvocationArguments::structured(&string_dynamic_prefix),
                )
                .is_empty(),
            "string match $mode a* abc cannot treat $mode as the pattern"
        );
    }

    #[test]
    fn source_aware_pattern_layouts_accept_proven_options_and_terminators() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{PatternArg, PatternType};

        let regexp_start = [
            InvocationWord::Literal("-start"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("a+"),
            InvocationWord::Dynamic,
        ];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words("regexp", InvocationArguments::structured(&regexp_start)),
            vec![PatternArg {
                index: 2,
                kind: PatternType::Regex,
            }],
            "a fixed value-taking option may safely carry a dynamic value"
        );
        let non_option_template = [InvocationWord::DynamicNonOption, InvocationWord::Dynamic];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "regexp",
                    InvocationArguments::structured(&non_option_template),
                ),
            vec![PatternArg {
                index: 0,
                kind: PatternType::Regex,
            }],
            "a template with a direct non-dash prefix remains a positional pattern"
        );
        let glob_terminator = [
            InvocationWord::Literal("--"),
            InvocationWord::Literal("-literal"),
        ];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words("glob", InvocationArguments::structured(&glob_terminator)),
            vec![PatternArg {
                index: 1,
                kind: PatternType::Glob,
            }],
            "-- freezes glob's option prefix before an option-shaped pattern"
        );
    }

    #[test]
    fn source_aware_pattern_layouts_respect_release_and_reserved_suffixes() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{PatternArg, PatternType};

        let strict_abbreviation = [
            InvocationWord::Literal("-sta"),
            InvocationWord::Literal("2"),
            InvocationWord::Literal("a+"),
            InvocationWord::Literal("aaa"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "regexp",
                    InvocationArguments::structured(&strict_abbreviation)
                )
                .is_empty(),
            "regexp's strict option table rejects a non-exact abbreviation"
        );
        let stride_abbreviation = [
            InvocationWord::Literal("-str"),
            InvocationWord::Literal("2"),
            InvocationWord::Literal("{a b}"),
            InvocationWord::Literal("a+"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl8.6")
                .commands()
                .pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&stride_abbreviation)
                )
                .is_empty(),
            "a Tcl 9-only option abbreviation is invalid before Tcl 9"
        );
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&stride_abbreviation),
                ),
            vec![PatternArg {
                index: 3,
                kind: PatternType::Glob,
            }],
            "the same abbreviation follows Tcl 9's profile-filtered option table"
        );
        let option_shaped_suffix = [
            InvocationWord::Literal("-regexp"),
            InvocationWord::Literal("--"),
        ];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&option_shaped_suffix),
                ),
            vec![PatternArg {
                index: 1,
                kind: PatternType::Glob,
            }],
            "lsearch's reserved list/pattern suffix remains positional"
        );
    }

    /// The source proof follows descriptors installed in the registry, not a
    /// closed list of Tcl command names. A pack or a later registry mutation
    /// can introduce the same option-dependent pattern/replacement grammar
    /// and all generic consumers receive the new answer unchanged.
    #[test]
    fn source_aware_pattern_and_format_queries_follow_mutated_descriptors() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{FormatType, PatternArg, PatternType};

        const OPTIONS: &[crate::hover::OptionSpec] = &[crate::hover::OptionSpec {
            name: "-skip",
            value: crate::hover::OptionValue::value("count"),
            detail: "fixture value",
            surface: None,
            aliases: &[],
            lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED,
            min_abbrev: None,
        }];
        fn roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
            let first =
                crate::spec::leading_option_word_count_with(OPTIONS, args, PrefixMatching::Strict);
            [
                (first, ArgRole::Pattern),
                (first + 2, ArgRole::FormatString),
            ]
            .into_iter()
            .filter_map(|(index, role)| {
                (index < args.len())
                    .then(|| u8::try_from(index).ok().map(|index| (index, role)))
                    .flatten()
            })
            .collect()
        }
        const SPEC: CommandSpec = CommandSpec {
            name: "mutated-pattern-format-owner",
            options: OPTIONS,
            prefix_matching: PrefixMatching::Strict,
            arg_role_resolver: Some(roles),
            pattern_type: Some(PatternType::Regex),
            format_string_type: Some(FormatType::Regsub),
            ..CommandSpec::DEFAULT
        };

        let mut registry = CommandRegistry::build_default();
        registry.insert(SPEC);
        let dynamic = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        assert!(
            registry
                .pattern_args_words(
                    "mutated-pattern-format-owner",
                    InvocationArguments::structured(&dynamic),
                )
                .is_empty()
        );
        assert!(
            registry
                .format_string_args_words(
                    "mutated-pattern-format-owner",
                    InvocationArguments::structured(&dynamic),
                )
                .is_empty()
        );

        let fixed = [
            InvocationWord::Literal("-skip"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        assert_eq!(
            registry.pattern_args_words(
                "mutated-pattern-format-owner",
                InvocationArguments::structured(&fixed),
            ),
            vec![PatternArg {
                index: 2,
                kind: PatternType::Regex,
            }]
        );
        assert_eq!(
            registry.format_string_args_words(
                "mutated-pattern-format-owner",
                InvocationArguments::structured(&fixed),
            ),
            vec![FormatStringArg {
                index: 4,
                kind: FormatType::Regsub,
                scan: false,
            }]
        );
    }

    /// Pattern locations come from the owning specs' option descriptors, not
    /// from an LSP walk guessing where a `-start` value ends.  Exercise the
    /// three shapes that previously drifted: abbreviations, value-taking
    /// options, and glob's unbounded pattern tail.
    #[test]
    fn pattern_roles_follow_declared_option_prefixes() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role("lsearch", &["-sta", "2", "{a b c}", "b*"], ArgRole::Pattern,),
            vec![3],
        );
        assert_eq!(
            reg.arg_indices_for_role("string", &["match", "-noc", "a*", "abc"], ArgRole::Pattern,),
            vec![2],
        );
        assert_eq!(
            reg.arg_indices_for_role("glob", &["-di", "tmp", "*.tcl", "*.tm"], ArgRole::Pattern,),
            vec![2, 3],
        );
        assert_eq!(
            reg.arg_indices_for_role("glob", &["--", "-literal", "*.tcl"], ArgRole::Pattern),
            vec![1, 2],
        );
        assert_eq!(
            reg.arg_indices_for_role("regexp", &["-start", "2", "a+", "aaa"], ArgRole::Pattern,),
            vec![2],
        );
        assert_eq!(
            reg.pattern_args("lsearch", &["-regexp", "{a b}", "a+"]),
            vec![crate::patterns::PatternArg {
                index: 2,
                kind: crate::patterns::PatternType::Regex,
            }],
        );
        assert!(
            reg.pattern_args("lsearch", &["-exact", "{a b}", "a+"])
                .is_empty()
        );
    }

    #[test]
    fn source_aware_lsearch_pattern_layout_respects_outer_option_boundary() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{PatternArg, PatternType};

        // Tcl's `Tcl_LsearchObjCmd` has used `for (i = 1; i < objc - 2;
        // i++)` since 8.4: it stops scanning exactly when list and pattern
        // remain. Keep the source-aware resolver aligned across every
        // supported core release, including the 9.0 -stride expansion.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            let proven_outer_option = [
                InvocationWord::Literal("-glob"),
                InvocationWord::Dynamic,
                InvocationWord::Literal("ba*"),
            ];
            assert_eq!(
                reg.pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&proven_outer_option),
                ),
                vec![PatternArg {
                    index: 2,
                    kind: PatternType::Glob,
                }],
                "{dialect}: lsearch -glob $list {{ba*}} has proven operands and glob mode"
            );

            let expanded_list = [
                InvocationWord::Literal("-glob"),
                InvocationWord::Expanded,
                InvocationWord::Literal("ba*"),
            ];
            assert!(
                reg.pattern_args_words("lsearch", InvocationArguments::structured(&expanded_list))
                    .is_empty(),
                "{dialect}: lsearch -glob {{*}}$list {{ba*}} has no known argv suffix"
            );

            let ambiguous_option_prefix = [
                InvocationWord::Dynamic,
                InvocationWord::Literal("{a b}"),
                InvocationWord::Literal("a+"),
            ];
            // `$mode` remains inside the `i < objc - 2` option region and
            // may become a value-taking switch, so it cannot be reclassified
            // as the list operand to claim a glob pattern.
            assert!(
                reg.pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&ambiguous_option_prefix),
                )
                .is_empty(),
                "{dialect}: lsearch $mode {{a b}} {{a+}} remains ambiguous"
            );
        }
    }

    #[test]
    fn lsearch_pattern_resolver_uses_profiled_options_and_the_reserved_suffix() {
        use crate::patterns::{PatternArg, PatternType};

        // These are oracle-shaped against Tcl's `i < objc - 2` scanner.  The
        // first two words of the short form are list + pattern, even though
        // both spell like lsearch switches.
        let option_shaped_operands = ["-regexp", "-glob"];
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            let patterns = registry.pattern_args("lsearch", &option_shaped_operands);
            assert_eq!(
                patterns,
                vec![PatternArg {
                    index: 1,
                    kind: PatternType::Glob,
                }],
                "{dialect}: lsearch -regexp -glob keeps both mandatory operands out of option scanning"
            );
            assert_eq!(
                registry.arg_indices_for_role("lsearch", &option_shaped_operands, ArgRole::Pattern),
                patterns
                    .iter()
                    .map(|pattern| usize::from(pattern.index))
                    .collect::<Vec<_>>(),
                "{dialect}: ArgRole::Pattern must share the resolver answer"
            );
        }

        // `-str` can name only -stride in Tcl 9.  Pre-9 interpreters reject
        // it, so the resolver must abstain rather than treating an invalid
        // dash-prefixed word as the list operand and inventing a pattern.
        let stride_abbrev = ["-str", "2", "{a b}", "a+"];
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            assert!(
                registry.pattern_args("lsearch", &stride_abbrev).is_empty(),
                "{dialect}: unavailable -stride abbreviation invalidates the invocation"
            );
            assert!(
                registry
                    .arg_indices_for_role("lsearch", &stride_abbrev, ArgRole::Pattern)
                    .is_empty(),
                "{dialect}: pattern roles abstain with the invalid invocation"
            );
        }
        for dialect in ["tcl9.0", "tcl9.1"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            let patterns = registry.pattern_args("lsearch", &stride_abbrev);
            assert_eq!(
                patterns,
                vec![PatternArg {
                    index: 3,
                    kind: PatternType::Glob,
                }],
                "{dialect}: -str is the available -stride abbreviation"
            );
            assert_eq!(
                registry.arg_indices_for_role("lsearch", &stride_abbrev, ArgRole::Pattern),
                vec![3],
                "{dialect}: roles, hover, and tokens share the Tcl 9 layout"
            );
        }

        // In Tcl 9 `-st` is ambiguous between -start and -stride. Unknown
        // words and unsupported `--` likewise cannot be reclassified as the
        // list operand while the mandatory suffix is still reserved.
        for args in [
            ["-st", "2", "{a b}", "a+"].as_slice(),
            ["-unknown", "{a b}", "a+"].as_slice(),
            ["--", "{a b}", "a+"].as_slice(),
        ] {
            let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
            assert!(
                registry.pattern_args("lsearch", args).is_empty(),
                "Tcl 9 invalid option prefix {args:?} must abstain"
            );
            assert!(
                registry
                    .arg_indices_for_role("lsearch", args, ArgRole::Pattern)
                    .is_empty(),
                "Tcl 9 invalid option prefix {args:?} must have no pattern role"
            );
        }

        // The final two words are always the mandatory list + pattern, even
        // when their spelling would otherwise be an invalid option.
        for args in [
            ["{a b}", "-unknown"].as_slice(),
            ["-regexp", "--"].as_slice(),
            ["-regexp", "-st"].as_slice(),
        ] {
            let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
            assert_eq!(
                registry.pattern_args("lsearch", args),
                vec![PatternArg {
                    index: 1,
                    kind: PatternType::Glob,
                }],
                "Tcl 9 final operands {args:?} stay positional"
            );
        }
    }

    #[test]
    fn source_aware_lsearch_dynamic_prefix_and_suffix_have_distinct_obligations() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{PatternArg, PatternType};

        for dialect in ["tcl8.6", "tcl9.0"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            let dynamic_early = [
                InvocationWord::Dynamic,
                InvocationWord::Literal("{a b}"),
                InvocationWord::Literal("a+"),
            ];
            assert!(
                registry
                    .pattern_args_words("lsearch", InvocationArguments::structured(&dynamic_early))
                    .is_empty(),
                "{dialect}: a dynamic word before the mandatory operands can still be a switch"
            );

            let dynamic_list_operand = [
                InvocationWord::Literal("-regexp"),
                InvocationWord::Dynamic,
                InvocationWord::Literal("a+"),
            ];
            assert_eq!(
                registry.pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&dynamic_list_operand),
                ),
                vec![PatternArg {
                    index: 2,
                    kind: PatternType::Regex,
                }],
                "{dialect}: the dynamic mandatory list is beyond the option boundary"
            );
        }
    }

    /// Issue #1185 — the repeated argument tails a fixed index table cannot
    /// express now answer through the ordinary role query.
    #[test]
    fn repeated_layouts_answer_through_arg_indices_for_role() {
        let reg = CommandRegistry::build_default();
        let vars =
            |name: &str, args: &[&str]| reg.arg_indices_for_role(name, args, ArgRole::VarWrite);
        // TP — every argument of `global`.
        assert_eq!(vars("global", &["a", "b", "c"]), vec![0, 1, 2]);
        // TP — every *even* argument of `variable` (names, not values).
        assert_eq!(vars("variable", &["x", "1", "y", "2"]), vec![0, 2]);
        // TP — the local of each `namespace upvar` pair, past the namespace.
        assert_eq!(
            vars("namespace", &["upvar", "::ns", "o1", "l1", "o2", "l2"]),
            vec![3, 5]
        );
        // TP — each `dict update` varName, with the trailing body excluded.
        // Index 1 is the dictionary variable itself, which the subcommand
        // already declared (it is read on entry and written back after the
        // body).  The pair locals answer as `LoopVarList` — bound once
        // before the body, and only when the key is present — NOT as
        // `VarWrite`: an unconditional SSA def both defeated the key-aware
        // read-before-set suppression and would hide the genuine
        // absent-key warning.
        assert_eq!(
            vars("dict", &["update", "d", "k1", "v1", "k2", "v2", "{body}"]),
            vec![1]
        );
        assert_eq!(
            reg.arg_indices_for_role(
                "dict",
                &["update", "d", "k1", "v1", "k2", "v2", "{body}"],
                ArgRole::LoopVarList
            ),
            vec![3, 5]
        );
        // TP — `foreach` / `lmap` variable specs, body excluded.
        for name in ["foreach", "lmap"] {
            assert_eq!(
                reg.arg_indices_for_role(
                    name,
                    &["{a b}", "$l1", "c", "$l2", "{body}"],
                    ArgRole::LoopVarList
                ),
                vec![0, 2],
                "{name}"
            );
        }
        // FN guard — the explicitly global spellings resolve identically.
        assert_eq!(vars("::global", &["a", "b"]), vars("global", &["a", "b"]));
        // TN — a command with no repeated tail is unaffected.
        assert_eq!(vars("puts", &["a", "b"]), Vec::<usize>::new());
        // FP guard — a short call names nothing it should not.
        assert_eq!(vars("global", &[]), Vec::<usize>::new());
        assert_eq!(
            reg.arg_indices_for_role("foreach", &["{body}"], ArgRole::LoopVarList),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn option_value_arity_shifts_registry_argument_roles() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "regexp",
                &["-start", "2", "pattern", "$text", "match"],
                ArgRole::VarWrite,
            ),
            vec![4]
        );
        assert_eq!(
            reg.arg_indices_for_role(
                "regsub",
                &["-start", "2", "pattern", "$text", "replacement", "result"],
                ArgRole::VarWrite,
            ),
            vec![5]
        );
        assert_eq!(
            reg.arg_indices_for_role("upvar", &["$level", "remote", "local"], ArgRole::VarWrite,),
            vec![2]
        );
    }

    /// Issue #1186 — `for`'s three script arguments share the semantic
    /// [`ArgRole::Body`], and the *presentation* fact is what separates the
    /// trailing body (block-expanded) from `start` / `next` (inline).
    #[test]
    fn for_declares_inline_presentation_for_start_and_next() {
        use crate::presentation::ArgPresentation;
        let reg = CommandRegistry::build_default();
        let args = ["{set i 0}", "{$i < 3}", "{incr i}", "{puts $i}"];
        // Semantics unchanged: all three scripts are still bodies.
        assert_eq!(
            reg.arg_indices_for_role("for", &args, ArgRole::Body),
            vec![0, 2, 3]
        );
        // Presentation splits them.
        assert_eq!(
            reg.arg_presentation("for", &args, 0),
            ArgPresentation::InlineScript
        );
        assert_eq!(
            reg.arg_presentation("for", &args, 2),
            ArgPresentation::InlineScript
        );
        assert_eq!(
            reg.arg_presentation("for", &args, 3),
            ArgPresentation::BlockScript
        );
        assert!(reg.arg_presentation("for", &args, 3).is_block());
        // The absolute global spelling resolves to the same spec.
        assert_eq!(
            reg.arg_presentation("::for", &args, 0),
            ArgPresentation::InlineScript
        );
        // A command with nothing to say answers the block default, and so
        // does one the registry does not know at all.
        assert_eq!(
            reg.arg_presentation("while", &["{$x}", "{body}"], 1),
            ArgPresentation::BlockScript
        );
        assert_eq!(
            reg.arg_presentation("no::such::command", &["{a}"], 0),
            ArgPresentation::BlockScript
        );
    }

    // -- Option-value roles (Phase 1) ------------------------------------

    fn opt(name: &'static str, value: crate::hover::OptionValue) -> crate::hover::OptionSpec {
        crate::hover::OptionSpec {
            name,
            value,
            ..crate::hover::OptionSpec::DEFAULT
        }
    }

    fn opt_with_alias(
        name: &'static str,
        aliases: &'static [&'static str],
        value: crate::hover::OptionValue,
    ) -> crate::hover::OptionSpec {
        crate::hover::OptionSpec {
            name,
            value,
            aliases,
            ..crate::hover::OptionSpec::DEFAULT
        }
    }

    fn indices(options: &[crate::hover::OptionSpec], args: &[&str], role: ArgRole) -> Vec<usize> {
        let mut out = Vec::new();
        push_option_value_roles(&mut out, options, args, 0, role);
        out
    }

    #[test]
    fn option_value_role_emits_body_index() {
        use crate::hover::OptionValue;
        let options = [
            opt("-command", OptionValue::script()),
            opt("-flag", OptionValue::flag()),
        ];
        let args = ["-command", "{puts hi}", "-flag", "x"];
        // The `-command` value is a Body; the flag consumes nothing.
        assert_eq!(indices(&options, &args, ArgRole::Body), vec![1]);
        // It is not a generic Value role.
        assert!(indices(&options, &args, ArgRole::Value).is_empty());
    }

    #[test]
    fn script_timing_is_exact_for_mixed_immediate_and_deferred_arguments() {
        use crate::hover::{OptionSpec, OptionValue, ScriptTiming};

        static OPTIONS: &[OptionSpec] = &[OptionSpec {
            name: "-later",
            value: OptionValue::deferred_script(),
            detail: "stored callback",
            ..OptionSpec::DEFAULT
        }];
        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: "mixed-script-timing",
            arity: crate::Arity::exact(3),
            arg_roles: &[(0, ArgRole::Body)],
            options: OPTIONS,
            ..CommandSpec::DEFAULT
        });

        let args = ["{error now}", "-later", "{error later}"];
        assert_eq!(
            reg.script_timing("mixed-script-timing", &args, 0, None),
            Some(ScriptTiming::SameInvocation),
        );
        assert_eq!(
            reg.script_timing("mixed-script-timing", &args, 2, None),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing("mixed-script-timing", &args, 1, None),
            None,
        );
    }

    #[test]
    fn shipped_script_options_distinguish_tk_callbacks_from_tcltest_bodies() {
        use crate::hover::ScriptTiming;

        let reg = CommandRegistry::build_default();
        let button = [".b", "-command", "{return}"];
        assert_eq!(
            reg.script_timing("button", &button, 2, None),
            Some(ScriptTiming::Deferred),
        );

        let test = ["sample", "description", "-body", "{error failure}"];
        assert_eq!(
            reg.script_timing("tcltest::test", &test, 3, None),
            Some(ScriptTiming::SameInvocation),
        );

        let dump = ["dump", "-command", "visit", "1.0"];
        assert_eq!(
            reg.script_timing("text", &dump, 2, None),
            Some(ScriptTiming::SameInvocation),
            "text dump calls its prefix synchronously for each returned item"
        );
        let sync = ["sync", "-command", "ready"];
        assert_eq!(
            reg.script_timing("text", &sync, 2, None),
            Some(ScriptTiming::Deferred),
            "text sync schedules its prefix after metrics become current"
        );
    }

    #[test]
    fn callback_taint_inputs_are_exact_and_exclude_tk_bookkeeping() {
        use crate::CallbackTaintInput;

        let reg = CommandRegistry::build_default();
        let validation = [".entry", "-validatecommand", "{eval %P}"];
        assert_eq!(
            reg.callback_taint_inputs("entry", &validation, 2, None),
            &[
                CallbackTaintInput::TK_PROPOSED_VALUE,
                CallbackTaintInput::TK_CURRENT_VALUE,
                CallbackTaintInput::TK_EDIT_TEXT,
            ],
        );
        let bind = [".entry", "<Key>", "{eval %A}"];
        assert_eq!(
            reg.callback_taint_inputs("bind", &bind, 2, None),
            &[CallbackTaintInput::TK_EVENT_CHAR],
        );
        let plain_command = [".entry", "-command", "{eval %P}"];
        assert!(
            reg.callback_taint_inputs("button", &plain_command, 2, None)
                .is_empty(),
            "a deferred callback without user substitutions must stay clean"
        );

        let canvas_bind = ["bi", "item", "<Key>", "{eval %A}"];
        assert_eq!(
            reg.callback_taint_inputs("canvas", &canvas_bind, 3, None),
            &[CallbackTaintInput::TK_EVENT_CHAR],
            "a unique-prefix widget subcommand must select its callback table"
        );

        let abbreviated_configure = ["conf", "-validatecommand", "{eval %P}"];
        assert_eq!(
            reg.instance_callback_taint_inputs("entry", ".entry", &abbreviated_configure, 2, None,),
            &[
                CallbackTaintInput::TK_PROPOSED_VALUE,
                CallbackTaintInput::TK_CURRENT_VALUE,
                CallbackTaintInput::TK_EDIT_TEXT,
            ],
            "a unique-prefix widget method must inherit constructor callback options"
        );

        let instance_bind = ["bind", "all", "<Key>", "{eval %A}"];
        assert_eq!(
            reg.instance_callback_taint_inputs("canvas", ".canvas", &instance_bind, 3, None,),
            &[CallbackTaintInput::TK_EVENT_CHAR],
            "a positional instance-method callback must use its method table"
        );
        assert!(
            reg.instance_callback_taint_inputs(
                "entry",
                ".entry",
                &["configure", "-validatecommand"],
                1,
                None,
            )
            .is_empty(),
            "the option word itself is not callback text"
        );
    }

    #[test]
    fn argument_sensitive_timing_distinguishes_registration_and_send_forms() {
        use crate::hover::ScriptTiming;

        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.script_timing("canvas", &["bind", "tag", "<Button-1>", "clicked"], 3, None,),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing("canvas", &["bi", "tag", "<Button-1>", "clicked"], 3, None,),
            Some(ScriptTiming::Deferred),
            "canvas bind's unique prefix must select the positional timing resolver",
        );
        assert_eq!(
            reg.script_timing(
                "wm",
                &["protocol", ".w", "WM_DELETE_WINDOW", "close"],
                3,
                None,
            ),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing("wm", &["prot", ".w", "WM_DELETE_WINDOW", "close"], 3, None,),
            Some(ScriptTiming::Deferred),
            "wm protocol's unique prefix must not fall back to same-invocation timing",
        );
        assert_eq!(
            reg.script_timing("send", &["other", "work"], 1, None),
            Some(ScriptTiming::SameInvocation),
        );
        assert_eq!(
            reg.script_timing("send", &["-async", "other", "work"], 2, None,),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing("send", &["-async", "other", "work", "argument"], 2, None,),
            None,
            "a concatenated multi-word script is not a fictional Body position",
        );
        assert_eq!(
            reg.script_timing(
                "selection",
                &["handle", "-type", "UTF8_STRING", ".w", "provide"],
                4,
                None,
            ),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing(
                "selection",
                &["h", "-type", "UTF8_STRING", ".w", "provide"],
                4,
                None,
            ),
            Some(ScriptTiming::Deferred),
            "selection handle's unique prefix must select its positional timing resolver",
        );
        assert_eq!(
            reg.script_timing("selection", &["o", "-command", "lost", ".w"], 2, None,),
            Some(ScriptTiming::Deferred),
            "subcommand option timing must use the same unique-prefix resolution",
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn command_prefix_hosts_report_their_exact_phase() {
        use crate::hover::ScriptTiming;

        let reg = CommandRegistry::build_default();
        let cases: &[(&str, &[&str], usize, ScriptTiming)] = &[
            // Immediate consumers execute the prefix before returning.
            (
                "lsort",
                &["-command", "compare", "{b a}"],
                1,
                ScriptTiming::SameInvocation,
            ),
            (
                "coroprobe",
                &["worker", "inspect"],
                1,
                ScriptTiming::SameInvocation,
            ),
            (
                "generator",
                &["reduce", "combine", "0", "values"],
                1,
                ScriptTiming::SameInvocation,
            ),
            // Registration and asynchronous forms store the prefix.
            (
                "socket",
                &["-server", "accept", "0"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "fcopy",
                &["in", "out", "-command", "done"],
                3,
                ScriptTiming::Deferred,
            ),
            (
                "chan",
                &["copy", "in", "out", "-command", "done"],
                4,
                ScriptTiming::Deferred,
            ),
            (
                "interp",
                &["alias", "", "aliasName", "", "target"],
                4,
                ScriptTiming::Deferred,
            ),
            (
                "interp",
                &["bgerror", "", "handler"],
                2,
                ScriptTiming::Deferred,
            ),
            (
                "namespace",
                &["unknown", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "package",
                &["unknown", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "coroinject",
                &["worker", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "tcltest::customMatch",
                &["mode", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "trace",
                &["add", "variable", "v", "write", "handler"],
                4,
                ScriptTiming::Deferred,
            ),
            (
                "trace",
                &["variable", "v", "w", "handler"],
                3,
                ScriptTiming::Deferred,
            ),
            (
                "generator",
                &["map", "transform", "values"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "hook",
                &["bind", "subject", "name", "observer", "handler"],
                4,
                ScriptTiming::Deferred,
            ),
            (
                "comm::comm",
                &["send", "-command", "handler", "peer", "work"],
                2,
                ScriptTiming::Deferred,
            ),
            (
                "mime::getbody",
                &["token", "-command", "handler"],
                2,
                ScriptTiming::Deferred,
            ),
            (
                "stringprep::register",
                &["profile", "-prohibitedCommand", "handler"],
                2,
                ScriptTiming::Deferred,
            ),
            (
                "tcl::chan::halfpipe",
                &["-close-command", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            // Removal forms name executable text without running or storing it.
            (
                "trace",
                &["remove", "variable", "v", "write", "handler"],
                4,
                ScriptTiming::ReferenceOnly,
            ),
            (
                "trace",
                &["vdelete", "v", "w", "handler"],
                3,
                ScriptTiming::ReferenceOnly,
            ),
        ];

        for &(command, args, index, expected) in cases {
            assert_eq!(
                reg.script_timing(command, args, index, None),
                Some(expected),
                "wrong phase for {command} {args:?} argument {index}",
            );
        }

        assert_eq!(
            reg.script_timing("trace", &["a", "v", "v", "write", "handler"], 4, None,),
            Some(ScriptTiming::Deferred),
            "unique-prefix subcommands and trace types preserve phase metadata",
        );
    }

    #[test]
    fn option_timing_resolver_overrides_static_default_for_bibtex_input_mode() {
        use crate::hover::ScriptTiming;

        let reg = CommandRegistry::build_default();
        let in_memory = ["-recordcommand", "record", "@book{x}"];
        assert_eq!(
            reg.script_timing("bibtex::parse", &in_memory, 1, None),
            Some(ScriptTiming::SameInvocation),
        );

        let channel = ["-recordcommand", "record", "-channel", "input"];
        assert_eq!(
            reg.script_timing("bibtex::parse", &channel, 1, None),
            Some(ScriptTiming::Deferred),
        );

        let completion = ["-command", "done", "-channel", "input"];
        assert_eq!(
            reg.script_timing("bibtex::parse", &completion, 1, None),
            Some(ScriptTiming::Deferred),
        );
    }

    #[test]
    fn known_stored_tk_command_prefixes_are_deferred() {
        use crate::hover::ScriptTiming;

        let reg = CommandRegistry::build_default();
        let stored = [
            ("canvas", "-xscrollcommand"),
            ("canvas", "-yscrollcommand"),
            ("entry", "-xscrollcommand"),
            ("listbox", "-xscrollcommand"),
            ("listbox", "-yscrollcommand"),
            ("menu", "-tearoffcommand"),
            ("scale", "-command"),
            ("scrollbar", "-command"),
            ("spinbox", "-xscrollcommand"),
            ("text", "-xscrollcommand"),
            ("text", "-yscrollcommand"),
            ("ttk::combobox", "-xscrollcommand"),
            ("ttk::entry", "-xscrollcommand"),
            ("ttk::scale", "-command"),
            ("ttk::scrollbar", "-command"),
            ("ttk::spinbox", "-xscrollcommand"),
            ("ttk::treeview", "-xscrollcommand"),
            ("ttk::treeview", "-yscrollcommand"),
            ("tk_chooseDirectory", "-command"),
            ("tk_getOpenFile", "-command"),
            ("tk_getSaveFile", "-command"),
            ("tk_messageBox", "-command"),
        ];
        for (command, option) in stored {
            assert_eq!(
                reg.script_timing(command, &[".w", option, "callback"], 2, None),
                Some(ScriptTiming::Deferred),
                "{command} {option} stores its command prefix",
            );
        }

        assert_eq!(
            reg.script_timing("text", &["dump", "-command", "visit", "1.0"], 2, None,),
            Some(ScriptTiming::SameInvocation),
        );
        assert_eq!(
            reg.script_timing("ttk::treeview", &["sort", "-command", "compare"], 2, None,),
            Some(ScriptTiming::SameInvocation),
        );
    }

    #[test]
    fn option_value_fixed_arity_emits_n_indices() {
        use crate::hover::OptionValue;
        let options = [opt("-rect", OptionValue::fixed(4, ArgRole::Value, "coord"))];
        let args = ["-rect", "1", "2", "3", "4", "tail"];
        assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3, 4]);
    }

    /// `OptionArity::Hook` equivalent of the old bare `Rest` variant —
    /// consumes every remaining word, always valid.
    fn rest_value(args: &[&str], start: usize) -> crate::hover::OptionValueOutcome {
        crate::hover::OptionValueOutcome {
            words: args.len() - start,
            invalid: None,
        }
    }

    #[test]
    fn option_value_rest_arity_stops_at_terminator() {
        let rest = crate::hover::OptionSpec {
            name: "-rest",
            value: crate::hover::OptionValue::Takes(crate::hover::OptionArg {
                arity: crate::hover::OptionArity::Hook(rest_value),
                ..crate::hover::OptionArg::DEFAULT
            }),
            ..crate::hover::OptionSpec::DEFAULT
        };
        let options = [rest];
        let args = ["-rest", "a", "b", "c", "--", "d"];
        // Consumes a, b, c up to `--`, not d.
        assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3]);
    }

    #[test]
    fn option_value_terminator_stops_scan() {
        use crate::hover::OptionValue;
        let options = [opt("-command", OptionValue::script())];
        let args = ["--", "-command", "{x}"];
        assert!(indices(&options, &args, ArgRole::Body).is_empty());
    }

    #[test]
    fn option_value_two_way_var_name_emits_for_both_roles() {
        use crate::hover::OptionValue;
        let options = [opt("-textvariable", OptionValue::var_name())];
        let args = ["-textvariable", "myvar"];
        assert_eq!(indices(&options, &args, ArgRole::VarWrite), vec![1]);
        assert_eq!(indices(&options, &args, ArgRole::VarRead), vec![1]);
    }

    #[test]
    fn option_value_alias_matches_and_dynamic_flag_skipped() {
        use crate::hover::OptionValue;
        let options = [opt_with_alias("-command", &["-cmd"], OptionValue::script())];
        // Alias resolves to the same value role.
        assert_eq!(indices(&options, &["-cmd", "{x}"], ArgRole::Body), vec![1]);
        // A `$var` in flag position isn't an option name → treated as a
        // positional, so the real `-command` after it still resolves.
        assert_eq!(
            indices(&options, &["$opt", "v", "-command", "{x}"], ArgRole::Body),
            vec![3]
        );
    }

    #[test]
    fn options_do_not_leak_into_positional_role_queries() {
        // A real command with value-taking options but no role annotations
        // must not surface any option value under Body/VarWrite (inert until
        // annotated).
        let reg = CommandRegistry::build_default();
        let b = reg.arg_indices_for_role("entry", &[".e", "-width", "10"], ArgRole::Body);
        assert!(b.is_empty(), "{b:?}");
    }

    #[test]
    fn command_prefix_option_is_captured_but_never_a_body() {
        // `lsort -command cmp {a b}` — the `-command` value is a CommandPrefix
        // (Phase 6), so it is captured under that role but never returned for a
        // Body query, i.e. a bareword prefix is not recursed as a script.
        let reg = CommandRegistry::build_default();
        let args = ["-command", "cmp", "{a b}"];
        assert!(
            reg.arg_indices_for_role("lsort", &args, ArgRole::Body)
                .is_empty(),
            "a command prefix must not be recursed as a body",
        );
        assert_eq!(
            reg.arg_indices_for_role("lsort", &args, ArgRole::CommandPrefix),
            vec![1],
            "the -command value should carry the CommandPrefix role",
        );
    }

    #[test]
    fn command_prefixes_carry_verified_arities() {
        // Ground-truthed vs real tclsh (stable 8.6→9.0). `command_prefixes`
        // is the single source of truth for both the position and the
        // appended arity.
        let reg = CommandRegistry::build_default();
        // `lsort -command cmp {a b}` → cmp invoked as `cmp x y` (2 appended).
        assert_eq!(
            reg.command_prefixes("lsort", &["-command", "cmp", "{a b}"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // `socket -server accept 9000` → accept invoked as `accept ch a p` (3).
        assert_eq!(
            reg.command_prefixes("socket", &["-server", "accept", "9000"]),
            vec![(1, AppendedArity::Exactly(3))],
        );
        // Positional: `tcltest::customMatch mode cmd`
        // → `cmd expected actual` (2).
        assert_eq!(
            reg.command_prefixes("tcltest::customMatch", &["exact", "cmp"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // Dynamic resolver: `selection handle window cmd` → the
        // last arg, invoked as `cmd offset maxChars` (2).
        assert_eq!(
            reg.command_prefixes("selection", &["handle", ".w", "getData"]),
            vec![(2, AppendedArity::Exactly(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_core_callback_commands() {
        // Ground-truthed vs real tclsh 8.6/9.0 (stable). Locks in the Phase-2
        // coverage of trace / interp / tcllib callbacks.
        let reg = CommandRegistry::build_default();
        // `trace add variable v w cb` → cb(name1 name2 op) = 3.
        assert_eq!(
            reg.command_prefixes("trace", &["add", "variable", "v", "write", "cb"]),
            vec![(4, AppendedArity::Exactly(3))],
        );
        // `trace add command c ops cb` → cb(old new op) = 3.
        assert_eq!(
            reg.command_prefixes("trace", &["add", "command", "c", "rename", "cb"]),
            vec![(4, AppendedArity::Exactly(3))],
        );
        // Execution operations select exact callback contracts.
        assert_eq!(
            reg.command_prefixes("trace", &["add", "execution", "c", "enter", "cb"]),
            vec![(4, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("trace", &["add", "execution", "c", "leave", "cb"]),
            vec![(4, AppendedArity::Exactly(4))],
        );
        assert_eq!(
            reg.command_prefixes("trace", &["add", "execution", "c", "enter leave", "cb"]),
            vec![(
                4,
                AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(&[2, 4]))
            )],
        );
        // Deprecated `trace variable v ops cb` → cb at index 3, 3 appended args.
        assert_eq!(
            reg.command_prefixes("trace", &["variable", "v", "write", "cb"]),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `interp alias {} a {} target x` (create form) → target at index 3,
        // variadic. The 2-arg query form has no target.
        assert_eq!(
            reg.command_prefixes("interp", &["alias", "{}", "a", "{}", "target"]),
            vec![(4, AppendedArity::Unknown)],
        );
        assert!(
            reg.command_prefixes("interp", &["alias", "{}", "a"])
                .is_empty(),
            "the interp-alias query form has no command prefix",
        );
        // tcllib: `struct::list filter seq cb` (1), `map seq cb` (1),
        // `fold seq init cb` (2).
        assert_eq!(
            reg.command_prefixes("struct::list", &["filter", "$s", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("struct::list", &["fold", "$s", "0", "cb"]),
            vec![(3, AppendedArity::Exactly(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_deferred_core_commands() {
        // Version-gated / resolver-driven callback tails wired in the deferred
        // pass. Ground-truthed vs tclsh 9.0 (all are 8.7/9.0-era surfaces).
        let reg = CommandRegistry::build_default();

        // `regsub -command re str cmdPrefix ?var?` (TIP 463, 9.0): with
        // `-command` the subSpec slot is a prefix called per match with the
        // whole match + capture groups appended (variadic ⇒ AtLeast(1)). The
        // slot index tracks the leading-switch shift.
        assert_eq!(
            reg.command_prefixes("regsub", &["-command", "re", "s", "cb"]),
            vec![(3, AppendedArity::AtLeast(1))],
        );
        assert_eq!(
            reg.command_prefixes("regsub", &["-all", "-command", "re", "s", "cb"]),
            vec![(4, AppendedArity::AtLeast(1))],
        );
        // `-c` is NOT an abbreviation of `-command`: regsub's switch table
        // resolves with Tcl_GetIndexFromObj(..., TCL_EXACT, ...), confirmed
        // live (tclsh 8.6.14: `regsub -c {a} aaa X y` -> `bad option "-c"`)
        // and against the real `Tcl_RegsubObjCmd` C source. Only the exact
        // spelling `-command` enables command-prefix mode.
        assert_eq!(
            reg.command_prefixes("regsub", &["-c", "re", "s", "cb"]),
            Vec::new(),
        );
        // Without `-command`, subSpec is a replacement template, not a prefix.
        assert!(
            reg.command_prefixes("regsub", &["re", "s", "template"])
                .is_empty(),
            "plain regsub subSpec is not a command prefix",
        );
        // `--` terminates switches, so a following `-command`-looking word is a
        // pattern, not the flag.
        assert!(
            reg.command_prefixes("regsub", &["--", "-command", "s", "template"])
                .is_empty(),
            "`--` disables -command detection",
        );

        // `namespace unknown handler` → handler(cmd ?arg...?) = AtLeast(1). The
        // zero-arg query form carries no prefix.
        assert_eq!(
            reg.command_prefixes("namespace", &["unknown", "handler"]),
            vec![(1, AppendedArity::AtLeast(1))],
        );
        assert!(
            reg.command_prefixes("namespace", &["unknown"]).is_empty(),
            "the namespace-unknown query form has no command prefix",
        );

        // `package unknown handler` → handler(name requirement ?requirement
        // ...?) = AtLeast(2): verified empirically on tclsh 8.6.14 that Tcl
        // always appends the package name *plus* at least one requirement
        // word, synthesizing a "0-" placeholder when `package require` was
        // given none itself — never just the bare name, so AtLeast(2), not
        // AtLeast(1) (see the fuller note on `package_.rs`'s `unknown`
        // subcommand).
        assert_eq!(
            reg.command_prefixes("package", &["unknown", "handler"]),
            vec![(1, AppendedArity::AtLeast(2))],
        );
        assert!(
            reg.command_prefixes("package", &["unknown"]).is_empty(),
            "the package-unknown query form has no command prefix",
        );

        // `coroinject coroName command ?arg...?` — per the Tcl 9.0/9.1
        // coroutine(n) manpage, exactly two more words are appended when the
        // injected command runs: the name of the command that suspended the
        // coroutine (yield or yieldto) and its current resumption value.
        assert_eq!(
            reg.command_prefixes("coroinject", &["myCoro", "cb", "x"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // `coroprobe coroName command ?arg...?` runs command immediately
        // inside the suspended coroutine (not deferred through a yield/
        // yieldto resumption), so no fixed extra-argument count is
        // documented -- it stays a reference-only prefix (Unknown).
        assert_eq!(
            reg.command_prefixes("coroprobe", &["myCoro", "cb"]),
            vec![(1, AppendedArity::Unknown)],
        );

        // `chan create mode cmdPrefix` / `chan push channelId cmdPrefix` — the
        // reflected-channel/transform handler is invoked as `cmdPrefix
        // subcommand handle ?args...?` ⇒ AtLeast(2).
        assert_eq!(
            reg.command_prefixes("chan", &["create", "rw", "handler"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );
        assert_eq!(
            reg.command_prefixes("chan", &["push", "$ch", "xform"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_tcllib_callbacks() {
        // tcllib callback tails wired in the deferred pass.  Fixed arities are
        // ground-truthed against the package man pages; ambiguous/variadic tails
        // are Unknown (reference-only).
        let reg = CommandRegistry::build_default();

        // `struct::list split seq cmdprefix` — the twin of filter: cmdprefix(el).
        assert_eq!(
            reg.command_prefixes("struct::list", &["split", "$s", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        // `fileutil::find basedir filtercmd` — filtercmd(name).  Shorter forms
        // carry no prefix (idx≥argc dropped).
        assert_eq!(
            reg.command_prefixes("fileutil::find", &["/base", "flt"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert!(
            reg.command_prefixes("fileutil::find", &["/base"])
                .is_empty(),
            "the basedir-only fileutil::find form has no filter prefix",
        );

        // generator functional ops: reference-only (multi-value yield ⇒ Unknown).
        assert_eq!(
            reg.command_prefixes("generator", &["map", "fn", "$g"]),
            vec![(1, AppendedArity::Unknown)],
        );
        assert_eq!(
            reg.command_prefixes("generator", &["filter", "pred", "$g"]),
            vec![(1, AppendedArity::Unknown)],
        );

        // math::calculus func callbacks (man-page-pinned fixed arities).
        assert_eq!(
            reg.command_prefixes("math::calculus::integral", &["0", "1", "100", "f"]),
            vec![(3, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("math::calculus::integral3D", &["xi", "yi", "zi", "f"]),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `newtonRaphson func deriv initval` — two prefixes, each f(x).
        assert_eq!(
            reg.command_prefixes("math::calculus::newtonRaphson", &["f", "d", "0.5"]),
            vec![
                (0, AppendedArity::Exactly(1)),
                (1, AppendedArity::Exactly(1)),
            ],
        );
        // `math::probopt::pso function bounds ?args?` — objective(coordVec).
        assert_eq!(
            reg.command_prefixes("math::probopt::pso", &["obj", "bnds", "-iter", "50"]),
            vec![(0, AppendedArity::Exactly(1))],
        );

        // log message-writer callbacks — `cmd level text` (Exactly(2)).
        assert_eq!(
            reg.command_prefixes("log::lvCmd", &["debug", "writer"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("log::lvCmdForall", &["writer"]),
            vec![(0, AppendedArity::Exactly(2))],
        );
        // `uevent::bind tag event command` — command(tag event ?details?).
        assert_eq!(
            reg.command_prefixes("uevent::bind", &["tag", "ev", "cb"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );

        // `hook bind subject hook observer binding` — the 4-word set form names
        // a command prefix (Unknown appended: the count is whatever the matching
        // `hook call` passes); the shorter query forms name none.
        assert_eq!(
            reg.command_prefixes("hook", &["bind", "sub", "hk", "obs", "cb"]),
            vec![(4, AppendedArity::Unknown)],
        );
        assert!(
            reg.command_prefixes("hook", &["bind", "sub", "hk", "obs"])
                .is_empty(),
            "the 3-arg `hook bind` query form has no callback prefix",
        );

        // `processman::onexit id cmd` — `cmd` is a deferred *script* (`eval $cmd`,
        // 0 appended), NOT a command prefix: it must declare none.
        assert!(
            reg.command_prefixes("processman::onexit", &["$pid", "cb"])
                .is_empty(),
            "processman::onexit cmd is a script body, not a command prefix",
        );
        assert_eq!(
            reg.get("processman::onexit")
                .and_then(|s| s.arg_roles.iter().find(|(i, _)| *i == 1))
                .map(|(_, r)| *r),
            Some(ArgRole::Body),
            "processman::onexit cmd (index 1) must carry the Body script role",
        );
    }

    #[test]
    fn command_prefixes_cover_option_value_callbacks() {
        // Option-value callbacks — the prefix is the value of a named `-flag`,
        // resolved through each command's `OptionSpec` array.  Arities are
        // ground-truthed against tcllib source (verified by re-running the exact
        // `uplevel`/`eval`/`{*}` idioms in tclsh — `uplevel`/`eval` re-split a
        // `[list …]` word, `[list {*}pfx {*}args]` keeps one word per element).
        let reg = CommandRegistry::build_default();

        // `mime::getbody token -command cb` — async body callback.  uplevel
        // re-splits `[list end]` → 1 word, `[list data $c]` → 2 ⇒ AtLeast(1).
        assert_eq!(
            reg.command_prefixes("mime::getbody", &["tok", "-command", "cb"]),
            vec![(2, AppendedArity::AtLeast(1))],
        );
        // `-decode` before `-command` is a value-less flag: the scan skips it
        // without swallowing the callback.
        assert_eq!(
            reg.command_prefixes("mime::getbody", &["tok", "-decode", "-command", "cb"]),
            vec![(3, AppendedArity::AtLeast(1))],
        );

        // `smtp::sendmessage tok -tlspolicy pol` — `eval $pol [list $code]
        // [list $diag]` ⇒ Exactly(2).  A preceding value-taking option's value
        // is skipped, not mistaken for the callback.
        assert_eq!(
            reg.command_prefixes("smtp::sendmessage", &["tok", "-tlspolicy", "pol"]),
            vec![(2, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes(
                "smtp::sendmessage",
                &["tok", "-servers", "mail.x", "-tlspolicy", "pol"],
            ),
            vec![(4, AppendedArity::Exactly(2))],
        );

        // `comm::comm send -command cb id cmd` — 7 `-key value` reply pairs ⇒
        // Exactly(14).  Subcommand-relative option scan offsets by 1.
        assert_eq!(
            reg.command_prefixes("comm::comm", &["send", "-command", "cb", "id", "cmd"]),
            vec![(2, AppendedArity::Exactly(14))],
        );

        // `bibtex::parse` — every callback prepends the parser token, then its
        // own payload words: `-command`/`-*command` ⇒ Exactly(2), except
        // `-recordcommand` ⇒ Exactly(4) (token type key recdata).
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-recordcommand", "cb", "text"]),
            vec![(1, AppendedArity::Exactly(4))],
        );
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-command", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-progresscommand", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );

        // `tcl::chan::halfpipe` — clean `[list {*}pfx {*}args]` idiom: write
        // appends (chan bytes) ⇒ 2, empty/close append (chan) ⇒ 1.  No
        // `-read-command` exists.
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-write-command", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-close-command", "cb"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-empty-command", "cb"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-read-command", "cb"])
                .is_empty(),
            "halfpipe has no -read-command",
        );
    }

    #[test]
    fn instance_method_command_prefixes_cover_struct_graph_and_tree() {
        // Object-instance method callbacks — the prefix is on a method of a
        // created object command (`$g walk … -command cb`, `$t walkproc … cb`),
        // resolved through the class's ObjectClassSpec.  Indices are relative to
        // the words after the method name.
        let reg = CommandRegistry::build_default();

        // struct::graph `walk node … -command cb` — option-value prefix,
        // Exactly(3) (action graphName node).
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::graph",
                "walk",
                &["root", "-order", "pre", "-command", "cb"],
            ),
            vec![(4, AppendedArity::Exactly(3))],
        );
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::graph",
                "walk",
                &["root", "-command", "cb"]
            ),
            vec![(2, AppendedArity::Exactly(3))],
        );

        // struct::tree `walkproc node … cmdprefix` — trailing positional prefix
        // (resolver), Exactly(3) (tree node action).  The prefix is the final
        // word regardless of intervening `-order`/`-type` options.
        assert_eq!(
            reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root", "cb"]),
            vec![(1, AppendedArity::Exactly(3))],
        );
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::tree",
                "walkproc",
                &["root", "-type", "dfs", "cb"],
            ),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `walkproc node` with no prefix word yet names none.
        assert!(
            reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root"])
                .is_empty(),
            "a walkproc with only the node names no prefix",
        );
        // An unmodelled method / class resolves to nothing.
        assert!(
            reg.instance_method_command_prefixes("struct::graph", "get", &["x"])
                .is_empty(),
            "an unmodelled instance method has no command prefix",
        );
    }

    #[test]
    fn tk_command_options_classified_prefix_vs_script() {
        // The Tk `script()→command_prefix` conversion (separate commit, highest
        // risk).  Locks in the classification: appended-arg callbacks are
        // prefixes (references / call-graph / W123 / arity); verbatim scripts and
        // percent-substitution callbacks stay `script()` (Body, not recorded as a
        // reference).  Ground truth: Tk 8.6/9.0 man pages.
        let reg = CommandRegistry::build_default();

        // PREFIXES — scroll callbacks append `first last` (2).
        for widget in ["listbox", "text", "canvas", "entry", "spinbox"] {
            assert_eq!(
                reg.command_prefixes(widget, &[".w", "-xscrollcommand", "cb"]),
                vec![(2, AppendedArity::Exactly(2))],
                "{widget} -xscrollcommand must be a prefix appending 2",
            );
        }
        // scale / ttk::scale append the new value (1).
        assert_eq!(
            reg.command_prefixes("scale", &[".s", "-command", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("ttk::scale", &[".s", "-command", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        // scrollbar -command: `moveto frac` (2) or `scroll n units` (3).
        assert_eq!(
            reg.command_prefixes("scrollbar", &[".sb", "-command", "cb"]),
            vec![(
                2,
                AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(&[2, 3]))
            )],
        );
        // menu -tearoffcommand appends the parent + torn-off menu paths (2).
        assert_eq!(
            reg.command_prefixes("menu", &[".m", "-tearoffcommand", "cb"]),
            vec![(2, AppendedArity::Exactly(2))],
        );

        // NOT prefixes — verbatim action scripts and percent-substitution
        // callbacks are `script()`, never recorded as a command reference.
        for (widget, opt) in [
            ("button", "-command"),
            ("checkbutton", "-command"),
            ("radiobutton", "-command"),
            ("menu", "-command"),
            ("menu", "-postcommand"),
            ("ttk::combobox", "-postcommand"),
            ("spinbox", "-command"), // percent-substitution (%W %s %d)
            ("spinbox", "-validatecommand"), // percent-substitution
            ("entry", "-validatecommand"), // percent-substitution
            ("entry", "-invalidcommand"), // percent-substitution
        ] {
            assert!(
                reg.command_prefixes(widget, &[".w", opt, "cb"]).is_empty(),
                "{widget} {opt} must stay a script, not a command prefix",
            );
        }
    }

    #[test]
    fn commands_naming_a_cmdprefix_declare_a_command_prefix() {
        // Drift guard: any command whose synopsis literally names a `cmdprefix`
        // argument must declare a `CommandPrefix` (static table, resolver, or
        // command-prefix option) so callbacks light up references / call-graph /
        // W123 / arity.  The allowlist holds genuinely-deferred callbacks that
        // still need modelling; it is empty now that the option-value
        // callbacks (`mime::getbody -command`, …) carry `OptionSpec` arrays.
        const DEFERRED_OPTION_PREFIX: &[&str] = &[];
        let reg = CommandRegistry::build_default();
        let mut gaps = Vec::new();
        for name in reg.command_names() {
            if DEFERRED_OPTION_PREFIX.contains(&name) {
                continue;
            }
            let spec = reg.get(name).expect("registered");
            let mut synopses: Vec<&str> = Vec::new();
            if let Some(h) = &spec.hover {
                synopses.extend(h.synopsis.iter().copied());
            }
            synopses.extend(spec.forms.iter().map(|f| f.synopsis));
            synopses.extend(spec.subcommands.iter().map(|s| s.synopsis));
            for syn in synopses {
                if !syn.to_ascii_lowercase().contains("cmdprefix") {
                    continue;
                }
                // Probe with the synopsis words after the command name; a
                // declared prefix yields a non-empty result.  Strip the `?…?`
                // optionality markers so an option-value prefix
                // (`?-command cmdprefix?`) presents its bare `-command` /
                // `cmdprefix` words to the option scanner.
                let args: Vec<&str> = syn
                    .split_whitespace()
                    .skip(1)
                    .map(|w| w.trim_matches('?'))
                    .collect();
                if reg.command_prefixes(name, &args).is_empty() {
                    gaps.push(format!("{name}: {syn}"));
                }
            }
        }
        gaps.sort();
        assert!(
            gaps.is_empty(),
            "commands whose synopsis names a cmdprefix but declare no CommandPrefix:\n{}",
            gaps.join("\n"),
        );
    }

    #[test]
    fn arg_indices_for_role_command_prefix_matches_command_prefixes() {
        // The delegation invariant: `arg_indices_for_role(CommandPrefix)` is
        // exactly the positions `command_prefixes` reports — so highlighting,
        // param-traits, and the call-reference extractor never drift.
        let reg = CommandRegistry::build_default();
        for (name, args) in [
            ("lsort", &["-command", "cmp", "{a b}"][..]),
            ("socket", &["-server", "accept", "9000"][..]),
            ("tcltest::customMatch", &["exact", "cmp"][..]),
            ("selection", &["handle", ".w", "getData"][..]),
            ("regsub", &["-command", "re", "s", "cb"][..]),
            ("namespace", &["unknown", "handler"][..]),
            ("package", &["unknown", "handler"][..]),
            ("coroinject", &["myCoro", "cb", "x"][..]),
        ] {
            let via_role = reg.arg_indices_for_role(name, args, ArgRole::CommandPrefix);
            let via_prefixes: Vec<usize> = reg
                .command_prefixes(name, args)
                .into_iter()
                .map(|(i, _)| i)
                .collect();
            assert_eq!(via_role, via_prefixes, "delegation mismatch for {name}");
        }
    }

    #[test]
    fn namespace_name_option_carries_name_role() {
        // `interp invokehidden -namespace ns cmd` — the `-namespace` value is a
        // symbolic (namespace) name (Phase 7): captured declaratively for a
        // Name query, never for Body/VarWrite (not recursed, not a var def).
        let reg = CommandRegistry::build_default();
        let args = ["invokehidden", "-namespace", "ns", "cmd"];
        assert_eq!(
            reg.arg_indices_for_role("interp", &args, ArgRole::Name),
            vec![2],
            "the -namespace value should carry the Name role",
        );
        assert!(
            reg.arg_indices_for_role("interp", &args, ArgRole::Body)
                .is_empty()
                && reg
                    .arg_indices_for_role("interp", &args, ArgRole::VarWrite)
                    .is_empty(),
            "a name value must not be recursed or treated as a variable",
        );
    }

    #[test]
    fn bind_script_form_recurses_only_the_trailing_script() {
        // `bind $w <KeyPress> {…}` binds a script (issue #785): the third
        // argument is a deferred event-handler body and must be recursed for
        // highlighting.  The `bind tag` / `bind tag sequence` query forms carry
        // no script and must not surface a Body.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role("bind", &["$w", "<KeyPress>", "{…}"], ArgRole::Body),
            vec![2],
            "the trailing script of the three-argument form is a body",
        );
        // The `+script` append form is still the trailing argument.
        assert_eq!(
            reg.arg_indices_for_role("bind", &[".b", "<Enter>", "+{puts hi}"], ArgRole::Body),
            vec![2],
        );
        assert!(
            reg.arg_indices_for_role("bind", &["$w"], ArgRole::Body)
                .is_empty(),
            "the single-tag query form has no script",
        );
        assert!(
            reg.arg_indices_for_role("bind", &["$w", "<KeyPress>"], ArgRole::Body)
                .is_empty(),
            "the tag+sequence query form has no script",
        );
    }

    #[test]
    fn wm_protocol_handler_is_a_body() {
        // `wm protocol . WM_DELETE_WINDOW {script}` registers a deferred
        // handler script as its third argument; the query forms carry none.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "wm",
                &["protocol", ".", "WM_DELETE_WINDOW", "{exit}"],
                ArgRole::Body,
            ),
            // subcommand path offsets by +1 for the `protocol` word.
            vec![3],
            "the wm protocol handler command is a script body",
        );
        assert!(
            reg.arg_indices_for_role("wm", &["protocol", ".", "WM_DELETE_WINDOW"], ArgRole::Body)
                .is_empty(),
            "the two-argument `wm protocol window name` query form has no script",
        );
        assert!(
            reg.arg_indices_for_role("wm", &["protocol", "."], ArgRole::Body)
                .is_empty(),
            "the one-argument `wm protocol window` query form has no script",
        );
    }

    #[test]
    fn canvas_bind_subcommand_script_is_a_body() {
        // `pathName bind tagOrId sequence script` binds a deferred handler.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "canvas",
                &["bind", "item", "<Button>", "{p}"],
                ArgRole::Body
            ),
            vec![3],
            "the canvas bind subcommand's trailing script is a body",
        );
        assert!(
            reg.arg_indices_for_role("canvas", &["bind", "item", "<Button>"], ArgRole::Body)
                .is_empty(),
            "the canvas bind query form has no script",
        );
    }

    #[test]
    fn ttk_widgets_require_tk_8_5() {
        // ttk (themed Tk) widgets were introduced in Tk 8.5, so they must be
        // gated out when only an older Tk is guaranteed by `package require`.
        let reg = CommandRegistry::build_default();
        for name in [
            "ttk::button",
            "ttk::treeview",
            "ttk::notebook",
            "ttk::style",
        ] {
            let spec = reg.get(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(
                !spec.available_for_version(Some("8.4")),
                "{name} must not be available under Tk 8.4",
            );
            assert!(
                spec.available_for_version(Some("8.5")),
                "{name} must be available under Tk 8.5",
            );
            assert!(
                spec.available_for_version(None),
                "{name} must be permissive when no Tk version is pinned",
            );
        }
    }

    #[test]
    fn text_tag_bind_script_is_a_body() {
        // `pathName tag bind tagName sequence script` binds a deferred
        // event-handler script as its trailing word (issue #785 class).
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "text",
                &["tag", "bind", "sel", "<Key>", "{p}"],
                ArgRole::Body,
            ),
            // `tag` subcommand offset (+1) plus index 3 within the tag args.
            vec![4],
            "the text tag bind trailing script is a body",
        );
        assert!(
            reg.arg_indices_for_role("text", &["tag", "add", "sel", "1.0", "end"], ArgRole::Body)
                .is_empty(),
            "non-bind tag subcommands carry no script",
        );
    }

    #[test]
    fn selection_handle_command_is_a_command_prefix() {
        // `selection handle window command` — the trailing command is a
        // prefix Tk appends offset/maxChars to, not a recursed script body.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "selection",
                &["handle", ".w", "getData"],
                ArgRole::CommandPrefix
            ),
            vec![2],
            "the selection handle command prefix is captured",
        );
        assert!(
            reg.arg_indices_for_role("selection", &["handle", ".w", "getData"], ArgRole::Body)
                .is_empty(),
            "a command prefix is not recursed as a script body",
        );
        // With leading option/value pairs the command is still the last arg.
        assert_eq!(
            reg.arg_indices_for_role(
                "selection",
                &["handle", "-format", "STRING", ".w", "getData"],
                ArgRole::CommandPrefix,
            ),
            vec![4],
        );
    }

    #[test]
    fn writes_first_arg_variable_membership() {
        let reg = CommandRegistry::build_default();
        // TP: the five first-arg writers.
        for cmd in ["set", "append", "lappend", "incr", "lset"] {
            assert!(reg.writes_first_arg_variable(cmd), "{cmd} writes arg 0");
        }
        // FP guards: `unset` destroys (not a write); value-taking and
        // unknown commands are out.
        for cmd in ["unset", "puts", "llength", "foreach", "nosuchcmd"] {
            assert!(!reg.writes_first_arg_variable(cmd), "{cmd} must not match");
        }
    }

    #[test]
    fn rmw_first_arg_variable_membership() {
        let reg = CommandRegistry::build_default();
        // TP: read-modify-write commands fold the current value in.
        for cmd in ["append", "lappend", "incr", "lset"] {
            assert!(reg.rmw_first_arg_variable(cmd), "{cmd} is RMW");
        }
        // FP guards: a whole-value `set` is rename-safe; `unset` destroys.
        for cmd in ["set", "unset", "puts", "nosuchcmd"] {
            assert!(!reg.rmw_first_arg_variable(cmd), "{cmd} must not match");
        }
    }

    #[test]
    fn commands_with_trait_query() {
        let reg = CommandRegistry::build_default();
        let control_flow = reg.commands_with_trait(Traits::CONTROL_FLOW);
        assert!(control_flow.contains(&"for"));
        assert!(control_flow.contains(&"if"));
        assert!(control_flow.contains(&"while"));
        assert!(!control_flow.contains(&"puts"));
    }

    #[test]
    fn byte_compiled_covers_the_core_builtins() {
        let reg = CommandRegistry::build_default();
        // Every registered command the minifier must never alias.
        // The former built-in skip list (minus the
        // non-command keywords `else` / `elseif` and the unregistered
        // `pwd`).
        let expected = [
            "set",
            "unset",
            "proc",
            "if",
            "while",
            "for",
            "foreach",
            "switch",
            "return",
            "break",
            "continue",
            "expr",
            "catch",
            "try",
            "throw",
            "package",
            "namespace",
            "upvar",
            "uplevel",
            "variable",
            "global",
            "append",
            "lappend",
            "incr",
            "info",
            "string",
            "list",
            "llength",
            "lindex",
            "lrange",
            "lsort",
            "lsearch",
            "lreplace",
            "linsert",
            "dict",
            "array",
            "regexp",
            "regsub",
            "scan",
            "format",
            "open",
            "close",
            "read",
            "gets",
            "eof",
            "flush",
            "seek",
            "tell",
            "fconfigure",
            "fcopy",
            "fileevent",
            "socket",
            "after",
            "update",
            "vwait",
            "rename",
            "source",
            "eval",
            "apply",
            "tailcall",
            "error",
            "cd",
            "file",
            "glob",
            "clock",
            "binary",
            "encoding",
            "interp",
            "load",
            "exit",
            "pid",
            "exec",
            "chan",
            "puts",
        ];
        for name in expected {
            assert!(
                reg.is_byte_compiled(name),
                "{name} should carry Traits::BYTE_COMPILED"
            );
        }
        // A user-proc-like name and a command outside the curated
        // skip set must not carry the trait.
        assert!(!reg.is_byte_compiled("my_helper_proc"));
        assert!(!reg.is_byte_compiled("split"));
    }

    #[test]
    fn not_proc_factory_covers_registered_skip_heads() {
        let reg = CommandRegistry::build_default();
        // Registered heads from the former `_FACTORY_SKIP_HEADS` list
        // (the four non-command heads method / classmethod /
        // itcl::class / ::itcl::class are handled by the scanner's
        // residual set, not the registry).
        let expected = [
            "proc",
            "namespace",
            "if",
            "switch",
            "while",
            "for",
            "foreach",
            "try",
            "catch",
            "eval",
            "apply",
            "expr",
            "uplevel",
            "upvar",
            "variable",
            "set",
            "lappend",
            "dict",
            "array",
            "string",
            "list",
            "lindex",
            "package",
            "source",
            "interp",
            "oo::class",
            "oo::define",
            "oo::objdefine",
        ];
        for name in expected {
            assert!(
                reg.is_not_proc_factory(name),
                "{name} should carry Traits::NOT_PROC_FACTORY"
            );
        }
        // A real factory-wrapper head must not be skipped.
        assert!(!reg.is_not_proc_factory("my_factory"));
    }

    #[test]
    fn frameless_runtime_covers_the_audited_allow_list() {
        let reg = CommandRegistry::build_default();
        let got: std::collections::HashSet<&str> = reg
            .commands_with_trait(Traits::FRAMELESS_RUNTIME)
            .into_iter()
            .collect();
        let expected: std::collections::HashSet<&str> = [
            "list",
            "lindex",
            "lrange",
            "linsert",
            "llength",
            "lsort",
            "lsearch",
            "lappend",
            "lreverse",
            "lreplace",
            // Tcl 9.0+: shares `lreplace`'s core (see
            // `tcl-vm/src/cmd_list.rs::cmd_ledit`); a flat single-variable
            // read-modify-write with no eval fallback and no sublist-index
            // descent (unlike `lset`, which deliberately stays off this
            // list) — see `commands/tcl/ledit.rs`.
            "ledit",
            "lrepeat",
            "lassign",
            // Tcl 9.0+: a flat native dispatch over `tcl-cmd-core::lseq`
            // (`tcl-vm/src/cmd_lseq.rs::cmd_lseq`) with no script-body eval
            // fallback of its own — its only recursive edge is evaluating
            // one argument word as an *expression* via `Vm::eval_expr`,
            // the same evaluator `expr` (also on this list) runs on, not a
            // general eval-a-script fallback — see `commands/tcl/lseq.rs`.
            "lseq",
            "concat",
            "split",
            "join",
            "string",
            "format",
            "expr",
            "global",
            "variable",
            "upvar",
            "namespace",
            "set",
            "incr",
            "append",
            "unset",
            "puts",
            "return",
            "error",
            "continue",
            "break",
            // `throw type message` (Tcl 8.6+) — `cmd_throw`
            // (`tcl-vm/src/cmd_try.rs`) takes its two already-substituted
            // arguments, validates `type` as a non-empty Tcl list, and
            // builds its return-options dict directly with no eval
            // fallback of any kind (it never touches `vm`), structurally
            // identical to `cmd_error` in this respect. See
            // `commands/tcl/throw_.rs`.
            "throw",
            // `::tcl::unsupported::corotype coroName` (and its
            // namespace-relative `tcl::unsupported::corotype` spelling) — a
            // flat lookup into the coroutine table
            // (`tcl-vm/src/cmd_coro.rs::cmd_corotype`): no eval fallback, no
            // `Frame` of its own. Both spellings share the same `make_spec`
            // and so the same traits. See
            // `commands/tcl/tcl_unsupported_corotype.rs`.
            "::tcl::unsupported::corotype",
            "tcl::unsupported::corotype",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            got, expected,
            "FRAMELESS_RUNTIME stamps drifted from the audited allow-list"
        );
    }

    #[test]
    fn dialect_filter() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get_for_surface("dict", Some(SurfaceQuery::core(Family::Tcl, "8.6")));
        assert!(spec.is_some());
        // dict is tcl8.5+ so should NOT be available in 8.4
        let spec84 = reg.get_for_surface("dict", Some(SurfaceQuery::core(Family::Tcl, "8.4")));
        assert!(spec84.is_none());
    }

    #[test]
    fn subcommand_arg_roles() {
        let reg = CommandRegistry::build_default();
        let bodies =
            reg.arg_indices_for_role("dict", &["for", "{k v}", "$d", "body"], ArgRole::Body);
        // dict for {varList} dictExpr body → body is at index 3 (subcmd=0, args 1-based +1)
        assert!(bodies.contains(&3));
    }

    #[test]
    fn variable_write_commands() {
        let reg = CommandRegistry::build_default();
        let set_vars = reg.arg_indices_for_role("set", &["x", "1"], ArgRole::VarWrite);
        assert_eq!(set_vars, vec![0]);
    }

    /// `unset x y z` names *every* argument as a variable (issue #774), not
    /// just the first, so all of them highlight as variables.
    #[test]
    fn unset_marks_every_name() {
        let reg = CommandRegistry::build_default();
        let vars = reg.arg_indices_for_role("unset", &["x", "y", "z"], ArgRole::VarWrite);
        assert_eq!(vars, vec![0, 1, 2]);
    }

    /// `unset -nocomplain -- a b` skips the leading options and names only the
    /// real variables (`a`, `b`), mirroring `lower_unset`.
    #[test]
    fn unset_skips_leading_options() {
        let reg = CommandRegistry::build_default();
        let vars =
            reg.arg_indices_for_role("unset", &["-nocomplain", "--", "a", "b"], ArgRole::VarWrite);
        assert_eq!(vars, vec![2, 3]);
    }

    /// `unset` recognises only `-nocomplain` / `--` as options — a dash-prefixed
    /// word like `-foo` is a real variable name (verified against tclsh), so it
    /// keeps its `VarWrite` role.
    #[test]
    fn unset_dash_name_is_a_variable() {
        let reg = CommandRegistry::build_default();
        // `unset -foo bar` — both are variables (no `--` needed to reach them).
        assert_eq!(
            reg.arg_indices_for_role("unset", &["-foo", "bar"], ArgRole::VarWrite),
            vec![0, 1]
        );
        // `unset -nocomplain -foo` — `-nocomplain` is skipped, `-foo` is a name.
        assert_eq!(
            reg.arg_indices_for_role("unset", &["-nocomplain", "-foo"], ArgRole::VarWrite),
            vec![1]
        );
    }

    #[test]
    fn arg_indices_for_role_trace_add_variable_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["add", "variable", "x", "write", "body"],
            ArgRole::VarWrite,
        );
        // +1 for subcommand offset.  arg "x" is at idx 2 in the
        // full args list (sub args[1] + 1).
        assert!(writes.contains(&2), "VarWrite writes={writes:?}");
    }

    /// `trace add execution` does NOT declare `VarWrite`
    /// (the second arg is a command name, not a variable).
    #[test]
    fn arg_indices_for_role_trace_add_execution_no_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["add", "execution", "foo", "enter", "body"],
            ArgRole::VarWrite,
        );
        assert!(writes.is_empty(), "VarWrite writes={writes:?}");
    }

    /// `trace add command`/`execution` trace a command *by name*, so the name
    /// argument is a `CommandName` reference (navigation follows it), while a
    /// `variable` trace's name is not.
    #[test]
    fn arg_indices_for_role_trace_add_command_name_reference() {
        let reg = CommandRegistry::build_default();
        for kind in ["command", "execution"] {
            let names = reg.arg_indices_for_role(
                "trace",
                &["add", kind, "foo", "enter", "body"],
                ArgRole::CommandName,
            );
            // +1 subcommand offset: `foo` is at full-args index 2.
            assert!(names.contains(&2), "{kind}: CommandName names={names:?}");
        }
        let var = reg.arg_indices_for_role(
            "trace",
            &["add", "variable", "x", "write", "body"],
            ArgRole::CommandName,
        );
        assert!(var.is_empty(), "a variable trace names no command: {var:?}");
    }

    /// `trace remove variable` behaves like `trace add variable`:
    /// both alias spellings flow through the same `VarWrite` query.
    #[test]
    fn arg_indices_for_role_trace_remove_variable_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["remove", "variable", "y", "write", "body"],
            ArgRole::VarWrite,
        );
        assert!(writes.contains(&2), "VarWrite writes={writes:?}");
    }

    /// `global` / `variable` / `upvar` carry
    /// `CREATES_DYNAMIC_BARRIER` so SSA's barrier-def walk knows the
    /// per-arg list belongs to `var_scoping`, not the role-driven
    /// `VarWrite` query.
    #[test]
    fn creates_dynamic_barrier_trait_marks_scope_aliases() {
        let reg = CommandRegistry::build_default();
        for cmd in &["global", "variable", "upvar"] {
            assert!(
                reg.get(cmd)
                    .unwrap()
                    .traits
                    .contains(Traits::CREATES_DYNAMIC_BARRIER),
                "{cmd} should carry CREATES_DYNAMIC_BARRIER",
            );
        }
        // `set` does NOT carry the trait — its VarWrite at arg 0 is
        // a single-target def, not a vararg list.
        assert!(
            !reg.get("set")
                .unwrap()
                .traits
                .contains(Traits::CREATES_DYNAMIC_BARRIER)
        );
    }

    /// `dict with` / `dict update` arg 0 (the dict variable) plays
    /// both `VarRead` and `VarWrite` roles. This is emitted via
    /// duplicate `(idx, role)` rows in the resolver (the multi-role
    /// observable behaviour is what consumers query).
    /// `DEFERS_BODY` marks the commands that **store** a body rather than
    /// run it, and it is the fail-safe direction that matters: a consumer
    /// asking "can an unreadable body word cost me the proof that this call
    /// completes?" must get `yes` for everything that has not declared
    /// otherwise.
    ///
    /// `BodyKind` cannot answer this — `proc` and `uplevel` are both
    /// `Structural`, because that descriptor answers *which frame*, not
    /// *when* — and neither can the absence of `control_arm_semantics`, which
    /// `uplevel` / `oo::define` share with `proc` (issue #1571, PR #1652
    /// review). (`apply` was in that list until issue #1656 gave its lambda
    /// position a typed frame boundary; it still declares no `DEFERS_BODY`,
    /// which is what this test checks.)
    #[test]
    fn defers_body_marks_only_stored_bodies() {
        use crate::body_kind::BodyKind;
        let mut reg = CommandRegistry::build_default();
        assert!(
            reg.get("proc")
                .unwrap()
                .traits
                .contains(Traits::DEFERS_BODY),
            "`proc` stores its body for a later call"
        );
        // Every command that runs its script as part of the same call — all
        // of which lack control-arm semantics exactly as `proc` does.
        for name in ["uplevel", "apply", "oo::define", "oo::objdefine", "eval"] {
            let spec = reg.get(name).unwrap_or_else(|| panic!("{name} is known"));
            assert!(
                !spec.traits.contains(Traits::DEFERS_BODY),
                "{name} executes its script argument at this call"
            );
        }
        // The trap the flag exists to close: same `BodyKind`, opposite timing.
        assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
        assert_eq!(reg.get("uplevel").unwrap().body_kind, BodyKind::Structural);
        // An iRules handler registration stores its body the same way.
        reg.load_irules();
        assert!(
            reg.get("when")
                .unwrap()
                .traits
                .contains(Traits::DEFERS_BODY)
        );
        // The flag is opt-in, so the silent default is the abstaining one.
        assert!(
            !CommandSpec::DEFAULT.traits.contains(Traits::DEFERS_BODY),
            "an undeclared command must never read as dormant"
        );
        assert!(
            !SubCommand::DEFAULT.traits.contains(Traits::DEFERS_BODY),
            "an undeclared subcommand must never read as dormant"
        );
    }

    /// `apply`'s lambda position is a typed **frame boundary**: the script
    /// runs as part of this call, in a fresh procedure frame (issue #1656).
    ///
    /// The declaration is what lets a consumer walking a statement stream
    /// treat an unreadable `apply $lambda` as the missing answer it is — the
    /// role alone never reached the question, so the walk claimed past a
    /// lambda that can raise.
    ///
    /// Gated on the role, not on the argument number: only the
    /// [`ArgRole::LambdaLiteral`] word is the script. `apply`'s trailing
    /// `arg1 arg2 …` are ordinary values bound to the lambda's parameters.
    #[test]
    fn apply_types_its_lambda_position_as_a_frame_boundary() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.control_arm_semantics("apply", &["{{} {puts hi}}"], 0),
            Some(ControlArmSemantics::FrameBoundary),
            "`apply`'s lambda runs now, in a fresh frame"
        );
        for index in 1..4 {
            assert_eq!(
                reg.control_arm_semantics("apply", &["{{a b} {puts hi}}", "1", "2"], index),
                None,
                "argument {index} is a value bound to a parameter, not a script"
            );
        }
        // The role is what the declaration keys on, so it survives a rename of
        // the position and answers for any future command sharing the shape.
        assert_eq!(
            reg.arg_indices_for_role("apply", &["{{} {puts hi}}"], ArgRole::LambdaLiteral),
            vec![0]
        );
    }

    /// Every shipped command carrying [`ArgRole::LambdaLiteral`] types that
    /// position, so the analyser's "read the body only where the registry says
    /// what running it means" gate is never the thing deciding a shipped
    /// command's fate.
    ///
    /// Pinned rather than assumed (issue #1656): `apply` is the only such
    /// command today, so dropping that gate would change nothing here — but a
    /// pack, or a future built-in, can carry the role without typing it, and
    /// then the gate is what keeps an untyped script that runs *now* from
    /// being walked into and claimed. This test says out loud that the shipped
    /// set does not exercise it.
    #[test]
    fn every_shipped_lambda_position_is_typed() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        let mut seen = 0usize;
        for name in names {
            for argc in 1..=4 {
                let args = vec!["{{} {puts hi}}"; argc];
                for index in reg.arg_indices_for_role(&name, &args, ArgRole::LambdaLiteral) {
                    seen += 1;
                    assert!(
                        reg.control_arm_semantics(&name, &args, index).is_some(),
                        "{name}/{argc} carries a lambda at {index} without typing what running \
                         it means"
                    );
                }
            }
        }
        assert!(seen > 0, "the sweep must not be vacuous");
    }

    /// Every command that carries an untyped [`ArgRole::Body`] position and
    /// runs it *now*, pinned by name.
    ///
    /// The analyser reads the absence of `DEFERS_BODY` as "this body runs as
    /// part of this invocation" and, since issue #1672, lets such a body stop
    /// its fall-through walk. That makes a *missing* `DEFERS_BODY` a silent
    /// wrong answer rather than a missing one, so the inventory is pinned:
    /// adding a Body-role command fails this test until its author puts it on
    /// one of the two lists below, which is the moment to ask the question.
    ///
    /// Store-only forms do not appear here — they carry `DEFERS_BODY` and the
    /// sweep skips them. Commands the registry *types* do not appear either:
    /// a typed arm is answered by [`ControlArmSemantics`], not by this trait.
    ///
    /// Every entry was measured against tclsh 8.6.16 and 9.0.4, which agreed
    /// byte-for-byte, with the shape `proc p {} { FORM {error stop}; set
    /// ::reached 1 }`: `::reached` unset means the body ran here.
    const BODY_RUNS_NOW: &[&str] = &[
        // Core Tcl: script evaluators and definition scripts.
        "::tcl::dict::for",
        "::tcl::dict::map",
        "::tcl::dict::update",
        "::tcl::dict::with",
        "eval",
        "oo::define",
        "oo::objdefine",
        "time",
        "timerate",
        "uplevel",
        // Runs its body now, but *absorbs* its completion — the harness
        // catches a failing test body and reports it. That is a completion
        // boundary, which is a typed answer this trait cannot give, so it
        // sits here and the walk over-abstains on it. Tracked separately.
        "tcltest::test",
        // tcllib: iterators and definition scripts.
        "control::do",
        "fileutil::foreachLine",
        "lfilter",
        "snit::compile",
        "snit::type",
        "snit::widget",
        "snit::widgetadaptor",
        "stooop::class",
        "uri::register",
        // itcl: `class` runs its definition script; `body` and `configbody`
        // install a body against an already-declared member and look like
        // store-only forms, but no itcl is installable in this environment,
        // so they are recorded as unmeasured rather than marked on a guess.
        "itcl::body",
        "itcl::class",
        "itcl::configbody",
        // iRules: these run the script now, in a different flow context.
        "clientside",
        "peer",
        "serverside",
    ];

    /// The gate for [`BODY_RUNS_NOW`]: no command may carry an untyped
    /// `ArgRole::Body` position without an explicit stance on whether that
    /// body runs now or is stored.
    #[test]
    fn every_untyped_body_role_declares_whether_its_body_runs_now() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        let mut unclassified: Vec<String> = Vec::new();
        let mut seen_runs_now = 0usize;
        for name in &names {
            // Role resolution is argv-shaped, so probe the argument counts a
            // grammar could take, and the leading words that select the
            // script-bearing subforms of the ensembles in this sweep. Both
            // questions are asked per shape, exactly as the analyser asks
            // them: `DEFERS_BODY` can live on a subcommand rather than the
            // command (`package ifneeded`), so a spec-level test would miss it.
            let mut untyped_body = false;
            for argc in 1..=6usize {
                for lead in ["x", "idle", "ifneeded", "0"] {
                    let mut args = vec!["x"; argc];
                    args[0] = lead;
                    // One name can carry more than one spec, resolved by
                    // dialect — `after` has a Tcl spec and an iRules one, and
                    // a sweep that looked at only one would let the other lose
                    // its trait unnoticed. Iterating the specs (rather than a
                    // list of points) also keeps every probe at a point the
                    // command actually resolves at: an unresolvable call
                    // answers with an empty trait set, which is indistinguish-
                    // able from a spec whose only trait was `DEFERS_BODY`
                    // until it was dropped — precisely the regression this
                    // gate exists to catch.
                    for spec in reg.specs(name) {
                        // Probe at a point the spec actually resolves under:
                        // its own first row, or any Tcl release when it names
                        // no provider.
                        let point = spec.surface.and_then(|rows| rows.first()).map_or(
                            SurfaceQuery::any_release(Family::Tcl),
                            |row| match row.provider {
                                SpecProvider::Core(family) => SurfaceQuery::any_release(family),
                                SpecProvider::Package(_) => SurfaceQuery::any_release(Family::Tcl),
                            },
                        );
                        let traits = reg.invocation_traits(name, &args, Some(point));
                        if spec.traits.contains(Traits::DEFERS_BODY)
                            || traits.contains(Traits::DEFERS_BODY)
                            // A `CONTROL_FLOW` command is the registry's own
                            // business: the analyser refuses to walk one whose
                            // arm it cannot type rather than reading the body,
                            // so a malformed probe shape of `if` / `try` is
                            // not a missing stance.
                            || traits.contains(Traits::CONTROL_FLOW)
                        {
                            continue;
                        }
                        for index in reg.arg_indices_for_role(name, &args, ArgRole::Body) {
                            if reg.control_arm_semantics(name, &args, index).is_none() {
                                untyped_body = true;
                            }
                        }
                    }
                }
            }
            if !untyped_body {
                continue;
            }
            if BODY_RUNS_NOW.contains(&name.as_str()) {
                seen_runs_now += 1;
            } else {
                unclassified.push(name.clone());
            }
        }
        assert!(
            seen_runs_now > 0,
            "the sweep must not be vacuous — no runs-now command was reached"
        );
        assert!(
            unclassified.is_empty(),
            "these commands carry an untyped ArgRole::Body and declare no \
             stance: either they store the body (give the spec \
             Traits::DEFERS_BODY, with an oracle row) or they run it now (add \
             them to BODY_RUNS_NOW, with an oracle row). Leaving them out \
             makes the analyser's fall-through walk treat their body as one \
             that can stop control — see issue #1672. Unclassified: {unclassified:?}"
        );
    }

    /// `DEFERS_BODY` and typed control-arm semantics are answers to different
    /// questions, and today no command gives both: everything the registry
    /// types (`if`, `switch`, `namespace eval`, `catch`, `try`, the loops,
    /// `apply`'s lambda)
    /// runs the arm it types.
    ///
    /// That is *why* a consumer cannot currently observe which of the two
    /// checks fires first — dropping either leaves every shipped command
    /// classified the same. It is a property of today's data, not of the
    /// question, so it is pinned rather than assumed: a future command that
    /// stored a body the registry also typed as an arm would make the order
    /// load-bearing, and this is the assertion that says so out loud before
    /// the analyser silently starts trusting a dormant body's arm.
    #[test]
    fn no_command_both_defers_a_body_and_types_it_as_a_control_arm() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let deferring: Vec<String> = reg
            .command_names()
            .filter(|name| {
                reg.get(name)
                    .is_some_and(|spec| spec.traits.contains(Traits::DEFERS_BODY))
            })
            .map(str::to_owned)
            .collect();
        assert!(!deferring.is_empty(), "the sweep must not be vacuous");
        // Positive control: the probe below has to be able to *see* a typed
        // arm, or the sweep proves nothing. Role resolution is argv-shaped, so
        // an empty argument list makes every command answer `None` — including
        // `if`. Probe each deferring command across the argument counts its
        // grammar could take instead.
        assert_eq!(
            reg.control_arm_semantics("if", &["{$c}", "{body}"], 1),
            Some(ControlArmSemantics::Selected),
            "the probe must detect a typed arm, else the sweep is inert"
        );
        for name in deferring {
            for argc in 1..=6 {
                let args = vec!["x"; argc];
                for index in 0..argc {
                    assert_eq!(
                        reg.control_arm_semantics(&name, &args, index),
                        None,
                        "{name}/{argc} declares DEFERS_BODY but types argument {index} as a \
                         control arm; `unreadable_body_is_material`'s check order now decides \
                         its behaviour"
                    );
                }
            }
        }
    }

    /// Every spec defaults to `BodyKind::Plain` unless it
    /// opts into `Structural`.
    #[test]
    fn body_kind_default_plain() {
        use crate::body_kind::BodyKind;
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("set").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("if").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("while").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("foreach").unwrap().body_kind, BodyKind::Plain);
    }

    /// `proc` / `oo::class` / `oo::define` / `oo::objdefine`
    /// stamp `Structural` so SSA skips their body args from the
    /// enclosing block's data flow.
    #[test]
    fn body_kind_structural_marks() {
        use crate::body_kind::BodyKind;
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
        assert_eq!(
            reg.get("oo::class").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("oo::define").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("oo::objdefine").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("snit::method").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("snit::typemethod").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("uri::register").unwrap().body_kind,
            BodyKind::Structural
        );
    }

    /// iRules `when` event handler bodies are structural.
    #[test]
    fn body_kind_irules_when_structural() {
        use crate::body_kind::BodyKind;
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        assert_eq!(reg.get("when").unwrap().body_kind, BodyKind::Structural);
    }

    /// `plain_body_arg_indices` surfaces the same-frame control-flow /
    /// `eval` bodies a nested dispatch (e.g. `TclOO` `my method`) still
    /// executes inside — every `Plain`-kind `ArgRole::Body` argument.
    #[test]
    fn plain_body_arg_indices_covers_same_frame_bodies() {
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.plain_body_arg_indices("if", &["1", "{body}"]), vec![1]);
        assert_eq!(
            reg.plain_body_arg_indices("while", &["1", "{body}"]),
            vec![1]
        );
        assert_eq!(
            reg.plain_body_arg_indices("foreach", &["v", "$list", "{body}"]),
            vec![2]
        );
        assert_eq!(reg.plain_body_arg_indices("eval", &["{body}"]), vec![0]);
        assert_eq!(
            reg.plain_body_arg_indices("catch", &["{body}", "res"]),
            vec![0]
        );
    }

    /// `Structural` bodies (`proc`, `oo::class create`, `uplevel`,
    /// `namespace eval`) never come back from `plain_body_arg_indices` —
    /// they run in a definition / different-frame context, so a dispatch
    /// found inside is not still the caller's scope.
    #[test]
    fn plain_body_arg_indices_excludes_structural_bodies() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.plain_body_arg_indices("proc", &["name", "args", "{body}"]),
            Vec::<usize>::new()
        );
        assert_eq!(
            reg.plain_body_arg_indices("uplevel", &["1", "{body}"]),
            Vec::<usize>::new()
        );
        assert_eq!(
            reg.plain_body_arg_indices("namespace", &["eval", "ns", "{body}"]),
            Vec::<usize>::new()
        );
    }

    /// An unknown command name (a user proc, a `TclOO` method body's own
    /// `unknownProc arg` call) has no registry spec at all — returns empty
    /// rather than panicking.
    #[test]
    fn plain_body_arg_indices_unknown_command_is_empty() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.plain_body_arg_indices("myUserProc", &["a", "b"]),
            Vec::<usize>::new()
        );
    }

    /// `body_arg_implicit_args` defaults to 0 and is set on
    /// `fileutil::updateInPlace` (which appends file contents to
    /// the body's first command at runtime).
    #[test]
    fn body_arg_implicit_args_defaults_zero_except_fileutil_updateinplace() {
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("set").unwrap().body_arg_implicit_args, 0);
        assert_eq!(reg.get("proc").unwrap().body_arg_implicit_args, 0);
        assert_eq!(
            reg.get("fileutil::updateInPlace")
                .unwrap()
                .body_arg_implicit_args,
            1,
        );
    }

    #[test]
    fn arg_indices_for_role_dict_with_multirole() {
        let reg = CommandRegistry::build_default();
        let reads = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarRead);
        let writes = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarWrite);
        assert!(reads.contains(&1), "VarRead reads={reads:?}");
        assert!(writes.contains(&1), "VarWrite writes={writes:?}");
    }

    #[test]
    fn arg_indices_for_role_dict_update_multirole() {
        let reg = CommandRegistry::build_default();
        let reads = reg.arg_indices_for_role(
            "dict",
            &["update", "$var", "k", "vname", "body"],
            ArgRole::VarRead,
        );
        let writes = reg.arg_indices_for_role(
            "dict",
            &["update", "$var", "k", "vname", "body"],
            ArgRole::VarWrite,
        );
        assert!(reads.contains(&1), "VarRead reads={reads:?}");
        assert!(writes.contains(&1), "VarWrite writes={writes:?}");
    }

    #[test]
    fn dynamic_arg_role_resolution() {
        let reg = CommandRegistry::build_default();
        // if expr body elseif expr body else body
        let roles = reg.arg_indices_for_role(
            "if",
            &[
                "$x", "then", "body1", "elseif", "$y", "body2", "else", "body3",
            ],
            ArgRole::Body,
        );
        // Bodies are at positions 2, 5, 7
        assert!(roles.contains(&2));
        assert!(roles.contains(&5));
        assert!(roles.contains(&7));
    }

    #[test]
    fn load_irules_dialect() {
        let mut reg = CommandRegistry::build_default();
        assert!(reg.get("HTTP::header").is_none()); // not loaded yet
        reg.load_irules();
        assert!(reg.get("HTTP::header").is_some());
        assert!(reg.len() > 200); // should have 1000+ commands now
    }

    #[test]
    fn irules_command_has_irules_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let spec = reg.get("HTTP::header").unwrap();
        assert_eq!(spec.surface, Some(SpecSurface::IRULES));
    }

    #[test]
    fn irules_idempotent_load() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let count1 = reg.len();
        reg.load_irules(); // second load should be no-op
        assert_eq!(reg.len(), count1);
    }

    #[test]
    fn default_includes_stdlib_and_tcllib() {
        let reg = CommandRegistry::build_default();
        // stdlib and tcllib are loaded by default
        assert!(reg.len() > 200);
    }

    #[test]
    fn load_tk_dialect() {
        // Tk is part of the always-known base registry now, so it is present
        // by default and a later `load_surface(Tk)` is an idempotent no-op.
        let reg = CommandRegistry::build_default();
        assert!(
            reg.get("grid").is_some(),
            "Tk commands are loaded by default"
        );
        let base_count = reg.len();
        let mut reg2 = CommandRegistry::build_default();
        reg2.load_surface(SurfaceLayer::Package("Tk"));
        assert_eq!(
            reg2.len(),
            base_count,
            "loading the Tk surface is a no-op after the default load"
        );
    }

    #[test]
    fn load_iapps_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_surface(SurfaceLayer::Package("iapps"));
        assert!(reg.len() > 100);
    }

    #[test]
    fn load_expect_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_surface(SurfaceLayer::Package("expect"));
        assert!(reg.get("expect").is_some() || reg.get("spawn").is_some());
    }

    #[test]
    fn resolve_call_unknown_command_returns_none() {
        let reg = CommandRegistry::build_default();
        assert!(reg.resolve_call("no_such_cmd", &[], None).is_none());
    }

    #[test]
    fn resolve_invocation_retains_words_and_canonicalises_the_registry_name() {
        let reg = CommandRegistry::build_default();
        let args = ["create", "key", "value"];
        let resolved = reg
            .resolve_invocation(
                "::dict",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("global dict spelling resolves through the registry");

        assert_eq!(resolved.words.head_literal(), Some("::dict"));
        assert_eq!(
            resolved.words.arguments(),
            InvocationArguments::literals(&args)
        );
        assert_eq!(resolved.canonical_command, "dict");
        let sub = resolved
            .subcommand
            .resolved()
            .expect("dict create subcommand");
        assert_eq!(sub.spelling, "create");
        assert_eq!(sub.canonical_name, "create");
        assert_eq!(
            resolved.subcommand.kind(),
            Some(crate::SubcommandResolutionKind::Exact)
        );
        assert_eq!(resolved.semantics.argument_offset, 1);
    }

    #[test]
    fn structured_resolution_keeps_dynamic_subcommands_indeterminate() {
        let reg = CommandRegistry::build_default();
        let arguments = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("text"),
        ];
        let resolved = reg
            .resolve_structured_invocation(
                InvocationWords::structured(crate::InvocationWord::Literal("string"), &arguments),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .expect("a literal command head is registry-known");

        assert!(matches!(
            resolved.subcommand,
            SubcommandResolution::Indeterminate {
                word_kind: crate::InvocationWordKind::Dynamic
            }
        ));
        assert!(resolved.form.is_none());
        assert_eq!(
            resolved.semantics.operation,
            crate::SemanticOperationId::Invoke,
            "a dynamic subcommand cannot select specialised subcommand metadata"
        );
    }

    #[test]
    fn expanded_argument_does_not_match_an_arity_form() {
        let reg = CommandRegistry::build_default();
        let arguments = [crate::InvocationWord::Expanded];
        let resolved = reg
            .resolve_structured_invocation(
                InvocationWords::structured(crate::InvocationWord::Literal("incr"), &arguments),
                None,
            )
            .resolved()
            .expect("a literal command head is registry-known");

        assert!(resolved.form.is_none());
        assert_eq!(
            resolved.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr),
            "the command-level common operation remains visible without claiming a form"
        );
    }

    #[test]
    fn literal_adapter_matches_the_structured_literal_view() {
        let reg = CommandRegistry::build_default();
        let arguments = ["counter"];
        let adapter = reg
            .resolve_invocation("incr", &arguments, None)
            .expect("literal adapter resolves");
        let structured = reg
            .resolve_structured_invocation(InvocationWords::literals("incr", &arguments), None)
            .resolved()
            .expect("structured literal input resolves");

        assert_eq!(adapter.canonical_command, structured.canonical_command);
        assert_eq!(adapter.subcommand, structured.subcommand);
        assert_eq!(
            adapter.form.map(|form| form.name),
            structured.form.map(|form| form.name)
        );
        assert_eq!(adapter.semantics.operation, structured.semantics.operation);
    }

    #[test]
    fn structured_resolution_reports_computed_and_unknown_command_heads() {
        let reg = CommandRegistry::build_default();
        let no_arguments: [crate::InvocationWord<'_>; 0] = [];

        let computed = reg.resolve_structured_invocation(
            InvocationWords::structured(crate::InvocationWord::Dynamic, &no_arguments),
            None,
        );
        assert_eq!(
            computed.unresolved(),
            Some(crate::InvocationResolutionUnresolved::ComputedHead {
                word_kind: crate::InvocationWordKind::Dynamic,
            })
        );

        let no_literal_arguments: [&str; 0] = [];
        let unknown = reg.resolve_structured_invocation(
            InvocationWords::literals("not-a-registry-command", &no_literal_arguments),
            None,
        );
        assert_eq!(
            unknown.unresolved(),
            Some(crate::InvocationResolutionUnresolved::UnknownLiteralHead {
                spelling: "not-a-registry-command",
            })
        );
    }

    #[test]
    fn irules_placement_contract_owns_declaration_and_execution_contexts() {
        use crate::events::{IrulesCommandPlacement as Placement, IrulesExecutionContext as Ctx};

        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        for declaration in ["when", "proc", "timing", "priority"] {
            assert_eq!(
                registry.irules_command_placement(declaration, Ctx::TopLevel),
                Placement::Allowed,
                "{declaration} is an iRules top-level declaration"
            );
            assert_eq!(
                registry.irules_command_placement(declaration, Ctx::EventBody),
                Placement::RequiresTopLevel,
                "{declaration} cannot be nested in an event"
            );
            assert_eq!(
                registry.irules_command_placement(declaration, Ctx::ProcedureBody),
                Placement::RequiresTopLevel,
                "{declaration} cannot be nested in a proc"
            );
        }
        for executable in ["set", "HTTP::uri", "call", "user_proc"] {
            assert_eq!(
                registry.irules_command_placement(executable, Ctx::TopLevel),
                Placement::RequiresEventOrProcedure,
                "{executable} cannot execute at iRules top level"
            );
            assert_eq!(
                registry.irules_command_placement(executable, Ctx::EventBody),
                Placement::Allowed
            );
            assert_eq!(
                registry.irules_command_placement(executable, Ctx::ProcedureBody),
                Placement::Allowed
            );
        }
    }

    #[test]
    fn irules_declaration_owner_requires_braced_source_bodies() {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        let events = crate::events::EventRegistry::build();
        let declaration = |name: &str, args: &[&str]| {
            let body_index = if name == "proc" {
                2
            } else {
                args.len().saturating_sub(1)
            };
            let tokens = args
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    tcl_lexer::Token::new(
                        if index == body_index {
                            tcl_lexer::TokenType::Str
                        } else {
                            tcl_lexer::TokenType::Esc
                        },
                        tcl_lexer::Span::empty(0),
                    )
                })
                .collect::<Vec<_>>();
            let single = vec![true; args.len()];
            let closed = tokens
                .iter()
                .map(|token| token.kind == tcl_lexer::TokenType::Str)
                .collect::<Vec<_>>();
            registry.irules_top_level_declaration(
                name,
                crate::events::IrulesDeclarationArguments::new(args, &tokens, &single, &closed)
                    .expect("parallel source-word views"),
                &events,
            )
        };
        assert!(matches!(
            declaration("when", &["HTTP_REQUEST", "set x 1"]),
            Some(Declaration::Event { .. })
        ));
        assert!(matches!(
            declaration("::when", &["HTTP_REQUEST", "set x 1"]),
            Some(Declaration::Event { .. })
        ));
        assert!(declaration("when", &["BOGUS_EVENT", "pool x"]).is_none());
        for valid in [
            &["HTTP_REQUEST", "body"][..],
            &["HTTP_REQUEST", "priority", "100", "body"][..],
            &["HTTP_REQUEST", "timing", "on", "body"][..],
            &[
                "HTTP_REQUEST",
                "priority",
                "100",
                "timing",
                "disable",
                "body",
            ][..],
            &["HTTP_REQUEST", "priority", "0", "body"][..],
            &["HTTP_REQUEST", "priority", "1000", "body"][..],
        ] {
            assert!(declaration("when", valid).is_some());
        }
        for invalid in [
            &["HTTP_REQUEST"][..],
            &["HTTP_REQUEST", "body", "trailing"][..],
            &["HTTP_REQUEST", "priority", "bad", "body"][..],
            &["HTTP_REQUEST", "priority", "-1", "body"][..],
            &["HTTP_REQUEST", "priority", "1001", "body"][..],
            &["HTTP_REQUEST", "timing", "maybe", "body"][..],
            &["HTTP_REQUEST", "timing", "on", "priority", "100", "body"][..],
        ] {
            assert!(declaration("when", invalid).is_none());
        }
        assert!(matches!(
            declaration("proc", &["p", "", "return"]),
            Some(Declaration::Procedure { .. })
        ));
        for value in ["0", "500", "1000"] {
            assert_eq!(
                declaration("priority", &[value]),
                Some(Declaration::Priority {
                    value: value.parse().unwrap()
                })
            );
        }
        for invalid in ["-1", "1001", "bad"] {
            assert!(declaration("priority", &[invalid]).is_none());
        }
        assert_eq!(
            declaration("timing", &["enable"]),
            Some(Declaration::Timing { enabled: true })
        );
        for malformed in [&["p", ""][..], &["p", "", "return", "extra"][..]] {
            assert!(declaration("proc", malformed).is_none());
        }
    }

    #[test]
    fn irules_declaration_owner_rejects_bare_and_quoted_source_bodies() {
        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        for (name, args, body_index) in [
            ("when", &["HTTP_REQUEST", "set x 1"][..], 1),
            ("proc", &["p", "", "return"][..], 2),
        ] {
            let mut tokens =
                vec![
                    tcl_lexer::Token::new(tcl_lexer::TokenType::Esc, tcl_lexer::Span::empty(0),);
                    args.len()
                ];
            let single = vec![true; args.len()];
            let closed = vec![false; args.len()];
            for (kind, in_quote) in [
                (tcl_lexer::TokenType::Esc, false),
                (tcl_lexer::TokenType::Esc, true),
                // Lexer recovery uses Str for `{body` at EOF; the separate
                // closed-brace fact must keep it out of declarations too.
                (tcl_lexer::TokenType::Str, false),
            ] {
                tokens[body_index] = tcl_lexer::Token {
                    kind,
                    in_quote,
                    ..tokens[body_index]
                };
                let arguments =
                    crate::events::IrulesDeclarationArguments::new(args, &tokens, &single, &closed)
                        .expect("parallel source-word views");
                assert!(
                    registry
                        .irules_top_level_declaration_shape(name, arguments)
                        .is_none(),
                    "{name} must reject a non-closed-braced body"
                );
            }
        }
    }

    #[test]
    fn literal_form_selectors_choose_the_longest_static_match() {
        const FORMS: &[CommandForm] = &[
            CommandForm {
                name: "tag-one",
                arity: Arity::exact(1),
                literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag"])),
                ..CommandForm::DEFAULT
            },
            CommandForm {
                name: "tag-fallback",
                arity: Arity::exact(3),
                literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag"])),
                ..CommandForm::DEFAULT
            },
            CommandForm {
                name: "tag-bind",
                arity: Arity::exact(3),
                literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag", "bind"])),
                ..CommandForm::DEFAULT
            },
        ];
        const SPEC: CommandSpec = CommandSpec {
            name: "literal-form-fixture",
            arity: Arity::any(),
            command_forms: FORMS,
            ..CommandSpec::DEFAULT
        };

        let mut registry = CommandRegistry::build_default();
        registry.insert(SPEC);
        let form = |args: &[&str]| {
            registry
                .resolve_invocation("literal-form-fixture", args, None)
                .and_then(|resolved| resolved.form)
                .map(|form| form.name)
        };

        assert_eq!(form(&["tag"]), Some("tag-one"), "arity disambiguates");
        assert_eq!(
            form(&["tag", "bind", "script"]),
            Some("tag-bind"),
            "the longer selector overrides its completed prefix"
        );
        assert_eq!(
            form(&["ta", "bi", "script"]),
            Some("tag-bind"),
            "each selector word still accepts a unique abbreviation"
        );
        assert_eq!(
            form(&["tag", "other", "value"]),
            Some("tag-fallback"),
            "a static non-match proves that the shorter selector applies"
        );

        let dynamic = [
            InvocationWord::Literal("tag"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("script"),
        ];
        let resolved = registry
            .resolve_structured_invocation(
                InvocationWords::structured(
                    InvocationWord::Literal("literal-form-fixture"),
                    &dynamic,
                ),
                None,
            )
            .resolved()
            .expect("fixture command resolves");
        assert!(
            resolved.form.is_none(),
            "a dynamic word that could extend a completed selector must abstain"
        );
    }

    #[test]
    fn unresolved_subcommands_never_fall_back_to_a_top_level_form() {
        const TOP_LEVEL_FORM: CommandForm = CommandForm {
            name: "top-level-form",
            arity: Arity::exact(1),
            ..CommandForm::DEFAULT
        };
        const SUBCOMMANDS: &[SubCommand] = &[
            SubCommand {
                name: "alpha",
                ..SubCommand::DEFAULT
            },
            SubCommand {
                name: "alpine",
                ..SubCommand::DEFAULT
            },
        ];
        const SPEC: CommandSpec = CommandSpec {
            name: "subcommand-form-fixture",
            arity: Arity::any(),
            subcommands: SUBCOMMANDS,
            command_forms: &[TOP_LEVEL_FORM],
            ..CommandSpec::DEFAULT
        };

        let mut reg = CommandRegistry::build_default();
        reg.insert(SPEC);
        let dynamic_arguments = [crate::InvocationWord::Dynamic];
        let dynamic = reg
            .resolve_structured_invocation(
                InvocationWords::structured(
                    crate::InvocationWord::Literal("subcommand-form-fixture"),
                    &dynamic_arguments,
                ),
                None,
            )
            .resolved()
            .expect("fixture command resolves");
        assert!(matches!(
            dynamic.subcommand,
            SubcommandResolution::Indeterminate { .. }
        ));
        assert!(dynamic.form.is_none());
        assert!(matches!(
            dynamic.facts().subcommand,
            crate::OwnedSubcommandResolution::Indeterminate {
                word_kind: crate::InvocationWordKind::Dynamic
            }
        ));

        let ambiguous = reg
            .resolve_invocation("subcommand-form-fixture", &["al"], None)
            .expect("fixture command resolves");
        assert!(matches!(
            ambiguous.subcommand,
            SubcommandResolution::Ambiguous { .. }
        ));
        assert!(ambiguous.form.is_none());
        assert!(matches!(
            ambiguous.facts().subcommand,
            crate::OwnedSubcommandResolution::Ambiguous { ref spelling } if spelling == "al"
        ));

        let unknown = reg
            .resolve_invocation("subcommand-form-fixture", &["missing"], None)
            .expect("fixture command resolves");
        assert!(matches!(
            unknown.subcommand,
            SubcommandResolution::Unknown { .. }
        ));
        assert!(unknown.form.is_none());
        assert!(matches!(
            unknown.facts().subcommand,
            crate::OwnedSubcommandResolution::Unknown { ref spelling } if spelling == "missing"
        ));
    }

    #[test]
    fn resolve_invocation_applies_registry_unique_prefix_subcommand_resolution() {
        let reg = CommandRegistry::build_default();
        let args = ["le", "hello"];
        let resolved = reg
            .resolve_invocation(
                "string",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("string is registry-known");

        let sub = resolved
            .subcommand
            .resolved()
            .expect("string le is a valid unique prefix");
        assert_eq!(sub.spelling, "le");
        assert_eq!(sub.canonical_name, "length");
        assert_eq!(
            resolved.subcommand.kind(),
            Some(crate::SubcommandResolutionKind::UniquePrefix)
        );
        assert_eq!(
            resolved.semantics.return_type,
            Some(crate::types::TclType::Int)
        );
    }

    #[test]
    fn resolve_invocation_preserves_an_ambiguous_subcommand_outcome() {
        let reg = CommandRegistry::build_default();
        let args = ["t", "hello"];
        let resolved = reg
            .resolve_invocation(
                "string",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("the command head remains known");

        assert!(matches!(
            resolved.subcommand,
            SubcommandResolution::Ambiguous { spelling: "t" }
        ));
        assert_eq!(
            resolved.subcommand.kind(),
            Some(crate::SubcommandResolutionKind::Ambiguous)
        );
        assert!(resolved.form.is_none());
    }

    #[test]
    fn resolve_invocation_preserves_an_unknown_subcommand_outcome() {
        let reg = CommandRegistry::build_default();
        let args = ["does-not-exist"];
        let resolved = reg
            .resolve_invocation(
                "string",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("the command head remains known");

        assert!(matches!(
            resolved.subcommand,
            SubcommandResolution::Unknown {
                spelling: "does-not-exist"
            }
        ));
        assert_eq!(
            resolved.subcommand.kind(),
            Some(crate::SubcommandResolutionKind::Unknown)
        );
    }

    #[test]
    fn resolve_invocation_projects_forms_without_backend_hooks() {
        let reg = CommandRegistry::build_default();
        let args = ["counter"];
        let resolved = reg
            .resolve_invocation("incr", &args, None)
            .expect("incr is registry-known");

        let form = resolved.form.expect("implicit incr form");
        assert_eq!(form.name, "implicit");
        assert_eq!(form.arity, Arity::exact(1));
        assert_eq!(form.arg_roles, &[(0, ArgRole::VarWrite)]);
        assert_eq!(resolved.semantics.arg_roles, form.arg_roles);
        assert_eq!(
            resolved.semantics.return_type,
            Some(crate::types::TclType::Int)
        );
        assert_eq!(
            resolved.semantics.lowering_hook,
            Some(LoweringHookId::Incr),
            "the common lowering descriptor survives the target-neutral projection"
        );
        assert_eq!(
            resolved.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
        );
    }

    #[test]
    fn resolve_invocation_identifies_channel_write_without_selecting_a_backend() {
        let reg = CommandRegistry::build_default();
        let args = ["hello"];
        let resolved = reg
            .resolve_invocation("puts", &args, Some(SurfaceQuery::core(Family::Tcl, "8.6")))
            .expect("puts resolves");

        assert_eq!(
            resolved.semantics.operation,
            crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ChannelWrite)
        );
        assert_eq!(
            reg.command_names_for_semantic_operation(crate::SemanticOperationId::Intrinsic(
                crate::IntrinsicId::ChannelWrite,
            ))
            .collect::<Vec<_>>(),
            vec!["puts"]
        );
    }

    #[test]
    fn semantic_operation_resolution_prefers_form_then_subcommand_then_command() {
        const FORM: CommandForm = CommandForm {
            name: "form",
            arity: Arity::exact(1),
            semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
                LoweringHookId::Set,
            )),
            ..CommandForm::DEFAULT
        };
        const SUB: SubCommand = SubCommand {
            name: "sub",
            arity: Arity::at_least(0),
            subcommand_forms: &[FORM],
            semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
                LoweringHookId::Incr,
            )),
            ..SubCommand::DEFAULT
        };
        const SPEC: CommandSpec = CommandSpec {
            name: "semantic-operation-precedence",
            arity: Arity::at_least(0),
            subcommands: &[SUB],
            semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
                LoweringHookId::Return,
            )),
            ..CommandSpec::DEFAULT
        };

        let mut reg = CommandRegistry::build_default();
        reg.insert(SPEC);

        let command = reg
            .resolve_invocation("semantic-operation-precedence", &[], None)
            .expect("command form resolves");
        assert_eq!(
            command.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
        );

        let sub = reg
            .resolve_invocation("semantic-operation-precedence", &["sub"], None)
            .expect("subcommand form resolves");
        assert_eq!(
            sub.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
        );

        let form = reg
            .resolve_invocation("semantic-operation-precedence", &["sub", "argument"], None)
            .expect("subcommand form resolves");
        assert_eq!(
            form.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        );
    }

    #[test]
    fn resolve_invocation_exposes_subcommand_effects_through_one_semantic_view() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let args = ["insert", "x-demo", "value"];
        let resolved = reg
            .resolve_invocation(
                "HTTP::header",
                &args,
                Some(SurfaceQuery::any_release(Family::F5Irules)),
            )
            .expect("HTTP::header insert resolves in the iRules dialect");

        assert!(resolved.semantics.traits.contains(Traits::PURE));
        assert_eq!(resolved.semantics.side_effects.len(), 1);
        let effect = resolved.semantics.side_effects[0];
        assert_eq!(
            effect.target,
            crate::side_effects::SideEffectTarget::HttpHeader
        );
        assert!(effect.reads);
        assert!(effect.writes);
    }

    #[test]
    fn resolved_call_and_resolved_invocation_share_selection() {
        let reg = CommandRegistry::build_default();
        let args = ["counter", "5"];
        let common = reg
            .resolve_invocation("incr", &args, None)
            .expect("incr resolves");
        let legacy = reg
            .resolve_call("incr", &args, None)
            .expect("legacy compatibility resolver remains available");

        assert_eq!(common.canonical_command, legacy.spec.name);
        assert_eq!(
            common.form.map(|form| form.name),
            legacy.form.map(|form| form.name)
        );
        assert_eq!(common.semantics.lowering_hook, legacy.lowering_hook);
        assert_eq!(common.semantics.arity, legacy.arity());
    }

    #[test]
    fn resolve_call_top_level_command() {
        let reg = CommandRegistry::build_default();
        let resolved = reg.resolve_call("set", &["x", "1"], None).unwrap();
        assert_eq!(resolved.spec.name, "set");
        assert!(resolved.sub.is_none());
    }

    #[test]
    fn resolve_call_subcommand() {
        let reg = CommandRegistry::build_default();
        let resolved = reg
            .resolve_call(
                "dict",
                &["create", "k", "v"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap();
        assert_eq!(resolved.spec.name, "dict");
        let sub = resolved.sub.expect("dict create resolves to a subcommand");
        assert_eq!(sub.name, "create");
    }

    #[test]
    fn resolve_call_dialect_filter_blocks_tcl84_dict() {
        let reg = CommandRegistry::build_default();
        // dict is tcl8.5+; resolving against tcl8.4 must fail.
        assert!(
            reg.resolve_call(
                "dict",
                &["create"],
                Some(SurfaceQuery::core(Family::Tcl, "8.4"))
            )
            .is_none()
        );
    }

    #[test]
    fn resolve_call_picks_arity_matched_command_form_for_incr() {
        let reg = CommandRegistry::build_default();
        // `incr counter` — arity 1 → matches the implicit form.
        let r1 = reg.resolve_call("incr", &["counter"], None).unwrap();
        let f1 = r1.form.expect("incr should match a CommandForm");
        assert_eq!(f1.name, "implicit");
        assert_eq!(f1.arity, crate::arity::Arity::exact(1));

        // `incr counter 5` — arity 2 → matches the explicit form.
        let r2 = reg.resolve_call("incr", &["counter", "5"], None).unwrap();
        let f2 = r2.form.expect("incr counter 5 should match a CommandForm");
        assert_eq!(f2.name, "explicit");
        assert_eq!(f2.arity, crate::arity::Arity::exact(2));
    }

    #[test]
    fn resolve_call_picks_arity_matched_command_form_for_lset() {
        let reg = CommandRegistry::build_default();
        // `lset lst value` — arity 2 → replace form.
        let replace = reg
            .resolve_call(
                "lset",
                &["lst", "value"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap();
        assert_eq!(replace.form.unwrap().name, "replace");

        // `lset lst 0 value` — arity 3 → single_index form.
        let single = reg
            .resolve_call(
                "lset",
                &["lst", "0", "value"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap();
        assert_eq!(single.form.unwrap().name, "single_index");

        // `lset lst 0 1 2 value` — arity 5 → flat_path form.
        let flat = reg
            .resolve_call(
                "lset",
                &["lst", "0", "1", "2", "value"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap();
        assert_eq!(flat.form.unwrap().name, "flat_path");
    }

    // -- ``resolve_option_terminator`` (W304 driver)
    //
    // Exercises option-terminator resolution for each command.
    // Each W304 fixture
    // is rooted in one of these resolver outcomes; the resolver tests
    // here pin the per-command shape, the analyser tests pin the
    // tristate-severity / two-diagnostic / code-fix behaviour.

    #[test]
    fn resolve_option_terminator_returns_none_for_unknown_command() {
        let reg = CommandRegistry::build_default();
        assert!(
            reg.resolve_option_terminator("unknownthing", &[], None)
                .is_none()
        );
    }

    #[test]
    fn resolve_option_terminator_returns_none_for_command_without_terminator() {
        let reg = CommandRegistry::build_default();
        // ``set`` does not declare a ``--`` terminator option.
        assert!(
            reg.resolve_option_terminator("set", &["x", "1"], None)
                .is_none()
        );
    }

    #[test]
    fn resolve_option_terminator_form_level_for_regexp() {
        let reg = CommandRegistry::build_default();
        let profile = reg
            .resolve_option_terminator("regexp", &[], None)
            .expect("regexp declares -- at the form level");
        assert_eq!(profile.scan_start, 0);
        assert!(profile.subcommand.is_none());
        // ``-start`` takes a value; ``-nocase`` does not.
        // ``-start`` takes a value; ``-nocase`` does not.  The
        // resolver returns the borrowed options slice; callers
        // filter via ``OptionSpec::takes_value``.
        assert!(
            profile
                .options
                .iter()
                .any(|o| o.name == "-start" && o.takes_value())
        );
        assert!(
            profile
                .options
                .iter()
                .any(|o| o.name == "-nocase" && !o.takes_value())
        );
    }

    #[test]
    fn resolve_option_terminator_subcommand_scoped_for_file_delete() {
        let reg = CommandRegistry::build_default();
        let profile = reg
            .resolve_option_terminator("file", &["delete", "$path"], None)
            .expect("file delete declares -- at the subcommand level");
        assert_eq!(profile.scan_start, 1);
        assert_eq!(profile.subcommand, Some("delete"));
    }

    #[test]
    fn resolve_option_terminator_subcommand_without_terminator_returns_none() {
        let reg = CommandRegistry::build_default();
        // ``file mtime`` has no ``--`` terminator.
        let profile = reg.resolve_option_terminator("file", &["mtime", "$p"], None);
        assert!(profile.is_none(), "got {profile:?}");
    }

    // -- ``is_canonical_list_command`` (W101 safe-idiom driver)

    #[test]
    fn is_canonical_list_command_includes_list_and_split_excludes_concat() {
        let reg = CommandRegistry::build_default();
        assert!(reg.is_canonical_list_command("list"));
        assert!(reg.is_canonical_list_command("linsert"));
        assert!(reg.is_canonical_list_command("split"));
        assert!(reg.is_canonical_list_command("lreverse"));
        // ``concat`` returns LIST but is the explicit non-canonical
        // exclusion.
        assert!(!reg.is_canonical_list_command("concat"));
        // Non-list commands (e.g. ``set``) are filtered out.
        assert!(!reg.is_canonical_list_command("set"));
        // Unknown commands return false.
        assert!(!reg.is_canonical_list_command("unknownthing"));
    }

    #[test]
    fn is_canonical_list_command_handles_compound_subcommand_keys() {
        let reg = CommandRegistry::build_default();
        // ``dict keys`` returns LIST.
        assert!(reg.is_canonical_list_command("dict keys"));
        // ``dict get`` returns String (or unspecified) — not canonical.
        assert!(!reg.is_canonical_list_command("dict froob"));
    }

    #[test]
    fn irules_sink_commands_carry_structural_options() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();

        let respond = reg.get("HTTP::respond").expect("HTTP::respond loaded");
        let opts: Vec<&str> = respond.options.iter().map(|o| o.name).collect();
        // Option set follows the reference standard:
        // -version/-content/-ifile/
        // -noserver/-reset. (`-status` is the positional status arg.)
        assert!(
            opts.contains(&"-version")
                && opts.contains(&"-content")
                && opts.contains(&"-noserver")
                && opts.contains(&"-reset"),
            "HTTP::respond options {opts:?} should include -version / -content / -noserver / -reset",
        );
        let noserver = respond
            .options
            .iter()
            .find(|o| o.name == "-noserver")
            .unwrap();
        assert!(!noserver.takes_value());
        let version = respond
            .options
            .iter()
            .find(|o| o.name == "-version")
            .unwrap();
        assert!(version.takes_value());

        let header = reg.get("HTTP::header").expect("HTTP::header loaded");
        let header_opts: Vec<&str> = header.options.iter().map(|o| o.name).collect();
        assert!(
            header_opts.contains(&"-noupdate"),
            "HTTP::header options {header_opts:?} should include -noupdate",
        );
    }

    #[test]
    fn xc_translatability_helpers_read_spec_flags() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();

        // `xc_translatable: Some(false)` → never translatable (consumed by the
        // `f5-xc` translator's XC300 branch).
        assert!(reg.is_xc_never_translatable("eval"));
        assert!(!reg.is_xc_translatable_override("eval"));

        // `xc_translatable: Some(true)` → translatable override despite an
        // otherwise-untranslatable namespace prefix (e.g. `IP::`, `ASM::`).
        assert!(reg.is_xc_translatable_override("IP::client_addr"));
        assert!(!reg.is_xc_never_translatable("IP::client_addr"));

        // Commands with no `xc_translatable` flag report neither.
        assert!(!reg.is_xc_never_translatable("set"));
        assert!(!reg.is_xc_translatable_override("set"));
        // An unknown command name is safely neither.
        assert!(!reg.is_xc_never_translatable("no_such_command_xyz"));
        assert!(!reg.is_xc_translatable_override("no_such_command_xyz"));
    }

    /// `unit_linkage` composes `spec.traits | sub.traits` and filters to the
    /// linkage union, so the answer is subcommand-precise: `package provide`
    /// publishes an API surface, `package require` pulls another unit in, and
    /// `package names` does neither (issue #977).
    #[test]
    fn unit_linkage_is_subcommand_precise() {
        let reg = CommandRegistry::build_default();
        let empty = None;
        assert_eq!(
            reg.unit_linkage("package", &["provide", "mylib", "1.0"], empty),
            Traits::PROVIDES_PACKAGE
        );
        assert_eq!(
            reg.unit_linkage("package", &["ifneeded", "mylib", "1.0", "body"], empty),
            Traits::PROVIDES_PACKAGE
        );
        assert_eq!(
            reg.unit_linkage("package", &["require", "mylib"], empty),
            Traits::LOADS_EXTERNAL_UNIT
        );
        assert_eq!(
            reg.unit_linkage("package", &["names"], empty),
            Traits::empty()
        );
    }

    /// `invocation_traits` composes `spec.traits | sub.traits` for the
    /// concrete call, which is the only way the eval-family bits on the
    /// compound members are visible: `namespace eval` / `namespace inscope` /
    /// `interp eval` carry `EVALUATES_CODE` on the **subcommand**, so a
    /// parent-only `get(name).traits` test misses them (issue #1055).
    #[test]
    fn invocation_traits_compose_subcommand_traits() {
        let reg = CommandRegistry::build_default();
        let empty = None;
        // TP — the eval family, bare and compound.
        for (name, args) in [
            ("eval", &["{set x 1}"][..]),
            ("uplevel", &["1", "{set x 1}"][..]),
            ("namespace", &["eval", "ns", "{set x 1}"][..]),
            ("namespace", &["inscope", "ns", "{set x 1}"][..]),
            ("interp", &["eval", "slave", "{set x 1}"][..]),
        ] {
            let traits = reg.invocation_traits(name, args, empty);
            assert!(
                traits.contains(Traits::EVALUATES_CODE),
                "{name} {args:?} must carry EVALUATES_CODE"
            );
            assert!(
                traits.contains(Traits::SCRIPT_CONCATENATES_ARGS),
                "{name} {args:?} must carry SCRIPT_CONCATENATES_ARGS"
            );
        }
        // TN — the parent spec alone carries neither bit, which is exactly
        // why composing is required.
        let parent = reg.get("namespace").expect("namespace spec").traits;
        assert!(!parent.contains(Traits::EVALUATES_CODE));
        // TN — a sibling subcommand that evaluates nothing stays clean.
        assert!(
            !reg.invocation_traits("namespace", &["delete", "ns"], empty)
                .contains(Traits::EVALUATES_CODE)
        );
        // TN — an unknown command carries no traits at all.
        assert_eq!(
            reg.invocation_traits("no_such_command_xyz", &["a"], empty),
            Traits::empty()
        );
        // The parent's own traits still compose in — a subcommand's traits
        // are additive, not a replacement.
        assert!(
            reg.invocation_traits("namespace", &["eval", "ns", "{}"], empty)
                .contains(Traits::LANGUAGE_KEYWORD),
            "the parent `namespace` spec's own bits survive composition"
        );
    }

    #[test]
    fn typed_control_arms_are_registry_owned() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.control_arm_semantics("try", &["{}", "finally", "{}"], 2),
            Some(ControlArmSemantics::Always)
        );
        assert_eq!(
            reg.control_arm_semantics("try", &["{}", "on", "error", "{m o}", "{}"], 4,),
            Some(ControlArmSemantics::Selected)
        );
        assert_eq!(
            reg.control_arm_semantics("namespace", &["eval", "::ns", "{}"], 2),
            Some(ControlArmSemantics::FrameBoundary)
        );
        assert_eq!(
            reg.control_arm_semantics("try", &["{}", "finally", "{}", "orphan"], 0),
            None,
            "a malformed trailing clause invalidates even the main arm"
        );
        for args in [
            &["{}", "on", "bogus", "{}", "{}"][..],
            &["{}", "on", "ok", "a b c", "{}"][..],
            &["{}", "on", "ok", "{}", "-"][..],
        ] {
            assert_eq!(reg.control_arm_semantics("try", args, 0), None);
        }
        assert_eq!(
            reg.control_invocation_valid("try", &["{}", "on", "ok", "{}", "-"], None,),
            Some(false),
            "a terminal handler fallthrough has no following script"
        );
        assert_eq!(
            reg.control_arm_semantics("try", &["{}", "finally", "-"], 2),
            Some(ControlArmSemantics::Always),
            "a finally body is a script, so `-` has no fallthrough meaning there"
        );
        let chained_fallthrough = [
            "{}",
            "on",
            "error",
            "{}",
            "-",
            "on",
            "error",
            "{}",
            "{set r fell}",
        ];
        assert_eq!(
            reg.control_invocation_valid("try", &chained_fallthrough, None),
            Some(true)
        );
        assert_eq!(
            reg.control_arm_semantics("try", &chained_fallthrough, 8),
            Some(ControlArmSemantics::Selected)
        );
        assert_eq!(
            reg.control_invocation_valid("if", &["1", "then"], None),
            Some(false)
        );
        assert_eq!(
            reg.control_invocation_valid("if", &["1", "a", "b"], None),
            Some(true)
        );
        assert_eq!(
            reg.control_arm_semantics("if", &["1", "a", "b"], 2),
            Some(ControlArmSemantics::Selected)
        );
    }

    #[test]
    fn invocation_completion_is_registry_owned() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.invocation_completion("return", &["-code", "error", "$w"], None,),
            InvocationCompletion::Terminates
        );
        assert_eq!(
            reg.invocation_completion("return", &["$w"], None),
            InvocationCompletion::ReturnsResult(Some(0))
        );
        assert_eq!(
            reg.invocation_completion("return", &["-code"], None),
            InvocationCompletion::ReturnsResult(Some(0)),
            "a trailing option-shaped word is return's result, not a missing option value"
        );
        assert_eq!(
            reg.invocation_completion("return", &["-level", "0", "$w"], None,),
            InvocationCompletion::FallsThrough
        );
        for args in [
            &["-level", "0", "-code", "error", "$w"][..],
            &["-code", "error", "-level", "0", "$w"][..],
        ] {
            assert_eq!(
                reg.invocation_completion("return", args, None),
                InvocationCompletion::Terminates
            );
        }
        for args in [
            &["-level", "$dynamic", "$w"][..],
            &["-level", "-1", "$w"][..],
            &["$w", "extra"][..],
        ] {
            assert_eq!(
                reg.invocation_completion("return", args, None),
                InvocationCompletion::Unknown
            );
        }
        assert_eq!(
            reg.invocation_completion("not-a-command", &[], None),
            InvocationCompletion::Unknown
        );
        assert_eq!(
            reg.invocation_completion("set", &[], None),
            InvocationCompletion::Unknown
        );
    }

    #[test]
    fn exact_return_options_and_process_exit_are_registry_owned() {
        use crate::completion::CompletionCode;

        let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
        // Oracle (tclsh 8.6/9.0): the default -level 1 return is caught by
        // `try on return`, even when its eventual procedure result is error.
        assert_eq!(
            reg.exact_invocation_completion("return", &["-code", "error", "payload"], None,),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return))
        );
        assert_eq!(
            reg.exact_invocation_completion(
                "return",
                &["-level", "0", "-code", "error", "payload"],
                None,
            ),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Error))
        );
        for args in [
            &["-foo", "bar", "payload"][..],
            &["-c", "error", "payload"][..],
            &["-level", "0x0", "-code", "error", "payload"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("return", args, None),
                Some(ExactInvocationCompletion::Tcl(if args[0] == "-level" {
                    CompletionCode::Error
                } else {
                    CompletionCode::Return
                }))
            );
        }
        for (spelling, expected) in [("2147483648", i32::MIN), ("4294967295", -1)] {
            assert_eq!(
                reg.exact_invocation_completion(
                    "return",
                    &["-level", "0", "-code", spelling, "payload"],
                    None,
                ),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Other(
                    expected
                )))
            );
        }
        for (spelling, expected) in [
            ("00", CompletionCode::Ok),
            ("+1", CompletionCode::Error),
            ("02", CompletionCode::Return),
            ("0x3", CompletionCode::Break),
            ("04", CompletionCode::Continue),
        ] {
            assert_eq!(
                reg.exact_invocation_completion(
                    "return",
                    &["-level", "0", "-code", spelling, "payload"],
                    None,
                ),
                Some(ExactInvocationCompletion::Tcl(expected)),
                "{spelling}"
            );
        }
        assert_eq!(
            reg.exact_invocation_completion("exit", &["0"], None),
            Some(ExactInvocationCompletion::ProcessExit)
        );
        assert_eq!(
            reg.exact_invocation_completion("return", &["-options", "$dynamic", "payload"], None,),
            None
        );
        assert_eq!(
            reg.exact_invocation_completion("return", &["-code", "$dynamic", "payload"], None,),
            None
        );
        assert_eq!(
            reg.exact_invocation_completion("return", &["--", "-code", "error"], None,),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return))
        );
    }

    #[test]
    fn exit_status_completion_follows_the_release_specific_tcl_oracle() {
        use crate::completion::CompletionCode;
        use crate::invocation_words::{InvocationArguments, InvocationWord};

        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            // Tcl 8.x's `exit` calls `Tcl_GetIntFromObj`. Despite the `int`
            // destination, its 64-bit implementation accepts the asymmetric
            // `-UINT_MAX..=UINT_MAX` range before casting to `int`.
            for args in [
                &[][..],
                &["0"][..],
                &["-7"][..],
                &["0x10"][..],
                &["-4294967295"][..],
                &["4294967295"][..],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("exit", args, None),
                    Some(ExactInvocationCompletion::ProcessExit),
                    "{dialect}: exit {args:?}"
                );
            }
            // One value beyond either Tcl 8.x conversion endpoint never gets
            // as far as process termination: it is a catchable Tcl error.
            for args in [
                &["nope"][..],
                &["1.5"][..],
                &["-4294967296"][..],
                &["4294967296"][..],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("exit", args, None),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: exit {args:?}"
                );
            }
        }

        for dialect in ["tcl9.0", "tcl9.1"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            // Tcl 9.0+ changed `exit` to `TclGetWideBitsFromObj`: every
            // integer, including a bignum beyond wide range, reaches
            // `Tcl_Exit` and has its low bits cast to `int` there.
            for args in [
                &[][..],
                &["0"][..],
                &["-4294967296"][..],
                &["4294967296"][..],
                &["18446744073709551616"][..],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("exit", args, None),
                    Some(ExactInvocationCompletion::ProcessExit),
                    "{dialect}: exit {args:?}"
                );
            }
            for args in [&["nope"][..], &["1.5"][..], &["0", "extra"][..]] {
                assert_eq!(
                    reg.exact_invocation_completion("exit", args, None),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: exit {args:?}"
                );
            }
        }

        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            let dynamic = [InvocationWord::Dynamic];
            assert_eq!(
                reg.invocation_completion_knowledge(
                    "exit",
                    InvocationArguments::structured(&dynamic),
                    None,
                ),
                Some(InvocationCompletionKnowledge::ExitOrError),
                "{dialect}: exit $status is only process-exit or Tcl error"
            );
            let expanded = [InvocationWord::Expanded];
            assert_eq!(
                reg.invocation_completion_knowledge(
                    "exit",
                    InvocationArguments::structured(&expanded),
                    None,
                ),
                Some(InvocationCompletionKnowledge::ExitOrError),
                "{dialect}: exit {{*}}$statuses has no normal completion either"
            );
        }
    }

    #[test]
    fn exact_return_option_shapes_follow_the_tcl86_and_tcl90_oracle() {
        use crate::completion::CompletionCode;

        // Tcl parses an option only when a following argv word can supply its
        // value. A lone option-shaped word is therefore return's sole result;
        // with `-level 0`, it exposes the default TCL_OK instead.
        for dialect in ["tcl8.6", "tcl9.0"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            for (args, expected) in [
                (&["-code"][..], CompletionCode::Return),
                (&["-level"][..], CompletionCode::Return),
                (&["-options"][..], CompletionCode::Return),
                (&["-foo"][..], CompletionCode::Return),
                (&["-level", "0", "-code"][..], CompletionCode::Ok),
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("return", args, None),
                    Some(ExactInvocationCompletion::Tcl(expected)),
                    "{dialect}: {args:?}"
                );
            }
            // Once a following word exists, the same spellings are options;
            // malformed option values and an extra result remain TCL_ERROR.
            for args in [
                &["-code", "bogus", "payload"][..],
                &["-level", "-1", "payload"][..],
                &["-code", "-level"][..],
                &["-level", "-code"][..],
                &["payload", "extra"][..],
                &["-level", "0", "-code", "-2147483649", "payload"][..],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("return", args, None),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: {args:?}"
                );
            }
        }
    }

    #[test]
    fn exact_invalid_arity_of_known_terminators_is_tcl_error() {
        use crate::completion::CompletionCode;

        // The descriptor of a well-formed call must never hide Tcl's prior
        // arity validation.  This table covers every fixed-arity terminating
        // core primitive, including ones whose normal effect is process exit
        // or a loop transfer rather than TCL_ERROR.
        for dialect in ["tcl8.6", "tcl9.0"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            for (name, args) in [
                ("error", &[][..]),
                ("error", &["message", "info", "code", "extra"][..]),
                ("throw", &[][..]),
                ("throw", &["type"][..]),
                ("throw", &["type", "message", "extra"][..]),
                ("break", &["extra"][..]),
                ("continue", &["extra"][..]),
                ("exit", &["0", "extra"][..]),
            ] {
                assert_eq!(
                    reg.exact_invocation_completion(name, args, None),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: {name} {args:?}"
                );
            }
        }
    }

    #[test]
    fn exact_return_source_words_distinguish_literals_from_substitution() {
        use crate::completion::CompletionCode;
        use crate::invocation_words::{InvocationArguments, InvocationWord};

        // Tcl 8.6 and 9.0 oracle: braces keep `$code` and `\\x32` literal,
        // while a bare/quoted substitution remains unknown and a decoded bare
        // escape can be supplied to this API as its effective static value.
        for dialect in ["tcl8.6", "tcl9.0"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            let trailing_option = [InvocationWord::Literal("-code")];
            assert_eq!(
                reg.exact_invocation_completion_words(
                    "return",
                    InvocationArguments::structured(&trailing_option),
                    None
                ),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
                "{dialect}: source-aware lone -code remains the result word"
            );
            for words in [
                vec![
                    InvocationWord::Literal("-code"),
                    InvocationWord::Literal("$code"),
                ],
                vec![
                    InvocationWord::Literal("-code"),
                    InvocationWord::Literal("\\x32"),
                ],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion_words(
                        "return",
                        InvocationArguments::structured(&words),
                        None
                    ),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: {words:?}"
                );
            }
            let decoded = [
                InvocationWord::Literal("-code"),
                InvocationWord::Literal("2"),
            ];
            assert_eq!(
                reg.exact_invocation_completion_words(
                    "return",
                    InvocationArguments::structured(&decoded),
                    None
                ),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
            );
            for dynamic in [InvocationWord::Dynamic, InvocationWord::Dynamic] {
                let words = [InvocationWord::Literal("-code"), dynamic];
                assert_eq!(
                    reg.exact_invocation_completion_words(
                        "return",
                        InvocationArguments::structured(&words),
                        None
                    ),
                    None,
                );
            }
            let dynamic_option = [InvocationWord::Dynamic, InvocationWord::Literal("error")];
            assert_eq!(
                reg.invocation_completion_knowledge(
                    "return",
                    InvocationArguments::structured(&dynamic_option),
                    None,
                ),
                Some(InvocationCompletionKnowledge::Dynamic),
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)] // descriptor layout matrix
    fn case_list_invocation_layout_is_registry_owned() {
        use crate::spec::CaseMatchMode;
        let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
        let Some((_, two_arg)) = reg.case_invocation(
            "switch",
            &["-glob", "default {}"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        ) else {
            panic!("two-argument case form must parse");
        };
        assert_eq!(two_arg.subject_index, Some(0));
        assert_eq!(two_arg.mode, CaseMatchMode::Exact);
        assert_eq!(two_arg.clause_list_index, Some(1));

        let Some((_, options)) = reg.case_invocation(
            "switch",
            &["-glob", "-nocase", "--", "subject", "p {}"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        ) else {
            panic!("option-bearing case form must parse");
        };
        assert_eq!(options.subject_index, Some(3));
        assert_eq!(options.mode, CaseMatchMode::Glob);
        assert!(options.nocase);

        let tcl91 = crate::model::ingress::static_context_for("tcl9.1").commands();
        let Some((_, integer)) = tcl91.case_invocation(
            "switch",
            &["-integer", "--", "1", "1 {set x 1}"],
            Some(SurfaceQuery::core(Family::Tcl, "9.1")),
        ) else {
            panic!("Tcl 9.1 integer switch must retain its case-list body");
        };
        assert_eq!(integer.mode, CaseMatchMode::Other);
        assert_eq!(integer.clause_list_index, Some(3));
        for args in [
            ["-integer", "-nocase", "1", "1 {set x 1}"],
            ["-nocase", "-integer", "1", "1 {set x 1}"],
        ] {
            assert!(
                tcl91
                    .case_invocation(
                        "switch",
                        &args,
                        Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                    )
                    .is_none(),
                "integer and nocase must be incompatible: {args:?}"
            );
            assert!(
                tcl91
                    .arg_indices_for_role("switch", &args, ArgRole::Body)
                    .is_empty(),
                "invalid integer/nocase form must expose no body: {args:?}"
            );
        }
        for args in [
            ["-integer", "-glob", "1", "1 {set x 1}"],
            ["-glob", "-integer", "1", "1 {set x 1}"],
            ["-integer", "-integer", "1", "1 {set x 1}"],
        ] {
            assert!(
                tcl91
                    .case_invocation(
                        "switch",
                        &args,
                        Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                    )
                    .is_none(),
                "multiple match modes must be rejected: {args:?}"
            );
            assert!(
                tcl91
                    .arg_indices_for_role("switch", &args, ArgRole::Body)
                    .is_empty(),
                "multiple match modes must expose no body: {args:?}"
            );
        }
        assert_eq!(
            tcl91.arg_indices_for_role(
                "switch",
                &["-integer", "--", "1", "1 {set x 1}"],
                ArgRole::Body,
            ),
            vec![3]
        );
        assert!(
            reg.case_invocation(
                "switch",
                &["subject", "pattern", "body", "orphan"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .is_none()
        );
        assert!(
            reg.case_invocation(
                "switch",
                &["subject", "pattern {} orphan"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0"))
            )
            .is_none()
        );

        let tcl84 = crate::model::ingress::static_context_for("tcl8.4").commands();
        for two_arg_switch in [
            ["-regexp", "default {puts hit}"],
            ["--", "default {puts hit}"],
        ] {
            assert!(
                tcl84
                    .case_invocation(
                        "switch",
                        &two_arg_switch,
                        Some(SurfaceQuery::core(Family::Tcl, "8.4"))
                    )
                    .is_none(),
                "Tcl 8.4 scans the option-like first word and rejects the missing subject: {two_arg_switch:?}"
            );
            assert!(
                tcl84
                    .arg_indices_for_role("switch", &two_arg_switch, ArgRole::Body)
                    .is_empty(),
                "Tcl 8.4 must not assign the second word a body role: {two_arg_switch:?}"
            );
            assert_eq!(
                tcl84
                    .resolve_option_terminator(
                        "switch",
                        &two_arg_switch,
                        Some(SurfaceQuery::core(Family::Tcl, "8.4"))
                    )
                    .expect("switch declares --")
                    .reserved_trailing_words,
                0,
                "Tcl 8.4 must scan both words in the option-like two-word form: {two_arg_switch:?}"
            );
        }
        for args in [
            &["subject", "default {puts hit}"][..],
            &["-regexp", "subject", "default {puts hit}"][..],
            &["--", "-subject", "default {puts hit}"][..],
        ] {
            assert!(
                tcl84
                    .case_invocation("switch", args, Some(SurfaceQuery::core(Family::Tcl, "8.4")))
                    .is_some(),
                "Tcl 8.4 accepts the unambiguous switch case-list form: {args:?}"
            );
            assert_eq!(
                tcl84.arg_indices_for_role("switch", args, ArgRole::Body),
                vec![args.len() - 1],
                "Tcl 8.4 assigns the final case-list word a body role: {args:?}"
            );
        }
        assert!(
            tcl84
                .case_invocation(
                    "switch",
                    &["-nocase", "subject", "pattern {}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.4")),
                )
                .is_none()
        );
        let tcl85 = crate::model::ingress::static_context_for("tcl8.5").commands();
        for (dialect, availability) in [
            ("tcl8.5", Some(SurfaceQuery::core(Family::Tcl, "8.5"))),
            ("tcl8.6", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
            ("tcl9.0", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
        ] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            for two_arg_switch in [
                ["-regexp", "default {puts hit}"],
                ["--", "default {puts hit}"],
            ] {
                assert!(
                    registry
                        .case_invocation("switch", &two_arg_switch, availability)
                        .is_some(),
                    "{dialect} accepts the optionless two-word switch form: {two_arg_switch:?}"
                );
                assert_eq!(
                    registry.arg_indices_for_role("switch", &two_arg_switch, ArgRole::Body),
                    vec![1],
                    "{dialect} assigns the clause-list word a body role: {two_arg_switch:?}"
                );
                assert_eq!(
                    registry
                        .resolve_option_terminator("switch", &two_arg_switch, availability)
                        .expect("switch declares --")
                        .reserved_trailing_words,
                    2,
                    "{dialect} reserves the subject and clause-list words: {two_arg_switch:?}"
                );
            }
        }
        assert!(
            tcl85
                .case_invocation(
                    "switch",
                    &["-nocase", "subject", "pattern {}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.5")),
                )
                .is_some()
        );
        let default = CommandRegistry::build_default();
        assert!(
            default
                .case_invocation(
                    "switch",
                    &["subject", "pattern {}"],
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                )
                .is_some()
        );
        assert!(
            default
                .case_invocation(
                    "switch",
                    &["--", "-x", "-x {puts hit} default {puts miss}"],
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                )
                .is_some(),
            "the descriptor's -- terminator keeps a hyphenated switch subject positional"
        );
        assert!(
            reg.case_invocation(
                "switch",
                &["subject", "pattern -"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0"))
            )
            .is_none()
        );

        let expect = crate::model::ingress::static_context_for("expect").commands();
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["\"password:\" {send pw} -re {ye+s} {send yes} timeout {puts slow}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
                )
                .is_some(),
            "clause-leading flags must not break valid Expect pattern/body pairs"
        );
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["{-re} {send literal}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
                )
                .is_some(),
            "a braced flag-shaped pattern is literal text, not a clause flag"
        );
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-re {ye+s}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                )
                .is_some(),
            "Expect permits a final pattern without an action"
        );
        for args in [
            ["-timeout", "5", "ready {action}"],
            ["-i", "spawn", "ready {action}"],
        ] {
            let (_, invocation) = expect
                .case_invocation(
                    "expect",
                    &args,
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
                )
                .expect("outer Expect value option followed by a final pattern");
            assert_eq!(invocation.clause_list_index, None, "{args:?}");
            assert_eq!(invocation.inline_clause_start, Some(2), "{args:?}");
            let clauses = crate::CaseListSpec::EXPECT
                .inline_clauses(&args, invocation.inline_clause_start.unwrap())
                .expect("one action-less inline clause");
            assert_eq!(clauses.len(), 1, "{args:?}");
            assert_eq!(clauses[0].pattern_index, 2, "{args:?}");
            assert_eq!(clauses[0].body_index, None, "{args:?}");
        }
        for pattern in ["#", ";"] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &[&format!(
                            "{pattern} {{send literal}} default {{send other}}"
                        )],
                        Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
                    )
                    .is_some(),
                "{pattern:?} is a literal Tcl list pattern, not script syntax"
            );
        }
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-timeout"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                )
                .is_none(),
            "a value-taking clause flag without a value/pattern/body is invalid"
        );
    }

    #[test]
    fn inline_expect_case_flags_and_actions_are_registry_owned() {
        let expect = crate::model::ingress::static_context_for("expect").commands();
        let args = [
            "-re",
            "{ye+s}",
            "{send yes}",
            "-timeout",
            "5",
            "timeout",
            "{send slow}",
        ];
        let (_, inline) = expect
            .case_invocation(
                "expect",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
            )
            .expect("inline Expect flags and value flags must parse");
        let clauses = crate::CaseListSpec::EXPECT
            .inline_clauses(&args, inline.inline_clause_start.expect("inline form"))
            .expect("inline clauses");
        assert_eq!(clauses.len(), 2);
        assert_eq!(clauses[0].pattern_index, 1);
        assert_eq!(clauses[0].body_index, Some(2));
        assert_eq!(clauses[0].mode, crate::spec::CaseMatchMode::Regexp);
        assert_eq!(clauses[1].pattern_index, 5);
        assert_eq!(clauses[1].body_index, Some(6));
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-not", "ready", "{send ok}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                )
                .is_some(),
            "unique Expect flag abbreviations must retain the action body"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Expect oracle layout matrix
    fn expect_inline_flag_table_matches_the_oracle() {
        let expect = crate::model::ingress::static_context_for("expect").commands();
        for flag in [
            "-glob",
            "-regexp",
            "-exact",
            "-notransfer",
            "-nocase",
            "-i",
            "-indices",
            "-iread",
            "-timestamp",
            "-nobrace",
        ] {
            let args = if matches!(flag, "-i") {
                vec![flag, "spawn", "pattern", "{action}"]
            } else {
                vec![flag, "pattern", "{action}"]
            };
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &args,
                        Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                    )
                    .is_some(),
                "canonical {flag} must parse"
            );
        }
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-timeout", "5", "pattern", "{action}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                )
                .is_some()
        );
        for flag in ["-gl", "-re", "-ex", "-not"] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &[flag, "pattern", "{action}"],
                        Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                    )
                    .is_some(),
                "unique abbreviation {flag} must parse"
            );
        }
        for flag in ["-g", "-r", "-e", "-noc", "-nob"] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &[flag, "pattern", "{action}"],
                        Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                    )
                    .is_some(),
                "unique canonical-prefix {flag} must remain an inline clause flag"
            );
        }
        for flag in ["-n", "-bogus"] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &[flag, "pattern", "{action}"],
                        Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                    )
                    .is_none(),
                "ambiguous or unknown {flag} must invalidate the invocation"
            );
        }
        let args = ["--", "-re", "{action}"];
        let (_, invocation) = expect
            .case_invocation(
                "expect",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
            )
            .expect("-- makes -re a pattern");
        let clauses = crate::CaseListSpec::EXPECT
            .inline_clauses(&args, invocation.inline_clause_start.expect("inline"))
            .expect("clause");
        assert_eq!(clauses[0].pattern_index, 1);
        assert_eq!(clauses[0].body_index, Some(2));
        let (_, omitted) = expect
            .case_invocation(
                "expect",
                &["-re", "pattern"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
            )
            .expect("omitted final action is valid");
        assert_eq!(
            crate::CaseListSpec::EXPECT
                .inline_clauses(&["-re", "pattern"], omitted.inline_clause_start.unwrap())
                .unwrap()[0]
                .body_index,
            None
        );
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-nobrace", "{pattern}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                )
                .is_some(),
            "-nobrace makes one braced word an action-less pattern"
        );
        let (_, brace) = expect
            .case_invocation(
                "expect",
                &["-brace", "{default {return FOLDED}}"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
            )
            .expect("exact -brace selects a clause list");
        assert_eq!(brace.clause_list_index, Some(1));
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-b", "{default {return FOLDED}}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
                )
                .is_none(),
            "-brace is exact-only, so -b is not a clause flag abbreviation"
        );
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-brac", "{default {return FOLDED}}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"])),
                )
                .is_none(),
            "-brace must not accept a near-complete prefix either"
        );
        for args in [
            &["-timeout", "5", "-brace", "{default {return FOLDED}}"] as &[&str],
            &["-i", "spawn", "-brace", "{default {return FOLDED}}"],
            &["-brace", "{default {return FOLDED}}", "extra"],
        ] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        args,
                        Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                    )
                    .is_none(),
                "force-list selector must be first and have one remainder: {args:?}"
            );
        }
    }

    #[test]
    fn subjectless_case_list_defers_clause_flags_after_outer_value_options() {
        use crate::hover::{OptionSpec, OptionValue};
        use crate::spec::CaseListSpec;

        // This deliberately does not name `expect`: a future descriptor with
        // one value-taking outer option and one overlapping clause flag must
        // use the same precedence rule. The body is the observable proof that
        // `-clause` was not swallowed by the outer scan.
        let case = CaseListSpec {
            subject_args: 0,
            two_arg_optionless_surface: None,
            regex_option: None,
            exact_option: None,
            glob_option: None,
            nocase_option: None,
            end_options_option: Some("--"),
            fallthrough_body: None,
            value_options_require_regex: &[],
            special_match_options: &[],
            clause_flags: &["-clause", "--"],
            clause_regex_flag: None,
            clause_value_flags: &[],
            clause_end_options_flag: Some("--"),
            clause_force_inline_flag: None,
            clause_force_list_flag: None,
            clause_force_list_shape: None,
            allow_omitted_final_body: true,
            keyword_patterns: &[],
            keyword_patterns_require_final: false,
            optional_subject_separator: None,
            warn_unbraced_bodies: false,
        };
        let options = [
            OptionSpec {
                name: "-outer",
                value: OptionValue::value("value"),
                ..OptionSpec::DEFAULT
            },
            OptionSpec {
                name: "-clause",
                ..OptionSpec::DEFAULT
            },
        ];
        let option_refs: Vec<&OptionSpec> = options.iter().collect();
        let args = ["-outer", "value", "-clause", "pattern", "{body}"];
        let invocation = case
            .invocation(
                &args,
                &option_refs,
                Some(SurfaceQuery::any_release(Family::Tcl)),
            )
            .expect("the outer value must precede, not consume, the clause flag");
        assert_eq!(invocation.inline_clause_start, Some(2));
        let clauses = case
            .inline_clauses(&args, invocation.inline_clause_start.unwrap())
            .expect("the deferred flag must retain the action body");
        assert_eq!(clauses[0].pattern_index, 3);
        assert_eq!(clauses[0].body_index, Some(4));
    }

    #[test]
    fn case_list_invocation_abstains_on_empty_and_truncated_shapes() {
        let switch = crate::model::ingress::static_context_for("tcl9.0").commands();
        for args in [
            &[][..],
            &["subject"][..],
            &["subject", ""][..],
            &["-regexp"][..],
            &["-regexp", "subject"][..],
            &["--"][..],
        ] {
            assert!(
                switch
                    .case_invocation("switch", args, Some(SurfaceQuery::core(Family::Tcl, "9.0")))
                    .is_none(),
                "truncated switch invocation must abstain: {args:?}",
            );
        }

        let expect = crate::model::ingress::static_context_for("expect").commands();
        for args in [
            &[][..],
            &["-brace"][..],
            &["-nobrace"][..],
            &["-timeout"][..],
            &["-timeout", "5"][..],
            &[""][..],
        ] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        args,
                        Some(SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&["expect"]))
                    )
                    .is_none(),
                "truncated Expect invocation must abstain: {args:?}",
            );
        }

        // Exercise the generic selector branch independently of either
        // built-in descriptor. The selector is intentionally outside the
        // clause-flag vocabulary, so a complete call consumes it as shape
        // syntax while a missing subject/body must simply abstain.
        let custom = crate::CaseListSpec {
            clause_force_inline_flag: Some("-inline"),
            allow_omitted_final_body: false,
            ..crate::CaseListSpec::EXPECT
        };
        for args in [
            &[][..],
            &["-inline"][..],
            &["-inline", "subject"][..],
            &[""][..],
        ] {
            assert!(
                custom
                    .invocation(args, &[], Some(SurfaceQuery::any_release(Family::Tcl)))
                    .is_none(),
                "truncated custom selector invocation must abstain: {args:?}",
            );
        }
        assert!(
            custom
                .invocation(
                    &["-inline", "pattern", "{body}"],
                    &[],
                    Some(SurfaceQuery::any_release(Family::Tcl)),
                )
                .is_some(),
            "complete custom inline-selector invocation remains valid",
        );

        // A dynamic two-word switch cannot expose a static body region, but
        // asking for roles must remain a conservative abstention rather than
        // a panic. The compiler's W304 regression separately proves Tcl's
        // optionless two-word rule is retained for diagnostics.
        assert!(
            switch
                .arg_indices_for_role("switch", &["$subject", "$cases"], ArgRole::Body)
                .is_empty(),
        );
    }

    /// The whole registry-declared boundary surface, resolved by name: every
    /// command that widens a file's caller set reports its kind, a
    /// `::`-qualified spelling resolves the same, and a command that does
    /// neither reports nothing.
    #[test]
    fn unit_linkage_covers_every_declared_boundary_command() {
        let reg = CommandRegistry::build_default();
        let empty = None;
        for (name, args, want) in [
            ("source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("::source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("load", &["libx.so"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("auto_load", &["helper"][..], Traits::LOADS_EXTERNAL_UNIT),
            (
                "auto_import",
                &["::lib::*"][..],
                Traits::LOADS_EXTERNAL_UNIT,
            ),
            // `namespace import` is deliberately not a boundary — see its
            // own spec comment.
            (
                "namespace",
                &["import", "::lib::helper"][..],
                Traits::empty(),
            ),
            (
                "namespace",
                &["export", "helper"][..],
                Traits::EXPORTS_COMMAND,
            ),
            (
                "namespace",
                &["ensemble", "create"][..],
                Traits::EXPORTS_COMMAND,
            ),
            ("namespace", &["eval", "::ns", "{}"][..], Traits::empty()),
            ("set", &["x", "1"][..], Traits::empty()),
            ("no_such_command_xyz", &["source"][..], Traits::empty()),
        ] {
            assert_eq!(
                reg.unit_linkage(name, args, empty),
                want,
                "unit_linkage({name}, {args:?})"
            );
        }
    }

    /// Widening `Traits` to `u128` gave `SAFE_INTERP_HIDDEN` a bit of its own
    /// (issue #1031): while it aliased `TRANSFERS_CONTROL` at bit 61, every
    /// `break`/`continue`/`tailcall`/`yield` read as safe-interp-hidden and
    /// every `cd`/`exec`/`glob`/… read as control-transferring, which
    /// `FRAME_SENSITIVE_TRAITS` consumes directly.
    #[test]
    fn safe_interp_hidden_no_longer_aliases_transfers_control() {
        let reg = CommandRegistry::build_default();
        assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);
        let cd = reg.get("cd").expect("cd is registered");
        assert!(cd.traits.contains(Traits::SAFE_INTERP_HIDDEN));
        assert!(!cd.traits.contains(Traits::TRANSFERS_CONTROL));
        assert!(!reg.is_frame_sensitive("cd"));
        let brk = reg.get("break").expect("break is registered");
        assert!(brk.traits.contains(Traits::TRANSFERS_CONTROL));
        assert!(!brk.traits.contains(Traits::SAFE_INTERP_HIDDEN));
    }

    /// Issue #1302: `declares_command_at` answers "is this in a fresh
    /// interpreter's command table", which is strictly narrower than `get`'s
    /// "is this a spelling a call site may write".
    ///
    /// Every row is oracle-verified on tclsh 9.0.4 and 8.6.14 via
    /// `info commands <name>` in a fresh `interp create`.
    #[test]
    fn declares_command_at_is_the_fresh_interpreter_command_table() {
        let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
        // `info commands ::set` -> ::set
        for name in ["set", "::set", "puts", "::puts", "if", "foreach"] {
            assert!(reg.declares_command_at(name), "{name} is a global builtin");
        }
        // `info commands ::tcl::mathop::+` -> ::tcl::mathop::+
        for name in [
            "tcl::mathop::+",
            "::tcl::mathop::+",
            "oo::define",
            "::oo::define",
        ] {
            assert!(
                reg.declares_command_at(name),
                "{name} is a namespaced builtin"
            );
        }
        // `info commands ::+` -> {} and `+ 1 2` -> invalid command name "+".
        // `get` still answers `Some` for these — that is the whole point.
        for name in ["+", "::+", "eq", "::eq", "in", "::in"] {
            assert!(
                reg.get(name).is_some(),
                "{name} must stay a resolvable *spelling*",
            );
            assert!(
                !reg.declares_command_at(name),
                "{name} is only callable after `namespace import ::tcl::mathop::*`",
            );
        }
        // A name nothing declares.
        for name in ["notACommand", "::notACommand", "define"] {
            assert!(!reg.declares_command_at(name), "{name}");
        }
    }

    /// The answer is per dialect, because the command table is: `HTTP::uri`
    /// exists for iRules and nowhere else.
    #[test]
    fn declares_command_at_is_dialect_specific() {
        assert!(
            crate::model::ingress::static_context_for("f5-irules")
                .commands()
                .declares_command_at("HTTP::uri"),
            "iRules declares HTTP::uri",
        );
        assert!(
            !crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .declares_command_at("HTTP::uri"),
            "plain Tcl does not",
        );
    }

    #[test]
    fn smoke_lookup_set_command() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get("set").expect("`set` must be a known command");
        assert!(
            spec.arity.min <= 1 && spec.arity.max >= 1,
            "set must accept at least a variable name: {:?}",
            spec.arity
        );
    }
}
