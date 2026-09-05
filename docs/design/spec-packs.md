# SpecTcl — spec packs as a Tcl DSL

The DSL is named **SpecTcl**. (Two prior arts knowingly share the name —
Sun's 1990s Tk GUI builder and the NSCL physics analysis tool; both are
dormant-to-distant enough in this space that the pun wins.)

> **Status:** under construction, for
> [issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363).
> Design-by-porting sketches live in
> [`spec-dsl-examples/`](spec-dsl-examples/); the frozen syntax is that
> directory's `README.md`.
>
> **Landed (phase 4, the runtime path):** the `.tclspec` loader, the three
> discovery tiers with nearest-wins precedence, multi-file pack merge, the
> compiled-pack cache in the OS cache directory, workspace-scope insertion into
> the per-profile cached `CommandRegistry` under the shipped-wins-unless-
> `-override` collision policy, and LSP integration — packs load at workspace
> init, reload on pack-file change, and their load notices are published as
> diagnostics on the pack file. All of it lives in the `tcl-spectcl` crate.
>
> **Landed (the EDA migration):** the boundary this document draws is now real
> on both sides. `sdc_base` and the five vendor packs — 346 commands — are
> bundled `.tclspec` loadables in `specs/`, shipped beside the server
> executable; their ~350 Rust modules and `CommandRegistry::load_eda_packs`
> are deleted. Every command was proved field-for-field equal to its compiled
> spec through render → load → draft before the modules went, with **zero**
> renderer losses and zero loader notices across all six packs. Two seams made
> it work: `install_into` filters a pack command by the profile's ambient
> packages (all six libraries are discovered for every dialect, and four of
> them declare a `report_timing`), and `Analyser::with_pack_overlay` lets the
> analyser — which resolves its own registry and cannot depend on the loader —
> read the pack-carrying entry, without which an EDA document's every command
> reported as unknown.
>
> **Landed (hook execution):** hook bodies run — see "What exists today"
> below for the crates that do it and the measured cost.
>
> **Landed (phase 2, the workbench):** the `.tclspec` document is the
> studio's one authoritative document, with the form and the Pack DSL pane
> as projections of it; the DSL and Test panes are Monaco with the language
> server compiled to wasm behind them; live save keeps the document, the
> sample, and the open tabs in IndexedDB; `/` searches the pack, the shipped
> registry, and the Reference vocabulary and says which each hit came from;
> ◀ ▶ and the browser's Back are one history; and up to twelve commands are
> open at once in a tab strip, every tab a view of the one document and
> never a copy. The contract is
> [`contracts/command-spec-studio.md`](contracts/command-spec-studio.md).
>
> **Not yet:** the skill-in-studio tab — the page reaches nothing but
> GitHub's two hosts, for the release fetcher.

## The problem

Users with private Tcl libraries cannot contribute their command specs to
the shipped registry, and stubs deliberately carry only a fraction of what
a `CommandSpec` can say. They need a way to author a full command database
for their own code and load it into the server — without a Rust toolchain,
and without rebuilding it for every tcl-lsp release.

## The ambition: one authoring format, two backends

The DSL is not a side door for private specs — it is the **primary
authoring format**, with two outputs:

- **Ahead-of-time**: `tcl spec build --emit rust` renders DSL sources to
  the registry `.rs` modules (the studio's `render_rs` already does
  draft → `.rs`; the DSL maps to the same draft model). The compiled-in
  boundary is decided: **core Tcl and the Tcl dialects, tcllib, Tk,
  iRules, iApps, and Expect all stay compiled in** (their *sources*
  migrating to SpecTcl files that generate the Rust); **the EDA vendor
  libraries ship as bundled `.tclspec` loadables**, as do future
  library additions of their kind — so the loader path is exercised in
  production from day one rather than reserved for private packs.
- **Runtime**: private packs load the same DSL directly, paying only a
  one-time parse.

The equivalence gate falls out of that architecture: re-express a shipped
pack in the DSL, load it, and assert field-for-field equality with the
compiled spec. That round-trip is both the migration test and the proof
the DSL "entirely covers" the spec surface. The design itself is being
driven the same way — by porting hard shipped specs (`lsort`, `switch`,
TclOO/snit definers, `upvar`, `return`) and by drafting specs for
external libraries (ticklecharts, apave, SpiceGenTcl, uncovered tcllib
modules) rather than by inventing syntax in the abstract.

**Where the migration half of that ambition stops, exactly.** It used to
stop at `command_forms` and `subcommand_forms`, which bundle arity, roles
and options together with native literal validators, compiler hooks and
proof descriptors: no partial syntax could be added without making a
half-authorable form look round-trippable.

Q12 resolved that by splitting the descriptor rather than subsetting it.
`refine NAME { … }` is the **invocation refinement** — arity, a literal
`selector`, argument roles, options and relations, availability, and the
replacement `traits` / `mutator` / effects one call shape states — written
in the owning scope's own words and read by the owning scope's own readers.
The native halves (`completion`, `dispatch_dependencies`,
`literal_argument_validator`, and the compiler hook ids) stay Rust-only,
and a form carrying one is *reported*, not thinned, so the round trip never
claims more than it preserves.

The migration test is Tk, whose widget methods are the largest structured-form
user: `tk_form_refinements_round_trip_through_the_pack_dsl` renders every Tk
command that refines its subcommand forms to a pack and asserts the reloaded
form tables equal the compiled ones. The original Tcl/iRules routing users
(`lset`, `incr`, `package vsatisfies`, `namespace upvar`, `HTTP::cookie`,
`HTTP2::stream`) keep their compiler hooks, which is exactly the layer that
stays native.

Plain `forms` remains documentation-only: `FormSpec` carries synopsis and
lifecycle, never traits or effects, and is not a semantic substitute for a
refinement.

## Performance: the format does not decide it

A pack — whatever its syntax — is parsed **once at load**, resolved
against the running registry's vocabularies, and inserted into the same
per-profile cached `CommandRegistry` the built-in packs live in (interned
and leaked once, keyed by content hash). After that, every query — hover,
role lookup, arity, taint — takes exactly the path a compiled-in spec
takes: same map, same struct, same field reads. Runtime cost is
**identical to compiled-in**; the only cost the format controls is a few
milliseconds of load at startup or on pack change, plus one resident copy
of the data.

The one design rule that matters for performance: packs layer into the
cached registry at workspace scope, **not** the per-document overlay path
stubs use — a pack is parsed per edit of the pack, never per edit of the
code that uses it.

### Measured: what the 2.0 vocabulary costs

`cargo run --release -p tcl-spectcl --example speclib_version_costs`
loads every bundled pack three ways over the same source — as it ships,
as `tcl spec upgrade` rewrites it (2.0), and as 2.0 with the static fast
path off, so the pack really is executed as a Tcl program by `tcl-vm`
rather than captured from its CST. All three register the same commands;
the example asserts that before it times anything.

The "1.x" column is **1.1**: that is what all eight bundled packs declare,
and it is the newest vocabulary any of them uses. The ladder is 1.0 → 1.1
→ 1.2 → 2.0, so there is no 1.3 to compare against; 1.2's additions
(versioned `arity`/`arg` rows, `ambient_package`, second-level option
blocks) are words no shipped pack needed. Median of 15 loads, release
build:

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

Three things the numbers say:

- **2.0 costs about a tenth of the load, and nothing else.** The
  `available {tcl 8.4-}` algebra is more words to segment than
  `dialects all-tcl`, which is the whole difference: repeated runs put
  the corpus figure between +8% and +12%, and individual packs swing
  either side of it. On the 35k-line corpus that is ~30 ms once, at
  workspace scope.
- **Memory is unchanged**, because the two spellings lower to the same
  rows: 4.2 MiB against 4.0 MiB across the corpus is noise at this
  resolution. The column measures resident bytes a load *retains* — the
  loader interns what it registers, so that, not a transient peak, is what
  a pack costs the process. It is measured at page granularity, so the
  small packs read `0`, and the interpreted column runs a little higher
  because the VM's own pages are still resident when it is sampled.
