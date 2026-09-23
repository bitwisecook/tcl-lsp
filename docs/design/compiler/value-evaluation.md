# Value evaluation — the routes, the cores, and the engine

How an exact answer is computed once the consumer interface in
[value-transfers.md](value-transfers.md) has asked for one. A
specialisation names its evaluator route on the spec; the route is a
declared capability with an identity, supported target semantics, declared
dependencies, and a budget; and every route ends in the same validated
answer. This page is the evaluation contract: the three routes, the shared
cores and adapters behind the direct route, the shared expression engine,
the regexp owner, the bounded engine for declared implementations, the one
analysis context every evaluation runs under, budgets and cancellation,
target semantics, and what a pack author writes. Read it before adding an
evaluator to a command, before touching the hook host or the fold engine,
and before promising that two implementations agree.

> **Status — a proposal.** The proposed vocabulary is: `EvalRoute`,
> `NativeEvalId`, `LanguageProfileId`, `NoRouteReason`; `ConstOps`,
> `ConstValue`, `Representation`, `Needs`, `TargetSemantics`, `Axis`,
> `SourceEncoding`; `RegexpPrecision`, `PrecisionDecline`,
> `PatternCacheKey`, `EngineIdentity`; `EvaluatorCapability`,
> `ImplementationIdentity`, `HostKind`, `DeclaredInput`, `Exactness`,
> `ContextDependency`, `CompletionSupport`, `ActivationStore`,
> `Engine::set_release`; `EvalMemoKey`, `SpecialisationId`,
> `EvalRouteId`, `TargetState`, `TargetDigest`, `EvaluatorGeneration`;
> `WorkUnits`, `EvaluationBudget`, `IterationBudget`, `RequestBudget`,
> `CancelPoint`, `CancelToken`; `NativeEvalTables`; the test
> `direct_route_needs_match_their_cores`; the core functions
> `scan::parse_format`, `scan::convert`, `binary::format_size_bound`,
> `numeric_core::tcl_incr`, and the field `MathFuncSpec::result_class`;
> and every `.tclspec` spelling shown as *proposed* — the `semantics` /
> `evaluate` / `facts` statements, the route flags `-direct` /
> `-expression` / `-implementation` / `-host`, the `inputs` / `depends` /
> `budget` / `body` rows, the option flags `-evaluate` /
> `-evaluate-reason`, the body verbs `write` and `preserve`, and DSL
> vocabulary 2.2. None of those name anything in the workspace.
> `DeclineReason` and every variant of it — `NoRoute`, `ReleaseAmbiguous`,
> and `NotText` included — are the interface contract's; this page defines
> the payloads `NoRouteReason` and `Axis` that two of them carry.
> The existing identifiers cited here were checked against the tree at
> the revision named in
> [value-transfers-migration.md](value-transfers-migration.md).
> Every Tcl behaviour quoted was run on `tclsh8.4` (8.4.20), `tclsh8.5`
> (8.5.19), `tclsh8.6` (8.6.18), `tclsh9.0` (9.0.4), and `tclsh9.1`
> (9.1b0).

## Three routes, declared on the spec

```mermaid
flowchart LR
    S["specialisation on the spec<br/>route declared, never inferred"] --> D["direct<br/>registry-owned Rust over<br/>tcl-cmd-core · tcl-regex · tcl-syntax<br/>through ConstOps"]
    S --> X["expression<br/>tcl_syntax::expr::eval + ExprOps<br/>lazy, read-only input services"]
    S --> I["declared implementation<br/>Engine in the bounded host:<br/>identity · target semantics ·<br/>dependencies · budget"]
    S --> N["none<br/>pure, but no backing:<br/>classification only"]
    D --> A["exact answer, or a typed decline"]
    X --> A
    I --> A
    N -. Declined(NoRoute) .-> A
```

- **Direct.** A registry-owned Rust function over the shared cores
  (`tcl-cmd-core`, `tcl-regex`, `tcl-syntax`), reached through one live
  string-backed `ValueOps` implementation that carries the target profile.
  The route for `string`, list and dict value operations, `binary`,
  `format`, `scan`, the numeric updates behind `incr`, `append`, and
  `lappend`, and `regexp` / `regsub`.
- **Expression.** The shared expression engine, `tcl_syntax::expr::eval`
  with its `ExprOps`, driven by the registry-owned `expr` specialisation and
  fed by lazy, read-only analysis services. The route for `expr`, for
  conditions, and for any dialect operation with expression semantics under
  its own language profile.
- **Declared implementation.** An implementation the spec names explicitly
  — a Tcl body authored in the pack, or a pinned package implementation —
  executed in the bounded engine behind `tcl_engine_api::Engine`. The route
  for a private command whose algorithm is not worth writing twice, and
  for an explicitly supported Tcl implementation where bounded execution
  avoids duplicating it.
- **None.** Representable, and the state of most pure commands today: a
  command declared pure with no route classifies as pure for CSE and
  effect reasoning and evaluates nothing. Purity never selects a route.

The four states are one closed enumeration, resolved with the binding and
the selected form:

```rust,ignore
/// The declared way an exact answer is computed. Resolved once per
/// invocation, from the spec's three declaration states (inherited,
/// declared, declined) at command, subcommand, and form scope.
enum EvalRoute {
    /// A registry-owned Rust function over the shared cores, named by
    /// its catalogue id.
    Direct { id: NativeEvalId },
    /// The shared expression engine under a named language profile
    /// (`tcl.expr`, `bpf.expr`).
    Expression { language: LanguageProfileId },
    /// An implementation the spec names, run in the bounded host.
    Implementation(EvaluatorCapability),
    /// Declared absence: classification only. The driver answers
    /// `Declined(NoRoute)` without consulting purity.
    None { reason: NoRouteReason },
}

/// Why a specialisation has no route: the payload of the interface
/// contract's `DeclineReason::NoRoute`, and the "none" column of the
/// migration plan's generated inventory.
enum NoRouteReason {
    /// `evaluate none`: the author abstained at this scope.
    Declared,
    /// No evaluator is authored for the form — the state of most pure
    /// commands today.
    Unauthored,
    /// The form or option is outside what the route models
    /// (`regexp -about`).
    FormUnsupported,
    /// The form runs a callback (`regsub -command`, a `-command`
    /// comparison), which needs a declared route of its own.
    Callback,
}
```

The route, its implementation identity, and its revision are part of the
specialisation's identity and of every memo key. A route that cannot
support the requested target semantics declines; it never answers under a
default it was not asked for.

## The direct route

### What exists

- **One value seam, two runtimes.** `tcl_syntax::value::ValueOps` is the
  generic value interface; `tcl-cmd-core` is written once over it; the
  bytecode VM (`impl ValueOps for Vm`, `rust/tcl-vm/src/value_ops.rs`) and
  the WASM runtime (`impl ValueOps for Interp`, `runtime/rust/src/value_ops.rs`)
  both implement it. Both route the `string` ensemble through
  `tcl_cmd_core::string::dispatch_canon`, `binary format` / `scan` through
  `tcl_cmd_core::binary`, and `format`, `scan`, `regsub`, list, dict, and
  the value half of `incr` / `append` / `lappend` through the cores; each
  runtime's adapter keeps the store, the write trace, and the
  const-variable check.
- **Not every core takes the seam.** `tcl_cmd_core::binary::{format, scan}`
  take `&[u8]` and `&[&[u8]]`, `tcl_cmd_core::regex::regsub` takes
  `&[&[u8]]`, and `tcl_cmd_core::string_is::class_check` takes `&str` —
  they are byte and string functions with no `ValueOps` parameter, so the
  direct route reaches them by materialising operands through
  `ValueOps::as_bytes` and returns through `ValueOps::new_bytes`. A value
  model whose `as_bytes` falls back to the UTF-8 string rep corrupts a
  `binary format` result
  ([byte-array-corruption.md](byte-array-corruption.md)), so overriding
  both is a precondition of the route, not an optimisation.
- **Release axes reach the cores three ways.** Through the implementation:
  `char_len` is `string_char_len(s, runtime_version)`, and `int_add` widens
  past the wide boundary in both runtimes — the VM through an `i128` fast
  tier into a `BigInt`, the WASM runtime into its bignum — as C Tcl does
  from 8.5. Through explicit arguments: `format_cmd_with_syntax(…, NumberSyntax)`,
  `string_is::class_check(…, NumberSyntax)`, `index::resolve_opt_with`,
  `binary::signedness_available(profile)`, `binary::specifier_min_version`,
  `format::is_available`. And in one place not at all:
  `index::resolve` reads an index numeral under the ambient grammar
  `tcl_syntax::number::runtime_syntax()` installed by `set_runtime_syntax`,
  and `string::range`, `string::index`, and `string::word_bound` call it,
  so both runtimes read `string range $s 010 end` under whatever release
  the process was pinned to. Both engines are release-pinnable
  (`Vm::set_runtime_version` / `set_dialect_profile`, `Interp::set_runtime_version`).
- **The registry's folders are a second implementation.** `fold_range` in
  `rust/tcl-registry/src/commands/tcl/string_.rs` is an ASCII-only
  re-implementation beside `tcl_cmd_core::string::range`; `fold_format` in
  `commands/tcl/format_.rs` is a hand-written subset beside
  `format_cmd_with_syntax`; `fold_is` classifies with its own code beside
  `string_is::class_check`; the list folds split through a deliberately
  conservative `split_list` that bails on any backslash. Two folders
  already *are* core calls — `fold_regsub`
  (`tcl_cmd_core::regex::regsub::<AreEngine>`) and `parse_index`
  (`index::resolve_opt_with` under `NumberSyntax::unanimous`) — so the
  precedent runs in both directions. There are three test-only
  string-backed `ValueOps` implementations
  (`rust/tcl-syntax/src/value.rs`, `tcl-cmd-core`, the registry's test
  module) and no live one.
- **A third family lives in codegen.** `rust/tcl-compiler/src/codegen/helpers.rs`
  carries `fold_list_cmd`, reached from codegen. `try_format_fold` (`%s`
  and `%d` only) went with `format`'s transitional table when VT3.8 gave
  the command a registry-owned route over the shared format core.
- **Codegen already emits folded values, guarded.** `try_emit_constant_fold`
  in `rust/tcl-compiler/src/codegen/values.rs` folds a literal-only
  `[cmd …]` through `ConstSubstCtx::fold_cmd_subst_resolved`, pushes the
  literal, and calls `require_command_binding` for every
  `CommandBindingIdentity` the fold consumed; the VM revalidates those
  identities on a command or trace epoch change
  ([vm-compiled-artifact-provenance.md](../contracts/vm-compiled-artifact-provenance.md)
  § *Invalidation*). The guard protects against rebinding; nothing protects
  against the fold and the runtime disagreeing, which a second
  implementation permits.
- **The oracle.** `rust/tcl-registry/tests/differential_fold.rs` runs every
  fold against a real `tclsh`; the fuzzer pairs `tclvm`, `runtime-rust`,
  and `tclsh`, with the rule that a two-way native pair has no oracle
  ([differential-fuzzing.md](../contracts/differential-fuzzing.md)).

```mermaid
flowchart TB
    VM["impl ValueOps for Vm<br/>rust/tcl-vm/src/value_ops.rs<br/>int_add: i128 fast tier, then BigInt"]
    RT["impl ValueOps for Interp<br/>runtime/rust/src/value_ops.rs<br/>int_add: widens to a bignum"]
    CO["ConstOps (proposed)<br/>byte-exact · carries the target profile<br/>int_add: 8.5+ widens, 8.4 declines; ValueError → decline"]
    CORE["tcl-cmd-core · written once over ValueOps<br/>string::dispatch_canon · string::range<br/>binary::{format, scan} over bytes<br/>format_cmd_with_syntax(NumberSyntax)<br/>string_is::class_check · index::resolve_opt_with<br/>regex::{regexp, regsub} · switch::select · list / dict"]
    VM -->|calls, passing self| CORE
    RT -->|calls, passing self| CORE
    CO -->|calls, passing self| CORE
    AD["admissibility adapter: ConstOps::admit(ctx, Needs)<br/>character-model unanimity before char_len<br/>index numerals pre-resolved under the target grammar<br/>bound charged before allocation"] --> CO
    CORE -->|evidence| OR["independent oracles<br/>differential_fold.rs → tclsh<br/>fuzzer: tclvm × runtime-rust × tclsh"]
```

### `ConstOps`, and why the adapters matter

One live `ValueOps` implementation, `ConstOps`, carrying the target
profile — the release when the profile names one or its dialect declares
a base release, and otherwise the unanimity rule, under which an
operation's answer stands only where every release the profile can denote
gives it — with every `ValueError` mapped to a decline. Every shipped
evaluator then *is* a core call: `fold_range`
becomes `string::range(&mut ops, s, first, last)`; the whole `string`
ensemble evaluates through `dispatch_canon`, whose `None` is a decline;
`format` is `format_cmd_with_syntax` under the profile's `NumberSyntax`;
`string is` is `class_check`; `binary format` / `scan` are
`tcl_cmd_core::binary`; the increment, append, and list-append updates are
the same value computations the runtime adapters call —
`ValueOps::int_add`, `var::append_bytes`, `var::lappend_value` — with the
lattice write as the compile-time store. The registry's ASCII folders, the
two codegen helpers, and the five name-keyed SCCP arms retire; the
conservative list splitter becomes a precondition on the input (a
backslash-free list) rather than a divergent parser; the registry's
cross-check tests become tautologies and are replaced by the `tclsh`
differential.

Introducing `ConstOps` alone does not deliver the release rules, because
the cores' signatures were written for a runtime that has already chosen
its release:

- `ValueOps::char_len` returns `usize`, not a fallible result, so it
  cannot express "the releases disagree, abstain". The admissibility check
  — `StringCharacterModel` unanimity for a profile that names no release —
  runs in a thin explicit adapter before the core is called, or in the
  semantic owner; it is not smuggled into an override of `char_len`.
- `string::range` and `string::index` use Unicode scalar vectors and call
  `index::resolve` directly; overriding `char_len` changes neither their
  indexing model nor their numeral grammar. The numeral-grammar
  precondition is likewise an adapter obligation, and the adapter
  discharges it by *resolving the index itself* rather than by wishing for
  a `*_with` variant of every core that reads one.
- The runtime adapters still own coercions, byte access, and result
  construction. Calling the same generic function does not prove that
  different adapters produce the same observable result; adapter-level
  tests remain.
