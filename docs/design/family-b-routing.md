# The Family-B contract and command routing

The Family-B runtime contract is the seam that lets one command body serve
both runtimes — the bytecode VM (`tcl-vm`) and the tree-walking interpreter
(`runtime/rust`). This document describes the contract, which command
families are lifted to shared cores in `tcl-cmd-core`, what deliberately
stays in each per-runtime adapter, and the gaps the contract does not yet
close.

## 1. The contract (`tcl-runtime-api`)

A light leaf crate (depends only on `tcl-core-types`) holding the Family-B role
traits, each generic over an associated `Value`. Both the bytecode VM (`tcl-vm`,
`Value = Rc<Obj>`) and `runtime/rust` (`Value = *mut TclObj`) satisfy all of
them, so a consumer generic over the traits drives either runtime:

| Trait | Surface |
|-------|---------|
| `VarStore` | `get`/`set`/`unset`/`exists` + the explicit array-element pairs `get_elem`/`set_elem`/`unset_elem`/`exists_elem`, addressed by `FrameId` |
| `Frames` | `push(NsId)`/`pop`/`current`/`link` (the `upvar` install), plus active-frame variable enumeration `in_proc()`/`var_names(include_links)` |
| `Commands` | `dispatch(name, argv)` and `dispatch_id(CommandId, argv)` — the resolve-then-invoke pair with `find_command` |
| `Namespaces` | `find_command(cxt, name) -> CommandId`, `current() -> NsId`, `name(NsId) -> String`, `command_name(CommandId) -> Option<String>`, tree nav `find_namespace`/`parent`/`children`, and member enumeration `commands_in(NsId)`/`procs_in(NsId)`/`vars_in(NsId)` |
| `Traces` | `fire(var, op)` (read/write/unset; read/write errors abort) |
| `Introspect` | `level()`, `level_argv(n)` |
| `Procs` | `proc_info(name) -> Option<ProcInfo>` (a proc's body + formals, for `info body`/`args`/`default`) |

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
- `info::{body, args, default}` — the **proc-introspection** subcommands, over a
  new `Procs` role trait (`proc_info(name) -> Option<ProcInfo>`, returning the
  proc's body + formals as plain owned bytes so the contract stays value-agnostic
  and a byte-oriented runtime never mints fresh result objects inside a `&self`
  query). The core resolves the proc (or raises the shared `"name" isn't a
  procedure` / `procedure "name" doesn't have an argument "arg"` errors) and
  builds the result through `ValueOps`. `info default`'s var-write stays
  per-adapter (trace-aware, like `incr`/`array set`): the core returns the
  `(value, has_default)` pair, the adapter does the single store and returns the
  bool. Both runtimes already resolved imported procs through their `proc_def`;
  the share keeps that and unifies the error catalogue.
- `info::command_list` — `info commands`/`procs` (a `procs_only` flag selects the
  latter), over two new `Namespaces` enumeration rungs (`commands_in`/`procs_in`,
  returning a namespace's direct command/proc members as unqualified tails — the
  command-table analogue of `VarStore::array_keys`). The core owns the whole
  namespace-aware listing: the qualified-pattern split on the last `::`,
  re-qualification through the target namespace's canonical name, the glob filter,
  and the **global-merge asymmetry** (C's `InfoCommandsCmd` merges the global
  namespace into an unqualified `info commands`; `InfoProcsCmd` never merges, so
  `procs` lists the current namespace only). The runtime implements the rungs over
  its namespace arena's command table; the VM over its flat command map (keyed by
  canonical name, so direct membership is a prefix test). This lifted the VM from a
  flat "all command keys" listing (which leaked namespaced names into the global
  scope and mishandled `::ns::*` patterns) to the correct behaviour, and **fixed a
  runtime bug** (`info procs` in a namespace wrongly merged global procs). The
  variable-listing subcommands stay per-adapter for now (see `info::{vars,…}`).
