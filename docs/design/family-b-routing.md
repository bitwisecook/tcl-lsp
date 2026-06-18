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
- `sort::{key_compare, dictionary_compare, parse_wide, parse_real}` — the
  `lsort`/`lsearch` comparison modes (`-ascii`/`-dictionary`/`-integer`/`-real`,
  `-nocase`), pure `&[u8] → Ordering`. The runtime's `lsort`/`lsearch` delegate
  here; the subtle `DictionaryCompare` port now lives once. `-command` (proc
  callback) and `-index` stay per-adapter.
- `binary::{hex,base64,uu}_{encode,decode}` + `format`/`scan` — value-model-free
  `&[u8]` codecs and the pack/unpack grammars. Each adapter bridges its value to
  bytes (the runtime's raw `obj_bytes`, the VM's byte-array `U+00xx` convention),
  so the codec between is identical. This is the **byte-oriented** family the plan
  scoped to Track B; sharing it lifted the VM from `binary`'s integer-only subset
  to the full code set (floats, 64-bit/big-endian ints, `encode`/`decode`) and
  gave it `binary`'s `errorCode`s. `scan`'s variable assignment stays in the
  adapter (the unpack core returns the values; the adapter sets the vars).

- `var::append_bytes` / `var::lappend_value` — the COW-aware *value computation*
  for `append`/`lappend`, over two new `ValueOps` rungs: a **byte-exact** seam
  (`as_bytes`/`new_bytes` + `try_append_bytes_in_place`) so `append` never routes
  binary data through the lossy char seam, and `try_list_append_in_place` for
  `lappend`. In-place amortised growth is preserved (the runtime grows an unshared
  value, returning the same object; the VM rebuilds), so a building loop stays
  O(1) per element, not O(n²).

`incr` is shared **only at the value seam**: all three sites (the VM's
`cmd_incr`, the VM's compiled `INCR_*` opcodes via `incr_var`, and the runtime's
`incr`) compute the new value through `ValueOps::int_add`, each over its own
native variable access. The trace-aware *store* and the const check stay in each
adapter on purpose — the contract's `VarStore::set` is storage-only and discards
the write-trace outcome, whereas C's `incr` must store yet fail when a write
trace errors. `append`/`lappend` follow the same split: the core computes the
value, the adapter does a **single store** (so the write trace fires once — the
common, user-visible case) and owns the const check and the no-argument read
forms.

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
| `append` (no-value unset) | the VM created an empty variable; now errors `can't read "x": no such variable` (tclsh) |
| `append`/`lappend` (in-place trace) | the runtime's in-place path skipped the store and so fired **no** write trace; now always stores → the trace fires once |
| `lappend` (no-value validate) | the runtime returned a malformed value unchanged; now validates it as a list and errors (`unmatched open brace in list`, tclsh) |
| `binary` (subcommands) | the VM lacked `encode`/`decode` and most `format`/`scan` codes (no floats/64-bit), and its bad-subcommand error said "must be format or scan"; routing gave it the full set + the tclsh message. The runtime's `base64` decode also gained its missing `TCL BINARY DECODE INVALID` errorCode. |
| `lsort` (VM modes) | `-dictionary` fell back to a byte compare, `-integer`/`-real` both used a `double` compare, and `-nocase` was absent; routing the shared `sort` core gave the VM the correct modes (numeric dictionary order, exact integer vs real, case folding, and mode-aware `-unique`). |

That is the payoff of the seam: one body (or one seam), enforced-identical
semantics, latent divergences caught.

## 3. What stays in the per-runtime adapter (the value/state split)

There is no longer a command family that *cannot* be shared at all — `incr`,
`append`, and `lappend` (the var-mutating commands) all route their **value
computation** through `tcl-cmd-core`. What stays per-runtime is the **state
mutation**, which is the point: a shared core that never names a runtime's frame
table, refcount discipline, or result protocol, paired with a thin adapter that
owns exactly those.

For `append`/`lappend` the adapter keeps three things, each genuinely
per-runtime or per-command:

- **The store + write trace.** The core returns the new value; the adapter does a
  single `var_set`, which fires the write trace once. This is deliberate — the
  contract's `VarStore::set` is storage-only (it discards the trace outcome),
  whereas a write trace that errors must store the value yet fail the command
  (C's `TclObjCallVarTraces`). The adapter maps that to its own protocol
  (`Completion` on the VM, set-result + `Code` on the runtime). "Always store,
  even when grown in place" is what fixes the runtime's old in-place-skips-the-
  trace bug; `store_scalar` is retain-then-release, so storing the in-place object
  back onto itself is alias-safe.
- **The const-variable check** (`const x` then `append x …`) — a runtime-only
  concept the VM has no notion of.
- **The no-argument read forms** — `append x` reads (erroring if unset),
  `lappend x` reads + validates-as-list (creating an empty list if unset). These
  differ per command and per runtime (the VM's value is UTF-8; the runtime's is
  bytes) and involve reads/misses, so they live in the adapter, not the core.

The **value-representation difference is bridged, not avoided**: the VM's value
is UTF-8 `Rc<str>`, the runtime's is raw bytes, so `append` works over the
byte-exact `as_bytes`/`new_bytes` rung (the runtime overrides them to its real
bytes; the VM uses the UTF-8 string-rep default — sound because a UTF-8-only
value only ever appends valid UTF-8). `lappend` is byte-exact for free: it
manipulates list *element values*, never their string rep. This is the
`ValueOps` "byte rung" the plan anticipated for `binary` (Track B), delivered
early by `append`.

## 4. Known contract gaps (no consumer yet — recorded, not fixed)

- The array-element methods (`get_elem`/`set_elem`/`unset_elem`/`exists_elem`)
  honour the active frame on both runtimes; the **non-active-frame** element path
  (`*_from`/`*_at`) is still scalar-only on the VM and ignored by the runtime
  (the element accessors take `FrameId` but use the active frame). No current
  consumer needs cross-frame element access.
- No enumeration surface (`info commands`/`vars`/`globals`, `namespace children`)
  — these need a listing API the state-mutation traits deliberately omit.
- `append`/`lappend` fire the write trace **once** over the whole operation, not
  per value (C's `append` fires per value). The user-visible common case — a
  write trace that runs on a mutating append — is covered; the exact count is
  not. The matching read-trace on the no-argument read forms is likewise not
  fired (the runtime's `var_get` doesn't). Recorded as an accepted simplification.

## 5. Recommended next steps (each its own decision)

1. ✅ *Done* — `VarStore` array-element methods; the `ValueOps::int_add`
   arithmetic seam (`incr` shared); `Namespaces::name`/`command_name`
   (`namespace current`/`which` shared); the `/`-based byte `path` core (`file`
   path-ops shared); the **byte-exact `ValueOps` rung** (`as_bytes`/`new_bytes` +
   `try_append_bytes_in_place`) and the **list in-place seam**
   (`try_list_append_in_place`) — `append`/`lappend` shared, in-place growth
   preserved, byte-exact.
2. The **`binary` family** can now build on the `as_bytes`/`new_bytes` rung — the
   biggest single dedup left (Track B).
3. `lset`/`linsert`-into-var can reuse the `try_list_append_in_place` pattern
   (a list in-place *replace* seam) if those are to share.
4. An enumeration surface on the contract for the `info`/`namespace` listing
   subcommands.