- For a release *range*, the adapter compares all relevant semantic cases
  or relies on an established invariance proof; one run at a default
  release does not establish unanimity.
- Resource limits apply to direct evaluators too. Checking output size
  after allocating a giant `string repeat`, an exponentiation result, or a
  binary buffer does not bound memory: bound before allocation, and charge
  significant native work to the request budget. `string::repeat` is
  `src.repeat(n)` with no bound of its own, and C Tcl refuses the same
  call before allocating — `string repeat ab 100000000000` raises
  `integer value too large to represent` on 8.4, 8.5, and 8.6 and
  `string size overflow: unable to alloc 200000000000 bytes` on 9.0 and
  9.1 — so the charge, not the core, is what keeps the analyser alive.

Release-sensitive checks live in the semantic owner or in one explicit
admissibility adapter, and conservative preconditions stay until a core
supports the target axis it lacks.

The answer shapes this page hands back — `EvalAnswer`,
`InvocationOutcome`, `StoreOutcome`, `CompletionOutcome`, `TypeFacts`,
`DependencyEvidence`, `TargetId`, `OperandId`, `AnalysisInputs`,
`AnalysisContext`, and `DeclineReason` — are the interface contract's and
are defined in [value-transfers.md](value-transfers.md) § *The interface*.
What this page contributes to them is the payloads two reasons carry —
`NoRouteReason` behind `NoRoute` and `Axis` behind `ReleaseAmbiguous` —
and the mapping of the regexp owner's `PrecisionDecline` onto reasons the
interface contract already names: `FuelExhausted`, `DepthExhausted`, and
`ApproximateCapture` are `Approximate`; `FormUnsupported` is
`Unsupported`; `Cancelled` is `Budget`; and `PatternError` is the call's
error completion, never a value. None of them is a second answer kind.
The `Budget` an evaluator is handed is that page's handle, the route's
view of one `EvaluationBudget` (§ *The three nested budgets*);
`tcl_engine_api::Budget` is the engine's own caps, and the two are named
apart wherever both appear.

### The `ConstOps` type

```rust,ignore
/// The one live compile-time value model. Byte-exact, release-bound,
/// budget-charging, and poisoned by the first fault so no partial answer
/// escapes.
struct ConstOps<'ctx> {
    /// The admitted target semantics: the release when the profile names
    /// one or its dialect declares a base release, and the unanimity rule,
    /// applied per operation, otherwise.
    target: TargetSemantics,
    /// What `admit` was asked for. Every core call is checked against it.
    admitted: Needs,
    /// The evaluation's slice of the request budget: the interface
    /// contract's `Budget` handle, which is one `EvaluationBudget` as a
    /// route sees it (§ *The three nested budgets*).
    budget: &'ctx mut Budget,
    /// The first fault any core or seam method recorded. Once set, `take`
    /// declines whatever value it is handed.
    fault: Option<DeclineReason>,
    /// The cancellation token every `CancelPoint` reads.
    cancel: &'ctx CancelToken,
}

/// A compile-time value: byte-exact, with the string rep derived on
/// demand and the representation kept as separate evidence.
#[derive(Clone)]
struct ConstValue {
    bytes: Rc<[u8]>,
    /// Representation evidence for the answer, never a coercion:
    /// `String`, `ByteArray`, `List`, `Dict`, `Int`, `Double`.
    repr: Representation,
}

/// The target semantics one evaluation runs under. Every field is the
/// answer of the release in force — the named one, or the dialect's
/// declared base — or `None` when no single release answers the axis: an
/// operation that reads that axis then answers with the value every
/// release the profile can denote gives for its own operands, and declines
/// with `ReleaseAmbiguous(axis)` where they differ.
struct TargetSemantics {
    release: Option<TclVersion>,
    numerals: Option<NumberSyntax>,
    character_model: Option<StringCharacterModel>,
    byte_strings: Option<ByteStringEncoding>,
    source_encoding: Option<SourceEncoding>,
    profile: Option<&'static DialectProfile>,
}
```

The `ValueOps` implementation surface — every method `ConstOps` must
override, and why the default is wrong for a compile-time model:

| `ValueOps` member | `ConstOps` behaviour |
|---|---|
| `type Value` | `ConstValue`, byte-exact; `Rc<str>` cannot hold a `binary format` result |
| `as_bytes` / `new_bytes` | the real bytes, both ways; the defaults round-trip through UTF-8 and are lossy |
| `as_str` | the bytes decoded as UTF-8; invalid bytes record `DeclineReason::NotText` and poison, rather than substituting U+FFFD |
| `char_len` | `StringCharacterModel::count` under the named release, else `count_for(None, s)`, whose `None` poisons with `ReleaseAmbiguous(Axis::CharacterModel)` |
| `as_int` / `as_double` / `as_bool` | `tcl_syntax::number::parse` under `target.numerals`; a `ValueError` is a decline, never a zero |
| `string_compare_length` | the `-length` argument read as a character count under the same model as `char_len` |
| `int_add` | widens into a `BigInt` from release 8.5 onward; on 8.4 an overflow is `ValueError::IntegerOverflow`, which is a decline; on an unnamed release the two answers differ, so the operation needs `Needs::INT_TOWER` and declines at `admit` |
| `list_elements` / `list_len` / `list_index` / `list_append` | `tcl_syntax::list::split_list`; `ValueError::BadList` is a decline carrying the canonical message |
| `dict_pairs` / `new_dict` | the seam's own canonical ordering, first-occurrence position with the last value winning |
| `dict_hash_bucket_count` | `None`, the string-model answer; `dict::info` therefore declines rather than inventing a bucket history |
| `new_int` / `new_double` / `new_bool` / `new_list` / `new_str` | the canonical spellings, with every construction charged to the budget before it allocates |
| `pin_value` / `unpin_value` | the owning-model defaults; there is no refcounted runtime object to hold |
| `try_append_bytes_in_place` / `try_list_append_in_place` | `true` when the `Rc` is unshared, so a building loop stays amortised rather than quadratic |

### The admissibility set

`Needs` is the complete set of release axes a core can read. One bit per
axis, and the axis's owner in the tree decides each bit:

| bit | axis | owner in the tree |
|---|---|---|
| `NUMERAL_GRAMMAR` | how a numeral operand is read | `NumberSyntax`, `tcl_syntax::number::parse` |
| `INDEX_GRAMMAR` | how an *index* numeral is read | `index::resolve_opt_with` |
| `CHAR_MODEL` | the character-counting rule | `StringCharacterModel::count_for` |
| `CHAR_INDEXING` | the addressing unit for a character index | `string::range`'s scalar vector, invariant in both runtimes for every release |
| `INT_TOWER` | whether an integer operation widens or raises | `ValueOps::int_add`, `ValueError::IntegerOverflow` |
| `BINARY_FIELDS` | the field-letter set and the `u` suffix | `binary::specifier_min_version`, `binary::signedness_available` |
| `FORMAT_VERBS` | the `format` conversion set | `format::is_available`, `format::is_verb` |
| `STRING_CLASSES` | the `string is` class set and its bounds | `string_is::resolve_class`, `NumberSyntax::is_tcl9_or_later` |
| `REGEXP_FEATURES` | ARE syntax, flags, and engine limits | `tcl_regex::cmd_core::AreEngine`, `regex::RegexFlags` |
| `LIST_RENDERING` | the canonical quoting of a list result | `tcl_syntax::list`, `rust/tcl-vm/tests/dict_canonicalisation_parity.rs` |
| `DICT_ORDER` | canonical key order and the duplicate rule | `ValueOps::dict_pairs` |
| `COLLATION` | case folding and ordering in a comparison | `string::simple_upper` / `simple_lower` / `simple_title`, `switch::select` |
| `BYTE_STRINGS` | how a code point above `U+00FF` crosses to bytes | `TclVersion::byte_string_encoding`, whose two answers are `ByteStringEncoding::LegacyTruncate` up to 8.6 and `CheckedLatin1` from 9.0 |
| `SOURCE_ENCODING` | how a non-ASCII source literal was decoded | the release's script reader; measured below |
| `PLATFORM` | host facts a platform-backed core reads | `tcl_cmd_core::platform`, `tcl_cmd_core::channel` |
| `WALL_CLOCK` | a clock, locale, or timezone read | `tcl_cmd_core::clock` |

```rust,ignore
/// The axis two answers differed on: the payload of the interface
/// contract's `DeclineReason::ReleaseAmbiguous`. One variant per `Needs`
/// bit, in CamelCase (`CharacterModel` for `CHAR_MODEL`, and so on), plus
/// the availability of a command, form, or option that the profile's
/// releases do not all have.
enum Axis {
    NumeralGrammar, IndexGrammar, CharacterModel, CharIndexing, IntTower,
    BinaryFields, FormatVerbs, StringClasses, RegexpFeatures, ListRendering,
    DictOrder, Collation, ByteStrings, SourceEncoding, Platform, WallClock,
    Availability(SpecSurface),
}
```

`SOURCE_ENCODING` is an axis because the same bytes are a different value
by release. A file whose bytes are `set s "<C3 89>"` followed by
`puts [string length $s]` prints **2** on `tclsh8.4`, `tclsh8.5`, and
`tclsh8.6` where `encoding system` is `iso8859-1`, **1** on the same three
under `LC_ALL=C.UTF-8`, and **1** on `tclsh9.0` and `tclsh9.1` under
either locale: the script encoding follows `encoding system` up to 8.6 and
is UTF-8 from 9.0. Our front end decodes every source file as UTF-8, which
is the 9.x answer, so an operand whose source spelling carries a non-ASCII
byte is admissible only when the target names 9.0 or newer, or the
workspace declares the source encoding; otherwise the releases disagree on
that operand and the operation declines with
`ReleaseAmbiguous(SourceEncoding)`.

`PLATFORM` and `WALL_CLOCK` are never satisfiable on the direct route,
under any profile. That is how a clock-reading or host-reading core is
kept off the route by construction rather than by a whitelist someone has
to remember to update: `tcl_cmd_core::clock::dispatch` and
`tcl_cmd_core::platform::{exec, pwd}` request them, so they decline before
they run.

### `admit`, and `take`

```rust,ignore
impl<'ctx> ConstOps<'ctx> {
    /// Open a value model for this evaluation, proving the target answers
    /// every axis in `needs`. The `Err` is the interface contract's
    /// `DeclineReason`, so a route's admissibility failure and a core's
    /// runtime failure are one kind of answer.
    fn admit(
        ctx: &'ctx AnalysisContext,
        budget: &'ctx mut Budget,
        needs: Needs,
    ) -> Result<Self, DeclineReason>;

    /// Resolve an index operand under the admitted grammar, returning the
    /// canonical decimal spelling the cores' own `index::resolve` reads
    /// identically in every release.
    fn index(&mut self, spec: &ConstValue, len: usize) -> Result<ConstValue, DeclineReason>;

    /// Charge `bytes` before the allocation that would produce them.
    fn charge_bytes(&mut self, bytes: u64) -> Result<(), DeclineReason>;

    /// Close the evaluation: the value, or the first recorded fault.
    fn take(self, value: ConstValue) -> Result<ExactValue, DeclineReason>;
}
```

The protocol, in order, and every step is mandatory:

1. **Resolve the target once.** `TargetSemantics` comes from the
   context's profile. A profile that names a release — `tcl8.4` to
   `tcl9.1` — fills every field through that release's own accessors:
   `number_syntax`, `string_character_model`, `byte_string_encoding`. A dialect that
   declares a base release evaluates under that release the same way —
   iRules on its 8.4-derived engine — except on an axis its pack declares
   divergent, which the base does not answer. A profile that names no
   release and declares none leaves every field open: the operation that
   reads an axis resolves it through that axis's unanimity rule
   (`NumberSyntax::unanimous`, `StringCharacterModel::count_for(None, …)`).
