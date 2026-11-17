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

//! The introductions the Reference tab and enum / flag-set fields show per picker catalogue.

/// `(id, title, introduction)` per picker catalogue, rendered by the
/// Reference tab and by the **?** on enum / flag-set fields.
pub const CATALOGUE_HELP: &[(&str, &str, &str)] = &[
    (
        "argRole",
        "Argument roles",
        "What an argument position *is*. Roles are how the tools know \
`while`'s second word is a script, `set`'s first word names a variable, \
and `regexp`'s first word is a pattern — for your command exactly as for \
the built-ins. Body and Expr positions are analysed as code; VarWrite / \
VarRead positions join variable tracking (rename, unused, read-before-set); \
the rest refine highlighting, completion, and checks.",
    ),
    (
        "tclType",
        "Value types",
        "The internal representation a Tcl value carries alongside its \
string form — what a Tcl developer meets as shimmering. Used for return \
types and argument expectations. `Numeric` is \"Int or Double\"; \
`String` means a plain string with no cached structure.",
    ),
    (
        "bodyKind",
        "Body kinds",
        "Whether a script body runs in the caller's frame — seeing and \
changing the caller's variables, like `if` and `while` bodies — or in a \
separate definition context of its own, like a `proc` body. The first \
joins the surrounding data flow; the second deliberately does not.",
    ),
    (
        "irulesSourceFlow",
        "iRules source flow grammars",
        "Select the source operation grammar. Original operands and command bindings remain necessary; the declaration supplies no reached handler, body entry or successful effect.",
    ),
    (
        "scriptLookupScope",
        "Executable lookup frames",
        "Which frame supplies namespace lookup when an executable operand runs: current invoker, global interpreter or triggering operation. This is independent of timing and does not prove a future command table or callback reach.",
    ),
    (
        "scriptTiming",
        "Script timing",
        "When a script-valued option is evaluated relative to the command that \
receives it. This is independent of scope: timing answers *when*; Body kind \
answers *which frame*. Use `Deferred` for stored callbacks, `ReferenceOnly` \
for executable text identified but not invoked, and `SameInvocation` for \
scripts the command may run before returning.",
    ),
    (
        "variableScope",
        "Variable scopes",
        "Where a variable-name option resolves an unqualified name. Use \
`CurrentFrame` for normal Tcl call-frame lookup and `Global` only when the \
command's documentation defines an interpreter-global link.",
    ),
    (
        "argPresentation",
        "Argument presentation",
        "Formatter layout preferences for body arguments: expanded onto \
indented lines (the default), or kept inline on the command's own line \
the way `for`'s start and next scripts are.",
    ),
    (
        "storageType",
        "Storage types",
        "The container kind a written variable ends up holding — list, \
dict, or array — so later reads are understood and kind mix-ups \
flagged.",
    ),
    (
        "byteArrayEffect",
        "Byte-array effects",
        "What a command does to binary data: pass it through, silently \
coerce it to a string (Tcl's classic binary-corruption gotcha), \
case-fold it, re-encode it, or restore a byte-array representation. \
Drives the binary-data corruption check.",
    ),
    (
        "commandTableEffect",
        "Command-table effects",
        "Ways a command changes which commands exist: defining a procedure, \
renaming or deleting one, or creating an alias. Keeps \"unknown \
command\" honest after such calls.",
    ),
    (
        "channelConfiguration",
        "Channel configuration",
        "Conditional source option relationship vocabulary. Original values and current channel state remain independent.",
    ),
    (
        "sourceIndexBounds",
        "Source index bounds",
        "The source index relationship vocabulary, independently of native operation identity or runtime values.",
    ),
    (
        "sourcePathOperation",
        "Source path operations",
        "The shared source algebra for joining paths, taking a parent directory, lexically normalising an anchored path or using an explicitly supplied document filename. Runtime execution and filesystem state remain separate.",
    ),
    (
        "sourceConfigurationReference",
        "Source configuration references",
        "Registry-owned source operand grammars for configuration objects. DataGroupComparison describes the comparison tail and its data-group operand; it grants no native execution authority.",
    ),
    (
        "patternType",
        "Pattern types",
        "The two pattern languages a Pattern argument can speak: glob \
(`string match`) and regular expressions (`regexp`). A `*` means \
something different in each, so the right label matters for validation \
and highlighting.",
    ),
    (
        "taintTransformCondition",
        "Transform conditions",
        "The argument-shape proofs a command can require before its taint \
transform colour is claimed. A command whose sanitising effect comes from \
the literal it was given — `string map` with a mapping that deletes CR and \
LF — earns its colour call by call, not once for the command.",
    ),
    (
        "taintNumericCoercion",
        "Numeric-coercion shapes",
        "Which argument words a call reads as numbers, so a tainted one is a \
T100 numeric-coercion sink. Each shape names the option that turns the \
coercion on — Tcl 9.1's `switch -integer` reads its subject as a wide integer, plain \
`switch` compares text — so only the calls that coerce are flagged.",
    ),
    (
        "formatType",
        "Format-string types",
        "The template mini-languages a format argument can use: \
printf-style (`format` / `scan`), `clock format` fields, `binary` \
format/scan cursors, and `regsub` replacement templates. Validation \
follows the declared language.",
    ),
    (
        "formKind",
        "Form kinds",
        "Labels a documented invocation form as the default, a read-only \
getter, or a modifying setter. The label improves reference output; it does \
not select semantics. Use `command_forms` when forms need different arity, \
traits, purity, effects, or argument roles.",
    ),
    (
        "definedSymbolKind",
        "Defined-symbol kinds",
        "The outline categories a symbol-defining command can bind: test \
cases, test constraints, result matchers, and iRules event handlers. \
Symbols land in the document outline and workspace search.",
    ),
    (
        "sideEffectTarget",
        "Side-effect targets",
        "The kinds of state a structured side effect can read or write — \
from Tcl variables and channels through files, network, logs, and the \
whole F5 surface (HTTP state, tables, pools, SSL). Pick the closest \
target; `Unknown` exists for effects that fit nothing.",
    ),
    (
        "connectionSide",
        "Connection sides",
        "For iRules effects: which side of the proxied connection an effect \
touches — client, server, both, or connection-independent. Non-iRules \
commands use None.",
    ),
    (
        "loweringHook",
        "Lowering hooks",
        "Compiler internals: the named per-command translations into the \
compiler's intermediate form. Listed for completeness when browsing core \
specs — a third-party command leaves the field unset.",
    ),
    (
        "codegenHook",
        "Codegen hooks",
        "Compiler internals: the named per-command bytecode emitters, \
mirroring what C Tcl byte-compiles specially. Third-party commands leave \
this unset.",
    ),
    (
        "inlineCodegenHook",
        "Inline codegen hooks",
        "Compiler internals: bytecode emitters for value-position \
(`set x [cmd …]`) and catch-body uses. Third-party commands leave this \
unset.",
    ),
    (
        "analyserHook",
        "Analyser hooks",
        "Compiler internals: hand-written analyser families for commands \
whose behaviour the declarative fields cannot fully express (`proc`, \
`upvar`, `package require`, …). The declarative fields should always be \
tried first.",
    ),
    (
        "returnTypeHook",
        "Return-type hooks",
        "Compiler internals: the algorithm that types a call whose result \
*kind* moves with the call (`regexp -inline` returns the matched substrings \
where a bare `regexp` returns a match count). A hook rather than a \
declarative table because the switches interact — `lsearch -inline` beats \
`-subindices`. The static `return_type` should always be tried first.",
    ),
    (
        "traits",
        "Traits",
        "The registry's behavioural vocabulary — one flag per fact a \
consumer might need: evaluates its argument as code, alters control \
flow, creates an upvar-style alias, is a taint sink, is hidden in safe \
interpreters, and so on. Analyses read traits instead of matching \
command names, which is why setting the right traits on your command \
buys the same treatment the built-ins get. Search this list before \
assuming a behaviour cannot be expressed.",
    ),
    (
        "taintColour",
        "Taint colours",
        "How untrusted data is tracked. A value read from the network is \
marked `TAINTED`; every value derived from it inherits the mark. \
Sanitisers and validators add colours — `HTML_ESCAPED`, `CRLF_FREE`, \
`IP_ADDRESS` — recording what has been *proved* about the value. A sink \
(output, exec, SQL, log) then checks arriving values: raw taint is a \
finding, while taint carrying the colour that sink accepts passes. \
Encoders also declare the colour that means \"already encoded\", which \
is how double-encoding is caught.",
    ),
    (
        "dialects",
        "Dialects",
        "The Tcl worlds a spec can be scoped to: every core release the \
catalogue carries, alongside the tool dialects — the F5 surfaces, Tk, \
Expect, BPF, and the SpecTcl DSL itself. The list below is the whole \
vocabulary, labelled as the dialect catalogue labels it. A command's \
dialect set decides where it resolves; unset means everywhere.\n\nThe EDA \
shells are not on it: a vendor shell is a base Tcl release plus \
package-gated command libraries, so an EDA command is scoped by its \
`required_package`, not by a dialect of its own.",
    ),
    (
        "optionPlacement",
        "Option placement",
        "Where a command's declared options may be found in its invocation. \
`Leading` is the default and what almost every core Tcl command does: its C \
option loop `break`s on the first word that is not a declared option, so an \
option-shaped word after that is a positional argument. `Anywhere` is the \
script-level shape — a parser that takes its fixed arguments and then loops \
`foreach {flag value} $args`, recognising options between positionals up to an \
explicit `--`. The option-relation checker reads this to find the options it \
judges; the wrong answer either misses a relation or invents one.",
    ),
    (
        "defaultFormFirstWord",
        "Default-form first words",
        "The value shapes a non-subcommand first word may take to select a \
command's default form — `after 200 …`, where an integer first word \
means a delay rather than an unknown subcommand.",
    ),
    (
        "sourceDeprecationAdvice",
        "Source deprecation advice",
        "Typed source shape proposals owned by the selected Registry descriptor. Every edit needs original written word coordinates and review.",
    ),
    (
        "prefixMatching",
        "Prefix matching",
        "Whether a keyword table accepts any unique prefix (`string le` for \
`string length` — Tcl's normal behaviour) or only exact spellings \
(strict mode, matching `TCL_INDEX_STRICT`).",
    ),
    (
        "appendedArity",
        "Appended arity",
        "For callback command prefixes: how many arguments the command \
appends when it invokes the callback — exactly N, at least N, or \
unknown. The callback checker verifies the target procedure accepts \
them.",
    ),
    (
        "semanticOperation",
        "Semantic operation",
        "The target-neutral operation a command performs, as a closed \
vocabulary: `invoke` — the generic call every command falls back to — an \
intrinsic (`intrinsic list-length`, `intrinsic dict-get`, …), or a \
structured lowering (`structured-lowering expr`, …). The compiler \
backends share one implementation per operation across its spellings.",
    ),
    (
        "definitionBody",
        "Shipped definer grammar",
        "The definition-body grammars the registry ships and a pack may name: \
`TclOO`'s class body (and its configurable variant), snit's type and widget \
bodies, and [incr Tcl]'s class body. Each lists its member keywords, where \
each member's name, parameter list and body sit, and what each member \
declares.",
    ),
    (
        "optionArity",
        "Option arity",
        "How many value words an option consumes: one (`-index i`) or a \
fixed count. Options that take no value are declared by leaving \
takes-value off instead.",
    ),
];
