# The spec-pack DSL — syntax specification

> **Status:** design sketch for [spec-packs.md](../spec-packs.md) and
> [issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363).
> Nothing here is implemented. The syntax below was **designed by
> porting**: every construct exists because one of the eleven
> `*.tclspec.tcl` files beside this page needed it to say something a
> shipped spec already says.

## The ports

Each file names the `.rs` it was ported from in its header comment, so a
later implementation can diff the loaded `CommandSpec` against the shipped
one.

| pack file | ported from | what it forced into the design |
|---|---|---|
| [`lsort.tclspec.tcl`](lsort.tclspec.tcl) | `commands/tcl/lsort_.rs` | option rows, command-prefix options, integer domains, per-option dialect gates |
| [`foreach.tclspec.tcl`](foreach.tclspec.tcl) | `commands/tcl/foreach_.rs` | stepped arity, repeated-argument layouts, the first hook body |
| [`string.tclspec.tcl`](string.tclspec.tcl) | `commands/tcl/string_.rs` (`length`, `is`, `map`, `range`) | pack-level shared tables, closed value sets, subcommand facts, const-folds as Tcl |
| [`switch.tclspec.tcl`](switch.tclspec.tcl) | `commands/tcl/switch_.rs` | `-also` arity, an inline `case_list`, option-skipping in a resolver |
| [`if.tclspec.tcl`](if.tclspec.tcl) | `commands/tcl/if_.rs` | the declarative `clause_grammar` (extra port — see below) |
| [`oo-class.tclspec.tcl`](oo-class.tclspec.tcl) | `commands/tcl/oo_class.rs` | manufacturer rows, derived role resolvers, named descriptors |
| [`geturl.tclspec.tcl`](geturl.tclspec.tcl) | `commands/tcllib/uri__geturl.rs` + `commands/stdlib/http__geturl.rs` | package gating, tri-state index lists, credential options |
| [`irules-http-header.tclspec.tcl`](irules-http-header.tclspec.tcl) | `commands/irules/http__header.rs` | event requirements, subcommand-scoped taint sinks |
| [`upvar.tclspec.tcl`](upvar.tclspec.tcl) | `commands/tcl/upvar_.rs` | `frame_effect`, and state transitions derived from it |
| [`snit-type.tclspec.tcl`](snit-type.tclspec.tcl) | `commands/tcllib/snit__type.rs` + `definer.rs`'s `SNIT_GRAMMAR` | the inline `definition_body` grammar: member rows, built-in method rows, member-body commands, bare-word construction |
| [`return.tclspec.tcl`](return.tclspec.tcl) | `commands/tcl/return_.rs` | the option-arity hook inside an option row, a context gate, per-option dialect gates, dialect-gated form rows |

`if` is a ninth, unrequested port: it is the only shipped consumer of
`clause_shape_check`, and the DSL's whole answer to that field is to make
it unnecessary, so the design is not testable without it.

`snit::type` and `return` are the two **pre-freeze** ports. The design
review found the sketch failing its own rubric in two places, and both
are places where the honest fix is a port rather than an amendment:
[`tricky-surfaces.md`](tricky-surfaces.md) requires definer grammars and
`body_scope` "from day one" while the sketch made both reference-only,
and it names `return -errorstack` as "the worked example" of an
option-arity hook while no such example existed and the family's inputs
were unspecified. Both constructs are now designed *by* those ports, not
around them.

## Shape of a pack

```tcl
speclib <pack-name> <dsl-version> {
    default     <key> <value>…    ;# pack-wide default for one availability key
    values      <name> { … }      ;# a shared argument-value table
    hook        <name> {params} { … }   ;# a shared hook body
    descriptor  <key> <name> { … }      ;# a shared block-valued descriptor
    command     <name> ?-override? { … }
}
```

A pack is **one Tcl script, read from the CST and never executed.** Every
declaration is `word word…` with braced words for blocks and prose — the
loader walks the parse tree. The only Tcl that is ever *evaluated* is a
hook body, and only at query time, in the sandbox described below.

`speclib`'s version is the **DSL vocabulary version**, not the library's:
it gates hard breaks (a word whose *meaning* changed), never additions.
It is the **only** loader directive: an earlier sketch of this grammar
also showed a `pragma …` statement, which is **dropped** — it was never
given a meaning, and the one job it could have had (gating a hard break)
is what the `speclib` version word already does. A pack containing
`pragma` therefore hits the ordinary unknown-property rule below: dropped
with a logged notice.

**Statement separation is ordinary Tcl.** Statements end at a newline or
at a `;`, so a short declaration can be written on one line —
`subcommand at { arity 1 ; detail {Get header name by index.} ;
synopsis {HTTP::header at <index>} }`, as
[`irules-http-header.tclspec.tcl`](irules-http-header.tclspec.tcl) does
throughout. The trap that comes with it is also ordinary Tcl: a `#` only
starts a comment **where a command word would start**, so
`arity 2 # not a comment` passes `#` and `not` and `a` and `comment` to
the `arity` row as flags (all four dropped with a notice), and a trailing
comment on a statement needs the `;#` spelling. Both behaviours are the
Tcl an author already knows; they are written down here because a spec
pack looks like data and invites the assumption that it is not a script.

## Words and values

| shape | spelling | notes |
|---|---|---|
| bool | `pure`, `pure yes`, `pure no` | the bare word means `yes`; **absent** means the field's default |
| tri-state bool | `xc_translatable no` | the argument is required — absent is `unset`, which is not `false` |
| count / index | `reserved_trailing_words 2` | plain integer |
| text | `taint_output_sink IRULE3002` | brace it when it has spaces |
| prose | `description { … }` | a braced word may span lines; **every byte between the braces is the value**, newlines included — see "Brace-quoted prose" below |
| text list | `excluded_events {HTTP_REQUEST CLIENT_ACCEPTED}` | a Tcl list |
| index list | `taint_network_sink_args {0}` | tri-state where the field is: `{}` is *declared empty*, absent is *unset* |
| enum | `body_kind Structural` | the variant name **verbatim** from the studio catalogue |
| enum + payload | `var_write_typing {ElementsOf 0}` | variant word first, then its fields in declaration order |
| flag set | `traits {PURE CSE_CANDIDATE}` | member words verbatim; unioned |
| dialect set | `dialects {all-tcl f5-irules}` | members verbatim, plus the set words below |

**Dialect set words.** Members are exactly as `fields.md` spells them
(`tcl8.4` … `tcl9.1`, `f5-irules`, `f5-iapps`, `tk`, `expect`, `bpf`,
`f5-tmsh`, `f5-bigip`). Two shorthands are added because writing five
version bits per option is unreadable: a `+` suffix on any Tcl version
means "and later" (`tcl8.5+`), and `all-tcl` / `tcl8.x` name the two
common closures. `{all-tcl f5-irules}` is `ALL_TCL.union(IRULES)`.

**Arity** is one range word plus modifiers, mirroring the four `Arity`
fields:

