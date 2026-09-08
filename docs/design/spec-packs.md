# SpecTcl — spec packs as a Tcl DSL

The DSL is named **SpecTcl**. (Two prior arts share the name — Sun's 1990s
Tk GUI builder and the NSCL physics analysis tool; both are distant enough
that the pun wins.)

**What ships.** The `.tclspec` loader, three discovery tiers with
nearest-wins precedence, multi-file pack merge, a compiled-pack cache in the
OS cache directory, workspace-scope insertion into the per-profile cached
`CommandRegistry` under the shipped-wins-unless-`-override` collision
policy, hook bodies running on `tcl-vm`, and LSP integration — packs load at
workspace init, reload on pack-file change, and their load notices are
published as diagnostics on the pack file. All of it lives in
`rust/tcl-spectcl`. The eight EDA loadables in `specs/` (`sdc_base`, `upf`,
and the six vendor packs) are bundled `.tclspec` packs read by the same
loader; `tcl_spectcl::bundled` is what puts them into a profile registry.
The Spec Studio is the DSL's IDE
([`contracts/command-spec-studio.md`](contracts/command-spec-studio.md));
the frozen syntax is
[`spec-dsl-examples/README.md`](spec-dsl-examples/README.md).

**Known gap.** The studio has no skill-in-studio tab: the page reaches
nothing but GitHub's two hosts, for the release fetcher.

## The problem

Users with private Tcl libraries cannot contribute their command specs to
the shipped registry, and stubs deliberately carry only a fraction of what
a `CommandSpec` can say. They need a way to author a full command database
for their own code and load it into the server — without a Rust toolchain,
and without rebuilding it for every tcl-lsp release.

## One authoring format

SpecTcl is the authoring format for every command surface that is not a
core: the EDA vendor libraries ship as bundled loadables, Jim's command
roster is a compiled-in pack (`rust/tcl-spectcl/core-surfaces/jim.tclspec`),
and private packs load the same DSL. The shipped cores — `commands/{tcl,
irules}` and the stdlib, tcllib, Tk, iApps and Expect surfaces — stay
native Rust; there is no ahead-of-time `.tclspec` → `.rs` path.

The equivalence gate falls out of that split: re-express a shipped surface
in the DSL, load it, and assert field-for-field equality with the compiled
spec. The design was driven the same way — by porting hard shipped specs
(`lsort`, `switch`, TclOO/snit definers, `upvar`, `return`) and by
drafting specs for external libraries (ticklecharts, apave, SpiceGenTcl,
tcllib modules) rather than by inventing syntax in the abstract.

**Where the migration half stops, exactly.** `refine NAME { … }` is the
**invocation refinement** — arity, a literal `selector`, argument roles,
options and relations, availability, and the replacement `traits` /
`mutator` / effects one call shape states — written in the owning scope's
own words and read by the owning scope's own readers. The native halves
(`completion`, `dispatch_dependencies`, `literal_argument_validator`, and
the compiler hook ids) stay Rust-only, and a form carrying one is
*reported*, not thinned, so the round trip never claims more than it
preserves. Tk's widget methods are the largest structured-form user:
`tk_form_refinements_round_trip_through_the_pack_dsl` renders every Tk
command that refines its subcommand forms to a pack and asserts the
reloaded form tables equal the compiled ones. Plain `forms` remains
documentation-only: `FormSpec` carries synopsis and lifecycle, never traits
or effects.

## Performance: the format does not decide it

A pack is evaluated **once at load**, resolved against the running
registry's vocabularies, and inserted into the same per-profile cached
`CommandRegistry` the built-in packs live in (interned once, keyed by
content hash). After that, every query — hover, role lookup, arity, taint —
takes exactly the path a compiled-in spec takes. Runtime cost is
**identical to compiled-in**; the only cost the format controls is a few
milliseconds of load at startup or on pack change, plus one resident copy
of the data.

The one design rule that matters for performance: packs layer into the
cached registry at workspace scope, **not** the per-document overlay path
stubs use — a pack is parsed per edit of the pack, never per edit of the
code that uses it.

### Measured: what the 2.0 vocabulary costs

`cargo run --release -p tcl-spectcl --example speclib_version_costs`
loads every bundled pack three ways over the same source — as it ships, as
`tcl spec upgrade` rewrites it to the newest vocabulary, and with the
static fast path off, so the pack really is executed as a Tcl program by
`tcl-vm` rather than captured from its parse tree. All three register the
same commands; the example asserts that before it times anything. Median
of 15 loads, release build:

| pack | lines | commands | 1.x ms | 2.0 ms | Δ | 2.0 VM ms | 1.x KiB | 2.0 KiB | 2.0 VM KiB |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| `eda_cadence` | 1076 | 77 | 3 | 4 | +13% | 21 | 129 | 144 | 129 |
| `eda_mentor` | 1262 | 69 | 5 | 6 | +9% | 39 | 126 | 201 | 197 |
| `eda_microchip` | 6366 | 257 | 36 | 37 | +3% | 398 | 809 | 852 | 1008 |
| `eda_quartus` | 1110 | 77 | 5 | 5 | +7% | 32 | 0 | 0 | 0 |
| `eda_synopsys` | 923 | 68 | 3 | 4 | +9% | 22 | 0 | 0 | 0 |
| `eda_xilinx` | 20887 | 788 | 145 | 174 | +20% | 2725 | 3164 | 2840 | 3162 |
| `sdc_base` | 1186 | 86 | 7 | 6 | -6% | 41 | 0 | 0 | 0 |
| `upf` | 2191 | 67 | 22 | 19 | -14% | 123 | 0 | 0 | 0 |
| **corpus** | | | **227** | **255** | **+12%** | **3401** | **4230** | **4038** | **4498** |

