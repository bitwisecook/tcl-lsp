# Rust runtime port — productionising C-Tcl-extension-to-WASM

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
| `valtypes/` value types | 20 (9211) | list, dict, string, array, arith, format, encoding, hash_table, bs, chars, regex, arena, parse_cache | `runtime/rust/` valtypes | not-started | per-type unit tests mirror `runtime/zig/test_tcl_*.zig`; tcltest sweep no-regress; **+ a representation-decision note per structure** (see [Choosing algorithms & data structures](#choosing-algorithms--data-structures-the-porting-method)) |
| `parse/` | 3 (956) | `tcl_parse`, `tcl_subst` | `runtime/rust/` parse | **partial** (T1.2) | `make runtime-rust-test` — parse/subst unit parity (`parse`/`subst`/`bs` modules); evaluation of `$var`/`[cmd]` segments wired with the eval loop (T1.3/T1.4) |
| `interp/tcl_interp.zig` | 1 (2065) | eval loop, interp object | `runtime/rust/` interp | **partial** (T1.4) | `make runtime-rust-test` — eval loop: parse→subst→dispatch, `{*}`, completion codes; control-flow/proc follow |
| `interp/` frames/ns/procs | 8 (6348) | frames, namespaces, procs, catch, caps, trace, interp_registry | `runtime/rust/` interp | **partial** (T1.3: frames + var store) | `make runtime-rust-test` — frame/var leak-checked round-trips (scalar/array/upvar/global); ns/procs/catch follow |
| `dispatch/` | 5 (746) | cmd registry, cmd table, dispatch, diag, stub_fallback | `runtime/rust/` dispatch | **partial** (T1.4) | `make runtime-rust-test` — `BTreeMap` command table + name dispatch; `make check-wasm-parity` once the builtin surface fills in |
| `cmds/` builtins | 34 (8367) | all builtin commands | `runtime/rust/` cmds | not-started | per-command parity + tcltest sweep per `.test` |
| `io/tcl_chan.zig` | 1 (1858) | channel subsystem | `runtime/rust/` io | not-started | chan/chanio/io/ioCmd tcltest suites (Memchan needs this) |
| `io/tcl_clock.zig` + `tcl_tz.zig` | 2 (3560) | clock + tz (+ `data/tzdata.bin`) | `runtime/rust/` io | not-started | clock tcltest slice (`run_clock_tcltest.py`) |
| `io/tcl_fs.zig` | 1 (1186) | filesystem (tclvfs needs `Tcl_FSRegister`) | `runtime/rust/` io | not-started | fs tcltest + tclvfs tier-1 gate |
| `sched/` | 7 (1660) | scheduler, coro, timer, vwait, fileevent, ready, asyncify | `runtime/rust/` sched | not-started | coroutine/after/vwait tcltest |
| `stubs/` | 6 (609) | env/fmt/fs/io/time stub surfaces | `runtime/rust/` stubs | not-started | covered by dependent command parity |
| `tcl_runtime.zig` (root) | 1 | export-aggregation root | `runtime/rust/` lib root | not-started | runtime builds + exports the `tcl_*`/`obj_*` symbol set codegen imports |
| `regex_include/` (C) | — | Henry Spencer ARE engine (C, vendored) | **C at start → port to Rust near the end** (see note) | not-started | start: ARE-fidelity corpus passes via the C engine; end: same corpus passes against the Rust port, zero diff |

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
- **T1.6 — builtins.** Port `cmds/*.zig` incrementally (string/list/dict/expr/
  control-flow/proc/…), each command (or small group) one PR with its tcltest
  delta. The value-type chunks (list/dict/string/array) each carry a
  [representation-decision note](#choosing-algorithms--data-structures-the-porting-method).
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

`tmp/tcl9.0.3/tests/*.test` (168 files), run via
`scripts/run_tcl9_tcltest_sweep.py`. **In scope: behaviour.** No file passing on
the Zig baseline may regress. Per-file pass/partial/excluded is captured against
the Zig baseline; seeded empty here — the first sweep establishes the baseline
column.

| `.test` file | Zig baseline (pass/total) | Rust (pass/total) | Status |
|---|---|---|---|
| _seed — captured by first `run_tcl9_tcltest_sweep.py` run_ | — | — | not-started |

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

### Outstanding

_(empty — populated as Zig lands behavioural fixes during the port)_

| Date | Zig commit | Module | Behavioural change | Mirrored into Rust |
|---|---|---|---|---|
| — | — | — | — | — |

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
   subst's command half**; no deferred-free queue needed). **Next: T1.5/T1.6** —
   namespaces (the ns tree) and the builtin surface (each value-type chunk with
   its representation-decision note); procs + the proc-call frame path.
7. **T3.0** — backend-agnostic emit protocol/trait + command-emission registry
   bound to the editor command registry; `NoEmitImpl` error for unimplemented
   commands (the codegen-side single-source-of-truth that all later AOT work
   builds on).
8. **T2.3** (de-risk against Zig first) — production loader, validated on
   Tier 0 dltest, separating loader risk from port risk.
9. **T3.1** — `wasm_link.py` extension linking + AOT-coverage measurement
   harness (seeds the scoreboard).
10. **S7 spec** — `wasm-aot-staircase-s7.md` (metaprogramming heuristics).

---

## Conventions

- Keep **this doc and `c-extension-abi.md` current every PR** (flip §13 items
  as they land; log every upstream Zig sync).
- **Re-derive every data structure** via the three-step method above
  (investigate the commands/subcommands → run WASM-compiled experiments →
  reason through C-extension ABI support); never transliterate a representation
  from Zig without it. Land the representation-decision note with the chunk.
- Add KCS / design docs per [`AGENTS.md`](../../../AGENTS.md); commits scoped
  and gated.
- Never merge a tier or stage without its gate green.
- If a needed surface is large, land it as its own gated PR before the gate that
  needs it.