| written | means |
|---|---|
| `arity 3` | exactly 3 |
| `arity 1..2` | 1 to 2 |
| `arity 1..` | at least 1 |
| `arity ..2` | 0 to 2 |
| `arity ..` | any |
| `arity 3.. -step 2` | 3, 5, 7, … (`foreach`) |
| `arity 3.. -step 2 -also 2` | that, plus exactly 2 (`switch`'s braced form) |

`max` and `min` are accepted as integer-domain sentinels
(`-integer {Range 2 max}`).

## Statements

### Per-argument facts

Six schema keys are indexed by the same argument position, and reading
them as six parallel tables is the single worst thing about the `.rs`
form. The DSL collapses them into one row per index — not every flag is
used together, but every one belongs to the same word:

```tcl
arg 0 -type String -shimmers -transparent {ByteArray} \
      -values-from is-classes -closed
arg 1 -role Body -layout InlineScript
arg 2 -appends {Exactly 2}
```

| flag | fills |
|---|---|
| `-role ROLE` | `arg_roles` |
| `-type T`, `-shimmers`, `-transparent {T …}` | `arg_types` |
| `-values {…}` / `-values-from NAME` | `arg_values` |
| `-closed` | `closed_value_args` |
| `-layout BlockScript\|InlineScript` | `arg_presentation` |
| `-appends {Exactly N}` | `command_prefixes` (implies `-role CommandPrefix`) |

Indices are 0-based after the command name, or after the subcommand word
inside a `subcommand` block — the same coordinates the registry uses. An
index above 255 is dropped with a notice, matching the `u8` tables.

### Option rows

```tcl
option -stride -takes strideLength -integer {Range 2 max} -dialects tcl8.6+ \
       -detail {Treat list as consecutive groups of strideLength elements …}
```

`-takes HINT` is what makes an option a value option; without it the
option is a flag. The value's own fields are flags on the same row:
`-arity {Fixed 4}`, `-arity-hook`, `-role`, `-also-role`, `-body-kind`,
`-values` / `-values-from`, `-closed`, `-integer`, `-appends`. The
option's own fields are `-detail`, `-aliases`, `-dialects`,
`-min-abbrev`, and the four lifecycle flags `-introduced` /
`-deprecated` / `-retired` / `-deprecation-fix`.

`-arity` takes the static shapes only (`{Fixed N}`; `One` is the
default). `OptionArity`'s third variant is a hook, and it gets its own
flag, `-arity-hook {words ctx} { … }` (or `-arity-hook -native ID`),
because a hook is *two* words — a parameter list and a body — where every
other flag value is one, and because a row parser must not have to tell
`{words ctx}` from an enum payload by inspection. See "The option-arity
hook" under Hooks, and [`return.tclspec.tcl`](return.tclspec.tcl).

### Other rows

```tcl
form Default {lsort ?options? list}
side_effect HttpHeader -reads -writes -side Both
repeat LoopVarList -from 0 -stride 2 -exclude-trailing 1
manufacturer create -names-instance-at 1 -definition-body-at 2 -constructor-args-from 2
option_conflict {-glob -regexp}
setter_constraint 0 -prefix / -code IRULE3101 -message {…}
sub_subcommand isa -detail {…} -synopsis {…}
oo_context_fact class DefiningClass
versioned_arg_value 0 utf-8 -introduced 8.6
```

### Documentation

```tcl
hover {
    summary  {Sort the elements of a list.}
    synopsis {lsort ?options? list}      ;# repeatable, in order
    description { … }                    ;# HoverSnippet.snippet
    source   {Tcl lsort(n)}
    example  { … }                       ;# repeatable, joined with newlines
    returns  { … }                       ;# HoverSnippet.return_value
}
```

Three words are renamed from their Rust field names — `snippet` →
`description`, `examples` → `example` (repeatable), `return_value` →
`returns`. These are the fields authors write most, and `snippet` in
particular describes prose that is not a snippet. The rename is recorded
in the coverage matrix; nothing else in the DSL renames a key.

This is where the DSL beats the `.rs` outright: a braced word keeps real
newlines and literal backslashes, so a multi-line example is written as
itself instead of as one `"…\n…\\w…"` string.

### Pack-level declarations

```tcl
values is-classes {
    value alnum -detail {Any Unicode alphabet or digit character.}
    value dict  -min-tcl tcl9.0 -detail {Any proper dict structure …}
    value ok    -code 0 -detail {…}
}
descriptor world_effects class-factory-effects { … }
hook ascii-only-length {words ctx} { … }
default required_package mylib
```

A name declared in the pack shadows a shipped catalogue name of the same
kind, and the loader says so. `default` takes only availability/identity
keys (`dialects`, `required_package`, `tcllib_package`, the three
versions, `warn_missing_import`, `is_namespace_exported`); a command
stating the key itself wins.

### Block statements

Ten properties take a braced block instead of a value, and each may
instead name a `descriptor` or a shipped constant. Their inner words are
the descriptor's own field names, so nothing new has to be learnt:

| block | inner words |
|---|---|
| `hover` | `summary`, `synopsis`*, `description`, `source`, `example`*, `returns` |
| `values NAME` | `value V ?-detail {…}? ?-min-tcl VER? ?-code N?`* |
| `case_list` | `subject_args`, `exact_option`, `glob_option`, `regex_option`, `nocase_option`, `end_options_option`, `fallthrough_body`, `value_options_require_regex`, `clause_flags`, `clause_regex_flag`, `clause_value_flags`, `keyword_patterns {…} ?-final-only?` |
| `clause_grammar` | `head {slots}`, `repeated KEYWORD {slots}`*, `tail ?KEYWORD? {slots}` |
| `event_requires` | `client_side`, `server_side`, `transport`, `profiles`, `also_in`, `init_only`, `flow`, `capability` |
| `world_effects` | `composition`, `access …`*, `callback -kinds {…} -reentrancy R`, `resolver`, `dynamic_fallback` |
| `state_transitions` | `composition`, `argument_shape`, `resolver`, `widen -operands L -domains {…}`*, `covers SOURCE -domains {…}`*, `commit` |
| `object_class NAME` | `superclasses`, `allow_unknown_methods`, `method NAME { … }`* (a `subcommand` body) |
| `definition_body` | `family`, `member …`*, `member_option …`*, `implicit_vars`, `member_body_namespace_path`, `builtin_type_methods`, `builtin_object_method …`*, `builtin_terminating_methods`, `member_body_command …`*, `bare_word_construction`, `dynamic_method_dispatch`, `manufacturer …`*, `unknown_dispatch_method`, `property_accessor_methods` |
| `body_scope` | `name`, `include_sibling_definitions`, `allow_unknown_commands`, `command NAME { … }`* |

`*` marks a repeatable row. `world_effects none` is the one-word
`WorldEffectDescriptor::EMPTY`. The last two are specified under
"Definer grammars and scoped bodies" below.

## Hooks

Eight fields across `CommandSpec` / `SubCommand`, plus one inside an
option row, are function pointers today. Each becomes a **small pure Tcl
body with a proc-shaped signature**:

```tcl
<field> {words ctx} { … body … }
```

or a reference to a shipped implementation:

```tcl
<field> -native <id>
```

`-native ID` is the spelling for **every field whose value names engine
code**: the function-pointer fields above and the closed compiler-hook
catalogues (`lowering_hook`, `codegen_hook`, `inline_codegen_hook`,
`analyser_hook`, `bpf_op`). They share a spelling because they share a
meaning — "the engine, by name" — and a load policy. IDs are
`command::hook` for a per-command implementation (`string::is`,
`oo::class::create`) and the bare catalogue variant for a shared one
(`Foreach`, `Upvar`). `semantic_operation` deliberately keeps the enum
spelling: it names an operation *identity*, not an implementation.

or, where the DSL can derive the behaviour from data the spec already
declares, a closed keyword — `arg_role_resolver from-manufacturers`,
`state_transitions … resolver from-frame-effect`, and the whole of
`clause_grammar`.

The ninth of those fields — the option-arity hook — is a *flag* on an
option row rather than a property statement, so it is written
`-arity-hook {words ctx} { … }` (or `-arity-hook -native ID`).
Everything in the rest of this section applies to it unchanged.

### Inputs

`words` is a Tcl list of the call's argument words **after the command
name** — or after the subcommand word, for a hook on a `subcommand`.
That is exactly the `args: &[&str]` every current hook receives.

`ctx` is a dict. Keys always present:

| key | value |
|---|---|
| `command` | the resolved command name |
| `subcommand` | the resolved subcommand word, or empty |
| `nwords` | `[llength $words]`, for symmetry with the argv-shaped hooks |
| `kinds` | one word per element of `words`: `literal`, `dynamic`, `expanded`, or `opaque` |
| `tcl-version` | `8.4` … `9.1`, or empty when the profile names no release |
| `dialect` | the active dialect member word |
| `in-event-body` | `0` / `1` — the one lexical fact `context_gate` takes today |

`kinds` is the load-bearing part for `literal_argument_validator`, whose
Rust signature takes `InvocationArguments` rather than plain strings
precisely so a substituted or `{*}`-expanded word cannot be mistaken for a
value. A word whose kind is not `literal` appears in `words` as the empty
string, so a hook that forgets to check `kinds` sees nothing rather than
source spelling.

**One family adds keys of its own**, because it is the only one whose
Rust signature takes more than `args`
(`fn(args: &[&str], start: usize)`):

| family | extra `ctx` keys |
|---|---|
| option-arity hook | `option` — the option word as written (`-errorstack`); `option-index` — its 0-based index in `words`; `option-value-start` — index of the first value word, i.e. `option-index + 1` |

Every other family reads only the keys above; `const_fold_versioned` in
particular needs nothing beyond `tcl-version`, which is always present.

### When a hook runs

**Normative.** `const_fold`, `const_fold_versioned`, and the option-arity
hook are invoked **only when every word's `kinds` entry is `literal`**;
every other family states its own precondition in the verb table below.
A loader that calls a fold body on a call carrying a dynamic word is
wrong, not merely imprecise.

This is not a new rule, it is the rule the Rust already obeys and the DSL
failed to write down. The fold callbacks are reached from
`ConstSubstCtx::fold_at_depth`
(`rust/tcl-compiler/src/const_subst.rs:103-134`), whose first act is
`literal_words_at_depth`, which bails — no fold at all — if any word is
not a single clean literal token. Because the DSL's own convention is
that a non-literal word arrives as the empty string, a fold body that
never consults `kinds` would otherwise fold `string length $x` to `0`,
`string range $s 0 1` to `""`, and `string map $m abc` to `abc`. With the
sentence above, all three bodies in
[`string.tclspec.tcl`](string.tclspec.tcl) are sound exactly as written;
without it, all three are unsound. **No fold body should re-check
`kinds` itself** — the precondition is the loader's, stated once, rather
than a guard every author must remember.

The option-arity hook's precondition is the same sentence but splits in
two on the way through, and the split is worth pinning because it is what
the shipped consumers do:

- Its **`consume N`** is a syntax fact the option scanner needs for every
  call, literal or not, so a non-literal call does not run the hook and
  takes the option's declared fallback span instead — `-arity {Fixed N}`
  when the row states one, otherwise one word. That fallback is what
  `errorstack_value` itself returns for any present word, so for `return`
  the split is behaviour-preserving; an option whose *word count* is a
  function of dynamic content cannot be a pack option and is a
  contribution.
- Its **`-invalid MESSAGE`** is discarded unless every value word it
  covers is literal. That half is not a design choice: `w141_hook_hit`
  (`rust/tcl-compiler/src/analyser/diagnostics/security.rs:1712-1726`)
  already skips the hook outright when any value word contains `$` or
  `[`, precisely so a content check never fires against a word whose
  content is unknown.

An unversioned `const_fold` body inherits the hook VM's Tcl semantics
while the target profile may be anything from 8.4 to 9.1, so a third
rule falls out of the same place: **an unversioned fold body must be
version-invariant over the inputs it accepts**, and the equivalence gate
runs per dialect. The `string` port is what this rule is drawn from —
its `length` / `map` / `range` bodies all guard on `string is ascii`
precisely to stay inside the range where 8.x and 9.x agree, and
`string is` stays `-native` because it is version-aware by nature.

### Outputs: the emitter protocol

**Every hook's own return value is ignored.** Each family injects one to
three verbs; calling none is an abstention. One protocol for all eight
fields, so "what does silence mean" has one answer per field and it is
always the conservative one.

| field | verbs | silence means |
|---|---|---|
| `arg_role_resolver` | `role IDX ROLE` | no roles (fall back to `arg_roles`) |
| `command_prefix_resolver` | `prefix IDX {Exactly N}` | no prefix positions |
| `const_fold` / `const_fold_versioned` | `fold VALUE` | no fold |
| `taint_sink_gate` | `sink-applies`, `sink-suppressed` | **the sink applies** |
| `context_gate` | `reject MESSAGE` | the call is allowed |
| `literal_argument_validator` | `invalid -index N -subject S -reason … -allowed {…} ?-replacement V?`, `abstain REASON` | valid |
| `clause_shape_check` | `missing-expr ?after?`, `missing-body after`, `extra-words first` | the shape is accepted |
| option-arity hook (`-arity-hook`) | `consume N ?-invalid MESSAGE?` | consume one word |

Returning early (`return`) is the ordinary way to abstain, which is why
the emitter protocol beats returning a value: `if {…} return` reads
naturally and cannot be confused with "folds to the empty string".

### The option-arity hook

The one family with no example until now, and the one the rubric names
as "the worked example of a hook *inside* an option row". Its calling
convention differs from the eight property hooks in exactly one way: the
Rust signature is `fn(args: &[&str], start: usize)`
(`rust/tcl-registry/src/hover.rs:89`), so a uniform `{words ctx}` would
throw away the `start` the body needs. The fix is `ctx`, not a third
parameter:

- `words` **is** `args` — the call's argument words after the command
  name, unchanged from every other family. Note that this is the *whole*
  argument list, not the option's value slice: the hook is free to look
  before and after its own option, which is what lets one option's span
  depend on a preceding flag.
- `ctx`'s `option-value-start` **is** `start` — the index in `words` of
  the first value word. `option-index` (`start - 1`) and `option`
  (the option word as written) are supplied alongside it so a body never
  recomputes either; the caller always knows both
  (`OptionSpec::value_span` derives `start` from `flag_idx + 1`,
  `hover.rs:557`).
- The emitter verb is `consume N ?-invalid MESSAGE?`, and silence
  consumes one word. `consume 0` is how the shipped hook reports a
  missing value: it is *not* an abstention, because a hook that emits
  nothing still consumes one.

[`return.tclspec.tcl`](return.tclspec.tcl) is the port. The whole family
exists for the `-errorstack` shape — a *fixed* one-word span whose
**content** needs checking — so its body never varies the span at all;
it emits `consume 1` three ways and `consume 0` once. Two notes from
writing it:

- The sandbox has no `catch`, so the Rust's `split_list_raw` →
  `Ok`/`Err` split is written as `string is list`. The two agree on
  which words are lists, and `string` is on the whitelist.
- The three messages are carried verbatim from the `.rs`. They are user
  visible (W141), so an equivalence gate compares them as data, not as
  behaviour.

### Error means abstain

Any error raised inside a hook body — a bad index, a malformed list, a
budget overrun — is an abstention. It is logged once per hook per pack
load and never reaches a diagnostic on the user's code. This is what lets
`string range`'s folder be three lines: a malformed index makes `string
range` raise, and raising is exactly the `None` the Rust returns.

### Purity and the sandbox

A hook body is evaluated in `tcl-vm` with only a whitelist exposed:
`set`, `expr`, `if`, `while`, `for`, `foreach`, `switch`, `return`,
`break`, `continue`, `incr`, `lappend`, `lassign`, `list`, `lindex`,
`llength`, `lrange`, `lreplace`, `lsearch`, `lsort`, `join`, `split`,
`string`, `format`, `scan`, `regexp`, `regsub`, `dict`, `binary`, plus
the family's emitter verbs. No `open`, `exec`, `source`, `socket`,
`after`, `interp`, `uplevel`, `upvar`, `trace`, `namespace`, `proc`,
`rename`, `info`, or `subst`. A hook has a step budget and a wall-clock
cap; exceeding either is an abstention.

Hooks run per call site, so they are the one part of a pack whose cost is
not "identical to compiled-in". Shipped packs keep native pointers, so
only pack-declared commands pay it, and the answer is memoisable by
(command, word-shape) — the key spec-packs.md's hot-path budget assumes.
The body is compiled to bytecode once at pack load, not per call.

## Clause grammars, declaratively

`clause_shape_check` exists because `if`'s grammar is not a `min..=max`
range. It is, however, perfectly regular, and the manpage already writes
it down. So the DSL writes the manpage:

```tcl
clause_grammar {
    head            {Expr ?then? Body}
    repeated elseif {Expr ?then? Body}
    tail     ?else? {Body}
}
```

- **`head {slots}`** — the mandatory leading clause, matched
  *positionally*. Its slots are never keyword-matched, which is why `if
  else {a}` is a well-formed `if` whose condition is the bareword `else`
  — the behaviour `IfConditionCallback` has.
- **`repeated KEYWORD {slots}`** — zero or more clauses, each introduced
  by that literal word (role `Keyword`).
- **`tail ?KEYWORD? {slots}`** — at most one clause, last. A bare
  `KEYWORD` requires the word; `?KEYWORD?` makes it optional, which is
  what allows `if`'s implicit trailing body.
- Inside a slot list, a bare word is an `ArgRole` name and `?word?` is an
  optional noise keyword.

**Normative — where keywords match.** A `clause_grammar` keyword is
matched **only at a clause boundary and at a `?noise?` position**; every
other slot is filled positionally and consumes whatever word is there,
including one spelled like a keyword. The rule is stated for the whole
grammar, not just for `head`: an earlier draft said it of `head` alone,
which is not enough to derive `walk_if`'s own test matrix. `if 1 a
elseif else b` is the row that proves it — the `else` is the `Expr` slot
of the `elseif` clause, matched positionally *inside a repeated clause*,
and the call is well-formed. Reading it any other way makes the tail
keyword win and turns a valid `if` into a shape error. The one-line
version an implementer can test against: at each step the walk asks
"does a clause start here?", and only that question ever compares a word
against a keyword.

From that one declaration the loader derives **both** hook behaviours:

- `arg_role_resolver` — the roles the walk assigns, and
- `clause_shape_check` — `MissingExpr{after}` for an absent `Expr` slot,
  `MissingBody{after}` for an absent `Body` slot, `ExtraWords{first_extra}`
  for anything past the tail.

Walked against `if_.rs`'s own test matrix, the generated walk agrees case
for case with `walk_if`, including the two subtle rows (`if else {a}` is
valid; a bare trailing body needs no `else` but nothing may follow it).
`STRUCTURALLY_CHECKED_ARITY` is *not* implied — the pack still declares
it, and the loader warns if a `clause_grammar` command omits it.

Case lists are the other clause shape and stay a separate field, because
they are a *value* (`{pattern body …}` inside one word) rather than a
word grammar. `case_list switch` names the shipped descriptor;
`case_list { … }` spells out all thirteen plain-data fields, which is
what a private Expect-like command needs.

## Derivations, exactly

A derivation keyword replaces a hook with a rule the loader runs over
data the spec already declares. That makes each one a **claim about the
Rust it replaces**, so each needs its behaviour written down normatively
rather than left in a port's comment — which is where two of the three
were living.

**`arg_role_resolver from-manufacturers`.** The rule is: look
`words[0]` up in **this spec's own `manufacturer` rows**; if it names one
and that row has a `-definition-body-at N`, emit `role N Body`, provided
`N` is a valid index into the call's arguments; otherwise emit nothing.
Three details are load-bearing and none is negotiable:

- **`words[0]` only.** The manufacturer keyword is the first argument
  word, never searched for elsewhere in the call.
- **`Body` only.** `-names-instance-at` and `-constructor-args-from` are
  read by other consumers and contribute **no** role here. A derivation
  that also emitted a role for the instance name would be a different
  (and wrong) resolver.
- **Bounds-checked.** `oo::class create` with no body word emits nothing
  rather than a role pointing past the end.

That is `oo_class_arg_roles` (`rust/tcl-registry/src/commands/tcl/oo_class.rs:245-256`)
exactly: `args.first()` → `manufacturer(word)` → `definition_body_at` →
`(body < args.len())`.

**`state_transitions … resolver from-frame-effect`.** The rule is: read
the command's own `frame_effect`; take the level word from its
`-level-word` policy; then walk the remaining words as the `-layout`
says. Two policies must be pinned because they are where an
implementation would silently differ from the shipped resolver
(`upvar_.rs:59-97`):

- **A dynamic level word aborts the whole derivation** — zero
  transitions for the call, not "assume the default frame".
- **A dynamic member of an alias pair skips that pair and continues** —
  the pairs after it still produce transitions.

**`clause_grammar`.** Derives both `arg_role_resolver` and
`clause_shape_check`; its own normative rule (where keywords match) is
stated above.

**Derivation is a proof obligation, not a shorthand.** Each keyword
above asserts that a data-driven rule reproduces a shipped function.
Reading the Rust, all three do — but an implementation must **prove**
it, not assume it, and the fidelity table below flags each port that
rests on one. The sharpest case is `oo::class`: the shipped resolver
reads `TCLOO_GRAMMAR.manufacturers` while the spec field the port
declares is `TCLOO_ROOT_CLASS_MANUFACTURERS`. Those are two *different*
tables — they differ on `new`'s visibility — that happen to agree on
every keyword and body index today. The derivation is therefore correct
now and is not correct *by construction*; a loader must diff the two
before trusting it, and a future divergence must fail the equivalence
gate rather than pass silently.

## Load policy

- Unknown property words, unknown flags on a known row, unknown trait /
  role / colour / hook names: **dropped with a logged notice**, the rest
  of the spec loads. New server + old pack works; old server + new pack
  degrades.
- A pack may **name** any of the closed native-hook catalogues
  (`lowering_hook`, `codegen_hook`, `inline_codegen_hook`,
  `analyser_hook`, `semantic_operation`, `bpf_op`) — bucket 2 of
  spec-packs.md's hook plan: a pack reuses named hooks, it cannot add to
  them. Naming a *lowering* or *codegen* hook is reported at load, since
  it changes how the compiler translates the command rather than what the
  editor knows about it.
- `command NAME -override { … }` claims a name a shipped spec already
  has; without it, shipped wins and the collision is reported.

## What a pack cannot author

Four kinds of field are **excluded** outright, and two more are
**reference-only**. Every one is in the coverage matrix with its reason;
the summary is:

| field(s) | why not |
|---|---|
| `command_forms`, `subcommand_forms` | per-form bundles of arity/roles/options/hooks. `forms` covers the getter/setter split; a command needing the structured form is deep enough in the compiler to be a contribution. Six shipped modules use them; the list is in [spec-packs.md](../spec-packs.md). |
| `completion` | a compiler proof obligation, not a description of the command. See the rationale below. |
| `dispatch_dependencies` | specialisation-proof machinery whose meaning is defined by the optimiser; `fields.md` itself says "leave unset". |
| `data_collection`, `bpf_op` | shared named descriptors, referenced by name — the boundary spec-packs.md's bucket 2 draws. `data_collection`'s descriptor is paired with protocol machinery outside the registry; `bpf_op` is a closed compiler catalogue. |
| the `resolver` of `world_effects` / `state_transitions` | a function producing typed transition facts. The surrounding plain data *is* authorable; only the resolver is `-native`, `none`, or a derivation keyword. |

`definition_body` and `body_scope` **were** on that list and are not any
more — see the next section.

### Why `completion` is excluded and `const_fold` is not

The one-line reason ("a wrong value is unsound, not imprecise") does not
survive contact with `const_fold`, which is authorable, is written in
Tcl by hand, and can be *more* wrong more easily. Stating the real
asymmetry now is cheaper than being asked later:

- **A wrong fold is a wrong value in one expression.** It is bounded by
  the call site, it is observable by running the corpus-output
  equivalence gate, and every consumer downstream of it is still
  operating on a well-formed program. The mitigation is a test, and the
  DSL provides one.
- **A wrong `completion` is a wrong control-flow graph.** It tells the
  compiler which edges leave the command — whether the block can fall
  through, whether a loop can exit here, whether the code after it is
  reachable at all. Get it wrong and the CFG is structurally false, so
  every later pass reasons about a program that does not exist: DCE
  deletes live code, the optimiser reorders across an edge that is real,
  the analyser reports on branches that cannot run. There is no output
  to diff, because the damage is to the shape of the analysis rather
  than to a value in it.
- **`fields.md` already says "leave unset"** for the same reason it says
  it of `dispatch_dependencies`: unset means "assume anything", which is
  always sound. An authored value can only ever *narrow* that, and
  narrowing is the unsound direction.

So the exclusion is not "completion is hard" — it is that completion is
the one field where the conservative default is free and the
non-default is a claim only the compiler can check. What a pack *can*
still say about the standard codes is the **traits**, which stay
authorable and are paired in two directions: `BREAKS_LOOP`,
`CONTINUES_LOOP`, and `CATCHABLE_THROW`
(`rust/tcl-spec-studio/src/catalogue.rs:524-526`) go on the command that
*performs* the non-normal completion — `break` carries `BREAKS_LOOP`
(`rust/tcl-registry/src/commands/tcl/break_.rs:37-42`) — and
`HAS_LOOP_BODY` goes on the command that *accepts* one, such as
`foreach`. Between them a pack can describe break/continue/raise
without touching `CompletionDescriptor`. A library-defined code —
`struct::tree`'s `return -code 5`, meaningful only inside its own `walk`
body — fits neither half: there is no way to pair a custom code to the
one command whose body legitimately accepts it. That is recorded as a
known limit below rather than papered over.

### Definer grammars and scoped bodies

`definition_body` and `body_scope` are **authorable**, inline or as a
`descriptor`, and this is a change from the first draft of this memo.
They were reference-only on the argument that no port needed an inline
form; [`tricky-surfaces.md`](tricky-surfaces.md) requires both "from day
one", and the honest way to close a rubric gap is a port, not an
amendment. [`snit-type.tclspec.tcl`](snit-type.tclspec.tcl) is that
port, and it designed the form.

`definition_body NAME` still names a shipped grammar (`tcloo`,
`tcloo-configurable`, `snit`, `snit-widget`, `itcl`) — that is what
every core port does. `definition_body { … }` and
`descriptor definition_body NAME { … }` spell one out. The block's rows
are `DefinitionBodyGrammar`'s own fields — the listing below is the
**grammar**, with `?…?` marking optional flags and `A|B` alternatives, so
it is not itself a loadable block; the loadable one is the snit port:

```tcl
definition_body {
    family Snit                              ;# TclOo | Snit | Itcl
    member method     -roles {0 Name 1 ParamList 2 Body}
    member superclass -all-refs Class -slot Set
    member variable   -all-vars -slot Append -dedup
    member property   -kind FlagKeyed -dialects tcl9.0+
    member self       -kind Wrapper -block-body
    member export     -all-refs Method -visibility Exported
    member renamemethod -all-refs Method -retracts FirstArgument
    member option                            ;# keyword-only
    member_option method 1 -export -role Option -visibility Public -dialects tcl9.0+
    implicit_vars {self selfns type options}
    member_body_namespace_path {::oo::Helpers}
    builtin_type_methods {info destroy}
    builtin_object_method cget ?-unexported? -receiver AnyObject|ClassObject -detail {…}
    builtin_terminating_methods {unknown}
    member_body_command install -detail {…} -binds-handle {-name-from {Word 0} -class-from {Word 2} -keyword {1 using}}
    bare_word_construction ?-hint-values {…}? ?-hint-prefixes {…}?
    dynamic_method_dispatch
    manufacturer create ?-unexported? ?-names-instance-at N? ?-definition-body-at N? -constructor-args-from N
    unknown_dispatch_method unknown
    property_accessor_methods {configure}
}
```

Four notes, three of them things the port changed:

- **`-roles {N ROLE …}` replaced the sketched
  `-name 0 -params 1 -body 2`.** `MemberSpec::arg_roles` is a list of
  (index, role) pairs, so the sketch needed one new flag word per
  `ArgRole` and still could not spell snit's `onconfigure -option
  valueVar BODY`, whose roles sit at 1 and 2 with index 0 carrying none.
  `-roles` *is* the field.
- **`builtin_object_method` is a row, not a name list.** Each entry
  carries a visibility and a receiver as well as a name, and the
  visibility decides which dispatch spellings reach it (`my variable v`
  works, `$obj variable v` does not). `?-unexported?` is spelled exactly
  as on `manufacturer`.
- **`bare_word_construction`'s hint is data.** It is the one function
  pointer in `DefinitionBodyGrammar`, and reading it
  (`definer.rs:1651-1653`) it is an exact-word set plus a prefix set —
  `%AUTO%`, or a leading `.`. A family whose hint is not that shape
  keeps `-native`.
- **`member_option` is spelled, not ported.** It is the one
  `MemberSpec` field the snit grammar does not exercise
  (`optional_argument`); its witnesses are the shipped TclOO rows
  `method ?-export|-private|-unexport?` and `definitionnamespace
  ?-class|-instance?`, read from `definer.rs:1196-1240`. It is a
  sibling row keyed by the member keyword and the fixed position, so
  option-bearing members stay rows rather than growing a nested block.

`body_scope` takes the same treatment, one level smaller —
`ScopedCommandEnv` is four fields and `ScopedCommand` is six:

```tcl
descriptor body_scope report-style {
    name {report style definition}
    include_sibling_definitions
    allow_unknown_commands no
    command top {
        arity 1..2
        detail {…}
        allow_unknown_subcommands no
        subcommand set { arity 1  detail {…} }
        subcommand get { arity 0  detail {…} }
        hover { … }
    }
}
```

Inside a `body_scope` block, `command` is a scoped-command row rather
than a pack-level command declaration — the same context rule that makes
`method` a member row inside `object_class`. Its nested `subcommand`
blocks are ordinary `subcommand` bodies, reused unchanged, because
`ScopedCommand::subcommands` really is `&[SubCommand]`.

`body_scope` is **spelled, not ported**: its shape was read off the two
shipped environments (`REPORT_DEFSTYLE_ENV` and `TCLPKG_MANIFEST_ENV`,
`rust/tcl-registry/src/scoped.rs:337` and `:479`), not driven by a port,
and the fidelity table records it as such. That is a weaker warrant than
the `definition_body` form has, and it is the honest status: the rubric
asks for expressibility from day one, and expressible-from-plain-data is
what this is.

## Ambiguities resolved, and roads not taken

**The schema key is the property word — except for row lists.**
spec-packs.md promises "that key is the DSL property name". Held for
every scalar field, which is where new fields land. A field holding a
list of rows instead gets a **singular row statement** (`options` →
`option`, `subcommands` → `subcommand`, `forms` → `form`,
`side_effects` → `side_effect`, `setter_constraints` →
`setter_constraint`, `repeated_args` → `repeat`, `manufacturer_methods`
→ `manufacturer`, `option_constraints` → `option_conflict`,
`oo_context_facts` → `oo_context_fact`, `sub_subcommands` →
`sub_subcommand`, `versioned_arg_values` → `versioned_arg_value`), and a
new field on a *row type* becomes a new flag on that statement — so the
tolerance rule still works in both directions. *Rejected:* a literal
`options { … }` block per key. It nests one level deeper for no gain and
makes every option a two-line edit.

**Per-index facts are one row, not six tables.** `arg N -role … -type …`
merges six schema keys. *Rejected:* one statement per key
(`arg_role 0 Body`, `arg_type 0 List`, …), which is faithful to the
schema and unreadable — `string is`'s single class argument would take
three statements instead of one, and `lsort`'s `-command` would split its
callback position from its appended arity.

**Emitter verbs, not return values.** *Rejected:* "the hook returns a
dict of results". A returned dict makes abstention (`{}`) look exactly
like an empty answer, and makes a multi-emit hook build a list by hand.
The verb form makes falling off the end the safe default and reads like
the Tcl people already write.

**One protocol for every hook family, and abstention is per-field
conservative.** The temptation is a uniform "no answer = no opinion".
That is wrong for `taint_sink_gate`, where silence must keep the security
finding alive. Silence is defined per field in the table above.

**Descriptors are declarative wherever they are plain data.** The
temptation is to treat every `RustExpr` field in the studio schema as
un-authorable. Most of them are plain data that the studio simply edits
as one text box: `frame_effect` is two closed enums, `event_requires` is
eight scalars, `case_list` thirteen, `binds_handle` three,
`defines_symbol` four, `byte_array_payload` two. Porting `upvar` and
`HTTP::header` is what surfaced this — both look like hard cases in the
`.rs` and are trivial in the DSL.

**Derived hooks beat written hooks.** The nine ports contain thirteen
function-pointer hook uses. Four of them need **no code at all** once the
data they read is declared — `if`'s two come from `clause_grammar`,
`oo::class`'s role resolver from its `manufacturer` rows, and `upvar`'s
state-transition resolver from its `frame_effect`. Five become Tcl
bodies, and four stay `-native`. Each derivation is an explicit one-word
opt-in (`from-manufacturers`, `from-frame-effect`), never an implicit
consequence of declaring something else — a spec that silently grows
behaviour when you add a row is worse than one that makes you say so.

**Brace-quoted prose, with a known sharp edge.** Prose and examples are
braced words, so backslashes and `$`/`[` are literal and newlines
survive. The edge: a braced word must have balanced braces. Text with a
lone `{` needs the quoted form and a backslash — ordinary Tcl quoting,
but it is the one place the format will bite an author writing about
Tcl syntax. *Rejected:* a heredoc form. It buys one rare case and costs
the property that a pack is an ordinary Tcl script.

**Newlines in a braced word are significant, and the loader does not
reflow.** The value of a braced word is *every byte between its braces*,
verbatim: leading and trailing newlines, interior blank lines, and the
author's indentation are all part of the string. Nothing is stripped,
joined, dedented, or wrapped. This has to be pinned rather than left to
taste because the equivalence gate compares the loaded field with the
compiled `&'static str` **byte for byte**, so a wrap-happy author who
breaks a long `description` across two lines to fit 80 columns has
changed the value and will fail the gate — the DSL cannot tell that
newline from a deliberate one. Two consequences worth stating
explicitly:

- **Prose is one long line, or it is not the same string.** Every ported
  `description` / `detail` / `returns` in this directory is a single
  physical line, however long, because its `.rs` original is.
- **Repeated `example` rows join with a single `\n`.** A spec whose Rust
  `examples` string separates its examples with a *blank* line therefore
  writes them as one braced block rather than three rows —
  [`return.tclspec.tcl`](return.tclspec.tcl) is the worked case, and says
  so at the site.

An author who wants soft-wrapped source can still have it: the escape is
ordinary Tcl line continuation inside a *quoted* word, which is a
different (and lossier) spelling, not a second meaning for braces.

## Fidelity of the ports

What each port loses, if anything, against its `.rs`.

| port | fidelity | what is missing |
|---|---|---|
| `lsort` | **complete** | — |
| `foreach` | **complete** | the resolver's `u8::try_from` guard becomes the loader's index cap, which is the same behaviour stated once instead of per hook |
| `switch` | **complete** | — |
| `if` | **complete, and smaller** | two hook functions (~110 lines of Rust) become a three-line grammar; the derived walk agrees with `walk_if` on its whole test matrix |
| `string` (4 subcommands) | **near-complete** | `string is`'s `const_fold_versioned` stays `-native`. Its Rust is a version-aware classifier (per-class availability floors, 8.x/9.x magnitude caps, radix prefixes, digit separators, ambiguous-form bail-outs); a Tcl body would be a re-implementation, not a port. `length` / `map` / `range` port fully — but see the note below. |
| `oo::class` | **partial** | the three subcommands' `state_transitions` resolvers stay `-native`: they emit typed `CommandBinding::Define` + `ObjectDispatch::Create` facts, and the DSL has no vocabulary for constructing transition facts. Everything around them (composition, argument shape, widening rules, effect coverage, commit) is data and ports. `arg_role_resolver` is *removed*, derived from the `manufacturer` rows — a **derivation claim**, not a transcription, and one that must be **proved, not assumed**: `oo_class_arg_roles` (`oo_class.rs:245-256`) reads `TCLOO_GRAMMAR.manufacturers`, while the port's rows are `TCLOO_ROOT_CLASS_MANUFACTURERS`. The two tables differ on `new`'s visibility and agree on every keyword and body index *today*, which is what makes the derivation correct now and not correct by construction. A loader must diff them; a future divergence must fail the equivalence gate. |
| `uri::geturl` + `http::geturl` | **complete** | — |
| `HTTP::header` | **complete** | including the shipped spec's `credential_arg 2` on `insert`/`replace`, carried verbatim. Design review verified it is **not** an off-by-one: the W310 consumer (`security.rs::emit_w310_hardcoded_credentials`) indexes with the *subcommand word at 0* (`args[0]` = `insert`, `args[1]` = header name, `args[2]` = value), so `2` is the value slot as consumed — the schema help text's "index after the subcommand word" is the erroneous half. A loader must store this field verbatim, **not** re-base it to after-subcommand coordinates |
| `upvar` | **near-complete** | `state_transitions.resolver` becomes `from-frame-effect`. That is a *derivation claim*, not a transcription: it asserts that the alias facts `upvar_state_transitions` produces are exactly determined by `AliasPairs` + `ArityParity`. Reading the Rust, they are — but an implementation must prove it, not assume it, and must match the two abstention policies pinned under "Derivations, exactly". |
| `snit::type` + `SNIT_GRAMMAR` | **complete** | all 14 `DefinitionBodyGrammar` fields, 15 member rows, 6 built-in object methods, the `install` member-body command with its handle binding, and the single `create` manufacturer, transcribed field for field. The one field that is a function pointer in the Rust — `bare_word_construction_hint` — becomes data (`-hint-values {%AUTO%} -hint-prefixes {.}`), which is a transcription of a two-clause boolean, not a derivation. Deliberate omissions, each equal to the Rust default: `member_body_namespace_path` (`&[]`), `builtin_terminating_methods` (`&[]`), `unknown_dispatch_method` (`None`), `property_accessor_methods` (`&[]`). `snit::widget` / `snit::widgetadaptor` are **not** ported: they carry `SNIT_WIDGET_GRAMMAR`, a second constant differing in `implicit_vars` and `member_body_commands` only. |
| `return` | **complete** | including the option-arity hook: `errorstack_value`'s four outcomes port one-for-one, with `string is list` standing in for `split_list_raw`'s `Ok`/`Err` because the sandbox has no `catch` (the two agree on which words are lists). `arg_role_resolver` and `context_gate` are Tcl bodies; `lowering_hook` / `inline_codegen_hook` stay `-native`, which is the closed-catalogue rule, not a loss. The three-example `examples` string is one `example` block, not three rows — see the newline rule. |

**The const-fold porting hazard.** A Tcl hook body inherits the *hook
interpreter's* Tcl semantics, and the registry's folders deliberately
under-approximate Tcl. `const_fold::split_list` is documented as a
fold-safety splitter that bails on any backslash or bare brace so the
optimiser only folds provably-simple lists; a Tcl body using `llength` /
`string map` gets the real, more permissive list grammar and would fold
*more* than the shipped folder does — changing optimiser output while
looking like a faithful port. `string map`'s port re-adds the backslash
guard by hand and says so in a comment. A residual gap remains for a
mapping containing a mid-word `"` or brace (`a"b X`, `a}b X`,
`a{b}c X`): Tcl's list grammar takes those literally, while
`split_list`'s bare-word scan bails on any of `\` `{` `}` `"`
(`const_fold.rs:112`), so the Tcl body folds calls the shipped folder
abstains on. Any equivalence gate must therefore
compare *folder outputs over a corpus*, not just spec fields; this is the
one place where "the DSL says the same thing" is not the same as "the DSL
does the same thing".

## Rulings on the census drafts

The four external drafts under [`external/`](external/) were written to
find out what a real private-library author would need to say. Doing
that surfaced places where a draft reached for a spelling this memo had
not settled, or had settled differently. Each is ruled here **once**, and
the drafts have been reconciled to match; until a ruling exists, a draft
is evidence about the *registry*, never an exemplar of the *syntax*.

| the drafts wrote | ruling | why |
|---|---|---|
| `object_class { superclasses {…} allow_unknown_methods no  method … }` | **`object_class NAME ?-superclass {…}? ?-allow-unknown? { method … }`** | `ObjectClassSpec` has four fields, three of which are one word each; a block with three one-word rows is a row wearing a block's clothes. The `NAME` word is `class_name`, which the drafts left implicit and which is *not* always the command name (a factory command may manufacture a differently-named class). |
| `option NAME -since VERSION` | **`-introduced VERSION`** | the option row already carries the four lifecycle flags `-introduced` / `-deprecated` / `-retired` / `-deprecation-fix`, named for `Lifecycle`'s own fields. `-since` is a second word for the first of them. The drafts' underlying point stands and is unaffected: the version axis here is the *library's*, not Tcl's, which is exactly what `Lifecycle` is documented as being ("orthogonal to `dialects`"). |
| `option_constraints { forbid {A B} … requires {A B} {C} }` | **`option_conflict {-a -b}`** rows for the `forbid` half; the `requires` half is a **known limit** (below), not syntax | `OptionConstraint` is a flat may-not-co-occur set with no directionality, so `forbid` maps and `requires` has nothing to map to. Writing `requires` inside an otherwise-backed block made an unbacked idea look one flag away from working; as a known limit it is visibly a registry gap. |
| `sub_subcommands { sub_subcommand … }` | **`sub_subcommand NAME …` rows** | the singular-row rule, unchanged: `sub_subcommands` is a list of rows, so it gets a row statement, exactly like `options` → `option`. |
| `completion { codes {…} custom_code 5 … }` | **removed** — `completion` is excluded | writing syntax for an excluded field is the one thing a draft must not do, however well marked: it reads as a proposal to un-exclude. The real content (a library-defined code scoped to one command's body) is recorded as a known limit. |
| `arg N -detail {…}` | **removed from the drafts** — there is no field to hold it | this was an *unmarked* invention in all four drafts, which is what makes it worth a ruling. No per-argument prose field exists on `CommandSpec` or `SubCommand`; argument documentation reaches the user through `synopsis` (inlay parameter names, signature help) and through `hover`'s `description`. *Rejected:* adding one. It would be a new registry field, so it is a contribution and an issue, not a DSL spelling — and the census's own rule is that every invention is marked. |
| repeated bare `synopsis` rows on a `method` | **one `synopsis` per subcommand/method; use `hover { synopsis … }` for the rest** | `SubCommand::synopsis` is a single `Text`. The repeatable `synopsis` row exists inside a `hover` block, where `HoverSnippet::synopsis` really is a list. A draft that repeated the subcommand-level row was writing hover data in a field that holds one string. |

One draft question the census raised and could not answer is answered
here, because it is a modelling question rather than a spelling one:
**where the constructor's argument shape lives for a class command.**
The drafts put arity and `arg` rows for `Resistor new name np nm`
directly on the class command, which cannot be right — word 0 of that
call is `new`, not `name`. The shipped answer already exists and is
exactly the shape `ticklecharts/mod.rs:409-457` uses: the class command
takes `arity 1..` (a method word plus its arguments) and
`allow_unknown_subcommands`; the constructor's shape lives on
`subcommand create` / `subcommand new`; and the `manufacturer` rows say
which word names the instance (`-names-instance-at`) and where the
constructor's own arguments begin (`-constructor-args-from`). The
drafts now do that.

## Known limits carried forward

Things the design cannot say, kept as a register so they are not
rediscovered as bugs. Each is real, each is grounded in a corpus
exemplar or a shipped struct, and none blocks the freeze.

| limit | evidence | why it is a limit |
|---|---|---|
| Per-flag-combination return typing | tarray column search: the returned representation depends on which *combination* of mode/shape flags is present | `return_type` is one value per command/subcommand, and even the excluded `command_forms` splits by form, not by flag combination |
| Library-defined completion codes | `struct::tree::prune`'s `return -code 5`, meaningful only inside `struct::tree walk`'s body | `completion` is excluded, and the authorable traits name only break/continue/raise. Nothing pairs a custom code to the single command whose body accepts it, the way `HAS_LOOP_BODY` pairs with `BREAKS_LOOP` |
| Option *requirement* relationships | SpiceGenTcl's `argparse` blocks: 189 `-require` vs 75 `-forbid` | `OptionConstraint` is a flat mutual-exclusion set with no direction. `option_conflict` covers the smaller half of what real libraries write |
| Method-scoped taint sinks | SpiceGenTcl's `Batch::runAndRead` — an `exec` inside a TclOO instance method | `taint_code_sink_args` / `taint_network_sink_args` are command-only fields; the methods that actually spawn processes are `SubCommand`-shaped and cannot declare them |
| Embedded-language naming beyond the closed catalogues | tDOM's XPath argument; ticklecharts' JS/G6 fragments | `pattern_type` and `format_string_type` are closed catalogues (Glob/Regex; Sprintf/Clock/Binary/Regsub) a pack cannot extend |
| Callback *callee-shape* declaration | tclvfs's 9-shape `vfshandler` callback | on the registering command, only `-appends {AtLeast N}` / `Unknown` is available. Mitigated, and well: spec the handler proc itself as a `command` with nine `subcommand`s, which is fully expressible |
| Runtime-assembled ensembles | `namespace ensemble create -map` where the map splices same-file procs | a pack declares commands, not the runtime construction of them |
| Runtime-computed argument grammars | SpiceGenTcl's computed `argparse` definitions; mustache's position-dependent lambda arity | irreducibly dynamic — `Unknown` is the honest answer for any spec system, native included |

Two of these (`requires`, method-scoped taint sinks) are registry
changes rather than DSL changes, and both are worth filing; the rest are
statements about what a static description of a command can be.

## Coverage matrix

Every key of both tables in `rust/tcl-spec-studio/src/schema.rs`, in
schema order. "excluded" rows carry the reason.

| `CommandSpec` key | DSL spelling | notes |
|---|---|---|
| `name` | `command NAME { … }` | the statement's own name word; `-override` claims a shipped name |
| `traits` | `traits {TRAIT …}` | trait words verbatim from the traits vocabulary; repeatable, unioned |
| `dialects` | `dialects {SET …}` | dialect members verbatim, plus the `tclX.Y+` and `all-tcl` / `tcl8.x` set words |
| `arity` | `arity N`, `N..M`, `N..`, `..M`, `..` ?-step S? ?-also N? | the `..` range word is the whole `Arity` struct |
| `arg_roles` | `arg N -role ROLE` |  |
| `arg_role_resolver` | `arg_role_resolver {words ctx} { … }` \| `-native ID` \| `from-manufacturers` | also **derived** from `clause_grammar`; emitter verb `role IDX ROLE` |
| `arg_presentation` | `arg N -layout BlockScript\|InlineScript` |  |
| `repeated_args` | `repeat ROLE -from N -stride N ?-exclude-trailing N? ?-optional-leading? ?-conditional?` | one row per layout |
| `frame_effect` | `frame_effect -level-word W -layout L` | both payloads are closed enums, so fully declarative |
| `clause_shape_check` | **derived** from `clause_grammar`; `-native ID` escape | no hook body needed for any chain the grammar can spell |
| `command_prefixes` | `arg N -appends {Exactly 2}` | implies `-role CommandPrefix` |
| `command_prefix_resolver` | `command_prefix_resolver {words ctx} { … }` \| `-native ID` | emitter verb `prefix IDX {Exactly N}` |
| `return_type` | `return_type T` |  |
| `var_write_typing` | `var_write_typing ReturnValue\|Destructured\|{Fixed T}\|{ElementsOf N}` | variant word + positional payload |
| `return_elements` | `return_elements {VARIANT payload …}` | same rule |
| `var_elements_effect` | `var_elements_effect {VARIANT payload …}` | same rule |
| `representation_effect` | `representation_effect {VARIANT payload …}` | same rule |
| `arg_types` | `arg N -type T ?-shimmers? ?-transparent {T …}?` |  |
| `subcommands` | `subcommand NAME { … }` | one block per subcommand |
| `allow_unknown_subcommands` | `allow_unknown_subcommands ?yes\|no?` |  |
| `prefix_matching` | `prefix_matching Enabled\|Strict` |  |
| `default_form_first_word` | `default_form_first_word Integer` |  |
| `hover` | `hover { … }` | block; see the hover statements below |
| `forms` | `form KIND {synopsis} ?-dialects {…}?` | one row per form |
| `command_forms` | **excluded** | per-form arity/roles/options/hook bundles; the studio carries it as one opaque Rust expression and `forms` covers the getter/setter split every pack has needed. A command needing it is a contribution. |
| `semantic_operation` | `semantic_operation Invoke\|{Intrinsic ID}\|{StructuredLowering ID}` | an operation identity, so it keeps the enum spelling rather than `-native` |
| `completion` | **excluded** | `CompletionDescriptor` describes the command's *control-flow edges*, so a wrong value corrupts the CFG rather than one value — see "Why `completion` is excluded and `const_fold` is not". The traits `BREAKS_LOOP` / `CONTINUES_LOOP` / `CATCHABLE_THROW` stay authorable and cover the standard codes |
| `assigns_variable_at` | `assigns_variable_at N` |  |
| `safe_on_uninit` | `safe_on_uninit {SET …}` |  |
| `const_fold` | `const_fold {words ctx} { … }` \| `-native ID` | emitter verb `fold VALUE`; no call = no fold |
| `const_fold_versioned` | `const_fold_versioned {words ctx} { … }` \| `-native ID` | same, with `tcl-version` in `ctx` |
| `lowering_hook` | `lowering_hook -native ID` | closed catalogue |
| `codegen_hook` | `codegen_hook -native ID` | closed catalogue |
| `inline_codegen_hook` | `inline_codegen_hook -native ID` | closed catalogue |
| `bpf_op` | `bpf_op -native ID` | BPF dialect only; reference-only |
| `analyser_hook` | `analyser_hook -native ID` | closed catalogue |
| `command_table_effect` | `command_table_effect DefinesProcedure\|RenamesCommands\|CreatesAliases` |  |
| `side_effects` | `side_effect TARGET ?-reads? ?-writes? ?-side S? ?-dialects {…}?` | one row per effect |
| `world_effects` | `world_effects none\|NAME\|{ … }` | block carries composition / access / callback / dynamic_fallback; `resolver` is reference-only |
| `state_transitions` | `state_transitions NAME\|{ … }` | block carries composition / argument_shape / widen / covers / commit; `resolver` takes `none`, `from-frame-effect`, or `-native ID` |
| `dispatch_dependencies` | **excluded** | specialisation-proof machinery whose meaning is defined by the optimiser, not by the command; fields.md itself says "leave unset" |
| `result_stability` | `result_stability Unknown\|ReferentiallyTransparent\|Volatile\|{ReadsVersionedWorld {D …}}` |  |
| `literal_argument_validator` | `literal_argument_validator {words ctx} { … }` \| `-native ID` | emitter verbs `invalid …` / `abstain REASON`; no call = valid |
| `inferred_storage_type` | `inferred_storage_type Dict\|List\|Array` |  |
| `required_package` | `required_package NAME` | also settable pack-wide with `default` |
| `excluded_events` | `excluded_events {EVENT …}` |  |
| `unsafe_command` | `unsafe_command ?yes\|no?` |  |
| `closed_value_args` | `arg N -closed` |  |
| `event_requires` | `event_requires NAME\|{ … }` | block of the eight plain-data fields |
| `event_requirement_forms` | `event_requirement_form {word …} ?-only-in {E …}? ?{ … }?` | trailing block is a nested `event_requires` |
| `data_collection` | `data_collection -native ID` | reference-only: the collect/release descriptor is paired with protocol machinery outside the registry |
| `side_switch_target` | `side_switch_target Client\|Server` |  |
| `event_handler_priority` | `event_handler_priority -default N ?-warn-implicit?` |  |
| `options` | `option NAME ?-flag value? …` | one row per option; see the option flag table |
| `option_constraints` | `option_conflict {-a -b} ?-dialects {…}?` | one row per constraint |
| `reserved_trailing_words` | `reserved_trailing_words N` |  |
| `arg_values` | `arg N -values {v …}` \| `arg N -values-from NAME` | `values NAME { … }` declares the shared table |
| `body_kind` | `body_kind Plain\|Structural` |  |
| `body_arg_implicit_args` | `body_arg_implicit_args N` |  |
| `taint_output_sink` | `taint_output_sink CODE` |  |
| `taint_output_sink_subcommands` | `taint_output_sink_subcommands {NAME …}` |  |
| `taint_log_sink` | `taint_log_sink CODE` |  |
| `taint_network_sink_args` | `taint_network_sink_args {N …}` | tri-state: absent = unset, `{}` = declared empty |
| `taint_code_sink_args` | `taint_code_sink_args {N …}` | same tri-state |
| `taint_interp_eval_subcommands` | `taint_interp_eval_subcommands {NAME …}` |  |
| `taint_source` | `taint_source {COLOUR …}` |  |
| `taint_transform` | `taint_transform {COLOUR …}` |  |
| `taint_double_encode_colour` | `taint_double_encode_colour {COLOUR …}` |  |
| `taint_sink_safe_colour` | `taint_sink_safe_colour {COLOUR …}` |  |
| `taint_sink_gate` | `taint_sink_gate {words ctx} { … }` \| `-native ID` | emitter verbs `sink-applies` / `sink-suppressed`; no call = **applies** |
| `credential_options` | `credential_options {-flag …}` |  |
| `sensitive_headers` | `sensitive_headers {NAME …}` |  |
| `setter_constraints` | `setter_constraint N -prefix P -code CODE -message {…}` | one row per constraint |
| `pattern_type` | `pattern_type Glob\|Regex` |  |
| `format_string_type` | `format_string_type Sprintf\|Clock\|Binary\|Regsub` |  |
| `tcllib_package` | `tcllib_package NAME` |  |
| `introduced_version` | `introduced_version V` | `Lifecycle.introduced` |
| `deprecated_version` | `deprecated_version V` | `Lifecycle.deprecated` |
| `retired_version` | `retired_version V` | `Lifecycle.retired` |
| `deprecation_fix` | `deprecation_fix -replace WORD -description {…} -safety S` | `Lifecycle.deprecation_fix`; the contextual-callback variant is reference-only |
| `warn_missing_import` | `warn_missing_import ?yes\|no?` |  |
| `is_namespace_exported` | `is_namespace_exported ?yes\|no?` |  |
| `xc_translatable` | `xc_translatable yes\|no` | argument required — absent means unset |
| `xc_operation` | `xc_operation NAME` |  |
| `deprecated_replacement` | `deprecated_replacement NAME` |  |
| `deprecated_replacement_drop_in` | `deprecated_replacement_drop_in ?yes\|no?` |  |
| `byte_array_payload` | `byte_array_payload -replace-data-index N ?-message-flag-shift?` |  |
| `byte_array_effect` | `byte_array_effect None\|Transparent\|Coerces\|CaseFolds\|Encodes\|{Rebinarifies N}` |  |
| `definition_body` | `definition_body NAME\|{ … }` | a shipped grammar by name (`tcloo`, `tcloo-configurable`, `snit`, `snit-widget`, `itcl`), a pack `descriptor`, or the inline block — see "Definer grammars and scoped bodies" |
| `manufacturer_methods` | `manufacturer KEYWORD ?-unexported? ?-names-instance-at N? ?-definition-body-at N? -constructor-args-from N` | one row per method |
| `case_list` | `case_list NAME\|{ … }` | `switch` / `expect` by name, or the 13 plain-data fields inline |
| `oo_context_facts` | `oo_context_fact WORD FACT` | one row per fact |
| `self_receiver_words` | `self_receiver_words {WORD …}` |  |
| `object_class` | `object_class NAME` \| `object_class NAME ?-superclass {…}? ?-allow-unknown? { method … }` | `method` rows reuse the `subcommand` body grammar |
| `defines_symbol` | `defines_symbol -name-arg N ?-detail-arg N? ?-requires-arg N? -kind KIND` |  |
| `body_scope` | `body_scope NAME\|{ … }` | a shipped environment by name, a pack `descriptor`, or the inline block; **spelled from `ScopedCommandEnv`, not exercised by a port** |
| `binds_handle` | `binds_handle -name-from {Word N} -class-from {Word N} ?-keyword {N WORD}?` |  |
| `creates_instance_at` | `creates_instance_at N` |  |
| `defines_command_at` | `defines_command_at N` |  |
| `context_gate` | `context_gate {words ctx} { … }` \| `-native ID` | emitter verb `reject MESSAGE`; `ctx` carries `in-event-body` |
| `implementation_namespace` | `implementation_namespace NS` |  |

| `SubCommand` key | DSL spelling | notes |
|---|---|---|
| `name` | `subcommand NAME { … }` | the statement's own name word |
| `traits` | `traits {TRAIT …}` | trait words verbatim from the traits vocabulary; repeatable, unioned |
| `arity` | `arity N`, `N..M`, `N..`, `..M`, `..` ?-step S? ?-also N? | counted after the subcommand word |
| `detail` | `detail {…}` |  |
| `synopsis` | `synopsis {…}` |  |
| `hover` | `hover { … }` | block; see the hover statements below |
| `arg_roles` | `arg N -role ROLE` |  |
| `arg_role_resolver` | `arg_role_resolver {words ctx} { … }` \| `-native ID` \| `from-manufacturers` | also **derived** from `clause_grammar`; emitter verb `role IDX ROLE` |
| `arg_presentation` | `arg N -layout BlockScript\|InlineScript` |  |
| `repeated_args` | `repeat ROLE -from N -stride N ?-exclude-trailing N? ?-optional-leading? ?-conditional?` | one row per layout |
| `command_prefixes` | `arg N -appends {Exactly 2}` | implies `-role CommandPrefix` |
| `command_prefix_resolver` | `command_prefix_resolver {words ctx} { … }` \| `-native ID` | emitter verb `prefix IDX {Exactly N}` |
| `return_type` | `return_type T` |  |
| `var_write_typing` | `var_write_typing ReturnValue\|Destructured\|{Fixed T}\|{ElementsOf N}` | variant word + positional payload |
| `return_elements` | `return_elements {VARIANT payload …}` | same rule |
| `var_elements_effect` | `var_elements_effect {VARIANT payload …}` | same rule |
| `representation_effect` | `representation_effect {VARIANT payload …}` | same rule |
| `arg_types` | `arg N -type T ?-shimmers? ?-transparent {T …}?` |  |
| `pure` | `pure ?yes\|no?` |  |
| `mutator` | `mutator ?yes\|no?` |  |
| `const_fold` | `const_fold {words ctx} { … }` \| `-native ID` | emitter verb `fold VALUE`; no call = no fold |
| `const_fold_versioned` | `const_fold_versioned {words ctx} { … }` \| `-native ID` | same, with `tcl-version` in `ctx` |
| `lowering_hook` | `lowering_hook -native ID` | closed catalogue |
| `codegen_hook` | `codegen_hook -native ID` | closed catalogue |
| `inline_codegen_hook` | `inline_codegen_hook -native ID` | closed catalogue |
| `analyser_hook` | `analyser_hook -native ID` | closed catalogue |
| `command_table_effect` | `command_table_effect DefinesProcedure\|RenamesCommands\|CreatesAliases` |  |
| `options` | `option NAME ?-flag value? …` | one row per option; see the option flag table |
| `option_constraints` | `option_conflict {-a -b} ?-dialects {…}?` | one row per constraint |
| `min_abbrev` | `min_abbrev N` |  |
| `prefix_matching` | `prefix_matching Enabled\|Strict` |  |
| `arg_values` | `arg N -values {v …}` \| `arg N -values-from NAME` | `values NAME { … }` declares the shared table |
| `versioned_arg_values` | `versioned_arg_value N VALUE ?-introduced V? ?-deprecated V? ?-retired V?` | one row per gate |
| `subcommand_forms` | **excluded** | the subcommand-level twin of `command_forms`, excluded for the same reason |
| `semantic_operation` | `semantic_operation Invoke\|{Intrinsic ID}\|{StructuredLowering ID}` | an operation identity, so it keeps the enum spelling rather than `-native` |
| `completion` | **excluded** | `CompletionDescriptor` describes the command's *control-flow edges*, so a wrong value corrupts the CFG rather than one value — see "Why `completion` is excluded and `const_fold` is not". The traits `BREAKS_LOOP` / `CONTINUES_LOOP` / `CATCHABLE_THROW` stay authorable and cover the standard codes |
| `dialects` | `dialects {SET …}` | absent inherits the parent command's set |
| `introduced_version` | `introduced_version V` | `Lifecycle.introduced` |
| `deprecated_version` | `deprecated_version V` | `Lifecycle.deprecated` |
| `retired_version` | `retired_version V` | `Lifecycle.retired` |
| `deprecation_fix` | `deprecation_fix -replace WORD -description {…} -safety S` | `Lifecycle.deprecation_fix`; the contextual-callback variant is reference-only |
| `safe_on_uninit` | `safe_on_uninit {SET …}` |  |
| `loop_list_header` | `loop_list_header ?yes\|no?` |  |
| `creates_scope_alias` | `creates_scope_alias ?yes\|no?` |  |
| `inferred_storage_type` | `inferred_storage_type Dict\|List\|Array` |  |
| `body_kind` | `body_kind Plain\|Structural` |  |
| `byte_array_effect` | `byte_array_effect None\|Transparent\|Coerces\|CaseFolds\|Encodes\|{Rebinarifies N}` |  |
| `closed_value_args` | `arg N -closed` |  |
| `arg_values_accept_prefix` | `arg_values_accept_prefix ?yes\|no?` |  |
| `body_arg_implicit_args` | `body_arg_implicit_args N` |  |
| `taint_transform` | `taint_transform {COLOUR …}` |  |
| `taint_double_encode_colour` | `taint_double_encode_colour {COLOUR …}` |  |
| `taint_output_sink` | `taint_output_sink CODE` |  |
| `credential_arg` | `credential_arg N` | **verbatim coordinates**: the W310 consumer counts the subcommand word itself as index 0, unlike every other per-index subcommand field — see the `HTTP::header` fidelity note |
| `sensitive_headers` | `sensitive_headers {NAME …}` |  |
| `pattern_type` | `pattern_type Glob\|Regex` |  |
| `format_string_type` | `format_string_type Sprintf\|Clock\|Binary\|Regsub` |  |
| `xc_operation` | `xc_operation NAME` |  |
| `side_effects` | `side_effect TARGET ?-reads? ?-writes? ?-side S? ?-dialects {…}?` | one row per effect |
| `world_effects` | `world_effects none\|NAME\|{ … }` | block carries composition / access / callback / dynamic_fallback; `resolver` is reference-only |
| `state_transitions` | `state_transitions NAME\|{ … }` | block carries composition / argument_shape / widen / covers / commit; `resolver` takes `none`, `from-frame-effect`, or `-native ID` |
| `dispatch_dependencies` | **excluded** | specialisation-proof machinery whose meaning is defined by the optimiser, not by the command; fields.md itself says "leave unset" |
| `result_stability` | `result_stability Unknown\|ReferentiallyTransparent\|Volatile\|{ReadsVersionedWorld {D …}}` |  |
| `literal_argument_validator` | `literal_argument_validator {words ctx} { … }` \| `-native ID` | emitter verbs `invalid …` / `abstain REASON`; no call = valid |
| `destructive` | `destructive ?yes\|no?` |  |
| `returns_path` | `returns_path ?yes\|no?` |  |
| `is_unescape` | `is_unescape ?yes\|no?` |  |
| `cfg_rewrite_name` | `cfg_rewrite_name NAME` |  |
| `sub_subcommands` | `sub_subcommand NAME ?-detail {…}? ?-synopsis {…}? ?-dialects {…}?` | one row per second-level word |
| `defines_command_at` | `defines_command_at N` |  |
| `max_leading_option_words` | `max_leading_option_words N` |  |