- **2.0 costs about a tenth of the load, and nothing else.** The
  `available {tcl 8.4-}` algebra is more words to segment than
  `dialects all-tcl`; on the 35k-line corpus that is ~30 ms once, at
  workspace scope.
- **Memory is unchanged**, because the two spellings lower to the same
  rows. The column measures resident bytes a load *retains* — the loader
  interns what it registers — at page granularity, so the small packs read
  `0`.
- **The static fast path is what makes evaluation affordable.** Executing
  the corpus as Tcl costs 3.4 s against 0.26 s: a wholly declarative pack
  is captured from its parse tree instead, and
  `rust/tcl-spectcl/tests/eval_loader.rs` loads every shipped pack both
  ways and asserts the snapshots are identical. A pack that templates its
  registrations pays the interpreter for the part that needs one.

## The format: a Tcl program, evaluated by our own toolchain

The canonical extension is **`.tclspec`** — the editor extensions and the
LSP register it as Tcl in the SpecTcl dialect, so a pack file gets the full
editor experience with no configuration. A compiled-in command pack for
SpecTcl itself (`rust/tcl-registry/src/commands/spectcl/`) — `speclib`,
`command`, `option`, `arg`, `subcommand`, the hook statements — with its
`definition_body` grammar gives authoring highlighting, completion, and
misspelled-trait diagnostics from the same machinery it configures.

```tcl
speclib mylib 2.1 {
    command with_var {
        synopsis {with_var varName script ?mode?}
        arity 2 3
        arg 0 -role varwrite
        arg 1 -role body
        traits {EVALUATES_CODE CREATES_SCOPE_ALIAS}
        hover -summary {Run a script with a caller variable bound.} \
              -returns {The script's result.}
    }
    command mylib::sort {
        arity 1 -1
        option -command -takes commandprefix -appends 2
        option --
        subcommand indices {arity 1 1  returns List  pure}
    }
}
```

Word spellings come from the registry's existing catalogues — traits,
roles, hook names — exactly as the Spec Studio and the reference manual
spell them, so the **?**-button help and the Reference tab document the
DSL for free.

Why this beats JSON for the author: the toolchain eats its own dogfood
(`speclib`/`command` are registry specs with a `definition_body` grammar,
the same machinery that gives snit and TclOO bodies highlighting, folding,
completion, and go-to-definition); it is the stub language grown up — same
file culture (`*.tclspec` beside the code), a superset of what stubs
express; and Tcl quoting and line-continuation are what our users already
know.

**A pack is a Tcl program, evaluated in a sandbox** (`tcl_spectcl::loader::evaluate_pack`,
the one loader every consumer uses). The registration calls it makes
produce a frozen snapshot; a straight-line pack — nearly every pack — is
captured from its parse tree without entering the interpreter, and a pack
that templates its declarations (`foreach`, a shared variable) pays the
interpreter for exactly that. See *Authoring rules for SpecTcl 2.0*.

## Covering the hooks

"Entirely cover hooks" is the hard requirement. Hooks are the tail — a few
dozen argument-role resolvers and const-folders, a handful of
command-prefix resolvers, single-digit counts of every other family — but
they gate the hardest commands, so the DSL takes them in four buckets:

1. **Declarative data, first.** The named-descriptor fields
   (`definition_body` grammars, `object_class`, `case_list`,
   `body_scope`, handle bindings, repeated-arg layouts, deprecation
   fixes) are plain structs, so they get first-class DSL forms, not code.
   The declarative `clause_grammar` absorbs most would-be
   `clause_shape_check` uses (`if`'s chain is a grammar, not an
   algorithm), and the four option-relation rows (`option_conflict`,
   `option_requires`, `option_requires_one_of`, `option_forbids`) are
   checked natively — `OptionRelation::evaluate` is a few slice scans, no
   VM entry — before a `constraints` hook is ever consulted.
2. **Named native hooks.** Lowering / codegen / analyser hook IDs, shared
   definer grammars, and `frame_effect` descriptors are closed sets
   referenced by name. A pack reuses them; it cannot add to them.
3. **Tcl hook bodies on the VM.** Resolvers, folders, and the predicate
   gates are small **pure functions from words to data**. Each hook kind
   has a fixed calling convention (inputs: the call's words plus a context
   dict; output: a declared result protocol; error or no result =
   abstain), compiled to bytecode once at pack load, fuel-limited, no
   ambient authority. This is the one executable surface a pack has at
   query time, and it is deprivileged, deterministic, and bounded.
4. **Stays native.** Anything that cannot be a pure words→data function is
   a contribution.

**Execution engine: `tcl-vm` is the canonical answer everywhere** —
server, CLI, and studio alike (it ships to WASM). Performance investment
lands in the VM — hooks give it a hot, measurable, real workload.

**The framework is a Rust crate linking the VM the way a C extension
links C Tcl.** The DSL host (loader, hook harness, sandbox) registers its
surface natively: the emitter verbs (`role`, `fold`, `reject`, …) and the
sandbox builtins (e.g. the conservative `foldlist`) are Rust commands in
the hook interpreter, and hook inputs/outputs cross the boundary as
structured VM values — never string round-trips. One crate backs every
deployment (native server, CLI, wasm studio).