2. **Admit by unanimity, per operation.** A route folds when its answer
   is proven identical under every release the profile can denote.
   `PLATFORM` and `WALL_CLOCK` decline here, because no release answers
   them; every other requested bit is admitted, and the operation that
   reads it compares the releases' answers for its own operands. `incr x
   5` folds under a profile that names no release, because every release
   adds 5. `incr x` over `010` does not: 8.x reads 8 and 9.x reads 10, so
   it declines with `DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar)`,
   and any other per-axis disagreement declines the same way, naming its
   axis. A vendor pack that diverges from its base on an axis blocks the
   fold by declaring the axis: a declared axis is a disagreement. Declining
   a release-less profile outright would lose precision without adding
   soundness, since the unanimous answer is the answer under whichever
   release runs (the owner's ruling of 2026-09-22,
   [value-transfers.md](value-transfers.md) § *Rulings*).
3. **Charge admission.** A fixed charge per admission, so a solver
   iteration that declines ten thousand times is still bounded, and the
   decline is recorded once per invocation rather than once per retry.
4. **Pre-resolve index operands.** An evaluator whose core reads an index
   calls `ops.index(spec, len)` and passes the canonical decimal spelling
   on. This is what keeps the `010` answer honest: a 12-element list reads
   `lindex $l 010` as `i` on 8.4, 8.5, and 8.6 and as `k` on 9.0 and 9.1,
   and `string range abcdefghijkl 010 end` is `ijkl` on the first three and
   `kl` on the last two. `parse_index` already declines that case through
   `NumberSyntax::unanimous`, and the CLI shows it: `tcl opt` folds
   `string range abcdefghijkl 3 6` to `defg` under O129 and O100 and folds
   nothing at all for `string range abcdefghijkl 010 end` under every
   dialect. Calling `string::range` directly would *lose* that decline,
   because the core's `index::resolve` reads the ambient grammar. The
   adapter's `index` step is the whole of the fix, and `index::bad_index`
   supplies the canonical message when the spec is malformed.
5. **Charge before allocating.** Every core whose output size is a
   function of its input charges the bound first.
6. **Read the cancellation token** at each `CancelPoint` (§ *Budgets and
   cancellation*).
7. **Close through `take`.** A poisoned run declines whatever value the
   core returned, so a `char_len` that could not decide cannot leak a zero
   into the lattice.

A `ConstValue` converts to an `ExactValue` without loss — the bytes and
the representation evidence carry over, and the numeric classification is
derived — so once `take` has closed the run over the result, the values
of the ordered stores convert directly; a poisoned run never reaches that
conversion, because `take` has already declined.

A core call outside `admitted` is a programming error, not a decline:
`debug_assert` catches it in development, and
`direct_route_needs_match_their_cores` calls every catalogued direct
evaluator with an empty `Needs` and requires each to decline.

### The cores the direct route calls

Every function below exists in `rust/tcl-cmd-core/src` at HEAD unless the
row says otherwise. "Declines on" lists what the *route* answers as a
decline, in addition to the universal ones the interface contract states
once. "Charge" is in `WorkUnits` (§ *Budgets and cancellation*).

| Core function | `Needs` | Release axes involved | Declines on | Charge |
|---|---|---|---|---|
| `string::dispatch_canon` | the union of the row it dispatches to | as dispatched | `None` — an unhandled subcommand, `is` among them | the dispatched row plus 1 |
| `string::length` | `CHAR_MODEL` | 8.x counts UTF-16 units, 9.x counts scalars; measured: `string length [encoding convertfrom utf-8 <F0 9F 98 80>]` is 4 on 8.4 and 8.5, 2 on 8.6, 1 on 9.0 and 9.1 | a supplementary character with no named release; a non-UTF-8 value | 1 per input byte |
| `string::index`, `string::range` | `INDEX_GRAMMAR`, `CHAR_INDEXING`, `SOURCE_ENCODING` | the index numeral grammar; the scalar addressing both runtimes use for every release | a malformed index; a non-ASCII operand with no named release | 1 per input byte, 1 per output byte |
| `string::word_bound` | `INDEX_GRAMMAR`, `CHAR_INDEXING` | as above | as above | 1 per input byte |
| `string::reverse` | `CHAR_INDEXING` | scalar reversal against code-unit reversal | a value the character models disagree on | 1 per byte both ways |
| `string::repeat` | none | none | a negative or non-integer count (`string repeat ab -1` is the empty string on every release, and is not a decline); an output past the charge | `len × count`, charged first |
| `string::compare` (`CompareMode::{Equal, Compare}`) | `COLLATION`, `CHAR_MODEL` | the `-length` argument is a character count; `-nocase` folding | a `-length` the models disagree on | 1 per compared byte |
| `string::string_match` | `COLLATION` | `-nocase` folding | — | 1 per pattern × subject step |
| `string::map` | `COLLATION` | `-nocase` folding | — | 1 per input byte × map size |
| `string::case_convert` (`CaseMode`, `simple_upper` / `simple_lower` / `simple_title` / `simple_title_rest`) | `COLLATION`, `CHAR_MODEL` | the optional `first` / `last` are character indices; the case tables | a first/last pair the models disagree on | 1 per byte |
| `string::replace`, `string::insert` | `INDEX_GRAMMAR`, `CHAR_INDEXING` | index grammar | a malformed index | 1 per byte both ways |
| `string::first`, `string::last` | `INDEX_GRAMMAR`, `CHAR_INDEXING` | the optional start index | a malformed index | 1 per compared byte |
| `string::trim`, whose `left` and `right` flags are `trimleft` and `trimright` | none | none | — | 1 per trimmed byte |
| `string::cat` | none | none | an output past the charge | 1 per output byte, charged first |
| `index::resolve`, `resolve_with`, `resolve_opt`, `resolve_opt_with`, `encodable`, `bad_index` | `INDEX_GRAMMAR` | measured on a 12-element list `a`…`l`: `lindex $l 010` is `i` up to 8.6 and `k` from 9.0, `lindex $l end-010` is `d` up to 8.6 and `b` from 9.0, `lindex $l 1_0` and `lindex $l 0d1` are `bad index` up to 8.6 and `k` and `b` from 9.0, while `lindex $l 0x2` is `c` on every release | disagreement with no named release | 1 |
| `index::drill` | `INDEX_GRAMMAR`, `LIST_RENDERING` | as above | a non-list step, an out-of-range step | 1 per path step |
| `binary::format` | `BINARY_FIELDS`, `BYTE_STRINGS` | `t n m r R q Q` arrive in 8.5 (`specifier_min_version`); the `u` suffix is 8.5+ (`signedness_available`) | a field the target lacks; an output past the charge | `binary::format_size_bound` (proposed), charged first |
| `binary::scan` | `BINARY_FIELDS` | as above | as above | 1 per scanned byte |
| `binary::specifiers`, `is_specifier`, `specifier_min_version`, `signedness_available` | `BINARY_FIELDS` | as above | — | 1 per format byte |
| `binary::hex_encode` / `base64_encode` / `uu_encode` / `hex_decode` / `base64_decode` / `uu_decode` (and `DecodeError`) | `BYTE_STRINGS` | the Tcl 9 checked conversion against the 8.x low-byte truncation | a `DecodeError`; an output past the charge | 1 per byte both ways, charged first |
| `format::format_cmd_with_syntax` (and `format_cmd`, `is_verb`, `is_available`) | `NUMERAL_GRAMMAR`, `FORMAT_VERBS`, `CHAR_MODEL` | `%b` is `bad field specifier "b"` on 8.4 and 8.5 and valid from 8.6; `%lld` is an error on 8.4 and valid from 8.5; `format %d 010` is 8 up to 8.6 and 10 from 9.0 | a verb the target lacks; a width past the charge | 1 per output byte, charged first |
| `string_is::class_check`, `resolve_class` | `STRING_CLASSES`, `NUMERAL_GRAMMAR`, `CHAR_MODEL` | `wideinteger` raises on 8.4 and exists from 8.5; `entier` raises up to 8.5 and exists from 8.6; `dict` raises up to 8.6 and exists from 9.0; `string is integer 4294967296` is 0 up to 8.6 and 1 from 9.0 | a class the target lacks; a bound the releases disagree on | 1 per input byte |
| `regex::regexp` over `tcl_regex::cmd_core::AreEngine` | `REGEXP_FEATURES`, `CHAR_INDEXING`, `BYTE_STRINGS` | index units follow the character model; ARE syntax is release-stable | every `PrecisionDecline`; `-about`, which the core refuses although C Tcl answers `1 {}` on every release | the engine's fuel, plus 1 per capture byte |
| `regex::regsub` | `REGEXP_FEATURES`, `BYTE_STRINGS` | `-command` is `bad switch` up to 8.6 and works from 9.0 | `-command`, which the core refuses; every `PrecisionDecline` | the engine's fuel, plus 1 per output byte |
| `switch::parse_options`, `switch::select` (`Mode`, `Options`, `Selection`, `extra_pattern_error`, `no_body_error`) | `REGEXP_FEATURES`, `COLLATION` | `-nocase` is `bad option` on 8.4 and exists from 8.5; exact `-nocase` matching in the core is `eq_ignore_ascii_case`, which is ASCII-only, while C Tcl folds the full range | a non-ASCII `-nocase` pattern; a pattern compile error | 1 per pattern, plus the regexp row per regexp pattern |
| `list::list`, `llength`, `lreverse`, `lrepeat`, `linsert`, `lreplace`, `concat` (with `trim_concat_element`), `join`, `split` | `LIST_RENDERING` | canonical quoting | `ValueError::BadList`; an output past the charge | 1 per element, charged first for `lrepeat` |
| `list::lindex`, `lindex_flat`, `lrange` | `LIST_RENDERING`, `INDEX_GRAMMAR` | index grammar and quoting | as above, plus a malformed index | 1 per element |
| `dict::create`, `get`, `getdef`, `exists`, `keys`, `values`, `size`, `filter`, `merge`, `replace`, `remove`, `lookup`, `upsert`, `dispatch_canon` (and `worded_parse_error`) | `DICT_ORDER`, `LIST_RENDERING` | canonical key order, last value winning on a duplicate | an odd-length list; a missing key where the form raises | 1 per pair |
| `dict::info` | `DICT_ORDER` | the retained bucket-array history | always. The core answers: it calls `dict_hash_bucket_count` and falls back to a fresh table when the answer is `None`, which `ConstOps` always returns. A bucket history is not derivable from a string, so the route declines rather than publish a statistic the analysed program's runtime may not have | 1 |
| `scan::validate_format`, `scan::scan_match` (`Scanned`, `ScanOutcome`) | `NUMERAL_GRAMMAR`, `CHAR_MODEL` | conversion numeral grammar | a format `validate_format` rejects | 1 per subject byte |
| `var::append_bytes` | none | none | an output past the charge | 1 per appended byte, charged first |
| `var::lappend_value` | `LIST_RENDERING` | canonical quoting | `ValueError::BadList` — which is why `lappend unused value` on a value of `{` is not removable | 1 per element |
| `ValueOps::int_add` (the `incr` arithmetic owner) | `NUMERAL_GRAMMAR`, `INT_TOWER` | 8.4 raises past the wide boundary and 8.5 onward widens; `incr` of `010` is 9 up to 8.6 and 11 from 9.0; `incr` of an absent variable raises on 8.4 and creates it from 8.5 | `ValueError::IntegerOverflow` with no named release; an unbound place with no existence proof | 1, plus 1 per digit of a bignum result |
| `lsearch::lsearch` | `LIST_RENDERING`, `INDEX_GRAMMAR`, `REGEXP_FEATURES`, `COLLATION` | `-nocase` is `bad option` on 8.4 and exists from 8.5; the sorted-list comparison folds with `to_ascii_lowercase` | an `LsearchError`; a non-ASCII `-nocase` operand; every `PrecisionDecline` under `-regexp` | 1 per element, plus the regexp row |
| `mathop::eval` | as the expression route | the operator set by release | — | the expression route's charge |
| `lsort`, `sort`, `prefix`, `lseq`, `path`, `ensemble`, `error` | `LIST_RENDERING`, `COLLATION` where each applies | `lsort -nocase` is `bad option` on 8.4 and exists from 8.5; `sort`'s `-nocase` and `-dictionary` orders fold with `to_ascii_lowercase` | a comparison command operand, which is a callback and needs a declared route; a non-ASCII `-nocase` or `-dictionary` element | 1 per element, `n log n` for a sort |
| `array`, `var` (beyond the two value helpers), `namespace`, `info`, `trace`, `channel` | — | — | always: they read the interpreter, not a value. Their invocations are structural plans, never direct evaluators | — |
| `platform::exec`, `platform::pwd` | `PLATFORM` | the host | always: `PLATFORM` is never satisfiable | — |
| `clock::dispatch` (and `clock::is_specifier`, `clock::specifiers`) | `WALL_CLOCK` | the clock, locale, and timezone | always: `WALL_CLOCK` is never satisfiable. The format-specifier helpers are pattern inspection, not evaluation, and stay available to diagnostics | — |

Four names the declarations in
[value-transfers-examples.md](value-transfers-examples.md) use are
*proposed* cores, and each has a standing-in owner today:

| Proposed | Stands in for | Note |
|---|---|---|
| `scan::parse_format` | `scan::validate_format` | today it returns the conversion count or a message, not a parsed format |
| `scan::convert` | `scan::scan_match` → `ScanOutcome` | the outcome already carries the per-target `Scanned` values |
| `binary::format_size_bound` | nothing | the charge cannot be derived from `binary::specifiers` alone, because `a`, `A`, and `x` take explicit counts; the bound is new code beside `binary::format` |
| `numeric_core::tcl_incr` | `ValueOps::int_add` plus `tcl_syntax::number::parse` | the seam already folds the absent-value-as-zero case in, so the proposed wrapper adds only the release's parse and the existence check |

No core gains a `*_with` variant for this page: the axes the cores read
implicitly are discharged by the adapter, either by pre-resolving the
operand (`INDEX_GRAMMAR`) or by proving unanimity before the call
(`CHAR_MODEL`).

## The expression route

The compiler shares the expression tree walk: `tcl_syntax::expr::ExprOps`
is implemented once for constant folding (`FoldOps` in
`rust/tcl-compiler/src/tcl_expr_eval.rs`) and once for the interface
(`ExprServices`, beside it), and both call the shared math-function
dispatcher. The route is `ExprServices` driven by `evaluate_expression`;
the two, and its answer `ExprAnswer`, are `pub(crate)` to `tcl-compiler`,
because the registry assembles the expression (`ExpressionRoute::assemble`)
and the compiler's driver runs it, so no crate outside the compiler calls
the engine. Nothing new parses or walks an expression. Three changes made
the walk an evaluator for the interface:

- **The full value comes back.** `ExprServices` answers the engine's own
  value, with its numeric classification as a fact beside the bytes, so a
  string-valued expression folds: `expr {"x"}` is the string `x` on every
  release from 8.4 to 9.1, and the lattice holds `x`. The old entry,
  `eval_with_config`, still ends in `to_number` (its `TclValue` has only
  `Int`, `Float` and `Big`) and still serves the callers that want a
  number: code generation's constant operands and the static loop
  simulator. It keeps the route's tower (a beyond-wide integer or an
  infinity folds nothing under an 8.4 runtime or a profile naming no
  release) and a math function's availability and case, but reads no
  binding evidence; the simulator gains it when it runs the registry's
  routes (slice 12). Every rewrite that replaces an expression with its
  value or decides a condition — O101, O112, a branch condition's fold,
  and the propagation folds of a return value, a call site and an
  assigned expression — asks the route instead, under the rewrite's
  whole-module trust (`value_transfer::evaluate_expression_detached` and
  `decide_condition_detached`), so it rewrites only what the lattice
  proves.
- **Inputs are lazy and read-only.** The `var`, `command`, and `call`
  services read the analysis inputs at this program point: a variable when
  it is reached, a supported nested invocation through its registry
  semantics when it is reached, a math function only when reached and only
  with binding evidence. `expr {0 && [expensive]}` never asks whether
  `expensive` is calculable. The nested-substitution policy is the
  interface contract's, stated in
  [value-transfers.md](value-transfers.md) § *`expr`: the first demanding
  client*: only nested operations whose effects cannot invalidate the
  observed inputs are accepted, the rest decline, and the generic ordered
  evaluation state that would lift the restriction is specified there, not
  here. The three measured cases the policy exists for are unanimous on
  8.4, 8.5, 8.6, 9.0, and 9.1: with `x` set to 1, `expr {$x + [incr x] + $x}`
  is 5 and leaves `x` at 2; `expr {0 && [incr x]}` is 0 and leaves `x` at
  1; `expr {$x + [set x 10] + $x}` is 21 and leaves `x` at 10.
- **Bindings are evidence.** The math-function dispatcher in
  `tcl_syntax::expr::mathfunc` is the one owner of the function table, and
  `type_infer.rs`'s duplicate return-type table went onto it:
  `MathFuncSpec::result_class` (a `MathResultClass`) is a field of the
  existing struct beside `name`, `since`, `arity`,
  `accepts_boolean_operand`, and `summary`, and `expr_call_type` reads it.
  Each function and nested command used is a dependency in the answer's
  evidence, and becomes one in the memo key when slice 4's key reaches it. `rand` and `srand` are the one non-determinism check the
  evaluator keeps by name, because non-determinism is an expression fact,
  and `tcl_syntax::expr::rand`'s `seed_from_wide` / `step` / `scale` /
  `next_draw` / `seed_and_draw` are a faithful model of the generator, not
  a licence to publish a draw as a constant. Rebinding is a real hazard
  and it is release-gated: `rename ::tcl::mathfunc::abs …` then
  `proc ::tcl::mathfunc::abs {x} {return 99}` makes `expr {abs(-2)}` answer
  99 on 8.5, 8.6, 9.0, and 9.1, while on 8.4 the `proc` itself fails with
  `can't create procedure "::tcl::mathfunc::abs": unknown namespace`
  because TIP 232 is 8.5 — so on 8.4 the builtin table is the binding, and
  from 8.5 it is evidence.

`MathFuncSince` already gates availability per release
(`Tcl84`, `Tcl85`, `Tcl90`, `Tcl91`), so a function the target lacks is a
decline from data the tree already has, not a new check.

A dialect that gives Tcl syntax different arithmetic gets its own
language-semantic adapter for the same engine, named by the route's
`LanguageProfileId`. BPF-Tcl's signed division truncates towards zero
where Tcl's floors — `expr {-7 / 2}` is −4 and `expr {-7 % 2}` is 1 on
every Tcl release from 8.4 to 9.1 — and its widths, overflow, signedness,
remainder, and shift rules differ; that profile is a route dependency, and
a Tcl engine result is never used as a BPF constant. The tower is
release-gated in plain Tcl too: `expr {1 << 70}` is 0 on 8.4 and
1180591620717411303424 from 8.5, and `expr {2**70}` is
`syntax error in expression "2**70": unexpected operator *` on 8.4 and
1180591620717411303424 from 8.5, so both the operator set and the tower
belong to `INT_TOWER` and `NUMERAL_GRAMMAR` in the target-semantics
matrix rather than to an invariant subset.

Partial simplification and algebraic regrouping are separate operations
with their own proofs, specified in the interface contract; they extend
`optimiser/helpers/expr_simplify.rs` and `optimiser/propagation.rs` and do
not live in this route.

## The regexp route

Concrete matching goes through our own engine: `tcl_regex::cmd_core::AreEngine`
implements `tcl_cmd_core::regex::RegexEngine`, and
`tcl_cmd_core::regex::{regexp, regsub}` own the command algorithms over it.
The registry already depends on `tcl-regex` with its `cmd-core` feature and
its `regsub` folder already takes this route, so this is an existing seam.
It serves `regexp` and `regsub` values, regexp-mode `switch` selection
through `tcl_cmd_core::switch::select`, `lsearch -regexp` through
`tcl_cmd_core::lsearch::lsearch`, and any expression or dialect operation
that needs a concrete match. The specialisation describes the command and
form and maps results onto the answer protocol; the shared plumbing owns
the command algorithms; the engine owns ARE syntax and matching; the
analyser gains no second matcher, and no engine invocation is needed to
match two known values.

### The typed precision result

`Regex::exec` in `rust/tcl-regex/src/lib.rs` returns an `Option` of
captures, and so does `RegexEngine::exec`. In
`rust/tcl-regex/src/exec.rs`, `Bt::m` returns false when the shared
`MATCH_FUEL` budget of 4,000,000 units or the `MAX_BT_DEPTH` recursion
limit of 256 runs out, and the search then reports no match;
`Matcher::dissect_repeat` documents a `MAX_DISSECT_DEPTH` fallback, also
256, that approximates a subgroup's capture span. The adapter therefore
cannot tell a completed no-match from an incomplete search, and an
approximate capture from an exact one. Neither can become a compile-time
fact: an incomplete search does not prove a branch false, and an
approximate capture cannot become a constant variable value or a
replacement string. Putting the same matcher behind an engine does not
recover the lost information.

The regexp owner's result becomes fallible and precision-aware, and the
same three-way answer travels the whole way out:

```rust,ignore
/// What a bounded match establishes. The only variant a compile-time
/// fact may be built from is `Exact`.
enum RegexpPrecision<V> {
    /// The search ran to completion and matched. Spans are half-open
    /// `[so, eo)` character offsets, exactly as `RegMatch` carries them;
    /// a non-participating subexpression is `None`.
    Exact {
        whole: Span,
        groups: Vec<Option<Span>>,
        /// Every span was produced by the exact path, never by
        /// `dissect_repeat`'s depth fallback.
        captures_exact: bool,
    },
    /// The search ran to completion and did not match. This is the only
    /// answer that proves a negative.
    NoMatch,
    /// The search did not establish either, for a recorded reason.
    Declined(PrecisionDecline),
}

enum PrecisionDecline {
    /// `MATCH_FUEL` ran out in `Matcher::reach` or `Bt::m`.
    FuelExhausted { spent: u64 },
    /// `MAX_BT_DEPTH` or `MAX_DISSECT_DEPTH` was reached.
    DepthExhausted { limit: u32 },
    /// A span came from `dissect_repeat`'s approximation.
    ApproximateCapture { group: usize },
    /// The pattern did not compile; carries the engine's detail bytes as
    /// `RegexError` already does.
    PatternError(RegexError),
    /// The option or form is outside what the core implements: `-about`,
    /// which C Tcl answers on every release, and `regsub -command`, which
    /// C Tcl answers from 9.0.
    FormUnsupported { option: &'static str },
    /// The request budget or the cancellation token stopped the match.
    Cancelled,
}
```

How it travels, one layer at a time:

1. `Regex::exec` returns `Option<Vec<Option<Span>>>` today; it gains the
   three-way answer, because only the engine knows whether the fuel
   counter or a depth limit ended the search. `Matcher` already holds
   `fuel` (a `u64` in the set-simulation path, a `Cell<u64>` in the
   backtracking one), so the distinction costs the engine one field on
   its result, not a second traversal.
2. `RegexEngine::exec` in `tcl_cmd_core::regex` carries the same answer, so
   every provider must state it. The trait's `notbol` contract and its
   `NO_MATCH` sentinel for a non-participating subexpression are unchanged.
3. `tcl_cmd_core::regex::regexp` and `regsub` map it onto their existing
   result types: `RegexpResult::Count { assign: None }` keeps its meaning
   — a completed no-match, match variables untouched — and a
   `PrecisionDecline` becomes a `RegexError` on the runtime path and a
   typed decline on the analysis path. `RegsubResult` gains the same
   discrimination, so a `regsub` whose match was cut short does not return
   the unsubstituted text as if nothing had matched.
4. `tcl_regex::cmd_core::AreEngine` is the one provider to update; it maps
   the engine's spans onto `RegMatch` and a decline onto the new variant.
5. The direct route's `ConstOps` poisons on any `Declined`, so
   `ConstOps::take` reports the reason and the analyser widens.

A consumer that needs only match existence uses a separately certified
exact-existence result, which is a different claim with its own
certification and is not derivable from this enum; capture consumers need
exact captures. The route declines the whole result on any
approximation. This is a contract change for existing regexp consumers,
with focused compatibility tests, and it precedes any widening of
regexp-derived constants or branch pruning.

### The witnesses

Three fixed witnesses pin the distinction, all run on 8.4.20, 8.5.19,
8.6.18, 9.0.4, and 9.1b0, and unanimous on all five. Spans below are
`RegMatch`'s half-open `[so, eo)`; the `-indices` column is Tcl's own
inclusive spelling.

| Program | `-indices` on every release | `RegexpPrecision` |
|---|---|---|
| `regexp -indices {^a*(b)\1$} <300×a>bb whole g1` | result 1, `whole` = `0 301`, `g1` = `300 300` | `Exact { whole: 0..302, groups: [Some(300..301)], captures_exact: true }`, never `NoMatch` |
| `regexp -indices {(x)*} <300×x> whole g1` | result 1, `whole` = `0 299`, `g1` = `299 299` | `Exact { groups: [Some(299..300)] }` exactly, never an approximated span |
| `regexp {^(a+)+\1$} <300×a>` and `regexp {^(a+)+b$} <300×a>` | 1 and 0 respectively, both under 2 ms | `Exact` and `NoMatch`; a fuel-exhausted engine would answer the *opposite* of the second, which is why `FuelExhausted` cannot be spelled `NoMatch` |

The third pair is the one that makes the enum load-bearing rather than
tidy: the oracle answers definitively and quickly on all five releases,
and the failure mode of collapsing exhaustion into no-match is a wrong
constant, not a lost one. The pair holds for a subject of 301 `a`
characters as well, so the witness is not an artefact of an even-length
run.

### Options, target semantics, and the forms

`-inline` returns a list and writes nothing; an ordinary no-match
preserves the match variables — `set a before; set b before;
regexp {(x)(y)} zz a b` is 0 with both variables still `before` on every
release — and unmatched subgroups on a successful match have their own
empty-string or `-1 -1` index result:
`regexp -inline -indices {(a)(b)?} ac` is `{0 0} {0 0} {-1 -1}` on 8.4,
8.5, 8.6, 9.0, and 9.1. `-all` and zero-length matches follow the core's
progress rules — `regexp -all {a*} xaax` is 3 and
`regsub -all {} abc -` is `-a-b-c` on every release. A regexp-based branch
fact and a regexp output-variable transfer derive from one evaluated match
outcome. Two forms are release-gated and the core refuses both:
`regexp -about` answers `1 {}` on every release from 8.4 to 9.1 and the
core returns `regexp -about is not yet supported`, and `regsub -command`
is `bad switch "-command"` on 8.4 and 8.5, `bad option "-command"` on 8.6,
and answers `Abc` for `regsub -command {a} abc {string toupper}` on 9.0
and 9.1 while the core refuses it. Both are `FormUnsupported` on the
analysis path; a callback form needs a declared route on the callback
before it can be anything else. Index units follow the target release's
character model, and copied substrings keep their exact spelling.

### The pattern cache, its bound, and the cancellation point

Compiled patterns are cached in one bounded, thread-local cache keyed by
everything that can change a compiled pattern or its meaning:

```rust,ignore
/// The compiled-pattern cache key. Every field is part of the identity a
/// compiled `Regex` is only valid under.
struct PatternCacheKey {
    /// The pattern's exact bytes, never a normalised or trimmed form.
    pattern: Rc<[u8]>,
    /// `RegexFlags`' four booleans, packed: nocase, expanded, linestop,
    /// lineanchor.
    flags: u8,
    /// The engine's identity and revision, so a change to `tcl-regex`
    /// cannot serve a stale compilation.
    engine: EngineIdentity,
    /// The target's character model and byte-string encoding, the two
    /// axes that change what a pattern matches.
    target: (Option<StringCharacterModel>, Option<ByteStringEncoding>),
}
```

The bound is on retained bytes, not entry count: the cache holds at most
4 MiB of compiled patterns and evicts the coldest entry first, because one
pathological pattern compiles to far more than an average one and an
entry-count bound does not limit memory. Pattern compilation is charged
before it runs, as the pattern's length squared — the parser's worst case
— and a compile past the charge is `Cancelled`, not a partial entry.
Matching is charged as the fuel the engine spends, and returned captures
as their bytes.

Cancellation reaches the match loop at exactly one place, and it is the
place the fuel counter already is: `Matcher::spend_fuel` and
`spend_fuel_n` in `rust/tcl-regex/src/exec.rs`, called from the frontier
expansion in `reach` and from every backtracking node visit in `Bt::m`.
The token is read there, and exhaustion and cancellation produce different
variants so a transient stop never becomes a permanent imprecision. A
command-level deadline does not interrupt one long native match by itself;
this point is why it does not have to. Resource exhaustion is a decline,
never "no match".

### Compatibility is evidence

Capture indices, Unicode, flags, malformed patterns, substitutions,
no-match writes, and target-release differences are validated against C
Tcl with deterministic witnesses; fuzz campaigns stay in the manual tier.
Static pattern diagnostics such as the ReDoS checks ask a different
question from matching one concrete subject: they consume the shared
pattern structure where available, and a successful bounded example match
proves nothing about all subjects, just as a timeout proves no
vulnerability.

## The declared-implementation route

### What exists

`tcl-spec-hooks` builds one `tcl-vm` engine per pack per thread
(`rust/tcl-spec-hooks/src/host.rs`), compiles each hook body once to a
proc, whitelists twenty-nine commands (`SANDBOX_COMMANDS` in
`rust/tcl-spec-hooks/src/sandbox.rs` — among them `set`, `incr`,
`lappend`, `foreach`, `while`) plus the `foldlist` host builtin, and
enforces a budget of 100,000 commands and 250 ms of wall clock per
invocation with a 16 MiB cap on any value (`HostConfig::default`); an
error is an abstention, a budget overrun quarantines the hook, and a
caught panic poisons the pack — `PackRuntime::poisoned` is that blast
radius ([spec-dsl-examples/README.md](../spec-dsl-examples/README.md)
§ *Purity and the sandbox*). The sandbox excludes `source` and `package`,
and the hook setup loads no package implementation. `tcl_engine_api::Engine`
(`rust/tcl-engine-api/src/lib.rs`) compiles a unit once, invokes it with
values, restricts the command surface through `restrict_commands`, and
enforces a `Budget` of `commands` / `wall_clock` / `max_value_bytes` whose
overrun it reports as `EngineError::BudgetExceeded(BudgetKind)`;
`TclVmEngine` (`rust/tcl-engine-tclvm/src/lib.rs`) is its only
implementation, builds a `Vm` with the default registry at the default
release, and exposes no release setter although the VM has one. The host
reaches the compiler through the per-thread `pack_hooks` installer
(`set_installer`, `install_host`), which is how the crate cycle is broken.

### Per-evaluation state: writes outside the activation are denied

Removing clock, I/O, `rand`, and `srand` from a whitelist does not make a
body deterministic. The host keeps one engine per pack, the sandbox permits
`incr` and `set`, and a qualified global needs no `global`, so a body
`fold [incr ::counter]` answers `1` on its first call and `2` on its second
through the actual slot and folder thunk. A content cache can hide that;
it cannot make the computation pure.

Of the three isolation mechanisms — a resettable snapshot, denying writes
outside the local activation, or discarding an execution context after
each evaluation — the route takes the second, and the other two are
rejected for stated reasons:

- **A resettable snapshot** needs a new `Engine` capability whose
  correctness is the engine's own completeness. A slot the reset forgets is
  a silent leak, and its failure mode is a wrong constant rather than an
  error. Nothing in the answer says the reset was incomplete.
- **Discarding the execution context** means a fresh engine per
  evaluation, and `install_pack_hooks` compiles every body at install. A
  fresh engine recompiles the pack's bodies per evaluation, which turns the
  measured 24.5 ns cached answer into a compile and puts the cost in the
  wrong place entirely.
- **Denying writes outside the activation** needs no engine capability,
  because the door it closes is already nearly shut: `upvar`, `global`,
  `variable`, `namespace`, `trace`, `uplevel`, and `info` are all off
  `SANDBOX_COMMANDS`, so the only remaining way out of the activation is a
  qualified name in an ordinary store write.

The mechanism, specified:

- The host replaces the four store-writing whitelist entries — `set`,
  `incr`, `lappend`, `lassign` — with activation-scoped host commands
  registered through `Engine::define_command`, which the engine already
  supports and which `builtins()` already uses for `foldlist`. The
  replacement, `ActivationStore`, refuses a `::`-qualified name, an array
  element whose base name the activation did not create, and any name
  outside the activation's own frame; it accepts everything else with the
  whitelisted command's exact semantics, so a body's locals, loops, and
  accumulators are unchanged.
- A refusal is an ordinary Tcl error, which is already an abstention, so
  silence stays the conservative answer and no new answer kind appears at
  the emitter protocol.
- Nothing outside the activation is writable, so nothing has to be reset
  between evaluations. The rule therefore covers several bodies in one
  pack — they cannot see each other's writes — and several analysis
  threads, which already have one engine each because `Engine` is
  `&mut self` and `Rc`-based and so thread-confined.
- The read side closes with the same list. A body that *reads* `::counter`
  reads the empty string, because nothing ever writes it. That is a
  decline only when the answer depends on it, and the capability's
  `depends` list is what makes such a dependency declared;
  `spectcl_check`'s `ctx_keys` and `unknown_ctx_keys` report is the
  author-facing half.
- The witness is the one the test anchors name: a body `fold [incr ::counter]`
  raises under `ActivationStore`, the evaluator declines, and the answer is
  identical on the first call and the thousandth.

### Two policies, not one whitelist

The commands available to *implement* an evaluator are distinct from the
subject invocations *eligible* for evaluation. Whitelisting an ensemble by
one form's purity exposes every subcommand unless dispatch is constrained;
an apparently pure subject form may invoke a callback or a replaceable
math function.

**The implementation surface is a list.** Exactly `SANDBOX_COMMANDS` —
`set`, `expr`, `if`, `while`, `for`, `foreach`, `switch`, `return`,
`break`, `continue`, `incr`, `lappend`, `lassign`, `list`, `lindex`,
`llength`, `lrange`, `lreplace`, `lsearch`, `lsort`, `join`, `split`,
`string`, `format`, `scan`, `regexp`, `regsub`, `dict`, `binary` — with
`set`, `incr`, `lappend`, and `lassign` supplied by `ActivationStore`;
plus the `builtins()` host command `foldlist`; plus the family's emitter
verbs. None of the four store writers is pure, and all four are necessary
facilities: that is the point of separating the two policies. `regexp` and
`regsub` inside a body reach the same `AreEngine` and therefore the same
`RegexpPrecision`, and a body that consumes a declined match declines.

**The eligible-subject set is a rule, not a list.** An analysed invocation
is eligible for evaluation when, and only when, all four hold:

1. Its resolved specialisation — `ResolvedInvocation`'s
   `(command, subcommand, form)` identity, not its command name — declares
   a route at the selected scope, under the interface contract's three
   declaration states.
2. Its head's binding is valid, as the interface contract's decline table
   states.
3. Every entry in the capability's `inputs` list resolves to an exact
   value at this program point.
4. Every axis in the capability's `target` set is satisfiable for the
   analysis context, by the same `admit` rule the direct route uses.

Because eligibility is keyed on the resolved form, declaring `string`
eligible on the strength of `string length` cannot expose `string is` or
`string cat`: those are different resolved identities with their own
declaration states, and the default state is inherited-from-declined.

### The capability declaration

```rust,ignore
/// Everything a declared implementation states about itself. Part of the
/// specialisation's identity, and therefore of every memo key.
struct EvaluatorCapability {
    /// Which implementation this evaluator models, and at what revision:
    /// the pack name, the declared id, and the content hash of the body
    /// or of the provisioned file. `PackRuntime` already carries the
    /// pack's `content_hash`, `dsl_version`, and name.
    identity: ImplementationIdentity,
    /// Where it runs. `BoundedTcl` is the only host word; the variant
    /// exists so a second host is a declaration rather than a
    /// reinterpretation of the first.
    host: HostKind,
    /// The target semantics it supports, as the same `Needs` bits the
    /// direct route admits. An axis absent here is an axis the evaluator
    /// declines, and `PLATFORM` and `WALL_CLOCK` are unsatisfiable here
    /// too — the host denies both.
    target: Needs,
    /// Exactly which inputs it reads: operand indices that must be
    /// exact, target places whose incoming value it reads, and options
    /// whose value it reads. Nothing outside this list is supplied, and
    /// a body that reads a target it did not list is a `spectcl_check`
    /// finding.
    inputs: Vec<DeclaredInput>,
    /// The context dependencies the answer carries and the memo key
    /// holds: the target profile, the implementation identity, the
    /// registry and overlay generation, the evaluator generation.
    depends: Vec<ContextDependency>,
    /// Its own budget, capped by the host's and charged to the request.
    /// This is `tcl_engine_api::Budget`, the engine's caps — not the
    /// interface contract's `Budget` handle the evaluator is passed.
    budget: tcl_engine_api::Budget,
    /// Which completion kinds it models. Normal only: an implementation
    /// that raises is a decline, never a completion fact, under the DSL's
    /// `Error means abstain` rule.
    completion: CompletionSupport,
}

enum DeclaredInput {
    /// Operand `index` must be an exact value.
    Operand { index: usize, exactness: Exactness },
    /// The incoming value and existence of target `index`.
    IncomingTarget { index: usize },
    /// The value of option `name`, when present.
    OptionValue { name: &'static str },
}

enum ContextDependency {
    TclProfile,
    ImplementationIdentity,
    RegistryGeneration,
    EvaluatorGeneration,
    /// One named math function or nested command binding.
    Binding(CommandBindingIdentity),
}
```

A resolver that cannot represent "pure, but no evaluator" is corrected by
`EvalRoute::None`, not by making purity double as executable backing.

### `Engine::set_release`

```rust,ignore
trait Engine {
    /// Pin every subsequent compilation and invocation to `profile`.
    ///
    /// Default: `Err(EngineError::Unsupported("pinning a release"))`, so
    /// an engine that cannot pin says so rather than running at its own
    /// default while the caller believes otherwise — the same contract
    /// `set_budget` already has for an unenforceable budget.
    fn set_release(
        &mut self,
        _profile: &'static DialectProfile,
    ) -> Result<(), EngineError> {
        Err(EngineError::Unsupported("pinning a release"))
    }
}
```

`TclVmEngine` implements it by calling `Interp::set_dialect_profile` in
`rust/tcl-vm/src/interp.rs`, which already pins the interpreter to one
`DialectProfile`; the wrapper adds only the release identity the memo key
carries. What `set_dialect_profile` does decides the wrapper's contract:
it bumps the command epoch, and on an actual profile change it increments
`profile_generation`, clears `eval_cache`, `eval_cache_plain`, and
`module_procs`, resets the root interpreter's standard-channel configs, and
installs the release's numeral grammar through
`tcl_syntax::number::set_runtime_syntax`. Its own documentation states that
the VM does not support switching release mid-execution. So:

- `set_release` is called once per (pack, profile) pair, after the engine
  is built and **before** `Engine::compile`, never between `compile` and
  `invoke`.
- A pinned profile is part of the pack's engine identity. Analysing the
  same pack against a second profile builds a second engine and recompiles
  its bodies; it is a pack reload, not a per-call setter.
- Because the grammar install is process-wide per thread, the host holds
  one engine per (pack, profile, thread) and the profile is part of the
  key that finds it.
- A pack whose capability names a release the engine cannot pin gets
  `Unsupported`, which is a load notice and no route — never a silent run
  at the engine's default.

### Provisioning is a pinned path

An implementation body reaches the engine three ways and no others:

1. **Embedded in the pack source.** The `body` block in the `.tclspec`,
   carried verbatim by the loader as `HookSource::Body` does today, and
   hashed into the pack's `content_hash`.
2. **A file the pack names, inside the pack's own discovery tier.** The
   loader resolves it at load against `rust/tcl-spectcl/src/discovery.rs`'s
   `Tier` and `Origin` — `StudioOverride`, `Workspace`, `User`, `Shipped`,
   and the host-supplied `VIRTUAL_PACK_MOUNT` — and hashes its content into
   the same identity. A path that resolves outside the pack's own tier is a
   load notice and no route.
3. **A package implementation embedded with the pack at build time**,
   under the established package policy.

`source` and `package` stay off `SANDBOX_COMMANDS`, so no body can widen
this at query time; every path is resolved at load, never because a
workspace file was opened during analysis. The iRules test simulator's
registry-generated stubs in `rust/tcl-irule-test/tcl/_mock_stubs.tcl`
dispatch through one `_stub` proc that logs a decision and returns the
empty string; they are a simulator's fallbacks and are not evaluators.

### The rest of the route contract

- **Unknown inputs stay unknown.** If a required operand is not exact the
  body is not invoked with a placeholder, the engine does not read a host
  environment, and one sampled run is never a proof; partial abstract
  reasoning stays with the analyser.
- **The execution realm is not the subject program.** The engine's own
  builtins are not evidence about the analysed program's bindings. Binding
  validity comes from the analysis context, transitively over every
  implementation the answer used, and redefining the analysed command
  afterwards cannot reuse an old answer because the spelling is unchanged.

The route is never used for BPF-Tcl and never for vendor filter strings.

```mermaid
flowchart LR
    T[".tclspec<br/>evaluate -implementation ID … { body }"]
    T -->|parses| L["loader · hook_source grammar<br/>-native resolves by SCOPE::FIELD<br/>catalogue for every family"]
    L -->|installs| P["pack_hooks slot tables<br/>declared inputs → cache eligibility<br/>(target values included)"]
    P -->|runs| H["bounded host<br/>one engine per pack per profile per thread;<br/>ActivationStore denies writes out of frame;<br/>release-pinned; budgeted; cancellable"]
    H -->|answers| E["fold · write · preserve<br/>silence or error → decline"]
    E --> V["validate → memo keyed by evaluator identity,<br/>exact inputs, incoming targets, context deps"]
    V --> S["the transfer driver"]
```

## One context, one memo

Every evaluation runs under the immutable analysis context the interface
contract defines, carried unchanged through lowering, unit construction,
per-function queries, optimiser consumers, and evaluator calls.

### The memo key

```rust,ignore
/// One invocation's evaluation memo key. Every field is something that
/// can change the answer; nothing that cannot is in it.
struct EvalMemoKey {
    /// The resolved specialisation: which command, subcommand, and form,
    /// and which declaration state supplied the route.
    specialisation: SpecialisationId,
    /// The route, its implementation identity, and its revision.
    route: EvalRouteId,
    /// Every declared input's exact value, byte for byte, in operand
    /// order. An absent optional input is distinguishable from an empty
    /// one.
    inputs: Rc<[Option<ExactValue>]>,
    /// Each declared target's incoming value *and* existence, because
    /// `incr` on an absent variable raises on 8.4 and creates it from
    /// 8.5, and `preserve` needs the prior state.
    incoming_targets: Rc<[TargetState]>,
    /// The admitted target semantics, as a hash of `TargetSemantics`.
    target: TargetDigest,
    /// Every declared context dependency, in a canonical order.
    depends: Rc<[ContextDependency]>,
    /// The evaluator-capability generation, below.
    evaluators: EvaluatorGeneration,
}
```

Today's hook cache in `rust/tcl-registry/src/pack_hooks.rs` has neither
of the two fields that matter here. `ShapeKey` carries `slot`, `nwords`,
two bits of `kinds` per word for up to 64 words, a `version`
discriminant, `in_event_body`, and a `content` hash of the words' literal
values — no incoming target value and no dependency set. Declared input exposure and cache eligibility move
together — `CacheMode::of(inputs)` is already that rule, over
`HookInputs::shape_only` and `content_cacheable` — and a new spelling in
the DSL is not enough on its own. A content hash is an index, not evidence
that two inputs are equal; `content_hash`'s `DefaultHasher` over the word
values is exactly such an index. A proof-bearing cache verifies equality on
a hit, so `EvalMemoKey` holds the input values themselves, interned, and
the hash is the bucket rather than the proof.

### Invalidation

| What changes | What it invalidates | Mechanism |
|---|---|---|
| a declared input's value | that one entry | it is part of the key |
| a target's incoming value or existence | that one entry | it is part of the key |
| the target profile | every entry under the old profile | `TargetDigest` |
| a pack reload | every entry of that pack's evaluators | `EvaluatorGeneration`, and the `clear_cache` that `allocate_stable` already performs when it rebinds a slot |
| a hook quarantine | every entry of that pack's evaluators | `EvaluatorGeneration` |
| host install or removal | every entry | `EvaluatorGeneration`, at `install_host` / `clear_host` |
| a `rename`, `proc` redefinition, or namespace opacity change | every entry whose `depends` names the affected binding, and every per-procedure lattice in the file | `ModuleCommandMutations` in the context; `CommandTrustSnapshot` is its hashable form |
| a registry or overlay generation change | every entry | `RegistryGeneration` in `depends` |
| a trace or escape fact | the affected place's transfers, through the solver | the existing observability owners |

A stale memo is an invalidation defect, not something an optimiser re-run
repairs. A second run is justified only by additional explicit assumptions
— a proven `TclOO` frame (`oo_defining_class`), which the shared lattice never
has — and is keyed as that different context.

### The evaluator generation

The engine host is thread-local, mutable, and can quarantine a hook. Its
availability and health must not silently change the answer to an
otherwise identical memoised query. Of the three ways to close that — a
stable evaluator snapshot supplied to each query, a capability generation
in the context, or keeping optional execution results outside canonical
proof facts — the route takes the second:

```rust,ignore
/// Bumped whenever the set or health of this thread's evaluators
/// changes. Part of the analysis context, so it enters every key once.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct EvaluatorGeneration(u32);
```

The reasons are concrete. The context is already one interned value
carried through every query, so one `u32` costs nothing and makes host
presence, pack reload, and quarantine visible in the key by construction. A
snapshot supplied per query needs a second carrier and a rule for what
happens when the two disagree. Keeping execution results outside canonical
facts contradicts ruling 3, which puts workspace-authored facts on the same
footing as a shipped spec.

The generation is bumped at exactly the three places that already call
`clear_cache()` in `rust/tcl-registry/src/pack_hooks.rs`: `install_host`,
`clear_host`, and the host's quarantine path. A worker with no host has a
distinct generation, so its honest declines are keyed as such and a
host-present worker and a host-absent worker never share an entry. Tests
cover host-present and host-absent workers, pack reload, and quarantine.

The salsa side of the same rule: `compilation_unit` and `function_lattice`
in `rust/tcl-lsp-db/src/lib.rs` resolve `db.registry`, the un-overlaid
registry, while `Analyser::with_pack_overlay` and the semantic-token
queries read the pack-specialised one; `FnLatticeKey` cannot carry
`ModuleCommandMutations` today, although `CommandTrustSnapshot` exists as
the hashable form of that binding fact and the key already carries two
whole-module facts of the same kind (`traced_variables`,
`has_dynamic_variable_trace`). The context enters the key once; a `rename`
anywhere in the file then invalidates every per-procedure lattice in it,
which is the correct sensitivity and the one the analyser's deferred-body
memo already has.

Dynamic dependencies must stay acyclic even when the crate graph is: SCCP
invokes an expression, whose `command` service asks for the same canonical
function lattice, whose computation requires the same expression; or a
host compiles a hook body using the pack evaluators it is still installing.
A callback reads the supplied current analysis state or follows a
separately specified nested evaluation protocol, and never demands the
completed query being computed; installation has a bootstrap mode
independent of the uninstalled evaluator; the host's execution realm stays
distinct from the subject program's proven bindings; an unsupported cycle
is a typed decline, not a depth cap that makes the answer meaningful.

## Budgets and cancellation

The existing per-invocation limits are containment, not an interactive
performance contract: ten thousand cache misses at twenty milliseconds
each cost two hundred seconds while every one stays far below the 250 ms
cap.

### Units and charges

The unit is one `WorkUnits` — one elementary step, defined so that a
million units is a few milliseconds of native work on the machines the
acceptance suite runs on. Each route converts its own work into the same
unit, so one request budget bounds all three:

| Route | What is charged | Conversion |
|---|---|---|
| direct core | input bytes examined, output bytes produced, list and dict elements, sort comparisons | the "Charge" column of the core table; output charged *before* the allocation |
| direct core admission | one `ConstOps::admit` | a fixed charge, so a decline loop is bounded |
| regexp compile | one `RegexEngine::compile` | the pattern's length squared, the parser's worst case, charged first |
| regexp match | the engine's own fuel | one unit per `MATCH_FUEL` unit spent, plus one per returned capture byte |
| expression | `tcl_syntax::expr::eval` node visits | one per `ExprNode` visited, plus the nested callback's own charge, plus the math function's |
| declared implementation | `Engine::commands_spent()` and the value-size cap | one per dispatched command; allocation through `Budget::max_value_bytes` |
| result construction | bytes published into a fact | one per byte |
| cache growth | one memo insertion | one per inserted entry plus one per retained byte |

Aggregate retained memory is limited, not only the largest individual
value: the request budget carries a retained-bytes ceiling alongside its
work ceiling, and the pattern cache's 4 MiB and the memo's retained bytes
both count against it.

### The three nested budgets

```rust,ignore
/// One editor or CLI request. Outermost; everything charges through it.
struct RequestBudget {
    work: WorkUnits,
    retained_bytes: u64,
    deadline: Instant,
    cancel: CancelToken,
}

/// One solver iteration inside a request, so a fixed-point loop cannot
/// spend the whole request in its first pass.
struct IterationBudget<'r> {
    request: &'r mut RequestBudget,
    work: WorkUnits,
}

/// One evaluation inside an iteration. This is the level the existing
/// `tcl_engine_api::Budget` already describes, and a declared
/// implementation's own `budget` block narrows it further.
struct EvaluationBudget<'i> {
    iteration: &'i mut IterationBudget<'_>,
    engine: Budget,
}
```

The per-evaluation defaults are `HostConfig::default`'s: 100,000 commands,
250 ms, 16 MiB per value. The per-iteration default is one tenth of the
request's *remaining* work, so the first pass of a fixed point cannot
starve the last. The per-request defaults are the interactive latency
target — 200 ms of evaluation work and 64 MiB retained — and the
acceptance measurement below is what sets the exact numbers. A declared
implementation's `budget` row narrows, never widens: a value above the
host's is a load notice and the host's value stands.

### Cancellation points

Cancellation reaches expensive native operations, because the VM's command
counter does not count every inlined bytecode operation, its own
documentation relies on wall-clock polling for that case, and a long core
call is not interrupted at a Tcl command boundary. There are five points,
and every route passes through at least one:

| Point | Where | Reached by |
|---|---|---|
| between evaluations | the generic transfer driver, before it resolves a route | every route |
| each expression callback | `ExprOps::{var, command, call}` in the `expr` adapter | expression |
| each regexp fuel charge | `Matcher::spend_fuel` / `spend_fuel_n` in `rust/tcl-regex/src/exec.rs` | regexp, and `switch -regexp`, `lsearch -regexp` |
| before each allocation | `ConstOps::charge_bytes`, and each list or dict element step | direct |
| each engine command and wall-clock poll | `Engine::commands_spent` and the host's clock check | declared implementation |

On cancellation or exhaustion the analysis is sound and incomplete, never
an exact negative answer. Caches distinguish a deterministic unsupported
case from a transient scheduler or host failure: `PrecisionDecline::Cancelled`
and `FuelExhausted` are different variants for exactly this reason, a
transient failure never permanently disables future precision, and a retry
never mutates an already published snapshot's meaning. Tests cover warm
and cold caches, deliberate churn, many packs, and worker migration, not
only a fast repeated hook. The 28 µs uncached and 24.5 ns cached figures
measured for existing hook workloads describe those workloads; they are
not measurements of this service or of arbitrary command implementations,
and performance acceptance compares the unchanged tree, direct-core
evaluation, expression evaluation, and declared execution on the same
workloads — cold host setup, warm evaluation, cache hits, changed inputs,
solver iterations, cancellation latency, memory, and incremental editor
latency.

## Target semantics

Passing a `TclVersion` does not make an engine implement that release. For
each route the contract states the numeric syntax, character and index
model, regexp features and limits, binary representation, platform
behaviour, and completion semantics it supports; an unsupported
combination declines. A profile with no release of its own is decided by
unanimity: a route folds where its answer is proven identical under every
release the profile can denote, and declines `ReleaseAmbiguous(axis)`
where two of them differ. A dialect that declares a base release
evaluates under that release — iRules on its 8.4-derived engine — and an
axis its pack declares divergent is a disagreement, which blocks the fold
(the owner's ruling of 2026-09-22). `TargetSemantics::of` takes the
release a profile declares, `DialectProfile::runtime_version` (its
`runtime_base`): each plain Tcl profile's own, iRules, iApps and tmsh on
8.4, `expect` on 8.6, each EDA shell on its vendor's. An axis on which the
profile declares an answer of its own that its release does not give — the
F5 dialects' `character_model`, which no TMOS measurement settles yet —
answers by unanimity instead, so the declaration blocks the base's answer.
A profile declaring no release (`tk`, the version-less `tcl` profile,
`f5-bigip`) answers every axis by unanimity. `TclVersion::from_profile` in
`rust/tcl-dialect/src/version.rs` still answers only for the five plain
Tcl profile names; routing a vendor profile's point through the evidence
gate per measured row is the consumer-contracts lane's CC9.2, and it
feeds the same field. `HookCall` carries `dialect` (the profile name,
deliberately not derived from `version`) and `version` (the `TclVersion`,
`None` when the profile names no release).

### The matrix

*Supported* means the route answers under the named axis. *Declines* means
the route answers `Declined` whenever the axis is requested. *Evidence*
means the route answers only for rows an independent oracle has measured,
and declines the rest.

| Axis | Direct | Expression | Declared implementation | None |
|---|---|---|---|---|
| numeric syntax (`NumberSyntax`, the integer tower, the operator set) | supported: `NUMERAL_GRAMMAR` and `INT_TOWER` are explicit arguments through `format_cmd_with_syntax`, `class_check`, and `tcl_syntax::number::parse`; under a profile that names no release, answers where every release it can denote agrees on the operand and declines `ReleaseAmbiguous(NumeralGrammar)` or `(IntTower)` where they differ | supported: `FoldOps` already carries a `NumberSyntax`, and the `Big` rung models the bignum tower | evidence: the engine's own release, pinned by `Engine::set_release`; declines when the engine returns `Unsupported` | declines |
| character and index model (`CHAR_MODEL`, `CHAR_INDEXING`, `INDEX_GRAMMAR`, `SOURCE_ENCODING`) | supported for counting through `StringCharacterModel::count_for` and for indices through the adapter's pre-resolution; under a profile that names no release, answers where every release it can denote agrees and declines where they differ — a supplementary character, a non-ASCII source literal, a leading-zero index | supported: an expression reads no character index; a string operand's ingress is the direct route's admission | evidence: the pinned release decides, and the host's 16 MiB value cap bounds the result | declines |
| regexp features and limits | supported through `AreEngine` with `RegexpPrecision`; declines `-about`, `regsub -command`, every `PrecisionDecline`, and a non-ASCII `-nocase` exact pattern (the core folds with `eq_ignore_ascii_case`) | supported only where an operand is already an exact value; the engine itself has no regexp operator | evidence: a body's `regexp` reaches the same engine and the same precision result | declines |
| binary representation (`BINARY_FIELDS`, `BYTE_STRINGS`) | supported: `specifier_min_version` and `signedness_available` gate the field grammar, and `ConstValue` is byte-exact with `Representation` as separate evidence | declines: the expression engine has no byte-array rung | evidence: the engine's own value model; a `binary` result crosses the boundary as bytes or declines | declines |
| platform behaviour (`PLATFORM`) | declines always: `PLATFORM` is never satisfiable, so `platform::exec` and `platform::pwd` are unreachable by construction | declines | declines: the host denies ambient files, network, clock, and randomness | declines |
| completion semantics | supported: the normal path from slice 2, and from slice 10 the interface contract's `CompletionOutcome::Error` under the prefix rule; before slice 10 an error is a decline | supported: the normal path, a short circuit being a normal path, and from slice 10 the exact error completion (`expr {1/0}`); before slice 10 an error is a decline | supported for the normal path only, through `CompletionSupport`; `EngineError::Script` is a decline and `BudgetExceeded` is a distinct one | declines |
| wall clock and locale (`WALL_CLOCK`) | declines always: `clock::dispatch` requests it and it is never satisfiable | declines | declines: `after` and `clock` are off the whitelist | declines |

### What "consistent" can mean

- **Among the direct route, the VM, and the WASM runtime** — achievable by
  construction: one function over one seam, with the adapter caveats above.
- **With C Tcl** — an evidence question. The cores are a port; the
  `tclsh` differential and the fuzzer's `tclsh` pair are the oracle, and a
  shared bug is invisible to a two-way native pair, so agreement between
  our compiler and our runtimes can mean both share a defect. Three kinds
  of divergence are known today. `index::resolve` reads index numerals
  under the ambient grammar for every release. Three `-nocase` folds are
  ASCII-only — `switch::select`'s exact mode, `lsearch`'s sorted-list
  comparison, and `sort`'s `-nocase` and `-dictionary` orders — while C
  Tcl folds the full range on every release: `string equal -nocase` of
  U+00C9 and U+00E9 is 1 on 8.4, 8.5, 8.6, 9.0, and 9.1, and
  `switch -nocase -- <U+00E9> {<U+00C9> …}` selects the arm on 8.5
  through 9.1, where an ASCII-only fold answers no match. And
  `regexp -about` and `regsub -command` are refused although C Tcl
  answers them — `regexp -about {a(b)c}` is `1 {}` on all five releases,
  and `regsub -command {a} abc {string toupper}` is `Abc` from 9.0.
  Independent target oracles and adapter-level tests stay; a missing
  target fact never falls back to the build machine's platform or the
  installed engine's default profile.
- **Across releases** — the profile decides; a dialect that declares a
  base release gets that release's answer, and a profile that names no
  release gets the unanimous answer or a decline, the rule the cores'
  explicit `NumberSyntax` arguments already enforce. Each test run records
  the oracle release and platform, and the oldest relevant release is
  tested for each supported behaviour, not whichever `tclsh` is on `PATH`.
  The character-model axis needs all five releases and not two, because
  the answers are not two-valued: `string length` of a supplementary
  character reached through `encoding convertfrom utf-8` is 4 on 8.4 and
  8.5, 2 on 8.6, and 1 on 9.0 and 9.1, so a rule written from an 8.6/9.0
  pair is wrong for 8.4 and 8.5.

## Authoring on the routes

Today's `const_fold {words ctx} {…}` and `const_fold -native ID`, the
`hook_source` grammar (`FIELD ?-inputs {…}? {params} {body}`,
`FIELD -native ID`, or `FIELD KEYWORD` for a derivation), and the native
`CommandSpec` fields are the compatibility baseline. The spellings below
are *proposed*: `semantics`, `evaluate`, `facts`, and the route flags are
not loader syntax, and adopting them means the registry field, loader,
exporter, renderer, studio form, documentation, and parity tests move
together under [command-spec-studio.md](../contracts/command-spec-studio.md).
All of them are additive, so they land as DSL vocabulary **2.2** — the
minor after 2.1's `arg_role_resolver_roles` — and an older loader meeting
a 2.2 pack keeps loading and loses only the three statements, which leaves
the command known and its evaluation `Unknown`: the "shape or value word"
degradation the load policy already defines.

### `incr`: direct arithmetic, independent result and write

```rust,ignore
CommandSpec {
    name: "incr",
    semantics: registry_semantics!(incr::SEMANTICS),
    // Existing arity, roles, version, and documentation fields retained.
    ..CommandSpec::DEFAULT
}

// Registry-owned specialisation over the shared numeric owner.
fn evaluate(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let form = incr_form(input.invocation())?;
    let target = form.declared_target();
    let old = input.prior_store(target.place(), FactDomain::ExactValue);
    let step = form.increment_or_exact_default("1");
    let out = numeric_core::tcl_incr(old, step, input.context(), budget)?;
    EvalAnswer::Evaluated(InvocationOutcome {
        completion: CompletionOutcome::Normal,
        result: ExactValueOrUnavailable::Exact(out.value.clone()),
        ordered_stores: vec![StoreOutcome::Write { target, value: out.value }],
        types: TypeFacts { result: Some(TclType::Int), per_target: vec![(target, TclType::Int)], shapes: vec![] },
        evidence: input.context().binding_evidence(),
    })
}
```

```tcl
# Proposed equivalent declaration; the native binding owns the algorithm.
command incr {
    semantics -native incr::semantics
    evaluate -direct incr::evaluate
    facts -native incr::facts
}
```

The structural plan declares a variable read-modify-write and its possible
failure. A missing scalar is created from 8.5 and an error under 8.4 —
`unset -nocomplain q; incr q` raises `can't read "q": no such variable` on
8.4.20 and answers 1 on 8.5.19, 8.6.18, 9.0.4, and 9.1b0; an unknown
scalar is not silently zero; an array, traced, or aliased cell carries its
effects. The shared numeric operation implements the selected release's
parsing and bignum behaviour: `set x 010; incr x` is 9 on 8.4, 8.5, and
8.6 and 11 on 9.0 and 9.1, so a profile naming no release declines.
`incr x` can produce both `result = 4` and `x = 4`, and knowing the result
does not permit deleting the write or its traces. "Direct" never means an
unchecked Rust `+` or a Unicode scalar count.

### `expr`: the shared engine, not a miniature interpreter

```rust,ignore
CommandSpec {
    name: "expr",
    semantics: registry_semantics!(expr::SEMANTICS),
    ..CommandSpec::DEFAULT
}

fn evaluate(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let expression = expr_arguments::prepare(input.invocation())?;
    let ops = ProvenTclExprOps::new(input, budget, NestedPolicy::EffectFreeOnly);
    shared_expr_engine::evaluate(expression, ops)
}
```

```tcl
# Proposed: names the shared expression route, never an engine fallback.
command expr {
    semantics -native expr::semantics
    evaluate -expression tcl.expr
    facts -native expr::facts
}
```

`ProvenTclExprOps` adapts the existing `ExprOps` contract: resolve a
variable when reached; evaluate a supported command or math function only
when reached; retain ordering and completion. Its `NestedPolicy` is
`EffectFreeOnly` in slice 3 and `LocalWrites` from slice 9, the two
states of the interface contract's ordered evaluation state. The result
is a Tcl value, not necessarily a number. Braced and concatenated or unbraced arguments
have different evaluation stages — with `a` set to `alpha` and `b` to
`beta`, `expr {$a == $b}` is 0 on every release while `expr "$a == $b"`
raises `syntax error in expression "alpha == beta": variable references
require preceding $` on 8.4 and `invalid bareword "alpha"` on 8.5, 8.6,
9.0, and 9.1. Math-function and nested-command binding dependencies go
into the evidence and the cache key.

### `regexp`: our engine, typed precision, per-target outcomes

```tcl
# Proposed declaration; no diagnostic codes, no compiler callback IDs.
command regexp {
    semantics -native regexp::semantics
    evaluate  -direct regexp::evaluate
    facts     -native regexp::facts
    option -about -evaluate none -evaluate-reason form_unsupported   ;# the core refuses it
}
```

The native specialisation delegates to the shared regexp command plumbing
over `tcl-regex`, resolves flags once from the canonical invocation, and
returns a typed exact match, exact no-match, Tcl pattern error, or decline.
For `regexp {a(b)} $subject whole capture`, an exact match yields the
integer result and ordered writes to `whole` and `capture`; no match
preserves their previous values and existence; an unknown subject yields
bounded conditional writes and a known numeric result type without inventing
captures. `-inline`, `-indices`, and `-all` select different forms, `-about`
declines through the option row above, and the existing dynamic
return-type hook is an adapter into those same form semantics.

### A private command, without granting the pack analyser mutation

A pack command `tenant::label NAME` whose runtime implementation returns
`string cat "tenant:" $NAME` exposes a declared route and a partial-value
relationship without extending SCCP:

```tcl
# The entire block is proposed API, including capability names and fact syntax.
command tenant::label {
    arity 1
    semantics {
        effects {no_store_writes no_external_io}
        result -semantic string
    }
    evaluate -implementation tenant.label.v1 -host bounded_tcl {
        inputs {arg 0 exact}
        depends {tcl_profile implementation_identity}
        budget {-commands 2000 -wall-clock 20 -value-bytes 65536}
        body {name} { fold [string cat "tenant:" $name] }
    }
    facts {
        result -string_segments {{constant "tenant:"} {operand 0}}
        taint  -result_from {arg 0}
    }
}
```

If the argument is unknown the body is not invoked with a placeholder: the
answer is an unknown exact value plus the proven prefix segment and the
taint relationship, and the prefix does not sanitise the unknown suffix.
The author declares which implementation the evaluator models, and under
the rulings no independent certification of that claim is required; the
resolved binding and the implementation revision still decide when the
model applies. The bounded host denies ambient files, network, clock,
randomness, and undeclared globals, accounts for aggregate request cost,
and denies every write outside the evaluation's own activation —
execution and correctness contracts, not an author-trust gate.

### The `semantics`, `evaluate`, and `facts` rows

Three property statements, in the `hook_source` shape the loader already
parses, each with a `-native` form, a block form, and the explicit
abstention that makes the interface contract's third declaration state
writable. All are legal at `command`, `subcommand`, and `refine` scope, and
the innermost declaration wins.

| keyword | operands | meaning | from |
|---|---|---|---|
| `semantics -native ID` | one id | the shipped structural plan, by name | 2.2 |
| `semantics { … }` | one block | inline plan rows: `effects {…}`, `result -semantic T`, `stores -targets {…} -outcome O`, `iterate {…}` | 2.2 |
| `semantics none` | — | explicit abstention at this scope; the enclosing scope's plan does not apply | 2.2 |
| `evaluate -direct ID` | one id | a registry-owned Rust evaluator over the shared cores, resolved through the `evaluate` catalogue | 2.2 |
| `evaluate -expression ID` | one id | the shared expression engine under the named language profile (`tcl.expr`, `bpf.expr`) | 2.2 |
| `evaluate -implementation ID -host HOST { … }` | an id, a host word, one block | a declared implementation; `bounded_tcl` is the only host word | 2.2 |
| `evaluate -native ID` | one id | a shipped evaluator whose route the catalogue entry itself names | 2.2 |
| `evaluate none` | — | `EvalRoute::None`: declared "no evaluator for this form" | 2.2 |
| `facts -native ID` | one id | the shipped abstract transfer, by name | 2.2 |
| `facts { … }` | one block | inline fact rows: `result -representation R`, `result -string_segments {…}`, `taint -result_from {…}`, `range -integer_add` | 2.2 |
| `facts none` | — | explicit abstention; the generic transfer applies | 2.2 |

Inside an `evaluate -implementation` block, four rows and no others:

| keyword | operands | meaning | from |
|---|---|---|---|
| `inputs { … }` | repeated `arg N exact`, `target N incoming`, `option -NAME exact` | exactly what the body reads; anything unlisted is not supplied, and an evaluator that reads a target value without listing it is a `spectcl_check` finding | 2.2 |
| `depends { … }` | words from the closed set `tcl_profile`, `implementation_identity`, `registry_generation`, `evaluator_generation`, `binding NAME` | the context dependencies the answer carries and the memo key holds | 2.2 |
| `budget { … }` | `-commands N`, `-wall-clock MS`, `-value-bytes N` | the per-evaluation budget; narrows the host's, never widens it | 2.2 |
| `body {params} { … }` | a parameter list and a body | the implementation, carried verbatim as `HookSource::Body` already is | 2.2 |

The option-level forms are two flags on an option row, and only two,
because a route belongs to a *form* and not to a flag:

| flag | operands | meaning | from |
|---|---|---|---|
| `-evaluate none` | — | when this option is present the selected form has no evaluator, and the route declines | 2.2 |
| `-evaluate-reason WORD` | one word from the decline vocabulary | which decline the driver records: `form_unsupported` and `callback` are `NoRoute` with that `NoRouteReason`; `release_ambiguous` is `ReleaseAmbiguous` on the option's availability axis | 2.2 |

Those two flags are what `regexp -about` and `regsub -command` need, and
they are the whole of the option-level vocabulary. Anything richer — an
option that selects a *different* evaluator rather than removing one — is
written as a `refine NAME { evaluate … }` block, the per-form overlay
vocabulary 2.0 already provides, because a route belongs to a resolved
form and `refine` is the statement that names one.

### The body verbs

Inside an authored body the verbs are the family's emitter protocol, as
every other family's are, with silence meaning the conservative answer:

| verb | operands | meaning | silence |
|---|---|---|---|
| `fold VALUE` | one value | the invocation's result. Already shipped for `const_fold` | a decline, never an empty result |
| `write TARGET VALUE` | a validated target index and a value | one ordered write outcome, in call order | the target is unstated, which declines the whole answer by rule 3 |
| `preserve TARGET` | a validated target index | the target keeps its prior value and existence | as above |

Three rules make the protocol total:

1. **Silence is a decline.** A body that calls no verb has established
   nothing, exactly as `HookAnswer::Abstain` means today.
2. **A `write` to a non-target raises.** The target index must be one the
   structural plan validated as a place-bearing target; anything else is an
   error, and raising is a decline — the `Error means abstain` rule the DSL
   already states.
3. **A declared target with no verb is a decline, not a preserve.** A body
   that writes two of three targets and says nothing about the third has
   not stated what happens to it, so the whole answer declines. The
   `kv::split3` example in
   [value-transfers-examples.md](value-transfers-examples.md) writes
   `preserve 1; preserve 2; preserve 3` on its no-match path for exactly
   this reason.

`answer_of` in `rust/tcl-spec-hooks/src/emit.rs` remains the exhaustive
match that forces the verb and silence decisions for every family, and
`verbs_for` remains the one place a family's verbs are registered as host
commands, so the three new verbs are three `Emission` variants and three
arms rather than a new protocol.

### `-native ID`, and the per-family catalogues

`-native ID` must mean one thing, and today it does not. It resolves for
the closed compiler catalogues — `lowering_hook -native Switch` reaches
`LoweringHookId::Switch`, and `native_hook_tables_cover_their_catalogues`
in `rust/tcl-spectcl/src/loader.rs` pins the five tables (`LOWERING_HOOKS`,
`CODEGEN_HOOKS`, `INLINE_CODEGEN_HOOKS`, `ANALYSER_HOOKS`,
`RETURN_TYPE_HOOKS`) against `catalogue`'s own variant lists — but not for
the body families: `const_fold_versioned -native string::is` in
`docs/design/spec-dsl-examples/string.tclspec` installs the family's
abstention and nothing else, because no name-to-function table exists for
folders and `HookSource::Native` is consumed nowhere but reports. Two
spellings are in use, `command::subcommand` in that example and
`<command>::<field>` in the renderer's synthesised form, which is the
whole of the ambiguity.

The rule, stated once:

- **An id is `SCOPE::FIELD`.** `SCOPE` is the spec scope the field hangs
  off — the command name, `command::subcommand` for a subcommand-scoped
  field, `command::subcommand::-option` for an option-scoped one — and
  `FIELD` is the field's own DSL keyword. So `string`'s `is` subcommand's
  versioned folder is `string::is::const_fold_versioned`, and the
  renderer's `probe::const_fold` is the command-scoped case of the same
  rule rather than a second convention.
- **A short form is a load notice naming the full spelling**, and the
  field installs nothing. That is what it does today; the notice is what
  is new, and it turns a silent abstention into an actionable one.
- **Every family gets a table.** `NativeEvalTables` holds one
  `&[(&'static str, FnPtr)]` per family, keyed by the full id: the eleven
  `HookFamily` variants — `ArgRoleResolver`, `CommandPrefixResolver`,
  `ScriptTimingResolver`, `ConstFold`, `ConstFoldVersioned`,
  `TaintSinkGate`, `ContextGate`, `LiteralArgumentValidator`,
  `ClauseShapeCheck`, `OptionArity`, `Constraints` — plus the three new
  fields `semantics`, `evaluate`, and `facts`.
- **`native_hook_tables_cover_their_catalogues` grows a row per family**,
  so a table that omits a shipped implementation fails the test rather
  than dropping an id at load. An id in the table but absent from the
  catalogue fails the same assertion from the other side.
- **`-direct` and `-expression` resolve through the `evaluate` table**,
  under the same rule, so a route flag cannot name an implementation the
  catalogue does not hold.

The `const_fold` family's table, as the shipped folders name it, is the
worked example:

| id | function | file |
|---|---|---|
| `string::range::const_fold` | `fold_range` | `rust/tcl-registry/src/commands/tcl/string_.rs` |
| `string::replace::const_fold` | `fold_replace` | the same file |
| `string::is::const_fold_versioned` | `fold_is` | the same file |
| `format::const_fold_versioned` | `fold_format` | `commands/tcl/format_.rs` |
| `regsub::const_fold` | `fold_regsub` | `commands/tcl/regsub_.rs` |
| `scan::const_fold` | `fold_scan` | `commands/tcl/scan_.rs` |
| `list::const_fold` | `const_fold::fold_list` | `rust/tcl-registry/src/const_fold.rs` |
| `lindex::const_fold` | `const_fold::fold_lindex` | the same file |
| `dict::get::const_fold` | `const_fold::fold_dict_get` | the same file |

The remaining `const_fold::fold_*` functions — `fold_concat`,
`fold_llength`, `fold_lreverse`, `fold_join`, `fold_split`, `fold_lrepeat`,
`fold_lrange`, `fold_dict_exists`, `fold_dict_size`, `fold_dict_keys`,
`fold_dict_values`, `fold_dict_create`, `fold_dict_merge` — take the same
`SCOPE::const_fold` spelling under their own commands, and each is one row.
The `evaluate` table's entries are the direct evaluators the migration's
slices land, one per resolved form, and its catalogue is what
`evaluate -direct` resolves against.

An unknown name is dropped with a load notice, and the renderer's
synthesised `FIELD -native <scope>::<field>` spelling is a `GAPS` entry
until the draft can recover the real name — the same `DraftOpaque` kind
`bpf_op` and `data_collection` already carry in
`rust/tcl-spec-studio/src/render_spectcl.rs`.

### The four surfaces, the parity tests, and `spectcl_check`

Registry, loader, renderer and export, and studio move together or carry a
`GAPS` entry, and five gates hold them together:

| Gate | What it forces |
|---|---|
| `rust/tcl-spec-studio/src/coverage.rs`'s exhaustive destructuring witness | fails to compile until `semantics`, `evaluate`, and `facts` are surfaced or marked `Surface::Excluded` with a reason |
| `native_hook_tables_cover_their_catalogues` | one row per family, including the three new fields |
| `value_tables_cover_their_catalogues` | one row per new closed vocabulary: the route kinds, the `-outcome` words, the `depends` words, and the `-evaluate-reason` words |
| `rust/tcl-spec-studio/tests/spectcl_roundtrip.rs` | a rendered-then-reloaded draft differs from its source only on `GAPS` keys; `export.rs` round-trips bodies verbatim as it does for `const_fold` |
| `rust/tcl-spec-studio/tests/reference_doc.rs` | `docs/references/command-spec/fields.md` is regenerated from the studio schema, so the three fields' help text is one string in `rust/tcl-spec-studio/src/help.rs` and not two |

The studio gains a route picker and a body box in the "Purity and folding"
cluster, closes its top-level-only carry-forward of hook bodies so a
subcommand's body survives a form edit, and offers a "try it" box over
`HookHost::install_pack_hooks` plus a synthetic `HookCall`.

`tcl-mcp`'s `spectcl_check` (`rust/tcl-mcp/src/spectcl.rs`) already
reports each hook's family, `shape_cacheable` with its reason, the
`ctx_keys` its body reads, `unknown_ctx_keys`, `declaration_conflict`, and
the family's `verbs`, `silence_means`, and `requires_all_literal`. The
routes join that report with three findings, each of which the existing
`CtxScan` and declaration machinery can reach:

| Finding | How it is detected | Why it matters |
|---|---|---|
| an evaluator reads a target value it did not declare | the body reads a target the `inputs` list omits, the same textual scan `CtxScan::of` runs over `ctx` reads, reported pessimistically | an undeclared input is outside the memo key, so the answer is cached against inputs it did not depend on and served when they change |
| an evaluator is silent on a declared target | a body with a `write` or `fold` verb and no verb for some declared target, found by comparing the plan's targets with the body's emissions over the corpus | silence is a decline for the whole answer, so a body that meant "preserve" and wrote nothing loses every fact it did establish |
| a `write` names a non-target | the target index is outside the structural plan's validated targets | the write raises at query time, which is a decline; reported at load, it is a typo the author can fix |

Each is a report, not an enforcement: the rule is the runtime's, stated
once, and `spectcl_check` is where an author sees it before a user does.

### Inference

`ai/claude/skills/spec-author/SKILL.md` infers arity, roles, traits, hover,
and packages from a library's sources. For a private command implemented
in loop-free Tcl over whitelisted commands, the skill can propose the body
as a declared implementation, but purity inferred from a summary is
classification only: the route, its dependencies, and its budget are still
authored, and the pack's differential corpus proves the implementation
against the library's real behaviour.

## Where each part lands

Every part of this contract lands in one of slices 1–7 of the thirteen
[value-transfers-migration.md](value-transfers-migration.md) numbers;
slices 8–13 land the rungs the interface contract states and take no part
from this page.

| This page's part | Slice | What the slice already names |
|---|---|---|
| the four `EvalRoute` states, resolved with the binding and the selected form | 1 | the explicit abstention state at command, subcommand, and form scope |
| the target-semantics matrix as a contract, and the `Needs` axis list | 1 | the two contract pages themselves |
| the analysis context reaching the per-function key | 1 | "the analysis context in `FnLatticeKey`" |
| `ConstOps`, `ConstValue`, `Needs`, `admit`, `take`, the index pre-resolution, and the core table for `string range` and `incr` | 2 | "`string range` and `incr` over `ConstOps` and its admissibility adapters, preserving exact values and target semantics" |
| `var::append_bytes` and `var::lappend_value` on the same adapter | 2 | "then `append` / `lappend`" |
| the `WorkUnits` unit, the three nested budgets, and the allocation charge | 2 | the same slice, because it is the first route that charges |
| the expression route: the full value, the lazy services, `MathFuncSpec::result_class`, the binding evidence | 3 | "registry-owned argument assembly over the shared expression engine with lazy input services and transitive binding evidence; the full value result" |
| the declared-implementation route: `ActivationStore`, the two policies, `EvaluatorCapability`, `Engine::set_release`, the provisioning path | 4 | "per-evaluation state isolation in the host, `-native` resolution for every family, and `Engine::set_release`" |
| `EvalMemoKey`'s incoming-target and dependency components, and `EvaluatorGeneration` | 4 | "cache inputs (target values), overlay invalidation (`spec_pack_key` reaching `compilation_unit`)" |
| the `semantics` / `evaluate` / `facts` statements, the body verbs, the `SCOPE::FIELD` id rule, the per-family tables, the four surfaces, and the `spectcl_check` findings | 4 | "the loader, renderer, studio … delivered together on one small executable example before any catalogue migration" |
| `RegexpPrecision`, `PrecisionDecline`, the pattern cache, and the regexp cancellation point | 5 | "the regexp owner's typed precision result; `regexp`, `scan`, `lassign`, `binary scan`" |
| the `binary` and `scan` core rows, and the `-inline` / no-match / partial-`scan` outcomes | 5 | "program (2) folds and is typed as a byte array" |
| regexp-mode `switch::select` and `lsearch -regexp` selection facts | 6 | "selection facts for opaque forms through `tcl_cmd_core::switch`" |
| the remaining core-table rows as their catalogue entries appear, and the `PLATFORM` / `WALL_CLOCK` exclusions as a recorded reason | 7 | "every command that declares purity has a route or an explicit "none" with its reason in the inventory" |

Two obligations are not a slice of their own because they are a gate on
every slice: the `tclsh` differential gains each route's witnesses as that
route lands, and the four-surface parity tests fail closed from slice 4
onward.

## File-path anchors

- `rust/tcl-syntax/src/value.rs` — `ValueOps`, `ValueError`, `string_char_len`, the seam `ConstOps` implements
- `rust/tcl-cmd-core/src/string.rs`, `binary.rs`, `format.rs`, `string_is.rs`, `index.rs`, `switch.rs`, `regex.rs`, `list.rs`, `dict.rs`, `scan.rs`, `var.rs`, `lsearch.rs`, `mathop.rs` — the cores the direct route calls
- `rust/tcl-cmd-core/src/clock.rs`, `platform.rs`, `channel.rs`, `trace.rs`, `array.rs`, `namespace.rs`, `info.rs` — the cores the direct route never calls, and the `Needs` bits that keep it that way
- `rust/tcl-dialect/src/version.rs`, `grammar.rs`, `profile.rs` — `TclVersion::from_profile`, `string_character_model`, `number_syntax`, `byte_string_encoding`, `StringCharacterModel`, `ByteStringEncoding`, `NumberSyntax`, `const_fold_version`
- `rust/tcl-vm/src/value_ops.rs`, `runtime/rust/src/value_ops.rs` — the two runtime implementations of the seam
- `rust/tcl-registry/src/const_fold.rs`, `commands/tcl/string_.rs`, `commands/tcl/format_.rs`, `commands/tcl/regsub_.rs` — the shipped folders, the ASCII re-implementations, and the two that are already core calls
- `rust/tcl-compiler/src/codegen/helpers.rs`, `codegen/values.rs` — the two codegen folders to retire; `try_emit_constant_fold`
- `rust/tcl-compiler/src/tcl_expr_eval.rs`, `rust/tcl-syntax/src/expr/eval.rs`, `rust/tcl-syntax/src/expr/mathfunc.rs`, `rust/tcl-syntax/src/expr/rand.rs` — `FoldOps`, `eval_with_config`, `ExprOps`, `MathFuncSpec`, `MathFuncSince`, the generator model
- `rust/tcl-compiler/src/type_infer.rs` — `expr_call_type`, the duplicate math return-type table
- `rust/tcl-regex/src/lib.rs`, `exec.rs`, `cmd_core.rs` — `Regex::exec`, `MATCH_FUEL`, `MAX_BT_DEPTH`, `MAX_DISSECT_DEPTH`, `Matcher::spend_fuel`, `AreEngine`
- `rust/tcl-spec-hooks/src/host.rs`, `sandbox.rs`, `emit.rs`, `pack_eval.rs`, `program.rs` — `HookHost`, `HostConfig`, `SANDBOX_COMMANDS`, `builtins`, `answer_of`, `verbs_for`, `HookProgram`
- `rust/tcl-registry/src/pack_hooks.rs` — `HookFamily`, `HookInputs`, `CacheMode`, `ShapeKey`, `content_hash`, `install_host`, `clear_host`, `clear_cache`, `const_fold_fn`, `DialectScope`
- `rust/tcl-engine-api/src/lib.rs`, `rust/tcl-engine-tclvm/src/lib.rs` — `Engine`, `Budget`, `BudgetKind`, `EngineError`, and the one implementation
- `rust/tcl-vm/src/interp.rs` — `Interp::set_dialect_profile`, what `Engine::set_release` wraps
- `rust/tcl-spectcl/src/hooks.rs`, `loader.rs`, `export.rs`, `discovery.rs` — the pack seam, `hook_source`, `HookSource`, the native-ID tables, `Tier` and `Origin`
- `rust/tcl-spec-studio/src/coverage.rs`, `render_spectcl.rs`, `schema.rs`, `draft.rs`, `help.rs` — the four-surface gates, `Surface::Excluded`, `GAPS`
- `rust/tcl-mcp/src/spectcl.rs` — `spectcl_check`, `CtxScan`, `shape_cacheability`, `declaration_conflict`
- `rust/tcl-lsp-db/src/lib.rs` — `FnLatticeKey`, `compilation_unit`, `function_lattice`
- `rust/tcl-compiler/src/command_binding.rs` — `CommandTrustSnapshot`
- `rust/tcl-irule-test/tcl/_mock_stubs.tcl` — the generated simulator stubs, which are not evaluators
- `docs/references/command-spec/fields.md` — the generated field reference the three new statements extend
- `ai/claude/skills/spec-author/SKILL.md` — the inference surface

## Test anchors

- `rust/tcl-registry/tests/differential_fold.rs` — every fold against a real `tclsh`; gains the storage-outcome witnesses per release found on `PATH`
- `rust/tcl-spec-hooks/tests/const_fold_e2e.rs` — O129 driven by a `.tclspec` body
- `rust/tcl-spec-hooks/tests/containment_e2e.rs` — the budget, quarantine, and poison paths the route's isolation rule extends
- `rust/tcl-spectcl/tests/spec_corpus.rs` — every shipped pack's hooks through the sandboxed host at budget: a loading and containment gate, not a value oracle
- `rust/tcl-spectcl/src/loader.rs` — `native_hook_tables_cover_their_catalogues`, `value_tables_cover_their_catalogues`
- `rust/tcl-spec-studio/tests/spectcl_roundtrip.rs` — the four-surface round trip
- `rust/tcl-spec-studio/tests/reference_doc.rs` — the generated field reference
- `rust/tcl-vm/tests/dict_canonicalisation_parity.rs` — the list-rendering parity the folders depend on
- fixed witnesses to add: `direct_route_needs_match_their_cores` — every catalogued direct evaluator declines under an empty `Needs`
- the `fold [incr ::counter]` isolation test: the same answer on every call, and a raise from `ActivationStore`
- the three regexp precision witnesses, including the `^(a+)+\1$` / `^(a+)+b$` pair whose oracle answers 1 and 0
- `string repeat` and `**` bounded before allocation, with the two release messages `integer value too large to represent` and `string size overflow`
- the `<C3 89>` source-literal witness at every release and under both `encoding system` values
- the `string range … 010 end` decline under a profile that names no release, and its two answers under 8.6 and 9.0
- host-absent and quarantined workers answering identically, and an `Engine::set_release` witness per release axis

## Related docs

- [value-transfers.md](value-transfers.md) — the interface this contract evaluates for, and the ordered evaluation state stateful nested substitutions need
- [value-transfers-examples.md](value-transfers-examples.md) — the programs each route is measured against, and the declarations in Rust and `.tclspec`
- [value-transfers-migration.md](value-transfers-migration.md) — the slices that land each route
- [registry-consumer-contracts.md](registry-consumer-contracts.md) — runtime backing, the engine's WASM sibling, and C hosting, none of which this contract waits for
- [byte-array-corruption.md](byte-array-corruption.md) — why `ConstOps` must override both byte methods
- [../runtime/family-b-routing.md](../runtime/family-b-routing.md) — the shared-core rule the direct route follows
- [../contracts/numeric-tower-and-expr-semantics.md](../contracts/numeric-tower-and-expr-semantics.md) — the numeral-grammar and tower owners the `Needs` bits name
- [../contracts/differential-fuzzing.md](../contracts/differential-fuzzing.md), [../contracts/registry-contract-tests.md](../contracts/registry-contract-tests.md) — the oracles
- [../contracts/vm-compiled-artifact-provenance.md](../contracts/vm-compiled-artifact-provenance.md) — how a folded value in bytecode is admitted and invalidated
- [../registry/spec-packs.md](../registry/spec-packs.md), [../spec-dsl-examples/README.md](../spec-dsl-examples/README.md) — the DSL, its hook contract, and the vocabulary changelog 2.2 extends
- [../contracts/command-spec-studio.md](../contracts/command-spec-studio.md) — the four-surface parity rule
- [../contracts/shared-utility-contracts-rust.md](../contracts/shared-utility-contracts-rust.md) — the owner manifest
- [compiler design index](README.md), [design docs index](../README.md)
