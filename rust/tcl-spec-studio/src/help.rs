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

//! Long-form help for the studio, written for Tcl developers.
//!
//! The one-line `doc` on each [`crate::schema::FieldSchema`] fits under a
//! label; this module carries the paragraph-or-three a **?** button opens —
//! what the field means in Tcl terms, with Tcl examples rather than Rust
//! ones. Three tables:
//!
//! - [`field_help`] — one entry per `CommandSpec` / `SubCommand` field key.
//!   Keys shared by both tables (most of them) share one entry; the index
//!   base ("counted after the subcommand word") is already stated by the
//!   field's one-line doc.
//! - [`group_help`] — one entry per form group heading, orienting the reader
//!   before the per-field text.
//! - [`catalogue_help`] — a title and an introduction per picker catalogue
//!   (traits, argument roles, taint colours, …), which is also what the
//!   Reference tab renders.
//!
//! The tests enforce full coverage in both directions: a field, group, or
//! catalogue added without help fails by name, and a help entry whose key no
//! longer exists fails too. "A **?** on everything" stays true by
//! construction.

/// Long-form help per field key, shared between the command and subcommand
/// tables. Paragraphs are separated by blank lines; the front-end renders
/// them as such.
const FIELD_HELP: &[(&str, &str)] = &[
    (
        "name",
        "The word a script calls the command by — exactly as it is typed in \
Tcl, including any namespace: `lappend`, `dict`, `HTTP::header`, \
`::mypkg::frobnicate`. For a namespaced command, use the fully qualified \
name without a leading `::`; the registry normalises a leading `::` at look-up \
time, so `::set` and `set` resolve to the same spec.\n\nOn a subcommand this \
is the subcommand word itself (`length` in `string length`).",
    ),
    (
        "traits",
        "Behavioural facts about the command, as a set of flags. Traits are \
how every analysis learns what a command *does* without anyone writing code: \
tick `EVALUATES_CODE` and body arguments are analysed as scripts; tick \
`CONTROL_FLOW` and unreachable-code checks understand it; tick `PURE` and the \
optimiser may fold it. If you only fill in one thing beyond arity, make it \
the traits — most diagnostics key off them.\n\nOpen the Reference tab for \
the full list with a one-line meaning each. When no trait fits, leave them \
all off — an empty set is always safe, it just tells the tools less.",
    ),
    (
        "surface",
        "Which Tcl worlds the command exists in — every dialect the picker \
lists, from the core releases through the F5, Tk, Expect, BPF and SpecTcl \
surfaces. A command only present from 8.5 onwards ticks 8.5 and every later \
release; one that is iRules-only ticks just F5 iRules. Leave the whole field \
unset for \"every dialect\".\n\nThis is what makes the same file lint \
differently as Tcl 8.4 versus 9.0: a command outside the active dialect is \
reported as unknown there.",
    ),
    (
        "arity",
        "How many arguments the command accepts, counted **after** the \
command name — the same rule behind Tcl's own `wrong # args` error. \
`set varName ?value?` is min 1, max 2. A trailing `args` parameter means \
no maximum, so leave max unbounded. `incr x ?increment?` is 1 to 2; \
`list` is 0 to unbounded.\n\nOn a subcommand, count after the subcommand \
word instead: `string length string` is exactly 1. The step and \
extra-exact-count fields cover commands whose argument tail comes in pairs \
(`array set` style `name value` lists).",
    ),
    (
        "arg_roles",
        "What each argument position *is*, for commands whose layout never \
changes. Index 0 is the first word after the command name. `proc name args \
body` declares 0 = Name, 1 = ParamList, 2 = Body; `foreach varlist list \
body` declares 0 = LoopVarList, 2 = Body.\n\nRoles are what make the editor \
light up a body argument as real code, treat `varName` arguments as \
variable reads and writes (so rename and \"unused variable\" work through \
them), and know a word is a pattern or a channel. See the Reference tab for \
every role. If the layout depends on the argument count or on option words, \
you need the argument-role resolver instead — say so in the issue you open, \
because resolvers are code.",
    ),
    (
        "arg_role_resolver",
        "A hook for commands whose argument layout cannot be a fixed table — \
`if` (any number of `elseif` clauses), `switch` (options before the value), \
`set` (arg 0 is written with two words but only read with one). The hook \
inspects the actual call and assigns roles.\n\nThis is Rust code, so the \
studio can only carry a reference to it. If your command needs one, describe \
the layout rule in plain words in the issue notes — for example \"the last \
argument is always the body; everything before it comes in `name value` \
pairs\" — and a maintainer writes the few lines.",
    ),
    (
        "arg_role_resolver_roles",
        "The complete set of roles the dynamic argument-role resolver can \
ever return. This is declarative even though the resolver itself is code. \
Consumers use it when substitutions or expansions hide the exact argument \
values, so omitting a possible role can suppress analysis while adding an \
impossible role makes analysis needlessly conservative.",
    ),
    (
        "arg_presentation",
        "How the formatter lays an argument out, when that differs from the \
default. Body arguments normally expand onto their own indented lines (the \
way everyone writes a `while` body); `InlineScript` keeps one on the command \
line instead — the way `for {set i 0} {$i < 3} {incr i} { … }` keeps its \
start and next scripts inline while only the body expands.\n\nOnly declare \
the exceptions; an empty map means \"format every body as a block\", which \
is right for almost every command.",
    ),
    (
        "repeated_args",
        "For argument tails that repeat a pattern without limit: `global a b \
c` (a variable name at every word), `variable n1 v1 n2 v2` (a name at every \
*other* word), `foreach v1 $l1 v2 $l2 {…}` (a pair pattern with the body \
excluded from the tail). Declared as a start index, a stride (1 = every \
word, 2 = every other), and how many trailing words to leave out.\n\nThis \
is what lets rename and highlighting reach *every* name in `global a b c`, \
not just the first.",
    ),
    (
        "frame_effect",
        "For commands that reach into another stack frame the way `upvar`, \
`uplevel`, and `namespace upvar` do: which word is the `?level?`, which \
arguments are variables in the *other* frame, and which scripts run in the \
caller's frame. This is a named descriptor a maintainer attaches; describe \
the frame behaviour in your issue notes if your command has one \
(most do not).",
    ),
    (
        "clause_shape_check",
        "A validator for commands whose legal shapes cannot be captured by a \
single min–max argument count — `if`'s `elseif`/`else` chain is the \
canonical case: any length is fine, but only in the right rhythm. This is \
code, so in the studio it is a reference; if your command has a clause \
grammar, write the rhythm out in the issue notes (\"`cond body` pairs, \
optionally ending `else body`\").",
    ),
    (
        "command_prefixes",
        "Argument positions that carry a *callback* — a command prefix the \
command will invoke later with extra arguments appended, like `lsort \
-command cmp` calling `cmp a b` or `trace add variable x write cb` calling \
`cb name1 name2 op`. Declare the position and how many arguments get \
appended, and the tools can check the callback procedure actually accepts \
them.\n\nA callback is a *reference* to code, not code itself — that is why \
it is not simply a Body role.",
    ),
    (
        "command_prefix_resolver",
        "The dynamic sibling of the command-prefix positions: a hook for when \
*which* word is the callback depends on the rest of the call (after options, \
say). Code, carried by reference — describe the rule in the issue notes.",
    ),
    (
        "script_timing_resolver",
        "The dynamic sibling of per-option `script_timing`: use it when the \
same executable position runs now in one invocation shape but is stored in \
another, as with `send -async`. It emits an exact index plus \
`SameInvocation`, `Deferred`, or `ReferenceOnly`; the index must already be a `Body`, \
`LambdaLiteral`, or `CommandPrefix`. Silence leaves the option timing or \
command-level compatibility fallback in force. In SpecTcl the body calls \
`timing IDX SameInvocation|Deferred|ReferenceOnly`.",
    ),
    (
        "callback_taint_inputs",
        "Lists only callback substitutions whose bytes are externally controlled. \
For Tk validation, `%P`, `%s`, and `%S` carry editable text; for key bindings, \
`%A` and `%K` carry the typed character or keysym. Do not declare widget paths, \
indices, validation actions, or reasons (`%W`, `%i`, `%d`, `%V`) here: those are \
framework metadata, not taint sources. The callback must be deferred; dynamic \
script construction remains intentionally unanalyzed. In SpecTcl, write an \
option's `-callback-taint-inputs {%P %S}` or the positional \
`callback_taint_inputs {{INDEX {%A %K}}}` table.",
    ),
    (
        "return_type",
        "The kind of value `[cmd …]` produces: string, integer, double, \
boolean, list, dict, byte array, object handle, or channel. `llength` \
returns Int; `lrange` returns List; `open` returns Channel. This feeds type \
inference — `set n [llength $l]` makes `$n` an integer downstream — and the \
warnings about treating a list as a string.\n\nLeave unset when the type \
varies and no single answer is honest.",
    ),
    (
        "var_write_typing",
        "What type the variables the command *writes* receive, when that is \
not the same as what the command *returns*. `lassign` returns the leftover \
list but writes list *elements*; `scan` and `regexp` return a count but \
write parsed pieces; `gets chan line` returns a character count but writes \
the line into `line`.\n\n`ReturnValue` (the default) says the written \
variable holds the return value. `Fixed` names one type for the written \
variable regardless of the return. `Destructured` says the pieces cannot be \
typed statically. Getting this right avoids false \"wrong type\" warnings \
on the written variables.",
    ),
    (
        "variable_write_min_args",
        "The minimum number of words after the command name at which any \
invocation can write a variable. This is a necessary-condition proof for \
variable-layout commands such as `regexp` and `regsub`, not the command's \
general arity. Leave it unset unless every shorter invocation is guaranteed \
not to have a variable-write target.",
    ),
    (
        "return_type_hook",
        "Names the algorithm that types a call whose result *kind* depends on \
how the command was called — `regexp` counts matches but `regexp -inline` \
returns the matched substrings instead, and `regsub` returns a replacement \
count until its `varName` is omitted and it returns the substituted string \
instead. The rule is a program rather than a table because the switches \
interact: `lsearch -inline` beats `-subindices`. An algorithm names a type \
only where the intrep is guaranteed and answers \"unknown\" otherwise, so \
some forms stay untyped even though their documented result is a list. Pick \
an existing hook only if it really describes this command; a new one needs \
an arm in `tcl_registry::return_type`. Leave it unset unless the result \
shape really \
moves — a wrong type here is worse than none.",
    ),
    (
        "return_elements",
        "How the result relates to its arguments *as a container* — for \
example, `list a b c` returns a list whose elements are exactly the \
arguments, and `lindex $l 0` returns an *element of* its list argument. \
Declaring it lets the analysis track values through list packing and \
unpacking. Expression-valued; when in doubt leave it unset and mention the \
relationship in the issue notes.",
    ),
    (
        "var_elements_effect",
        "How the command reshapes the container inside the variable it \
writes, in place: `lappend v x y` appends list elements, `dict set v k val` \
sets one dict value, `lpop v` removes one. This keeps element-level tracking \
alive across in-place edits instead of giving up on the whole \
variable. Expression-valued; describe the effect in words if unsure.",
    ),
    (
        "representation_effect",
        "Tcl values carry both a string form and a cached internal form \
(list, dict, integer …), and some commands convert or copy between them — \
the \"shimmering\" a Tcl developer knows from performance work. This field \
records such an effect (for example copy-on-write on a shared list) so the \
performance lints can see it. Rarely needed; safe to leave unset.",
    ),
    (
        "arg_types",
        "The value type each argument position expects — the input-side \
mirror of the return type. `lindex list index` expects a List at 0 and an \
index at 1; `incr` expects an integer variable. Drives \"this argument will \
shimmer\" and type-mismatch warnings.\n\nOnly declare positions where the \
expectation is firm; a generic \"any value\" position is better left out.",
    ),
    (
        "subcommands",
        "For ensemble commands — one word selects an operation, as in \
`string length`, `dict get`, `info exists`. Each subcommand is a small spec \
of its own: its arity, argument roles, options, return type, and \
documentation, all counted after the subcommand word.\n\nDeclare every \
subcommand the real command has, even thinly: an undeclared word gets \
flagged as an unknown subcommand (unless you allow unknowns below), and a \
declared one gets completion and its own arity check. Unique-prefix \
abbreviations (`string le` for `string length`) are handled for you — \
declare full names only.",
    ),
    (
        "allow_unknown_subcommands",
        "Turn this on when users can legitimately extend the command with \
their own subcommand words — `namespace ensemble` ensembles that scripts \
add to, or an object system whose methods are user-defined. It suppresses \
the \"unknown subcommand\" warning for words you did not declare, while \
keeping everything you *did* declare fully checked.",
    ),
    (
        "prefix_matching",
        "Real Tcl resolves any unique prefix of a keyword — `string le` is \
`string length`, `lsort -uni` is `-unique`. That is the default here too. \
Set Strict for commands that demand exact spellings (the C API's \
`TCL_INDEX_STRICT` mode), where an abbreviation is an error, not a \
shorthand.",
    ),
    (
        "default_form_first_word",
        "For commands where a first word that is *not* a subcommand selects a \
default behaviour instead of being an error. The registry's example is \
`after`: `after 200 script` — an integer first word means \"delay\", not an \
unknown subcommand. Declaring the accepted shape stops the false \
warning.",
    ),
    (
        "hover",
        "What the editor shows when the pointer rests on the command: a \
one-line summary, the synopsis line(s) as the man page writes them \
(`lappend varName ?value …?`), a short prose description, where the \
command comes from, an example call, and what it returns.\n\nWrite the \
summary like the first line of a man page — one sentence, present tense. \
This is pure documentation: nothing here changes any diagnostic, so it is \
the easiest high-value field to fill in.",
    ),
    (
        "forms",
        "The distinct documented ways the command can be called, each with a \
synopsis and kind — most usefully a read form and a write form: \
`$w cget -opt` versus `$w configure -opt value`, or `testConstraint NAME` \
(getter) versus `testConstraint NAME value` (setter). These rows document \
what users see in hover and reference output; they do not select an invocation \
or change arity, traits, effects, taint, or purity. Use `command_forms` when \
one call shape needs different executable semantics.\n\nA form may also be \
limited to a dialect or package lifecycle. Those facts are preserved when the \
Studio opens an existing spec; set them in the source or SpecTcl until the \
form editor exposes availability controls.",
    ),
    (
        "command_forms",
        "A per-form refinement for commands whose forms differ more deeply \
than a synopsis line can say. Alongside arity, roles and options, each form \
may replace the inherited `traits`, `mutator`, and `side_effects` facts. \
Replacement lets a query form remove a coarse parent mutation or callback \
classification instead of only adding more effects. A `selector` picks between \
overlapping-arity forms from known literal source words, with unique-prefix \
matching unless `-exact` says otherwise. The longest static selector wins when \
one selector extends another; substitutions and expansions abstain while a \
longer selector remains possible and retain the conservative parent facts.\n\n\
SpecTcl authors these as `refine NAME { … }` blocks. The descriptor's native \
halves — the completion contract, dispatch proofs, and the literal-argument \
validator — stay Rust-only, and a form carrying one is reported rather than \
thinned. Use plain `forms` only when the difference is documentation-only.",
    ),
    (
        "semantic_operation",
        "Names the abstract operation the command performs (\"list length\", \
\"dict get\") so the compiler backends can share one implementation across \
spellings. Only meaningful for commands the compiler executes; user \
packages leave it unset.",
    ),
    (
        "completion",
        "Which of Tcl's completion codes the command can finish with — \
normal return, `error`, `break`, `continue`, `return` — and what it \
promises about the result. `error` always raises; `break` only makes sense \
in a loop. This powers checks like \"this `break` is outside any loop\" \
and dead-code reasoning after a command that always raises.",
    ),
    (
        "assigns_variable_at",
        "The one argument position naming a variable the command writes — 0 \
for `set varName value`. This is the older, simpler cousin of a VarWrite \
argument role; prefer declaring the role, but either works, and for a \
one-target command they mean the same thing.\n\nWhat it buys: the written \
variable counts as *defined* afterwards, so \"used before set\" stays \
quiet, and rename reaches the name.",
    ),
    (
        "safe_on_uninit",
        "Whether the command may be handed a variable name that does not \
exist yet. `lappend v x` happily creates `v`; so does `append`. `incr` \
creates it in Tcl 8.5+ but errors in 8.4 — which is why this is a *set of \
dialects* rather than yes/no.\n\nThe compiler resolves this set for the \
active profile, stores the result in lowered IR, and W210 uses it for the \
command's own read-before-write. A VarWrite role still records the eventual \
definition. Without a concrete profile, lowering abstains and treats the \
operation as not safe.",
    ),
    (
        "const_fold",
        "A compile-time evaluator: when every argument is a literal, compute \
the result now — `string length abc` is always 3. This is code, carried by \
reference. If your command is a pure function of its arguments, saying so \
in the issue notes (plus the `PURE` trait) is what a maintainer needs.",
    ),
    (
        "const_fold_versioned",
        "The same as the constant folder, for commands whose literal result \
depends on the Tcl version being targeted (behaviour that changed between \
8.x and 9.x). Takes priority over the plain folder when both are set.",
    ),
    (
        "lowering_hook",
        "Compiler internals: picks a specialised translation of this command \
into the compiler's intermediate form (`if`, `foreach`, and friends have \
one). User packages leave this unset — the generic path handles any \
command.",
    ),
    (
        "codegen_hook",
        "Compiler internals: a specialised bytecode emitter for the Tcl VM, \
mirroring the commands C Tcl byte-compiles specially. Leave unset; the \
generic \"invoke the command\" path is always correct.",
    ),
    (
        "inline_codegen_hook",
        "Compiler internals: the bytecode emitter used when the command sits \
in value position (`set x [llength $l]`) or in a catch body. Leave unset \
for user packages.",
    ),
    (
        "bpf_op",
        "Only for the BPF-Tcl dialect: how this command lowers to a BPF \
operation. Anything outside that dialect leaves it unset.",
    ),
    (
        "native_lowering",
        "Compiler internals: which native code shape the executable-IR lowering \
gives this command — a structural hook, a cell read-modify-write, an \
intrinsic, a fixed completion, a scope link, or a definition. It is stamped \
beside the lowering hook or intrinsic it mirrors; unset means the generic \
argv invocation through runtime dispatch.",
    ),
    (
        "analyser_hook",
        "Compiler internals: routes the command to a hand-written analyser \
family (`proc`, `foreach`, `package require`, …) for behaviour the \
declarative fields cannot express. The goal of this whole form is to make \
these unnecessary — fill in roles, traits, and effects first, and reach for \
a hook only when something still cannot be said.",
    ),
    (
        "command_table_effect",
        "Whether the command changes which commands *exist*: `proc` defines \
one, `rename` moves or deletes one, `interp alias` creates one under \
another name. Declaring it keeps \"unknown command\" honest after the call \
— a name created by your command stops being reported as undefined.",
    ),
    (
        "side_effects",
        "What state the command touches, as structured reads and writes: \
variables, channels, files, the network, logs, HTTP headers, session \
tables, and so on — each with whether it reads, writes, or both, and (for \
iRules) which connection side. `puts` writes channel I/O; `file delete` \
writes filesystem state; `HTTP::header insert` writes HTTP headers on the \
current side.\n\nThis is the backbone of dead-code and ordering analysis: \
a command with no declared effects and no result being used looks \
removable. When a command does anything externally visible, say so \
here.",
    ),
    (
        "world_effects",
        "A compiler-oriented summary of the same idea as side effects: which \
broad domains of the running interpreter's world (variables, commands, \
namespaces, traces, channels …) the command reads, writes, or can call back \
into. Used by the optimiser to decide what survives across the call. \
Expression-valued; leave unset and the optimiser stays conservative.",
    ),
    (
        "state_transitions",
        "Declares precise identity changes the command performs on the Tcl \
world — a command coming into being, a namespace appearing, a trace being \
attached, a variable cell changing identity. Finer-grained than world \
effects; used by the most exacting optimiser proofs. Leave unset unless a \
maintainer asks for it.",
    ),
    (
        "dispatch_dependencies",
        "What must stay *unchanged* for the registry's knowledge about this \
command to remain trustworthy at a call site — e.g. that nobody renamed or \
shadowed the command in between. Compiler-proof machinery; leave unset.",
    ),
    (
        "result_stability",
        "Whether calling the command twice with the same arguments yields \
the same value. `string length` always does; `clock seconds` never does; \
`info commands` depends on what has been defined in the meantime. Purity \
says \"no side effects\"; this says \"same answer again\" — a command can \
be pure yet unstable (`clock seconds` changes nothing but never repeats). \
The optimiser only reuses results it can prove stable.",
    ),
    (
        "option_placement",
        "Where this command's declared options may be found. `Leading` — the \
default, and what every core Tcl command does — stops option parsing at the \
first word that is not a declared option, so a later `-`-looking word is a \
positional. `Anywhere` keeps recognising options between the positional words \
up to an explicit `--`, which is the shape of a script-level parser that loops \
`foreach {flag value} $args` after taking its fixed arguments (`http::geturl`). \
Getting this wrong invents option relations the interpreter never applies.",
    ),
    (
        "constraints",
        "The rare escape hatch for an option relation no declarative row can \
express (E-R14). It is consulted **only** when the spec declares one and every \
`option_conflict` / `option_requires` / `option_requires_one_of` / \
`option_forbids` row already reported nothing — reach for a declarative row \
first, because those are checked natively with no VM entry. Declare \
`-inputs {invocation}` so the verdict is cached on the call's content.",
    ),
    (
        "literal_argument_validator",
        "A hook validating relationships *between* literal arguments that a \
per-position value list cannot express — \"this mode word is only legal \
when that flag is present\". Code, carried by reference; spell the rule \
out in the issue notes.",
    ),
    (
        "inferred_storage_type",
        "The container kind the command's target variable ends up holding: \
`array set` makes an array, `lappend` a list, `dict set` a dict. Downstream \
reads of that variable are then understood — and mixing kinds (using a \
dict variable as an array) is flagged.",
    ),
    (
        "required_package",
        "The package a script must `package require` before this command \
exists — `sqlite3` for the `sqlite3` command. Until the require is seen, \
the command is hidden from completion and its use draws the \
missing-import warning; after it, everything lights up. Leave unset for \
commands that are simply always there.",
    ),
    (
        "tk_geometry",
        "Declares how a Tk geometry manager chooses and owns its effective \
container. `Exclusive` managers such as `pack` and `grid` conflict when both \
claim one container; `Independent` managers such as `place` do not claim that \
exclusive ownership. Set the container option to `-in` when calls may redirect \
placement away from the widget's lexical parent. Static preview and TK1001 \
consume that target. Declare whether the default form places widgets, the \
placement subcommand (usually `configure`), and every subcommand that releases \
widgets (`forget`, plus `remove` for `grid`). Both consumers read the whole \
descriptor through the registry, so adding a manager needs no command-name \
branch in either consumer.",
    ),
    (
        "container_policy",
        "`Exclusive` claims propagation ownership of the effective container, \
so a different exclusive manager conflicts there; `Independent` positions \
widgets without making that claim.",
    ),
    (
        "container_option",
        "The geometry option whose literal value replaces the widget pathname's \
parent as the effective container. Tk's built-in managers use `-in`; leave it \
unset only when the manager has no such redirection option.",
    ),
    (
        "direct_form",
        "Set when `manager widget ?options?` itself places widgets. Clear it for \
a manager that only places through a named subcommand.",
    ),
    (
        "placement_subcommand",
        "The subcommand that places or reconfigures its widget arguments, such \
as `configure` for `pack`, `grid`, and `place`. Leave unset when none exists.",
    ),
    (
        "release_subcommands",
        "Every subcommand that stops managing its widget arguments. Order is \
descriptive; include both `forget` and `remove` when their persistence details \
differ but both release current placement.",
    ),
    (
        "excluded_events",
        "iRules only: event contexts where this command must not be used, \
by event name (`HTTP_REQUEST`, `CLIENT_ACCEPTED`, …). The validity check \
reports a use inside any listed event.",
    ),
    (
        "taints_var_write",
        "Marks one variable-valued option as an external-input link. Use it \
when later user interaction can write the named variable, as with an editable \
entry or combobox `-textvariable`. Leave it off for display-only links such as \
a label's `-textvariable`. This taints the linked variable's SSA definition; \
it does not taint the widget command returned by its constructor. The option \
value must have `VarWrite` as its role or secondary role; the Studio hides \
this control for values that cannot write a variable.",
    ),
    (
        "variable_scope",
        "Controls where a variable-valued option resolves an unqualified name. \
`CurrentFrame` is Tcl's ordinary rule: a name used inside a procedure denotes \
that procedure's local unless aliased. `Global` is for APIs such as Tk \
`-textvariable` and `-variable`, whose manuals explicitly link a global \
variable; `value` therefore denotes `::value` even when the widget is created \
inside a procedure. Set this only on a `VarRead` or `VarWrite` option value.",
    ),
    (
        "script_timing",
        "Separates *when* a script runs from `body_kind`, which says only which \
frame it uses. `SameInvocation` means the receiving command may evaluate the \
script before it returns, so an error or return can affect current control \
flow. `Deferred` means the command stores it for a later callback, as Tk does \
for `-command` and `-validatecommand`; `ReferenceOnly` means executable text \
is matched or queried but never invoked, as in `trace remove`. Neither can \
abort the receiving command or hide its definitions. Set this only on an \
executable option (`Body`, `LambdaLiteral`, or `CommandPrefix`).",
    ),
    (
        "method_prefix_matching",
        "Controls lookup in this object's instance-method table, independently \
of the command's own `prefix_matching`. `Strict` (the safe default) requires \
the complete method spelling. `Enabled` accepts a non-empty prefix only when \
it identifies exactly one declared method; an ambiguous prefix still abstains. \
Enable it only when the runtime's object dispatcher is documented or source-proven \
to accept unique prefixes, as Tk widget commands do.",
    ),
    (
        "unsafe_command",
        "Marks a command that escapes the sandbox in restricted dialects — \
in iRules, things that reach the underlying system. Drives the \
\"unsafe command\" security diagnostic there. Not related to Tcl's safe \
interpreters (that is the `SAFE_INTERP_HIDDEN` trait).",
    ),
    (
        "closed_value_args",
        "Argument positions whose legal values are *exactly* the ones \
declared under argument values — a closed set, like an enum. A value \
outside the set is then a diagnostic, not just a missing completion. Only \
close a position when the real command genuinely accepts nothing else.",
    ),
    (
        "event_requires",
        "iRules only: what the surrounding event context must provide for \
this command to work — transport layer, profile, connection side. Feeds \
the \"command not valid in this event\" check. Named descriptor; describe \
the requirement in words in the issue notes.",
    ),
    (
        "event_requirement_forms",
        "iRules only: overrides of the event requirements for specific \
argument spellings — when `CMD mode-a` is valid in different events than \
`CMD mode-b`. Named descriptor, like the event requirements themselves.",
    ),
    (
        "data_collection",
        "iRules only: for the `collect`/`release`/payload family — which \
protocol, which action, when payload data is available, and how release \
behaves. Drives the collect/release pairing diagnostics and their quick \
fixes.",
    ),
    (
        "side_switch_target",
        "iRules only: for commands whose body runs in the *other* side's \
context (`clientside { … }` / `serverside { … }`) — which side the body \
switches to. Side-sensitive commands inside the body are then checked \
against the right side.",
    ),
    (
        "event_handler_priority",
        "iRules only: for event-handler commands like `when` — the runtime's \
default priority (BIG-IP uses 500) and whether omitting an explicit \
priority is worth reporting.",
    ),
    (
        "irules_top_level_effect",
        "iRules only: declares a file-level command whose effect persists for \
later declarations. `priority N`, for example, changes the inherited \
priority of following `when` handlers until another priority declaration \
replaces it.",
    ),
    (
        "options",
        "The command's `-flag` switches, each with whether it takes a value \
(`-nocase` takes none; `-index i` takes one), what role and type the value \
has, which dialects have the flag, and a one-line description for \
completion. Declare `--` here too if the command accepts it as an \
end-of-options marker — that is what enables the \"put `--` before a \
dynamic value\" safety warning.\n\nDeclared options get completion, spelling \
checks, and correct highlighting of flag-versus-value; undeclared ones are \
reported as unknown.",
    ),
    (
        "option_relations",
        "What this command's options and arguments require of one another. \
Four relations, and the checker evaluates every one of them natively — no \
script runs, whatever the document does. `option_conflict {-glob -regexp}` is \
the symmetric \"not together\"; `option_requires -command {-channel}` is the \
directional one (`bibtex::parse`'s `-command` is a channel callback and is \
useless without `-channel`); `option_requires_one_of {} {-channel {arg 0}}` \
says a call must supply at least one of a set, subject optional; and \
`option_forbids {-order in} {{-type bfs}}` is the asymmetric exclusion \
(`struct::tree walk` rejects an in-order breadth-first walk). A term is an \
option (`-channel`), an option carrying a value (`{-type bfs}`), a positional \
argument (`{arg 0}`), or a positional carrying a value (`{arg 1 text}`). \
Absence is only ever proven on a call the analyser could read to its end, so a \
`{*}$opts` call abstains instead of accusing.",
    ),
    (
        "reserved_trailing_words",
        "How many words at the *end* of the call are never option \
candidates, matching how C Tcl scans options only up to a point. \
`lsearch ?options? list pattern` reserves the final 2: a pattern that \
happens to start with `-` is data there, not a flag.",
    ),
    (
        "arg_values",
        "The completable values for specific argument positions — the mode \
words of `binary scan`, the event names for an iRules command, the \
subcommand-like keywords of a mode argument. Purely additive for \
completion and hover unless the position is also listed under closed \
value arguments, which upgrades it to \"only these\".",
    ),
    (
        "body_kind",
        "Whether a Body argument runs *in the caller's frame*, seeing and \
changing the caller's variables (`while`, `if`, `catch` — \"Plain\"), or \
in a separate context of its own (`proc` bodies, class definition bodies — \
\"Structural\"). Plain bodies join the surrounding data flow: a `set` \
inside them changes the enclosing scope. Structural bodies deliberately do \
not.",
    ),
    (
        "body_interpreter",
        "Which Tcl interpreter owns evaluated Body arguments. `Current` is \
the normal case. `Argument` names the complete post-command argument index \
whose evaluated value selects another interpreter, as in `interp eval`. \
This is independent of body kind: interpreter ownership and stack-frame \
shape are separate axes.",
    ),
    (
        "body_arg_implicit_args",
        "For callback-style bodies whose first command receives extra \
positional arguments supplied by the runtime when it invokes the body. \
Rare; leave at 0 unless the command's documentation spells such arguments \
out.",
    ),
    (
        "taint_output_sink",
        "Marks the command as a place where attacker-influenced data becomes \
*output* — echoed into a page or response — and names the diagnostic to \
raise when tainted data reaches it (cross-site-scripting style). The value \
is the diagnostic code; leave unset for commands that are not output \
sinks. See the Reference tab's taint-colour section for how taint is \
tracked.",
    ),
    (
        "taint_output_sink_subcommands",
        "Restricts the output sink to specific subcommands — `respond`-like \
operations — so the rest of the ensemble stays clean. Empty means the sink \
applies to every invocation of the command.",
    ),
    (
        "taint_log_sink",
        "Like the output sink, but for log writes: tainted data reaching a \
log line is a log-injection finding (forged entries via embedded \
newlines). The value is the diagnostic code to raise.",
    ),
    (
        "taint_network_sink_args",
        "Argument positions that take a network destination (host, URL). \
Tainted data reaching one is a server-side request forgery finding — an \
attacker steering *where* the script connects.",
    ),
    (
        "taint_code_sink_args",
        "Argument positions where a value is evaluated as code. Tainted data \
reaching one is the classic injection: `eval $userInput`. Declaring the \
precise slots keeps the finding accurate on commands where only some \
arguments are executed.",
    ),
    (
        "taint_interp_eval_subcommands",
        "Subcommands that evaluate code in *another* interpreter (`interp \
eval` style). Tainted data reaching them raises the cross-interpreter \
evaluation finding.",
    ),
    (
        "taint_source",
        "Declares the command's *result* as attacker-influenced — the way \
`HTTP::header` or a socket read hands you data the client controls. The \
colours say what is known about the value beyond \"tainted\"; usually just \
`TAINTED`. Everything derived from a tainted value stays tainted until a \
sanitiser cleans it, and sinks report when raw taint reaches them.",
    ),
    (
        "taint_transform",
        "Declares the command a *sanitiser* or encoder: the colours it adds \
to a value passing through. An HTML-escaper adds `HTML_ESCAPED`; `file \
join` adds path colours; a validator that proves \"this is an IP address\" \
adds `IP_ADDRESS`. A sink that requires a given colour then accepts the \
cleaned value — this is how \"escaped before output\" is recognised.",
    ),
    (
        "taint_double_encode_colour",
        "The colour that means the input is *already* encoded the way this \
command encodes. Feeding an HTML-escaped value through the HTML escaper \
again produces `&amp;amp;` — declaring the colour lets the double-encoding \
check catch exactly that.",
    ),
    (
        "taint_sink_safe_colour",
        "For a command that is a sink: the colour(s) that make a tainted \
value acceptable here. An output sink might accept `HTML_ESCAPED`; an exec \
sink might accept `SHELL_ATOM`. A tainted value carrying the required \
colour passes without a finding.",
    ),
    (
        "taint_sink_gate",
        "A predicate deciding whether the sink applies to *this particular \
call*, based on the call's own flags — `subst -novariables` is a different \
risk than bare `subst`. Code, carried by reference; state the condition in \
the issue notes.",
    ),
    (
        "credential_options",
        "Option flags whose value is a secret — `-password`, `-token`. A \
literal secret passed to one is reported as a hard-coded credential, and \
the value is treated as sensitive by anything that echoes code.",
    ),
    (
        "sensitive_headers",
        "HTTP header names whose values are secrets (`Authorization`, \
`Cookie`). Reads of these through this command are treated as sensitive \
data for the credential-handling checks.",
    ),
    (
        "setter_constraints",
        "iRules hardening: setter forms that must be called with a given \
literal argument prefix to be safe — the pattern behind \"this header must \
be set with an explicit name, not a variable\". Drives its own diagnostic; \
rarely needed outside the F5 command packs.",
    ),
    (
        "pattern_type",
        "Which pattern language the command's Pattern argument speaks: glob \
(`string match`, `lsearch` default) or regular expression (`regexp`, \
`regsub`, `lsearch -regexp`). The pattern is then checked and highlighted \
in the right language — a `*` means something very different in each.",
    ),
    (
        "pattern_arg_resolver",
        "A native hook that selects the Pattern argument positions and language \
for this particular call. Use it when options change the pattern grammar, such \
as `lsearch -regexp`; the Studio preserves the need for the hook but cannot \
recover a Rust function pointer from a loaded spec, so supply the expression.",
    ),
    (
        "format_string_type",
        "Which template mini-language the command's format argument uses: \
printf-style (`format`/`scan`), `clock format` fields, `binary` \
format/scan cursors, or `regsub` replacement backreferences. The template \
is then validated in the right language — `%b` is a fine clock field but \
means binary in printf.",
    ),
    (
        "tcllib_package",
        "When the command comes from a tcllib module, the module name \
(`json`, `struct::list`). Works like the required package — the command \
activates for a document once the matching `package require` is seen — \
and also labels the command's origin in completion.",
    ),
    (
        "introduced_version",
        "The version of the owning package (or of Tcl itself, for core \
commands) that first shipped the command — `8.5` for `dict`, `8.6` for \
`try`. Using the command under an older target dialect is then reported, \
which is how \"this needs Tcl 8.6\" warnings work.\n\nThe same three \
releases sit on everything a version can gate, not just the command: an \
option, a subcommand, a second-level subcommand, an invocation form, a side \
effect, an option conflict, and a single enumerable argument value each carry \
their own, edited in their own row.",
    ),
    (
        "deprecated_version",
        "The first version of the owning package where the command still \
works but is discouraged. From this version on, uses draw a deprecation \
warning (and the replacement below, if named, is offered).",
    ),
    (
        "retired_version",
        "The first version *without* the command — exclusive, so \"retired: \
9.0\" means gone *in* 9.0, present in 8.6. Uses under a dialect at or past \
this version are reported as errors, not warnings.",
    ),
    (
        "deprecation_fix",
        "The quick fix the editor offers on a deprecated call — typically \
\"replace this word with the new spelling\", with a safety level saying \
whether the replacement is semantically identical. Carried as an \
expression; name the replacement and whether arguments change in the \
issue notes.\n\nAn option row carries its own, so a renamed flag can offer \
the new spelling without the whole command being deprecated. A fix that is a \
registry callback rather than a replacement word cannot be written down here \
— the studio says so rather than dropping it.",
    ),
    (
        "warn_missing_import",
        "Whether using the command without its `package require` draws the \
missing-import warning. On by default when a required package is set; turn \
it off for commands an environment auto-loads — the Tk commands under \
`wish` are the classic case: present without any visible require.",
    ),
    (
        "is_namespace_exported",
        "Whether the owning namespace exports the bare name, so `namespace \
import` can bring it in — i.e. whether `string` alone can ever mean \
`::textutil::string`. Affects how unqualified uses resolve after an \
import.",
    ),
    (
        "xc_translatable",
        "F5 only: whether the iRules-to-XC translator can carry this command \
across. Unset follows the default rules; set it only to override them in \
either direction.",
    ),
    (
        "deprecated_replacement",
        "The command to use instead, shown in the deprecation warning and \
offered by the quick fix — the `lmap` to your deprecated mapping \
helper.",
    ),
    (
        "deprecated_replacement_drop_in",
        "Whether the replacement accepts the *same argument list* unchanged \
— if yes, the quick fix can rewrite calls automatically; if no, it only \
points at the replacement and leaves the arguments to the author.",
    ),
    (
        "byte_array_payload",
        "F5 only: describes a `<proto>::payload`-style command's layout so \
the binary-data corruption check (string operations applied to raw \
payload bytes) knows where the bytes flow.",
    ),
    (
        "byte_array_effect",
        "What happens when the command's operand is binary data (a byte \
array): passed through intact, silently coerced to a string (corrupting \
it), case-folded, re-encoded, or re-binarified. This powers the \
\"binary data corrupted by string operation\" check — Tcl's classic \
gotcha where `string tolower` quietly destroys bytes.",
    ),
    (
        "definition_body",
        "For commands that *define a class or type* with a body of member \
declarations — `oo::class create`, `snit::type`, `itcl::class`. The \
grammar lists the member keywords (`method`, `constructor`, `variable`, \
…) and which words of each are the name, the parameter list, and the \
body, so navigation, folding, and highlighting work inside the class \
body with no code written.\n\nGrammars are shared, named descriptors: if \
your package has its own definer, the studio cannot author the grammar \
inline — describe the member keywords and their shapes in the issue \
notes.",
    ),
    (
        "manufacturer_methods",
        "For class-like commands: which methods manufacture an instance \
(`new`, `create`), which argument (if any) names the instance command \
being created, and where constructor arguments start. This is how \
`oo::class create Foo` makes `Foo` a known command, and `set o [Foo new]` \
makes `$o` a known object.",
    ),
    (
        "case_list",
        "For commands taking a final braced `{pattern body pattern body …}` \
clause list — `switch`'s second form. The descriptor says how the pairs \
read, so each body is analysed as a script and each pattern in the right \
pattern language.",
    ),
    (
        "oo_context_facts",
        "TclOO fine print: keyword words whose value is fixed by the \
enclosing method frame (`self`, the defining class), letting the \
optimiser fold them. Leave unset outside the TclOO core.",
    ),
    (
        "self_receiver_words",
        "TclOO fine print: for introspection commands where one specific \
word's result is the current object itself — `[self] m` dispatching like \
`my m`. Lists the argument words for which that holds (`self`'s \
`object`).",
    ),
    (
        "object_class",
        "Attaches class metadata to a factory command: the methods its \
instances answer to, superclasses for inherited resolution, and whether \
unknown methods are acceptable. With it, `$obj method args` gets method \
completion, arity checks, and option highlighting — the full treatment a \
built-in ensemble gets.\n\nPlain data all the way down — the instance \
methods are ordinary subcommands — so a pack can author the whole thing: \
`object_class NAME ?-superclass {…}? ?-allow-unknown? { method NAME { … } }`, \
where each `method` body is the `subcommand` body grammar unchanged. The \
class NAME is not always the command name: a factory may manufacture a \
differently-named class.",
    ),
    (
        "defines_symbol",
        "Marks a command that *names* something worth listing in the \
document outline — `tcltest::test` names a test case, `tcltest::\
testConstraint` a constraint. Says which argument is the name, which (if \
any) is a description, and the outline category. The named things then \
appear in outline and workspace-symbol search.",
    ),
    (
        "body_scope",
        "Extra commands that exist only *inside* this command's body \
argument — a mini-vocabulary like snit's `install` inside a type body, or \
a report-writing DSL's directives. Keeps those words resolving inside the \
body without leaking them into the global namespace.",
    ),
    (
        "binds_handle",
        "Declares that a call makes a *variable* hold an object handle, and \
which word says the handle's class — the `set axis [::verticalAxis \
$win.a]` and `install axis using ::verticalAxis …` shapes. With it, the \
variable's later `$axis method …` calls resolve against the right \
class.",
    ),
    (
        "remote_method",
        "Declares that the command takes part in a *cross-language* RPC \
family: it either opens a handle onto a remote extension (`ILX::init \
PLUGIN EXTENSION`) or invokes a method that extension implements in \
another language (`ILX::call HANDLE ?-timeout ms? ?--? METHOD …`). Says \
which word carries the handle, where the method name sits — at a fixed \
index, or after the command's own leading options — and whether the call \
waits for a reply. With it, go-to-definition on the method word crosses \
into the Node.js `ILXServer.addMethod` registration that implements it.",
    ),
    (
        "creates_instance_at",
        "The argument position that names an object command of this spec's \
own class — the `Foo` in `oo::class create Foo`. After the call, `Foo` is \
a known command dispatching this class's methods.",
    ),
    (
        "defines_command_at",
        "The argument position whose *literal* value becomes a callable \
command once the call runs — the `NAME` of `coroutine NAME cmd …`, or (on \
the subcommand) `interp create name`. Later calls to that name stop being \
\"unknown command\". Dynamic words at the position are simply not \
recorded — no guessing.",
    ),
    (
        "context_gate",
        "A validity rule keyed on *where* the call sits rather than what its \
arguments are — `return -code` spellings only valid inside a procedure, \
iRules commands only valid at the top level of an event. Code, carried by \
reference; describe the context rule in the issue notes.",
    ),
    (
        "implementation_namespace",
        "For ensembles whose subcommands are also reachable as plain \
commands in a namespace — `::tcl::string::length` behind `string length`. \
Naming the namespace makes both spellings resolve to the same spec.",
    ),
    // --- Subcommand-only keys -------------------------------------------
    (
        "detail",
        "A few words for the completion list — what shows next to the \
subcommand name in the picker. `string length` says \"the number of \
characters\". Keep it under a dozen words; the hover carries the long \
version.",
    ),
    (
        "synopsis",
        "The usage line for this subcommand as a man page would write it: \
`string length string`, `dict get dictionary ?key …?`. Shown in \
completion and hover, and worth writing even when nothing else is \
filled in.",
    ),
    (
        "pure",
        "Side-effect free: the subcommand changes nothing — no variables, \
no I/O, no interpreter state. `string length` is pure; `lappend` is not. \
Purity feeds the optimiser and lets \"result unused\" warnings fire (a \
pure call whose result is discarded does nothing at all).",
    ),
    (
        "mutator",
        "The opposite declaration: this subcommand changes state — a \
variable, a table, the interpreter. `dict set` and `array unset` are \
mutators. A subcommand can be neither (unknown), but never both.",
    ),
    (
        "min_abbrev",
        "Unique-prefix abbreviation is computed automatically; this field is \
only for the rare subcommand whose *documented* minimum abbreviation is \
longer than uniqueness requires. Leave unset almost always.",
    ),
    (
        "arity_windows",
        "Per-release signature shapes, for the rare command whose argument \
count changed between releases of the package that owns it. Leave empty \
unless it did — the plain arity above already describes a signature that \
never changed, and it stays the fallback whenever no window covers the \
document's resolved floor. Windows must not overlap, so consecutive ones are \
written closed: retire each where the next is introduced.",
    ),
    (
        "versioned_arg_values",
        "Version gates for individual literal argument values — when one mode \
word appeared in (or left) a specific package release, like a persistence \
mode added mid-release-train. Indices count from after the command name at \
command level and from after the subcommand word at subcommand level. The \
value list itself lives under argument values; this adds the since/until per \
value.",
    ),
    (
        "subcommand_forms",
        "Per-form refinement for this subcommand — the subcommand-level twin \
of `command_forms`, written the same way, as `refine NAME { … }`. A form may \
replace the parent row's `traits`, `mutator`, and `side_effects`, which is how \
one method can be a read at one arity and a mutation at another. Its optional \
`selector` also separates same-arity operation words without treating a \
computed word as literal, and the longest statically matched selector wins \
when selectors overlap.",
    ),
    (
        "loop_list_header",
        "Marks the subcommand a loop header whose arguments include list \
expressions evaluated once before the body iterates — the `dict for` \
shape. Feeds loop analysis; leave off for anything that is not a \
loop.",
    ),
    (
        "creates_scope_alias",
        "Marks the subcommand as creating an `upvar`-style alias: after it, \
one name is another variable in disguise (`namespace upvar` does this). \
Writes through the alias then count as writes to the real \
variable.",
    ),
    (
        "arg_values_accept_prefix",
        "Whether this subcommand's closed argument values accept unique \
prefixes the way keyword tables do — `persist add u` for `uie`. Off means \
exact spellings only.",
    ),
    (
        "credential_arg",
        "The argument position whose value is a secret — a password or key \
handed to this specific subcommand. A literal there is a hard-coded \
credential finding.\n\nCoordinate warning: unlike every other subcommand \
index field, the consumer counts the subcommand word itself as 0 — \
`HTTP::header insert name value` declares 2 for the value slot. Store the \
index verbatim; never re-base it.",
    ),
    (
        "destructive",
        "An irreversible operation — `file delete`, a table purge. Feeds \
the \"destructive operation\" cautions and keeps such subcommands out of \
casually suggested quick fixes.",
    ),
    (
        "returns_path",
        "The result is a filesystem path (`file join`, `file dirname`). \
Path-aware checks then follow the value — e.g. the path-taint colours \
that prove a user-influenced path stays inside a known root.",
    ),
    (
        "is_unescape",
        "The subcommand *decodes* — URL-decoding, HTML-unescaping. In taint \
terms it undoes sanitisation: a value that was safe because it was \
encoded is dangerous again after this returns.",
    ),
    (
        "cfg_rewrite_name",
        "Compiler internals: the plain command name this ensemble \
subcommand is rewritten to during lowering. Leave unset for user \
packages.",
    ),
    (
        "sub_subcommands",
        "A third level of keywords — operations selected by the word *after* \
this subcommand, as in `info object isa`. Deliberately lighter than a full \
subcommand: each carries its name, a one-line detail, a synopsis, and an \
optional dialect gate — enough for highlighting, hover, and completion. \
Arity stays on the owning subcommand. An operation may also carry its own \
**option table**, and should whenever the operations disagree about which \
options exist: `namespace ensemble create` takes `-command`, `configure` \
takes `-namespace`, and each rejects the other's. A table here replaces the \
subcommand's for that operation rather than adding to it. Leaving it unset \
inherits the subcommand's table; setting it to an *empty* table says the \
operation takes no options at all, which is a different claim — \
`namespace ensemble exists` needs the second, or it is offered flags that \
are not options there.",
    ),
    (
        "max_leading_option_words",
        "A cap on how many leading words the option scan will consume for \
this subcommand; anything past the cap is positional even if it starts \
with `-`. Matches commands whose C implementation stops looking for \
options after a fixed count.",
    ),
];