**Two layers, deliberately.** The bottom layer is a **Tcl extension
interface** in idiomatic Rust — traits and owned structured values, no raw
interp pointers — playing the role Tcl's C API plays for C Tcl. `tcl-vm`
and the Tcl→WASM codegen runtime implement it (selecting one must not
require the other), and it is designed against exactly **two consumers**:
the hook framework, written in Rust on top of it; and the C-Tcl shim
(`tcl-cshim`, [c-extension-shim.md](c-extension-shim.md)), which lets
users compile existing C Tcl extensions to run on our tclvm or in wasm
through the same surface. The interface is **stable and common to the
backends** — tclvm and the wasm codegen runtime, explicitly *not* the BPF
backend — and **all C-required mangling lives in the shim, never in the
interface**. Shimmed extensions are trusted native code loaded only by
host configuration, and nothing in the SpecTcl vocabulary can reference
one. The **hook host** is the layer above: it owns everything DSL-specific
— the emitter verbs, per-family calling conventions and preconditions,
abstention and error policy, fuel budgets, memoisation — and speaks only
the engine interface beneath it, so a new engine (or a shimmed C extension)
slots in under the unchanged host.

**Crash containment is a load-bearing guarantee.** A spec must never be
able to take the LSP down: every hook invocation crosses a containment
boundary — `catch_unwind` plus the fuel and memory budgets in-process (the
WASM engine is additionally sandboxed by construction) — and a panic,
budget blowout, or stack overflow in a hook is converted to abstention,
never propagation. Isolation is **per pack**: each loaded pack gets its
own sandboxed VM with no shared interpreter state, so one library's bad
hook costs that library its hooks, nothing else. On the first crash the
offending hook is **quarantined** for the session (the command falls back
to its declarative facts), and a structured crash record is written: pack
name and content hash, command and hook family, the SpecTcl vocabulary and
server versions, the input word *shapes*, and the panic payload/backtrace.
The user gets an editor toast — "a spec pack hook crashed; tcl-lsp is
unaffected" — with an **Open a GitHub issue** action that pre-fills an
issue from the crash record, shown to the user before posting; nothing
leaves the machine without a click.

**Hot-path budget — measured** (release build, cross-checked against the
`tclvm` CLI): a VM resolver body costs **28 µs** per invocation against a
whole-call-site native budget of **410 ns** — the VM's floor is ~487 ns
per Tcl command, so no marshalling trick closes a 68× gap. A const-folder
is 16.6 µs; pack load by the static fast path is **4.28 ms** for a
2,000-line pack. The design consequences are binding:

- **Granularity is not restricted — consequences are documented and
  shown live.** A resolver that depends only on its declared inputs and
  returns the complete index→role map in one invocation is
  **shape-cacheable**: the registry caches it by command + word-shape at
  24.5 ns/call — indistinguishable from native. A hook that declares
  broader dependencies stays fully legal but uncacheable, at ~28 µs per
  call site, ~48 ms of semantic-token time per 2,000-line file. That
  consequence is documented here, in the syntax memo's hook chapter, and
  **live in the Spec Studio** — a performance badge on the hook editor and
  measured timings in the Test tab. A plain word-vector memo is not a
  rescue (2.94 µs at a 90% hit rate — fresh variable names are all
  misses).
- **Folders and predicate gates run on the VM freely** — bounded, off the
  interactive path, contractually pure.
- **The VM's embedder APIs exist for this**: a public
  `Vm::invoke_command`, `Vm::invoke_function` over pre-compiled handles
  with no per-call `FunctionAsm` clone, and an enforced command-limit
  budget at both dispatch funnels. The registry's shape-keyed caching
  clears budget regardless of VM speed.

## What exists today

| crate | layer | what it is |
|---|---|---|
| `tcl-engine-api` | bottom | The **Tcl extension interface**: `CompileUnit` → engine handle, invoked with owned structured `Value`s (list and dict are first-class, so `words`/`ctx` never round-trip through text); `HostCommand` for embedder-registered commands; `Budget` the engine must enforce; `EngineError` distinguishing a script error, a budget blowout, and a crash. No dependencies at all. |
| `tcl-engine-tclvm` | bottom | The `tcl-vm` implementation: `Vm::define_procedure` (compile once), `Vm::invoke_command`, `Vm::register_native_command` (stateful host commands), `Vm::retain_commands` (a closed whitelist), and the enforced `commands` limit + wall-clock cap. |
| `tcl-spec-hooks` | top | The **hook host**: emitter verbs as native commands, the per-family calling conventions and the literal-only precondition, abstention and error policy, per-pack engines, `catch_unwind`, quarantine-on-first-crash with a structured crash record, and the sandbox whitelist plus `foldlist`. Also the pack evaluator (`pack_eval`) that runs a whole pack file under the same sandbox. |
| `tcl-cshim` | consumer 2 | The **C-Tcl shim** ([c-extension-shim.md](c-extension-shim.md)): a C extension compiled against `include/tclshim.h` registers its commands through `Engine::define_command`, with `Tcl_Obj` crossing as typed values. |
| `tcl-registry::pack_hooks` | seam | Slots, per-family thunk tables, the thread-local host, and the **shape-keyed cache**. A pack hook is a plain function pointer of the family's shipped type, so `run_const_fold` and every other consumer is unchanged and unaware. |

