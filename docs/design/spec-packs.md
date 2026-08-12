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
> **Not yet:** hook bodies do not run — every pack-declared hook installs as an
> abstaining function pointer, so the cache has no bytecode to hold and stores
> only the parsed statement tree. See "Hot-path budget" and phase 5 below.

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

**Where the migration half of that ambition stops, exactly.** The DSL
excludes `command_forms` and `subcommand_forms` — per-form bundles of
arity, roles, options, and hooks — on the grounds that `forms` covers
the getter/setter split every pack has needed and that a command needing
the structured version is deep enough in the compiler to be a
contribution. That exclusion is kept, but it has a consequence worth
naming rather than discovering later: **six shipped modules use those
fields today and therefore cannot round-trip through the DSL at all.**

| module | field |
|---|---|
| `commands/tcl/lset.rs` | `command_forms` |
| `commands/tcl/incr_.rs` | `command_forms` |
| `commands/tcl/package_.rs` | `subcommand_forms` (`vsatisfies`) |
| `commands/tcl/namespace_.rs` | `subcommand_forms` (`upvar`) |
| `commands/irules/http__cookie.rs` | `subcommand_forms` |
| `commands/irules/http2__stream.rs` | `subcommand_forms` |

Six of ~2,210 modules is not a v1 problem, and none of the six is a
command a private pack would want to redeclare. It is written down
because "shipped sources migrate to DSL" is stated above as an ambition,
and this is the precise, finite set for which it is false — and because
the same construct is what the `tls::socket` class of real third-party
library wants (one command whose positional count, option set, and
callback shape all change together with a mode flag). If that pressure
grows, un-excluding `command_forms` is the change to weigh, with these
six as the migration test.

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
The C-Tcl shim itself is later, separate work (#1372); the interface's
only obligation to it today is to not preclude it. The
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

- **`object_class` is ratified vocabulary the runtime loader does not
  implement.** It is in the frozen syntax and in the compiled-in `SpecTcl`
  self-spec (`commands/spectcl/blocks.rs`), and it appears nowhere in
  `tcl-spectcl/src/loader.rs` — so all four external drafts lose their
  handle-returning factories' method tables at load, with a notice each.
  Nothing under `specs/` uses the statement, which is how the gap survived
  the EDA migration. It is the largest remaining loader gap and is recorded,
  with its eleven notices, in the harness's baseline.
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
- A `speclib` version pragma gates hard breaks only — a word whose
  *meaning* changed, not one that was added. The server refuses a major
  it does not know and names the fix.
- The studio schema-coverage gates already force every new `CommandSpec`
  field to a named key; that key is the DSL property name, so the format
  cannot silently fall behind the registry.

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
  command search (registry and pack alike, keyboard-first); browser-style
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