- `info::{vars, locals, globals}` — the variable-listing subcommands, over a new
  `Namespaces::vars_in` (a namespace's variables, the variable analogue of
  `commands_in`) plus two active-frame `Frames` rungs (`in_proc()` and
  `var_names(include_links)`). `info vars` is the context-sensitive one (C's
  `InfoVarsCmd`): a qualified pattern lists that namespace re-qualified (the same
  `qualified_listing` helper commands/procs use); unqualified **in a proc** lists
  the frame's own variables — locals *and* `upvar`/`global`/`variable` links by
  alias (`var_names(true)`); unqualified **at namespace scope** lists the current
  namespace's variables. `info locals` is the frame's genuine locals only
  (`var_names(false)`); `info globals` is the global namespace's variables
  (`vars_in(ROOT)`), with the Bug 1057461 leading-`::` pattern strip in the core.
  This resolved the "VM has no namespace variables" block: the VM *does* store
  them — in the global frame keyed by qualified name (`foo::v`), exactly as it
  keys commands — so `vars_in` is the same direct-membership prefix test as
  `commands_in`, and the frame rungs read the active frame's table. Routing split
  `info vars` from `info locals` on the VM (it had aliased them, so `info vars` in
  a proc dropped its links) and gave `info globals` the global-only filter.
  `info consts` (TIP 677) stays per-adapter — the VM has no `const`.
- `array::dispatch` — the `array` **read-side** (`exists`/`size`/`names`/`get`)
  + `unset`, over `VarStore` + `Frames` + `ValueOps`. This is the first stateful
  *command* family shared over the interp-state seam (the `info`/`namespace`
  entries above are individual subcommands). It needed one new contract rung,
  `VarStore::array_keys` — the **enumeration** surface the otherwise-listing-free
  state traits expose, returning an array's element keys (or `None` for a
  scalar/unset, the existence signal). `array set`'s per-element write-trace store
  stays per-adapter (`VarStore::set_elem` is storage-only, like `incr`/`append`);
  `array default`/`array for` stay per-adapter (TIP 508 state / Family-B
  iteration). Routing fixed a VM bug: `array unset a` with no pattern now removes
  the **whole array** (was: iterate-and-unset elements, leaving an empty array).
- `namespace::{tail, qualifiers}` — pure byte ops.
- `namespace::{current, which_command}` — over `Namespaces` (`current`/`name`/
  `command_name`/`find_command`).