- **The two budgets bound different things.** The command limit counts
  *dispatched* commands — as C Tcl's does — so a loop the compiler inlines
  into bytecode is not charged; the wall-clock cap, polled inside the
  bytecode trampoline, is what stops that case. Containment needs both,
  and the host's default config sets both.
- **The cacheability rule has a spelling**: `-inputs {nwords kinds}` on a
  hook statement. A hook whose declared inputs exclude the words' content
  is answered from the registry's shape-keyed cache; a hook that declares
  nothing is fully legal, always correct, and uncacheable.

### The corpus-validation harness

`rust/tcl-spectcl/tests/spec_corpus.rs` is the gate over **every
`.tclspec` the repository ships** — the eight bundled loadables under
`specs/`, the eleven ports and the five external drafts under
[`spec-dsl-examples/`](spec-dsl-examples/). Per pack it loads through the
real loader, installs into a real per-profile registry, runs the analyser
and the optimiser over the corpus files in `samples/` that call the pack's
commands (synthesising an exercising call from a command's own arity and
argument roles when nothing in the corpus reaches it), drives the hook
bodies through the sandboxed host at its shipped budgets, and reports
commands loaded, notices, hooks invoked, quarantines, and load + analysis
wall clock. Accepted load notices live in
`rust/tcl-spectcl/tests/spec_corpus_baseline.txt`, compared as a multiset
in both directions so a fixed notice must also be deleted from the
baseline. The `state_transitions` / `world_effects` rows the loader does
not yet read are the bulk of that baseline.

Its negative half is `rust/tcl-spectcl/tests/fixtures/hostile.tclspec`: an
unbounded loop, a dispatch-heavy fold, and a body that panics. All three
end as abstention with a crash record and a quarantined hook, on a
watchdogged thread.

Beside it: `tests/golden_packs.rs` holds all 24 shipped packs to
checked-in snapshots (regenerated by `cargo xtask pack-goldens`), so a
loader change cannot silently alter what a pack means; `tests/eval_loader.rs`
holds the static fast path and the interpreter route byte-identical; and
`tests/pack_is_real_tcl.rs` sources every shipped pack with a stock
`tclsh9.0` so a pack is always parseable Tcl.

## Compatibility policy

The "ABI" is the DSL vocabulary, and it follows the tolerance rules that
let packs survive releases without rebuilds:

- Unknown property words, trait names, role names, or hook names are
  dropped with a logged notice; the rest of the spec loads. New server +
  old pack always works; old server + new pack degrades gracefully.
