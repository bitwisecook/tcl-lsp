# Spec packs — a Tcl DSL for command databases

> **Status:** proposal under active design, for
> [issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363).
> Nothing here is implemented. Design-by-porting sketches live in
> [`spec-dsl-examples/`](spec-dsl-examples/) as they land.

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
  draft → `.rs`; the DSL maps to the same draft model). The most common
  libraries — tcllib, Tk, the core packs — stay **compiled in** exactly
  as today, for zero-cost loading in the general case. Over time their
  *sources* can migrate to DSL files that generate the Rust.
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

We have a full Tcl compiler; the authoring format should be Tcl. A
`.tclspec.tcl` file is a declarative script in a small spec DSL:

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
- It is the stub language grown up — same file culture (`*.tclspec.tcl`
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

**Two layers, deliberately.** The bottom layer is the **engine
interface**: compile a unit → handle, invoke a handle with structured
values → structured results — nothing DSL-specific in it. `tcl-vm` and
the Tcl→WASM codegen runtime are its two implementations (selecting one
must not require the other to be present), and it is the same surface
the C Tcl extension shim (#1372) adapts existing extensions to later —
one plug point for every way executable behaviour can arrive. The
**hook host** is the layer above: it owns everything DSL-specific — the
emitter verbs, per-family calling conventions and preconditions,
abstention and error policy, fuel budgets, memoisation — and speaks
only the engine interface beneath it. Engine-agnosticism is therefore
structural: the host cannot depend on an engine's identity, and a new
engine (or a shimmed C extension) slots in under the unchanged host.

Hot-path budget: role resolvers run per call site during semantic
tokens. Built-in packs keep native pointers (bucket 2 of the
architecture above), so only pack-declared commands pay the VM cost —
and those calls are memoisable by (command, word-shape). Whether that is
enough is a measurement question, currently being benchmarked (bytecode
compile cost per hook, ns/invocation VM vs native, memo hit-rate
sensitivity); the numbers land here when they exist.

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

- Discovery: `tclLsp.specPacks`, plus `*.tclspec.tcl` beside a
  `tclpkg.tcl` manifest or under `.tcl-lsp/`. Name collisions with
  shipped specs are reported, shipped wins unless the pack says
  `-override`.
- `tcl spec check lib.tclspec.tcl` validates a pack from the CLI; the
  Spec Studio gains a DSL renderer beside the `.rs` and stub renderers
  (drafts round-trip through it), and the `spec-author` skill emits the
  DSL for the private-library path.
- An optional compiled cache (`tcl spec build`) is a load-time
  optimisation only — never required, never version-locked beyond the
  DSL itself.

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
  stub panes, and a DSL *reader*: open an existing `.tclspec.tcl` (hand
  written, studio-saved, or emitted by the `spec-author` skill) and get
  the whole pack as editable drafts. The studio's unit of work becomes
  the **library**: a pack browser beside the registry browser, multi
  command editing, pack-level defaults and shared tables surfaced.
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
- **Skill-in-studio.** A tab to configure an OpenAI-compatible endpoint
  and API key, driving the spec-author flow in the page: the model gets
  the studio's own wasm functions (import/inference, command lookup,
  analyse) as its tools, so the evidence loop is identical to the CLI
  skill and everything except the model calls stays client-side.
  **Design tension, stated up front:** today's page ships
  `connect-src 'none'` — the never-phones-home guarantee. The AI tab
  requires network. Resolution: the default build keeps `'none'`; AI is
  a separately-built variant (or an explicit opt-in page) whose CSP
  allows only user-configured origins, with the key held in local
  storage and a plain warning that it is sent to the endpoint the user
  named. The guarantee is preserved by *build*, not by promise.

## What a pack still cannot say

Behaviour that is not a pure words→data function stays native:
commands needing new lowering/codegen/analyser specialisations are
contribution candidates, as today. Private data stays private; new
*code* in the engine goes through review.
