# Rust runtime port — productionising C-Tcl-extension-to-WASM

> **Based on `rust`@`8150eca`** (#549, the spike merge) — the commit this entire
> WASM runtime port is built on. Every Rust module mirrors its Zig/C sources
> *as of this hash*; the [upstream sync log](#upstream-sync-log-zig--rust) diffs
> `runtime/zig/` against it. **Update this hash (and re-baseline the sync log)
> only on a deliberate rebase onto a newer `rust`.**

Status: **bootstrapping.** The end-to-end mechanism (compile an unmodified C
Tcl extension to WASM and link it against our runtime + compiled user code,
API-not-ABI) is proven by the three throwaway spikes under
[`runtime/rust-spike/`](../../../runtime/rust-spike/README.md). The durable
contract is [`c-extension-abi.md`](c-extension-abi.md). This document is the
**source of truth** for turning that proof into a shipped capability, and it
must be kept current **every PR**.

> Modelled on [`docs/rust-rewrite.md`](../../rust-rewrite.md) — same SYNC-* /
> GAP-AUDIT-* sync discipline, same "component status + gate" tracking. Where
> `rust-rewrite.md` ports the *compiler/LSP* Python tree to Rust, this document
> ports the *WASM runtime* (`runtime/zig/`) to Rust and links C extensions into
> the AOT-first whole-program artifact. The two efforts share the `rust`
> branch's crate workspace but have disjoint component tables.

## North star

AOT-first, one artifact. The WASM AOT compiler
(`core/compiler/codegen/wasm/`) compiles as much Tcl as it can prove ahead of
time; the Rust-ported runtime (`runtime/rust/`) is the **support library +
interpreter fallback** for what can't be proven AOT-safe; C Tcl extensions are
linked in; and everything links into **one WASM+WASI artifact**.

**The deploy goal, concretely:** take the **runtime `.wasm`** + **any C Tcl
extension** (`.wasm` from `zig cc`) + the **compiler-output `.wasm`** for the
user's packages and files (AOT-compiled), and **link them into a single
runnable `.wasm`** the user executes under `wasmtime` (or any WASI host) — a
self-contained Tcl program. This is the whole-program static link (Model A,
`c-extension-abi.md` §5.1); `package require` at runtime is the dynamic Model B
loader. Track 3's `wasm_link.py` extension linking (T3.1) is the build step that
produces it.

The end state, stated as testable claims:

1. A program not heavily reliant on metaprogramming (no `eval`/`uplevel`/
   dynamic command or variable names) compiles **entirely AOT** and never
   enters the interpreter.
2. Common metaprogramming patterns are AOT-compiled by heuristics with a clean
   fall-through to runtime interpretation (a new staircase stage beyond S6 —
   see [Track 3](#track-3--aot-first-execution--whole-program-link)).
3. C Tcl extensions up to **sqlite3/tclsqlite** load and run, against the Rust
   runtime, via the production loader, with the surrounding script AOT-compiled.
4. The **C Tcl 9 test suite** (`tmp/tcl9.0.3/tests/*.test`) is the correctness
   gold standard; no file passing on the Zig baseline regresses.

The Zig runtime (`runtime/zig/`) stays the behavioural **oracle** until Rust
reaches parity. Do **not** regress the compiler/LSP or the Zig runtime.

## Design levers we own (and constraints we don't)

Two facts shape every representation choice; keep both in mind alongside the
[algorithm/data-structure method](#choosing-algorithms--data-structures-the-porting-method).

### We compile the extensions — transformation freedom (no source edits)

We own the whole **source → WASM** path for every extension: the authored
`tcl.h`/`tclOO.h`/`tclTomMath.h`, the C compiler (`zig cc`), the linker flags,
and any post-link WASM rewriting. So we may apply **any transformation that does
not require editing the extension's C source** — that is the one hard line
(`c-extension-abi.md` §1: "unmodified extension source"). This generalises the
API-not-ABI thesis and the [shim escape-hatch](#the-shim-escape-hatch--decouple-the-internal-rep-from-the-abi-view):

- The `Tcl_Obj` field layout is something **we impose** via the authored header,
  not something the extension dictates — so we choose it.
- Extensions only ever hold a `Tcl_Obj *` and touch the **declared** prefix
  fields (`refCount`/`bytes`/`length`/`typePtr`/`internalRep`) through macros;
  they never stack-allocate or array `Tcl_Obj` by value (they call
  `Tcl_NewObj`). So the runtime's actual allocation **may carry private trailing
  fields** the public header doesn't declare, or — the cleaner, Tcl-native route
  — keep the header at exactly 24 bytes and hang per-type data off a struct
  pointed to by `internalRep` (what T1.1 does). Both are open to us; pick per
  type on the evidence.
- Header-level, compile-flag-level, link-level, and post-link transforms are all
  fair game (instrumentation, relocation rewriting, calling-convention choices)
  as long as the `.c` stays untouched.

The corollary: a transformation that *would* require editing extension source is
out of bounds — that is exactly the line `c-extension-abi.md` draws, and the
tier gates (which vendor extensions byte-identical) enforce it.

### Bitness / pointer model — wasm32 addresses, native i64 values

WASM is **not** 64-bit native. The target the entire C-extension toolchain
supports (`wasm32-wasip1`; `zig cc` + wasi-libc, `wasm-ld --experimental-pic`,
`dylink.0`) is **wasm32**: linear-memory **addresses are 32-bit** (`i32`
pointers, 4 GiB cap), and `__indirect_function_table` indices are `i32`. The
`memory64` proposal exists (and Rust has an experimental `wasm64` target), but
its PIC / wasi-libc / dynamic-linking story is **not** viable for the
extension-loading path, so the **ABI is fixed at wasm32 i32 pointers**:
`Tcl_Size = i32`, `Tcl_Obj *` = i32, `Tcl_Obj` = 24 bytes (§4.2).

What this means for handles/pointers (answering "should we go 64-bit where we
can?"):

- **Addresses into linear memory must be 32-bit** — anything crossing the
  C-extension boundary (`Tcl_Obj *`, `char *`, table indices) is `i32`. We can't
  widen those without leaving wasm32, which the toolchain won't support.
- **Values can be 64-bit for free.** wasm32 has **native `i64`** locals and
  arithmetic. Tcl wide ints are already `i64`; the AOT codegen's unboxed
  fast-path values and any tagged-immediate / NaN-boxed value representation
  carried in WASM locals (not stored as a linear-memory address) can be 64-bit
  at no cost. So the rule is: **64-bit for values, 32-bit for addresses.**
- **Tagged / "packed" pointers work normally.** There is nothing special in
  WASM here — a pointer is just an `i32` in linear memory; the allocator's
  8-byte alignment frees the low 3 bits for tags (the Zig S6.4 tagged-immediate
  small-int trick, low-bit tag). On a hypothetical wasm64 future an `i64` handle
  would have even more tag room, but that is not today's target.
- **Native test build caveat.** On the host, `isize`/pointers are 64-bit, so the
  native `cargo test` build exercises the runtime *logic*, not the wasm32
  *layout*. Layout fidelity is asserted separately under
  `cfg(target_arch = "wasm32")` (a `size_of::<TclObj>() == 24` static assert
  lands with the wasm build, T1.6).

### The AOT compiler is ours to restructure (within the LSP guardrails)

We may make **any architectural change to the AOT compiler**
(`core/compiler/codegen/wasm/`, the IR/lowering it owns, the codegen↔runtime
ABI) to make this port easier — **provided two guardrails hold**:

1. **No loss of LSP precision/accuracy.** The frontend the LSP shares (lexer,
   parser, analyser, the registry/spec data, diagnostics) must keep producing
   the same results. Restructure the *backend* (lowering → WASM, the emit path,
   the runtime ABI it targets) freely; treat the shared frontend as the line.
2. **No tremendous LSP performance cost.** Changes must not materially slow the
   LSP hot paths (parse/analyse/respond).

This is the latitude behind T3.0 (the backend-agnostic emit registry) and the
codegen↔runtime ABI contract (Zig lesson #4): the emit path and the runtime ABI
can be redesigned to align cleanly with the runtime — e.g. one canonical
parser→component-model→evaluator contract the codegen lowers from and the
runtime walks, an explicit O()-annotated runtime ABI the codegen targets, and a
single source of truth for runtime imports — as long as the LSP-facing frontend
stays precise and fast. (This sits alongside the standing rule: do **not**
regress the compiler/LSP *behaviour* or the Zig runtime; this note clarifies
that *backend architecture* is nonetheless free to change.)

## Core tenet — low algorithmic complexity from the start

**Every operation gets a target asymptotic complexity, decided when it is
designed — not retrofitted after a hang.** This is a first-class design tenet,
not a later optimisation pass, because the recurring failure mode of the Zig
runtime was *hidden* super-linear complexity (see the lessons below): a list
that re-parsed its string made `lindex`/`lappend` O(n) each → O(n²) loops; a
dict without a real index hung (`dict-24.24`); the codegen↔runtime seam called
an O(idx) helper per element inside an inlined loop → O(n²) invisible to both
halves.

Concretely, from day one:

- **Name a complexity for each op** in the data-structure method's step 1, and
  pick the representation that achieves it (the list = contiguous `Vec` for O(1)
  `lindex`; the dict = ordered `Vec` + hash index for O(1) by-key + O(n) ordered
  iterate, chosen by experiment).
- **No hidden re-parse / re-shimmer on the hot path.** A typed value keeps its
  intrep; the eval loop passes **objects** through (the object-passthrough fast
  path) rather than stringifying-and-re-shimmering — so `$list` → `lindex` stays
  O(1), not O(n) per access.
- **Iterate by cursor/snapshot, never indexed re-access** across the
  codegen↔runtime seam (lesson #4): `foreach`/`dict for` walk elements once, not
  `lindex i` in a loop.
- **No fixed-size buffers that silently truncate** (lesson #5): grow to the heap
  or raise — never cap (`{*}`/args/list builders). A `Vec` that grows, not a
  `[T; 128]`.
- **Build cliffs are correctness bugs, not perf nits** — an O(n²) build loop
  (`dict set` in a loop, `append`/`lappend` without capacity) hangs on real
  inputs; treat the asymptotics as part of "correct".

When the right complexity needs an empirical call (constant factors, crossover),
that is exactly what the [WASM experiments](#choosing-algorithms--data-structures-the-porting-method)
settle — but the *target* is set up front.

## Lessons from the Zig runtime — the four contracts

The Zig runtime (~82k lines) works and passes a wide upstream slice, but it grew
bottom-up and its hard bugs trace back to **four contracts left implicit**. This
port nails all four **on day one**; the table records the Zig pain point, the
lesson, and where the Rust port stands.

| # | Zig pain point | Lesson | Rust port status |
|---|---|---|---|
| a | One string-primary 32-byte `TclObj` with int/float/inline-string/dict-cache/bignum crammed into one shared slot; lists re-parsed their string (O(n²)); ad-hoc shimmer | **Dual-ported obj from day one**: typed intrep with its *own* pointer + lazy cached string rep; shimmer first-class; lists as an element vector | **✅ done.** `#[repr(C)] TclObj` + per-type backing hung off `internalRep` (own pointer, not a shared slot); list = `Vec<*mut TclObj>`; dict = ordered `Vec`+index; shimmer via the `typePtr` free/dup/update-string procs (the keystone) |
| b | Refcounts + slab allocator + deferred-free queue + slab recycling + parse-cache-keyed-on-raw-pointers; ownership hand-tracked per call site (leaks) | **One ownership discipline**, explicit at every ABI boundary | **✅ done.** Refcount + the `fresh_zero`/`borrowed`/`owned` contract ([`c-api-ownership-contract.md`](c-api-ownership-contract.md)), leak-checked every test. **No** deferred-free queue, **no** slab recycler, **no** parse-cache-on-pointers (the parser borrows `&[u8]`, so that stale-slab bug is a *compile error*) |
| c | i32 handles vs u32 addresses (`@intCast` panic past 2 GB), tagged-immediate low-bit overloads, `ALIAS_GLOBAL`/`ALIAS_EXT` negative sentinels | **One handle ABI** up front; never `@intCast` | **✅ sidestepped.** Real `*mut TclObj` pointers (no i32-handle/address split → that panic class can't exist); `Var::Link` enum, not sentinels. Tagged immediates deferred; **when added, define the one ABI** (tag bits / width / sentinel space) in a doc first |
| d | Codegen inlines control flow but calls an O(idx) runtime helper per element → O(n²) hidden in the seam; `_scan.py` a second source of truth for imports | **Runtime ABI is an explicit contract** the codegen targets (which ops are O(1), which take obj handles); iterate by cursor, never indexed re-access; one import source | **⚠️ in progress.** Object-passthrough (this chunk) keeps `$list`→`lindex` O(1); the full codegen↔runtime ABI contract + the "one import source of truth" land with T1.7 / T3.0 (the emit registry). Tracked as a tenet above |

Other Zig lessons folded into the plan: error/traceback via a **real call-frame
stack carrying source spans** (not after-the-fact synthesis) →
[`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md); a **uniform
variable-cell + explicit alias indirection + defined trace re-entrancy** →
`frame.rs` `Var::Link` (T1.3) + the trace model (later — its precise error/ignore/
stop/LIFO contract is captured in
[the scope/trace/info lessons](#lessons-from-the-tcl-9-wasm-correctness-campaign--scope--trace--info)
below); **byte-exact C-Tcl
compatibility as the contract**, with the incompatible-by-design set decided up
front → the [Tcl 9 scoreboard exclusions](#out-of-scope-exclusions-by-design);
**no silent truncation** → the O() tenet above.

## Lessons from the Tcl 9 WASM correctness campaign — scope / trace / info

A second, independent body of hard-won contracts from the Tcl 9 WASM campaign
(the `foreach`-qualified-var, variable-trace error-wrapping, unset-trace-ignore,
and `info exists`-after-`unset` bugs). These are **authoritative v2 design
inputs** — like the four contracts above, they are recorded here so the port is
built against them, not retrofitted. The reference is the C source
(`tclTrace.c::TclCallVarTraces` — the `done:` label — and `tclVar.c`), **not
intuition**; the WASM dispatcher mirrors its control-flow shape.

**Scope / global / namespaces** (extends [meta-system 1](#meta-system-1--resolution--frames-upleveluplevelglobalvariablenamespaces);
these are mostly **AOT-compiler** contracts — ours to restructure):

- **S1 — scope class is a first-class lowering input, not an afterthought.** A
  name's class (frame-local / qualified-namespace / global) decides codegen.
  Make it an **explicit field on the IR op**, and make "must run via the
  interpreter" an explicit lowering **outcome** — never an emergent side effect
  of swapping the IR node type (a stale registry flag silently ate exactly that
  decision: a `::`-qualified loop var that must drop to a generic invoke).
- **S2 — the "emits-nothing" footgun.** A per-*command-name* `wasm_emits_nothing`
  flag is a footgun: the same spelling can be both a structural no-op marker
  (`foreach`'s synthetic loop-header def) **and** a real opaque call (the
  qualified-var eval-fallback). The global flag swallowed the real invoke → the
  loop ran zero iterations. v2: attach emits-nothing to the **specific synthetic
  node instance**, never to the command name in a shared registry.
- **S3 — eval-fallback must be token-faithful.** When a construct is handed back
  to the interpreter, carry the **original command tokens** so braces/quoting on
  varLists, lists, and bodies round-trip exactly. Reconstructing from
  brace-stripped IR strings pre-substitutes `\n`, mis-parses a leading `[`, and
  corrupts bodies. v2: thread `tokens` **uniformly** on every IR node that can
  fall back (`foreach`/`lmap`/`catch`/`while`/`for`/`switch`). (Ties to
  meta-system 2's parse-once + the contract's "tokens are the eval-fallback
  payload"; the runtime port's borrowed-`&[u8]` `Command`/`WordPart` model
  already carries source spans for exactly this.)
- **S4 — unqualified resolution rule, encoded in the compiler too.** Unqualified
  ≠ namespace var inside a proc; unqualified = global at script top level. The
  cell/frame model states this — the compiler's local-tracking must encode the
  same rule and route `set X`/`$X` through the frame name table **unless it has
  proven** the local is promotion-safe (no `upvar`/`global`/`variable`/`array`/
  trace/`eval` reach).

**Info / trace — introspection & re-entrant interrupts** (a subsystem in its own
right; the trace model the Zig-lessons table deferred):

- **T-INFO — introspection reads LIVE runtime state, never compile-time-folded
  assumptions.** `set x 1; unset x; info exists x` returned `1` because the
  compiled `info exists` was answered from "local-was-assigned" and never
  invalidated by `unset` (the variable *was* gone — a read errored correctly —
  only the introspection lied). Any const/local-liveness tracking the compiler
  keeps **must be invalidated** by `unset`, `upvar`, `global`, `variable`,
  `trace add`, and **every** `eval`/`uplevel`/dispatch boundary. Treat `info
  exists`/`vars`/`locals`/`level`, `trace info`, `array exists` as **runtime
  queries against the cell table**, not foldable pure functions. (Reinforces
  [meta-system 1](#meta-system-1--resolution--frames-upleveluplevelglobalvariablenamespaces)'s
  "introspection over the cell model" and the contract's invalidation set.)
- **T-FIRE — variable traces are re-entrant interrupts with a precise error
  contract; the firing site must PROPAGATE and SHAPE the callback's result
  code** (evaluating the callback and discarding its code is the bug):
  - read-trace error → result becomes `can't read "NAME": <msg>`
  - write-trace error → result becomes `can't set "NAME": <msg>` (verb `set`,
    `errorInfo` type `write`)
  - **NAME is the user-facing name** — `arr(key)` for an element even when the
    trace was installed on the whole array (the *accessed element's* name is
    carried through, not the matched key)
  - **unset-trace error is IGNORED**: the unset still succeeds, the pre-trace
    interp state is restored, and the **remaining** unset traces still fire
  - read/write trace error **STOPS** firing further traces (`break`); unset does
    not. Fire order is **newest-first (LIFO)**; **whole-array traces fire before
    element traces**.
  - v2: model trace firing as **one shared dispatcher** taking `(op, name,
    verb-for-errors, leave-err-msg?)` that centralises the wrap/ignore/stop
    policy — every read/write/unset/array path funnels through it (mirrors
    `TclCallVarTraces` + `SaveInterpState`/`RestoreInterpState`).
- **T-COMMIT — the actual operation completes independently of trace outcome
  where C says so.** The variable is removed during `unset` teardown
  before/regardless of unset-trace errors; the stored value is committed before
  write traces run. **Don't gate the mutation on the trace's success.**

**Cross-cutting:** error strings and stack traces **are** the contract (tested
verbatim — match C's verb, quoting, and the `(read trace on "x")` `errorInfo`
frame, not an approximation); internal rep / refcounts / bytecode line tables are
incompatible-by-design (the W9-internal set →
[exclusions](#out-of-scope-exclusions-by-design)). These land with the **proc /
frame / trace** chunks ([`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md)),
written **against the C control flow**.

### Command binding & aliasing — the command-layer parallel

The authority is the PR #554 contracts (on `claude/great-euler-2VtXp`, merging
to `main`): [`command-binding-and-aliasing.md`](../contracts/command-binding-and-aliasing.md),
[`variable-trace-dispatch-and-introspection.md`](../contracts/variable-trace-dispatch-and-introspection.md),
[`compiled-scope-and-name-lowering.md`](../contracts/compiled-scope-and-name-lowering.md)
— the as-built companions to the scope/trace lessons above. Every form of
command-name indirection is **the variable cell/frame model one layer up**: a
name resolves to a target through a chain that is mutable at runtime, often
mid-eval. The port-facing directives:

- **A1 — one resolver.** Exactly one way to reach a command:
  `resolve(currentNs, name) → target`, evaluated against the command tables **at
  the moment of the call** — never memoised across an eval/trace/sourced-file
  boundary. `rename`, `interp alias`, `namespace import`/`path`, ensembles, and
  a `proc` shadowing a builtin are all redirects *through this one function*.
- **A2 — the resolution order (exactly this, every call).** Parse the name:
  leading/embedded `::` ⇒ qualified (absolute or ns-relative) → look up directly
  in that namespace (no path/import fallback). Unqualified → **(a)** current ns
  table, **(b)** each ns on the current ns's `namespace path` in order, **(c)**
  global `::`, **(d)** `unknown` (auto-load / ensemble-unknown / `invalid command
  name "x"`). This is why `namespace path ::tcl::mathop; + 1 2` finds bare `+`.
- **A3 — per-form contracts** (verbatim error strings are the contract):
  `rename old ""` deletes + splices the command out of every importer's redirect
  list (built-ins `return`/`error` protected — `can't rename "X": …`); `interp
  alias` re-resolves its target **by name on each dispatch, anchored at global**,
  with frozen prefix args — sees target *deletion* lazily, does **not** follow
  *rename*, and is **not** unwrapped by lookup; `namespace import` installs a
  **transparent** (`CMD_IMPORTED`) redirect that lookups unwrap, a **snapshot** at
  import time (later additions not retroactively imported); `namespace path` is
  pure search fallback (no redirect); ensembles map `ens sub` → target
  (default `::ens::sub` or `-map`, unambiguous-prefix unless `-prefix 0`) —
  **the `dict for`→`::tcl::dict::for` rewrite IS the ensemble alias, generalise
  it**. `::tcl::mathop`/`::tcl::mathfunc` are real commands; **do not conflate
  `expr`'s internal op dispatch with the command path** — but `::tcl::mathfunc::X`
  *is* overridable and `expr`'s function-call path resolves it through the
  command table (model that one hook).
- **A4 — the AOT binding lattice (the guard).** `canonical_command` is a
  *lowering-time snapshot* of `resolve`, sound only while the binding can't have
  changed by call time. Track a **binding state per name** (`pristine-builtin →
  user-proc → aliased → renamed-away → shadowed`); **only `pristine-builtin` may
  inline** — any rebinding op demotes the name to a live-table dispatch (an
  interpreter **barrier**, never an inlined builtin). This is the command-layer
  analogue of T-INFO's liveness invalidation: rebinding ops epoch-invalidate the
  resolution caches **coarsely** (wipe the LRU on any rename — coarse-but-correct
  beats clever). Keep **both spellings** (issue #246): match patterns on the
  canonical form, but retain the *source* spelling for eval-fallback (a bare name
  resolves through the live scope walk; an eagerly-globalised `::name` misses a
  namespace-local proc — ties to S3's token-faithful fallback).
- **A5 — aliasing ≍ traces.** Both are re-entrant interrupts that invalidate
  compile-time assumptions and must funnel through one resolver/dispatcher;
  resolve by-name per call (lazy) so chains/cycles can't loop at creation.
  Internal `Command` layout / redirect-list pointers / LRU contents are
  incompatible-by-design (object-rep probes never match).

These land with the **namespace + command-table + proc** chunks (T1.5/proc),
built against the C dispatch (`tclNamesp.c`/`tclBasic.c`).

## Deep Tcl semantics — the design contracts (decide before commands)

Tcl is **homoiconic and late-bound**: `eval`/`uplevel`/`subst`/`source`/`apply`/
runtime-built proc bodies / `$dynamic_cmd` / `{*}$computed` all mean *the code to
run does not exist at compile time*. So the AOT compiler is **always half of a
pair** — it must ship a complete runtime parser+evaluator that is byte-for-byte
identical to the compiled path. The Zig runtime's hard semantic bugs all came
from a deep subsystem being designed bottom-up instead of as a contract.

> **The authority is the three day-one contracts** in `docs/design/contracts/`
> (from PR #551, written first-principles from the Zig experience):
> [`runtime-variable-frame-model.md`](../contracts/runtime-variable-frame-model.md),
> [`parser-and-aot-interpret-boundary.md`](../contracts/parser-and-aot-interpret-boundary.md),
> [`numeric-tower-and-expr-semantics.md`](../contracts/numeric-tower-and-expr-semantics.md).
> Each marks **Contract** vs **incompatible-by-design** behaviours. This section
> does **not** re-derive them — it maps the **Rust port's status** against the
> **three meta-systems** they define. (Until #551 lands on `rust`, those links
> point forward; see the sync log.)

### Meta-system 1 — resolution + frames (`uplevel`/`upvar`/`global`/`variable`/namespaces)

> Contract: [`runtime-variable-frame-model.md`](../contracts/runtime-variable-frame-model.md)
> (frame → name → cell indirection; resolution algorithm; alias/trace/cycle rules).

`uplevel`/`upvar`/`global`/`variable` are all facets of **one variable model**,
and namespace command/var resolution is the same problem one level up.

- **Two stacks** — the *call* stack (`info level`) and the *var* frame (where
  `set` looks); `uplevel N` runs a script in a caller's var frame, `upvar` links
  a local **name** to a cell in another frame/namespace. → designed in
  [`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md) (the
  `caller`/`caller_var` split).
- **One variable-cell layer**: a frame is a name→cell table; a cell is
  refcounted and **may be an alias** (points at another cell); **traces hang off
  cells**; every var op goes through it; non-aliased hot locals are optimised
  later behind a guard. → **partial**: `frame.rs` has `Var::Scalar|Array|Link`
  (the alias) with path resolution (T1.3); the single resolution order
  (qualified → frame-local-in-proc → current/global namespace, then link walk)
  is **done** as one classification + cross-table walk over a `VarHome`
  (`vars.rs`, T1.5). **Gaps to design in**: independent *cell* refcounting (Tcl
  `VarInHash`) and the **trace** hook on cells + re-entrancy/ordering model.
  These are designed before traces land, **not appended**.
- **Namespaces**: hierarchical `::a::b` with own var+command tables, `namespace
  path`, `import`/`rename`, `which`/`origin`, **ensembles**, `namespace
  eval/code/inscope` (capture/restore current-ns). Command resolution =
  current-ns → `namespace path` → global, modified by import/rename, then the
  **`unknown` handler**; var resolution is the parallel algorithm. → **T1.5**,
  built as a *core service* (the two resolution algorithms + `unknown` +
  ensembles as first-class dispatch) **before** the bulk of commands. The flat
  global table used by the list/dict commands so far is the degenerate case.

### Meta-system 2 — parse-once + the AOT/interpreter duality (`eval`/`source`/`package`)

> Contract: [`parser-and-aot-interpret-boundary.md`](../contracts/parser-and-aot-interpret-boundary.md)
> (one canonical grammar; the compile-vs-interpret disposition table; the
> compiled ≡ interpreted identity contract; `source`/`package` behind a VFS).

- **One canonical scanner → two clients (LSP CST + runtime eval tree).**
  "Parse once" means **one lexer**, not one lexer per consumer. The workspace
  already has the canonical Rust scanner — **`rust/tcl-lexer`** (lexer +
  `expr_lexer` + `substitution` + spans; `TokenType` = `Esc`/`Str`/`Cmd`/`Var`/
  `Sep`/`Eol`/`Comment`/`Expand`, i.e. exactly the runtime's needs incl. `{*}`
  and comments) — used by the LSP/compiler (which builds a green/red CST over
  it, `tcl-compiler::parsing`). The runtime's `parse.rs` currently
  **re-implements** that scanning, which is the contract's "N scanners → N
  drifts" — and it already cost two real bugs (the `{*}`-prefix and braced
  `\<newline>` hard edges). **Decision: the runtime converges onto `tcl-lexer`**
  (verified wasm-buildable — `thiserror`-only) and lowers its tokens into the
  eval `WordPart`/`Command` tree, dropping `parse.rs`'s scanners + `bs.rs`'s
  backslash (which `substitution::backslash_subst` already provides). The hard
  edges then come correct for free. `parse.rs`/`bs.rs` are **interim**; see
  [Reuse the compiler/analyser suite](#reuse-the-compileranalyser-suite-survey-before-building).
  (`eval` + `subst` + list parsing already share the one runtime scanner today;
  the convergence makes that scanner the *same* one the LSP uses.)
- **"Fall back to interp" is a defined boundary** that `eval`/`uplevel`/`subst`/
  `source`/`apply`/dynamic-name all funnel through, with **source spans threaded
  from the first byte** so `info frame`/line numbers/`errorInfo` survive across
  eval/source. → designed in `proc-call-and-stack-traces.md` (the `CmdFrame`
  source stack); the S7 staircase stage compiles the static metaprogramming
  cases, everything else funnels to the interpreter.
- **`source`/`package`/auto-load** sit behind a **VFS + loader interface** so
  WASI being absent is a *missing impl*, not a missing design. → noted; the
  loader is Track 2 / a VFS shim; `source` threads `info script` + relative-path
  resolution through the `Source` frame kind.

### Meta-system 3 — the numeric tower + `expr` (a second language)

> Contract: [`numeric-tower-and-expr-semantics.md`](../contracts/numeric-tower-and-expr-semantics.md)
> (the small→wide→bignum→double tower with one promote/normalise/compare;
> `expr`'s grammar/precedence/operators; the braced-compile/unbraced-interpret
> split; verbatim numeric error wording).

- **One numeric tower** — tagged-small-int → `i64` → **bignum** → `double` —
  with **one** promotion/demotion/normalisation path and **one** compare/equality
  used by `expr` and every numeric command; **canonicalise on every op** (a
  bignum that fits a wide demotes back, so equality/hashing/string-rep are
  stable); **no command rolls its own int parse**; **integer overflow promotes
  to bignum, never wraps**. → **gap, flagged now**: today `obj` has `i64` int +
  `double` but **no bignum**, and `incr` had its own `parse_i64` + `wrapping_add`
  (the two anti-patterns this warns of). Fixed the silently-wrong wrap (→ checked
  + explicit error pending bignum); the **numeric-tower module** (one parse, one
  promote/normalise, one compare, bignum via a ported `tclTomMath`) is a
  dedicated chunk **before `expr`**, and every numeric command routes through it.
- **`expr` is a separate language** — own grammar/precedence (`**` right-assoc),
  own operators (`eq`/`ne`/`in`/`ni`, ternary, short-circuit), own numeric rules
  (int `/` truncates, `%` follows divisor sign, numeric-vs-string `==`,
  `0x`/`0o`/`0b`/`1_000` literals, NaN/Inf, locale-free); functions dispatch
  through the overridable `tcl::mathfunc::*` namespace (ties to meta-system 1);
  only braced `expr {...}` is safe/compilable, unbraced double-substitutes. →
  its own lexer+parser+evaluator over the numeric tower, mathfunc through the
  command table; **braced compiles to guarded native ops, unbraced interprets**.
  A dedicated chunk after the numeric tower.

### Cross-cutting contracts

- **Parse once, canonically** — one scanner (done, above).
- **Traces are re-entrant interrupts** on ordinary var/command ops (fire during
  access/dispatch, through upvar/namespace links, can error and re-enter). →
  define dispatch order + re-entrancy + error propagation **before commands rely
  on them** (with meta-system 1's cell model).
- **The result is not a string** — a return code + an options dict
  (`-errorinfo`/`-errorcode`/`-level`/`-errorstack`); `catch`/`try`/`return
  -options`/`error` manipulate it. → designed (`Code` + the `ExceptionState`/
  options model in `proc-call-and-stack-traces.md`); the universal return type
  from day one.
- **Encoding — UTF-8 is THE internal string rep.** Decision: strings are stored
  **UTF-8** (the obj's `bytes`); `encoding convertto/convertfrom` and channel
  encodings **translate to/from UTF-8 at the boundary**. Rationale: the vast
  majority of WASM use cases are UTF-8 already, and a single internal rep avoids
  the dual UTF-8/UTF-16 cache complexity Tcl 9 carries; `string`/`regexp`/
  `binary` operate on the UTF-8 bytes (with the EXP-STRING ASCII fast path for
  char indexing). Non-UTF-8 codecs are an *edge translation* (deferred-WASI), not
  an internal-rep concern.

## Reuse the compiler/analyser suite (survey before building)

**Core tenet — share with the compiler as a shared crate wherever we can.**
Before building any subsystem in the runtime, survey the existing Rust
compiler/analyser suite (`rust/`) for a component to reuse, and when both sides
need the same logic, **factor it into a shared crate** (`tcl-lexer` /
`tcl-syntax`) rather than re-deriving it. Much of what the runtime needs (lexing,
expr parsing **and evaluation**, command metadata, name resolution, shimmer
rules) is already implemented, tested, and LSP-precise there. Reimplementing it
in the runtime is the contract's "N implementations → N drifts" — the exact
failure mode behind the parser bugs found above. The aim (per "the AOT compiler
is ours to restructure, within the LSP guardrails") is **clean shared crates
consumed by both the LSP/compiler and the runtime.**

**The sharing pattern, when the value type differs.** Where consumers need the
same *logic* over different *value types* (the runtime's `Tcl_Obj`+tower vs the
compiler's const-fold `TclValue`), share the logic as a **generic over a trait**,
not a copy. The `expr` evaluator is the worked example:
`tcl_syntax::expr::eval<O: ExprOps>` owns the **walk** — operator dispatch,
short-circuit `&&`/`||`, `?:`, the numeric-vs-string comparison rule,
`eq`/`ne`/`in`/`ni` — once, and each consumer implements `ExprOps` with only its
value ops (the runtime over the tower via `bignum`; the compiler's const-folder
bails where it can't model the tower). This is the evaluation parallel of sharing
the lexer/parser/AST. Same shape applies to future shared semantics
(name-resolution, shimmer): one algorithm, a trait for the value/store, two
impls.

**Convergence status — ✅ done (both directions).** The runtime *and* the
compiler's const-folder now drive the **one** shared `eval<ExprOps>` walk; the
entire expression system (lexer → AST → parser → evaluation walk) is shared,
with only the value type differing. Landed:
- `parse_literal` → `tcl_syntax::number`; the shared `ExprOps::binary_other` hook
  (default = unsupported) so the compiler keeps its **iRules dialect ops**.
- `tcl_expr_eval` (`tcl-compiler`) is now a `FoldOps: ExprOps` impl over
  **`FoldValue { Int(i64), Float(f64), Str(raw_text) }`** — `Str` keeps literals'
  raw text and parses lazily per-context (numeric ops via `parse_literal`; string
  ops use the raw text), reproducing the old `eval`-vs-`eval_as_string` split, so
  the raw-text string compares (`5.00 eq 5.0` → 0, #519) are preserved. The four
  walk functions (`eval`/`eval_binary`/`eval_unary`/`eval_call` + `eval_as_string`/
  `apply_string_compare`/`resolve_var`) were **deleted**; `FoldOps` reuses the
  kept operator helpers (`apply_binary`/`dispatch_math`/`apply_irules_string_op`).
  `eval_tcl_expr` maps `FoldValue` → `TclValue` at the boundary, so the 8
  optimiser consumers are unchanged. **All 2343+ `tcl-compiler` tests pass**
  (gated by `cargo test`, not the Python bytecode-compare).
- **Math functions + double formatting single-sourced.** `tcl_syntax::expr::mathfunc`
  (a `dispatch(name, &[Num])` over a shared `Num{Int,Float}`) replaces the
  compiler's `dispatch_math` + 4 helpers; the compiler maps `TclValue`↔`Num`, the
  runtime maps `Tcl_Obj`↔`Num` (via `bignum::as_math_num`, bignum→double as in
  C Tcl) — so `expr` math functions work end-to-end in the runtime too.
  `tcl_syntax::number::format_double` is the one canonical double→string
  (integer-valued → `.0`, `Inf`/`NaN`), used by the runtime's `double` rep **and**
  the compiler's `format_tcl_value`.
- **✅ `::tcl::mathfunc::*` / `::tcl::mathop::*` as real commands** (T1.5,
  registry-backed). `cmd_mathfunc.rs` registers one builtin per function name
  (each forwarding to the shared `tcl_syntax::expr::mathfunc::dispatch`);
  `expr`'s function-call path resolves `::tcl::mathfunc::NAME` through the
  command table first (absolutely anchored), so a user override / `rename`
  wins (the [A3 contract](#command-binding--aliasing--the-command-layer-parallel):
  "model that one hook") — `expr`'s `call` hook now goes through
  `ExprCtx::call_function`, falling back to the shared dispatch only in the
  standalone evaluator. `cmd_mathop.rs` registers every operator
  (`~ ! + - * / % ** & | ^ << >> == != < <= > >= eq ne in ni`) with the
  variadic-fold / identity / chained-comparison / arity semantics over the same
  tower ops; per A3 these are **commands only** — `expr`'s inline operator
  dispatch (`arith`) is unchanged. Both tower-gated. **Remaining:** `rand`/
  `srand` (need interp RNG state). The lexer's `math_functions()` name set stays
  the lexable list (it is a lower crate than `tcl-syntax`).

### Deep survey (four sweeps) + the `tcl-syntax` decision

Before convergence step 3, a four-way survey ran: the **compiler/LSP suite** and
**reference C Tcl 9.0** were each swept for (a) `subst`/list/backslash and (b)
`expr`/`format`/`scan`/numeric-tower. Findings (paths are `rust/…`):

| Primitive | Already in the compiler suite | C-Tcl ground truth | Verdict |
|---|---|---|---|
| **Backslash decode** | `tcl-lexer::substitution::backslash_subst(&str)->Cow` — full table incl. `\U`+surrogate pairs, `\<nl>` collapse; public, wasm-clean | `TclParseBackslash` (`tclParse.c:783`) | **canonical exists** — reuse; retire `runtime/src/bs.rs` |
| **List split** | THREE copies: `tcl-registry::const_fold::split_list` (validating, bails on `\`), `tcl-compiler::codegen::helpers::split_list_simple` (best-effort), `runtime::parse::split_list` | `Tcl_SplitList`/`TclFindElement`/`FindElement` (`tclUtil.c:522/577`) with the **`literal` zero-copy flag** + `TclCopyAndCollapse` | **consolidate** to one canonical splitter |
| **List merge/quote** | `tcl-compiler::codegen::helpers::tcl_list_element` | `Tcl_ScanElement`/`Tcl_ConvertElement` + `ConvertFlags` (`tclUtil.c:1056/1420`) | consolidate (the inverse) |
| **Subst decomposition** | `tcl-compiler::codegen::helpers::parse_subst_template`, `subst_nocommands` | `TclSubstParse`/`TclSubstTokens` (`tclParse.c:1902/2098`), `TCL_SUBST_*` flags, the `TCL_TOKEN_{TEXT,BS,COMMAND,VARIABLE}` taxonomy | converge (runtime's `subst.rs` + the literal-passthrough rules for invalid `$`/`[`) |
| **Expr lexer** | `tcl-lexer::expr_lexer::tokenise_expr` — full number forms, 31 mathfuncs, `eq/ne/in/ni`, iRules dialect | `ParseLexeme` + lexeme/precedence tables (`tclCompExpr.c:1907/154`) | **reuse** (already in tcl-lexer) |
| **Expr AST + Pratt parser** | `tcl-compiler::{expr_ast,expr_parser}` — `ExprNode`, 38 binops w/ Tcl-9 precedence, right-assoc `**`, `parse_expr`, graceful `Raw` | `ParseExpr`/`OpNode`/`prec[]` (`tclCompExpr.c:544/294`) | **extract** to the shared crate |
| **Expr evaluator** | `tcl-compiler::tcl_expr_eval` — i64/f64 **const-fold** only (no bignum), C-9-faithful floor-div/mod/`**`/round, `dispatch_math` | bytecode `INST_*` over the **int→wide→bignum→double tower** (`tclExecute.c`) | AST/parser shared; the **tower-aware evaluator stays runtime-side** |
| **Numeric grammar** | inside `tcl_expr_eval::parse_literal` (+ version-aware wrappers in `format_`) | `TclParseNumber` state machine (`tclStrToD.c:377`): `0x/0o/0b`, underscores, `Inf/NaN`, the tower | **extract** the grammar to the shared crate; both const-fold and the runtime tower build on it |
| **Format/scan** | `tcl-registry::commands::tcl::{format_,scan_}` — spec parser + version-aware int width/octal; `binary format/scan` **absent everywhere** | `Tcl_AppendFormatToObj` (`tclStringObj.c:1834`), `Tcl_ScanObjCmd` (`tclScan.c`) | **extract** the spec parser; `binary` is net-new |
| Command metadata | `tcl-registry` (spec/arity/forms/arg-roles/const-fold/side-effects) | — | **already a crate** — runtime command table + T3.0 emit registry bind to it |
| Name/var resolution | `tcl-compiler::{var_resolve,var_scoping,var_refs,var_observability}` | `tclVar.c`/`tclNamesp.c` | extract later (`tcl-resolve`), meta-system 1 |

**Decision (chosen): extract one shared leaf crate `tcl-syntax`.** It path-deps
only `tcl-lexer` (so it stays `wasm32`-clean) and holds the pure, parse-tree +
byte-exact-semantics layer: list split/merge, subst token model, the expr
AST + Pratt parser, the `TclParseNumber` numeric grammar, and the format/scan
spec parser. Dependency DAG (no cycles):

```
tcl-lexer  ←  tcl-syntax  ←  { tcl-registry, tcl-compiler, runtime/rust }
```

`tcl-lexer` stays the **scanner** (command lexer + `expr_lexer` + `backslash_subst`).
`tcl-compiler` keeps its IR/passes/codegen and its **const-fold evaluator**
(which becomes a thin consumer of `tcl-syntax`'s AST/grammar); the runtime adds
its **tower-aware evaluator** over the same AST. Two things are consumer-specific
by necessity: the **value type** (compiler folds i64/f64; runtime needs the
bignum tower) and **codegen emit**. The `&str`↔`&[u8]` boundary resolves on the
UTF-8-internal-rep invariant: `tcl-syntax` is `&str`-based (Tcl is UTF-8), and
the runtime converts at the call (it already guarantees UTF-8 internally) — so no
backslash/list/expr logic is duplicated for bytes.

**Phased extraction** (each phase its own gated commit; the workspace
`cargo test` + the LSP gates **must** stay green; `runtime-rust-test`/`-lint`
gate the runtime side):

1. ✅ Create `tcl-syntax`; **list** module (canonical `FindElement` w/ `literal`
   flag + split + `ScanElement`/`ConvertElement` merge), reusing
   `backslash_subst`. Wired the runtime's `split_list` to it.
2. ✅ **backslash/subst**: added `tcl_syntax::backslash` (re-exports the canonical
   `backslash_subst` + a `decode_bytes`); **retired `runtime/src/bs.rs`** (a
   second decoder, buggy on `\xff` → invalid UTF-8). Dropped `WordPart::Backslash`
   — escapes fold into the `Text` run, decoded once (`subst.rs`/`interp.rs`
   converged; one decoder, not two). Full subst-token-model + `TCL_SUBST_*`
   convergence onto a shared `tcl-syntax::subst` is the remaining sub-step.
3. ✅ **expr**: moved `expr_ast` + `expr_parser` (+ `naming`) into `tcl-syntax`;
   `tcl-compiler` re-exports them under the original paths (the ~45 consumers +
   LSP bindings unchanged). The `TclParseNumber` grammar + the tower-aware
   evaluator land with the runtime numeric-tower chunk (the const-fold evaluator
   stays in `tcl-compiler`).
4. ✅ **format**: moved the conversion-specifier **grammar** (`FmtFlags` + `Spec`
   + `parse_spec`) into `tcl_syntax::format`; the registry keeps its version-aware
   renderers (output byte-identical, `parse` delegates). `scan` + the runtime
   renderer follow; **`binary format/scan` is designed fresh with the
   dual-ported byte-array value type** (see below) — it is byte-domain, not the
   UTF-8 string domain `format`/`subst`/lists live in.
5. ✅ **list quoter convergence**: `const_fold::list_element` (registry) +
   `codegen::helpers::tcl_list_element` (compiler) now delegate to
   `tcl_syntax::list::list_element` (also a correctness fix: leading-`#` +
   control chars). The two `split_list` variants stay distinct **on purpose**
   (documented): `const_fold::split_list` is a conservative fold-safety splitter,
   `codegen::split_list_simple` is non-decoding (raw round-trip) — converging
   either would change optimiser/codegen output.

Each runtime chunk's first step remains "what in `rust/` already does this?"
Restructuring is allowed under the [LSP guardrails](#the-aot-compiler-is-ours-to-restructure-within-the-lsp-guardrails)
(no loss of LSP precision/perf).

**`binary` ⇒ a dual-ported byte-array value type.** `binary format`/`binary
scan` (and `Tcl_GetByteArrayFromObj`/`Tcl_NewByteArrayObj`) operate on a raw
*bucket of bytes*, not a UTF-8 string — so they need a `TCL_BYTEARRAY_TYPE`
internal rep, **dual-ported** like the other value types (`#[repr(C)]` `TclObj`
with the byte buffer as the internal rep) so C extensions interoperate. This is
a value-type chunk (alongside list/dict/string), co-designed with the `binary`
template parser, not a parser extraction — tracked for the value-types track.

### Parser convergence — status + plan

**Started.** `runtime/rust` now path-depends on `tcl-lexer` (verified: the
**workspace-excluded** runtime crate builds against the workspace lexer, and
`tcl-lexer` builds for `wasm32` — `thiserror`-only). A probe
(`runtime/rust/examples/probe.rs`) dumped the token stream for the hard cases
and confirms `tcl-lexer` already handles the edges the runtime's `parse.rs` got
wrong — `{*} x` → `STR("{*")` (literal `*`, not expansion); `# comment` vs
`set x #y` (comment only in command position); `$arr($i)` as one `Var`. Token →
eval-model mapping (using delimiter-stripped `token_text`):

| `tcl-lexer` token | runtime lowering |
|---|---|
| `Sep` / `Eol` | word / command boundaries |
| `Esc` (literal + escapes) | split into `Text`/`Backslash` `WordPart`s (borrow `src`) |
| `Str` (braced, stripped) | `WordBody::Literal` (a whole braced word) |
| `Var` (`$x`/`${x}`/`$arr(i)`) | `WordPart::Variable` (parse name + index; re-lex the index) |
| `Cmd` (`[...]`, stripped) | `WordPart::Command` (inner script) |
| `Expand` (`{*}`) | the next word's `expand` flag |

**Plan (phased):**
1. ✅ Wire the dependency; verify the crate boundary + wasm build; map the tokens.
2. ✅ Lower `tcl-lexer` tokens → the existing borrowed `Command`/`WordPart` model
   (so `interp`/`subst`/`cmd_*` are **unchanged**); `parse_script`/`parse_command`
   now run off the token stream; deleted `parse.rs`'s now-dead `find_bare_end` /
   `skip_space` + their tests. The **81 eval tests pass** (the equivalence gate)
   plus `make runtime-rust-lint`. Three lowering edges, all confirmed via the
   probe and now encoded in `parse.rs`:
   - **Content slicing must use `SourceMap::token_text`, not a naive
     `content_offset..span.end()` range.** For the degenerate empty forms (`{}`,
     `[]`, `""`) the scanner *extends the span by one* to cover the closer, so the
     naive range leaks a trailing `}`/`]`/`"`; and at a `"…$`/`"…[` boundary the
     scanner emits a **zero-content quote-marker `Esc`** whose raw bytes overlap
     the following `Var`/`Cmd`. `token_text` is the one place that clamps both to
     `""`. The runtime recovers the **byte range** from the returned sub-slice by
     pointer offset (it borrows the same buffer) — but **short-circuits empty
     first**, because the empty-clamp returns a `&'static ""` whose pointer is
     unrelated to `src` (else the offset subtraction underflows).
   - **Quoted-kind detection keys off the opening source byte
     (`src[first_tok.span.start()] == b'"'`), not `Token::in_quote`.** `in_quote`
     is cleared on the *last* token of a quoted word and never set on a
     single-token quoted word, so it is not a reliable "this word is quoted"
     signal; the first token of a quoted word always starts at the `"`.
   - **The empty quoted word `""` lowers to `WordBody::Literal(b"")`** (its only
     token clamps to empty → zero parts → collapse to an empty literal), alongside
     the existing lone-`Text` → `Literal` `SIMPLE_WORD` fast path.

   (`find_braced`/`find_quoted`/`skip_command_subst`/`scan_parts`/`split_list`
   stay for now — `subst` and `Tcl_SplitList` are distinct grammars, converged in
   step 3.)
3. Converge the remaining grammars (the contract's other "parse once" clients):
   `subst` (with `-no*` flags) and `Tcl_SplitList` are not script-lexing, so they
   need a **co-evolution of `tcl-lexer`** (a subst mode + a list mode) — allowed,
   since nothing external uses it and we design a common surface that suits both
   consumers (within the LSP guardrails). Then drop the runtime's `scan_parts` +
   `split_list` + `bs.rs`.
4. Likewise resolve the `&str`-vs-runtime-`&[u8]` boundary by co-designing
   `tcl-lexer` (a bytes entry point, or the runtime upholds its UTF-8 internal-rep
   invariant and converts at the call) — the spans are byte offsets either way,
   so lowered `WordPart`s still borrow the original `&[u8]`.

## Reference implementations (use both freely)

- **Canonical C Tcl 9 source** — `tmp/tcl9.0.3/generic/*.c`
  (`tclBasic.c`, `tclExecute.c`, `tclObj.c`, `tclParse.c`, `tclUtil.c`,
  `tclCmdIL.c` / `tclCmd*.c`, `tclInterp.c`, `tclIO.c`, `tclOO.c`,
  `tclTomMath/*`, …). First-class information source for exact semantics, edge
  cases, the C API, and error/refcount behaviour.
- **`runtime/zig/`** — the current port being mirrored *and* the behavioural
  oracle (parity gate, tcltest sweep, leak baseline).

Where the two differ or Zig is incomplete, **defer to the C source + the Tcl 9
test suite** as ground truth.

## Read first (build on these; do not re-derive)

| Doc | What it gives you |
|---|---|
| [`c-extension-abi.md`](c-extension-abi.md) | ABI (§4), link models (§5), measured GOT findings (§11), scoped next steps (§13) |
| [`runtime/rust-spike/README.md`](../../../runtime/rust-spike/README.md) | The three throwaway spikes — reimplement properly, do not derive shape from |
| [`memory-management.md`](memory-management.md) + [`refcount-contract.md`](refcount-contract.md) | TclObj model + refcount discipline (cross-check vs `tclObj.c`) |
| [`c-api-ownership-contract.md`](c-api-ownership-contract.md) | T2.1 — ownership + error category for every shipped C-API function (the `fresh_zero` convention) |
| [`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md) | The call protocol: the two stacks (CallFrame + CmdFrame), exceptions/return-options, stack-trace construction, AOT↔interp interop — **read before the proc chunk**. Conservative-first; dynamic cross-scope (`uplevel`/`upvar`/`namespace`/`eval`) correct before optimising |
| **The three day-one contracts** (`docs/design/contracts/`, from PR #551 — the from-scratch "if starting over" semantics, authoritative): [`runtime-variable-frame-model.md`](../contracts/runtime-variable-frame-model.md) (cell/frame/namespace resolution), [`parser-and-aot-interpret-boundary.md`](../contracts/parser-and-aot-interpret-boundary.md) (parse-once + the AOT/interpret boundary), [`numeric-tower-and-expr-semantics.md`](../contracts/numeric-tower-and-expr-semantics.md) (the numeric tower + `expr`) | The canonical contracts the Rust port implements; the "Deep Tcl semantics" section below maps port status against them |
| **The three PR #554 contracts** (`docs/design/contracts/`): [`command-binding-and-aliasing.md`](../contracts/command-binding-and-aliasing.md) (one `resolve(ns,name)` + the binding lattice), [`variable-trace-dispatch-and-introspection.md`](../contracts/variable-trace-dispatch-and-introspection.md) (trace fire-order/error-wrap/ignore + live introspection), [`compiled-scope-and-name-lowering.md`](../contracts/compiled-scope-and-name-lowering.md) (scope class as a lowering output, emits-nothing, token-faithful fallback) | The command-layer + scope/trace contracts; captured as port directives in [the scope/trace/info + command-binding lessons](#lessons-from-the-tcl-9-wasm-correctness-campaign--scope--trace--info) |
| [`../compiler/wasm-aot-staircase.md`](../compiler/wasm-aot-staircase.md) (+ s0..s6) | AOT north star + staircase; the metaprog heuristics extend this |
| [`zig-runtime-roadmap.md`](zig-runtime-roadmap.md) | The Zig runtime's own roadmap and layering |
| [`../../../AGENTS.md`](../../../AGENTS.md) | Zig runtime layering, the WASM parity gate (`make check-wasm-parity`), workflow |
| [`../../rust-rewrite.md`](../../rust-rewrite.md) + `docs/design/rust/` | The Rust migration this fits into |

Upstream trees: `tmp/tcl9.0.3/{generic/{tcl.h,tclDecls.h},doc/*.3,tests/*.test}`;
dltest samples in `tmp/tcl9.0.3/unix/dltest/`; tcllib at `tmp/tcllib-2.0`.

## Branch + base

Branched off `rust` (the spikes + design doc are merged there). Many small,
individually-gated PRs. Branch anchor: `rust`@`8150eca` (#549 — the spike
merge). The Zig sync log below anchors against this same commit (the state of
`runtime/zig/` at branch point).

---

## Target — WASM + WASI (chosen target + rationale)

**Decision.** Two surfaces, deliberately split:

| Surface | Target | Why |
|---|---|---|
| **Extension-loading path** (side modules: compiled user code + C extensions) | **core wasm + shared linear memory + a growable, exported `__indirect_function_table`** | The component model is *shared-nothing*; it fights the shared-linear-memory C-extension ABI (`c-extension-abi.md` §3–§5). Dynamic linking needs `__memory_base`/`__table_base` allocated from one shared memory and `call_indirect` across modules through one shared table. This stays on **core wasm** regardless of how the outer runtime ships. |
| **Outer host/runtime interface** (clock, filesystem, channels, stdio) | **WASI preview 1** today (`wasm32-wasip1`); **evaluate preview 2/3** as the host interface only | Preview 1 is what the shared-memory dynamic-linking model supports today. Preview 2/3 (the component model) may wrap the *outer* artifact later, but must not push the extension path off core wasm. |

Rust targets in use: `wasm32-wasip1` (runtime + host), `wasm32-unknown-unknown`
(side modules where no WASI is needed). Newer wasip targets are added as the
shared-memory model adopts them. Record any target change here with rationale.
The bitness consequences (i32 addresses, native i64 values, fixed wasm32 ABI)
are in [Design levers we own](#bitness--pointer-model--wasm32-addresses-native-i64-values).

**Linker flags (from `c-extension-abi.md` §8 / §5.2):**

- Runtime/main module: `--export-table --growable-table` + exported `memory`.
- Side modules: `-fPIC` → `wasm-ld --experimental-pic -shared --no-entry
  --import-memory --import-table`.

**Toolchain (pre-installed):** stable Rust (`wasm32-wasip1` +
`wasm32-unknown-unknown`); `zig` (use `zig cc` for C — bundled wasi-libc);
`wasm-ld`; `wasmtime`.

---

## Chunking strategy

Three interleaved tracks, sequenced by what each tier/north-star step needs.
Each chunk is one PR (or a short PR series), scoped and gated; never merge a
tier or stage without its gate green. If a needed surface is large (channels
for Memchan; eval-loop depth for sqlite's `db eval`), it lands as its own gated
PR **before** the gate that needs it.

- **Track 1 — port the runtime to Rust** (`runtime/rust/`): real TclObj +
  refcount, then parse/subst → eval loop → frames → namespaces → command table
  → builtins, mirroring `runtime/zig/` with C source for semantics. Re-export
  the `tcl_*`/`obj_*` symbols AOT codegen imports so parity stays green.
- **Track 2 — production C dynamic-linking interface**: promote the spike
  headers to shipped headers backed by real impls; land the C-API
  ownership/error contract first (a gate rejects un-annotated exports); move the
  loader from the Python spike into the runtime/host; add the
  external-command-registration dispatch entry.
- **Track 3 — AOT-first execution + whole-program link**: factor per-command
  lowering behind a **backend-agnostic emit protocol/trait + command-emission
  registry** (one source of truth targeting tclvm / wasm / llvm-ir); make AOT
  the primary path; extend `wasm_link.py` to link extension objects (static
  Model A where possible, dynamic Model B otherwise); drive AOT coverage up the
  staircase; add the new metaprogramming-heuristics stage (S7).

**De-risking (allowed).** The ABI is language-independent
(`c-extension-abi.md` §9), so the dynamic loader + a tier MAY be validated
against the existing **Zig** runtime first to separate *loader risk* from *port
risk*. The end state is the Rust runtime, AOT-first, passing all three tiers
and the Tcl 9 gold-standard suite.

---

## Choosing algorithms & data structures (the porting method)

Porting is **not** transliterating the Zig source line-by-line. Every
data-structure-bearing chunk — the value types above all (`obj` internal-rep
union, list, dict, string, array, hash table, the parse cache) — must
**re-derive** the right representation from first principles and prove the
choice empirically. A faithful port can still pick a bad structure; this
discipline stops that. It is the runtime-port analogue of `rust-rewrite.md`'s
"what a good port looks like", specialised for the runtime's performance- and
ABI-critical data structures.

Apply these three steps to every such chunk, in order, and record the outcome
(see *Recording the decision*). Treat it as a hard checklist, not advice.

### 1. Investigate the commands/subcommands that exercise the structure

Before choosing a representation, enumerate **every command and subcommand**
that reads or mutates it, and classify the resulting access-pattern profile.
The structure exists to serve those operations; its shape is dictated by them.

- Read both sources: the Zig handlers (`runtime/zig/cmds/*.zig` — e.g. list
  ops span `list.zig`, `dict.zig`, `string.zig`, `loop.zig`, `tcl_cmd_*.zig`)
  **and** the canonical C (`tmp/tcl9.0.3/generic/tcl{ListObj,DictObj,Hash}.c`,
  `tclCmd*.c`). The C source is ground truth for *what operations must be
  cheap* and *what invariants the value model guarantees* (ordering, sharing,
  shimmering).
- For each operation, record: frequency/hotness (loop bodies vs setup),
  complexity demand (random index vs sequential), mutation shape (append-only
  vs middle-insert vs in-place set), and whether it observes order or identity.
- **Assign each operation a target asymptotic complexity** and choose the
  representation that meets it (the [low-O() core tenet](#core-tenet--low-algorithmic-complexity-from-the-start)).
  Watch for the two cliffs the Zig runtime hit: a **build loop** turning O(1)
  inserts into O(n²) (capacity / index), and **indexed re-access in a loop**
  (use a cursor/snapshot, and rely on object-passthrough so a typed value isn't
  re-shimmered per access).

Worked seed — the **list** type drives the choice to a contiguous growable
`Tcl_Obj*` array (mirroring `tclListObj.c`'s `List` struct), **not** a linked
list:

| Operation | Demand | Implication |
|---|---|---|
| `lindex` / `lset` | O(1) random access | array indexing, not list traversal |
| `lappend` (hot, loop bodies) | amortised O(1) append | growable with spare capacity + end pointer |
| `linsert` / `lreplace` / `lrange` | block shift / slice | contiguous storage; share-on-write for unshared tail |
| `lsort` / `lsearch` | build a working array | already an array — sort in place / over a copy |
| `foreach` / `lmap` | sequential scan | cache-friendly contiguous walk |

Worked seed — the **dict** type must be an **insertion-ordered** hash map
(Tcl 8.5+ semantics, `tclDictObj.c` chains entries in insertion order):
`dict for`, `dict keys`, `dict map` iterate in insertion order, so a plain
unordered map is wrong; the representation needs a hash index **plus** an order
chain. Subcommands `get/set/exists/keys/values/append/lappend/incr/unset/`
`merge/filter/map/for/update/with/size/replace/remove/create/info` define the
full op set to satisfy.

### 2. Run WASM-compiled experiments to verify

Do **not** settle constant-factor or crossover questions by intuition — wasm
changes them. Under `wasm32` there is no native SIMD by default, the allocator
is libc-`malloc` (~100 ns/alloc since MM-A, vs ~5 ns for the retired bump
allocator), linear memory is a single growable region, and branch/indirect-call
costs differ from native. A structure that wins on the host can lose under
wasmtime.

- Build the candidate(s) into a small experiment, compile to `wasm32-wasip1`,
  and run under `wasmtime` — the **actual target**, not a host `cargo bench`.
  Experiments live under `runtime/rust/experiments/` (throwaway, each with a
  one-line "what question does this answer"); keep the *decision*, discard the
  code (like the spikes).
- Measure what the op profile from step 1 says matters: e.g. for the list,
  append throughput at N = 10/1e3/1e6, `lindex` random-access latency, and the
  alloc count per op (the MM-A cost makes alloc count, not just wall time, a
  first-class metric — fewer allocations usually beats a cleverer structure).
- Reuse the existing harnesses where they fit (`scripts/bench_wasm_runtime.py`,
  `scripts/perf_microbench.py`, the S6 microbench baseline) so numbers are
  comparable to the Zig runtime and the staircase gates.

### 3. Reason through C-extension support (the ABI constraint)

The representation is **not free to choose** — it must satisfy what extensions
observe through the public C API (`c-extension-abi.md` §4, `tcl.h`). This often
*forces* the structure outright, and it is the step most easily missed:

- **The obj stays `#[repr(C)]`.** Whatever internal rep a value type uses hangs
  off `Tcl_Obj.internalRep` / `typePtr`; the 24-byte header layout (§4.2) is
  fixed because extensions dereference it.
- **`Tcl_ListObjGetElements` hands back a `Tcl_Obj **` array** the extension
  indexes directly — so the list **must** be able to materialise a contiguous
  `Tcl_Obj*` array. This alone rules out a rep that can't produce one cheaply.
- **`Tcl_HashTable` is ABI-visible.** Extensions (e.g. `pkgua.c`) embed a
  `Tcl_HashTable` **by value**, set `Tcl_HashEntry` fields, and walk buckets via
  `Tcl_FirstHashEntry`/`Tcl_NextHashEntry`. We cannot substitute an arbitrary
  Rust `HashMap` for that surface — the chained-bucket layout and entry struct
  are part of the contract. (An internal-only table *can* use a better
  structure; the ABI-exposed one cannot.)
- **Custom `Tcl_ObjType`** means the internal rep is *pluggable* by extensions —
  our types coexist with extension-registered ones, so type dispatch goes
  through `typePtr`, never a closed enum that assumes only built-in types.
- **Shimmering + sharing** (`Tcl_Obj` shared across references, string rep
  generated on demand) constrain mutation: an unshared (`refCount == 1`) value
  may mutate in place; a shared one must copy. The structure must support both.

#### The shim escape-hatch — decouple the internal rep from the ABI view

The ABI constrains the **boundary behaviour**, not necessarily the **internal**
representation. Where there are **big gains to be had**, the runtime may use a
different (better) data structure or algorithm internally — optimised for the
hot AOT-compiled paths — and provide a **shim that materialises the
ABI-expected view on demand** when an extension crosses the boundary. The
internal/dual-rep machinery already does exactly this for strings (lazy string
rep); the same pattern generalises to other types when it pays.

Whether a surface is shimmable depends on **how the extension observes it**:

- **Function-mediated surfaces are shimmable.** When the extension only ever
  reaches the data through C-API calls (`Tcl_ListObjGetElements`,
  `Tcl_DictObjGet`/`First`/`Next`, the `Tcl_Get*FromObj` accessors), the runtime
  can keep a smarter internal rep and **lazily build the ABI view inside the
  call** (e.g. materialise the contiguous `Tcl_Obj**` only when
  `Tcl_ListObjGetElements` is actually called, and cache it on the obj until the
  next mutation). The internal rep wins on the hot path; the shim cost is paid
  only at the (often rare) boundary crossing.
- **By-value / direct-layout surfaces are *not* shimmable** (or only by keeping
  the layout). When the extension embeds the struct **by value** and reads its
  raw memory — `Tcl_HashTable` walked via `Tcl_HashEntry.nextPtr`, or
  `objPtr->bytes` dereferenced directly — there is no call to interpose on, so
  the layout itself is the contract and must be honoured (a nominal/faithful
  struct, §6/§11). A shim cannot help here.

Decide with the same evidence as everything else: the gain is only real if the
internal-rep speedup on the hot path (step 2's WASM numbers) **exceeds the
shim's materialisation cost amortised over the boundary-crossing frequency**
(step 1's op-profile — how often extension code actually touches this value).
Record a shimmed type's two reps and the materialise-on-demand contract in its
representation-decision note.

### Experiment log

Concrete experiments that settled a structure (the method's step 2, on the real
WASM target). Each lives under `runtime/rust/experiments/` and is throwaway; the
**decision** is what's kept.

#### EXP-DICT (2026-06) — dict internal representation

Question: the dict needs by-key get/set (hot, incl. `dict set` build loops) and
**insertion-ordered** iteration (`dict keys`/`dict for`). Its rep is free to
choose (the dict C ABI is function-mediated — see below). Five candidates,
built to `wasm32-wasip1`, run under `wasmtime` (`experiments/dict_rep.rs`):

| cand | structure | build | lookup | iter | verdict |
|---|---|---:|---:|---:|---|
| A | linear `Vec` | **23.5 s** | 24 s | 52 µs | O(n²) build — **out** |
| B | `Vec` + `BTreeMap` index | 85 ms | 47 ms | 65 µs | ok; btree by-key slower |
| C | `BTreeMap`+seq, sort-on-iter | 82 ms | 46 ms | 1233 µs | sort kills iter — out |
| D | `HashMap`+seq, sort-on-iter | 11 ms | 12 ms | 3979 µs | sort kills iter — out |
| **E** | **`Vec` + FNV-hash index** | **15 ms** | **14 ms** | **68 µs** | **chosen** |

(N=65536; small-N all within noise.) **Decision: E** — an insertion-ordered
`Vec` of `(key,value)` object pairs (O(n) ordered iteration, no sort) + a
`HashMap<key-bytes, index>` with a fixed FNV hasher (O(1) by-key). Deterministic
(output order = `Vec` order; the hash is never iterated for output), zero-dep.
The linear `Vec` (A) showed the **`dict set` build-loop cliff** is real (23.5 s);
sort-on-iterate (C/D) is 18–60× slower on the very common `dict for`.

**C-extension compatibility (explicitly checked).** Compatible — and a good
illustration of the shim escape-hatch. The dict C API
(`Tcl_DictObjGet`/`Put`/`Remove`/`First`/`Next`) is **function-mediated**: an
extension *never* observes the dict's internal structure (contrast
`Tcl_HashTable`, embedded by value with its bucket layout a hard contract), so
the rep is ours to choose. The two ABI touch-points are honoured by
construction: keys/values cross as `Tcl_Obj *`, so we store the **key objects**
(not just bytes) for `Tcl_DictObjFirst` fidelity (the byte-keyed index is just
for lookup — dicts compare keys by string); and `Tcl_DictObjFirst`/`Next`
iterate via an opaque `Tcl_DictSearch` struct the runtime fills with a `Vec`
index + an `epoch` (added with that C API) for modify-during-iteration
detection — and insertion order is exactly what the ordered `Vec` provides. So
we are not constrained *to* a structure; we are **free to choose, then make the
boundary compatible** — which is the general posture for every function-mediated
value type.

#### EXP-STRING (2026-06) — string char-access + append

Question: two cliffs the low-O() tenet forbids. (1) `string index`/`length`/
`range` are **character**-indexed, but UTF-8 makes naive char access O(n) → an
O(n²) char-indexed loop. (2) `append`/`string cat` build strings → an O(n²)
realloc-each loop. `experiments/string_rep.rs`, on wasm/wasmtime:

| | strategy | @1000 | @4000 | verdict |
|---|---|---:|---:|---|
| char | S: scan UTF-8 each access | 751 µs | 12 603 µs | O(n²) — **out** |
| char | I: lazy char-offset index | 2.6 µs | 10 µs | O(n) |
| char | A: ASCII fast path (else I) | **0 µs** | **0 µs** (ascii) | **chosen** |

| | strategy | @20 000 | @200 000 | verdict |
|---|---|---:|---:|---|
| append | R: realloc exact each | 43 ms | **9.3 s** | O(n²) — **out** |
| append | V: amortized capacity | 30 µs | **407 µs** | **chosen** |

**Decisions.** (1) Char access = **ASCII fast path** (the common case: byte index
== char index, O(1), zero overhead) + a **lazily-built char-offset index** for
non-ASCII (O(n) once, O(1) after) — never scan-per-access. (2) `append`/string
build = a **capacity-backed string rep** (amortized growth) — never realloc-each
(the 9.3 s hang is a correctness bug per the tenet). So the **string** type is a
dual-ported obj like list/dict: a capacity-backed byte buffer (the lazy string
rep, NUL-terminated for the C ABI) + cached `numChars` and an optional non-ASCII
char-offset index. C-extension compatible: extensions read bytes via
`Tcl_GetStringFromObj` (contiguous NUL-terminated — preserved); the char-indexed
C API (`Tcl_GetCharLength`/`Tcl_GetUniChar`/`Tcl_GetRange`) is function-mediated
→ free to choose the cache. **Implemented** (T1.6): a plain string's buffer
capacity lives in `internal_rep` + `obj::string_append_inplace` grows it
amortised; ASCII fast-path char ops in `cmd_string.rs`. The non-ASCII
char-offset cache stays deferred (object-passthrough keeps the obj, so it can be
added later without re-shimmering); experiment kept as evidence.

#### EXP-BIGNUM (2026-06) — the numeric tower's bignum representation

Question: the integer tower is `small → wide (i64) → bignum → double` with
overflow-promotion and demote-when-fits ([the numeric-tower
contract](#meta-system-3--the-numeric-tower--expr-a-second-language)). `wide`
and `double` already exist in `obj.rs` (`TCL_INT_TYPE` = i64 in `internal_rep`,
`TCL_DOUBLE_TYPE` = `f64::to_bits`). The open decision is the **bignum** rep, and
it is the one numeric piece with a hard **C-extension ABI** constraint:
extensions get a bignum via `Tcl_GetBignumFromObj` **by value as an `mp_int`**,
and they call `mp_*` on it.

**Who controls the bignum ABI? — we do, entirely** (verified against the Tcl 9
headers). Extensions do **not** bundle libtommath: `tclTomMathDecls.h` is
`#define mp_add TclBN_mp_add …`, so extension `mp_*` calls are macros routing to
`TclBN_*` functions the runtime **exports via the stubs table**
(`tclTomMathStubsPtr`), and the `mp_int` struct comes from **`tclTomMath.h`,
which the runtime ships**. So we own the header (the `mp_int` layout) *and* the
implementation (the `TclBN_*` symbols). There is no external libtommath to
match — only internal consistency between our header, our exported `TclBN_*`,
and our `Tcl_GetBignumFromObj`. And we need those `TclBN_*` exported *anyway*
(extensions call them on objs we hand out), so **libtommath must be in the
artifact regardless** — which makes a second, separate Rust bignum a pure
liability (two reps + a conversion at every C boundary, the contract's
"N implementations → N drifts"). **Decision: libtommath `mp_int` *is* our
bignum**, exactly as C Tcl does.

**The wasm-matched representation (measured).** libtommath picks its limb width
from the *pointer* width, so on wasm32 it defaults to **MP_32BIT** (28-bit usable
limbs). But wasm has **native i64 value ops** (the "wasm32 addresses, native i64
values" tenet) — so we **force MP_64BIT** (60-bit limbs, native-i64 multiply).
Layout probe (`tclTomMath.h` via `zig cc --target=wasm32-wasi`, run under
`wasmtime`):

| target / config | `mp_digit` | `mp_int` size | field offsets |
|---|---:|---:|---|
| native 64-bit (default → MP_64BIT) | 8 | 24 | used@0 alloc@4 sign@8 **dp@16** |
| wasm32 (default → MP_32BIT) | 4 | 16 | used@0 alloc@4 sign@8 **dp@12** |
| **wasm32 + `-DMP_64BIT` (chosen)** | **8** | **16** | used@0 alloc@4 sign@8 **dp@12** |

So forcing MP_64BIT on wasm32 keeps the **struct at 16 bytes** (only the heap
digit array widens 4→8 B/limb) — fewer limbs, native i64 arithmetic, and the
`mp_int` ABI is unchanged for extensions (they compile against the *same*
`tclTomMath.h` with MP_64BIT, so it is consistent by construction). libtommath
also **compiles and runs on wasm32** (the probe built + executed under wasmtime).

**Obj storage = Tcl's inline two-pointer PACK, which is already wasm-native.** On
wasm32 the obj `internalRep` is 8 bytes = two i32 pointers — exactly what our
`internal_rep: u64` models. Tcl's `PACK_BIGNUM` stores `ptr1 = dp` (the i32
digit pointer) and `ptr2 = (sign<<30)|(alloc<<15)|used` (the header packed into
the other i32) — **no separate heap `mp_int` struct** for the common case
(`used`/`alloc` ≤ 0x7FFF ≈ 245k decimal digits at 60-bit limbs), with a
heap-`mp_int` fallback (`ptr2 = -1`) for larger. This packs the whole bignum
header into the 8-byte `internal_rep` (low 32 = `dp`, high 32 = packed), so it is
both byte-identical to C Tcl on wasm32 **and** wasm-optimal (no extra
allocation). `change_type`'s free-proc calls `mp_clear` + frees the digit array;
the dup-proc is `mp_init_copy`; the update-string proc is `mp_to_radix(…,10)`.

**Canonicalisation (contract, observable).** Every integer-producing op
overflow-checks the wide fast path and promotes to bignum on overflow (`i64`
`checked_*`, never wrapping); every bignum result is **demoted back to
`TCL_INT_TYPE` when it fits a wide** (`mp_count_bits ≤ 63` + range check), so
equality/hashing/string-rep stay stable. Floor division/modulo (sign-of-divisor)
and the `**`/shift/bit-op rules follow `tclExecute.c`'s
`ExecuteExtendedBinaryMathOp` exactly. `WIDE_MIN` negation and `WIDE_MIN / -1`
are the wide-path overflow escapes into bignum.

**Status.** Representation decided + **validated end-to-end on both targets** —
`2**100` computes correctly under both `clang` (native) and `zig cc
--target=wasm32-wasi` + `wasmtime`, with MP_64BIT giving 2 limbs (60-bit). The
build recipe is **solved** (it had looked blocked): Tcl's bundled libtommath is
wired into Tcl's stubs (`tclTomMath.h` renames `mp_*`→`TclBN_*` and tangles the
`MP_INIT_INT` code-gen templates when its own `.c` is compiled), so build
**pristine** with `-DTCL_WITH_EXTERNAL_TOMMATH -DLTM_ALL -Ilibtommath`, all
`libtommath/*.c` except `bn_deprecated`/`*rand*`/`*prime*` (139 files; the
integer tower needs no RNG/primality). See
[`runtime/rust/experiments/bignum/`](../../../runtime/rust/experiments/bignum/).
This is what the runtime's `build.rs` drives to link `mp_*` for the
`TCL_BIGNUM_TYPE` FFI (source vendoring for a fresh-checkout-reproducible build
is the one remaining follow-up).

Implementation order: ✅ (1) the shared **`tcl_syntax::number`** grammar;
✅ (2) the `TCL_BIGNUM_TYPE` obj rep + the `mp_*` FFI (`Big`→`mp_int`,
demote-when-fits) via `build.rs`; ✅ (3) the tower arithmetic (`+ - * / % **`
floor-div, bit-ops, shifts, comparison — `bignum.rs`) **and** the `expr`
evaluator over the **shared** `tcl_syntax::expr::eval<ExprOps>` walk (`expr.rs`).
Remaining: `rand`/`srand` (interp RNG state — `mathfunc` dispatch otherwise lands
as overridable `::tcl::mathfunc::*` commands, done), the C-extension boundary
(`Tcl_GetBignumFromObj` + the `TclBN_*` stubs table, Track 2/3), wiring `expr`/
`tcl::mathop`/`incr` builtins to the eval loop, and the compiler-side
`tcl_expr_eval`→`ExprOps` convergence.

### The value kinds to reason through (and their two relationships)

Work through **every** value kind below. Each has a representation question and
**two relationships that constrain it from opposite sides** — the AOT-compiled
code (which wants values unboxed and shimmer-free) and the C extensions (which
observe values through the `#[repr(C)]` `Tcl_Obj` + the public C API). A
representation that serves one but not the other is wrong.

| Value kind | Representation question | Relationship to AOT-compiled code | Relationship to C extensions (ABI) |
|---|---|---|---|
| **String** (the canonical rep — EIAS) | owned vs borrowed vs inline (≤N-byte) buffer; Tcl 9 internal encoding; NUL-termination; append capacity | literals interned once; the compiler keeps `(ptr,len)` when it can prove no mutation | `Tcl_GetStringFromObj` returns the buffer pointer **directly** → must be contiguous, stable, NUL-terminated |
| **Scalar variable** (one value) | var slot → single `Tcl_Obj` handle (+ a retain) | a non-escaping scalar local can become a WASM local holding an *unboxed* value or an obj handle (S2 frame elision) | `Tcl_ObjSetVar2`/`Tcl_SetVar2` read/write the slot; the var holds a `Tcl_Obj` the extension may retain |
| **Number — int (wide)** | `internalRep.wideValue` (i64), `typePtr=int`; overflow → bignum | the hot path: tagged immediates (S6.4) keep small ints unboxed in WASM i32/i64 — no alloc, no shimmer; `expr` runs on unboxed i64 | `Tcl_NewWideIntObj` / `Tcl_GetIntFromObj` / `Tcl_GetWideIntFromObj` read `internalRep` |
| **Number — float** | `internalRep.doubleValue` (f64), `typePtr=double`; `%.17g` string gen | unboxed f64 in WASM locals for `expr` | `Tcl_NewDoubleObj` / `Tcl_GetDoubleFromObj` |
| **Number — bignum** | `internalRep` → `mp_int` (pointer to a digit array), `typePtr=bignum` | rare; falls to runtime arithmetic (libtommath) — not unboxed | `Tcl_NewBignumObj`/`Tcl_GetBignumFromObj` + the `mp_*` ABI; digits **must** allocate through `Tcl_Alloc` (single-allocator invariant §4.4) |
| **List / arrays of scalars / of lists / of objects** | contiguous growable `Tcl_Obj*` array (`tclListObj.c` `List`); elements are scalars, nested lists, or arbitrary-typed objs | `foreach`/`lmap` walk; a proven-non-escaping list can live as a WASM-side array | `Tcl_ListObjGetElements` hands back a `Tcl_Obj **` → the rep must materialise a contiguous array; append retains into the list |
| **Tcl array variable (assoc array)** | name → hash(element-key → `Tcl_Obj`); **nesting** ("array of arrays") is modelled via dicts / flattened `a(b,c)` keys — Tcl arrays don't nest, and an array is **not** a first-class `Tcl_Obj` | `arr(key)` compiles to a hash lookup against the frame/ns-resident array | `Tcl_SetVar2(name, key, …)` addresses elements; an array **cannot** be passed as a value — the rep must honour that asymmetry |
| **Dict** (ordered map) | hash index **plus** an insertion-order chain (Tcl 8.5+ `tclDictObj.c`); key obj → value obj | `dict for`/`dict get` iterate in insertion order; may stay an obj | `Tcl_DictObjGet`/`Put`/`First`/`Next` expose ordered iteration — a plain unordered map is wrong |
| **Object (TclOO)** | instance record: command + namespace + class ptr + method table + instance vars | method dispatch is **dynamic** (through OO resolution, like extension commands — never inlined) | `tclOO.h`: `Tcl_NewObjectInstance` / `Tcl_GetObjectFromObj` / `Tcl_ObjectContextInvokeNext` |
| **Shimmer + dual representation** | every `Tcl_Obj` carries a string rep **and** an optional internal rep; converting the internal rep on demand = shimmer; the string rep is regenerated lazily | the AOT compiler's *whole job* is to **avoid** shimmer — prove the type statically and keep the value unboxed; fall back to the dual-rep obj only when a value escapes into dynamic use. Minimising shimmer minimises both allocs and string regen | extensions depend on the dual rep: `Tcl_GetString` always returns a valid string (regenerating if needed); `Tcl_Get*FromObj` may shimmer; a **custom `Tcl_ObjType`** supplies the four procs (`freeIntRep`/`dupIntRep`/`updateString`/`setFromAny`) our shimmer/free/dup machinery **must** call through `typePtr` — so type handling is open, never a closed enum of built-ins |

The **dual-rep / shimmer** row is the cross-cutting keystone: it is simultaneously
the thing the compiler works hardest to avoid (unboxing, type proofs, immediates)
and the thing extensions most rely on (lazy string regen, pluggable `Tcl_ObjType`).
Get its contract right first — `free_string_buffer`/`get_string` shimmer in
`obj.rs` (T1.1) is the seed; extension-typed objs extend it to dispatch through
`typePtr` to the extension's procs.

### Recording the decision

Each data-structure chunk lands with a **representation-decision note** in its
component-table row (and, when substantive, a short `docs/design/runtime/`
KCS/design doc): the op profile (step 1), the WASM experiment numbers that
settled it (step 2), and the ABI constraints that bound it (step 3). A chunk
that picks a non-obvious structure without these three is not done. The gate for
such a chunk includes its experiment evidence, not just green tests.

---

## Component status table — `runtime/zig/` → Rust

Status vocabulary: **not-started** / **partial** / **landed**. "Gate" is the
concrete artifact that proves the component (a test, a sweep delta, a parity
entry). Anchor: every row is **not-started** at branch point (`runtime/rust/`
does not exist yet); the spike code is *not* a port and does not count toward
any row.

| Zig module | Files (lines) | Role | Rust target | Status | Gate that proves it |
|---|---|---|---|---|---|
| `valtypes/tcl_obj.zig` | 1 (1104) | `Tcl_Obj` model, refcount, shimmer | `runtime/rust/` obj core | **partial** (T1.1) | `make runtime-rust-test` — `round_trip_zero_residual` leaves zero residual under the alloc/free counters |
| `valtypes/` value types | 20 (9211) | list, dict, string, array, arith, format, encoding, hash_table, bs, chars, regex, arena, parse_cache | `runtime/rust/` valtypes | **partial** (obj typed-rep machinery + **list** + **dict** + **string** capacity/char-ops, T1.6) | `make runtime-rust-test` — list + dict (ordered-`Vec`+FNV-index, EXP-DICT) + string (capacity-backed append + ASCII-fast char ops, EXP-STRING) leak-checked; array/etc. follow, each **+ a representation-decision note** (see [Choosing algorithms & data structures](#choosing-algorithms--data-structures-the-porting-method)) |
| `parse/` | 3 (956) | `tcl_parse`, `tcl_subst` | `runtime/rust/` parse | **partial** (T1.2) | `make runtime-rust-test` — parse/subst unit parity (`parse`/`subst`/`bs` modules); evaluation of `$var`/`[cmd]` segments wired with the eval loop (T1.3/T1.4) |
| `interp/tcl_interp.zig` | 1 (2065) | eval loop, interp object | `runtime/rust/` interp | **partial** (T1.4) | `make runtime-rust-test` — eval loop: parse→subst→dispatch, `{*}`, completion codes; control-flow/proc follow |
| `interp/` frames/ns/procs | 8 (6348) | frames, namespaces, procs, catch, caps, trace, interp_registry | `runtime/rust/` interp | **partial** (T1.3 frames + var store; T1.5 **namespace tree + command *and* variable resolvers**) | `make runtime-rust-test` — frame/var round-trips (scalar/array/upvar/global); `namespace.rs` arena tree + the one `resolve(currentNs, name)`; `rename`/`interp alias` (`Alias` redirect) + the `namespace` command (`Imported` redirect); **`vars.rs` variable resolver** (per-namespace var tables, `VarHome` links, `global`/`variable`/`upvar`); procs/catch follow |
| `dispatch/` | 5 (746) | cmd registry, cmd table, dispatch, diag, stub_fallback | `runtime/rust/` dispatch | **partial** (T1.4/T1.5) | `make runtime-rust-test` — dispatch resolves through the **namespace tree** (`Builtin`/`Alias`/`Imported` handles), no flat table; `make check-wasm-parity` once the builtin surface fills in |
| `cmds/` builtins | 34 (8367) | all builtin commands | `runtime/rust/` cmds | **partial** (T1.4/T1.5/T1.6 + M1–M4) | `make runtime-rust-test` — `set`/`incr`(tower)/`return`/`unset`(`-nocomplain`) + `expr` + `subst` + list cmds (`lindex` index-path) + `dict` ensemble (full: get-path/replace/remove/filter/map/update/with) + `append` + `string` ensemble (incl. `match`/`map`/`is`) + `scan`/`format` + `rename`/`interp alias`/`namespace` (+ `ensemble`/`code`/`origin`) + `global`/`variable`/`upvar` + `::tcl::mathfunc/mathop::*` + `proc`/control-flow/`puts` + `catch`/`error`/`try`/`throw` + `info`(incl. `level N`/`complete`) + `array`/`switch`/`package` + `source`/`file`/`glob`/channels + `trace` (variable) + **`regexp`/`regsub`** (real Tcl ARE engine, `have_regex`); **drives the unmodified Tcl 9 library to `package require tcltest` 2.5.10** (M3) and **runs real compute `*.test` files** — `list.test` 78/78, `split.test` 18/18, `linsert` 28/28, `dict.test` 272/373, `lrange` 1759/1766 (M4); per-command parity + tcltest sweep as more land |
| `io/tcl_chan.zig` | 1 (1858) | channel subsystem | `runtime/rust/` io | not-started | chan/chanio/io/ioCmd tcltest suites (Memchan needs this) |
| `io/tcl_clock.zig` + `tcl_tz.zig` | 2 (3560) | clock + tz (+ `data/tzdata.bin`) | `runtime/rust/` io | not-started | clock tcltest slice (`run_clock_tcltest.py`) |
| `io/tcl_fs.zig` | 1 (1186) | filesystem (tclvfs needs `Tcl_FSRegister`) | `runtime/rust/` io | not-started | fs tcltest + tclvfs tier-1 gate |
| `sched/` | 7 (1660) | scheduler, coro, timer, vwait, fileevent, ready, asyncify | `runtime/rust/` sched | not-started | coroutine/after/vwait tcltest |
| `stubs/` | 6 (609) | env/fmt/fs/io/time stub surfaces | `runtime/rust/` stubs | not-started | covered by dependent command parity |
| `tcl_runtime.zig` (root) | 1 | export-aggregation root | `runtime/rust/` lib root | not-started | runtime builds + exports the `tcl_*`/`obj_*` symbol set codegen imports |
| `regex_include/` (C) | — | Henry Spencer ARE engine (C, vendored) | **C at start → port to Rust near the end** (see note) | **partial — C engine linked** (M3): `build.rs` compiles `regcomp.c`/`regexec.c`/`regfree.c`/`regerror.c` to a static archive (`have_regex`); `regex_shim/` provides the host hooks; `src/regex.rs` is the FFI wrapper; `regexp`/`regsub` run on it (`cmd_regex.rs`). Rust port of the algorithm is the end-stage swap. | start: ARE-fidelity corpus passes via the C engine; end: same corpus passes against the Rust port, zero diff |

`data/tzdata.bin` is a data asset consumed by the clock/tz port, not code.

**Regex engine — use the C ARE engine at the start, port to Rust near the
end.** `c-extension-abi.md` §10 keeps Tcl's Henry Spencer ARE engine as the
*first C library compiled against the runtime* (bit-for-bit ARE fidelity for
free). **Phasing: start on the C engine — it is fine, even preferred, for the
early stages** (it lets the rest of the runtime port and the tier gates proceed
without regex risk). **Near the end of the effort, port the ARE engine from C
Tcl to Rust** so the runtime has no C-language core dependency and the whole
support library is one language. The bar is unchanged: the Rust port must
reproduce ARE semantics — backreferences, lookahead, and POSIX leftmost-longest
— which no off-the-shelf pure-Rust regex crate matches (`regex` /
`regex-automata` are deliberately non-backtracking and reject
backreferences/lookaround), so this is a *port of the ARE engine's algorithm*
(transcribed from `tmp/tcl9.0.3/generic/regc*.c` / `rege_*.c` / `regexec.c`),
not a swap to an existing crate. Gate: the ARE-fidelity corpus passes against
the C-ARE baseline with zero behavioural diff at the time of the swap. If the
Rust port proves materially harder than budgeted, the §10 keep-as-C path
remains the standing fallback (the C-extension toolchain keeps it available
either way). Sequencing: this is a **late** chunk — after Track 1's core
modules land and the tier gates are green on the C engine.

> Update this table every PR: flip a row to **partial**/**landed** with its gate
> the moment it lands. Add new rows if a Zig refactor introduces a module.

---

## Track 1 — Rust runtime port

Goal: a `runtime/rust/` that the AOT codegen links against, parity-green, with
no leak/tcltest regression vs the Zig baseline. Every value-type chunk (T1.2
onward) applies the [algorithm/data-structure method](#choosing-algorithms--data-structures-the-porting-method)
— derive the representation from the command op-profile + WASM experiments +
the C-extension ABI, and land the decision note — rather than transliterating
the Zig rep.

- **T1.1 — Real `TclObj` + refcount discipline.** **Partial — landed.**
  Created `runtime/rust/` (`tcl-runtime`, a standalone crate excluded from the
  workspace — §9 needs `unsafe`). Modules: `obj` (the `#[repr(C)]` `TclObj`,
  ABI-faithful to §4.2, with `fresh_zero` constructors, immediate
  refcount-driven free per `tclObj.c`'s `TclFreeObj`, and on-demand int→string
  shimmer), `interp` (result-only `Interp` with the `Tcl_SetObjResult`/
  `Tcl_GetObjResult` handshake), `counters` (the `tcl_test_*` leak
  instrumentation, MM-C), `capi` (the `#[no_mangle] extern "C"` exports). Gate:
  `make runtime-rust-test` — `round_trip_zero_residual` (`Tcl_NewObj` → incr →
  set-result → decr → interp teardown) leaves **zero residual** and zero
  double-frees under the counters (6 tests pass). **Remaining for full Track-1
  obj parity:** lists/dicts/string-append shimmer, the deferred-free queue
  (`tcl_obj_drain_pending`, lands with the eval loop T1.3), Tcl-faithful
  `double`→string formatting (T1.5), and the codegen handle / tagged-immediate /
  inline-string optimisations (T1.5/S6).
- **T1.2 — parse/subst.** Port `parse/tcl_parse.zig` + `tcl_subst.zig` using
  `tclParse.c` for semantics. Gate: parse/subst unit parity.
  - **Representation decision (re-derived, not transliterated).** The Zig is a
    proof-of-concept, not a guiding light: it carries a C-idiom — a fixed
    `MAX_WORDS=32` flat per-word array **plus** a shallow, unfinished
    `Tcl_Token`-style flat tree (`n_children` always 0). The Rust runtime
    parser instead uses a **borrow-based enum tree** over `&'s [u8]`:
    `Command{ words }`, `Word{ kind, expand, body }`,
    `body = Literal(&[u8]) | Parts(Vec<WordPart>)`,
    `WordPart = Text | Backslash | Variable{name,index} | Command`. Rationale:
    (1) the only consumers are the interpreter-fallback eval loop and the
    `subst`/`eval` family — **not** the AOT compiler (own parser in
    `core/parsing`/`tcl-lexer`) or the LSP — and both want **sum-type dispatch**
    with a `Literal` fast path (Tcl's `SIMPLE_WORD`), not `numComponents` index
    arithmetic; (2) borrowed spans make the Zig `parse_cache` stale-slab hazard
    (MM-B.6) a **compile error**; (3) **one** component scanner serves both the
    word parser and `subst` (the Zig duplicates it across `parse_bare` /
    `subst_flagged`); (4) the whole parser is `#![forbid(unsafe_code)]`.
    Not an ABI surface today (`Tcl_ParseCommand`/`Tcl_Token` aren't in the
    81-function header); if a tier needs it, the enum flattens to `Tcl_Token`
    **on demand** at the boundary — the function-mediated **shim escape-hatch**.
    No WASM experiment gates this: per-command allocations are comparable to a
    flat array, and the real perf lever is *caching the parse* (orthogonal,
    deferred to a measured MM-D-style chunk), so clarity/safety decides.
    Also fixes a `tclParse.c` correctness item the Zig got wrong: `$` not
    followed by a valid name char is a literal `$` (Zig synthesised an empty
    var name). **Seam:** `subst`'s evaluation half needs vars/eval/arrays
    (T1.3/T1.4), so T1.2 lands parse + backslash decode + the shared component
    **scanner** (the parse-level half); segment *evaluation* follows.
  - **Status: partial — landed.** Modules `bs` (backslash decode), `parse` (the
    enum-tree parser + shared `scan_parts` component scanner), `subst` (the
    substitution engine: scan + `resolve_with`, where variable/command lookups
    are caller closures the eval loop supplies in T1.3/T1.4). Gate:
    `make runtime-rust-test` — 36 tests (parse corpus mirrors
    `test_tcl_parse.zig` + the component decomposition the Zig left as a TODO +
    backslash table + subst assembly against mock resolvers). `unsafe`-free
    (`#![forbid(unsafe_code)]` on all three). **Remaining:** wire the real var
    table + eval into the `subst` resolver closures (T1.3/T1.4).
- **T1.3 — eval loop + frames.** Port `interp/tcl_interp.zig` +
  `tcl_frames.zig`. Gate: eval-loop tcltest sweep no-regress.
  - **Split for review:** the eval loop needs the command table (T1.4), so
    **T1.3 = frames + the variable store** (the data-structure foundation, and
    the half subst's *variable* resolver needs); the eval loop + dispatch +
    deferred-free queue pair with the command table in T1.4.
  - **Representation decision (re-derived).** Canonical `Var` (`tclInt.h`) is a
    tagged union `{scalar objPtr | array tablePtr | linkPtr}`; the Zig PoC
    encodes it as i32 handles with sentinels (`ALIAS_GLOBAL = -1`, negated heap
    addresses) — a handle-world artifact. Rust uses the **enum**
    `Var = Scalar(*mut TclObj) | Array(map) | Link{level,name,elem}`.
    - **`BTreeMap` (not `HashMap`) for the var-name table and array elements.**
      Consumers: `set`/`incr` (by-key, hot) + `info vars`/`array names`/`array
      get` (iterate). The O(1)-`HashMap`-+-fixed-hasher vs O(log n)-`BTreeMap`
      crossover is real but small-n; `BTreeMap` chosen for **deterministic
      iteration** (`std::HashMap`'s `RandomState` would make `info vars` /
      `array names` vary run-to-run — poison for an oracle-diffed port) and
      **zero deps**. WASM experiment deferred to the perf gate if array/frame-
      heavy workloads show it matters.
    - **Links resolved by path** (level+name+elem), not Tcl's direct `linkPtr`
      — avoids dangling on map reallocation; trades a lookup for memory safety.
    - **Explicit release, no `Drop`** — matches `TclFreeVar`, keeps refcount
      accounting visible to the leak counters.
  - **Status: partial — frames + var store landed.** `frame.rs`: `Var` enum +
    `FrameStack` with scalar/array vars, `upvar`/`global` path-resolved links,
    push/pop with full release, enumeration (`var_names`/`array_names`), and
    `resolve_var_bytes` — which **closes the variable half of T1.2's subst
    seam** (a test runs `subst` over a real frame store). Counters made
    **thread-local** so the leak-checked tests are correct under parallel
    `cargo test` (and identical on the single-threaded WASM reactor). Gate:
    `make runtime-rust-test` (43 tests, leak-checked frame round-trips).
    **Remaining (T1.4):** the eval loop + command dispatch (closing subst's
    *command* half), namespace var tables, the deferred-free queue, and the
    `info`/proc-call frame metadata (argv, level).
- **T1.4 — eval loop + command table + dispatch. Partial — landed.**
  `interp.rs`: `Interp` (frame stack + command table + result), `eval_str` →
  parse → per-word substitution (with `{*}` expansion via `parse::split_list`)
  → dispatch; `Code` completion codes (Ok/Error/Return/Break/Continue);
  `Command` enum (`Builtin` now; `Proc`/`External{table_index}` — the §13.2
  extension-command entry — are the next variants). **Closes the command half
  of T1.2's subst seam** (a `[cmd]` recursively evaluates its inner script).
  Command-table decision: **`BTreeMap` name→`Command`** (deterministic `info
  commands`, zero deps), same reasoning as the frame tables.
  **No deferred-free queue** (the Zig `tcl_obj_drain_pending`): immediate
  `TclFreeObj` + retain-into-result makes argv release safe without it.
  `builtins.rs`: starter set `set`/`incr`/`return`/`unset` to drive the loop
  end-to-end. Gate: `make runtime-rust-test` (53 tests, leak-checked:
  set/read-back, `[cmd]` subst, `incr`, `{*}` expansion, error paths).
  **Remaining:** the full builtin surface (T1.6), procs + the proc-call frame
  path, full `return -code`/`expr`/control-flow.
- **T1.5 — namespaces.** Port `tcl_ns.zig` (the namespace tree) +
  namespace-qualified command/var resolution; extend the flat global command
  table to the namespace tree. Gate: `make check-wasm-parity` green;
  namespace-tree behaviour preserved (`namespace-tree.md`).
  - **Tree + the one resolver — ✅ done.** `namespace.rs`: an arena
    (`Vec<Namespace>` + `NsId` indices, `GLOBAL = 0`; no `Rc`/parent pointers,
    wasm-clean) with **one** `resolve(currentNs, name)` (A1/A2): qualified →
    direct lookup in the named ns (absolute from `::`, else relative); unqualified
    → current ns → its `namespace path` → global. `Interp` now holds
    `namespaces: Namespaces` + `current_ns` in place of the flat `BTreeMap`;
    `register_builtin`/`dispatch`/`command_names` all route through it. A shared
    `home_of` underlies `resolve`/`delete`/`rename` so they hit the same binding.
  - **`rename` + `interp alias` — ✅ done** (`cmd_alias.rs`, the rename-alias
    wave; mirrors [`rename-alias.md`](rename-alias.md) §3–4 and the
    [alias-resolution contract](../contracts/command-alias-resolution.md)). The
    `Command` enum grew an `Alias { target, prefix }` variant (so `Command` is now
    `Clone`, not `Copy` — `resolve` clones the small handle out of the table). The
    **dispatch trampoline** re-resolves the alias `target` *by name, anchored at
    global, on each call* — lazily observing the target's **deletion** but **not**
    following its **rename** (matches C Tcl) — then prepends the frozen `prefix`
    words. `rename` moves/deletes a binding (built-ins `return`/`error` protected
    with `can't rename "X": built-in command`; `rename old ""` deletes;
    self-rename is a no-op); `interp alias {} new {} target ?arg…?` create / `{}
    new` query / `{} new {}` delete, and `interp aliases {}` lists. Single-interp
    only (non-empty interp paths → explicit error; child interps deferred). Gate:
    `make runtime-rust-test` (114 tests) + `make runtime-rust-lint` green.
  - **The `namespace` command — ✅ done** (`cmd_namespace.rs`): `current`, `eval`
    (switches `current_ns` so commands defined in the body land in the right
    table, then restores), `exists`, `parent`, `children`, `qualifiers`, `tail`,
    `which -command` (one-liner over the resolver — returns the FQN it resolves
    to), `export ?-clear?`, `import ?-force?`, `forget`, and `path`. `import`
    installs a transparent `Command::Imported { source }` redirect (a third
    `Command` variant; dispatch re-resolves the source FQN anchored at global and
    forwards argv unchanged) only for commands the source ns actually **exports**
    (export patterns matched with `string match` glob); `forget` removes those
    redirects by matching the stored source FQN. Gate: `make runtime-rust-test`
    (124 tests) + `make runtime-rust-lint` green.
  - **Shared `string match` glob — ✅ done** (`tcl_syntax::glob`, the
    share-with-the-compiler tenet): one byte-exact mirror of
    `Tcl_StringCaseMatch` (`*`/`?`/`[a-z]` ranges/`\` escape/`nocase`/unclosed-`[`)
    for every consumer. Converged **two** prior compiler copies onto it — the
    `matches_glob` const-fold (`tcl_expr_eval.rs`) and the `switch -glob` fold
    (`structure_elimination.rs`) — and the runtime's `namespace export`/`import`/
    `forget` use it. (`string match`/`lsearch -glob`/`array names` land on it next.)
  - **Variable-namespace side — ✅ done** (`vars.rs` + `cmd_var.rs`, the
    variable parallel of the command resolver; `tclVar.c:TclLookupSimpleVar` +
    `namespace-tree.md` §5.3). Variables live in **per-namespace var tables**
    (`Namespace.vars`; the global ns holds globals) instead of a flat per-frame
    map. A `VarTable` (name→`Var` cell + scalar/array/element ops + the refcount
    discipline + release-on-`Drop`) is shared by both a call `Frame` and a
    `Namespace`. `Var::Link` generalised to a **`VarHome`** (frame level **or**
    namespace id), so `global`/`variable`/`upvar` all produce one link shape
    (level-0 frame ⇒ global ns, since they share a table). One classification
    (qualified → namespace; else in-proc → frame-local, at global/`namespace
    eval` scope → current ns) + one cross-table link walk; `set ::ns::x`,
    `$::ns::x`, `unset ::ns::x` resolve through the tree, and `::pinged` ≡
    `pinged` at top level (the headline fix — before, `::pinged` was a literal
    frame key). `global`/`variable` (`cmd_var.rs`) link the tail to a namespace
    var (no-op at namespace scope; `variable name value` still initialises);
    `upvar ?#N|N? other local` links to a caller frame or, qualified, a
    namespace var. `set`-into-a-missing-namespace raises `parent namespace
    doesn't exist` (reads/unsets just miss) — verified vs tclsh 9.0. The
    `::`-qualifier split is the shared `tcl_syntax::naming::qualifier_segments`.
    Gate: `make runtime-rust-test` (142 tower / 117 reduced) + `-lint` green.
  - **`::tcl::mathfunc::*` / `::tcl::mathop::*` as overridable commands — ✅ done**
    (`cmd_mathfunc.rs` / `cmd_mathop.rs`, tower-gated; the A3 contract).
    `::tcl::mathfunc::NAME` is one builtin per function forwarding to the shared
    `tcl_syntax::expr::mathfunc::dispatch`; `expr`'s function-call path resolves
    it through the command table first (absolutely anchored, so overrides /
    `rename` win — `expr`'s `call` hook now goes through `ExprCtx::call_function`,
    falling back to the shared dispatch only standalone). A missing function gives
    C's `invalid command name "tcl::mathfunc::NAME"`. `::tcl::mathop::OP` registers
    every operator with variadic-fold / identity / chained-comparison / arity
    semantics over the same tower ops; per A3 these are **commands only** —
    `expr`'s inline `arith` is unchanged. All verified vs tclsh 9.0.
  - **Ensembles — ✅ done** (`ensemble.rs` + `cmd_namespace.rs` + the
    `Command::Ensemble` trampoline in `interp.rs`). The canonical `ens sub`→
    target redirect (the generalised `dict for`→`::tcl::dict::for` rewrite, A3):
    `namespace ensemble create ?-command? ?-map? ?-subcommands? ?-prefixes?` +
    `exists`. Dispatch picks the subcommand set (explicit `-subcommands`, else
    `-map` keys, else the namespace's exported commands), resolves it (exact then
    unambiguous prefix unless `-prefixes 0`), maps to the target (`-map` entry or
    `<ns>::<sub>`), and re-dispatches; `unknown [or ambiguous] subcommand` errors
    match tclsh. Same build/dispatch split as `interp alias`. (`namespace
    ensemble configure` is a follow-up.)
  - **Remaining:** `rand`/`srand` (interp RNG state), `namespace delete`,
    `namespace ensemble configure`, and per-frame `current_ns` + the proc-local
    var branch (wired in `vars.rs` but inert — a proc runs in its defining
    namespace) — gated on the proc chunk (which pushes the proc frames).
- **T1.6 — builtins.** Port `cmds/*.zig` incrementally (string/list/dict/expr/
  control-flow/proc/…), each command (or small group) one PR with its tcltest
  delta. The value-type chunks (list/dict/string/array) each carry a
  [representation-decision note](#choosing-algorithms--data-structures-the-porting-method).
  Procs followed a design, not started blind:
  [`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md) fixes the
  call protocol (the CallFrame + CmdFrame stacks), the exception/return-options
  model, stack-trace construction, and AOT↔interp interop — built on the
  conservative-first principle and "get the dynamic cross-scope core
  (`uplevel`/`upvar`/`namespace`/`eval`) correct, then optimise". The proc
  chunk follows that doc's PC-1..PC-7 plan.
  - **Status (ahead of the prose above):** the bulk of T1.6 has landed —
    string/list/dict/array/control-flow/info/scan/format/chan/trace/package
    builtins, plus the **proc chunk PC-2/PC-3** (`proc`/`apply`, `uplevel`/
    `upvar`/`global`/`variable`, `info level`, `catch`/`error`/`try`/`throw`)
    **PC-1/PC-4 — faithful `::errorInfo` stack traces** (the incremental
    `while executing` / `invoked from within` / `(procedure "x" line N)`
    unwinder, byte-verified vs tclsh 9.0), and **PC-5 — `info frame` + `source`
    frames** (the persistent `CmdFrame` stack: `type`/`line`/`cmd`/`proc`/
    `file`/`level`, byte-verified vs tclsh 9.0 — including file-absolute lines
    for source-defined procs and the `uplevel`/`eval`-body cases). Remaining
    proc-chunk items: `return -options` errorinfo restore, and the
    expr/`foreach`/`eval`-body **bytecode-boundary** trace approximations noted
    in `proc-call-and-stack-traces.md` §8. `info errorstack` (TIP 348) is
    **out of scope** — its `INNER` element exposes tclvm bytecode opcodes a
    WASM-targeting runtime cannot reproduce (the bytecode/disassembly exclusion
    class).
- **T1.7 — re-export the codegen ABI.** The AOT codegen imports a fixed set of
  `tcl_*`/`obj_*` primitives; the Rust runtime must export the same names/sigs
  so the parity check and the compiled-script harness stay green. Also the wasm
  build: exported `memory` + growable `__indirect_function_table`, and the
  `cfg(target_arch="wasm32")` `size_of::<TclObj>() == 24` layout assert.

**Track 1 gates:** `make check-wasm-parity` green; the Tcl 9 suite
(`scripts/run_tcl9_tcltest_sweep.py`) + leak-check
(`scripts/leak_sweep.py` / `make leakcheck`) do **not** regress vs the Zig
baseline.

---

## Track 2 — Production C dynamic-linking interface

Promotes the spike into shipped infrastructure. Tracks the open items in
[`c-extension-abi.md`](c-extension-abi.md) §13 — flip them here as they land.

### T2.1 — C-API ownership / error contract (§13.1) — **land first**

A contract doc (sibling to `refcount-contract.md`) that, for every public
C-API function we ship, states its refcount category (callee-consumes /
callee-borrows / returns-owned-`+1` / returns-borrowed) and error-path
behaviour (`errorCode`/`errorInfo`/`Tcl_SetErrorCode`/return codes),
transcribed from `tmp/tcl9.0.3/doc/*.3` + the C source, mapped onto
`refcount-contract.md`. Plus a **gate that rejects a new C-API export lacking
an ownership annotation**.

- Status: **partial — contract doc + gate landed; runtime impl pending.**
  - [`c-api-ownership-contract.md`](c-api-ownership-contract.md) annotates all
    81 shipped C-API functions (the `tcl.h`/`tclOO.h`/`tclTomMath.h` surface)
    with an ownership category **and** an error-path category. It fixes the
    **`fresh_zero`** convention (C-API constructors return refCount **0**,
    unlike the internal `obj_new_*` which return rc=1) — the single biggest
    extension-author correctness subtlety.
  - `scripts/check_c_api_ownership.py` (+ `make check-c-api-ownership`, wired
    into `_prep-pr-checks-noty`) is the parity-style gate: a header-declared
    C-API function with no contract row — or a stale row — fails prep-pr. It
    correctly excludes `#define` macros and the nominal stub-table data
    symbols.
  - **Remaining:** encode the categories in the `runtime/rust/` C-API impls and
    extend the gate to cross-check the real `#[no_mangle] extern "C"` exports
    once they land.
- Acceptance: every shipped C-API function carries an ownership category
  (**done**, gated); the round-trip extension (`Tcl_NewObj` →
  `Tcl_IncrRefCount` → `Tcl_SetObjResult` → `Tcl_DecrRefCount`) shows zero
  residual under the `-Dleak-check` counter (**pending the impl**).

### T2.2 — Shipped headers (§4.1, §7, §11)

Promote `runtime/rust-spike/include/{tcl.h,tclOO.h,tclTomMath.h}` to shipped
headers, widened to the full public-survey surface, backed by real impls. Ship
the full versioned `Tcl_ChannelType` / `Tcl_Filesystem` / `Tcl_ObjType` bodies
(the spike carries only probed fields). Status: **not-started.**

### T2.3 — Production dynamic loader (§5.2, §11)

Move the loader from the Python spike into the runtime/host. Parse `dylink.0`;
allocate `__memory_base`/`__table_base` from shared memory + the growable
table; resolve `GOT.mem.*` / `GOT.func.*` (the 4 `pkgooa` symbols characterise
the space — address-of-runtime-symbol); run `__wasm_apply_data_relocs` +
`__wasm_call_ctors` + `Foo_Init`. Status: **not-started.**

### T2.4 — Real-compiler dispatch (§13.2)

AOT-compiled user code resolves and calls an extension-registered command via
the runtime command table. Add the "register external command → shared-table
index" entry the dispatch needs. Status: **not-started.**

### T2.5 — Nominal stub tables (§6)

A real struct populated with our function pointers for the rare
stubs-introspection pattern (`pkgooa.c`). Status: **not-started.**

---

## Track 3 — AOT-first execution & whole-program link

Make the AOT compiler the primary path and link the whole program (runtime +
compiled user code + C extensions) into one artifact.

### T3.0 — Codegen command registry + backend-agnostic emit protocol — **foundational**

AOT codegen has to know **how to emit each Tcl command** into the target
instruction stream, and today that per-command knowledge is split across
backend-specific code (`core/compiler/codegen/bytecoded/` for the tclvm
bytecode VM, `core/compiler/codegen/wasm/` for WASM, each with its own
`_emitter` / `_imports` / `_statements`). The two emitters re-derive the same
command semantics independently, which is exactly the kind of drift the parity
gate exists to catch — but parity is a *cross-check*, not a *shared source*.

The port needs a **command-emission registry** distinct from the existing
command **spec** registry (`core/commands/registry/tcl/`, which is dialect/lint
metadata): a registry keyed by command (and sub-command) whose entries describe
how to lower that command, behind a **single backend-agnostic emit
protocol/trait** so one registration can target **any** backend:

- **tclvm** — the existing bytecode VM (`codegen/bytecoded/` → `opcodes.py`).
- **wasm** — the AOT WASM emitter (`codegen/wasm/`), the north-star path.
- **llvm ir** — a future native/JIT backend.

Shape (Rust trait, mirrored by the Python transitional surface):

```
trait CommandEmitter {                  // one impl per backend
    fn emit_call(&mut self, cmd: &ResolvedCommand, args: &[IrValue]) -> EmitResult;
    fn emit_builtin(&mut self, op: BuiltinOp, ...) -> EmitResult;   // set/incr/expr/list/...
    fn emit_dispatch_fallback(&mut self, name: &IrValue, argv: &[IrValue]) -> EmitResult;
}
// CommandEmitRegistry: command -> lowering rule, parameterised over the backend.
```

Each command registers its lowering **once**, against the trait; the WASM,
tclvm, and (future) LLVM backends are interchangeable implementations of the
trait. This is the codegen-side analogue of the runtime's "one command table":
the AOT compiler resolves a command to a lowering rule the same way the runtime
resolves it to a `CmdEntry`, and an extension-registered command (no static
lowering) falls through to `emit_dispatch_fallback` → the runtime command table
(§4.6 in `c-extension-abi.md`), which is also where the metaprogramming-S7
fallbacks land.

**Tie it to the editor command registry (single source of truth).** The
emit-lowering rule must be **bound to the same command registry the editor
uses** (`core/commands/registry/tcl/` — the spec/lint/hover/completion data),
so the set of commands the editor knows about and the set the compiler can emit
**cannot drift**. Preferred shape: the lowering rule *lives in* (or is
registered against) that registry as one more facet of a command's entry —
alongside its signature/dialect/lint metadata — rather than in a parallel table
that has to be kept in sync. This makes the existing `make check-wasm-parity`
cross-check a *consequence* of one source of truth rather than the thing holding
two tables together.

**Not every command has an emit impl yet — that's an explicit, well-formed
error.** A command can exist in the registry (so the editor lints/completes it)
without yet having a lowering rule for a given backend. Compiling a script that
*uses* such a command must raise a **clear compile-time error/exception**
(e.g. `NoEmitImpl{ command, backend }`) naming the command and backend — never
a silent miscompile, a panic, or a fallthrough that pretends success. This is
the codegen analogue of the runtime's trapping stub
(`dispatch/tcl_stub_fallback.zig`): a registry entry with no backing emitter is
a known-missing capability, surfaced loudly, and is distinct from an
*extension-/runtime-registered* command (which legitimately has no static
lowering and instead routes through `emit_dispatch_fallback` → the runtime
command table). The two must not be conflated: "no emitter for a builtin we
should support" is an error to fix; "no static lowering for a dynamically
registered command" is the designed dispatch path.

- Status: **not-started.** Today's per-backend emitters are the starting point;
  T3.0 factors their shared command knowledge behind the trait and binds it to
  the editor command registry.
- Why it belongs in this effort: AOT-first means the WASM emitter is the primary
  path, and linking C extensions adds a *third* class of command (runtime-/
  extension-registered) the emitter must dispatch uniformly. A backend-agnostic
  registry keeps tclvm (the oracle), wasm (the target), and a future llvm-ir
  backend emitting from **one** source of per-command lowering truth instead of
  N drifting copies guarded only by the parity cross-check — and binding it to
  the editor registry guarantees editor/compiler alignment by construction.
- Gate: WASM and tclvm backends emit from the shared registry with
  `make check-wasm-parity` green and no tcltest regression; the trait has ≥2
  live backend impls (wasm + tclvm) so the abstraction is proven, not
  speculative, before an llvm-ir impl is attempted; compiling a script that uses
  a command with no lowering rule for the active backend raises the
  `NoEmitImpl{ command, backend }` error (covered by a test), not a silent
  miscompile.

### Remaining Track-3 chunks

- **T3.1 — extension linking in `wasm_link.py`.** Extend
  `core/compiler/codegen/wasm_link.py` to also link extension objects — static
  Model A where possible, dynamic Model B otherwise.
- **T3.2 — drive AOT coverage up the staircase** so non-metaprogramming
  programs compile **100% AOT** (interpreter fallback never reached). Track in
  the [AOT-coverage scoreboard](#aot-coverage-scoreboard) below.
- **T3.3 — S7: metaprogramming heuristics (new staircase stage, beyond S6).**
  Heuristics that AOT-compile common metaprogramming patterns — `eval`/`subst`
  of statically-known scripts, list-built command/arg construction, constant
  `uplevel`/`upvar`/`namespace` forms — each **proven-safe or it falls through
  to the interpreter** (staircase rule: emit static WASM only where behaviour is
  provable, else fall back). Spec as a new `wasm-aot-staircase-s7.md` stage doc.

### AOT staircase context

S0–S6 are landed/partial on the compile side (see
[`wasm-aot-staircase.md`](../compiler/wasm-aot-staircase.md) stage skeleton):
S0–S2 landed, S3 partial, S4–S6 landed. **S7 (metaprogramming heuristics) is
the new stage this effort adds.** It obeys the same staircase rule — static
WASM only where provable, else interpreter fallback — so it never regresses
correctness, only widens the AOT surface.

### AOT-coverage scoreboard

Share of a representative corpus that **fully AOT-compiles** (zero interpreter
fallback at runtime). Seeded empty — baseline to be captured once T3.1 lands the
measurement harness.

| Corpus | Fully-AOT share | Falls back (why) | Notes |
|---|---|---|---|
| _seed — to be captured_ | — | — | establish baseline with T3.1 |

#### Metaprogramming-heuristic backlog (S7)

| Pattern | AOT heuristic | Fallback trigger | Status |
|---|---|---|---|
| `eval`/`subst` of statically-known script | compile the known script inline | non-constant script body | not-started |
| list-built command/arg construction | resolve the command + args at compile time | dynamic command name | not-started |
| constant `uplevel`/`upvar` forms | static frame resolution | dynamic level/var name | not-started |
| constant `namespace eval` body | compile body in target ns | dynamic ns name | not-started |

---

## Extension tier gates

Each tier is a PR series: vendor real extensions byte-identical with
provenance/licence, extend the compile-check, add LOAD+RUN tests under
`wasmtime`. Never merge a tier without its gate green.

### Tier 0 — in-tree dltest (9 samples)

All 9 `tmp/tcl9.0.3/unix/dltest/` samples LOAD and RUN: `pkga`, `pkgb`, `pkgc`,
`pkgd`, `pkge`, `pkgt`, `pkgua`, `pkgπ`, `pkgooa`. (`embtest.c` excluded — it
*embeds* Tcl, the opposite of extending it.)

| Sample | Exercises | LOAD | RUN |
|---|---|---|---|
| `pkga` | command/obj/result/UTF core | ☐ | ☐ |
| `pkgb` | int/wide accessors, `Tcl_AppendResult`, `Tcl_EvalEx` | ☐ | ☐ |
| `pkgc` / `pkgd` | int accessor + string/int obj results | ☐ | ☐ |
| `pkge` | error-returning init | ☐ | ☐ |
| `pkgt` | Tcl 9 `Tcl_*ObjCmd2` (`Tcl_Size` arity) | ☐ | ☐ |
| `pkgua` | load/unload + hash tables + thread-data | ☐ | ☐ |
| `pkgπ` | non-ASCII init naming | ☐ | ☐ |
| `pkgooa` | the GOT path + nominal stub table | ☐ | ☐ |

Status: **not-started** (spike compiles them; production LOAD+RUN not yet).

### Tier 1 — small real extensions (libc-only)

| Extension | Exercises | package-require + round-trip |
|---|---|---|
| Memchan | channel driver API (needs `io/tcl_chan` port) | ☐ |
| tclvfs | `Tcl_FSRegister` (needs `io/tcl_fs` port) | ☐ |
| tcllib critcl digest (sha1c/md5c) | custom `Tcl_ObjType` + byte arrays | ☐ |

Status: **not-started.** Large prerequisite surfaces (channels, VFS) land as
their own gated PRs first.

### Tier 2 — flagship sqlite3/tclsqlite

Acceptance: `package require sqlite3; sqlite3 db :memory:; db eval {create
table t(x); insert into t values(42); select x from t}` returns `42` under
`wasmtime`, against the **Rust** runtime via the loader — with the surrounding
script **AOT-compiled**. (amalgamation already builds to WASM; `tclsqlite.c` is
`tcl.h`-only.) Prerequisite: eval-loop depth for `db eval`, landed as its own
gated PR. Status: **not-started.**

---

## Tcl 9 test-suite scoreboard (gold standard)

`tmp/tcl9.0.3/tests/*.test` (168 files). The **Zig/WASM** backend is swept by
`scripts/dev/run_tcl9_tcltest_sweep.py`; the **Rust interpreter** is swept by
`scripts/dev/rust_tcltest_sweep.py`, which sources each file through the
`run_script` example (real `tcltest` loads via `--init`/`init.tcl`) and parses
tcltest's own `Total/Passed/Skipped/Failed` summary. **In scope: behaviour.**

### Rust runtime baseline — 2026-06-10 (first sweep)

The Rust interpreter runs the real Tcl 9 suite end-to-end (real `tcltest`
2.5.10). First measured baseline, then the same day's two unblocking fixes
(idempotent `namespace import` re-import + `tcl::build-info`):

| Sweep | Files run-to-summary | Errored before summary | Tests passed |
|---|---|---|---|
| Initial baseline | 86 / 168 | 81 | 5572 / 11022 |
| + reimport / build-info | 94 / 168 | 73 | 6139 / 12299 |
| + totitle / nsdelete / file / pkg-require-global | 105 / 168 | 62 | 6830 / 14079 |
| + panic fixes / qualified-name fallback | 110 / 168 | 57 | 7214 / 15345 |
| + `binary format`/`scan` | 116 / 168 | 51 | 8325 / 17673 |
| + child interpreters | 118 / 168 | 49 | 8406 / 17855 |
| + TclOO core | 120 / 168 | 47 | 8415 / 18298 |
| + TclOO expand / info prefix / hidden+safe | 122 / 168 | 45 | 8529 / 18775 |
| + cross-interp aliases | 122 / 168 | 45 | 8546 / 18775 |
| + auto-load fixes + re-entrant Safe Base | **124 / 168** | 43 (+1 timeout) | **8654 / 18939** |
| + `ledit` + three-way var-read-miss error | **124 / 168** | 43 (+1 timeout) | **10448 / 18939** |
| + `lmap` + empty-script result reset | **125 / 168** | 43 (0 timeout) | **10566 / 19027** |
| + `lseq` (arithmetic-series generator) | **125 / 168** | 43 (0 timeout) | **10660 / 19027** |
| + `trace` command/execution/step + lifecycle | **125 / 168** | 43 (0 timeout) | **10818 / 19027** |
| + `lset` (list-element set in a variable) | **126 / 168** | 42 (0 timeout) | **10870 / 19027** |
| + `lsort` options (`-stride`/`-index`/`-dictionary`/`-command`/`-indices`) | **128 / 168** | 40 (0 timeout) | **11116 / 19027** |
| + `lsearch` options (`-sorted`/`-index`/`-stride`/`-regexp`/`-subindices`/…) | **128 / 168** | 40 (0 timeout) | **11216 / 19027** |
| + `string` insert/replace/wordstart/wordend/compare-opts/is-dict + `tcl::prefix` | **128 / 168** | 40 (0 timeout) | **11408 / 19027** |
| + `binary encode`/`decode` (hex/base64/uuencode) + `u` scan modifier | **128 / 168** | 40 (0 timeout) | **11516 / 19027** |
| + `info cmdtype`/`cmdcount`/`functions`/`loaded` | **128 / 168** | 40 (0 timeout) | **11534 / 19027** |
| + OO object-lifetime sync on `rename`/delete | **128 / 168** | 40 (0 timeout) | **11536 / 19027** |
| + TclOO classes-as-objects (`oo::define … self`, class methods) | **128 / 168** | 40 (0 timeout) | **11542 / 19027** |
| + TclOO call-chain refactor + `filter`s + `info` oo subcommands | **128 / 168** | 40 (0 timeout) | **11546 / 19027** |
| + TclOO class-destroy cascades to subclasses | **128 / 168** | 40 (0 timeout) | **11557 / 19027** |
| + TclOO per-object `my` (not global) | **128 / 168** | 40 (0 timeout) | **11561 / 19027** |
| + TclOO `private` methods + `unknown` method-list message | **128 / 168** | 40 (0 timeout) | **11572 / 19027** |
| + TclOO `oo::object`/`oo::class` as real objects (uniform dispatch) | **128 / 168** | 40 (0 timeout) | **11576 / 19027** |
| + TclOO object built-ins (`my variable`/`varname`/`eval`) | **128 / 168** | 40 (0 timeout) | **11584 / 19027** |
| + TclOO `info object creationid` / `info class definitionnamespace` | **128 / 168** | 40 (0 timeout) | **11602 / 19027** |
| + TclOO define-subcommand abbreviation + `class`/`deletemethod`/`renamemethod` | **128 / 168** | 40 (0 timeout) | **11616 / 19027** |
| + TclOO `export` of built-ins + `info object`/`info class` abbreviation & subcommands | **128 / 168** | 40 (0 timeout) | **11624 / 19027** |
| + `info commands` namespace-qualified patterns (`::ns::glob`) | **129 / 168** | 39 (0 timeout) | **11662 / 20532** |

The 2026-06-13 **TclOO filters** chunk (**+4 tests over two commits, zero
regressions**) — refactored the method-call chain to a list of `(provider,
method)` steps (object-vs-class resolution by identity, not position), which is
behaviour-preserving and lets **filters** be modelled as steps whose method is
the filter name, prepended ahead of the target-method steps with `next`
advancing through the chain. Added the `filter` define-subcommand (class +
objdefine), `self target`, and the `info object mixins` / `info class
mixins`/`variables` introspection. `oo.test` 41 → 45.

The 2026-06-13 **TclOO classes-as-objects** chunk (**+6 tests, zero
regressions**) — a class is now also registered as an object, so `oo::define C
self method …` (define-context `self` routing to objdefine on the class) and
class methods (`C foo`) work, and a failed class-definition script rolls the
class back (clearing the dominant `oo.test` setup-cascade). Method resolution is
now **positional** (chain head = per-object methods, rest = class instance
methods) since a class lives in both registry maps. `oo.test` 35 → 41. Next:
filters, private methods, the remaining `info object`/`info class` subcommands.

The 2026-06-13 **OO rename/delete** fix (**+2 tests, zero regressions**) — an
OO object/class is tied to its command, so `rename obj {}` (the tests' cleanup
idiom) must drop it from the `OoState` registry (both the object and class
maps — a class is in both) and a rename must move it; otherwise the name could
not be recreated (the dominant `oo.test` setup-cascade) and a stale half-entry
could panic a later method dispatch (now a clean `object … has been deleted`).
The bulk of `oo.test` remains the TclOO **meta-protocol** (filters, private
methods, `my`/`self` subcommands, classes-as-objects, full C3 linearisation) —
the deferred 3-file blocker.

The 2026-06-13 **TclOO meta-protocol** chunks (**+13 tests over three commits,
zero regressions**, `oo.test` 56 → 71):
1. **class-destroy cascade** — `oo_destroy_class` now recursively destroys
   subclasses (classes listing this one as a superclass or mixin) before its
   instances, matching `TclOO`'s "a class's epoch invalidates its dependants";
   without it the dominant `oo.test` cleanup-cascade left half-deleted classes
   that could not be recreated (`oo.test` 45 → 56, sweep 11546 → 11557).
2. **per-object `my`** — C `TclOO` creates `my` in each *object's* namespace,
   not globally; the tests' cleanup idiom `catch {rename ::my {}}` previously
   deleted our single global `my` and broke every later object. `my` is now
   registered as `<fqn>::my` per object (`oo.test` 56 → 60, sweep 11557 →
   11561).
3. **`private` + `unknown` method list** — the `private` define-subcommand and
   `method -private`/`-export` flags mark a method unexported (`o secret` →
   unknown, `my secret` → works), and an unknown method now emits the C
   `unknown method "X": must be a, b or destroy` enumeration (sorted, non-Oxford
   join; classes also list `create`/`new`). Byte-identical to `tclsh9.0`
   (`oo.test` 60 → 71, sweep 11561 → 11572).
4. **base classes as real objects** — `oo::object`/`oo::class` are now full
   objects (in both the `objects` and `classes` maps) that dispatch through the
   normal `oo_dispatch` path rather than dedicated builtins, so `create`/`new`/
   `destroy`, the empty-name check (`object name must not be empty`), the
   `wrong # args: should be "<cmd> method ?arg ...?"` / `"<cmd> create
   objectName ?arg ...?"` usages, and unknown-method enumeration are all handled
   uniformly. `::oo::class` is a singleton (`new` unexported → lists only
   `create or destroy`); an object whose only method (`destroy`) is unexported
   reports `object "X" has no visible methods`; construction with no constructor
   silently ignores extra args (matching `tclsh9.0`). (`oo.test` 71 → 75, sweep
   11572 → 11576).
5. **object built-in methods** — the unexported `oo::object` methods `variable`
   (link instance variables into the calling method frame, rejecting `::`-
   qualified names), `varname` (the fully-qualified instance-variable name) and
   `eval` (evaluate a script in the object's namespace), reachable only via
   `my`. (`oo.test` 75 → 83, sweep 11576 → 11584).
6. **`creationid` / `definitionnamespace` introspection** — `info object
   creationid` (a unique, monotonic per-object ID stable across rename) and
   `info class definitionnamespace … ?-class|-instance?` (TIP 524; the built-in
   `::oo::define`/`::oo::objdefine` defaults). Also corrected the
   `does not refer to an object`/`is not a class` messages (no surrounding
   quotes for the not-an-object case, resolving the object before the class as
   C does) across `oo::define`/`oo::objdefine`/`info object`/`info class`.
   (`oo.test` 83 → 101, sweep 11584 → 11602).
7. **define-subcommand surface** — definition bodies now resolve an unknown
   leading word as C's define ensemble does: an exact name or a unique prefix
   (`super` → `superclass`, `forw` → `forward`, `meth` → `method`; an ambiguous
   prefix like `m` stays an error). Adds the `deletemethod`/`renamemethod` and
   (objdefine) `class` subcommands, and fixes `self { script }` in a definition
   to evaluate the body. (`oo.test` 101 → 115, sweep 11602 → 11616).
8. **`export` of built-ins + `info` introspection** — `export` now promotes a
   default-unexported built-in (`eval`/`variable`/`varname`) to a public method
   (tracked in a per-target `exported` set). `info object`/`info class` resolve
   abbreviated subcommands (exact or unique prefix) and emit C's `unknown or
   ambiguous subcommand "X": must be …` message; added the `forward`/`filters`/
   `definition`/`methodtype` subcommands (`call`/`properties` still deferred).
   (`oo.test` 115 → 123, sweep 11616 → 11624).

The 2026-06-13 **`info commands` namespace-qualified patterns** fix (**+38
tests, zero regressions**) — `info commands ::ns::glob` now resolves the
namespace qualifier, lists that namespace's commands and matches the tail glob,
re-qualifying the results (the C behaviour); an unqualified pattern keeps the
current+global visible-command listing. This unblocked **apply.test** (0 → 21,
it errored at setup), **safe.test** (52 → 61), `namespace-old.test` (+3),
`info.test`/`oo.test`/`tm.test`/`safe-stock.test`; sweep **11624 → 11662**.

The 2026-06-13 **`info`** increment (**+18 tests, zero regressions**) — the
`info cmdtype`/`cmdcount`/`functions`/`loaded` subcommands (`tclCmdIL.c`). The
bulk of `info.test`'s remainder is `info frame` source-location exactness
(PC-5, deferred) and the 5 missing `::tcl::mathfunc` names that `info functions`
lists. `info.test` 81 → 99.

The 2026-06-13 **`binary`** increment (**+108 tests, zero regressions**) —
`binary encode`/`decode` for `hex`/`base64` (`-maxlen`/`-wrapchar`/`-strict`)/
`uuencode` (`tclBinary.c`), plus the `u` unsigned modifier on integer `binary
scan` codes. Byte-identical to `tclsh9.0`. `binary.test` 308 → 404.

The 2026-06-13 **`string`-surface** increment (**+192 tests, zero regressions**)
— `string insert`/`replace`/`wordstart`/`wordend` subcommands, `-nocase`/`-length`
on `string compare`/`equal`, the `dict` class and class-named usage for `string
is`, the `::tcl::string::insert` direct command, and the `tcl::prefix
match`/`all`/`longest` command (`tclIndexObj.c`). Byte-identical to `tclsh9.0`.
`string.test` 372 → 552.

The 2026-06-13 **`lsearch`-options** increment (**+100 tests, zero
regressions**) — completed `lsearch` from the `-exact`/`-glob`/`-nocase`/`-all`/
`-not`/`-inline` subset to the full `Tcl_LsearchObjCmd`: the `-sorted` binary
search (+`-bisect`, `-increasing`/`-decreasing`), datatypes `-ascii`/
`-dictionary`/`-integer`/`-real` (numeric elements validated lazily, as C does),
`-regexp` (via the runtime regex engine), `-index` (nested key), `-stride`
(+leading-`-index` group offset), `-start`, and `-subindices`. Shares
`dictionary_compare`/`select_by_index`/`index_spec` with `lsort`. Byte-identical
to `tclsh9.0`. `lsearch.test` 30 → 130.

The 2026-06-13 **`lsort`-options** increment (**+246 tests, zero regressions**)
— completed `lsort` from the `-ascii`/`-integer`/`-real`/`-nocase`/`-unique`
subset to the full `Tcl_LsortObjCmd` switch set: `-dictionary` (ported
`DictionaryCompare` — case-insensitive, embedded decimals compared numerically,
leading-zero secondary tiebreak), `-index` (drill into each element by a nested
path, `SelectObjFromSublist`), `-stride` (group the flat list; the leading
`-index` value, default 0, picks the key element within each group and the rest
of the path applies inside it; output regroups), `-indices` (return positions),
and `-command` (a stable merge sort whose comparator evals the user prefix —
reentrant, so not a plain `sort_by`). Byte-identical to `tclsh9.0` incl. all the
`stride length`/`multiple of the stride`/`within the group`/`missing from
sublist` error strings. `error.test` 123 → 261 (its `lsort -stride 2` errorcode-
normalising `customMatch`), `cmdIL.test` 48 → 125, `lsearch.test` 0 → 30.

The 2026-06-13 **`lset`** increment (**+52 tests, zero regressions**) — the
in-variable nested list-element set (`Tcl_LsetObjCmd`/`TclLsetList`/
`TclLsetFlat`, `tclListObj.c`): `lset listVar ?index ...? value`. A lone index
arg is an index *path* (`lset x {1 0} v`), multiple args each one index; each
index resolves against its sublist length (`end`/`end±N`, range `0..=len` with
`len` appending), descending and rebuilding the nested list bottom-up
(`cmd_list.rs::lset_descend`, sharing `index_spec`/`bad_index` with
`lindex`/`ledit`), then storing it back through `var_set` (so write traces fire)
and returning it. Byte-identical to `tclsh9.0` incl. the empty-list/append
quirks (single-element sublists stringify without braces, so `lset x 0 0 Z` on
`{}` → `Z`). `reg.test` 0 → 32 (now runs to a summary), `lsetComp.test` 2 → 19;
`lset.test` stays constraint-skipped (`testevalex`, a `tcl::test` C command).

The 2026-06-13 **`trace`** increment (**+158 tests, 56.0% → 56.9%**, zero
regressions) — `trace.test` **49 → 195** (the rest are
`tcl::test`/`testcmdtrace` C-tier commands, the `after`/`update` event loop, and
`const`, all out of this command's scope):

- **command + execution traces** (`cmd_trace.rs`, `tclTrace.c`'s
  `Tcl_TraceObjCmd` + the three type helpers): `trace add|remove|info
  command|execution`. Command traces (`rename`/`delete`) fire from
  `rename_command` *before* the table mutation as `command oldName newName
  rename` / `command oldName {} delete`, following a renamed command and dying
  on delete (`Interp::fire_cmd_trace`). Execution `enter`/`leave` wrap the
  dispatch chokepoint (`dispatch → dispatch_traced → dispatch_inner`):
  enter newest-first, leave oldest-first, `<prefix> {cmd args} [<code>
  <result>] <op>`, with enter-error abort and leave-error override; the result
  is saved once and restored after the callbacks but live between them (C's
  single `SaveInterpState`/`RestoreInterpState`). Keyed by resolved FQN
  (`resolve_cmd_fqn`); registry is a `CmdTrace` Vec with a `TraceOps` bitset.
- **step traces** (`enterstep`/`leavestep`): a command carrying step ops
  installs a `StepActive` on entry (deduped against recursion so only the
  outermost installs); while any is live, every executed command fires
  enterstep (reverse) before and leavestep (forward) after — matching C's
  interp-trace order (interp enter before per-command enter; per-command leave
  before interp leave). Byte-identical to `tclsh9.0` on the recursive-factorial
  and nested-error step scenarios.
- **variable-trace error propagation** (`fire_var_trace` → `pending_err`): a
  `read`/`write` callback error now fails the access with `can't read|set
  "name": <msg>` (C's `TclObjCallVarTraces` + `TclObjVarErrMsg`); `unset`/`array`
  errors stay swallowed. Write routes through a new unit `VarError::TraceError`
  (keeps `VarError` `Copy`; the ~40 `var_error` callers propagate unchanged);
  read routes through `fire_read_trace` at the `$var` and `set name` chokepoints.
- **trace lifecycle** (matches C; prevents unbounded accumulation that otherwise
  poisoned later tests into exponential step output): redefining a command
  (`proc`) deletes the old one — fire its `delete` command-traces and drop all
  its traces (`Tcl_CreateObjCommand` replace); unsetting a variable drops its
  variable traces; and a **proc-local** variable's traces die when the call
  frame pops (`VarTrace::frame_level` + `clear_frame_var_traces`, C frees a
  local var's trace list at frame teardown).

The earlier 2026-06-13 increments (**+2006 tests, 45.7% → 56.0%**, zero
regressions):

- **`ledit listVar first last ?element ...?`** (`cmd_list.rs`) — the Tcl 8.7/9.0
  in-place `lreplace` on a list *variable*. Shares `lreplace`'s index/clamp/
  replace logic (mirroring the Zig oracle's shared `do_lreplace`, `cmds/list.zig`,
  and C's `Tcl_LeditObjCmd`, `tclCmdIL.c`): read the var (error on a miss, which
  C does via `TCL_LEAVE_ERR_MSG`), splice the `[first,last]` range, store the new
  list back, return it; addresses `a(k)` array elements like `set`/`lappend`.
  `lreplace.test` **1790 → 3578 / 3579** (the lone residual is a pre-existing
  backslash-trailing-space list-quoting edge in `Tcl_ConvertElement`, shared by
  all list rendering — not a `ledit` bug).
- **three-way variable-read-miss error** (`interp.rs::read_miss_msg`, routed from
  `set`/`ledit`/`expr $var`) — `tclVar.c`'s distinction: a scalar read of an
  array is `variable is array`, a missing element of an *existing* array is `no
  such element in array` (previously wrongly `no such variable`), and a wholly
  missing variable is `no such variable`. Lifted `set.test`/`set-old.test`/
  `trace.test` (+6 beyond `ledit`).
- **`lmap varList list ?varList list ...? body`** (`cmd_control.rs`) — `foreach`
  that collects each non-`continue` body result into a list (refactored `foreach`
  into a shared `each_loop(collect)` engine, mirroring C's one `EachloopCmd`,
  `tclCmdAH.c`). `lmap.test` **0 → 57 / 66**.
- **empty-script result reset** (`interp.rs::eval_script`) — a script with no
  commands (empty / whitespace / comments only) now resets the result to `""`, as
  C's `Tcl_EvalEx` does; previously a stale prior result leaked through (surfaced
  by an `lmap` body of `{}`, also affects empty proc bodies / `eval {}`). Rippled
  +6 `uplevel.test`, recovered `for.test` from a timeout (+51), +1 each to
  `if`/`compile`/`execute`/`namespace-old`.
- **`lseq start ?(..|to)? end ??by? step?`** / `lseq start count count …` /
  `lseq count …` (`cmd_lseq.rs`, new module, `have_tommath`-gated) — the
  arithmetic-series generator, ported from C's `Tcl_LseqObjCmd` +
  `TclNewArithSeriesObj`: the argument-decode key, the `..`/`to`/`count`/`by`
  keywords, expression-valued arguments (via `eval_expr_obj`), int-vs-double
  selection, the `ArithSeriesLenInt`/`ArithSeriesLenDbl` length formula, and the
  `maxObjPrecision`/`ArithRound` double-precision matching (`lseq 0 0.5 by 0.1` →
  `0.0 0.1 0.2 0.3 0.4 0.5`). We materialise a concrete list (C's lazy abstract
  series is representation-only, incompatible-by-design) and cap at 100M elements
  with C's "max length of a Tcl list exceeded" rather than OOM-aborting on the
  multi-billion-element lazy-series tests. `lseq.test` **0 → 94 / 134** (the
  remainder are `tcl::unsupported::representation` / lazy-series / extreme-
  magnitude `1e50`-formatting cases — the last a pre-existing `tcl-syntax`
  `format_double` limitation, not `lseq`).

Cumulative: **+36 files** now run to a tcltest summary, errored-before-summary
**81 → 45**, **+2974 tests pass**, and **zero panics** (the passed *count*
matters more than the ~47% rate — the denominator grows as more files run their
full test sets). The unblocking fixes:

- idempotent `namespace import` re-import (same source ⇒ no-op);
- `tcl::build-info` (the tcltest constraint source);
- `string totitle` + the `?first? ?last?` range form for `toupper`/`tolower`;
- `namespace delete` (arena tombstoning of the subtree);
- `file delete`/`mkdir`/`size`/`type`/`pathtype`/`executable`;
- `package require` evaluates its load script at global scope (C's `uplevel
  #0`), so a package's `namespace eval foo` creates `::foo` even when required
  from inside a namespace;
- three panics → clean errors: `format` width overflow (`max size for a Tcl
  value exceeded`), `proc` `args`-split with all-defaulted positionals, and a
  required parameter after a defaulted one (`wrong # args`);
- relative qualified command names fall back to the global namespace (so
  `tcl::build-info` resolves from inside a namespace);
- `binary format`/`binary scan` (the core type codes — see `cmd_binary`);
- basic child interpreters (`interp create`/`eval`/`exists`/`children`/`delete`
  + the child as a command), each a full `Interp` with startup globals;
- **TclOO** (`cmd_oo`): `oo::class`/`oo::object`/`oo::define`/`oo::objdefine`/
  `oo::copy`, methods + `forward`, constructor/destructor, single/multiple
  inheritance + `mixin` over a linearised dispatch chain, `export`/`unexport`,
  `self`/`my`/`next`, per-object methods/mixins/instance-variables, and `info
  object`/`info class` introspection (oo.test 9 → 31). `package require
  tcl::oo`/`TclOO` succeed.
- `info` is an ensemble (unambiguous-prefix subcommands) — `info command` →
  `commands` (unblocks interp.test, 48/354).
- hidden commands (`interp hide`/`expose`/`invokehidden`/`hidden`, + the
  `$child` forms) and `interp create -safe` (hides the host-touching commands
  the runtime has);
- **cross-interp aliases** — a child alias delegating to a parent command
  (`interp alias child name {} target …`, `$child alias name target …`).
  interp.test 79 → 94.
- **library auto-loading fixes** — five conformance bugs that blocked the Safe
  Base's pure-Tcl libraries (tm.tcl, safe.tcl, opt) from auto-loading on demand:
  `namespace ensemble create -command NAME` now qualifies a relative `NAME`
  against the current namespace (so `tcl::tm::path` binds at the right FQN and
  `auto_load`'s `namespace which` check passes); `lappend a(k)` addresses array
  elements; index specs accept the full `integer±integer` grammar (`string range
  $s 0 $last-1`); `namespace upvar` is implemented; `glob -types` filters by
  file-kind/permission.
- **re-entrant Safe Base (idiomatic, sound, lock-free)** — the cross-interp eval
  engine supports genuine parent⇄child recursion (a child's aliased `source`
  calls back into the parent, which calls `interp invokehidden $child …` back
  into the *same* child while its outer eval is on the stack — exactly C's nested
  `Tcl_Eval`). This is sound **by construction**, not by a bounded raw-pointer
  hack: `Interp` is a cheap `Rc<InterpState>` handle and **every field of
  `InterpState` is interior-mutable** (`RefCell`/`Cell`), borrowed only for the
  span of a single operation — never across a sub-eval (the command resolver
  already returns *cloned* `Command` handles so dispatch holds no table borrow).
  Re-entry into an interp clones its handle (an `Rc` bump) and reaches the shared
  state through `Rc` + `RefCell`, so there is **no aliased `&mut`** — a discipline
  slip is a clean panic, never UB (Miri-clean for the interp layer). Children are
  `RefCell<BTreeMap<…, Interp>>`; the parent link is a `Weak<InterpState>` (no
  ownership cycle). Single-threaded throughout — `Rc`/`RefCell`/`Cell`, no locks.
  `CROSS_INTERP_DEPTH` survives only as a native-stack bound. A child deleted
  *during* its own eval (the self-deleting `exit`→`interp delete` alias) has its
  teardown **deferred** (`eval_active`/`pending_delete`) until the eval unwinds
  (C's deferred `Tcl_DeleteInterp`). `safe::interpCreate`/`interpDelete` and the
  full Safe Base lifecycle work (safe.test: 0 run → **51 passed** of 155, no
  crashes; interp.test 94 → 104). `interp issafe`/`aliases` (+`$child` forms)
  added. The whole-runtime conversion (`&mut self` field access → interior
  mutability) is behavior-preserving: the sweep is byte-identical before/after
  (8654/18939), and all 237 lib tests + clippy/fmt stay green.

Biggest remaining error-before-summary blockers, by file count:

| Blocker | Files | Notes |
|---|---|---|
| TclOO meta-protocol | 3 | `oo`/`ooNext2`/`ooUtil`: classes-as-objects, filters, private methods (8.7+), full C3 mixin linearisation, the rarer `info object`/`class` subcommands |
| `zipfs` | 3 | zip virtual filesystem |
| `tcl::test` package | 2 | the C-tier test commands |
| `auto_load` in children | 2 | child interps lack the full `init.tcl` |

**Deferred:** the full Safe Base (`safe.tcl` — `source`/`load`/`file`
re-aliasing that `safe*.test` needs) builds on cross-interp aliases (now
present) plus auto-loading in children and *deep* cross-interp re-entrancy
(parent alias → back into the same child). The latter is what the depth guard
currently rejects; lifting it needs the parent threaded through the child eval
(or a flat interp registry replacing the ownership tree) so re-entrant
`&mut Interp` access is provably non-overlapping.

> Per-file detail is in the sweep's `--json` (`scripts/dev/rust_tcltest_sweep.py
> --json`). Drive these down toward the Zig baseline.

### Out-of-scope exclusions (by design)

These assert things our implementation **cannot match by design** — we emit
WASM, not Tcl bytecode, and own a different allocator/representation. Excluded
at **test granularity** (most of these files also contain in-scope behavioural
tests that stay in scope); only the specific constraint-bearing tests are
excluded.

| Exclusion class | Mechanism | Affected files (Tcl 9.0.3) | Rationale |
|---|---|---|---|
| Representation / shimmering | `tcl::unsupported::representation` | `abstractlist`, `expr`, `format`, `history`, `lrange`, `lseq`, `string`, `uplevel` | internal repr is an impl detail; we shimmer differently |
| Bytecode / disassembly | `tcl::unsupported::disassemble` / `getbytecode` | `compExpr`, `compile`, `namespace` | we emit WASM, not Tcl bytecode |
| Memory introspection / allocator | `memory` command | `apply`, `assemble`, `basic`, `cmdIL`, `compExpr`, `compile`, `coroutine`, `dict`, `env`, `error`, `fileName`, `for`, `listObj`, `namespace`, `oo`, `ooNext2`, `parse`, `proc`, `regexp`, `string`, `trace`, `var` | internal allocator layout / `memory` introspection is not matchable |

> When a file is fully excluded (vs per-test), record it here with the reason.
> The sweep harness (`run_tcl9_tcltest_sweep.py`) and the excluded set are the
> authority; this table mirrors it.

---

## Upstream sync log (Zig → Rust)

The Zig runtime keeps getting fixed during the port. On a cadence, diff
`runtime/zig/` against the last-synced commit and record dated sync /
gap-audit entries (mirroring `rust-rewrite.md`'s SYNC-* / GAP-AUDIT-*
discipline), noting which behavioural changes have been mirrored into Rust.

**Audit workflow** (run before each chunk and on each SYNC family):

```
git fetch origin
git log --oneline <last-synced>..origin/rust -- runtime/zig/   # Zig changes since last sync
git diff --stat <last-synced>..origin/rust -- runtime/zig/      # impact
```

Classify each Zig commit: **out-of-scope** (Zig-only infra, build) → record and
skip; **in-scope behavioural** (a fix in a module already ported to Rust) → add
an Outstanding row with the source commit + the Rust file(s) to update; mirror
it in the same or a follow-up PR.

### SYNC anchor — 2026-06-05 (branch point)

- Last-synced commit: `rust`@`8150eca` (#549, the spike merge).
- `runtime/rust/` did not exist at the anchor, so there was **nothing to
  mirror** — every component tracks the Zig source as-of this anchor.

### SYNC baseline — 2026-06-05 (T1.1, `runtime/rust/` created)

- `runtime/rust/src/obj.rs` mirrors `runtime/zig/valtypes/tcl_obj.zig` as of
  `rust`@`8150eca` — the obj model + refcount semantics
  (`memory-management.md` MM-A/MM-B/MM-C), cross-checked against
  `tmp/tcl9.0.3/generic/tclObj.c` (`Tcl_NewObj` rc-0 creation, `TclFreeObj`
  immediate free, `Tcl_GetStringFromObj` shimmer). Deliberate divergences from
  the Zig source, all later chunks: the Zig 32-byte handle/tagged-immediate
  layout (the Rust port uses the ABI-faithful 24-byte `#[repr(C)]` `Tcl_Obj`
  instead; codegen optimisations layer on at T1.5/S6) and the deferred-free
  queue (T1.3). Subsequent Zig commits touching `valtypes/tcl_obj.zig` after
  `8150eca` are diffed against this baseline in the Outstanding table.
- `runtime/rust/src/{parse,subst,bs}.rs` (T1.2) mirror
  `runtime/zig/parse/{tcl_parse,tcl_subst}.zig` + `valtypes/tcl_bs.zig` as of
  `8150eca` for **semantics**, cross-checked against `tclParse.c`. **Deliberate
  structural divergence** (re-derived, not transliterated — see T1.2): a
  borrow-based enum tree (`Command`/`Word`/`WordBody`/`WordPart`) replacing the
  Zig flat per-word array + shallow token tree; one shared `scan_parts` scanner
  replacing the duplicated `parse_bare`/`subst_flagged` scans; the
  `$`-not-a-name-is-literal fix. Zig commits touching these files are diffed for
  *behavioural* changes (not structure) against this baseline.
- `runtime/rust/src/frame.rs` (T1.3) mirrors `runtime/zig/interp/tcl_frames.zig`
  + `valtypes/tcl_array.zig` for **semantics**, cross-checked against
  `tclVar.c`/`tclInt.h`. **Structural divergence:** the `tclInt.h` `Var` union
  as a Rust `Var` enum (Scalar/Array/Link) replacing the Zig i32-sentinel
  encoding; `BTreeMap` var/array tables; path-resolved links. Behaviour-diffed
  against this baseline going forward.
- `runtime/rust/src/{interp,builtins}.rs` (T1.4) mirror
  `runtime/zig/interp/tcl_interp.zig` (eval loop) + `dispatch/*.zig` for
  **semantics**, cross-checked against `tclBasic.c` (return codes, eval). **Structural
  divergence:** `Command` enum + `BTreeMap` command table; eval-loop word
  substitution walks the T1.2 `WordPart` enum directly (with `&mut self` for
  recursive `[cmd]` eval) rather than the closure API; **no deferred-free
  queue** (immediate `TclFreeObj` + retain-into-result). Behaviour-diffed going
  forward.
- `runtime/rust/src/list.rs` + the typed-internal-rep machinery in `obj.rs`
  (T1.6) mirror `runtime/zig/valtypes/tcl_list*.zig` for **semantics**,
  cross-checked against `tclListObj.c` (the `List` array backing, the
  `Tcl_ConvertElement` quoting). **Structural choice:** `Tcl_ObjType` carries
  real free/dup/update-string fn-pointer procs dispatched via `typePtr` (the
  shimmer keystone, also the path extension custom types take), and the list
  backing is `Vec<*mut TclObj>` (the contiguous array the ABI forces) hung off
  `internalRep`. Behaviour-diffed going forward.
- `runtime/rust/src/dict.rs` (T1.6) mirrors `runtime/zig/valtypes/tcl_dict.zig`
  for **semantics**, cross-checked against `tclDictObj.c` (insertion order,
  `k v k v` string form). **Structural choice (EXP-DICT, WASM-benchmarked):**
  an insertion-ordered `Vec` of `(key,value)` objects + an FNV-hashed
  `key-bytes → index` map — *not* the Zig list-rep-plus-hash-side-cache.
  Function-mediated dict ABI ⇒ compatible (key objects stored for
  `Tcl_DictObjFirst`). Behaviour-diffed going forward.
- `runtime/rust/src/{cmd_list,cmd_dict}.rs` (T1.6) implement the list commands +
  the `dict` ensemble over those value types, cross-checked against
  `tclCmdIL.c`/`tclDictObj.c`. `lappend`/`dict set`/`dict unset` do
  copy-on-write via `Tcl_IsShared`/`Tcl_DuplicateObj` (`obj::is_shared`/
  `obj::duplicate`). Behaviour-diffed going forward.

### SYNC inbound — 2026-06-05 (the three day-one contracts, PR #551)

PR #551 (against `main`) adds the three first-principles "if starting over"
contracts under `docs/design/contracts/` —
`runtime-variable-frame-model.md`, `parser-and-aot-interpret-boundary.md`,
`numeric-tower-and-expr-semantics.md` — which are the **authoritative** design
for the from-scratch runtime this port *is*. They are **not yet on `rust`**
(the contract links above point forward until #551 merges + `rust` syncs).
**Action:** when they land, re-verify the Rust port against each contract's
*Contract*/*incompatible-by-design* tables. Current alignment + known gaps to
close against them:

| Contract | Rust port alignment | Gaps to close |
|---|---|---|
| variable-frame-model | frame → name → `Var` cell (`Var::Scalar/Array/Link`), array-element + scalar resolution, upvar/global aliasing (T1.3); **per-namespace var tables + one classification/link-walk over a `VarHome` (frame ∣ namespace), qualified `::a::b::x`, `global`/`variable`/`upvar` (T1.5, `vars.rs`)** | path-resolved links (vs the contract's `link → *Cell`) — deliberate (memory-safety); **upvar/global/variable-link cycle must *error*** (today the shared 1000-hop `LINK_LIMIT` silently stops — fix with the proc-chunk recursion bound); independent **cell refcount**; **traces on cells** + re-entrancy/ordering. The **proc-local branch** of the classifier is now **live** (procs push frames): `set` in a body is frame-local and the "unqualified ≠ namespace var inside a proc" rule holds; proc-call recursion is bounded (1000), though the var-link `LINK_LIMIT` still silently stops rather than erroring (separate fix) |
| parser-and-aot-interpret-boundary | the **LSP/compiler `tcl-lexer` is now the canonical scanner** for command/word parsing (`parse.rs` lowers its tokens → the eval `Command`/`WordPart` model); object-passthrough; spans from byte 0 | converge `subst`/`Tcl_SplitList` onto `tcl-lexer` too (step 3, co-evolving a subst + list mode); the compiled≡interpreted identity gate; `source`/`package` VFS+loader; the AOT side lowering from the same component model (T1.7) |
| numeric-tower-and-expr | `i64` int + `double` types; ASCII fast-path strings | the **tower** (small→wide→**bignum**→double, one promote/normalise/compare; canonicalise-on-every-op; no per-command int parse — `incr` overflow now errors instead of wrapping); `expr` as its own lexer/parser/evaluator; `mathfunc` via the command table |

### SYNC inbound — 2026-06-05 (merge `origin/rust` through #555)

The branch was brought up to date with `origin/rust` (merged, not rebased):
the base advanced `8150eca` (#549) → `b3ea465b` (#555) — #550 (BIG-IP object
specs), #552 (typecheck fixes + catalog gap), #553 (rust-rewrite SYNC-JUN09
doc), **#555 (GAP-A/B/C: analysis checks, LSP features, optimisations, the
`tcl-registry` Tk/BIG-IP specs + registry-audit tooling)**. Clean merge, **zero
conflicts**.

- **The Zig-mirror baseline stays `8150eca`.** `git diff 8150eca..origin/rust --
  runtime/zig/` is **empty** — none of #550–#555 touched the Zig runtime, so
  every `runtime/rust/` module still mirrors its Zig source as-of the original
  anchor. The top-of-doc hash is intentionally *not* bumped (it tracks the
  Zig-mirror point, not the workspace tip).
- **Shared-crate impact, verified green.** Of the merged changes, only
  `rust/tcl-lexer/` (`lexer.rs`, `expr_lexer.rs`) is upstream of the runtime
  (via `tcl-syntax`). Post-merge: `make runtime-rust-test` (114 then 124),
  `cargo build --workspace` (all 8 crates), `tcl-syntax` (129) + `tcl-compiler`
  (2434) all pass — the expr/number/glob convergence behaves identically against
  the newer lexer.

### SYNC inbound — 2026-06-13 (`info` subcommands)

`cmd_info.rs`: `info cmdtype` (Command-variant → native/proc/alias/import/
ensemble/object), `info cmdcount` (a per-dispatch counter), `info functions`
(the `::tcl::mathfunc` names), `info loaded` (empty — no C extensions). No Zig
fix back-ported (mirror `8150eca`). `info.test` 81 → 99; sweep **11516 →
11534**, zero regressions. Residual: `info frame` byte-exactness (PC-5) and the
`isnormal`/`issubnormal`/`isunordered`/`rand`/`srand` mathfuncs.

### SYNC inbound — 2026-06-13 (`binary encode`/`decode` + unsigned scan)

`cmd_binary.rs`: `binary encode`/`decode hex`/`base64`/`uuencode` (the base64
`-maxlen`/`-wrapchar` wrapping and `-strict` invalid-char error, ported from
`tclBinary.c`), and the `u` unsigned modifier on integer `binary scan` codes
(mask the low `size*8` bits). No Zig fix back-ported (mirror `8150eca`);
byte-checked against `tclsh9.0`. `binary.test` 308 → 404; sweep
**11408 → 11516**, zero regressions.

### SYNC inbound — 2026-06-13 (`string` surface + `tcl::prefix`)

`cmd_string.rs`: added the `string insert`/`replace`/`wordstart`/`wordend`
subcommands (`StringInsertCmd`/`StringRplcCmd`/`StringStartCmd`/`StringEndCmd`,
incl. `string insert`'s `end == length` append base and the single-non-word-char
word rule), `-nocase`/`-length` on `string compare`/`equal`, the `dict` class +
class-named wrong-args for `string is`, the `::tcl::string::insert` direct
command, and `tcl::prefix match`/`all`/`longest` (`tclIndexObj.c`). No Zig fix
back-ported (mirror `8150eca`); byte-checked against `tclsh9.0`. `string.test`
372 → 552; sweep **11216 → 11408**, zero regressions.

### SYNC inbound — 2026-06-13 (`lsearch` option completion)

`lsearch` (`cmd_list.rs`) completed to the full `Tcl_LsearchObjCmd`: `-sorted`
binary search (`-bisect`, `-increasing`/`-decreasing`), `-ascii`/`-dictionary`/
`-integer`/`-real` datatypes (elements validated lazily during the scan, per C),
`-regexp` (runtime regex engine), `-index`/`-stride` (with the leading-`-index`
group offset), `-start`, `-subindices` — added to the existing exact/glob/
nocase/all/not/inline. Shares `dictionary_compare`/`select_by_index`/`index_spec`
with `lsort`. No Zig fix back-ported (mirror `8150eca`); byte-checked against
`tclsh9.0`. `lsearch.test` 30 → 130; sweep **11116 → 11216**, zero regressions.

### SYNC inbound — 2026-06-13 (`lsort` option completion)

`lsort` (`cmd_list.rs`) completed to the full `Tcl_LsortObjCmd` switch set —
`-dictionary`/`-index`/`-indices`/`-stride`/`-command` added to the existing
`-ascii`/`-integer`/`-real`/`-nocase`/`-increasing`/`-decreasing`/`-unique`.
`DictionaryCompare` is ported verbatim; the decorate-sort-output structure
mirrors C (logical elements keyed by `SelectObjFromSublist`, `-stride` grouping
with a leading-`-index` group offset, regrouped output). `-command` is a stable
merge sort whose comparator dispatches the user prefix (reentrant — `sort_by`
cannot host an interp-evaluating, fallible comparator). Numeric keys are
validated up front so the non-command comparators stay infallible. No Zig fix
back-ported (mirror `8150eca`); byte-checked against `tclsh9.0`. `error.test`
123 → 261, `cmdIL.test` 48 → 125, `lsearch.test` 0 → 30; sweep
**10870 → 11116**, zero regressions.

### SYNC inbound — 2026-06-13 (`lset`)

`lset` (`cmd_list.rs`), the in-variable nested list-element set, ported from C's
`Tcl_LsetObjCmd` + `TclLsetList`/`TclLsetFlat` (`tclListObj.c`). We re-derive the
recursive descent (parse index → range-check `0..=len` → descend/rebuild) rather
than C's copy-on-write `pendingInvalidates` chain (a string-rep-invalidation
detail of C's representation, not behaviour); single-element sublists stringify
without braces, which reproduces the empty-list/append quirks for free. Shares
`index_spec`/`bad_index` with `lindex`/`ledit`; stores back through `var_set`
(write traces fire) like `ledit`. No Zig fix back-ported (mirror anchor
`8150eca`); byte-checked against `tclsh9.0`. `reg.test` 0 → 32, `lsetComp.test`
2 → 19; sweep **10818 → 10870**, zero regressions.

### SYNC inbound — 2026-06-13 (`trace` command/execution/step + lifecycle + var-trace error propagation)

Completes the `trace` command beyond the prior variable-only subset, derived
from C's `tclTrace.c` (`Tcl_TraceObjCmd` + `TraceExecutionObjCmd`/
`TraceCommandObjCmd`/`TraceExecutionProc`/`TclCheckExecutionTraces`/
`TclCheckInterpTraces`) and `tclBasic.c` (`TclEvalObjvInternal` enter/leave,
`TclRenameCommand`) + `tclVar.c` (`TclObjVarErrMsg`). Five gated chunks:
command-trace registration/info → command-trace firing → execution enter/leave
→ step traces + redefine/unset/frame lifecycle → variable-trace error
propagation. `trace.test` **49 → 195**; sweep **10660 → 10818**, zero
regressions (a transient var.test regression from exposing a pre-existing
local-variable-trace leak was fixed by tying frame-local traces to their call
frame).

- **Verbatim against C, byte-checked vs `tclsh9.0`.** Every new form — the
  `bad option`/`bad operation[ list]`/`unknown command "X"` errors, the
  enter/leave/step callback strings and their newest-first/oldest-first/
  reverse/forward ordering, the live-result-between-callbacks leak, the
  `can't read|set "name": <msg>` propagation — was confirmed identical to
  `tclsh9.0`, including the recursive-`factorial` and nested-`bar`-error step
  scenarios (`trace-23.2`, `trace-28.2`).
- **No Zig behavioural fix back-ported.** `runtime/zig/` has no `trace`
  command-family module (variable-trace firing only); the port follows C's
  control flow directly. Mirror anchor unchanged (`8150eca`).
- **Representation note.** Three sibling registries on `TraceTable` (variable
  `Vec`, command/execution `CmdTrace` `Vec` keyed by FQN with a `TraceOps`
  `u8` bitset, live-step `StepActive` `Vec`) rather than one unified enum vec —
  the kinds fire from disjoint chokepoints and the hot dispatch path early-outs
  on `cmd_traces.is_empty()`. Step-trace lifetime is bracketed by the call
  stack (install on the step-traced command's `dispatch_traced` entry, dedup by
  owner+prefix for recursion, pop on exit) rather than C's explicit
  `startLevel`/`startCmd` interp-trace, which the single-threaded `Rc`/`RefCell`
  model makes unnecessary.

### SYNC inbound — 2026-06-13 (`ledit`/`lmap`/`lseq` + var-read-miss + eval-reset; audit re-baseline)

Chunks: `ledit`, `lmap`, `lseq`, the three-way variable-read-miss error, and
the empty-script result reset (scoreboard above). `lmap` was written against C's
shared `EachloopCmd` (`tclCmdAH.c`, `TCL_EACH_COLLECT`) — `foreach` refactored to
the same engine; `lseq` against `Tcl_LseqObjCmd`/`TclNewArithSeriesObj`
(`tclCmdIL.c`/`tclArithSeries.c`); the empty-script reset matches `Tcl_EvalEx`
(`tclBasic.c`).

- **Derived from C, cross-checked vs the Zig oracle.** `ledit` was written
  against C's `Tcl_LeditObjCmd` (`tmp/tcl9.0.3/generic/tclCmdIL.c`) and the
  `no such element in array` distinction against `tclVar.c`; both were confirmed
  consistent with the Zig oracle (`runtime/zig/cmds/list.zig`'s shared
  `do_lreplace`, and `runtime/zig/cmds/var.zig`). No Zig behavioural fix was
  back-ported — the port matches the same C control flow the Zig oracle does.
- **Audit note (mirror anchor still `8150eca`).** `git log 8150eca..origin/rust
  -- runtime/zig/` is **no longer empty** (it was, as of #555): the post-#555
  main-rebases (#573 onto `0becf577`, then #583/#595/#596) carried main's Zig
  evolution onto the branch — a large diff (≈119 files) that is the **general
  Zig-on-main churn**, *not* a behavioural fix to a Rust-ported module triggered
  by this chunk. Reconciling that diff against the per-module mirror baselines is
  its own (pre-existing) audit task, independent of `ledit`/read-miss; recorded
  here so it is not lost. The top-of-doc mirror hash is intentionally left at
  `8150eca` until a deliberate re-baseline.

### Outstanding

_(empty — populated as Zig lands behavioural fixes during the port)_

| Date | Zig commit | Module | Behavioural change | Mirrored into Rust |
|---|---|---|---|---|
| — | — | — | — | — |

### Review follow-ups — PR #557

Tracked from the T1.1–T1.6 review (the ✅ items were fixed in the same wave;
the rest are deferred with the reviewer's concurrence):

- ✅ **Command substitution propagated only `Code::Error`** (`interp.rs`) — now
  propagates any non-`Ok` code (`return`/`break`/`continue`) out of `[...]`.
- ✅ **`string first`/`last` ignored the optional index** (`cmd_string.rs`) — the
  `startIndex`/`lastIndex` bound is now honoured (char-based, `end±N` aware).
- ✅ **`set arr` (scalar read of an array)** reported `no such variable` instead
  of `variable is array` (`builtins.rs`) — fixed via `frames.is_array`.
- ✅ **`{*}`/list-parse errors** collapsed to one hardcoded message
  (`interp.rs`/`cmd_list.rs`) — now map the `ListError` variant to its shared
  `tcl_syntax` message (the `…FollowedByJunk` byte-exact `"<frag>" instead of
  space` suffix still needs the offending fragment surfaced from the splitter —
  minor follow-up).
- ◐ **Recursion / alias-cycle bound** (`interp.rs`) — **partly done with the
  proc chunk:** `Interp.recursion_depth` bounds **proc-call** nesting at C's
  default 1000, so infinite proc recursion raises the catchable `too many nested
  evaluations (infinite loop?)` instead of a stack overflow. **Remaining:**
  extend the same counter to unbounded `eval`/`[...]`/`dispatch_alias` nesting
  (land with PC-3's `eval`), and make `interp recursionlimit` configurable.
- ⏳ **NaN ordering in `bignum::compare`** — a NaN operand maps to
  `Ordering::Greater`, so `x > NaN` is spuriously true. Tcl makes every ordered
  comparison with NaN false (and in `expr` a NaN-producing op is itself a domain
  error). Settle the NaN/domain-error semantics when `expr` NaN handling is
  finished; add `expr {1.0 < (0.0/0.0)}`-style tests then.
- ⏳ **`Tcl_DecrRefCount` on a `fresh_zero` (rc 0) obj** refuses to free (counted
  as a double-free), unlike `tcl.h`'s macro which frees at rc≤1; and
  `Tcl_DecrRefCount`/`TclFreeObj` are a **macro**, so a C extension never calls
  the exported function. Capture both in
  [`c-api-ownership-contract.md`](c-api-ownership-contract.md) (the
  extension-side decref→free path is a Track-2 loader/ABI item).
- ✅ **String rep now preserved across a string→typed shimmer** (`obj.rs`
  `change_type`). A plain string keeps its buffer's capacity in `internal_rep`;
  rather than free the bytes when the typed rep claims that slot, `change_type`
  now **shrinks the buffer to exact `length + 1` and keeps it** as the cached
  (immutable) string rep — so `set x {a  b   c}; llength $x; set x` returns the
  original `a  b   c`, and an in-place mutation (`lappend`/`dict set`, which
  already `invalidate_string`) regenerates the canonical form. No `TclObj`
  layout change was needed (spare capacity only matters while mutable-as-string);
  this also fixes `Tcl_DuplicateObj` of a typed obj (the `dup_int_rep_proc` path
  routes through `change_type`), and the `ensure_list`/`ensure_dict` "string rep
  is kept" comments are now accurate.

---

## Gates summary

| Gate | Command | Applies to |
|---|---|---|
| WASM command parity | `make check-wasm-parity` | Track 1 (registry/dispatch/builtins) |
| Tcl 9 tcltest sweep | `scripts/run_tcl9_tcltest_sweep.py` | Track 1, Tier gates, correctness gold standard |
| Leak sweep | `scripts/leak_sweep.py` / `make leakcheck` (Zig); `make runtime-rust-test` (Rust port, T1.1+) | Track 1 (refcount discipline), T2.1 |
| Tier LOAD+RUN | per-tier `wasmtime` tests | Tier 0/1/2 |
| C-API annotation | `make check-c-api-ownership` (`scripts/check_c_api_ownership.py --strict`) | Track 2 — **landed**, in `_prep-pr-checks-noty` |
| AOT coverage | T3.1 coverage harness | Track 3 |

No `.test` file that passes on the Zig baseline may regress. `make
check-wasm-parity` and the editor extensions stay green — do **not** regress the
compiler/LSP or the Zig runtime.

---

## Next-up priority queue

1. ✅ **This document** (the first deliverable) — established; kept current.
2. ✅ **T2.1** — C-API ownership/error contract + un-annotated-export gate.
   Contract doc + gate landed ([`c-api-ownership-contract.md`](c-api-ownership-contract.md),
   `make check-c-api-ownership`); the categories are now realised in the
   `runtime/rust/` obj/interp impls (T1.1). Extending the gate to the real
   `extern "C"` exports happens as the surface grows.
3. ✅ **T1.1** — real `TclObj` + refcount core in `runtime/rust/`
   (`make runtime-rust-test`, zero-residual round-trip); Zig source baseline
   recorded in the sync log.
4. ✅ **T1.2** — parse/subst port. Landed as a re-derived borrow-based enum
   model (`bs`/`parse`/`subst`, `unsafe`-free); segment evaluation wires into
   the eval loop next.
5. ✅ **T1.3** — frames + variable store (`frame.rs`: `Var` enum, `FrameStack`,
   scalar/array/upvar/global, leak-checked; closed subst's variable half).
6. ✅ **T1.4** — eval loop + command table + dispatch (`interp.rs`/`builtins.rs`:
   parse→subst→dispatch, `{*}`, completion codes, starter builtins; **closed
   subst's command half**; no deferred-free queue needed).
7. ◐ **T1.6 (value types + their commands)** — obj **typed-internal-rep
   machinery** (shimmer keystone + custom-`Tcl_ObjType` path); **list** (contiguous
   `Vec`) + list commands; **dict** (ordered `Vec` + FNV index, EXP-DICT) + the
   `dict` ensemble; **string** (capacity-backed append + ASCII-fast char ops,
   EXP-STRING) + `append` + the `string` ensemble — all leak-checked. **Parser
   convergence step 2 landed**: command/word parsing lowers from the shared
   `tcl-lexer` token stream (step 3 — `subst`/`Tcl_SplitList` — and dropping
   `bs.rs` follow).
8. ✅ **Numeric tower + `expr`** — the small→wide→**bignum** (libtommath
   `mp_int` FFI, EXP-BIGNUM)→double tower with promote/normalise/compare;
   `incr` rewired onto it (overflow promotes, never wraps); `expr` wired into the
   eval loop via the **shared** `tcl_syntax::expr` walk (lexer→AST→parser→eval→
   mathfunc→double-format all single-sourced with the compiler's const-folder).
9. ◐ **T1.5 (namespaces)** — **done:** the namespace **tree + the one
   `resolve(currentNs, name)` resolver**; **`rename` + `interp alias`** (the
   `Alias` redirect + by-name-anchored-at-global dispatch trampoline); the
   **`namespace` command** (`current`/`eval`/`exists`/`parent`/`children`/
   `qualifiers`/`tail`/`which`/`export`/`import`/`forget`/`path`, with the
   `Imported` redirect); the **shared `string match` glob** (`tcl_syntax::glob`,
   converging two compiler copies); and the **variable-namespace side** — the
   variable parallel of the command resolver: per-namespace var tables
   (`Namespace.vars`; the global ns holds globals), one classification + link
   walk (`vars.rs`) over a `VarHome` (frame level **or** namespace id), so
   `set ::ns::x` / `$::ns::x` / `unset ::ns::x` resolve through the tree and
   `::pinged` ≡ `pinged` at top level; plus **`global`/`variable`/`upvar`**
   (`cmd_var.rs`) installing the links, and `set`-into-a-missing-namespace
   raising `parent namespace doesn't exist`. The `::`-qualifier split is shared
   with the compiler (`tcl_syntax::naming::qualifier_segments`); and
   **`::tcl::mathfunc::*` / `::tcl::mathop::*` as overridable commands** —
   `expr`'s function-call path resolves `::tcl::mathfunc::NAME` through the
   command table (overrides/`rename` win; A3), and every operator is a real
   `::tcl::mathop::` command over the shared tower; and **ensembles** — the
   canonical `ens sub`→target redirect (`namespace ensemble create`/`exists`
   with `-map`/`-subcommands`/`-prefixes`/`-command`, dispatching to `-map` or
   `<ns>::<sub>`), the generalisation of the `dict for`→`::tcl::dict::for`
   rewrite. **Next in T1.5:** `rand`/`srand` (interp RNG state), `namespace
   delete`, `namespace ensemble configure`. (Per-frame `current_ns` + the
   proc-local var branch are now **live** — see #10.)
10. ◐ **Procs + control flow** (per [`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md))
    — **done (PC-2, conservative):** `Command::Proc(Rc<ProcDef>)` + `call_proc`
    (arity/`wrong # args`, push a frame in the proc's defining namespace, bind
    params/defaults/`args`, eval body, `return`→Ok, pop) — this **activates the
    proc-local var branch + per-frame current namespace** (`set` in a body is
    frame-local; `variable`/`global` hit the proc's ns); a **recursion bound**
    (C's default 1000) makes infinite recursion a catchable error; **control
    flow** `if`/`while`/`for`/`foreach` + `break`/`continue`; **`puts`**; and an
    `examples/run_script.rs` that runs a fib/for/namespace/foreach script end to
    end. A body-level `break`/`continue` that escapes a proc now errors
    (`invoked "break" outside of a loop` — Zig-oracle fix). **Next:** PC-1
    `CmdFrame` source/line stack; PC-3 `uplevel` + `eval` + generalised `upvar`;
    PC-4 exceptions (`error`/`catch`/`return -options` + `errorInfo`); PC-5
    `info level`/`info frame`/`source`; PC-6 AOT interop.
11. ◐ **Run the real Tcl library + `tcltest`** (new north-star bring-up — see
    [`tcltest-bringup.md`](tcltest-bringup.md); **in progress** — sweep at
    **11662/20532**, the 2026-06-13 TclOO meta-protocol increments (class-
    destroy cascade, per-object `my`, `private` methods + `unknown` method
    list, `oo::object`/`oo::class` as real objects) took `oo.test` 45 → 123, the
    2026-06-13 `ledit`/`lmap`/`lseq` +
    var-read-miss +
    empty-script reset increments landed the list/loop-command surface that
    `lreplace.test`/`lmap.test`/`lseq.test`/`set*.test` exercise, the
    `trace` command/execution/step + lifecycle increment took `trace.test`
    49 → 195, `lset` unblocked `reg.test`/`lsetComp.test`, the full
    `lsort` option set took `error.test` 123 → 261 and `cmdIL.test` 48 → 125,
    and the full `lsearch` option set took `lsearch.test` 30 → 130).
    Run the **unmodified** pure-Tcl
    `init.tcl`/`tcltest.tcl` + real C-Tcl-9 `*.test` files by **porting the C
    command surface** (not re-porting the library): L1 eval/exception/
    introspection core (`eval`/`uplevel`/`apply`/`subst`/`catch`/`error`/`return
    -options`/`switch`/`info`/`array`/`package` + list ops — this is PC-3/PC-4),
    then L2 VFS + channels (`source`/`file`/`glob`/`open`/…), then L3 host
    (`clock`/`encoding`/`format`/`scan`/`regexp`/`exec`/…). Reason over the
    library code, reference C Tcl + the Zig oracle (the discoveries appendix in
    the bring-up doc), empirical loop (source → wall → port → repeat).
12. **T3.0** — backend-agnostic emit protocol/trait + command-emission registry
    bound to the editor command registry; `NoEmitImpl` error for unimplemented
    commands (the codegen-side single-source-of-truth that all later AOT work
    builds on).
13. **T2.3** (de-risk against Zig first) — production loader, validated on
    Tier 0 dltest, separating loader risk from port risk.
14. **T3.1** — `wasm_link.py` extension linking + AOT-coverage measurement
    harness (seeds the scoreboard).
15. **S7 spec** — `wasm-aot-staircase-s7.md` (metaprogramming heuristics).

---

## Conventions

- Keep **this doc and `c-extension-abi.md` current every PR** (flip §13 items
  as they land; log every upstream Zig sync).
- **Always record the base git hash** the port is built on — the banner at the
  top of this doc (`rust`@`8150eca` today) and the sync-log anchor. Every Rust
  module's doc comment / the sync log states which Zig+C sources it mirrors *as
  of that hash*. On a deliberate rebase onto a newer `rust`, bump the hash here
  and open a fresh SYNC family.
- **Re-derive every data structure** via the three-step method above
  (investigate the commands/subcommands → run WASM-compiled experiments →
  reason through C-extension ABI support); never transliterate a representation
  from Zig without it. Land the representation-decision note with the chunk.
- Add KCS / design docs per [`AGENTS.md`](../../../AGENTS.md); commits scoped
  and gated.
- Never merge a tier or stage without its gate green.
- If a needed surface is large, land it as its own gated PR before the gate that
  needs it.