- A trait name the registry has since retired is dropped the same way,
  but the notice names what carries the fact now: a pack saying
  `traits HAS_SWITCH_BODY` is told that a `case_list` block carries it and
  that `CommandSpec::has_switch_body` is the derived answer. The table is
  `RETIRED_TRAITS` in `rust/tcl-registry/src/traits.rs`; the rule for
  retiring a trait is in
  [command-registry.md](compiler/command-registry.md#retiring-a-trait).
- **Except where dropping the word would strengthen the answer.** An
  unknown word in a pack declaring a vocabulary this build postdates is
  classified by its compatibility effect (`VocabularyClass`):
  *presentation* words warn and drop as above; *assistance* words (shapes,
  roles, value sets) leave the command known but degraded, so the
  affected capability answers `Unknown`; *semantic* words (security,
  control flow, binding, lowering, codegen — and every unknown word inside
  a `dialect` or `environment` block) exclude the command or block from
  strong analysis. An old server that ignored a "this method is a sink"
  word would otherwise report a *cleaner* result than a new one.
- A `speclib` version pragma gates hard breaks only — a word whose
  *meaning* changed, not one that was added. The server refuses a major it
  does not know and names the fix: the pack loads nothing at all, and one
  notice says why. An unknown *minor* within a known major keeps loading
  maximally.
- `VOCABULARY_VERSION` (`rust/tcl-spectcl/src/lib.rs`, part of the
  compiled-cache key) bumps only when a word's meaning changes — once, for
  2.0, because the legacy `dialects` word's translation output changed.
  The vocabulary ladder and every word each revision added are in
  [`spec-dsl-examples/README.md`](spec-dsl-examples/README.md); this
  document does not duplicate the spelling tables.
- The studio schema-coverage gates force every new `CommandSpec` field to
  a named key; that key is the DSL property name, so the format cannot
  silently fall behind the registry.

## Version ranges: introduced, deprecated, retired

Every entity the registry can gate carries the same `Lifecycle` triple —
an introducing release, a deprecating release, and a retiring release, on
either the owning package's version axis or the core Tcl axis — at
**every** gateable level:

- the command itself (`CommandSpec::lifecycle`),
- a subcommand, and a second-level operation of a two-level ensemble
  (`SubSubCommand`, e.g. `info object class`),
- an option, at command or subcommand scope,
- an option relation (`OptionRelation::lifecycle`),
- a side effect,
- an invocation form (`FormSpec`), and
- a literal argument value in a closed set (`ArgValue::lifecycle`), which
  rides the owning-package axis and is a separate fact from
  `ArgValue::min_tcl` (the Tcl-core-gated argument-DSL rung, `W137`).

`CommandSpec::versioned_arg_values` is the command-level mirror of the
per-subcommand literal-value gate, for a value that sits directly on the
command rather than behind a subcommand word (`HTTP::respond <status>
noserver`, `close $chan read`). A value gated by both is narrowed by
`Lifecycle::intersect` and recorded once, so a doubly-gated value never
draws two diagnostics for one word.

The command, subcommand and sub-subcommand, option, and argument-value
levels feed `W135`/`W136`/`W139`/`W144` via `version_gate.rs`. Option
relation, side effect, and form lifecycles are registry data validated by
the registry sweep but not yet read by a diagnostic: a pack or a shipped
spec can declare them, and the field round-trips through the studio, but
nothing in the editor reports a use of a deprecated or retired *form* or
*side effect*.

### Ordering and containment: notice-only for packs, a hard gate for shipped specs

A `Lifecycle` must be internally ordered (introduced ≤ deprecated <
retired, where each is present) and a child's declared releases must fall
inside its parent's window. Both properties are checked at two strengths:

- **Shipped specs — a hard gate.** `rust/tcl-registry/tests/registry_sweep.rs`
  walks every compiled-in command, recursing into every level, and asserts
  both properties (`Lifecycle::validate`, `Lifecycle::intersect`). A
  shipped spec with an invalid or non-contained lifecycle fails the suite.
- **Packs — notice-only.** The loader runs the same two checks on every
  `-introduced`/`-deprecated`/`-retired` lifecycle it parses. An invalid
  ordering drops the lifecycle back to `Lifecycle::UNSPECIFIED` with a
  notice naming the field and the reason; a lifecycle that reaches outside
  its parent's window is reported the same way but **kept**. A pack is
  authored elsewhere and must still load.

### The requirement-range straddle rule

`package require Foo A-B` guarantees only that the loaded `Foo` is
somewhere in `[A, B)` — the version-gate floor check asks about `A`
alone, so a range that runs past a retirement can pass the floor while
still failing for part of the accepted window. `version::requirement_upper_bound`
extracts that ceiling from a **stated** range only: `A-B` participates,
but a bare `A` or an open `A-` states no ceiling and is exempt. `A-A` is a
degenerate pin the floor check alone already decides.

When the ceiling reaches strictly past a retirement,
`Analyser::requirement_straddle_diagnostic` emits the same `W139` the
plain retirement check would, but hedged:

```text
'trace variable' is not available in every version satisfying requirement
`8.5-9.1`: removed in Tcl 9.0.
```

The straddle diagnostic only fires while the floor's own verdict is
`Available`; a floor that already fails keeps its own, more specific
message. See [W139](../kcs/codes/kcs-diagnostic-w139-retired-at-resolved-version.md).

### Versioned signatures and ambient packages

- **Arity windows.** An `arity` row with the three lifecycle flags is a
  **window**: the shape the command had over one span of its owning
  package's releases. Several may be declared, and the plain row stays the
  fallback for a resolved floor no window covers. Windows must **not
  overlap** — consecutive windows are written *closed*, the first retiring
  where the second begins. A pack that overlaps keeps the first window and
  gets a notice; a shipped spec is rejected by `registry_sweep`. The
  version importer (`tcl-spec-studio`'s `import_package_versions`) derives
  windows from several release snapshots, keeping the note that produced
  each as evidence.

  The window covering the document's resolved floor is the shape a call is
  checked against. A call whose count fails the selected shape but fits
  *another* declared window draws
  [W149](../kcs/codes/kcs-diagnostic-w149-arity-matches-other-version.md)
  rather than a bare "too many arguments"; a count fitting no window stays
  an ordinary E002/E003.

- **Gated `arg` rows.** The loader reads each `arg` row as one record and
  *projects* it into the six parallel per-argument tables the registry
  stores, so a row outside the floor drops out of all six at once. The
  stored tables are the projection at **no floor**; the authored rows are
  retained beside them so a consumer holding a resolved floor re-projects
  at it.

- **`ambient_package NAME VERSION`.** A package the pack's dialect
  provides without a `package require` — the pack-authored twin of an
  ambient `LibraryPin`. It composes with the other two sources of a floor
  by taking the greatest, exactly as two `package require` lines do; when
  two are equal the diagnostic names the one closest to the author (the
  require in this file, then the pack in this workspace, then the profile
  compiled into the server). The row is **unscoped by construction**: it
  floors its package for every document the pack is active in. To scope a
  placement to one environment, write it inside the environment — see
  *Declaring an environment* below. `ambient_package NAME VERSION
  -dialects {…}` is not vocabulary: the row is dropped whole with a notice
  naming the environment spelling, because an availability-**narrowing**
  word a reader cannot honour must not leave the wider claim standing.

- **Callback timing and taint.** `option … -script-timing
  SameInvocation|Deferred|ReferenceOnly` separates temporal control flow
  from `-body-kind`; `SameInvocation` is the default and remains a
  lowering barrier. `option … -callback-taint-inputs {%P …}` and the
  positional `callback_taint_inputs {{INDEX {%A …}} …}` declare the finite
  set of externally controlled substitutions a *deferred* callback host
  injects (the authorable Tk values are `%P`, `%s`, `%S`, `%A`, `%K`;
  framework metadata such as `%W`, `%d`, `%i`, `%V` is rejected). Static
  analysis replays only a literal script or a literal `[list command …]`
  prefix, each in an independent synthetic callback frame; an explicit
  shared global (`set ::state …`) is the deliberate remaining limit,
  because real events can race. `script_timing_resolver {words ctx} { … }`
  handles positions whose timing depends on the written form
  (`send` versus `send -async`).

- **Variables and geometry.** `option … -taints-var-write` marks a
  variable-valued option whose linked variable can be written from
  external input; `option … -variable-scope CurrentFrame|Global` declares
  where an unqualified variable-name option resolves (Tk `-textvariable`
  and `-variable` are `Global`); `tk_geometry POLICY …` declares a Tk
  geometry manager's container policy and placement/release forms, which
  static preview and TK1001 consume without naming `pack`, `grid`, or
  `place`.

## Loading and tooling

- **A pack is a logical unit, not a file.** Authors group however they
  like — one big `.tclspec`, one per namespace, one per command — and
  every file whose `speclib` names the same pack merges into one pack
  model at load. Merging is deterministic (files in sorted path order); a
  command defined twice within one pack is a load-time diagnostic with the
  first definition winning. The compiled cache keys per file and per
  merged pack, so touching one file recompiles one file.
- **Discovery, three tiers, nearest wins** (workspace > user > bundled):
  **workspace** — `tclLsp.specPacks` paths (mirrored as a setting in every
  editor integration), plus `*.tclspec` beside a `tclpkg.tcl` manifest or
  under `.tcl-lsp/`; **user** — packs in the platform config directory
  (`$XDG_CONFIG_HOME/tcl-lsp/specs/` and the macOS/Windows equivalents via
  `tcl_userdirs`), loaded for every workspace; **bundled** — the shipped
  EDA loadables in the `specs/` directory beside the running executable
  (`TCL_LSP_SPEC_PACK_DIR` puts a different directory in front of it).
  Name collisions with shipped specs are reported; shipped wins unless the
  pack says `-override`. A browser worker has no filesystem, so when the
  bundled directory yields nothing discovery walks the server's
  closed-file store at `discovery::VIRTUAL_PACK_MOUNT`, where the host
  upserts its own packs additively, keyed by file name; the shipped
  loadables are compiled into the server as the fallback for any name the
  host did not mount. See
  [contracts/lsp-source-store.md](contracts/lsp-source-store.md), "The
  virtual spec-pack mount".
- **Compiled-pack cache in the OS cache directory**
  (`$XDG_CACHE_HOME/tcl-lsp/spectcl/` and platform equivalents): a pack's
  evaluated snapshot is written keyed by `EvalSnapshotKey` — a
  non-cryptographic hash of the pack source **plus the SpecTcl vocabulary
  version, the loader-eval version and the tier** — so an edited pack or
  an upgraded server recompiles exactly once. The cache is disposable by
  contract: delete it and nothing breaks but first-load time; a corrupt or
  stale entry falls back to a fresh evaluation, never an error.
- **CLI and MCP.** `tcl spec import` derives version ranges for a
  package's commands from several releases; `tcl spec upgrade` rewrites a
  1.x pack to the newest vocabulary (`--check`, `--verify`, `--restyle`;
  [dialect-and-package-registry-centralisation.md](dialect-and-package-registry-centralisation.md)
  §6); `tcl spec export` renders a pack as canonical SpecTcl — its
  expansion, if it is a program. The MCP server carries `spectcl_check`
  (evaluate a pack and report notices, `load_error`, target-dependence,
  and what the workspace tier would refuse), `spectcl_expand` (`spec
  export` over MCP), and `spec_import`. There is no CLI `spec check`
  verb. The `spec-author` skill emits the DSL for the private-library
  path.

### Editor registration of pack-claimed file extensions

A pack's `file_extension NAME -dialect D` row (and the `file_extensions` of
a pack-declared `environment` block) routes the extension server-side the
moment the pack is discovered. The editor learns its extension-to-language
mapping from a static manifest, so it needs telling.

The server *advertises* the pairs. `pack_file_extensions` appears on the
`tcl-lsp.getEffectiveConfig` result and again in the
`tcl-lsp/specPacksReloaded` notification, which is sent once a reload has
fully landed. Each row carries the extension, the claiming pack, the
dialect, and the **existing** editor language id the extension should
ride, because no editor can mint a new language id at runtime.

Registration is a per-editor problem, and reversibility is the hard half:
reconciliation has to delete as well as add, so "did we write this" must
be answerable exactly.

#### VS Code

The advertised set is projected into **workspace-scoped**
`files.associations`, and the entries the extension owns are remembered in
workspace state as `{glob: languageId}` — the value written, not just the
key. An entry is ours only while the configuration still says what we last
wrote there, so a user who retargets `*.foo` by hand takes ownership
permanently. Globs are case-folded per character (`*.[fF][oO][oO]`),
matching the server's case-insensitive routing. Already-open documents are
flipped onto the new language with `setTextDocumentLanguage`, but only for
the associations reconciliation actually owns.

#### JetBrains

`FileTypeManager` associations are **IDE-global**, so the JetBrains half
keeps its own ledger.

- **What is registered.** Each advertised row maps onto a file type the
  plugin already contributes: `language_id` `tcl-irule` selects the
  **iRule** type, every other id (including plain `tcl`) selects **Tcl**.
  Association happens through `FileTypeManager.associate` with an
  `ExtensionFileNameMatcher`, on the event dispatch thread inside a write
  action.
- **The ownership ledger.** The application-level `TclLspPackAssociations`
  state persists `{extension: fileTypeName}` for the associations *the
  plugin itself installed*. An association is retired only when the
  extension is no longer claimed **and** the IDE still reports exactly the
  file type the ledger records. Anything else is dropped from the ledger
  and left alone.
- **Manual associations win.** Before claiming an extension the plugin
  asks `FileTypeManager.getFileTypeByExtension`; if anything already owns
  it, the claim is skipped and never recorded.
- **Restart survival.** The ledger is persisted, and IDE file-type
  associations persist on their own. The first report of the next session
  retires an association whose pack has gone.
- **Multi-project.** Associations are global but claims are per project.
  The service registers the **union** of every open project's claims; an
  association is retired only when no open project still claims it. If
  two projects map the same extension to different file types the plain
  **Tcl** type wins. A closing project drops its claims, and what it alone
  claimed is retired there and then — but only while another project is
  still open, because every project closes in turn when the IDE exits.
- **Attachment follows the claim.** A dynamically-claimed extension is
  added to the set the plugin starts the language server for.

### Workspace trust: the setting is gated, the workspace tier is not

`tclLsp.specPacks` is in the extension's `restrictedConfigurations`
(`editors/vscode/package.json`), so an untrusted workspace's value for it
is ignored — only the user's own value applies. The setting names
arbitrary paths, and a `.vscode/settings.json` is workspace-authored
content.

The rest of the workspace tier — a `.tclspec` under `.tcl-lsp/`, or beside
a `tclpkg.tcl` manifest — is **not** gated, and loads in an untrusted
workspace with no setting at all. It cannot name anything outside the
folder the user opened, so it is the same class of content as the `.tcl`
files the analyser already reads.

What makes that safe is the sandbox, not trust: a pack's executable
surface is its evaluation and its hook bodies, both pure words-to-data on
a closed command whitelist (no `open`, `exec`, `source`, or `socket`),
each pack gets its own engine, and every invocation runs under a command
count and wall-clock budget with `catch_unwind` and
quarantine-on-first-crash around it. A workspace pack can make the editor
say something wrong about the workspace's own code; it cannot reach the
machine. Two further floors hold regardless of tier: an override can never
weaken a shipped spec's security facts (`tcl-registry::security_floor`
unions set-valued facts and keeps single-valued ones — the I6 invariant),
and the workspace and Spec Studio tiers cannot `-override` a compiled
command name, declare a `dialect` block, or claim a reserved `environment`
name (refused at registration, with the provenance named). If a hook family
ever gains ambient authority, the workspace tier has to become trust-gated
in the same breath.

