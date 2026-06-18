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
| `VarStore` | `get`/`set`/`unset`/`exists`, addressed by `FrameId` (frame-addressed access for a non-active frame) |
| `Frames` | `push(NsId)`/`pop`/`current`/`link` (the `upvar` install) |
| `Commands` | `dispatch(name, argv)` and `dispatch_id(CommandId, argv)` — the resolve-then-invoke pair with `find_command` |
| `Namespaces` | `find_command(cxt, name) -> CommandId`, `current() -> NsId` |
| `Traces` | `fire(var, op)` (read/write/unset; read/write errors abort) |
| `Introspect` | `level()`, `level_argv(n)` |

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

**Routing repeatedly surfaced real VM bugs** (the runtime was correct; the shared
core unified both to the correct behaviour):

| Command | VM bug fixed by routing |
|---------|-------------------------|
| `info level` | non-integer arg reported `bad level` instead of `expected integer but got "x"` |
| `info exists` | scalar-only check missed arrays (`info exists ::env` → 0) — also fixed `VarStore::exists` |
| `namespace tail`/`qualifiers` | naive `rsplit("::")` mishandled colon runs (`tail foo:::` → `:`) |
| `info complete` | counted `[]` inside `{braces}` (`info complete {[}` → 0) |

That is the payoff of the seam: one body, enforced-identical semantics, latent
divergences caught.

## 3. Boundaries — commands that are *not* shared (and why)

These are not caution; they are real value-model / representation boundaries that
vindicate the architecture's "trait calls, not shared code" stance for stateful
commands.

- **`incr`** — irreducible. The VM's integer rep is a closed `i64` enum (no
  bignum); `runtime/rust` has a bignum tower (promotes on overflow). `ValueOps`
  exposes only `as_int -> i64` and no arithmetic, so a shared core would force
  `i64` on the runtime (losing bignum) or require bignum in the VM.
- **`append`/`lappend`** — the runtime versions are hand-optimised with in-place
  buffer growth, copy-on-write (`is_shared`/`duplicate`), const-variable checks
  and manual `*mut TclObj` refcounting; the VM versions are trivial `Rc` rebuilds.
  `ValueOps` can abstract the value op, but the in-place-aliasing + refcount
  correctness is runtime-specific and high-risk to share.
- **`file dirname`/`tail`/`extension`/`rootname`** — the VM uses `std::path::Path`
  (platform-dependent separators); the runtime uses `/`-based **byte** logic
  (paths can be non-UTF-8). A shared str core would lose byte-exactness and
  platform-independence; they match for common Linux paths but can diverge on
  edge cases (`//`, normalisation). (`tcl-cmd-core::path` exists, str-based, but
  is currently unused.)

## 4. Known contract gaps (no consumer yet — recorded, not fixed)

- `VarStore::exists` was fixed for arrays on the *active* frame; the
  frame-addressed path (`exists_from`/`unset_from` on the VM) is still
  scalar-only. Same for `VarStore::unset` of an array *element* — handled by the
  `unset` command directly, not yet by the trait.
- `Namespaces` has no `NsId -> name` accessor, so `namespace current`/`which`
  (which return names) can't yet route through the contract.
- No enumeration surface (`info commands`/`vars`/`globals`, `namespace children`)
  — these need a listing API the state-mutation traits deliberately omit.

## 5. Recommended next steps (each its own decision)

1. Extend `VarStore` with explicit array-element methods (`get_elem`/`set_elem`/…)
   so array handling is uniform across both runtimes — the prerequisite for ever
   sharing `append`/`lappend`.
2. Add an integer-arithmetic seam to `ValueOps` (per-runtime overflow: VM `i64`,
   runtime bignum) if `incr`-class commands are to be shared.
3. Add `Namespaces::name(NsId)` to unlock `namespace current`/`which`.
4. A `/`-based **byte** path core (mirroring the namespace ops) if `file`
   path-ops are to be unified — and to make the VM platform-independent.
