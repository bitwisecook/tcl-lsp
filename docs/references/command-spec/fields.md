# CommandSpec fields

> Generated from the spec studio's schema — do not edit by hand.
> Regenerate with `UPDATE_REFERENCE=1 cargo test -p tcl-spec-studio --test reference_doc`.

Every field of `CommandSpec` and `SubCommand`, grouped the way the [Spec Studio](https://bitwisecook.github.io/tcl-lsp/spec-studio/) groups its form. The same text sits behind the studio's **?** buttons. Impact tables — which fields drive which diagnostics, optimisations, and editor features — live in [README.md](README.md).

## Identity

What the command is called. The name is the anchor everything else hangs off — get it exactly as scripts type it, namespace and all.

### `name` — Command name

*command and subcommand* — Command name as written in Tcl — `for`, `dict`, `HTTP::header`.

The word a script calls the command by — exactly as it is typed in Tcl, including any namespace: `lappend`, `dict`, `HTTP::header`, `::mypkg::frobnicate`. For a namespaced command, use the fully qualified name without a leading `::`; the registry normalises a leading `::` at look-up time, so `::set` and `set` resolve to the same spec.

On a subcommand this is the subcommand word itself (`length` in `string length`).

## Availability

Where and when the command exists: which dialects ship it, which package must be required first, and the version that introduced, deprecated, or removed it. This group is what makes "unknown command", "needs Tcl 8.6", and "missing package require" accurate — for most third-party commands it is the highest-value group after the name and arity.

### `dialects` — Dialects

*command and subcommand* — Dialects the command exists in. Unset means every dialect.

Which Tcl worlds the command exists in — every dialect the picker lists, from the core releases through the F5, Tk, Expect, BPF and SpecTcl surfaces. A command only present from 8.5 onwards ticks 8.5 and every later release; one that is iRules-only ticks just F5 iRules. Leave the whole field unset for "every dialect".

This is what makes the same file lint differently as Tcl 8.4 versus 9.0: a command outside the active dialect is reported as unknown there.

### `safe_on_uninit` — Safe on uninitialised

*command and subcommand* — Dialects where the command safely initialises an uninitialised variable.

Whether the command may be handed a variable name that does not exist yet. `lappend v x` happily creates `v`; so does `append`. `incr` creates it in Tcl 8.5+ but errors in 8.4 — which is why this is a *set of dialects* rather than yes/no.

The compiler resolves this set for the active profile, stores the result in lowered IR, and W210 uses it for the command's own read-before-write. A VarWrite role still records the eventual definition. Without a concrete profile, lowering abstains and treats the operation as not safe.

### `required_package` — Required package

*command only* — Package that must be `require`d before the command is visible.

The package a script must `package require` before this command exists — `sqlite3` for the `sqlite3` command. Until the require is seen, the command is hidden from completion and its use draws the missing-import warning; after it, everything lights up. Leave unset for commands that are simply always there.

### `excluded_events` — Excluded iRules events

*command only* — iRules events the command is not valid in.

iRules only: event contexts where this command must not be used, by event name (`HTTP_REQUEST`, `CLIENT_ACCEPTED`, …). The validity check reports a use inside any listed event.

### `versioned_arg_values` — Versioned argument values

*command and subcommand* — Package-version gates for literal positional argument values.

Version gates for individual literal argument values — when one mode word appeared in (or left) a specific package release, like a persistence mode added mid-release-train. Indices count from after the command name at command level and from after the subcommand word at subcommand level. The value list itself lives under argument values; this adds the since/until per value.

### `tcllib_package` — Tcllib package

*command only* — Tcllib package providing the command, for per-document activation.

When the command comes from a tcllib module, the module name (`json`, `struct::list`). Works like the required package — the command activates for a document once the matching `package require` is seen — and also labels the command's origin in completion.

### `introduced_version` — Introduced in

*command and subcommand* — Dotted version of the owning package that introduced the command.

The version of the owning package (or of Tcl itself, for core commands) that first shipped the command — `8.5` for `dict`, `8.6` for `try`. Using the command under an older target dialect is then reported, which is how "this needs Tcl 8.6" warnings work.

The same three releases sit on everything a version can gate, not just the command: an option, a subcommand, a second-level subcommand, an invocation form, a side effect, an option conflict, and a single enumerable argument value each carry their own, edited in their own row.

### `deprecated_version` — Deprecated in

*command and subcommand* — First package version where the command still exists but should warn.

The first version of the owning package where the command still works but is discouraged. From this version on, uses draw a deprecation warning (and the replacement below, if named, is offered).

### `retired_version` — Retired in

*command and subcommand* — First package version without the command (exclusive upper bound).

The first version *without* the command — exclusive, so "retired: 9.0" means gone *in* 9.0, present in 8.6. Uses under a dialect at or past this version are reported as errors, not warnings.

### `deprecation_fix` — Deprecation quick-fix hook

*command and subcommand* — Registry-owned edit plan or contextual callback for replacing deprecated syntax.

The quick fix the editor offers on a deprecated call — typically "replace this word with the new spelling", with a safety level saying whether the replacement is semantically identical. Carried as an expression; name the replacement and whether arguments change in the issue notes.

An option row carries its own, so a renamed flag can offer the new spelling without the whole command being deprecated. A fix that is a registry callback rather than a replacement word cannot be written down here — the studio says so rather than dropping it.

### `warn_missing_import` — Warn on missing import

*command only* — Whether W120 fires when the command is used without a `package require`.

Whether using the command without its `package require` draws the missing-import warning. On by default when a required package is set; turn it off for commands an environment auto-loads — the Tk commands under `wish` are the classic case: present without any visible require.

### `is_namespace_exported` — Namespace exported

*command only* — Whether the source namespace exports the bare name.

Whether the owning namespace exports the bare name, so `namespace import` can bring it in — i.e. whether `string` alone can ever mean `::textutil::string`. Affects how unqualified uses resolve after an import.

## Arity and arguments

How many arguments the command takes and what each position means. Arity powers the wrong-number-of-arguments check; argument roles tell every tool which words are scripts, variable names, patterns, and channels — which is what makes highlighting, rename, and "unused variable" work through your command the way they work through `foreach`.

### `arity` — Arity

*command and subcommand* — Argument-count constraint, counted after the command name.

How many arguments the command accepts, counted **after** the command name — the same rule behind Tcl's own `wrong # args` error. `set varName ?value?` is min 1, max 2. A trailing `args` parameter means no maximum, so leave max unbounded. `incr x ?increment?` is 1 to 2; `list` is 0 to unbounded.

On a subcommand, count after the subcommand word instead: `string length string` is exactly 1. The step and extra-exact-count fields cover commands whose argument tail comes in pairs (`array set` style `name value` lists).

### `arity_windows` — Arity windows

*command and subcommand* — Per-release signature shapes, for a command whose argument count changed across its owning package's releases. Empty unless it did; the plain arity is the fallback whenever no window covers the resolved floor.

Per-release signature shapes, for the rare command whose argument count changed between releases of the package that owns it. Leave empty unless it did — the plain arity above already describes a signature that never changed, and it stays the fallback whenever no window covers the document's resolved floor. Windows must not overlap, so consecutive ones are written closed: retire each where the next is introduced.

### `arg_rows` — Versioned argument rows

*command and subcommand* — The authored per-argument rows the parallel argument tables above are projected from, retained so a consumer holding a resolved package floor can re-project at it. Empty unless some argument carries a release window.

The authored per-argument rows the argument tables above are projected from, kept so a document with a resolved package floor can re-project at it. Empty unless some argument carries a release window; when it is empty the tables above are the whole truth.

### `arg_roles` — Argument roles

*command and subcommand* — Static role per 0-based argument index, for fixed-layout commands.

What each argument position *is*, for commands whose layout never changes. Index 0 is the first word after the command name. `proc name args body` declares 0 = Name, 1 = ParamList, 2 = Body; `foreach varlist list body` declares 0 = LoopVarList, 2 = Body.

Roles are what make the editor light up a body argument as real code, treat `varName` arguments as variable reads and writes (so rename and "unused variable" work through them), and know a word is a pattern or a channel. See the Reference tab for every role. If the layout depends on the argument count or on option words, you need the argument-role resolver instead — say so in the issue you open, because resolvers are code.

### `arg_presentation` — Argument presentation

*command and subcommand* — Formatter layout override per 0-based argument index; body arguments are block-expanded unless declared inline.

How the formatter lays an argument out, when that differs from the default. Body arguments normally expand onto their own indented lines (the way everyone writes a `while` body); `InlineScript` keeps one on the command line instead — the way `for {set i 0} {$i < 3} {incr i} { … }` keeps its start and next scripts inline while only the body expands.

Only declare the exceptions; an empty map means "format every body as a block", which is right for almost every command.

### `command_prefixes` — Command-prefix positions

*command and subcommand* — Argument indices carrying a callback command prefix, with the arity appended to it.

Argument positions that carry a *callback* — a command prefix the command will invoke later with extra arguments appended, like `lsort -command cmp` calling `cmp a b` or `trace add variable x write cb` calling `cb name1 name2 op`. Declare the position and how many arguments get appended, and the tools can check the callback procedure actually accepts them.

A callback is a *reference* to code, not code itself — that is why it is not simply a Body role.

### `assigns_variable_at` — Assigns variable at

*command only* — Legacy shorthand: the single argument index naming a variable the command writes.

The one argument position naming a variable the command writes — 0 for `set varName value`. This is the older, simpler cousin of a VarWrite argument role; prefer declaring the role, but either works, and for a one-target command they mean the same thing.

What it buys: the written variable counts as *defined* afterwards, so "used before set" stays quiet, and rename reaches the name.

### `body_arg_implicit_args` — Body implicit arguments

*command and subcommand* — Runtime-supplied positional arguments the body's first command receives.

For callback-style bodies whose first command receives extra positional arguments supplied by the runtime when it invokes the body. Rare; leave at 0 unless the command's documentation spells such arguments out.

## Types

What kind of values flow in and out: the return type, per-argument expectations, and how written variables are typed. Everything here feeds type inference and the shimmering / wrong-type warnings. All optional — unset means "unknown", never "wrong".

### `return_type` — Return type

*command and subcommand* — The intrep of the value `[cmd …]` yields.

The kind of value `[cmd …]` produces: string, integer, double, boolean, list, dict, byte array, object handle, or channel. `llength` returns Int; `lrange` returns List; `open` returns Channel. This feeds type inference — `set n [llength $l]` makes `$n` an integer downstream — and the warnings about treating a list as a string.

Leave unset when the type varies and no single answer is honest.

### `var_write_typing` — Variable-write typing

*command and subcommand* — How the command types the variables it writes, when that differs from its return value.

What type the variables the command *writes* receive, when that is not the same as what the command *returns*. `lassign` returns the leftover list but writes list *elements*; `scan` and `regexp` return a count but write parsed pieces; `gets chan line` returns a character count but writes the line into `line`.

`ReturnValue` (the default) says the written variable holds the return value. `Fixed` names one type for the written variable regardless of the return. `Destructured` says the pieces cannot be typed statically. Getting this right avoids false "wrong type" warnings on the written variables.

### `return_elements` — Return elements

*command and subcommand* — How the result relates to container element structure.

How the result relates to its arguments *as a container* — for example, `list a b c` returns a list whose elements are exactly the arguments, and `lindex $l 0` returns an *element of* its list argument. Declaring it lets the analysis track values through list packing and unpacking. Expression-valued; when in doubt leave it unset and mention the relationship in the issue notes.

### `var_elements_effect` — Variable element effect

*command and subcommand* — How the command evolves the container elements of the variable it writes in place.

How the command reshapes the container inside the variable it writes, in place: `lappend v x y` appends list elements, `dict set v k val` sets one dict value, `lpop v` removes one. This keeps element-level tracking alive across in-place edits instead of giving up on the whole variable. Expression-valued; describe the effect in words if unsure.

### `representation_effect` — Representation effect

*command and subcommand* — Effect on Tcl's dual string/internal representation or shared-object storage.

Tcl values carry both a string form and a cached internal form (list, dict, integer …), and some commands convert or copy between them — the "shimmering" a Tcl developer knows from performance work. This field records such an effect (for example copy-on-write on a shared list) so the performance lints can see it. Rarely needed; safe to leave unset.

### `arg_types` — Argument type hints

*command and subcommand* — Expected intrep, shimmer risk, and transparent source types per argument index.

The value type each argument position expects — the input-side mirror of the return type. `lindex list index` expects a List at 0 and an index at 1; `incr` expects an integer variable. Drives "this argument will shimmer" and type-mismatch warnings.

Only declare positions where the expectation is firm; a generic "any value" position is better left out.

### `inferred_storage_type` — Inferred storage type

*command and subcommand* — The container kind the target variable is inferred to hold.

The container kind the command's target variable ends up holding: `array set` makes an array, `lappend` a list, `dict set` a dict. Downstream reads of that variable are then understood — and mixing kinds (using a dict variable as an array) is flagged.

## Subcommands

For ensemble commands (`string length`, `dict get`): one entry per operation word, each a small spec of its own. Indices inside a subcommand are counted after the subcommand word. Unique-prefix abbreviation is handled automatically — declare full names only.

### `subcommands` — Subcommands

*command only* — Ensemble subcommands, each a self-contained metadata bundle.

For ensemble commands — one word selects an operation, as in `string length`, `dict get`, `info exists`. Each subcommand is a small spec of its own: its arity, argument roles, options, return type, and documentation, all counted after the subcommand word.

Declare every subcommand the real command has, even thinly: an undeclared word gets flagged as an unknown subcommand (unless you allow unknowns below), and a declared one gets completion and its own arity check. Unique-prefix abbreviations (`string le` for `string length`) are handled for you — declare full names only.

### `allow_unknown_subcommands` — Allow unknown subcommands

*command only* — Accept a subcommand word that is not declared, without a W001 warning.

Turn this on when users can legitimately extend the command with their own subcommand words — `namespace ensemble` ensembles that scripts add to, or an object system whose methods are user-defined. It suppresses the "unknown subcommand" warning for words you did not declare, while keeping everything you *did* declare fully checked.

### `prefix_matching` — Prefix matching

*command and subcommand* — Whether this command's keyword tables accept unique-prefix abbreviations (`Tcl_GetIndexFromObj`) or only exact spellings.

Real Tcl resolves any unique prefix of a keyword — `string le` is `string length`, `lsort -uni` is `-unique`. That is the default here too. Set Strict for commands that demand exact spellings (the C API's `TCL_INDEX_STRICT` mode), where an abbreviation is an error, not a shorthand.

### `default_form_first_word` — Default-form first word

*command only* — The value shape a non-subcommand first word may take (`after 200 …`).

For commands where a first word that is *not* a subcommand selects a default behaviour instead of being an error. The registry's example is `after`: `after 200 script` — an integer first word means "delay", not an unknown subcommand. Declaring the accepted shape stops the false warning.

### `creates_instance_at` — Creates instance at

*command only* — Argument index naming an object command of this spec's own object class.

The argument position that names an object command of this spec's own class — the `Foo` in `oo::class create Foo`. After the call, `Foo` is a known command dispatching this class's methods.

### `defines_command_at` — Defines command at

*command and subcommand* — Argument index whose literal value becomes a callable command name.

The argument position whose *literal* value becomes a callable command once the call runs — the `NAME` of `coroutine NAME cmd …`, or (on the subcommand) `interp create name`. Later calls to that name stop being "unknown command". Dynamic words at the position are simply not recorded — no guessing.

### `implementation_namespace` — Implementation namespace

*command only* — Namespace the ensemble's subcommands are also individually callable under.

For ensembles whose subcommands are also reachable as plain commands in a namespace — `::tcl::string::length` behind `string length`. Naming the namespace makes both spellings resolve to the same spec.

### `sub_subcommands` — Second-level subcommands

*subcommand only* — Operations selected by the word after this subcommand (`info object <op>`), each with its own release window.

A third level of keywords — operations selected by the word *after* this subcommand, as in `info object isa`. Deliberately lighter than a full subcommand: each carries its name, a one-line detail, a synopsis, and an optional dialect gate — enough for highlighting, hover, and completion. Arity stays on the owning subcommand. An operation may also carry its own **option table**, and should whenever the operations disagree about which options exist: `namespace ensemble create` takes `-command`, `configure` takes `-namespace`, and each rejects the other's. A table here replaces the subcommand's for that operation rather than adding to it, so declare it on every operation that takes options, or leave it empty and let the subcommand's table answer.

## Documentation

What the editor shows humans: hover text, synopsis lines, and completion details. Nothing here changes a diagnostic — it is the safest group to fill in generously, and the one users see most.

### `hover` — Hover documentation

*command and subcommand* — Summary, synopsis lines, prose, source, examples, and return value.

What the editor shows when the pointer rests on the command: a one-line summary, the synopsis line(s) as the man page writes them (`lappend varName ?value …?`), a short prose description, where the command comes from, an example call, and what it returns.

Write the summary like the first line of a man page — one sentence, present tense. This is pure documentation: nothing here changes any diagnostic, so it is the easiest high-value field to fill in.

### `forms` — Invocation forms

*command only* — Synopsis per invocation form, for completion and arity-dependent lookup; each form has its own release window.

The distinct ways the command can be called, each with its own synopsis and argument count — most usefully a read form and a write form: `$w cget -opt` versus `$w configure -opt value`, or `testConstraint NAME` (getter) versus `testConstraint NAME value` (setter). The right form is picked by argument count, so each can carry its own purity and effects: a getter is harmless where its setter is not.

A form can also come and go with the package: each row carries the same three releases the command does, so a form added in 1.4 and dropped in 2.0 says so on its own row.

### `detail` — Detail

*subcommand only* — Short description for the completion list.

A few words for the completion list — what shows next to the subcommand name in the picker. `string length` says "the number of characters". Keep it under a dozen words; the hover carries the long version.

### `synopsis` — Synopsis

*subcommand only* — Invocation synopsis.

The usage line for this subcommand as a man page would write it: `string length string`, `dict get dictionary ?key …?`. Shown in completion and hover, and worth writing even when nothing else is filled in.

## Options and values

The command's `-flag` switches and the literal values specific argument positions accept. Declared options get completion, spelling checks, and flag-versus-value highlighting; declared values get completion, and can be closed into an "only these" set.

### `closed_value_args` — Closed value arguments

*command and subcommand* — Argument indices whose declared values are an exhaustive legal set (W127).

Argument positions whose legal values are *exactly* the ones declared under argument values — a closed set, like an enum. A value outside the set is then a diagnostic, not just a missing completion. Only close a position when the real command genuinely accepts nothing else.

### `options` — Options

*command and subcommand* — Declared option flags, their values, roles, and dialect gates.

The command's `-flag` switches, each with whether it takes a value (`-nocase` takes none; `-index i` takes one), what role and type the value has, which dialects have the flag, and a one-line description for completion. Declare `--` here too if the command accepts it as an end-of-options marker — that is what enables the "put `--` before a dynamic value" safety warning.

Declared options get completion, spelling checks, and correct highlighting of flag-versus-value; undeclared ones are reported as unknown.

### `option_constraints` — Option constraints

*command and subcommand* — Registry-declared sets of leading options that may not occur together.

Pairs or sets of options that must not appear together in one call — mutually exclusive modes like `-glob` and `-regexp`. The checker reports a call using both, with no code written for the specific command.

### `reserved_trailing_words` — Reserved trailing words

*command only* — Trailing words C Tcl's own option scan never treats as option candidates.

How many words at the *end* of the call are never option candidates, matching how C Tcl scans options only up to a point. `lsearch ?options? list pattern` reserves the final 2: a pattern that happens to start with `-` is data there, not a flag.

### `arg_values` — Argument values

*command and subcommand* — Enumerable positional values per argument index, for completion; each carries its own Tcl floor and release window.

The completable values for specific argument positions — the mode words of `binary scan`, the event names for an iRules command, the subcommand-like keywords of a mode argument. Purely additive for completion and hover unless the position is also listed under closed value arguments, which upgrades it to "only these".

### `pattern_type` — Pattern type

*command and subcommand* — The pattern language the command's pattern argument uses.

Which pattern language the command's Pattern argument speaks: glob (`string match`, `lsearch` default) or regular expression (`regexp`, `regsub`, `lsearch -regexp`). The pattern is then checked and highlighted in the right language — a `*` means something very different in each.

### `pattern_arg_resolver` — Pattern-argument resolver

*command only* — Native hook selecting pattern positions and languages for a concrete call.

A native hook that selects the Pattern argument positions and language for this particular call. Use it when options change the pattern grammar, such as `lsearch -regexp`; the Studio preserves the need for the hook but cannot recover a Rust function pointer from a loaded spec, so supply the expression.

### `format_string_type` — Format-string type

*command and subcommand* — The format-string language the command's format argument uses.

Which template mini-language the command's format argument uses: printf-style (`format`/`scan`), `clock format` fields, `binary` format/scan cursors, or `regsub` replacement backreferences. The template is then validated in the right language — `%b` is a fine clock field but means binary in printf.

### `min_abbrev` — Minimum abbreviation

*subcommand only* — Documented minimum abbreviation length for this subcommand's name, when longer than uniqueness alone requires. Unset = uniqueness only.

Unique-prefix abbreviation is computed automatically; this field is only for the rare subcommand whose *documented* minimum abbreviation is longer than uniqueness requires. Leave unset almost always.

### `arg_values_accept_prefix` — Values accept a prefix

*subcommand only* — Whether a closed value is accepted as a unique prefix rather than an exact match.

Whether this subcommand's closed argument values accept unique prefixes the way keyword tables do — `persist add u` for `uie`. Off means exact spellings only.

### `max_leading_option_words` — Max leading option words

*subcommand only* — Cap on leading option words consumed; further option-shaped words are positional.

A cap on how many leading words the option scan will consume for this subcommand; anything past the cap is positional even if it starts with `-`. Matches commands whose C implementation stops looking for options after a fixed count.

## Behaviour

The command's behavioural traits — the facts every analysis reads instead of special-casing command names: does it evaluate code, alter control flow, run a loop body, mutate state, act as a language keyword? The Reference tab lists every trait with its meaning.

### `traits` — Traits

*command and subcommand* — Behavioural trait flags every consumer reads instead of naming the command.

Behavioural facts about the command, as a set of flags. Traits are how every analysis learns what a command *does* without anyone writing code: tick `EVALUATES_CODE` and body arguments are analysed as scripts; tick `CONTROL_FLOW` and unreachable-code checks understand it; tick `PURE` and the optimiser may fold it. If you only fill in one thing beyond arity, make it the traits — most diagnostics key off them.

Open the Reference tab for the full list with a one-line meaning each. When no trait fits, leave them all off — an empty set is always safe, it just tells the tools less.

### `unsafe_command` — Unsafe command

*command only* — Allows context escalation in sandboxed dialects — drives IRULE2003.

Marks a command that escapes the sandbox in restricted dialects — in iRules, things that reach the underlying system. Drives the "unsafe command" security diagnostic there. Not related to Tcl's safe interpreters (that is the `SAFE_INTERP_HIDDEN` trait).

### `body_kind` — Body kind

*command and subcommand* — Whether body arguments run in the caller's frame or a separate context.

Whether a Body argument runs *in the caller's frame*, seeing and changing the caller's variables (`while`, `if`, `catch` — "Plain"), or in a separate context of its own (`proc` bodies, class definition bodies — "Structural"). Plain bodies join the surrounding data flow: a `set` inside them changes the enclosing scope. Structural bodies deliberately do not.

### `byte_array_effect` — Byte-array effect

*command and subcommand* — How the command transforms a byte-array operand it derives its result from.

What happens when the command's operand is binary data (a byte array): passed through intact, silently coerced to a string (corrupting it), case-folded, re-encoded, or re-binarified. This powers the "binary data corrupted by string operation" check — Tcl's classic gotcha where `string tolower` quietly destroys bytes.

### `pure` — Pure

*subcommand only* — Side-effect free.

Side-effect free: the subcommand changes nothing — no variables, no I/O, no interpreter state. `string length` is pure; `lappend` is not. Purity feeds the optimiser and lets "result unused" warnings fire (a pure call whose result is discarded does nothing at all).

### `mutator` — Mutator

*subcommand only* — Mutates state.

The opposite declaration: this subcommand changes state — a variable, a table, the interpreter. `dict set` and `array unset` are mutators. A subcommand can be neither (unknown), but never both.

### `loop_list_header` — Loop-list header

*subcommand only* — A CFG header whose arguments are list expressions (`foreach` / `lmap`).

Marks the subcommand a loop header whose arguments include list expressions evaluated once before the body iterates — the `dict for` shape. Feeds loop analysis; leave off for anything that is not a loop.

### `creates_scope_alias` — Creates scope alias

*subcommand only* — Creates an upvar-like binding.

Marks the subcommand as creating an `upvar`-style alias: after it, one name is another variable in disguise (`namespace upvar` does this). Writes through the alias then count as writes to the real variable.

### `destructive` — Destructive

*subcommand only* — An irreversible operation (`file delete`).

An irreversible operation — `file delete`, a table purge. Feeds the "destructive operation" cautions and keeps such subcommands out of casually suggested quick fixes.

### `returns_path` — Returns a path

*subcommand only* — Returns a filesystem path.

The result is a filesystem path (`file join`, `file dirname`). Path-aware checks then follow the value — e.g. the path-taint colours that prove a user-influenced path stays inside a known root.

### `is_unescape` — Unescapes

*subcommand only* — Performs unescaping or decoding.

The subcommand *decodes* — URL-decoding, HTML-unescaping. In taint terms it undoes sanitisation: a value that was safe because it was encoded is dangerous again after this returns.

## Side effects

What state the command touches — variables, channels, files, network, logs, HTTP state — and how stable its result is across calls. This is what dead-code, ordering, and result-reuse reasoning stand on. Declaring nothing is safe but blinds those checks to your command.

### `completion` — Completion contract

*command and subcommand* — Possible Tcl completion codes and result/options payload obligations.

Which of Tcl's completion codes the command can finish with — normal return, `error`, `break`, `continue`, `return` — and what it promises about the result. `error` always raises; `break` only makes sense in a loop. This powers checks like "this `break` is outside any loop" and dead-code reasoning after a command that always raises.

### `command_table_effect` — Command-table effect

*command and subcommand* — How the command mutates the interpreter's command table.

Whether the command changes which commands *exist*: `proc` defines one, `rename` moves or deletes one, `interp alias` creates one under another name. Declaring it keeps "unknown command" honest after the call — a name created by your command stops being reported as undefined.

### `side_effects` — Side effects

*command and subcommand* — Structured declarations of the state the command reads and writes, each with its own release window.

What state the command touches, as structured reads and writes: variables, channels, files, the network, logs, HTTP headers, session tables, and so on — each with whether it reads, writes, or both, and (for iRules) which connection side. `puts` writes channel I/O; `file delete` writes filesystem state; `HTTP::header insert` writes HTTP headers on the current side.

This is the backbone of dead-code and ordering analysis: a command with no declared effects and no result being used looks removable. When a command does anything externally visible, say so here.

### `world_effects` — World effects

*command and subcommand* — Target-neutral mutable-world footprint for common compiler analysis.

A compiler-oriented summary of the same idea as side effects: which broad domains of the running interpreter's world (variables, commands, namespaces, traces, channels …) the command reads, writes, or can call back into. Used by the optimiser to decide what survives across the call. Expression-valued; leave unset and the optimiser stays conservative.

### `state_transitions` — State transitions

*command and subcommand* — Target-neutral command, namespace, interpreter, trace, and alias transitions.

Declares precise identity changes the command performs on the Tcl world — a command coming into being, a namespace appearing, a trace being attached, a variable cell changing identity. Finer-grained than world effects; used by the most exacting optimiser proofs. Leave unset unless a maintainer asks for it.

### `dispatch_dependencies` — Dispatch dependencies

*command and subcommand* — Mutable Tcl domains that must remain stable before specialisation.

What must stay *unchanged* for the registry's knowledge about this command to remain trustworthy at a call site — e.g. that nobody renamed or shadowed the command in between. Compiler-proof machinery; leave unset.

### `result_stability` — Result stability

*command and subcommand* — Whether repeated calls return the same value, or depend on mutable or volatile state.

Whether calling the command twice with the same arguments yields the same value. `string length` always does; `clock seconds` never does; `info commands` depends on what has been defined in the meantime. Purity says "no side effects"; this says "same answer again" — a command can be pure yet unstable (`clock seconds` changes nothing but never repeats). The optimiser only reuses results it can prove stable.

## Compiler hooks

Named entry points into the compiler for commands that need special-cased lowering, bytecode, or analysis. Core Tcl commands use these; a third-party command spec almost never should — prefer expressing behaviour through roles, traits, and effects, which need no code.

### `semantic_operation` — Semantic operation

*command and subcommand* — Target-neutral operation identity selected before backend dispatch.

Names the abstract operation the command performs ("list length", "dict get") so the compiler backends can share one implementation across spellings. Only meaningful for commands the compiler executes; user packages leave it unset.

### `lowering_hook` — Lowering hook

*command and subcommand* — Per-command lowering specialisation in the compiler's dispatch table.

Compiler internals: picks a specialised translation of this command into the compiler's intermediate form (`if`, `foreach`, and friends have one). User packages leave this unset — the generic path handles any command.

### `codegen_hook` — Bytecode codegen hook

*command and subcommand* — Per-command TclVM bytecode emitter. Unset uses the generic invoke emitter.

Compiler internals: a specialised bytecode emitter for the Tcl VM, mirroring the commands C Tcl byte-compiles specially. Leave unset; the generic "invoke the command" path is always correct.

### `inline_codegen_hook` — Inline codegen hook

*command and subcommand* — Emitter for the value-position and catch-body paths.

Compiler internals: the bytecode emitter used when the command sits in value position (`set x [llength $l]`) or in a catch body. Leave unset for user packages.

### `bpf_op` — BPF-Tcl lowering descriptor

*command only* — Typed BPF-Tcl lowering descriptor; the BPF front-end dispatches on this, never on the command name.

Only for the BPF-Tcl dialect: how this command lowers to a BPF operation. Anything outside that dialect leaves it unset.

### `analyser_hook` — Analyser hook

*command and subcommand* — Per-command handler family in the analyser's central dispatch.

Compiler internals: routes the command to a hand-written analyser family (`proc`, `foreach`, `package require`, …) for behaviour the declarative fields cannot express. The goal of this whole form is to make these unnecessary — fill in roles, traits, and effects first, and reach for a hook only when something still cannot be said.

### `literal_argument_validator` — Literal argument validator

*command and subcommand* — Registry callback for relationships and member sets within statically-known arguments.

A hook validating relationships *between* literal arguments that a per-position value list cannot express — "this mode word is only legal when that flag is present". Code, carried by reference; spell the rule out in the issue notes.

### `cfg_rewrite_name` — CFG rewrite name

*subcommand only* — Lowered command name for an ensemble subcommand the lowering pass rewrites.

Compiler internals: the plain command name this ensemble subcommand is rewritten to during lowering. Leave unset for user packages.

## Taint and security

How attacker-influenced data flows through the command: whether it is a source (returns untrusted data), a sink (a dangerous place for untrusted data to arrive), or a sanitiser (adds a safety colour as data passes through). The colours are listed on the Reference tab. Only security-relevant commands need anything here.

### `taint_output_sink` — Output-sink code

*command and subcommand* — Diagnostic code emitted when tainted data reaches the output position (`T101`).

Marks the command as a place where attacker-influenced data becomes *output* — echoed into a page or response — and names the diagnostic to raise when tainted data reaches it (cross-site-scripting style). The value is the diagnostic code; leave unset for commands that are not output sinks. See the Reference tab's taint-colour section for how taint is tracked.

### `taint_output_sink_subcommands` — Output-sink subcommands

*command only* — Restricts the output sink to these subcommands. Empty means every invocation.

Restricts the output sink to specific subcommands — `respond`-like operations — so the rest of the ensemble stays clean. Empty means the sink applies to every invocation of the command.

### `taint_log_sink` — Log-sink code

*command only* — Log-injection sink diagnostic code (`IRULE3003`).

Like the output sink, but for log writes: tainted data reaching a log line is a log-injection finding (forged entries via embedded newlines). The value is the diagnostic code to raise.

### `taint_network_sink_args` — Network-sink arguments

*command only* — Argument indices taking a network address — SSRF sinks.

Argument positions that take a network destination (host, URL). Tainted data reaching one is a server-side request forgery finding — an attacker steering *where* the script connects.

### `taint_code_sink_args` — Code-sink arguments

*command only* — The specific slots where a tainted value reaches eval-style evaluation.

Argument positions where a value is evaluated as code. Tainted data reaching one is the classic injection: `eval $userInput`. Declaring the precise slots keeps the finding accurate on commands where only some arguments are executed.

### `taint_interp_eval_subcommands` — Cross-interpreter eval subcommands

*command only* — Subcommands evaluating code in another interpreter (T105).

Subcommands that evaluate code in *another* interpreter (`interp eval` style). Tainted data reaching them raises the cross-interpreter evaluation finding.

### `taint_source` — Taint source colours

*command only* — Colour bits the return value carries when the command is a taint source.

Declares the command's *result* as attacker-influenced — the way `HTTP::header` or a socket read hands you data the client controls. The colours say what is known about the value beyond "tainted"; usually just `TAINTED`. Everything derived from a tainted value stays tainted until a sanitiser cleans it, and sinks report when raw taint reaches them.

### `taint_transform` — Taint transform colours

*command and subcommand* — Colour bits the command adds to a tainted value it returns.

Declares the command a *sanitiser* or encoder: the colours it adds to a value passing through. An HTML-escaper adds `HTML_ESCAPED`; `file join` adds path colours; a validator that proves "this is an IP address" adds `IP_ADDRESS`. A sink that requires a given colour then accepts the cleaned value — this is how "escaped before output" is recognised.

### `taint_double_encode_colour` — Double-encode colour

*command and subcommand* — Input colour whose presence means this command would double-encode (T106).

The colour that means the input is *already* encoded the way this command encodes. Feeding an HTML-escaped value through the HTML escaper again produces `&amp;amp;` — declaring the colour lets the double-encoding check catch exactly that.

### `taint_sink_safe_colour` — Sink-safe colour

*command only* — Colour that suppresses the dangerous-sink warning for this sink.

For a command that is a sink: the colour(s) that make a tainted value acceptable here. An output sink might accept `HTML_ESCAPED`; an exec sink might accept `SHELL_ATOM`. A tainted value carrying the required colour passes without a finding.

### `credential_options` — Credential options

*command only* — Option flags whose value carries a secret (W310).

Option flags whose value is a secret — `-password`, `-token`. A literal secret passed to one is reported as a hard-coded credential, and the value is treated as sensitive by anything that echoes code.

### `sensitive_headers` — Sensitive headers

*command and subcommand* — HTTP header names whose values are secrets.

HTTP header names whose values are secrets (`Authorization`, `Cookie`). Reads of these through this command are treated as sensitive data for the credential-handling checks.

### `setter_constraints` — Setter constraints

*command only* — Required argument prefixes on setter forms (IRULE3101).

iRules hardening: setter forms that must be called with a given literal argument prefix to be safe — the pattern behind "this header must be set with an explicit name, not a variable". Drives its own diagnostic; rarely needed outside the F5 command packs.

### `credential_arg` — Credential argument

*subcommand only* — Credential-value index, counted WITH the subcommand word at 0 (unlike every other subcommand index field) — `HTTP::header insert name value` declares 2.

The argument position whose value is a secret — a password or key handed to this specific subcommand. A literal there is a hard-coded credential finding.

Coordinate warning: unlike every other subcommand index field, the consumer counts the subcommand word itself as 0 — `HTTP::header insert name value` declares 2 for the value slot. Store the index verbatim; never re-base it.

## Deprecation and translation

The replacement story for ageing commands — what to use instead and whether the switch is a drop-in — plus the F5 XC translation mapping.

### `xc_translatable` — XC translatable

*command only* — Cross-compile translatability override. Unset uses the default rules.

F5 only: whether the iRules-to-XC translator can carry this command across. Unset follows the default rules; set it only to override them in either direction.

### `xc_operation` — XC operation

*command and subcommand* — The XC operation the command maps to when translatable.

F5 only: the XC-side operation this command (or subcommand) maps to when translated.

### `deprecated_replacement` — Deprecated replacement

*command only* — Replacement command name surfaced by the deprecation code action.

The command to use instead, shown in the deprecation warning and offered by the quick fix — the `lmap` to your deprecated mapping helper.

### `deprecated_replacement_drop_in` — Replacement is drop-in

*command only* — Whether the replacement accepts the deprecated argument list unchanged.

Whether the replacement accepts the *same argument list* unchanged — if yes, the quick fix can rewrite calls automatically; if no, it only points at the replacement and leaves the arguments to the author.

## Advanced

Fields the studio carries as raw Rust expressions: function pointers and references to shared, named descriptors. You can see that a loaded command sets one, but not edit it structurally. When your command needs one, describe the behaviour in plain words in the issue notes — that description is exactly what a maintainer needs to write the few lines of Rust.

### `arg_role_resolver` — Argument-role resolver

*command and subcommand* — Callback assigning roles from the actual argument list; wins over `arg_roles`.

A hook for commands whose argument layout cannot be a fixed table — `if` (any number of `elseif` clauses), `switch` (options before the value), `set` (arg 0 is written with two words but only read with one). The hook inspects the actual call and assigns roles.

This is Rust code, so the studio can only carry a reference to it. If your command needs one, describe the layout rule in plain words in the issue notes — for example "the last argument is always the body; everything before it comes in `name value` pairs" — and a maintainer writes the few lines.

### `repeated_args` — Repeated argument layouts

*command and subcommand* — Roles that recur at a fixed stride over the argument tail (`global a b c`, `variable n v n v`).

For argument tails that repeat a pattern without limit: `global a b c` (a variable name at every word), `variable n1 v1 n2 v2` (a name at every *other* word), `foreach v1 $l1 v2 $l2 {…}` (a pair pattern with the body excluded from the tail). Declared as a start index, a stride (1 = every word, 2 = every other), and how many trailing words to leave out.

This is what lets rename and highlighting reach *every* name in `global a b c`, not just the first.

### `frame_effect` — Frame effect

*command only* — How the command crosses stack frames: level word, frame-selected variable args, and caller-frame scripts.

For commands that reach into another stack frame the way `upvar`, `uplevel`, and `namespace upvar` do: which word is the `?level?`, which arguments are variables in the *other* frame, and which scripts run in the caller's frame. This is a named descriptor a maintainer attaches; describe the frame behaviour in your issue notes if your command has one (most do not).

### `clause_shape_check` — Clause-shape checker

*command only* — Validator for a clause chain whose shapes are not a single min..=max range.

A validator for commands whose legal shapes cannot be captured by a single min–max argument count — `if`'s `elseif`/`else` chain is the canonical case: any length is fine, but only in the right rhythm. This is code, so in the studio it is a reference; if your command has a clause grammar, write the rhythm out in the issue notes ("`cond body` pairs, optionally ending `else body`").

### `command_prefix_resolver` — Command-prefix resolver

*command and subcommand* — Callback locating command-prefix positions that depend on the argument list.

The dynamic sibling of the command-prefix positions: a hook for when *which* word is the callback depends on the rest of the call (after options, say). Code, carried by reference — describe the rule in the issue notes.

### `command_forms` — Structured command forms

*command only* — Per-form arity, roles, options, and hooks, for form-specific routing.

A structured, per-form bundle of arity, roles, options, and hooks for commands whose forms differ more deeply than a synopsis line can say. Expression-valued and rarely needed — plain `forms` covers the common getter/setter split.

### `const_fold` — Constant folder

*command and subcommand* — Compile-time folder returning the command's constant result.

A compile-time evaluator: when every argument is a literal, compute the result now — `string length abc` is always 3. This is code, carried by reference. If your command is a pure function of its arguments, saying so in the issue notes (plus the `PURE` trait) is what a maintainer needs.

### `const_fold_versioned` — Versioned constant folder

*command and subcommand* — Tcl-version-aware folder; takes priority over the plain folder.

The same as the constant folder, for commands whose literal result depends on the Tcl version being targeted (behaviour that changed between 8.x and 9.x). Takes priority over the plain folder when both are set.

### `event_requires` — Event requirements

*command only* — Layer-based iRules event requirements used by the IRULE1001 validity check.

iRules only: what the surrounding event context must provide for this command to work — transport layer, profile, connection side. Feeds the "command not valid in this event" check. Named descriptor; describe the requirement in words in the issue notes.

### `event_requirement_forms` — Event requirement forms

*command only* — Argument-prefix-specific iRules event contracts that override the command-level requirements.

iRules only: overrides of the event requirements for specific argument spellings — when `CMD mode-a` is valid in different events than `CMD mode-b`. Named descriptor, like the event requirements themselves.

### `data_collection` — Data collection

*command only* — Registry descriptor for a collect, release, or payload operation.

iRules only: for the `collect`/`release`/payload family — which protocol, which action, when payload data is available, and how release behaves. Drives the collect/release pairing diagnostics and their quick fixes.

### `side_switch_target` — Side-switch target

*command only* — Connection-side context selected while evaluating this command's body.

iRules only: for commands whose body runs in the *other* side's context (`clientside { … }` / `serverside { … }`) — which side the body switches to. Side-sensitive commands inside the body are then checked against the right side.

### `event_handler_priority` — Event-handler priority

*command only* — Runtime default and implicit-priority policy for an event-handler command.

iRules only: for event-handler commands like `when` — the runtime's default priority (BIG-IP uses 500) and whether omitting an explicit priority is worth reporting.

### `irules_top_level_effect` — iRules top-level effect

*command only* — Stateful effect of an iRules declaration, such as the priority inherited by later handlers.

iRules only: declares a file-level command whose effect persists for later declarations. `priority N`, for example, changes the inherited priority of following `when` handlers until another priority declaration replaces it.

### `taint_sink_gate` — Taint-sink gate

*command only* — Predicate over the call's own flags deciding whether the sink applies.

A predicate deciding whether the sink applies to *this particular call*, based on the call's own flags — `subst -novariables` is a different risk than bare `subst`. Code, carried by reference; state the condition in the issue notes.

### `byte_array_payload` — Byte-array payload

*command only* — `<proto>::payload` layout for the S110 byte-array-corruption check.

F5 only: describes a `<proto>::payload`-style command's layout so the binary-data corruption check (string operations applied to raw payload bytes) knows where the bytes flow.

### `definition_body` — Definition-body grammar

*command only* — Body grammar for a class or type definer, so the generic walker can recurse.

For commands that *define a class or type* with a body of member declarations — `oo::class create`, `snit::type`, `itcl::class`. The grammar lists the member keywords (`method`, `constructor`, `variable`, …) and which words of each are the name, the parameter list, and the body, so navigation, folding, and highlighting work inside the class body with no code written.

Grammars are shared, named descriptors: if your package has its own definer, the studio cannot author the grammar inline — describe the member keywords and their shapes in the issue notes.

### `manufacturer_methods` — Manufacturer methods

*command only* — Class-command methods that create an instance, including their name, body, and constructor-argument positions.

For class-like commands: which methods manufacture an instance (`new`, `create`), which argument (if any) names the instance command being created, and where constructor arguments start. This is how `oo::class create Foo` makes `Foo` a known command, and `set o [Foo new]` makes `$o` a known object.

### `case_list` — Case list

*command only* — The `{pattern body …}` clause list the command takes as its final braced word.

For commands taking a final braced `{pattern body pattern body …}` clause list — `switch`'s second form. The descriptor says how the pairs read, so each body is analysed as a script and each pattern in the right pattern language.

### `oo_context_facts` — TclOO context facts

*command only* — Keyword words whose value the enclosing TclOO method frame fixes, so the optimiser can fold them.

TclOO fine print: keyword words whose value is fixed by the enclosing method frame (`self`, the defining class), letting the optimiser fold them. Leave unset outside the TclOO core.

### `self_receiver_words` — TclOO self-receiver words

*command only* — Argument-0 closed words for which a bracketed `[cmd ?word?]` dispatch head denotes the current TclOO receiving object, same target as `my` (e.g. `self`'s `object`).

TclOO fine print: for introspection commands where one specific word's result is the current object itself — `[self] m` dispatching like `my m`. Lists the argument words for which that holds (`self`'s `object`).

### `object_class` — Object class

*command only* — Class metadata for a factory whose `new`/`create` returns a dispatchable handle.

Attaches class metadata to a factory command: the methods its instances answer to, superclasses for inherited resolution, and whether unknown methods are acceptable. With it, `$obj method args` gets method completion, arity checks, and option highlighting — the full treatment a built-in ensemble gets.

Plain data all the way down — the instance methods are ordinary subcommands — so a pack can author the whole thing: `object_class NAME ?-superclass {…}? ?-allow-unknown? { method NAME { … } }`, where each `method` body is the `subcommand` body grammar unchanged. The class NAME is not always the command name: a factory may manufacture a differently-named class.

### `defines_symbol` — Defines symbol

*command only* — Descriptor for an argument binding a name the document outline should list.

Marks a command that *names* something worth listing in the document outline — `tcltest::test` names a test case, `tcltest::testConstraint` a constraint. Says which argument is the name, which (if any) is a description, and the outline category. The named things then appear in outline and workspace-symbol search.

### `body_scope` — Body scope

*command only* — Extra commands available only inside the command's body argument.

Extra commands that exist only *inside* this command's body argument — a mini-vocabulary like snit's `install` inside a type body, or a report-writing DSL's directives. Keeps those words resolving inside the body without leaking them into the global namespace.

### `binds_handle` — Binds handle

*command only* — Which argument becomes an object handle, and which says its class.

Declares that a call makes a *variable* hold an object handle, and which word says the handle's class — the `set axis [::verticalAxis $win.a]` and `install axis using ::verticalAxis …` shapes. With it, the variable's later `$axis method …` calls resolve against the right class.

### `context_gate` — Context gate

*command only* — Validity gate keyed on lexical or dispatch context rather than argument shape.

A validity rule keyed on *where* the call sits rather than what its arguments are — `return -code` spellings only valid inside a procedure, iRules commands only valid at the top level of an event. Code, carried by reference; describe the context rule in the issue notes.

### `subcommand_forms` — Structured subcommand forms

*subcommand only* — Per-form arity, roles, options, and hooks matched after the subcommand word.

Structured per-form routing for this subcommand — the subcommand-level twin of the command's structured forms. Expression-valued and rarely needed.

## Vocabularies

The closed value sets the fields above draw from. The studio's Reference tab searches the same lists.

### Analyser hooks

Compiler internals: hand-written analyser families for commands whose behaviour the declarative fields cannot fully express (`proc`, `upvar`, `package require`, …). The declarative fields should always be tried first.

| Value | Meaning |
|---|---|
| `Set` | set |
| `Variable` | variable |
| `Global` | global |
| `Proc` | proc |
| `OptProc` | argparse-style proc |
| `Apply` | apply |
| `Uplevel` | uplevel |
| `NamespaceEval` | namespace eval |
| `NamespaceEnsemble` | namespace ensemble |
| `NamespaceImport` | namespace import |
| `NamespaceExport` | namespace export |
| `NamespaceForget` | namespace forget |
| `NamespacePath` | namespace path |
| `NamespaceUnknown` | namespace unknown |
| `NamespaceUpvar` | namespace upvar |
| `Foreach` | foreach |
| `For` | for |
| `Switch` | switch |
| `Catch` | catch |
| `Try` | try |
| `Upvar` | upvar |
| `DictFor` | dict for |
| `DictUpdate` | dict update |
| `DictWith` | dict with |
| `InterpAlias` | interp alias |
| `InterpEval` | interp eval |
| `InterpCreate` | interp create |
| `InterpDelete` | interp delete |
| `InterpHide` | interp hide |
| `InterpExpose` | interp expose |
| `Rename` | rename |
| `OoDefine` | oo::define |
| `OoObjdefine` | oo::objdefine |
| `PackageRequire` | package require |
| `PackageProvide` | package provide |
| `PackageIfneeded` | package ifneeded |
| `PackagePrefer` | package prefer |
| `Source` | source |
| `Append` | append |
| `Lappend` | lappend |
| `RegexPatternCapture` | regexp capture binding |
| `Incr` | incr |
| `Load` | load |

### Appended arity

For callback command prefixes: how many arguments the command appends when it invokes the callback — exactly N, at least N, or unknown. The callback checker verifies the target procedure accepts them.

| Value | Meaning |
|---|---|
| `Exactly` | exactly N arguments are appended |
| `AtLeast` | at least N arguments are appended |
| `Unknown` | indeterminate — no arity check |

### Argument presentation

Formatter layout preferences for body arguments: expanded onto indented lines (the default), or kept inline on the command's own line the way `for`'s start and next scripts are.

| Value | Meaning |
|---|---|
| `BlockScript` | expanded onto its own indented lines (the default for a body argument) |
| `InlineScript` | kept on the command's own line — `for`'s start / next scripts |

### Argument roles

What an argument position *is*. Roles are how the tools know `while`'s second word is a script, `set`'s first word names a variable, and `regexp`'s first word is a pattern — for your command exactly as for the built-ins. Body and Expr positions are analysed as code; VarWrite / VarRead positions join variable tracking (rename, unused, read-before-set); the rest refine highlighting, completion, and checks.

| Value | Meaning |
|---|---|
| `Body` | Tcl script body, recursed into by the analyser |
| `Expr` | expr sub-language expression |
| `VarWrite` | names a variable the command writes |
| `VarRead` | names a variable the command reads |
| `LoopVarList` | loop variable list (foreach / lmap) |
| `ParamList` | procedure parameter list |
| `Name` | symbolic name (proc, namespace, class) |
| `Pattern` | glob or regex pattern |
| `Option` | option flag word |
| `Value` | generic value — the default |
| `Subcommand` | ensemble subcommand keyword |
| `OptionTerminator` | the `--` end-of-options marker |
| `FormatString` | format template (format / puts style) |
| `ScanFormat` | scan conversion template |
| `Channel` | I/O channel handle |
| `Index` | list or string index expression |
| `Keyword` | fixed keyword word (`in`, `from`, `to`) |
| `CommandPrefix` | callback command prefix the command appends to |
| `CommandName` | names a command that must exist |
| `CommandNameProbe` | names a command that need not exist yet |
| `LambdaLiteral` | an `apply`-style lambda literal |
| `NamespaceName` | names a namespace (`namespace children ::ns`) |
| `Boolean` | consumed as a boolean (`Tcl_GetBoolean` spellings) |
| `NumericOrBoolean` | consumed as a number or a boolean (`-validate 0`/`yes`) |
| `Result` | becomes the command's own result (`return $w`) |

### Body kinds

Whether a script body runs in the caller's frame — seeing and changing the caller's variables, like `if` and `while` bodies — or in a separate definition context of its own, like a `proc` body. The first joins the surrounding data flow; the second deliberately does not.

| Value | Meaning |
|---|---|
| `Plain` | runs in the caller's frame — joins the enclosing data flow |
| `Structural` | runs in a separate definition / dispatch context — opts out of the enclosing data flow |

### Byte-array effects

What a command does to binary data: pass it through, silently coerce it to a string (Tcl's classic binary-corruption gotcha), case-fold it, re-encode it, or restore a byte-array representation. Drives the binary-data corruption check.

| Value | Meaning |
|---|---|
| `None` | no byte-array relationship |
| `Transparent` | passes bytes through unchanged |
| `Coerces` | forces the operand to a string, losing the byte array |
| `CaseFolds` | case-folds, which is lossy for binary data |
| `Encodes` | re-encodes the operand into a text representation |
| `Rebinarifies { value_arg: 0 }` | reinstalls a byte-array rep on the operand at the given index |

### Codegen hooks

Compiler internals: the named per-command bytecode emitters, mirroring what C Tcl byte-compiles specially. Third-party commands leave this unset.

| Value | Meaning |
|---|---|
| `Lassign` | lassign |
| `Llength` | llength |
| `Lrange` | lrange |
| `Linsert` | linsert |
| `Lset` | lset |
| `Dict` | dict |
| `Array` | array |
| `Namespace` | namespace |
| `Append` | append |
| `Lappend` | lappend |
| `Unset` | unset |
| `Tailcall` | tailcall |
| `Concat` | concat |
| `Global` | global |
| `Upvar` | upvar |

### Command-table effects

Ways a command changes which commands exist: defining a procedure, renaming or deleting one, or creating an alias. Keeps "unknown command" honest after such calls.

| Value | Meaning |
|---|---|
| `DefinesProcedure` | defines a new procedure (`proc`) |
| `RenamesCommands` | moves or deletes a command (`rename`) |
| `CreatesAliases` | creates an alias (`interp alias`) |

### Connection sides

For iRules effects: which side of the proxied connection an effect touches — client, server, both, or connection-independent. Non-iRules commands use None.

| Value | Meaning |
|---|---|
| `None` | not iRules, or side-neutral |
| `Client` | client side |
| `Server` | server side |
| `Both` | both client and server sides |
| `Global` | connection-independent |

### Default-form first words

The value shapes a non-subcommand first word may take to select a command's default form — `after 200 …`, where an integer first word means a delay rather than an unknown subcommand.

| Value | Meaning |
|---|---|
| `Integer` | an integer first word selects the default form |

### Defined-symbol kinds

The outline categories a symbol-defining command can bind: test cases, test constraints, result matchers, and iRules event handlers. Symbols land in the document outline and workspace search.

| Value | Meaning |
|---|---|
| `Test` | a test case (`tcltest::test NAME …`) |
| `Constraint` | a named test constraint (`tcltest::testConstraint NAME`) |
| `Matcher` | a custom result matcher (`tcltest::customMatch MODE command`) |
| `Event` | an event handler named for its event (iRules `when EVENT { … }`) |

### Dialects

The Tcl worlds a spec can be scoped to: every core release the catalogue carries, alongside the tool dialects — the F5 surfaces, Tk, Expect, BPF, and the SpecTcl DSL itself. The list below is the whole vocabulary, labelled as the dialect catalogue labels it. A command's dialect set decides where it resolves; unset means everywhere.

The EDA shells are not on it: a vendor shell is a base Tcl release plus package-gated command libraries, so an EDA command is scoped by its `required_package`, not by a dialect of its own.

| Value | Meaning |
|---|---|
| `tcl8.4` | Tcl 8.4 |
| `tcl8.5` | Tcl 8.5 |
| `tcl8.6` | Tcl 8.6 |
| `tcl9.0` | Tcl 9.0 |
| `tcl9.1` | Tcl 9.1 |
| `f5-irules` | F5 iRules |
| `f5-iapps` | F5 iApps |
| `tk` | Tk |
| `expect` | Expect |
| `bpf` | BPF |
| `f5-tmsh` | F5 tmsh Scripts |
| `f5-bigip` | F5 BIG-IP |
| `spectcl` | SpecTcl |

### Form kinds

Classifies an invocation form: the default, a read-only getter, or a modifying setter. Getter and setter forms of one command can differ in arity, purity, and effects.

| Value | Meaning |
|---|---|
| `Default` | the default form |
| `Getter` | read-only getter form |
| `Setter` | modifying setter form |

### Format-string types

The template mini-languages a format argument can use: printf-style (`format` / `scan`), `clock format` fields, `binary` format/scan cursors, and `regsub` replacement templates. Validation follows the declared language.

| Value | Meaning |
|---|---|
| `Sprintf` | printf-style template (`format`, `scan`) |
| `Clock` | clock format / scan template |
| `Binary` | binary format / scan template |
| `Regsub` | regsub replacement template |

### Inline codegen hooks

Compiler internals: bytecode emitters for value-position (`set x [cmd …]`) and catch-body uses. Third-party commands leave this unset.

| Value | Meaning |
|---|---|
| `Expr` | expr |
| `Incr` | incr |
| `InfoExists` | info exists |
| `String` | string |
| `Lindex` | lindex |
| `Lrange` | lrange |
| `Lreplace` | lreplace |
| `Linsert` | linsert |
| `Regexp` | regexp |
| `List` | list |
| `Array` | array |
| `DictGet` | dict get |
| `Catch` | catch |
| `Return` | return |
| `Error` | error |
| `Break` | break |
| `Continue` | continue |
| `Try` | try |

### Lowering hooks

Compiler internals: the named per-command translations into the compiler's intermediate form. Listed for completeness when browsing core specs — a third-party command leaves the field unset.

| Value | Meaning |
|---|---|
| `Expr` | expr |
| `Return` | return |
| `Set` | set |
| `Incr` | incr |
| `AppendOrLappend` | append / lappend |
| `Unset` | unset |
| `Global` | global |
| `Variable` | variable |
| `Upvar` | upvar |
| `Proc` | proc |
| `When` | iRules when |
| `NamespaceEval` | namespace eval |
| `If` | if |
| `Switch` | switch |
| `For` | for |
| `While` | while |
| `Foreach` | foreach |
| `Lmap` | lmap |
| `ForeachLine` | foreach-line reader idiom |
| `Catch` | catch |
| `Try` | try |
| `Dict` | dict |
| `Eval` | eval |
| `Uplevel` | uplevel |
| `Apply` | apply |
| `ArrayFor` | array for |

### Option arity

How many value words an option consumes: one (`-index i`) or a fixed count. Options that take no value are declared by leaving takes-value off instead.

| Value | Meaning |
|---|---|
| `One` | consumes one value word |
| `Fixed` | consumes a fixed number of value words |

### Pattern types

The two pattern languages a Pattern argument can speak: glob (`string match`) and regular expressions (`regexp`). A `*` means something different in each, so the right label matters for validation and highlighting.

| Value | Meaning |
|---|---|
| `Glob` | glob pattern (`string match`, `lsearch -glob`) |
| `Regex` | regular expression (`regexp`, `regsub`) |

### Prefix matching

Whether a keyword table accepts any unique prefix (`string le` for `string length` — Tcl's normal behaviour) or only exact spellings (strict mode, matching `TCL_INDEX_STRICT`).

| Value | Meaning |
|---|---|
| `Enabled` | any unique prefix resolves (Tcl_GetIndexFromObj) |
| `Strict` | only the exact spelling resolves (TCL_INDEX_STRICT) |

### Side-effect targets

The kinds of state a structured side effect can read or write — from Tcl variables and channels through files, network, logs, and the whole F5 surface (HTTP state, tables, pools, SSL). Pick the closest target; `Unknown` exists for effects that fit nothing.

| Value | Meaning |
|---|---|
| `Variable` | Tcl variable read or write |
| `SessionTable` | session table entry |
| `PersistenceTable` | persistence record |
| `DataGroup` | data group / class lookup |
| `HttpHeader` | HTTP header read or write |
| `HttpBody` | HTTP payload / body |
| `HttpStatus` | HTTP status code |
| `HttpUri` | HTTP URI components |
| `HttpCookie` | HTTP cookie |
| `HttpMethod` | HTTP method |
| `Http2State` | HTTP/2 protocol state |
| `ResponseCommit` | commits or sends an HTTP response |
| `ConnectionControl` | drop / reject / discard / forward |
| `TcpState` | TCP connection state |
| `SslState` | SSL/TLS state |
| `UdpState` | UDP state |
| `PoolSelection` | pool selection |
| `NodeSelection` | node selection |
| `SnatSelection` | SNAT selection |
| `FileIo` | filesystem I/O |
| `NetworkIo` | network I/O |
| `LogIo` | logging output |
| `StreamProfile` | stream profile state |
| `DnsState` | DNS state |
| `ClassificationState` | classification state |
| `Dosl7State` | L7 DoS state |
| `FlowState` | flow state |
| `LsnState` | LSN state |
| `FtpState` | FTP state |
| `IcapState` | ICAP state |
| `MessageState` | message-routing state |
| `IStats` | iStats counters |
| `ApmState` | APM state |
| `AsmState` | ASM state |
| `BigipConfig` | BIG-IP configuration |
| `ProcDefinition` | procedure definition table |
| `NamespaceState` | namespace state |
| `InterpState` | interpreter state |
| `Process` | process creation / control |
| `ChannelIo` | channel I/O |
| `EventControl` | iRules event control flow |
| `Unknown` | unclassified effect |

### Storage types

The container kind a written variable ends up holding — list, dict, or array — so later reads are understood and kind mix-ups flagged.

| Value | Meaning |
|---|---|
| `Dict` | the target variable holds a dict |
| `List` | the target variable holds a list |
| `Array` | the target is a Tcl array |

### Taint colours

How untrusted data is tracked. A value read from the network is marked `TAINTED`; every value derived from it inherits the mark. Sanitisers and validators add colours — `HTML_ESCAPED`, `CRLF_FREE`, `IP_ADDRESS` — recording what has been *proved* about the value. A sink (output, exec, SQL, log) then checks arriving values: raw taint is a finding, while taint carrying the colour that sink accepts passes. Encoders also declare the colour that means "already encoded", which is how double-encoding is caught.

| Value | Meaning |
|---|---|
| `TAINTED` | attacker-controlled |
| `PATH_PREFIXED` | guaranteed to start with a path separator |
| `NON_DASH_PREFIXED` | cannot begin with `-` (option-injection safe) |
| `CRLF_FREE` | contains no CR or LF (header-injection safe) |
| `SHELL_ATOM` | a single shell atom (exec-safe) |
| `LIST_CANONICAL` | canonical list form (eval-safe) |
| `REGEX_LITERAL` | quoted as a regex literal |
| `PATH_NORMALISED` | path-normalised |
| `PATH_BOUNDED` | bounded within a known path root |
| `HEADER_TOKEN_SAFE` | safe as an HTTP header token |
| `HTML_ESCAPED` | HTML-escaped |
| `URL_ENCODED` | URL-encoded |
| `IP_ADDRESS` | a validated IP address |
| `PORT` | a validated port number |
| `FQDN` | a validated fully-qualified domain name |
| `PATH_JOINED` | produced by `file join` |
| `CHANNEL` | a channel handle |

### Traits

The registry's behavioural vocabulary — one flag per fact a consumer might need: evaluates its argument as code, alters control flow, creates an upvar-style alias, is a taint sink, is hidden in safe interpreters, and so on. Analyses read traits instead of matching command names, which is why setting the right traits on your command buys the same treatment the built-ins get. Search this list before assuming a behaviour cannot be expressed.

| Value | Meaning |
|---|---|
| `CONTROL_FLOW` | alters control flow |
| `LANGUAGE_KEYWORD` | a language keyword rather than a plain command |
| `HAS_BOOLEAN_COND` | takes a boolean condition |
| `TERMINATES_BLOCK` | terminates the enclosing basic block |
| `TERMINATES_PROCESS` | terminates the interpreter process without Tcl unwinding |
| `HAS_LOOP_BODY` | takes a loop body |
| `NEVER_INLINE_BODY` | its body must never be inlined |
| `LOOP_LIST_HEADER` | a loop header with list-expression arguments |
| `PURE` | side-effect free |
| `CSE_CANDIDATE` | eligible for common-subexpression elimination |
| `PURE_EVALUATION` | evaluation itself has no side effects |
| `DEFINES_PROCEDURE` | defines a procedure |
| `DESTROYS_VARIABLE` | destroys a variable |
| `READS_BEFORE_WRITE` | reads its target before writing it |
| `CREATES_SCOPE_ALIAS` | creates an upvar-like scope alias |
| `ALIASES_GLOBAL` | creates an alias to the interpreter global namespace |
| `CREATES_BARRIER` | creates an analysis barrier |
| `EVALUATES_CODE` | evaluates its argument as code |
| `PERFORMS_SUBSTITUTION` | performs Tcl substitution |
| `OPENS_CHANNEL` | opens an I/O channel |
| `SOURCES_FILE` | sources another file |
| `HAS_SWITCH_BODY` | takes a switch-style clause list |
| `STRING_LIST_CONFUSION` | at risk of string/list confusion |
| `CONFIGURES_CHANNEL` | configures a channel |
| `HAS_INTERP_EVAL` | evaluates code in another interpreter |
| `HAS_DESTRUCTIVE_OPS` | has irreversible operations |
| `IS_EVENT_HANDLER` | an iRules event handler |
| `UNNORMALISED_HTTP_GETTER` | returns unnormalised HTTP data |
| `REQUIRES_HTTP_CONTEXT` | requires an uncommitted HTTP transaction |
| `RETURNS_PATH` | returns a filesystem path |
| `IS_UNESCAPE` | performs unescaping or decoding |
| `PRODUCES_CANONICAL_LIST` | produces a canonical list |
| `BUILDS_COMMAND_PREFIX` | builds a command prefix |
| `WRAPS_COMMAND_PREFIX` | wraps a script into a command prefix |
| `UNSAFE` | unsafe in sandboxed dialects |
| `PASSWORD_OPTION` | takes a password-bearing option |
| `IS_SIDE_SWITCH` | switches the iRules connection side |
| `IRULES_TOP_LEVEL_ONLY` | iRules: valid only at the top level |
| `SETS_EVENT_PRIORITY` | sets the inherited iRules event priority |
| `IS_OO_METACLASS` | a TclOO metaclass factory |
| `OBJECT_COMMAND_SURFACE` | a TclOO object-command method surface |
| `CONFIGURES_BY_PROPERTY` | answers `configure`/`cget` from declared properties |
| `ABSTRACT_CLASS_FACTORY` | manufactures classes that cannot create instances |
| `DIAGRAM_ACTION` | an action node in extracted diagrams |
| `NEEDS_START_CMD` | needs an explicit start command |
| `TAINT_SINK` | a taint sink |
| `TAINT_SOURCE` | a taint source |
| `IRULES_DATA_GETTER` | an iRules data getter |
| `CREATES_DYNAMIC_BARRIER` | creates a dynamic (eval-like) barrier |
| `INVOKES_USER_PROC` | invokes a user-defined procedure |
| `BYTE_COMPILED` | byte-compiled by C Tcl |
| `NOT_PROC_FACTORY` | never defines a procedure |
| `FRAMELESS_RUNTIME` | runs without pushing a call frame |
| `FIRST_ARG_VARNAME` | its first argument is a variable name |
| `WHOLE_ARRAY_ARG` | takes a whole array as an argument |
| `DYNAMIC_EVAL_BODY` | its body is evaluated dynamically |
| `INTROSPECTS_BY_NAME` | introspects state by name |
| `CURRENT_FRAME_INTROSPECTION` | observes the current Tcl call frame |
| `EXPANSION_ESCAPE_SAFE` | expanded arguments cannot introduce a frame-sensitive name |
| `TARGETS_VARIABLE_BY_NAME` | targets a variable by name |
| `FRAME_HASH_BUILTIN` | a frame-hash builtin |
| `REFLECTS_COMMAND_NAMES` | can observe procedure names as data |
| `ALIASES_CALLER_FRAME` | aliases variables out of a runtime-chosen caller frame |
| `OVERRIDABLE_LIBRARY_PROC` | a library proc a script may override |
| `STRUCTURALLY_CHECKED_ARITY` | arity is checked structurally, not by range |
| `EXPR_CONCATENATES_ARGS` | concatenates its arguments into one expression |
| `SCRIPT_CONCATENATES_ARGS` | concatenates its trailing words into one script |
| `SCRIPT_APPENDS_LIST_ARGS` | appends its trailing words to the script as list elements |
| `ESTABLISHES_VARIABLE_TRACE` | establishes a variable trace |
| `TRANSFERS_CONTROL` | transfers control elsewhere |
| `FIRE_AND_FORGET_TEARDOWN` | fire-and-forget teardown |
| `OPERATOR_COMMAND` | an operator in command form |
| `TCLOO_NEXT_CHAIN` | participates in the TclOO next chain |
| `TCLOO_SELF_DISPATCH` | dispatches on the current TclOO object |
| `TCLOO_INTROSPECTION` | introspects the current TclOO method context |
| `BRANCH_SELECTED_BODY` | its bodies run at most once, chosen by a branch |
| `CATCHABLE_THROW` | throws a catchable error |
| `BREAKS_LOOP` | breaks out of a loop |
| `CONTINUES_LOOP` | continues a loop |
| `REPLACES_FRAME` | replaces the current call frame |
| `SAFE_INTERP_HIDDEN` | hidden in a safe interpreter |
| `PROVIDES_PACKAGE` | declares this file a loadable package |
| `LOADS_EXTERNAL_UNIT` | runs another unit's script in this interpreter |
| `EXPORTS_COMMAND` | publishes a command name for another unit |
| `UNRESOLVED_COMMAND_HANDLER` | handles the dialect's unresolved command words |
| `EVALUATES_IN_SHIFTED_FRAME` | runs its body script in another stack frame |
| `INSTALLS_NAMED_DEFINITION` | installs, moves, or extends a definition named by an argument |
| `TCLOO_METHOD_CONTEXT` | resolves only inside a TclOO method body |
| `TCLOO_BINDS_METHOD_ALIAS` | binds bareword aliases for methods of the current object |
| `TCLOO_REQUIRES_METHOD_FRAME` | calling it needs a real method invocation, not just an object frame |
| `DECLARES_NAMESPACE` | declares the namespace its NamespaceName word names |
| `TK_GEOMETRY_MANAGER` | a Tk geometry manager that claims a container |

### Value types

The internal representation a Tcl value carries alongside its string form — what a Tcl developer meets as shimmering. Used for return types and argument expectations. `Numeric` is "Int or Double"; `String` means a plain string with no cached structure.

| Value | Meaning |
|---|---|
| `String` | pure string, no cached intrep |
| `Int` | integer |
| `Double` | double-precision float |
| `Boolean` | boolean |
| `List` | Tcl list |
| `Dict` | Tcl dict |
| `ByteArray` | byte array (binary data) |
| `Numeric` | abstract join of Int and Double |
| `Object` | TclOO object instance |
| `Channel` | I/O channel handle |
