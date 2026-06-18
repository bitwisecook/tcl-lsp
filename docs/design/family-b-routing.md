# Family-B contract & command routing — outcomes

Companion to [`common-runtime-emitter-architecture.md`](common-runtime-emitter-architecture.md)
(§4 Family B). Records what the Family-B runtime contract looks like once
implemented on both runtimes, which command families were lifted to shared
cores, the bugs that lifting surfaced, and — importantly — the boundaries where
a command **cannot** be a shared body.

## 1. The contract (`tcl-runtime-api`)

A light leaf crate (depends only on `tcl-core-types`) holding the Family-B role
traits, each generic over an associated `Value`. Both the bytecode VM (`tcl-vm`,
`Value = Rc<Obj>`) and `runtime/rust` (`Value = *mut TclObj`) satisfy all of
them, so a consumer generic over the traits drives either runtime:

| Trait | Surface |
|-------|---------|
| `VarStore` | `get`/`set`/`unset`/`exists` + the explicit array-element pairs `get_elem`/`set_elem`/`unset_elem`/`exists_elem`, addressed by `FrameId` |
| `Frames` | `push(NsId)`/`pop`/`current`/`link` (the `upvar` install) |
| `Commands` | `dispatch(name, argv)` and `dispatch_id(CommandId, argv)` — the resolve-then-invoke pair with `find_command` |
| `Namespaces` | `find_command(cxt, name) -> CommandId`, `current() -> NsId`, `name(NsId) -> String`, `command_name(CommandId) -> Option<String>` |
| `Traces` | `fire(var, op)` (read/write/unset; read/write errors abort) |
| `Introspect` | `level()`, `level_argv(n)` |

The **value seam** (`ValueOps` in `tcl-syntax`) also grew an arithmetic rung,
`int_add(Option<&V>, &V) -> Result<V, ValueError>`, whose `Option` left operand
folds in "absent value = 0". Its default is fixed-`i64` with overflow →
`ValueError::IntegerOverflow`; the bignum runtime overrides it to widen. This is
the seam that let `incr` be shared (§2) without the core ever naming a number
representation.

Notes:
- `CompileService` (the runtime-`eval` injection point) was abstracted behind an
  associated `Module` type so the contract crate carries no bytecode dependency
  — the prerequisite for `tcl-cmd-core` depending on these traits.
- Handle bridges: the VM is string/`i64`-native, so it interns namespace names
  (`NsId`) and command FQNs (`CommandId`) in side-table arenas; `runtime/rust`'s
  `NsId`/`Code` are distinct types from the contract's and are mapped explicitly.
- `CommandId` is produced by `find_command` and consumed by `dispatch_id`
  (reverse-mapped to the absolute FQN, then dispatched) — that pairing closed the
  "handle with no consumer" gap.

## 2. What is shared, and where

The split (architecture §6): **value-shaped** command bodies are shared
*concrete code* in `tcl-cmd-core` (generic over `ValueOps`, plus a role trait for
the stateful ones); **stateful** commands are, in general, *trait calls*, not a
shared body.

Shared in `tcl-cmd-core`:
- Value families (pre-existing): `string`, `list`, `dict`, `format`, `scan`,
  `index`, `string is`, plus `platform`/`path` helpers.
- `info::level` — over `Introspect` + `ValueOps`.
- `info::exists` — over `VarStore::exists` + `Frames::current`.
- `info::complete` — pure (`Tcl_CommandComplete`).
- `namespace::{tail, qualifiers}` — pure byte ops.
- `namespace::{current, which_command}` — over `Namespaces` (`current`/`name`/
  `command_name`/`find_command`).
- `path::{tail, dirname, extension, rootname}` — a `/`-based **byte** path core
  (platform-independent), replacing the VM's old `std::path::Path` versions.

`incr` is shared **only at the value seam**: all three sites (the VM's
`cmd_incr`, the VM's compiled `INCR_*` opcodes via `incr_var`, and the runtime's
`incr`) compute the new value through `ValueOps::int_add`, each over its own
native variable access. The trace-aware *store* and the const check stay in each
adapter on purpose — the contract's `VarStore::set` is storage-only and discards
the write-trace outcome, whereas C's `incr` must store yet fail when a write
trace errors.

**Routing repeatedly surfaced real VM bugs** (the runtime was correct; the shared
core unified both to the correct behaviour):

| Command | VM bug fixed by routing |
|---------|-------------------------|
| `info level` | non-integer arg reported `bad level` instead of `expected integer but got "x"` |
| `info exists` | scalar-only check missed arrays (`info exists ::env` → 0) — also fixed `VarStore::exists` |
| `namespace tail`/`qualifiers` | naive `rsplit("::")` mishandled colon runs (`tail foo:::` → `:`) |
| `info complete` | counted `[]` inside `{braces}` (`info complete {[}` → 0) |
| `incr` (overflow) | `i64` `wrapping_add` silently wrapped past `i64::MAX`; now errors `integer value too large to represent` (the VM has no bignum) |
| `incr` (error order) | a non-integer increment was reported before a non-integer current value; now current-first, matching C's `TclIncrObj` and the runtime |