- **The static fast path is what makes design E affordable.** Executing
  the corpus as Tcl costs 3.4 s against 0.26 s: a wholly declarative pack
  is captured from its CST instead, and `tests/eval_loader.rs` loads
  every shipped pack both ways and asserts the snapshots are identical,
  which is what makes the shortcut an optimisation rather than a second
  reading of the file. A pack that templates its registrations pays the
  interpreter for the part that needs one.

## The format: a Tcl DSL, parsed by our own toolchain

We have a full Tcl compiler; the authoring format should be Tcl. The
canonical extension is **`.tclspec`** — the editor extensions and the
LSP register it as Tcl in the SpecTcl dialect, so a pack file gets the
full editor experience with no configuration. That requires two
implementation pieces alongside the loader: file-type registration
across every editor integration, and a **compiled-in command pack for
SpecTcl itself** — `speclib`, `command`, `option`, `arg`,
`subcommand`, the hook statements — with its `definition_body` grammar,
so authoring a pack gets highlighting, completion, and
misspelled-trait diagnostics from the same machinery it configures.
(Once the loader exists, that self-spec is written *in* SpecTcl and
AOT-compiled — the final dogfood.) A `.tclspec` file is a declarative
script in a small spec DSL:

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
spell them, so the **?**-button help and the searchable Reference tab
document the DSL for free.

Why this beats JSON for the author:

- **The toolchain can eat its own dogfood.** `speclib`/`command` become
  registry specs themselves, with a `definition_body` grammar — the same
  machinery that gives snit and TclOO bodies highlighting, folding,
  completion, and go-to-definition with no new walker code. Authoring a
  spec pack then feels like writing tcltest or snit: full editor support,
  diagnostics for a misspelled trait or role at the point you type it.
- It is the stub language grown up — same file culture (`*.tclspec`
  beside the code), a superset of what stubs express, with a migration
  path from existing sidecars.
- Tcl quoting and line-continuation are what our users already know.

