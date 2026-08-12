# The tricky surfaces the DSL must cover

The acceptance rubric for the spec-pack DSL. Every item names a real Tcl
behaviour the shipped registry already models; the DSL is not done until
each is expressible, and the design review ticks these off against the
ported examples rather than against intent.

## Operators, math functions, and implementation namespaces

- `::tcl::mathop::*` — commands that *are* operators (`OPERATOR_COMMAND`:
  excluded from completion, exempt from W113). The DSL must declare an
  operator command and its `expr`-operator twin.
- `::tcl::mathfunc::*` — `expr` functions as commands; a pack must be
  able to declare new expr functions and operators (stubs already spell
  `expr-func` / `expr-op` — the DSL is their superset).
- Ensemble implementation namespaces: `string length` ↔
  `::tcl::string::length` (`implementation_namespace`, the W143
  private-call diagnostic and its public-spelling fix). Equivalents for
  `dict`, `array`, `binary`, `info`. A pack must also express ensembles
  users assemble at runtime (`namespace ensemble create -map -prefixes
  -unknown`), where the map splices same-file procs.

## TclOO, every corner

- Definer grammars: `method` / `constructor` / `destructor` /
  `variable` / `forward` / `filter` / `mixin` / `superclass` /
  `unexport`, the three member kinds (flat, wrapper — itcl's
  `public`/`protected`, flag-keyed — `oo::configurable`'s `property`,
  which is also 9.0-gated *per member*), implicit member-body variables,
  member-body-only commands (snit's `install`).
- Both definition spellings: the body form (`oo::define C { … }`) and
  the inline form (`oo::define C method …`) — the inline form is
  currently gated on an analyser hook, a known generic-coverage gap the
  DSL must not inherit.
- Object machinery: manufacturer methods (`new`/`create` and custom
  factories), `object_class` instance methods with superclass
  resolution and `allow_unknown_methods`, abstract factories (W250),
  `self` receiver words, `oo_context_facts` (`self class` folding),
  handle binding (`set o [Class new …]`, snit's `install x using …`),
  `creates_instance_at`, `my` / `next` / `nextto` dispatch traits,
  method-alias binders (`callback` / `mymethod`-style prefix builders),
  `forward` targets.

## Options as they are really used

- The `--` terminator as an *option*: W304 and T102 exist only when
  declared; `reserved_trailing_words` exempts trailing structural words
  (`switch`'s string + pattern list).
- Options that change other arguments: `-regexp`/`-glob` selecting the
  pattern language of a *later* word; `-command` implying a callback
  with appended arity; `-index` changing index interpretation; mutual
  exclusion sets (`option_constraints`); options whose *value* carries a
  role (`-textvariable` reads and writes) or a closed/integer domain;
  fixed multi-word values; per-option dialect and lifecycle gates;
  abbreviation rules (`min_abbrev`, strict tables, aliases like `-bg`);
  `max_leading_option_words`; trailing-option commands (Tk `configure`).
- The one shipped option-arity hook (`return -errorstack`) as the
  worked example of a hook *inside* an option row.

## Arity beyond min–max

- Paired and n-paired tails: stepped arity with an extra exact count
  (`array set`), strided repeats (`global` stride 1, `variable` stride
  2, `foreach` var/list pairs with the body excluded from the tail).
- Clause grammars: `if`'s `elseif`/`else` rhythm declaratively
  (`STRUCTURALLY_CHECKED_ARITY` + shape), `switch` / `expect` case
  lists with their mode options and keyword patterns.
- Dynamic arity that needs a resolver: `set` read-vs-write, `dict with`
  leading keys + trailing body, count-dependent layouts.
- `{*}` expansion semantics (arity abstains on min, still enforces
  max), `default_form_first_word` (`after 200 …`),
  `body_arg_implicit_args`.

## Analysis facts and hooks

- Typing: per-subcommand return types, written-variable typing
  (`scan`/`lassign` destructuring), element-structure facts, storage
  kinds, shimmer hints with `transparent_from`, byte-array effects and
  payload layouts, representation effects.
- Effects: structured side effects, world effects and state transitions
  (where unset means "assume anything"), frame effects (`upvar`,
  `uplevel`, `namespace upvar`), result stability, context gates
  (`return -code` only in a proc; top-level-only commands).
- Completion codes, **as amended**: a pack must be able to say that a
  command breaks a loop, continues a loop, or always raises — the traits
  `BREAKS_LOOP` / `CONTINUES_LOOP` / `CATCHABLE_THROW` on the command
  that performs it, paired with `HAS_LOOP_BODY` on the command that
  accepts it. All four are authorable. The `completion` field itself (`CompletionDescriptor`)
  stays **excluded**, because it describes control-flow *edges* and a
  wrong value corrupts the CFG rather than one value; see the DSL memo's
  "Why `completion` is excluded and `const_fold` is not". A
  library-defined code scoped to one command's body — `struct::tree`'s
  `return -code 5`, meaningful only inside `struct::tree walk`
  (`tree_tcl.tcl:181-183`, consumed at `tree_tcl.tcl:2109-2134`) — fits
  neither the traits nor the excluded field, and is recorded as a known
  limit rather than claimed. This line was rewritten during the
  pre-freeze review: as originally written the rubric demanded a field
  the design excludes, so the design could not have passed its own gate.
- Folding: const-folders including the degenerate "run the pure Tcl
  implementation on literal args" case; versioned folders.
- Every hook calling convention must state inputs, output protocol,
  abstention, and error-means-abstain.

## Documentation, fixes, and editor surface

- Hover at all three levels (command / subcommand / sub-subcommand),
  synopsis-driven signature help and inlay parameter names, completion
  `detail` strings, form synopses feeding the arity `usage:` suffix and
  the append-missing-optional-word fix.
- Deprecation as data: the lifecycle triple, replacement +
  drop-in flag, and the typed quick-fix hook (word replacement with a
  stated safety level); validator-supplied replacement values (W146);
  setter constraints that carry their own diagnostic code and message —
  already the proof that third-party specs can drive checks.
- Outline symbols (`defines_symbol` with name/detail/kind), semantic
  tokens driven entirely by roles, folding via body roles.

## Taint and security

Colour vocabulary by name; sink code strings selecting the diagnostic
(T101 vs IRULE300x); gates over call flags; per-slot code/network
sinks; credential options and args; sensitive headers; double-encode
and sink-safe colours; sanitising transforms.

## Dialects, versions, events

Dialect sets and per-anything lifecycle gates; versioned literal
values; iRules event requirements including **argument-prefix-specific
forms**, excluded events, data collect/release/payload pairing, side
switching, and handler priority policy.

## Command-model oddities

Script/expr concatenation (`eval`, `uplevel`, `expr` multi-word),
list-appending script tails, language-keyword commands, frameless
builtins, safe-interp hidden commands, the `unknown` handler,
`package provide`/`ifneeded`, `interp alias` / `rename` command-table
effects, `defines_command_at` (`coroutine NAME`, `interp create`),
`body_scope` mini-vocabularies — which the DSL itself will use for its
own editing experience, so it must be expressible from day one.