That is the payoff of the seam: one body (or one seam), enforced-identical
semantics, latent divergences caught.

## 3. Boundaries — commands that are *not* shared (and why)

These are not caution; they are real value-model / representation boundaries that
vindicate the architecture's "trait calls, not shared code" stance for stateful
commands.

- **`append`/`lappend`** — deliberately kept per-runtime after a full
  investigation. The blocker is the **value representation**, which is exactly the
  irreducible per-target difference the architecture isolates:

  - *`append` is byte-vs-UTF-8 bound.* The runtime stores raw bytes (`*mut TclObj`,
    `obj_bytes`) and its `append` is byte-exact (`string_append_inplace`); the VM's
    `Value` is `Rc<str>` — **UTF-8 only**, it cannot hold arbitrary bytes. The
    `ValueOps` value seam is deliberately char-correct (`as_str -> Rc<str>`, via
    `from_utf8_lossy` in the runtime). Routing `append` through it would silently
    corrupt binary data in the runtime (`append data $bytes`), a **correctness**
    regression. Byte-exact sharing needs a *byte-oriented* `ValueOps` extension —
    the same one the plan scopes to the `binary` family (Track B) — not the
    char-correct seam.

  - *In-place growth is load-bearing, not a micro-opt.* The runtime grows its
    string/list buffer in place when the value is unshared (amortised O(1)); a
    shared core built on the default rebuild seam makes `append`/`lappend` in a
    loop **O(n²)** — and string-building via `append` in a loop is a core Tcl
    idiom. Preserving it across the seam requires per-element in-place capabilities
    (`try_append_str_in_place` exists for strings; an analogous
    `try_list_append_in_place` would be new) plus a `(value, needs_store)` return
    so the adapter knows whether the variable was mutated in place.

  - *Write-trace semantics already diverge — sharing would be a behaviour
    change.* C's `Tcl_AppendObjCmd` fires a write trace **per value**, while
    `Tcl_LappendObjCmd` appends all at once and fires it **once**; crucially both
    **always store back** (`TclPtrSetVarIdx`) — the unshared case modifies in place
    *and still* stores, so the write trace fires. The runtime's in-place path skips
    `var_set` entirely and so fires **no** write trace (a latent bug vs C), while
    its copy/new path fires once. A C-faithful shared core would "always store"
    (firing the trace, fixing the bug) — but that is a behaviour change to verify
    against `tclsh`, alongside the matching read-trace-once that the runtime's
    `var_get`-based read also currently skips. Worth doing as a deliberate
    C-reconciliation, not folded silently into a dedup.

  Net: every sharing path either regresses correctness (lossy bytes), regresses
  performance (O(n²)), or only breaks even on dedup while adding a new value-seam
  capability and `(value, needs_store)` plumbing — and touches the delicate
  trace/refcount machinery. That is a deliberate contract extension (the Track-B
  byte seam + a list in-place seam), better made with review than as an autonomous
  side-effect. Contrast `incr`: it had a *clean* number-model seam (`int_add`) with
  no byte/COW/trace entanglement, so it was shared.

## 4. Known contract gaps (no consumer yet — recorded, not fixed)

- The array-element methods (`get_elem`/`set_elem`/`unset_elem`/`exists_elem`)
  honour the active frame on both runtimes; the **non-active-frame** element path
  (`*_from`/`*_at`) is still scalar-only on the VM and ignored by the runtime
  (the element accessors take `FrameId` but use the active frame). No current
  consumer needs cross-frame element access.
- No enumeration surface (`info commands`/`vars`/`globals`, `namespace children`)
  — these need a listing API the state-mutation traits deliberately omit.
- No **byte-oriented** `ValueOps` rung. `as_str` is char-correct (UTF-8); a
  `binary`/byte family (and a byte-exact `append`) needs `as_bytes`/`new_bytes`
  (+ a byte in-place append). Scoped to Track B.

## 5. Recommended next steps (each its own decision)

1. ✅ *Done* — `VarStore` array-element methods; the `ValueOps::int_add`
   arithmetic seam (`incr` shared); `Namespaces::name`/`command_name`
   (`namespace current`/`which` shared); the `/`-based byte `path` core (`file`
   path-ops shared, VM made platform-independent).
2. The **byte-oriented `ValueOps` extension** (`as_bytes`/`new_bytes` + a byte
   in-place append). Unlocks the `binary` family *and* a byte-exact shared
   `append`; the make-or-break call for §3's `append`/`lappend` boundary.
3. A **list in-place seam** (`try_list_append_in_place`, mirroring
   `try_append_str_in_place`) + a `(value, needs_store)` core convention, if
   `lappend`/`lset`/`linsert`-into-var are to share without losing amortised
   growth — and a deliberate reconciliation of their write-trace firing against C.
4. An enumeration surface on the contract for the `info`/`namespace` listing
   subcommands.