**The declaration layer is static, never a live interp.** Declarations
are read from the CST the way stubs are — loading a pack executes
nothing. If loops/templating prove necessary ("declare these 40
subcommands"), the escape hatch is evaluation inside `tcl-vm` with only
the spec-building commands exposed (the sandbox posture `tcl pkg`
already takes for untrusted build scripts), still producing pure data.

## Covering the hooks

"Entirely cover hooks" is the hard requirement, and the shipped registry
says how big it really is: of ~2,210 command modules, 44 set an
argument-role resolver, 45 a const-folder, 11 a command-prefix resolver,
and the remaining hook kinds are single digits (one `taint_sink_gate`,
one `context_gate`, one `clause_shape_check`, four literal-argument
validators, five frame effects). Hooks are the 5% tail — but they gate
the hardest commands, so the DSL takes them in four buckets:

1. **Declarative data, first.** The named-descriptor fields
   (`definition_body` grammars, `object_class`, `case_list`,
   `body_scope`, handle bindings, repeated-arg layouts, deprecation
   fixes) are plain structs — `coverage.rs` already enumerates every
   field — so they get first-class DSL forms, not code. A declarative
   clause-grammar form should also absorb most would-be
   `clause_shape_check` uses (`if`'s chain is a grammar, not an
   algorithm).
2. **Named native hooks.** Lowering / codegen / analyser hook IDs,
   shared definer grammars, and `frame_effect` descriptors are closed
   sets referenced by name. A pack reuses them; it cannot add to them.
3. **Tcl hook bodies on the VM.** Resolvers, folders, and the predicate
   gates are small **pure functions from words to data** — ideal
   sandboxed-VM material. Each hook kind gets a fixed calling
   convention (inputs: the call's words plus a context dict; output: a
   declared result protocol; error or no result = abstain), compiled to
   bytecode once at pack load, fuel-limited, no ambient authority. This
   is the one executable surface a pack has, and it is deprivileged,
   deterministic, and bounded. A const-folder even has a degenerate
   sweet case: for a command implemented in pure Tcl, "fold" can mean
   running the implementation itself on the literal arguments.
4. **Stays native.** Anything that cannot be a pure words→data function
   is a contribution, as today.

**Execution engine: `tcl-vm` is the canonical answer everywhere** —
server, CLI, and studio alike (it ships to WASM already). The studio
therefore **defaults to tclvm-in-wasm**, with a visible switch to run
the same hook bodies through the compiler's Tcl→WASM codegen instead:
useful dogfooding that
pressures that backend to mature, and any behavioural divergence
between the two engines on a hook body is differential-testing signal,
not just a bug. Performance investment lands in the VM — hooks give it
a hot, measurable, real workload to justify that effort.

**The framework is a Rust crate linking the VM the way a C extension
links C Tcl.** The DSL host (loader, hook harness, sandbox) registers
its surface natively: the emitter verbs (`role`, `fold`, `reject`, …)
and the sandbox builtins (e.g. the conservative `foldlist`) are Rust
commands in the hook interpreter, and hook inputs/outputs cross the
boundary as structured VM values — never string round-trips. One crate
backs every deployment (native server, CLI, wasm studio), so the
extension-registration path itself becomes a first-class, dogfooded
API — the same road a future C-extension-style plugin story would
take — and the DSL gets the best performance floor the VM can offer.

**Two layers, deliberately.** The bottom layer is a **Tcl extension
interface** in modern, idiomatic Rust — traits and owned structured
values, no raw interp pointers — playing the role Tcl's C API plays for
C Tcl. `tcl-vm` and the Tcl→WASM codegen runtime implement it
(selecting one must not require the other to be present), and it is
designed against exactly **two consumers**: the hook framework, written
in Rust on top of it; and the C-Tcl shim (#1372), which lets users
compile existing C Tcl extensions to run on our tclvm or in wasm
through the same surface. One plug point for every way executable
behaviour can arrive — and an API whose shape is disciplined by having
both a native-Rust and a legacy-C consumer in view from day one.

Three rules govern that interface. It is **stable and common to the
backends** — tclvm and the wasm codegen runtime, explicitly *not* the
BPF backend, where hosted extensions make no sense. It is **designed
for the two use cases and built for the first**: the clean, modern,
idiomatic Rust surface the DSL framework needs now. And **all
C-required mangling lives in the shim, never in the interface** — the
shim absorbs the impedance mismatch (string lifetimes, interp-pointer
idioms, result codes) so the interface never grows a C-shaped wart.
The C-Tcl shim's first leg has landed as `tcl-cshim`
([c-extension-shim.md](c-extension-shim.md)): shimmed extensions are
trusted native code loaded only by host configuration, and nothing in the
`SpecTcl` vocabulary can reference one. The
**hook host** is the layer above: it owns everything DSL-specific — the
emitter verbs, per-family calling conventions and preconditions,
abstention and error policy, fuel budgets, memoisation — and speaks
only the engine interface beneath it. Engine-agnosticism is therefore
structural: the host cannot depend on an engine's identity, and a new
engine (or a shimmed C extension) slots in under the unchanged host.

**Crash containment is a load-bearing guarantee, not a nicety.** A
spec must never be able to take the LSP down: every hook invocation
crosses a containment boundary — `catch_unwind` plus the fuel and
memory budgets in-process (the WASM engine is additionally sandboxed
by construction) — and a panic, budget blowout, or stack overflow in a
hook is converted to abstention, never propagation. Isolation is
**per pack**: each loaded pack gets its own sandboxed VM (or wasm
worker) with no shared interpreter state, so a crash or quarantine in
one pack can never degrade another — one library's bad hook costs that
library its hooks, nothing else. On the first
crash, the offending hook is **quarantined** for the session (the
command falls back to its declarative facts, so one bad hook cannot
re-crash on every keystroke), and a structured crash record is
written: pack name and content hash, command and hook family, the
SpecTcl vocabulary and server versions, the input word *shapes*, and
the panic payload/backtrace. The user gets an editor toast — "a spec
pack hook crashed; tcl-lsp is unaffected" — with an **Open a GitHub
issue** action that pre-fills an issue on tcl-lsp from the crash
record, shown to the user before posting exactly like the studio's
issue flow (raw user-code words are included only in what the user
reviews and sends, never auto-transmitted — nothing leaves the machine
without a click).

**Hot-path budget — measured** (release build, cross-checked against
the shipping `tclvm` CLI): a VM resolver body costs **28 µs** per
invocation against a whole-call-site native budget of **410 ns** — the
VM's floor is ~487 ns per Tcl command, so no marshalling trick closes
a 68× gap. A const-folder is 16.6 µs; pack load by the CST route is
**4.28 ms** for a 2,000-line pack (the "few milliseconds" claim holds
— and pack load must use the CST route, never the flat analyser path,
which is 75× slower). The design consequences are binding:

- **Granularity is not restricted — consequences are documented and
  shown live.** A resolver that depends only on its declared inputs
  and returns the complete index→role map in one invocation is
  **shape-cacheable**: the registry caches it by command + word-shape
  at 24.5 ns/call — indistinguishable from native. A hook that
  declares broader dependencies stays fully legal but uncacheable, and
  the cost is stated plainly rather than hidden: ~28 µs per call site,
  ~48 ms of semantic-token time per 2,000-line file at today's VM
  floor. That consequence is documented here, in the syntax memo's
  hook chapter, and **live in the Spec Studio** — a performance badge
  on the hook editor and measured timings in the Test tab — and
  `tcl spec check` reports it. Users who need the power take the cost
  knowingly; packs that take it and matter are exactly the motivation
  signal for the VM performance work (#1373). A plain word-vector memo
  is not a rescue (2.94 µs at a 90% hit rate — fresh variable names
  are all misses).
- **Folders and predicate gates run on the VM freely** — bounded,
  off the interactive path, contractually pure.
- **The VM boundary itself is on the table** (#1373): building the
  extension interface is the occasion to give `tcl-vm` the first-class
  embedder APIs it lacks — a public `Vm::invoke_command`,
  invoke-by-handle over pre-compiled function handles with no per-call
  `FunctionAsm` clone, enforced command-limit fuel (load-bearing for
  the containment guarantee above), and the qualified-name
  compile-cache fix. The engine implementations of the interface drive
  these improvements rather than shimming around the gaps; the
  measured floors above are the *before* numbers, and the registry's
  shape-keyed caching stays regardless — it clears budget even if the
  VM never gets faster.

## What exists today

Phase 5's first half has landed: hook bodies run, in the two layers this
document specifies, and a pack-declared `const_fold` folds real call
sites in the optimiser. The crates are:

| crate | layer | what it is |
|---|---|---|
| `tcl-engine-api` | bottom | The **Tcl extension interface**: `CompileUnit` → engine handle, invoked with owned structured `Value`s (list and dict are first-class, so `words`/`ctx` never round-trip through text); `HostCommand` for embedder-registered commands; `Budget` the engine must enforce; `EngineError` distinguishing a script error, a budget blowout, and a crash. No dependencies at all — selecting one engine cannot drag in another. |
| `tcl-engine-tclvm` | bottom | The `tcl-vm` implementation. Each interface obligation maps onto a real VM embedder API rather than a shim: `Vm::define_procedure` (compile once), `Vm::invoke_command` (now public), `Vm::register_native_command` (stateful host commands), `Vm::retain_commands` (a closed whitelist), and the enforced `commands` limit + wall-clock cap. |
| `tcl-spec-hooks` | top | The **hook host**: emitter verbs as native commands, the per-family calling conventions and the literal-only precondition, abstention and error policy, per-pack engines, `catch_unwind`, quarantine-on-first-crash with a structured crash record, and the sandbox whitelist plus `foldlist`. Names no engine except in one convenience constructor. |
| `tcl-cshim` | consumer 2 | The **C-Tcl shim** ([c-extension-shim.md](c-extension-shim.md)): a C extension compiled against `include/tclshim.h` registers its commands through `Engine::define_command`, with `Tcl_Obj` crossing as typed values. Trusted native, host-loaded only; unreachable from a pack or a hook body. The interface gained `Engine::remove_command` and the in-invocation `CommandRegistrar` door for it. |
| `tcl-registry::pack_hooks` | seam | Slots, per-family thunk tables, the thread-local host, and the **shape-keyed cache**. A pack hook is a plain function pointer of the family's shipped type, so `run_const_fold` and every other consumer is unchanged and unaware. |

Three consequences of the implementation are worth recording against the
design above:

- **The VM boundary work is done in the VM** (#1373): `Vm::invoke_command`
  is public, `Vm::invoke_function` runs a pre-compiled handle with no
  per-call `FunctionAsm` clone, and the `commands` limit is enforced at
  both dispatch funnels instead of merely stored.
- **The two budgets bound different things.** The command limit counts
  *dispatched* commands — as C Tcl's does — so a loop the compiler inlines
  into bytecode is not charged; the wall-clock cap, polled inside the
  bytecode trampoline, is what stops that case. Containment needs both,
  and the host's default config sets both.
- **The cacheability rule has a spelling**: `-inputs {nwords kinds}` on a
  hook statement. A hook whose declared inputs exclude the words' content
  is answered from the registry's shape-keyed cache; a hook that declares
  nothing is fully legal, always correct, and uncacheable — which is this
  document's "granularity is not restricted, consequences are documented"
  made executable.

### The corpus-validation harness

`rust/tcl-spectcl/tests/spec_corpus.rs` is the gate over **every `.tclspec`
the repository ships** — the six bundled loadables under `specs/`, the
eleven ports and the four external drafts under
[`spec-dsl-examples/`](spec-dsl-examples/). Per pack it loads through the
real loader, installs into a real per-profile registry, runs the analyser
and the optimiser over the corpus files in `samples/` that call the pack's
commands (synthesising an exercising call from a command's own arity and
argument roles when nothing in the corpus reaches it — which is the case
for all six vendor packs and all four drafts), drives the hook bodies
through the sandboxed host at its shipped budgets, and reports commands
loaded, notices, hooks invoked, quarantines, and load + analysis wall
clock. Accepted load notices live in `tests/spec_corpus_baseline.txt`,
compared as a multiset in both directions so a fixed notice must also be
deleted from the baseline.

Its negative half is `tests/fixtures/hostile.tclspec`: an unbounded loop, a
dispatch-heavy fold, and a body that panics. All three end as abstention
with a crash record and a quarantined hook, on a watchdogged thread, so
"a spec must never be able to take the LSP down" fails in bounded time
rather than wedging the suite.

Two things it found on its first run, both now true rather than claimed:

- **`object_class` was ratified vocabulary the runtime loader did not
  implement — now closed.** It is in the frozen syntax and in the
  compiled-in `SpecTcl` self-spec (`commands/spectcl/blocks.rs`), and it
  appeared nowhere in `tcl-spectcl/src/loader.rs` — so all four external
  drafts lost their handle-returning factories' method tables at load, with
  a notice each. Nothing under `specs/` uses the statement, which is how the
  gap survived the EDA migration. The loader now reads
  `object_class NAME ?-superclass {…}? ?-allow-unknown?
  ?-method-prefix-matching Enabled|Strict? { method … }` into a
  real `ObjectClassSpec` — `method` rows are the `subcommand` body grammar
  unchanged, because `instance_methods` *is* `&[SubCommand]` — and the spec
  studio's `SpecTcl` renderer writes it back out, so the descriptor left the
  renderer's gap register too. All thirteen notices are gone from the
  baseline; what replaced them is six `arg`-row flag drops and one
  unbindable method hook body that those blocks were hiding, plus the
  twenty-seven for the `state_transitions` / `world_effects` rows the loader
  still says are "not yet loadable".
- **A pack could abort the analyser through declarative data alone.**
  `command_table_effect CreatesAliases` describes `interp alias`'s word
  grammar and the shipped registry stamps it on that subcommand; the tcllib
  draft stamps it at command level on `struct::tree` / `struct::graph`,
  which build their handle through `interp alias` internally. The
  destructuring helpers `debug_assert`ed the `alias` word, so a debug build
  aborted, and a release build would have invented an alias out of the
  command's own arguments. `crate::alias::is_interp_alias_shape` makes that
  a fact check instead: a call that is not `interp alias`-shaped states no
  alias. Containment is a hook-body promise in this document; this is the
  reminder that *data* crosses the same boundary.

## Compatibility policy

The "ABI" is the DSL vocabulary, and it follows the tolerance rules that
let packs survive releases without rebuilds:

- Unknown property words, trait names, role names, or hook names are
  dropped with a logged notice; the rest of the spec loads. New server +
  old pack always works; old server + new pack degrades gracefully.
- **Except where dropping the word would strengthen the answer.** Since
  `SpecTcl` 2.0 (redesign §6.1, review B13) an unknown word in a pack
  declaring a vocabulary this build postdates is classified by its
  compatibility effect: *presentation* words warn and drop as above;
  *assistance* words (shapes, roles, value sets) leave the command known
  but degraded, so the affected capability answers `Unknown`; *semantic*
  words (security, control flow, binding, lowering, codegen — and every
  unknown word inside a `dialect` or `environment` block) exclude the
  command or block from strong analysis. An old server that ignored a
  "this method is a sink" word would otherwise report a *cleaner* result
  than a new one, purely by not understanding the field.
- A `speclib` version pragma gates hard breaks only — a word whose
  *meaning* changed, not one that was added. The server refuses a major
  it does not know and names the fix: the pack loads nothing at all, and
  one notice says why. An unknown *minor* within a known major keeps
  loading maximally.
- The studio schema-coverage gates already force every new `CommandSpec`
  field to a named key; that key is the DSL property name, so the format
  cannot silently fall behind the registry.

## Version ranges: introduced, deprecated, retired

Every entity the registry can gate carries the same `Lifecycle` triple —
an introducing release, a deprecating release, and a retiring release, on
either the owning package's version axis or the core Tcl axis. Registry
work has closed the parity holes between what a compiled-in spec could
say and what a pack can say: the triple now exists at **every** gateable
level, not just the command:

- the command itself (`CommandSpec::lifecycle`),
- a subcommand, and a second-level operation of a two-level ensemble
  (`SubSubCommand`, e.g. `info object class`),
- an option, at command or subcommand scope,
- an option constraint (a mutual-exclusion or requires-together rule
  between options),
- a side effect,
- an invocation form (`FormSpec`), and
- a literal argument value in a closed set (`ArgValue::lifecycle`), which
  rides the owning-package axis and is a separate fact from
  `ArgValue::min_tcl` (the Tcl-core-gated argument-DSL rung, `W137`).

`CommandSpec` also gained `versioned_arg_values`: a command-level mirror
of the per-subcommand literal-value gate, for a value that sits directly
on the command rather than behind a subcommand word (`HTTP::respond
<status> noserver`, `close $chan read`). A value gated by both its own
`ArgValue::lifecycle` and a `versioned_arg_values` entry is narrowed by
`Lifecycle::intersect` and recorded once, so a doubly-gated value never
draws two diagnostics for one word.

Only three of these levels are wired into the analyser's diagnostics
today — the command, the subcommand and sub-subcommand, options, and
argument values (command- and subcommand-scoped) all feed `W135`/`W136`/
`W139`/`W144` via `version_gate.rs`. Option constraint, side effect, and
form lifecycles are registry data and are validated by the registry
sweep (below) but are not yet read by a diagnostic — a pack or a shipped
spec can declare them, and the field round-trips through the studio, but
nothing in the editor reports a use of a deprecated or retired *form* or
*side effect* yet.

### Ordering and containment: notice-only for packs, a hard gate for shipped specs

A `Lifecycle` must be internally ordered (introduced ≤ deprecated <
retired, where each is present) and a child's declared releases must fall
inside its parent's window — a subcommand cannot claim an introduction
before the command that hosts it existed. Both properties are checked at
two different strengths, on purpose:

- **Shipped specs — a hard gate.** `rust/tcl-registry/tests/registry_sweep.rs`
  walks every compiled-in command, recursing into every level above, and
  asserts both properties (`Lifecycle::validate`, `Lifecycle::intersect`
  for containment) for each one. A shipped spec with an invalid or
  non-contained lifecycle fails the test suite outright.
- **Packs — notice-only.** The loader (`rust/tcl-spectcl/src/loader.rs`)
  runs the same two checks on every `-introduced`/`-deprecated`/
  `-retired` lifecycle it parses, at every level. An invalid ordering
  drops the lifecycle back to `Lifecycle::UNSPECIFIED` with a logged
  notice naming the field and the reason; a lifecycle that reaches
  outside its parent's window is reported the same way but **kept** — the
  declaration still loads, unlike the ordering failure. This follows the
  format's own compatibility policy above: a pack degrades gracefully
  where a shipped spec is held to a hard gate, because only the shipped
  registry is what the rest of the toolchain treats as ground truth.

### The requirement-range straddle rule

`package require Foo A-B` guarantees only that the loaded `Foo` is
somewhere in `[A, B)` — the version-gate floor check asks about `A`
alone, so a range that runs past a retirement can pass the floor while
still failing for part of the accepted window. `version::requirement_upper_bound`
extracts that ceiling from a **stated** range only: `A-B` participates,
but a bare `A` or an open `A-` states no ceiling and is silently exempt
from the straddle check — there is nothing to straddle. `A-A` is a
degenerate pin the floor check alone already decides.

When the ceiling reaches strictly past a retirement, `Analyser::requirement_straddle_diagnostic`
emits the same `W139` the plain retirement check would, but hedged:

```text
'trace variable' is not available in every version satisfying requirement
`8.5-9.1`: removed in Tcl 9.0.
```

rather than the ordinary "was removed in … but …" phrasing, because the
floor itself is satisfied — the item really is available at the low end
of the requirement, just not everywhere the requirement admits. The
straddle diagnostic only fires while the floor's own verdict is
`Available`; a floor that already fails keeps its own, more specific
message. See [W139](../kcs/codes/kcs-diagnostic-w139-retired-at-resolved-version.md).

### `speclib` 1.1: an additive vocabulary revision

`speclib` 1.1 unifies the lifecycle spelling to `-introduced`/
`-deprecated`/`-retired` flags at every level described above, including
the command and subcommand levels, which previously took
`introduced_version`/`deprecated_version`/`retired_version` as separate
dict-style properties. This is additive, not a breaking change — the
older spellings keep working, and 1.1 only adds a consistent shorthand —
so `VOCABULARY_VERSION` (`rust/tcl-spectcl/src/lib.rs`) stays `"1"` per
the compatibility policy above: a version pragma bump is reserved for a
word whose *meaning* changes, and no word's meaning changed here. The
authoritative spelling table for every level lives in
[`spec-dsl-examples/README.md`](spec-dsl-examples/README.md); this
document does not duplicate it.

### `speclib` 1.2: versioned arity, versioned arguments, ambient packages

1.2 is additive in the same sense 1.1 was — no word's meaning changed, so
a 1.0 or 1.1 pack loads unaltered, and a 1.2 pack loads on this server
whatever it declares. It closes the versioned-signature gap and adds the
  registry data needed to author Tk input links, callback timing, geometry
  managers, and source-proven object-method prefix dispatch.

- **The three lifecycle flags on an `arity` row.** An `arity` row without
  them is the command's plain arity, exactly as before. One with them is
  a **window**: the shape the command had over one span of its owning
  package's releases. Several may be declared, and the plain row stays
  the fallback for a resolved floor no window covers.

  Windows must **not overlap** — two covering the same release would make
  the selected signature depend on declaration order, which no pack can
  have meant. Consecutive windows are therefore written *closed*: a
  window with no `-retired` never ends, so "two arguments from 3.0, three
  from 5.0" is spelled with the first retiring where the second begins.
  A pack that overlaps keeps the first window and gets a notice; a spec
  this repository ships is rejected outright by `registry_sweep`. That
  split is the standing one — a pack is authored elsewhere and must
  still load.

- **The three lifecycle flags on an `arg` row.** A per-argument fact
  (role, type, values, layout, `-closed`, `-appends`) can now be gated to
  the releases that have it. The loader reads each `arg` row as one
  record and *projects* it into the six parallel per-argument tables the
  registry stores, so a row outside the floor drops out of all six at
  once rather than being filtered in five places and forgotten in the
  sixth.

  The registry's stored tables are the projection at **no floor**, which
  is what every consumer already reads; the authored rows are retained
  beside them so a consumer holding a resolved floor re-projects at it.
  A pack with no gated row retains no rows and is bit-for-bit what it was
  before 1.2.

- **`ambient_package NAME VERSION`.** A package the pack's dialect
  provides without a `package require` — the pack-authored twin of an
  ambient `LibraryPin`. A package that comes *with* the dialect is never
  required in the documents that use it, so before this nothing could
  give its commands a version floor and every per-release gate on them
  went unchecked.

  It composes with the other two sources of a floor by taking the
  greatest, exactly as two `package require` lines already do. When two
  are equal, the diagnostic names the one closest to the author's own
  control: the require in this file, then the pack in this workspace,
  then the profile compiled into the server.

  The row is **unscoped by construction**: it floors its package for
  every document the pack is active in. Issue #1643 — filed before 2.0
  existed — asked for a `-dialects {…}` flag to narrow it to some of the
  pack's dialects. That flag is *not* vocabulary, and the loader refuses
  it rather than reading it or ignoring it: see
  ["Scoping an ambient package"](#scoping-an-ambient-package-issue-1643)
  below for what to write instead and why the row fails closed.

- **`option … -taints-var-write`.** Marks a variable-valued option whose
  linked variable can be written from external input. The bit belongs to the
  individual option argument, so an editable widget's `-variable` can be a
  source while a display-only `-textvariable` remains clean.

- **`option … -variable-scope CurrentFrame|Global`.** Declares where an
  unqualified variable-name option resolves. `CurrentFrame` is the default;
  `Global` models documented interpreter-global links such as Tk
  `-textvariable` and `-variable`, allowing SSA and taint summaries to treat
  `value` as `::value` even when the option appears inside a procedure.

- **`option … -script-timing SameInvocation|Deferred|ReferenceOnly`.** Separates temporal
  control flow from `-body-kind`, which describes only a Body's scope. The
  timing also applies to executable `CommandPrefix` and `LambdaLiteral` values.
  `SameInvocation` is the default and remains a lowering barrier;
  `Deferred` says the receiver stores the script for a later callback;
  `ReferenceOnly` says it identifies code without invoking or storing it.
  Neither can abort the current command or hide definitions after it.

- **`option … -callback-taint-inputs {%P …}` and
  `callback_taint_inputs {{INDEX {%A …}} …}`.** Declare the finite set of
  externally controlled substitutions a *deferred* callback host injects.
  The option form applies to its script value; the table form applies to a
  positional callback argument (indices use the normal command/subcommand
  coordinates). Today the authorable Tk values are validation text `%P`, `%s`,
  `%S` and key-event text `%A`, `%K`. Widget paths, validation actions,
  indices, and reasons (`%W`, `%d`, `%i`, `%V`) are intentionally rejected:
  they are framework metadata, not user input. Static analysis replays only a
  literal script or a literal `[list command …]` prefix; dynamic construction
  abstains rather than guessing. Each recoverable replay has an independent
  synthetic callback frame and direct external-input seed, so one handler's
  locals, `return`, or `error` cannot affect another handler's result. Calls
  to real global procedures retain ordinary interprocedural propagation. The
  deliberate remaining limit is an explicit shared global (for example
  `set ::state …`): real events can race or run in either order, while the
  static model does not attempt an event-order proof.

- **`script_timing_resolver {words ctx} { … }`.** Handles executable
  positions whose timing depends on the written form rather than one option
  row. It emits `timing IDX SameInvocation|Deferred|ReferenceOnly`; silence preserves the
  exact option timing and then the invocation-wide fallback. The index must
  already be a `Body`, `LambdaLiteral`, or `CommandPrefix`, so the resolver
  cannot turn ordinary data into code. Command and subcommand hooks use their
  usual coordinates: after the command name, or after the subcommand word.
  This is what lets one `send`-shaped command describe a synchronous form and
  a `-async` form without applying command-wide `DEFERS_BODY` to both.

- **`object_class … -method-prefix-matching Enabled|Strict`.** Controls the
  instance-method table rather than the factory command's own subcommand
  table. It defaults to `Strict`; use `Enabled` only when runtime source or
  documentation proves unique-prefix dispatch (as for Tk widget commands).

- **`tk_geometry POLICY ?-container-option OPTION? ?-direct-form?
  ?-placement-subcommand NAME? ?-release-subcommands {NAME …}?`.** Declares a
  Tk geometry manager's container policy as `Exclusive` or `Independent`, the
  option that redirects placement into another container, and the exact forms
  that place or release widgets. For example, `grid` has a direct placement
  form, `configure` also places, and both `forget` and `remove` release current
  placement. Static preview and TK1001 consume this descriptor without naming
  `pack`, `grid`, or `place`.

`ambient_package` is also the prerequisite for modelling a package as a
pack at all (issue #1631): a package's version floor must not depend on
this repository happening to know the package's name.

**Deriving windows rather than writing them.** The version importer
(`tcl-spec-studio`'s `import_package_versions`) reads several releases of
a package's sources and now *derives* the windows: runs of equal shape
across the snapshots become windows, each closed where the next shape
arrives. It used to report an arity change as a note and stop, because
the registry had nowhere to put the answer. The note survives beside the
derived field as its evidence — this is a derivation from observed
snapshots rather than a transcription, so the trail that produced it has
to be inspectable.

**What the analyser does with them.** The window covering the document's
resolved floor is the shape a call is checked against, and the plain
`arity` is the fallback when no window covers it. A call whose count
fails the selected shape but fits *another* declared window draws
[W149](../kcs/codes/kcs-diagnostic-w149-arity-matches-other-version.md)
rather than a bare "too many arguments": the call is not malformed, it is
written for a different release, and the two have different fixes. A
count fitting no window at all stays an ordinary E002/E003.

### Scoping an ambient package (issue #1643)

`ambient_package` says "this package is here, at this version" about the
whole pack. Issue #1643 asked for the narrower claim — *here, under this
dialect and not that one* — and proposed spelling it as a flag,
`ambient_package NAME VERSION -dialects {…}`. The issue predates SpecTcl
2.0, and 2.0 answers the same question from the other end.

**What to write.** State the placement inside the environment that has
the package, with `environment NAME { ambient PACKAGE VERSION }` (the
redesign document's §6.2). An `environment` body is an evaluated script,
exactly as a `command` body is, so a version several environments share
is written once as an ordinary variable and a repetitive ladder is an
ordinary `foreach` — there is no scoping vocabulary to learn, and nothing
says the package is ambient anywhere it is not:

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
`mypack-plain` gets neither. Both answers come from the same placement,
so they cannot drift apart.

The block reader is still the single owner of what a row *means*: the
body decides which rows are registered, and `environment_block` decides
whether each one is valid. An unknown row is semantic-class vocabulary
and rejects the whole block however it was produced.

**Why the flag is refused rather than read.** Two reasons, both from the
model rather than from taste:

- **It cannot be desugared.** A `-dialects` list naming a compiled
  environment (`tcl8.6`, `f5-irules`) means "add an ambient placement to
  that environment", which is `environment NAME -extend { ambient … }` —
  and §6.4's trust lattice forbids exactly that for the workspace and
  Spec Studio tiers most packs come from. The sugar would work for
  bundled packs and fail the whole registration for everyone else.
- **It would fork the floor model.** A scoped row kept in the pack's own
  ambient table reports as a pack-declared floor; the same claim written
  as a placement reports as the environment's own. One question with two
  answers, differing only in how it was spelled, is the parallel table
  the #1631 redesign set out to remove.

**Why the whole row is dropped.** Ignoring an unknown flag and keeping
the row would leave the *unscoped* claim standing — the pack would floor
the package in every dialect, including the ones it had just said the
package is absent from. That is the compatibility contract's fail-closed
rule read backwards: an availability-**narrowing** word that a reader
cannot honour must not leave the wider claim behind. So the row fails
closed, with a notice naming the environment spelling to use instead, and
the same rule is registered generically — a scoping word is
semantic-class vocabulary, never decoration.

## Loading and tooling

- **A pack is a logical unit, not a file.** Authors group however they
  like — one big `.tclspec`, one per namespace, one per command — and
  every file whose `speclib` names the same pack merges into one pack
  model at load. Merging is deterministic (files in sorted path order);
  a command defined twice within one pack is a load-time diagnostic
  with the first definition winning, never a silent overwrite. The
  compiled cache keys per file and per merged pack, so touching one
  file recompiles one file.
- Discovery, three tiers with defined precedence (nearest wins:
  workspace > user > bundled): **workspace** — `tclLsp.specPacks`
  paths (mirrored as a setting in every editor integration), plus
  `*.tclspec` beside a `tclpkg.tcl` manifest or under `.tcl-lsp/`;
  **user** — packs the user drops in the platform config directory
  (`$XDG_CONFIG_HOME/tcl-lsp/specs/` and the macOS/Windows
  equivalents via the platform-dirs machinery `tcl pkg` already uses),
  loaded for every workspace; **bundled** — the shipped EDA loadables.
  Name collisions with shipped specs are reported, shipped wins unless
  the pack says `-override`.
  The bundled tier reads the `specs/` directory beside the running
  executable. A browser worker has neither, so when that yields nothing
  discovery walks the server's closed-file store at
  `discovery::VIRTUAL_PACK_MOUNT`, where the host upserts its own packs —
  additively, so the shipped loadables survive. Additive is **keyed by file
  name**: the shipped loadables are compiled into the server as a fallback,
  and one is used only when the host mounted no file of that name. A host may
  mount a vendor pack of its own, the shipped packs themselves (which is what
  the VS Code web extension does at startup), or both, and each pack loads
  exactly once either way. See
  [contracts/lsp-source-store.md](contracts/lsp-source-store.md),
  "The virtual spec-pack mount".
- **Compiled-pack cache in the OS cache directory**
  (`$XDG_CACHE_HOME/tcl-lsp/spectcl/` and platform equivalents): on
  first load a pack's compiled form — resolved drafts plus hook
  bytecode — is written keyed by a lightweight non-cryptographic hash
  (xxhash-class) of the pack source **plus the SpecTcl vocabulary
  version and loader build**, so an edited pack or an upgraded server
  recompiles exactly once and everything else is a hash-check and an
  mmap-fast read. The cache is disposable by contract: delete it and
  nothing breaks but first-load time; a corrupt or stale entry falls
  back to a fresh parse, never an error.
- `tcl spec check lib.tclspec` validates a pack from the CLI; the
  Spec Studio gains a DSL renderer beside the `.rs` and stub renderers
  (drafts round-trip through it), and the `spec-author` skill emits the
  DSL for the private-library path. `tcl spec build` pre-warms the same
  cache — an optimisation, never a requirement.

### Editor registration of pack-claimed file extensions

A pack's `file_extension NAME -dialect D` row (and the `file_extensions` of a
pack-declared `environment` block) routes the extension server-side the moment
the pack is discovered. The editor is a step behind: it learns its
extension-to-language mapping from a static manifest written long before the
user's pack existed, so the file opens as plain text and the language client
never attaches (issue #1626).

The server closes that gap by *advertising* the pairs. `pack_file_extensions`
appears on the `tcl-lsp.getEffectiveConfig` result and again in the
`tcl-lsp/specPacksReloaded` notification, which is sent once a reload has
fully landed — a client cannot derive that moment for itself. Each row carries
the extension, the claiming pack, the dialect, and the **existing** editor
language id the extension should ride, because no editor can mint a new
language id at runtime.

Registration is therefore a per-editor problem, and reversibility is the hard
half: reconciliation has to delete as well as add, so "did we write this"
must be answerable exactly.

#### VS Code

The advertised set is projected into **workspace-scoped**
`files.associations`, and the entries the extension owns are remembered in
workspace state as `{glob: languageId}` — the value written, not just the key.
An entry is ours only while the configuration still says what we last wrote
there, so a user who retargets `*.foo` by hand takes ownership permanently: it
is neither rewritten nor retired. Globs are case-folded per character
(`*.[fF][oO][oO]`), matching the server's case-insensitive routing.
Already-open documents are flipped onto the new language with
`setTextDocumentLanguage`, but only for the associations reconciliation
actually owns.

#### JetBrains

`FileTypeManager` associations are **IDE-global** — there is no
workspace-scoped layer to write into — so the JetBrains half (issue #1650)
keeps its own ledger instead of relying on a scope to contain the damage.

- **What is registered.** Each advertised row maps onto a file type the plugin
  already contributes: `language_id` `tcl-irule` selects the **iRule** type,
  every other id (including plain `tcl`) selects **Tcl**. Association happens
  through `FileTypeManager.associate` with an `ExtensionFileNameMatcher`, on
  the event dispatch thread inside a write action; the platform fires its own
  file-types-changed event from there, so editors already showing a
  newly-associated file re-detect without further help.
- **The ownership ledger.** The application-level `TclLspPackAssociations`
  state persists `{extension: fileTypeName}` for the associations *the plugin
  itself installed*. An association is retired only when the extension is no
  longer claimed **and** the IDE still reports exactly the file type the ledger
  records. Anything else — an extension the plugin never claimed, or one whose
  association the user has since changed — is dropped from the ledger and left
  alone.
- **Manual associations win.** Before claiming an extension the plugin asks
  `FileTypeManager.getFileTypeByExtension`. If anything already owns it, the
  claim is skipped and never recorded, so a later pack removal cannot delete a
  user's mapping. That covers a user who mapped the extension to the plugin's
  own Tcl type by hand: the plugin did not install it, so the plugin will not
  remove it.
- **Restart survival.** The ledger is persisted, and IDE file-type
  associations persist on their own, so nothing is torn down at shutdown. The
  first report of the next session is what retires an association whose pack
  has gone: the extension is absent from the claim set, the recorded file type
  still matches, so it is removed.
- **Multi-project.** Associations are global but claims are per project. The
  service keeps one claim set per open project and registers their **union**;
  an association is retired only when no open project still claims it. If two
  projects map the same extension to different file types the plain **Tcl**
  type wins — both file types use the same language, so it is the safe
  superset. A project that has not started its server yet contributes nothing,
  so at startup an association can briefly retire and return; the file type
  settles as soon as that project's server reports. A closing project drops
  its claims through its own project service, and what it alone claimed is
  retired there and then — but only while another project is still open,
  because every project closes in turn when the IDE exits and a pass with
  nothing left would retire the lot.
- **Attachment follows the claim.** The plugin decides whether to start the
  language server from the file's extension, so a dynamically-claimed
  extension is added to that set too — otherwise the file would open as Tcl
  and still get no server.

### Workspace trust: the setting is gated, the workspace tier is not

VS Code's untrusted-workspace mode splits the loading rules above in two,
and the split is deliberate.

`tclLsp.specPacks` is in the extension's `restrictedConfigurations`
(`editors/vscode/package.json`), so an untrusted workspace's value for it is
ignored — only the user's own value applies. It has to be: the setting names
arbitrary paths, and a `.vscode/settings.json` is workspace-authored content,
so honouring it would let a repository point the loader at any file on the
machine, including one the user never opened.

The rest of the workspace tier — a `.tclspec` under `.tcl-lsp/`, or beside a
`tclpkg.tcl` manifest — is **not** gated, and loads in an untrusted workspace
with no setting at all. It cannot name anything outside the folder the user
opened, so it is exactly the same class of content as the `.tcl` files the
analyser already reads.

What makes that safe is not trust, it is the hook sandbox above: a pack's
only executable surface is its hook bodies, they are pure words-to-data
functions on a closed command whitelist (no `open`, `exec`, `source`, or
`socket`), each pack gets its own engine, and every invocation runs under a
command count and wall-clock budget with `catch_unwind` and
quarantine-on-first-crash around it. A workspace pack can therefore make the
editor say something wrong about the workspace's own code; it cannot reach
the machine. If that ever stops being true — a hook family gaining ambient
authority — the workspace tier has to become trust-gated in the same breath.
## Authoring rules for SpecTcl 2.0 (design E)

Under design E a pack is **evaluated**, not walked: the file runs as a Tcl
program in a deterministic sandbox, and what loads is the snapshot of
registrations it made
([`spectcl-design-e-deep-dive.md`](spectcl-design-e-deep-dive.md) §1). Two
consequences shape everything an author does — a pack can now *template* its
declarations, and a reader of the file no longer necessarily sees the surface
it produces. The rules below keep the first without paying for the second.

- **Write canonical form unless repetition is the problem being solved.**
  The **canonical subset** (E-R11) is straight-line registration calls only —
  today's declarative vocabulary, no `proc`, `set`, `foreach`, or computed
  argument. Every pack shipped so far is canonical, canonical source and
  snapshot are a bijection modulo formatting, and everything that *generates*
  a pack emits it: the studio's renderer, `tcl spec import` and its MCP twin,
  `spec upgrade --restyle`, stub-tier conversions. Programs are for humans
  with a repetition problem — forty commands differing in two fields — not
  for saving four lines.
- **Run `spec export` and read the expansion before shipping.**
  `tcl spec export` (MCP: `spectcl_expand`) renders any snapshot back as
  canonical source. Generate the template, expand it, read the expansion as a
  diff against intent, iterate. Expansion is **total**; contraction —
  recovering a program from its snapshot — is never attempted, so the
  expansion is the whole truth about what a pack registered. A templated pack
  whose author has never read its expansion is exactly the opacity the
  frozen-snapshot model exists to prevent.
- **Prefer an `-available` row to a branch on `available?`.** Branching on
  `available?` while registering makes the snapshot **one analysis target's**
  answer rather than the pack's: the pack is marked target-dependent (E-R1),
  carries a notice saying so, and is excluded from snapshot caching. An
  `-available` row states the same fact as *data*, and one snapshot then
  serves every target correctly. Keep `available?` for the rare case where the
  *shape* of a declaration differs between targets, not its availability.
- **Keep the data table adjacent to the loop that consumes it.** A templated
  declaration is readable exactly when its rows are on the screen above it.
  A table assembled across three `proc`s in another part of the file is a
  program, not a spec, and the next reader will run `spec export` instead of
  reading it — which is a smell, not a workflow.
- **Patch packs, not edited programs.** The studio edits a canonical pack in
  place and byte-stably, and **never rewrites a programmed pack** (E-R12).
  A form edit against one becomes a canonical patch pack in the
  `StudioOverride` tier, layered over the base by the ordinary collision
  policy (`-override` on each patched declaration, patch installed after the
  base), with the source opening read-only beside its expansion. Standing
  overrides are reported — by the store, and by `spec check` — so a patch
  cannot rot silently: fold it back into the program by hand when the program
  is the thing that should change, or keep it layered deliberately.
- **What the sandbox guarantees, so authoring can lean on it.** No clock, no
  IO, no network, no processes, no environment, no threads; registration is
  transactional (any hard error loads nothing at all); budgets bound command
  steps and value size, and wall clock on targets that have a real one — the
  browser evaluates under the step budget alone, because a page's throttled
  `Date.now()` would make the same pack load in one tab and fail in another.
  A runaway `foreach` is therefore a budget notice naming its axis, not a hung
  tool, which is what makes it safe to run a *generated* pack through
  `spectcl_check` before reading a line of it.

### Declaring an environment: `environment NAME { … }`

A pack that describes a shell — an interpreter with packages already
loaded, its own file extensions, a fixed base release — declares it as an
environment, and the six bundled EDA packs do (`specs/eda_*.tclspec`; the
compiled shells they replaced are gone, D17). The rows, in the order the
shipped blocks write them:

| row | meaning |
|---|---|
| `display_name {TEXT}` | the human-facing name (defaults to the id) |
| `core FAMILY RELEASE ?-build PROFILE?` | the base release; a compiled family or a `dialect` block the pack declares |
| `version_ceiling RELEASE` | the upper-bound release for option gating (§5.2 of the redesign), on the core's ladder |
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
placements to an environment declared elsewhere.

### Composing a surface: `include from … into …`

`include NAME` splices another `.tclspec`'s declarations in — file
composition. `include from SOURCE into TARGET ?-available {WINDOW}?
{names…}` composes **surfaces**: it enumerates which of one family's
command names another family, which reimplements it, actually has. The
two share a word because they are the same idea at two scales, and they
are told apart by the second word — `from` is never a file name.

```tcl
include from tcl into jim {
    append apply array break catch cd clock close concat …
}
include from tcl into jim -available {0.77-} { interp }
```

It exists because an ancestry edge alone is too generous. A
`Lineage::Fork` inherits its ancestor's surface wholesale and should:
a fork *is* the ancestor's source plus changes. A
`Lineage::Reimplementation` implements a *subset* — Jim implements "a
significant subset of the Tcl 8.6 command set" — and a subset inherited
wholesale over-admits everything outside it. The roster is that subset,
written down.

Rules a roster author needs:

- **The row names both ends.** A roster is a two-ended fact, and the
  target is a compiled family a pack cannot otherwise claim (`dialect
  jim { … }` is refused — compiled family names are reserved). Saying
  `into TARGET` out loud beats deriving it from which file the row sits
  in.
- **`-available` is a window on the *target's* own axis.** `{0.77-}`
  reads on Jim's ladder, not Tcl's: it is when *Jim* grew the name.
  A row with no `-available` covers the whole ladder.
- **Several rows for one pair are one roster.** That is how per-release
  windows are written without repeating the pair on every line; they
  merge at conversion.
- **A malformed row is dropped whole, with a notice.** A roster that
  loaded *partly* would narrow a family's surface by an amount nobody
  wrote.
- **Rosters fail open.** A pair with no registered roster inherits
  wholesale — today's behaviour. A build that did not load the surface
  pack offers a few heads too many; it never offers nothing.
- **Only a trusted tier may narrow a compiled family.** Rosters sit with
  the grammar declarations at the top of the trust lattice: a workspace
  pack that could enumerate `jim`'s inherited surface could delete `proc`
  from it.

Jim's own roster ships compiled into the binary
(`rust/tcl-spectcl/core-surfaces/jim.tclspec`) rather than in `specs/`,
for the reason the last rule gives: `specs/` is *replaceable* — a
distribution, `TCL_LSP_SPEC_PACK_DIR`, or a dev checkout can put a
different directory in front of it — and that contract is right for a
vendor library and wrong for a core surface. It is `SpecTcl` in every
sense that matters, read by the one loader through the same words a
third-party pack would use; it is simply not a file anyone can take
away.

## The acceptance rubric

[`spec-dsl-examples/tricky-surfaces.md`](spec-dsl-examples/tricky-surfaces.md)
is the checklist the design is reviewed against: the `::tcl::mathop` /
`::tcl::mathfunc` operator-command aliasing and the ensemble
implementation namespaces, every TclOO corner (both `oo::define`
spellings, flag-keyed and dialect-gated members, manufacturers, handle
bindings), options as really used (`--` terminators, options changing
later arguments' language or arity, abbreviation rules), paired and
n-paired tails, dynamic arity, and the full documentation / quick-fix /
analysis-hook surface. No construct graduates from sketch to proposal
until the review ticks its rubric lines against a ported example, not
against intent.

## Phasing

1. **Freeze the surface**: one DSL word per schema key, enumerated from
   the studio's coverage gates; the syntax memo and ported examples in
   `spec-dsl-examples/`.
2. **Port the hard shipped specs** by hand (`lsort`, `switch`, `string`,
   the TclOO/snit definers, `upvar`, an iRules taint command) and let
   the syntax fight back before any loader exists.
3. **External corpus**: draft specs for ticklecharts, apave,
   SpiceGenTcl, and uncovered tcllib modules; every construct the DSL
   cannot express is a design bug filed against phase 1.
4. **Loader + equivalence gate**: static parse → draft → registry
   insert; round-trip a shipped pack and assert equality with the
   compiled specs.
5. **Hook bodies on the VM**, resolver and folder first, behind the
   measured budget; then the `--emit rust` backend, at which point
   shipped packs can migrate to DSL sources with no runtime change.

## Phase 2: the studio becomes the DSL's IDE

Once the DSL definition is frozen (post-review), the Spec Studio grows
from single-spec editor to pack workbench. Agent-built (Sonnet/Opus per
task weight), spec'd here so the work has a target.

**Architecture: the DSL text is the single backing store.** The pack's
DSL source is the studio's one authoritative document; every surface is
a live projection of it. The GUI form edits it through targeted
syntax-tree edits (the red/green CST makes comment- and
formatting-preserving surgery possible — a form edit never clobbers the
author's layout); the DSL tab edits it as text; the AI tab proposes
patches to it; the Test tab loads it through the real pack loader; and
the `.rs` renderer becomes a pure DSL→Rust translator. One change
anywhere flows through everywhere immediately — same parse, same
overlay registry, same outputs — and live-save persists exactly that
one document, which is also the deliverable. The JSON draft model
remains only as an internal intermediate; it is no longer a store.

Concretely, the models live apart from every UI. Two of them: the
**built-ins** (the immutable registry the wasm ships — reference
material, never edited) and the **user definitions** (the DSL-backed
pack store above). One resolution facade merges them — pack overlays
registry, collision rules applied — so every surface queries the same
merged world the user's editor would see. UI surfaces hold no state of
their own: interaction anywhere (a form field, a DSL keystroke, an AI
patch, a sidebar action) dispatches an edit to the user-defs model,
and every projection re-renders from the models. That is what makes
"live everywhere" a property of the architecture rather than a matrix
of pairwise syncs to maintain.

- **DSL as a first-class surface.** A DSL renderer beside the `.rs` and
  stub panes, and a DSL *reader*: open an existing `.tclspec` (hand
  written, studio-saved, or emitted by the `spec-author` skill) and get
  the whole pack as editable drafts. The studio's unit of work becomes
  the **library**: a pack browser beside the registry browser, multi
  command editing, pack-level defaults and shared tables surfaced.
- **Point it at a directory; the machine does the mechanical share.**
  The import surface grows from per-file proc inference to a
  whole-library pass (directory picker in the browser, a path for the
  CLI twin): the compiler's inference plus **the corpus-derived shape
  heuristics implemented as deterministic analysis**, not model
  guesses — option tables recovered from `-*` dispatch loops and
  `Tcl_GetIndexFromObj`-style tables, closed value sets and integer
  domains from the consuming branches, `--` handling, n-paired tails
  from stride loops and parity checks, mutual-exclusion checks, mode
  words that select tails becoming subcommands, callbacks with their
  observed appended arity, `namespace ensemble -map` assembly,
  TclOO/snit classes into `object_class` + manufacturers, aliasing via
  `interp alias`/`rename`. Every mechanical conclusion carries its
  evidence line, exactly as today's importer does; what remains for
  the human (or the AI tab) is judgment — taint, effects, versions —
  not transcription. One Rust engine serves all three consumers: the
  studio's import tab, the spec-author skill, and a `tcl spec infer`
  CLI. When the user has configured AI access, the studio steers them
  to the skill path as the better one — the model runs on top of the
  same mechanical engine and adds the judgment layer (naming the
  ambiguous shapes, drafting hover prose, proposing taint and effect
  facts for review); the pure-mechanical pass remains the full-featured
  floor for everyone without a key.
- **A Test tab.** Paste code that uses built-ins *plus* the pack under
  edit. The embedded wasm already carries the real analyser and
  registry; the pack loads as an overlay, then the tab shows exactly
  what an editor would show — highlighted tokens, diagnostics, hover —
  and a deep-inspection view: click any word to see the resolved spec,
  the argument role and where it came from, type facts, taint colours,
  and which spec field produced each token. "My stuff is working" is
  observed, not asserted.
- **Navigation built for many commands.** `/` anywhere opens
  command search (the pack, the registry, and the Reference vocabulary
  alike, each hit saying which, keyboard-first); browser-style
  forward/back moves through the commands visited, with unsaved edits
  carried by the live-save layer rather than blocked by prompts. The
  pack's own commands live in an **always-visible sidebar** — one click
  from any command being defined to any other, with per-command state at
  a glance (fields set, warnings, done/draft) — so defining a library is
  constant motion between its commands, never a trip back through a
  search box. The workflow is explicitly *many commands to one
  deliverable*: build out the pack command by command, then emit the
  finished set as one DSL file, one rendered `.rs` batch, or one
  pre-filled GitHub issue.
- **Live save, always.** Every keystroke persists to browser storage
  (IndexedDB): pack sources, drafts, settings, the Test tab's sample
  code. Reload, crash, or restart resumes exactly; explicit export
  remains the way bytes leave the browser.
- **Skill-in-studio, fully integrated.** A tab to configure an
  OpenAI-compatible endpoint and API key, driving the complete
  spec-author flow in the page: the model gets the studio's own wasm
  functions (import/inference, command lookup, analyse, and
  `spectcl_check`'s report) as its tools, so the evidence loop is
  identical to the CLI skill and everything except the model calls
  stays client-side.
- **A real code editor, a real LSP.** The DSL and Test tabs embed
  Monaco (the editor VS Code uses) with **the LSP itself compiled to a
  wasm bundle** attached as its language server — so pack authoring in
  the browser gets the identical SpecTcl experience an editor gets:
  the self-spec pack's keywords, hover, completion, diagnostics, all
  from the same code, not a re-implementation.
- **Deployment: static documents, same-origin only.** The single-file
  constraint is dropped — the studio ships as a set of static
  documents (wasm bundles, JS, CSS, pages) served from one origin.
  The privacy guarantee restates for that shape: CSP permits requests
  **only to the origin serving the studio** (its own static assets) —
  never anywhere else — with exactly one exception, the user-configured
  OpenAI-style endpoint for the built-in skill, key in local storage
  and a plain warning that it is sent to the endpoint the user named.

## What a pack still cannot say

Behaviour that is not a pure words→data function stays native:
commands needing new lowering/codegen/analyser specialisations are
contribution candidates, as today. Private data stays private; new
*code* in the engine goes through review.