## Authoring rules for SpecTcl 2.0 (design E)

A pack is **evaluated**, not walked: the file runs as a Tcl program in a
deterministic sandbox, and what loads is the snapshot of registrations it
made. The execution model:

- **Evaluation produces a frozen snapshot — nothing else does.** A pack is
  evaluated once per `EvalSnapshotKey`, and everything downstream —
  assembly, hover, completion, the compiler, `tcl spec export` — reads the
  snapshot, never the source.
- **Determinism.** No clock, no IO, no network, no processes, no
  environment, no threads, no `rand`; budgets bound command steps and
  value size, and wall clock on targets that have a real one (the browser
  evaluates under the step budget alone). Registration is transactional:
  any hard error loads nothing at all, and a runaway `foreach` is a budget
  notice naming its axis.
- **Conditionality is data, not control flow.** Every registration takes
  `-available`; the row is always registered, carrying its gate, and
  assembly resolves gates per environment. `available?` marks the pack
  target-dependent and uncacheable.
- **The sandbox always runs; provenance gates what a registration may
  touch** (the tier rules above), enforced at the registration call.

Two consequences shape everything an author does — a pack can *template*
its declarations, and a reader of the file no longer necessarily sees the
surface it produces. The rules below keep the first without paying for the
second. (Executable registration was chosen over synopsis-first,
proc-mirror, namespace-native, pure-dict and annotated-stub surfaces for
one reason: templating is the only thing that shrinks a 788-command vendor
pack, and the execution model above is what answers its costs — static
opacity, trust, and cacheability.)