- `namespace::{exists, parent, children}` — the namespace-tree **navigation**
  subcommands, over three new `Namespaces` rungs (`find_namespace`/`parent`/
  `children`) that mirror C's `Namespace` struct directly: a namespace **is** a
  handle (`NsId` = C's `nsId` / `Tcl_Namespace*` identity), and its FQN/parent/
  children are queried *from* it (`Tcl_FindNamespace`/`parentPtr`/`childTable`).
  This is the handle-model answer (not a name-based shortcut): it matches the C
  reference and composes for the harder ops (`eval`/`import`/`export`/`upvar` all
  address namespaces by identity). The VM's String namespace model honours the
  handles via its `ns_arena`/`ns_intern` id arena (every namespace interned on
  creation, so the `&self` nav methods are pure lookups). `export`/`import`/
  `eval`/`delete` stay per-adapter (namespace *state*/control, needing heavier
  surface). Routing also fixed two VM bugs: `namespace children` ignored its
  `?pattern?`, and `parent`/`children` on a missing namespace returned a computed
  result instead of erroring `namespace "X" not found`.
- `path::{tail, dirname, extension, rootname}` — a `/`-based **byte** path core
  (platform-independent), replacing the VM's old `std::path::Path` versions.
- `mathop::eval` — `::tcl::mathop::*` (every `expr` operator as a command) over
  the existing `ExprOps` seam, so **no new value
  seam**: the fold/identity/chain logic is shared, each primitive going through
  each runtime's `ExprOps` (the WASM runtime's bignum tower, the VM's i64+double).
  The VM had no `mathop` at all; it now has the full operator set.
- `sort::{key_compare, dictionary_compare, parse_wide, parse_real}` — the
  `lsort`/`lsearch` comparison modes (`-ascii`/`-dictionary`/`-integer`/`-real`,
  `-nocase`), pure `&[u8] → Ordering`. The subtle `DictionaryCompare` port lives
  once; the full `lsort`/`lsearch` commands (below) build on it.
- `lsort` — the **whole** `lsort` command (the option set, up-front numeric-key
  validation, mode-aware `-unique`, `-index` path, `-stride`, `-indices`), over
  `ValueOps` + a `-command` comparator callback. `-command` evaluates a user Tcl
  proc per comparison (Family-B), so — like `lseq`'s expression edge — it is split
  into three sequential calls so the interp the comparator evaluates against is
  not double-borrowed: `prepare` (everything needing `ValueOps`, including the
  full non-command sort+build), `sort_command` (the reentrant stable merge sort
  over the adapter's comparator, **no `ValueOps`**), and `build_command`. The VM's
  comparator goes through `vm.dispatch` (argv-based, so an element containing
  `$`/`[` is passed literally); the runtime's through `interp.dispatch`. This
  lifted the VM from a flat-comparison-only `lsort` to `-index`/`-stride`/
  `-indices`/`-command`.
- `lsearch` — the **whole** `lsearch` command (every option: `-exact`/`-glob`/
  `-regexp`/`-sorted`/`-bisect`, `-all`/`-inline`/`-not`, the four `-ascii`/
  `-dictionary`/`-integer`/`-real` types, `-nocase`, `-increasing`/`-decreasing`,
  `-start`, `-stride`, `-index` *path*, `-subindices`), the sorted binary search,
  and the stride / sub-index result shapes — over `ValueOps` + the `RegexEngine`
  provider (`-regexp` reuses the engine seam: the real ARE engine on the runtime,
  the `regex` crate on the VM). `lsearch` never writes a variable, so it is a pure
  value→value function; the adapter only maps the result/error. The `-index` path
  resolution moved to the shared `index::{resolve_opt, encodable}`. This lifted
  the VM from a `-exact`/`-glob`-only stub to the full command.
- `clock::dispatch` — the **net-new** `clock` command (neither runtime had it),
  written once over `ValueOps`: `seconds`/`milliseconds`/`microseconds`/`clicks`,
  `format` (the civil-date strftime specifiers — incl. Tcl's quirks: `%D`/`%x`
  use a 4-digit year, and an unknown specifier like `%F` passes through verbatim),
  and `add` (count/unit arithmetic incl. calendar months/years). The civil↔days
  math is Hinnant's branch-free algorithm. The command stays **host-free**: the
  per-runtime adapter reads the current time from its host's
  `Clock` capability and passes it in as a `Now` plus a
  `local_offset(ts)` callback, so the core never touches the host (resolving the
  same `ops`+host borrow the `exec` slice hit, via each runtime's owned
  `Rc<dyn Host>`). The `Clock` trait grew `now_micros` + `local_offset_secs`; the
  std host has no timezone database, so local time currently equals UTC (a host
  with TZ data plugs in later) — `format`/`scan` against a fixed instant use
  `-gmt 1` for determinism. `clock scan -format` (the inverse — parse an input
  per a format with the `%b`/`%s` etc. specifiers, base-date defaulting, and the
  `invalid month` / `does not match` errors) is implemented; only **free-form**
  `clock scan "next tuesday"` (Tcl's natural-language date grammar) remains.
  Pinned vs tclsh 9.0 on both runtimes (runtime leak-gate clean).
- `trace::{parse_ops, bad_type_error}` — the `trace` **argument decoding**: the
  op-list parser (split + per-type validation of `read`/`write`/`unset`/`array`,
  `rename`/`delete`, `enter`/`leave`/`enterstep`/`leavestep`) and the `bad type` /
  `bad operation` catalogue. `trace` is heavily stateful (each runtime owns its
  trace tables and the firing wired into variable/command/execution access), so
  only the decoding is shared; the runtime folds the canonical op names into its
  bitset, the VM keeps the name list. This fixed two VM bugs: it did **no** op
  validation (`trace add variable v bogus cmd` was silently accepted) and used the
  wrong type error (`bad type "X": must be command, execution, or variable` vs C's
  `bad option "X": must be execution, command, or variable`). The trace *engines*
  (the VM fires variable traces only; the runtime fires all three) stay
  per-adapter. (`catch` was assessed and **kept per-adapter**: its body eval +
  completion→`(code,result,options)` mapping and the `-errorcode`/`-errorinfo`/
  `-errorstack`/`-during` options dict are built from each runtime's own error
  accumulator, with almost no representation-independent logic to share.)
- `switch::{parse_options, select}` — the `switch` **decision** logic: the option
  table (`-exact`/`-glob`/`-regexp`/`-nocase`/`-indexvar`/`-matchvar`/`--`, with
  the prefix-matching + the error catalogue), and the value/pattern selection
  across all three modes — incl. `default`-only-as-final-pattern, and the regexp
  mode driving the shared `RegexEngine` provider to build the TIP #75
  `-matchvar`/`-indexvar` values. Like `lsort -command`, `switch` evaluates a body
  script, so only the decision is shared: each adapter keeps the pattern/body pair
  extraction (the inline vs. brace-list forms — the runtime with its `info frame`
  line tracking), the `-` fall-through resolution, the trace-aware variable
  writes, and the transparent body eval. The runtime's list-form patterns are
  sub-strings of a literal (no `Tcl_Obj`), so the adapter mints temporary objects
  for `select` and frees them (leak-gate-validated). This lifted the VM from a
  basic exact/glob `switch` (regexp fell back to exact; `default` matched
  anywhere; no `-matchvar`/`-indexvar`) to the full superset, and deduplicated the
  runtime's 770-line implementation down to its per-target edges. The shared
  `select` is exercised on the VM via `-glob`/`-regexp` (exact switches are
  codegen-inlined), and on the runtime (a tree-walker, always calling the builtin)
  across the whole option/error surface — both pinned vs tclsh 9.0.
- `string::word_bound` — `string wordstart`/`wordend` (the word-boundary scan
  over the Unicode word-char + connector-punctuation classification), added to the
  shared `string` dispatch. The VM lacked these entirely (it errored "not yet
  implemented"); the runtime's hand-rolled `str_word` is deleted.
- `dict::filter` — `dict filter key|value ?glob ...?` (the pure glob-filter half).
  The `script` filter type evaluates a body per pair (Family-B) and stays in each
  adapter, so the core returns `None` for it. The VM had no `dict filter` at all;
  it now has key/value (shared) plus a small script adapter. Routing also fixed a
  runtime ordering bug — the filterType is now validated **before** the dict is
  parsed (`dict filter {a b c} bogus` → "bad filterType", not the dict error).
- `binary::{hex,base64,uu}_{encode,decode}` + `format`/`scan` — value-model-free
  `&[u8]` codecs and the pack/unpack grammars. Each adapter bridges its value to
  bytes (the runtime's raw `obj_bytes`, the VM's byte-array `U+00xx` convention),
  so the codec between is identical. This is the **byte-oriented** family: the
  shared core owns the full code set (floats, 64-bit and big-endian ints,
  `encode`/`decode`) and the `errorCode`s. `scan`'s variable assignment stays in the
  adapter (the unpack core returns the values; the adapter sets the vars).

- `lseq::{decode, generate}` — the `lseq` arithmetic-sequence generator (the
  argument-decode key, the `..`/`to`/`count`/`by` keywords, the int-vs-double
  selection, and the `maxObjPrecision`/`ArithRound` precision matching). The split
  is what makes it shareable despite the **expression-valued-argument** edge
  (`lseq $n*2 to 10`): `decode` runs the argument state machine over an injected
  `eval_expr` callback (so the core never names an interp), `generate` builds the
  element list over `ValueOps` — **two separate calls**, so a runtime whose
  value-ops *is* its interp runs the eval callback first (interp borrowed by the
  closure) and the generation second (interp borrowed as the ops) without a borrow
  conflict. `lseq` is `i64`-based on both runtimes (C's `assignNumber` rejects
  `TCL_NUMBER_BIG`), so the shared `Num` carries a fixed `i64`/`f64` pair. This
  lifted the VM from **no `lseq` at all** to the full command.
- `regex::{regexp, regsub}` — the `regexp`/`regsub` **command plumbing** (option
  parsing, the match/advance loop, `-indices`/`-inline`/`-start`/`-all` handling,
  submatch-variable assignment, the `regsub` substitution-spec expansion, and the
  match-count semantics) over a new `RegexEngine` **provider trait** + `ValueOps`.
  This is the explicit *engine-divergence* share: the contract (compile / `nsub` /
  codepoint-offset `exec`) is identical, the **engine is not** — `runtime/rust`
  drives the real linked Tcl ARE engine (byte-for-byte tclsh), the VM drives the
  Rust `regex` crate (approximate; full ARE like `\m`/`\M`/`[[:<:]]` out of scope).
  The seam is **character**-offset based (Tcl's index model), so the VM's crate
  engine translates byte↔char behind it (`captures_at` for context-correct `^`/`\b`
  at resumed offsets — the `notbol` hint is then unneeded). The var writes (match
  vars / result var, with the const check) stay per-adapter (Family-B). The pure
  `decode_utf8` moved here as the canonical copy (the runtime's `regex.rs` re-exports
  it). This lifted the VM substantially: it gained `-indices`/`-start`, the full
  option set, char-correct offsets, the tclsh error messages, and the
  vars-untouched-on-no-match rule it was getting wrong.
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
`ValueOps` byte rung, which `binary` also builds on.

## 4. Known contract gaps

- The array-element methods (`get_elem`/`set_elem`/`unset_elem`/`exists_elem`)
  honour the active frame on both runtimes; the **non-active-frame** element path
  (`*_from`/`*_at`) is still scalar-only on the VM and ignored by the runtime
  (the element accessors take `FrameId` but use the active frame). No current
  consumer needs cross-frame element access.
- The enumeration surface is complete for the shared listing subcommands:
  `VarStore::array_keys` (array elements), `Namespaces::commands_in`/`procs_in`/
  `vars_in` (a namespace's commands/procs/variables), and the active-frame
  `Frames::var_names`/`in_proc` (frame locals + links). These back `info
  commands`/`procs`/`vars`/`locals`/`globals` and `array names`. The one
  listing left per-adapter is `info consts` (TIP 677, runtime-only — the VM
  has no `const`). The VM stores namespace variables flat, in the global
  frame keyed by qualified name, so the rungs read them directly.
- `append`/`lappend` fire the write trace **once** over the whole operation, not
  per value (C's `append` fires per value). The user-visible common case — a
  write trace that runs on a mutating append — is covered; the exact count is
  not. The matching read-trace on the no-argument read forms is likewise not
  fired (the runtime's `var_get` does not). This is an accepted simplification.
- `regexp`/`regsub` engine divergence is deliberate (the shared layer is the
  *plumbing*, not the engine): (a) the VM's `regex` crate is not full ARE, so
  ARE-only syntax (`\m`/`\M`/`[[:<:]]`, some back-reference forms) compiles on the
  runtime but errors on the VM; (b) the runtime's engine driving *slices*
  `text[offset..]` (+ `REG_NOTBOL`) rather than tclsh's whole-string+offset, so a
  **truly-empty** pattern at end-of-string diverges (`regsub -all {} abc X` →
  `XaXbXcX` vs tclsh's `XaXbXc`) — a pre-existing low-level quirk, not introduced
  by the share; (c) `regexp -about` and `regsub -command` are `not yet supported`
  on both (the latter invokes a proc — Family-B).