/// Long-form help per form group, shown from the **?** on the group heading.
pub const GROUP_HELP: &[(&str, &str)] = &[
    (
        "Identity",
        "What the command is called. The name is the anchor everything else \
hangs off — get it exactly as scripts type it, namespace and all.",
    ),
    (
        "Availability",
        "Where and when the command exists: which dialects ship it, which \
package must be required first, and the version that introduced, \
deprecated, or removed it. This group is what makes \"unknown command\", \
\"needs Tcl 8.6\", and \"missing package require\" accurate — for most \
third-party commands it is the highest-value group after the name and \
arity.",
    ),
    (
        "Arity and arguments",
        "How many arguments the command takes and what each position means. \
Arity powers the wrong-number-of-arguments check; argument roles tell \
every tool which words are scripts, variable names, patterns, and \
channels — which is what makes highlighting, rename, and \"unused \
variable\" work through your command the way they work through `foreach`.",
    ),
    (
        "Types",
        "What kind of values flow in and out: the return type, per-argument \
expectations, and how written variables are typed. Everything here feeds \
type inference and the shimmering / wrong-type warnings. All optional — \
unset means \"unknown\", never \"wrong\".",
    ),
    (
        "Subcommands",
        "For ensemble commands (`string length`, `dict get`): one entry per \
operation word, each a small spec of its own. Indices inside a subcommand \
are counted after the subcommand word. Unique-prefix abbreviation is \
handled automatically — declare full names only.",
    ),
    (
        "Documentation",
        "What the editor shows humans: hover text, synopsis lines, and \
completion details. Nothing here changes a diagnostic — it is the safest \
group to fill in generously, and the one users see most.",
    ),
    (
        "Options and values",
        "The command's `-flag` switches and the literal values specific \
argument positions accept. Declared options get completion, spelling \
checks, and flag-versus-value highlighting; declared values get \
completion, and can be closed into an \"only these\" set.",
    ),
    (
        "Behaviour",
        "The command's behavioural traits — the facts every analysis reads \
instead of special-casing command names: does it evaluate code, alter \
control flow, run a loop body, mutate state, act as a language keyword? \
The Reference tab lists every trait with its meaning.",
    ),
    (
        "Side effects",
        "What state the command touches — variables, channels, files, \
network, logs, HTTP state — and how stable its result is across calls. \
This is what dead-code, ordering, and result-reuse reasoning stand on. \
Declaring nothing is safe but blinds those checks to your command.",
    ),
    (
        "Compiler hooks",
        "Named entry points into the compiler for commands that need \
special-cased lowering, bytecode, or analysis. Core Tcl commands use \
these; a third-party command spec almost never should — prefer expressing \
behaviour through roles, traits, and effects, which need no code.",
    ),
    (
        "Taint and security",
        "How attacker-influenced data flows through the command: whether it \
is a source (returns untrusted data), a sink (a dangerous place for \
untrusted data to arrive), or a sanitiser (adds a safety colour as data \
passes through). The colours are listed on the Reference tab. Only \
security-relevant commands need anything here.",
    ),
    (
        "Deprecation and translation",
        "The replacement story for ageing commands — what to use instead \
and whether the switch is a drop-in — plus the F5 XC translation \
mapping.",
    ),
    (
        "Advanced",
        "Fields the studio carries as raw Rust expressions: function \
pointers and references to shared, named descriptors. You can see that a \
loaded command sets one, but not edit it structurally. When your command \
needs one, describe the behaviour in plain words in the issue notes — \
that description is exactly what a maintainer needs to write the few \
lines of Rust.",
    ),
];

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
        "patternType",
        "Pattern types",
        "The two pattern languages a Pattern argument can speak: glob \
(`string match`) and regular expressions (`regexp`). A `*` means \
something different in each, so the right label matters for validation \
and highlighting.",
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
        "optionArity",
        "Option arity",
        "How many value words an option consumes: one (`-index i`) or a \
fixed count. Options that take no value are declared by leaving \
takes-value off instead.",
    ),
];