- **Write canonical form unless repetition is the problem being solved.**
  The **canonical subset** is straight-line registration calls only — the
  declarative vocabulary, no `proc`, `set`, `foreach`, or computed
  argument. Every shipped pack is canonical, canonical source and snapshot
  are a bijection modulo formatting, and everything that *generates* a
  pack emits it: the studio's renderer, `tcl spec import` and its MCP
  twin, `spec upgrade --restyle`, stub-tier conversions. Programs are for
  humans with a repetition problem — forty commands differing in two
  fields — not for saving four lines.
- **Run `spec export` and read the expansion before shipping.**
  `tcl spec export` (MCP: `spectcl_expand`) renders any snapshot back as
  canonical source. Expansion is **total**; contraction — recovering a
  program from its snapshot — is never attempted, so the expansion is the
  whole truth about what a pack registered.
- **Prefer an `-available` row to a branch on `available?`.** Keep
  `available?` for the rare case where the *shape* of a declaration
  differs between targets, not its availability.
- **Keep the data table adjacent to the loop that consumes it.** A table
  assembled across three `proc`s in another part of the file is a
  program, not a spec.
- **Patch packs, not edited programs.** The studio edits a canonical pack
  in place and byte-stably, and **never rewrites a programmed pack**
  (`PackStore::programmed`). A form edit against one becomes a canonical
  patch pack in the `StudioOverride` tier, layered over the base by the
  ordinary collision policy (`-override` on each patched declaration,
  patch installed after the base), with the source opening read-only
  beside its expansion. Standing overrides are reported by the store
  (`PackStore::standing_overrides`) so a patch cannot rot silently: fold
  it back into the program by hand when the program is the thing that
  should change, or keep it layered deliberately.
- **Hooks emit facts; assembly evaluates gates.** A hook body never tests
  a target itself, which is what keeps hook results target-independent and
  shape-cacheable.

### Declaring an environment: `environment NAME { … }`

A pack that describes a shell — an interpreter with packages already
loaded, its own file extensions, a fixed base release — declares it as an
environment, as the six bundled EDA packs do (`specs/eda_*.tclspec`). The
rows, in the order the shipped blocks write them:

| row | meaning |
|---|---|
| `display_name {TEXT}` | the human-facing name (defaults to the id) |
| `core FAMILY RELEASE ?-build PROFILE?` | the base release; a compiled family or a `dialect` block the pack declares |
| `version_ceiling RELEASE` | the upper-bound release for option gating, on the core's ladder |
| `editor_identity ID` | one of the **contributed** editor language ids — an environment selects, never mints |
| `ambient PACKAGE VERSION\|tracks-base\|keyed KEY` | a package present with no `package require`; `keyed` names an external version axis (`ToolVersion`, `SdcVersion`, `UpfVersion`, `BigipVersion`) |
| `hosted PACKAGE REQUIREMENT` | an installable package, floored on its own axis |
| `alias NAME` | a retired or convenience spelling that resolves here |
| `file_extension EXT ?-name TEXT?`, `filename NAME`, `signature TEXT` | server-side detection facts |
| `policy open\|closed\|ambient-plus-require` | resolution strictness |
| `help_terms {WORD …}` | the lower-case terms `tcl help --dialect` filters the knowledge base by |

Compiled names are reserved; a bundled pack's names are reserved against
every lower tier; an unknown row rejects the block (it is all semantic
vocabulary). `environment NAME -extend { … }` adds detection facts and
placements to an environment declared elsewhere. The block body is an
evaluated scope like a `command` body, so a version several environments
share is one variable:

```tcl
speclib mypack 2.0 {
    set tkver 8.6

    environment mypack-shell {
        core    tcl 8.6
        ambient Tk $tkver
    }

    environment mypack-plain {
        core tcl 8.6
    }
}
```

A document that resolves to `mypack-shell` gets the Tk 8.6 floor and is
never asked for a `package require Tk`; one that resolves to
`mypack-plain` gets neither. The bundled packs' blocks are also projected
into a generated, drift-gated seed (`cargo xtask gen-bundled-environments`
→ `rust/tcl-dialect/src/model/bundled_environments.rs`) so the six
environments resolve before any pack is published.

### Composing a surface: `include from … into …`

`include NAME` splices another `.tclspec`'s declarations in — file
composition. `include from SOURCE into TARGET ?-available {WINDOW}?
{names…}` composes **surfaces**: it enumerates which of one family's
command names another family, which reimplements it, actually has. `from`
is never a file name.

```tcl
include from tcl into jim {
    append apply array break catch cd clock close concat …
}
include from tcl into jim -available {0.77-} { interp }
```

An ancestry edge alone is too generous: a `Lineage::Fork` inherits its
ancestor's surface wholesale and should, but a `Lineage::Reimplementation`
implements a *subset* — Jim implements "a significant subset of the Tcl
8.6 command set" — and a subset inherited wholesale over-admits everything
outside it. The roster is that subset, written down.

- **The row names both ends.** The target is a compiled family a pack
  cannot otherwise claim (`dialect jim { … }` is refused).
- **`-available` is a window on the *target's* own axis.** `{0.77-}`
  reads on Jim's ladder. A row with no `-available` covers the whole
  ladder.
- **Several rows for one pair are one roster**; they merge at conversion.
- **A malformed row is dropped whole, with a notice.**
- **Rosters fail open.** A pair with no registered roster inherits
  wholesale. A build that did not load the surface pack offers a few heads
  too many; it never offers nothing.
- **Only a trusted tier may narrow a compiled family.** A workspace pack
  that could enumerate `jim`'s inherited surface could delete `proc` from
  it.

Jim's own roster is compiled into the binary
(`rust/tcl-spectcl/core-surfaces/jim.tclspec`) rather than shipped in
`specs/`, because `specs/` is *replaceable* — right for a vendor library,
wrong for a core surface.

## The acceptance rubric

[`spec-dsl-examples/tricky-surfaces.md`](spec-dsl-examples/tricky-surfaces.md)
is the checklist the DSL is held to: the `::tcl::mathop` /
`::tcl::mathfunc` operator-command aliasing and the ensemble implementation
namespaces, every TclOO corner, options as really used, paired and
n-paired tails, dynamic arity, and the full documentation / quick-fix /
analysis-hook surface. No construct enters the frozen syntax until it is
ticked against a ported example, not against intent.

## The studio is the DSL's IDE

The `.tclspec` document is the studio's one authoritative document, with
the form and the Pack DSL pane as projections of it; the contract is
[`contracts/command-spec-studio.md`](contracts/command-spec-studio.md).

## What a pack still cannot say

Behaviour that is not a pure words→data function stays native: commands
needing new lowering/codegen/analyser specialisations are contribution
candidates. The `state_transitions` and `world_effects` block rows are
documented vocabulary the loader does not yet read (dropped with a
notice), a library-defined completion code scoped to one command's body
has no spelling, and a method-scoped taint sink is a registry change
rather than a DSL one — the register is in
[`spec-dsl-examples/README.md`](spec-dsl-examples/README.md), "Known
limits carried forward".