/// The long-form help for a field key, shared between the command and
/// subcommand schemas.
#[must_use]
pub fn field_help(key: &str) -> Option<&'static str> {
    FIELD_HELP
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, text)| *text)
}

/// The long-form help for a form group heading.
#[must_use]
pub fn group_help(group: &str) -> Option<&'static str> {
    GROUP_HELP
        .iter()
        .find(|(g, _)| *g == group)
        .map(|(_, text)| *text)
}

/// The `(title, introduction)` for a catalogue id.
#[must_use]
pub fn catalogue_help(id: &str) -> Option<(&'static str, &'static str)> {
    CATALOGUE_HELP
        .iter()
        .find(|(i, _, _)| *i == id)
        .map(|(_, title, intro)| (*title, *intro))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;

    /// Every schema field key, from both tables, deduplicated.
    fn all_schema_keys() -> Vec<&'static str> {
        let mut keys: Vec<&'static str> = schema::COMMAND_FIELDS
            .iter()
            .chain(schema::SUBCOMMAND_FIELDS)
            .map(|f| f.key)
            .collect();
        keys.extend(schema::NESTED_FIELDS.iter().map(|field| field.key));
        keys.sort_unstable();
        keys.dedup();
        keys
    }

    /// A **?** on everything: every field of both schemas has long-form help.
    #[test]
    fn every_schema_field_has_help() {
        for key in all_schema_keys() {
            let text = field_help(key)
                .unwrap_or_else(|| panic!("field {key} has no long-form help in help::FIELD_HELP"));
            assert!(
                text.len() >= 60,
                "help for {key} is too short to be worth a button"
            );
        }
    }

    /// No orphans: every help entry names a field that still exists.
    #[test]
    fn every_help_entry_names_a_live_field() {
        let keys = all_schema_keys();
        for (key, _) in FIELD_HELP {
            assert!(
                keys.contains(key),
                "help::FIELD_HELP names {key}, which is not a schema field"
            );
        }
    }

    #[test]
    fn field_help_keys_are_unique() {
        let mut seen: Vec<&str> = Vec::new();
        for (key, _) in FIELD_HELP {
            assert!(!seen.contains(key), "duplicate help entry for {key}");
            seen.push(key);
        }
    }

    /// Every group heading has help, and no help names a dead group.
    #[test]
    fn group_help_matches_the_group_list() {
        for group in schema::GROUPS {
            assert!(
                group_help(group).is_some(),
                "group {group} has no help in help::GROUP_HELP"
            );
        }
        for (group, _) in GROUP_HELP {
            assert!(
                schema::GROUPS.contains(group),
                "help::GROUP_HELP names {group}, which is not a form group"
            );
        }
    }

    /// Every picker catalogue has a title and introduction, and no entry
    /// names a catalogue that no longer exists.
    #[test]
    fn catalogue_help_matches_the_catalogues() {
        let cats = schema::catalogues();
        let cats = cats.as_object().expect("catalogues is an object");
        for id in cats.keys() {
            assert!(
                catalogue_help(id).is_some(),
                "catalogue {id} has no entry in help::CATALOGUE_HELP"
            );
        }
        for (id, _, _) in CATALOGUE_HELP {
            assert!(
                cats.contains_key(*id),
                "help::CATALOGUE_HELP names {id}, which is not a catalogue"
            );
        }
    }
}
