# The Family-B contract and command routing

The Family-B runtime contract is the seam that lets one command body serve
both runtimes — the bytecode VM (`tcl-vm`) and the tree-walking interpreter
(`runtime/rust`). This document describes the contract, which command
families are lifted to shared cores in `tcl-cmd-core`, what stays in each
per-runtime adapter, and the contract's supported behavior and limitations.

## 1. The contract (`tcl-runtime-api`)

An interface crate depending on `tcl-core-types`, `tcl-dialect`, `tcl-lexer`
and `tcl-syntax`, holding the Family-B role traits, each generic over an
associated `Value`. Both the bytecode VM (`tcl-vm`,
`Value = Rc<Obj>`) and `runtime/rust` (`Value = *mut TclObj`) satisfy all of
them, so a consumer generic over the traits drives either runtime:

`ScriptCompletion` is the corresponding host/embedding boundary: a shared
`Completion<Vec<u8>>` whose result and return-options fields hold exact Tcl
string-representation bytes. Runtime engines snapshot into it before the
outermost evaluation publishes Tcl's error globals and resets the live
exception state, then perform that ordinary publication before returning to a
host. UTF-8 or another text conversion belongs only in an explicitly textual
host adapter; native, WASM, CLI, and library consumers do not each invent a
result/error conversion policy.

| Trait | Surface |
|-------|---------|
| `VarStore` | Byte `get_bytes`/`set_bytes`/`unset_bytes`/existence doors and Unicode conveniences, with explicit array-element access addressed by `FrameId`; `unset_command` preserves immutable-cell refusals for command consumers; `ArrayTarget` and the `*_at` rungs preserve a located array cell across callbacks when the runtime has stable `VarId`s |
| `Frames` | `push(NsId)`/`pop`/`current`/`link` (the `upvar` install), plus active-frame variable enumeration `in_proc()`/`var_names(include_links)`/`const_names()` |
| `Commands` | `dispatch(name, argv)` and `dispatch_id(CommandId, argv)` — the resolve-then-invoke pair with `find_command` |
| `Namespaces` | `find_command(cxt, name) -> CommandId`, `current() -> NsId`, `name(NsId) -> String`, `command_name(CommandId) -> Option<String>`, tree nav `find_namespace`/`parent`/`children`, and member enumeration `commands_in(NsId)`/`procs_in(NsId)`/`vars_in(NsId)`/`consts_in(NsId)` |
| `Traces` | `fire(var, op)` (read/write/unset; read/write errors abort) |
| `Introspect` | `level()`, `level_argv(n)` |
| `Procs` | Checked `proc_info_bytes` and Unicode `proc_info` expose byte body/formal metadata; default queries retain the original runtime value object for `info default` |

The **value seam** (`ValueOps` in `tcl-syntax`) has an arithmetic rung,
`int_add(Option<&V>, &V) -> Result<V, ValueError>`, whose `Option` left operand
folds in "absent value = 0". Its default is fixed-`i64` with overflow →
`ValueError::IntegerOverflow`; the bignum runtime overrides it to widen. This is
the seam used by shared `incr` (§2) without the core naming a number
representation.

The string rung retains exact native bytes. `as_bytes` and `new_bytes` are
required adapter operations; `try_as_str` and `try_char_len` are fallible.
The VM's `RawString` keeps invalid UTF-8 and surrogate encodings intact and
caches only a checked Unicode projection. Byte-capable list and dictionary
operations preserve element objects and compare original key bytes.
`CmdError` carries byte-valued guest results separately from a typed host-access
refusal. Adapters check that refusal before publishing a guest completion;
guest `catch` cannot intercept it and execution cannot replay an already
reached operation. Unicode-only consumers decline unsupported byte inputs; the byte constructor
alone supplies no Unicode projection. Native source and name consumers use their
selected byte protocols rather than route through that textual view.

Arbitrary-precision `format` conversions use
`ValueOps::integer_magnitude(value, radix, syntax)`. The adapter returns a sign
and unsigned lowercase digits without a radix prefix, under the selected
release's numeral grammar. The VM uses its bignum value model and the native
runtime uses libtommath; neither narrows this path through `i64`. Fixed-width
format conversions on Tcl 8.5+/9 also use that magnitude seam: fixed-width
`d`/`i`/`u`/`x`/`X`/`o`/`b`/`p` operands are reduced modulo 2^64 before the
selected `short`/`int`/`wide` width is applied. Tcl 8.4 and Jim retain their
legacy wide-integer coercion and overflow behavior. This truncation is local to
the formatter; dynamic width and precision arguments, `%c`, and unbounded
`ll`/`L` conversions retain their existing paths. The shared formatter owns
modifier selection, prefixes, case, precision, padding, and the structured
`TCL FORMAT BADUNSIGNED` error for negative unsigned bignum conversions.

Notes:
- `CompileService`, the runtime-`eval` injection point, uses an associated
  `Module` type so the contract crate carries no bytecode dependency.
  `tcl-cmd-core` depends on these traits.
- Handle bridges: the VM is string/`i64`-native, so it interns namespace names
  (`NsId`) and command FQNs (`CommandId`) in side-table arenas; `runtime/rust`'s
  `NsId`/`Code` are distinct types from the contract's and are mapped explicitly.
- `CommandId` is produced by `find_command` and consumed by `dispatch_id`
  (reverse-mapped to the absolute FQN, then dispatched) — that pairing closed the
  "handle with no consumer" gap.

## 2. What is shared, and where

The split: **value-shaped** command bodies are shared
*concrete code* in `tcl-cmd-core` (generic over `ValueOps`, plus a role trait for
the stateful ones); **stateful** commands are, in general, *trait calls*, not a
shared body.

Shared in `tcl-cmd-core`:
- Value families: `string`, `list`, `dict`, `format`, `scan`,
  `index`, `string is`, plus `platform`/`path` helpers.
- `info::level` — over `Introspect` + `ValueOps`.
- `info::exists` — over `VarStore::exists` + `Frames::current`.
- `info::complete` — pure (`Tcl_CommandComplete`).
- `info::{body, args, default}` — the **proc-introspection** subcommands, over a
  `Procs` role trait (`proc_info(name) -> Option<ProcInfo>`, returning the
  proc's body + formals as plain owned bytes so the contract stays value-agnostic
  and a byte-oriented runtime never mints fresh result objects inside a `&self`
  query). The core resolves the proc (or raises the shared `"name" isn't a
  procedure` / `procedure "name" doesn't have an argument "arg"` errors) and
  builds the result through `ValueOps`. `info default`'s var-write stays
  per-adapter (trace-aware, like `incr`/`array set`): the core returns the
  `(value, has_default)` pair, the adapter does the single store and returns the
  bool. Both runtimes resolve imported procs through their `proc_def`; the core
  owns the shared error catalogue.
- `info::command_list` — `info commands`/`procs` (a `procs_only` flag selects the
  latter), over two `Namespaces` enumeration rungs (`commands_in`/`procs_in`,
  returning a namespace's direct command/proc members as unqualified tails — the
  command-table analogue of `VarStore::array_keys`). The core owns the whole
  namespace-aware listing: the qualified-pattern split on the last `::`,
  re-qualification through the target namespace's canonical name, the glob filter,
  and the **global-merge asymmetry** (C's `InfoCommandsCmd` merges the global
  namespace into an unqualified `info commands`; `InfoProcsCmd` never merges, so
  `procs` lists the current namespace only). The runtime implements the rungs over
  its namespace arena's command table; the VM over its flat command map (keyed by
  canonical name, so direct membership is a prefix test).
- `info::{vars, locals, globals}` — the variable-listing subcommands, over a
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
  `vars_in` enumerates a namespace's direct variable bindings; `commands_in`
  enumerates its direct commands. The frame interfaces read the active
  frame's table.
  `info::consts` adds the binding-specific constant rungs
  (`Frames::const_names`/`Namespaces::consts_in`). They inspect the direct
  binding rather than following an ordinary link: `info constant alias` follows
  an alias, but `info consts` enumerates only direct constants and typed TclOO
  instance projections. At namespace scope the core merges unshadowed global
  constants for scans and preserves Tcl's direct-lookup fallback for a
  metacharacter-free exact pattern. Its byte entry points keep runtime names
  lossless through glob matching and result construction.
- `array::{dispatch, dispatch_at}` — the `array` **read-side** (`exists`/`size`/`names`/`get`)
  + `unset`, over `VarStore` + `Frames` + `ValueOps`.
  `VarStore::array_keys` enumerates an array's element keys and returns `None`
  for a scalar or unset target. `ArrayTarget` is the operation-scoped
  LocateArray result. Both adapters select the array-operation trace operand
  through `InvocationFacts::sole_argument_index_for_roles`; the helper applies
  the dialect-selected member arity and resolver-first argument roles before
  returning an argv index. A stable-cell runtime retains the cell and its direct
  binding shell across the command, and routes `exists`/`size`/`names`, the
  key half of `get`, and patterned `unset` through its `VarId`; the shared core
  deliberately re-resolves the spelling for `get` values and whole-array
  `unset`, matching Tcl's post-operation-trace target policy. Whole-array
  mutation uses the fallible `VarStore::unset_command` rung, so a trace that
  retargets the live spelling to a Tcl 9 constant keeps its structured
  `TCL UNSET CONST` refusal in either adapter. The trace-aware
  value read required by `array get` remains tracked in #1932. `array set`'s per-element write-trace store
  stays per-adapter (`VarStore::set_elem` is storage-only, like `incr`/`append`);
  `array default`/`array for` stay per-adapter (TIP 508 state / Family-B
  iteration), with shared storage rungs for physical search keys, live candidate
  existence, and active-search revision. `array unset a` with no pattern removes
  the **whole array**.
- `prefix::{OptionTable, scan}` — the `Tcl_GetIndexFromObjStruct` port: one
  unique-prefix matcher and one `bad <noun> "X": must be …` formatter for every
  option and subcommand table, static or runtime-built.
- `ensemble::…` — the `namespace ensemble` option tables (`create` and
  `configure` do not share one), the subcommand scan over `prefix::scan`, and
  the ensemble-flavoured `must be …` enumeration wording.
- `namespace::{tail, qualifiers}` — pure byte ops.
- `namespace::{current, which_command}` — over `Namespaces` (`current`/`name`/
  `command_name`/`find_command`).
- `namespace::{exists, parent, children}` — the namespace-tree **navigation**
  subcommands, over three `Namespaces` rungs (`find_namespace`/`parent`/
  `children`) that mirror C's `Namespace` struct directly: a namespace **is** a
  handle (`NsId` = C's `nsId` / `Tcl_Namespace*` identity), and its FQN/parent/
  children are queried *from* it (`Tcl_FindNamespace`/`parentPtr`/`childTable`).
  This is the handle-model answer (not a name-based shortcut): it matches the C
  reference and composes for the harder ops (`eval`/`import`/`export`/`upvar` all
  address namespaces by identity). The VM's exact `ByteNamespacePath` namespace model honours the
  handles via its `ns_arena`/`ns_intern` id arena (every namespace interned on
  creation, so the `&self` nav methods are pure lookups). `export`/`import`/
  `eval`/`delete` stay per-adapter (namespace *state*/control, needing heavier
  surface). `namespace children` honours its `?pattern?`, and `parent`/`children`
  on a missing namespace errors `namespace "X" not found`.
- `path::{tail, dirname, extension, rootname}` — a `/`-based **byte** path core,
  platform-independent.
- `mathop::eval` — `::tcl::mathop::*` (every `expr` operator as a command) over
  the existing `ExprOps` seam, through the same value
  seam: the fold/identity/chain logic is shared, each primitive going through
  each runtime's `ExprOps` (the WASM runtime's bignum tower, the VM's i64+double).
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
  `$`/`[` is passed literally); the runtime's through `interp.dispatch`.
- `lsearch` — the **whole** `lsearch` command (every option: `-exact`/`-glob`/
  `-regexp`/`-sorted`/`-bisect`, `-all`/`-inline`/`-not`, the four `-ascii`/
  `-dictionary`/`-integer`/`-real` types, `-nocase`, `-increasing`/`-decreasing`,
  `-start`, `-stride`, `-index` *path*, `-subindices`), the sorted binary search,
  and the stride / sub-index result shapes — over `ValueOps` + the `RegexEngine`
  provider (`-regexp` uses the selected shared regex engine). The C search
  computation returns its result through the adapter. Jim's separate
  `native_jim_lsearch` owner retains original List members, selected command
  heads and integer callback results; callbacks can mutate state, and native
  result publication and failure ordering remain part of the contract. `-index` path
  resolution goes through the shared `index::{resolve_opt, encodable}`.
- `clock::dispatch` — the `clock` command,
  implemented over `ValueOps`: `seconds`/`milliseconds`/`microseconds`/`clicks`,
  `format` (the civil-date strftime specifiers — incl. Tcl's quirks: `%D`/`%x`
  use a 4-digit year, and an unknown specifier like `%F` passes through verbatim),
  and `add` (count/unit arithmetic incl. calendar months/years). The civil↔days
  math is Hinnant's branch-free algorithm. The command stays **host-free**: the
  per-runtime adapter reads the current time from its host's
  `Clock` capability and passes it in as a `Now` plus a
  `local_offset(ts)` callback, so the core never touches the host. Each runtime
  retains its own `Rc<dyn Host>`. The `Clock` trait supplies `now_micros` and
  `local_offset_secs`; the standard host has no timezone database, so its local
  time equals UTC. `format`/`scan` comparisons against a fixed instant use
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
  bitset, the VM keeps the name list. The trace *engines*
  (the VM fires variable traces only; the runtime fires all three) stay
  per-adapter. `catch` also remains adapter-owned: body evaluation,
  completion→`(code,result,options)` mapping and the `-errorcode`/`-errorinfo`/
  `-errorstack`/`-during` options dict use each runtime's error accumulator.
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
  for `select` and frees them (leak-gate-validated). The shared
  `select` is exercised on the VM via `-glob`/`-regexp` (exact switches are
  codegen-inlined), and on the runtime (a tree-walker, always calling the builtin)
  across the whole option/error surface — both pinned vs tclsh 9.0.
- `string::word_bound` — `string wordstart`/`wordend`, the word-boundary scan
  over the Unicode word-char + connector-punctuation classification, in the
  shared `string` dispatch.
- `dict::filter` — `dict filter key|value ?glob ...?` (the pure glob-filter half).
  The `script` filter type evaluates a body per pair (Family-B) and stays in each
  adapter, so the core returns `None` for it. The filterType is validated
  **before** the dict is parsed (`dict filter {a b c} bogus` → "bad filterType",
  not the dict error).
- `binary::{hex,base64,uu}_{encode,decode}` + `format`/`scan` — value-model-free
  `&[u8]` codecs and the pack/unpack grammars. Each adapter bridges its value to
  bytes through the selected native conversion policy (the runtime's raw
  `obj_bytes`, the VM's retained `ByteArray` payload or exact `RawString` bytes),
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
  `TCL_NUMBER_BIG`), so the shared `Num` carries a fixed `i64`/`f64` pair.
- `regex::{regexp, regsub}` — the `regexp`/`regsub` **command plumbing** (option
  parsing, the match/advance loop, `-indices`/`-inline`/`-start`/`-all` handling,
  submatch-variable assignment, the `regsub` substitution-spec expansion, and the
  match-count semantics) over a `RegexEngine` **provider trait** + `ValueOps`.
  Both ports use the shared `tcl-regex` provider, with independently selected
  C ARE and Jim integer-program semantics. Character offsets, original pattern
  cache ownership, native flags and error publication follow the actual engine
  protocol. Original variable operands and callback completions remain adapter
  responsibilities; matching an output string does not certify those roles.
  Shared command preparation retains the original option and target objects
  rather than recreating them from a byte-only argv projection.
- `var::append_bytes` / `var::lappend_value` — the COW-aware *value computation*
  for `append`/`lappend`, over two `ValueOps` rungs: a **byte-exact** seam
  (`as_bytes`/`new_bytes` + `try_append_bytes_in_place`) so `append` never routes
  binary data through the lossy char seam, and `try_list_append_in_place` for
  `lappend`. In-place amortised growth is preserved (the runtime grows an unshared
  value, returning the same object; the VM rebuilds), so a building loop stays
  O(1) per element, not O(n²).

`incr` shares numeric and update protocols while each runtime supplies its
physical cell, selected getter, alias lifetime and callback owner. The selected
native protocol determines operand-conversion order and missing-content
behavior. A numeric result alone cannot prove that the actual store completed.

## 3. What stays in the per-runtime adapter

The portable core owns value operations and native update recipes. Each adapter
owns reached variable lookup, captured receiver lifetime, object sharing,
mutable interpreter state and guest or host completion publication.

`native_append` retains each original source object and selects the authentic
append protocol. C writes after each operand; Jim writes the batch. The adapter
keeps the original receiver across callbacks rather than re-resolve a same-named
replacement. Generic `lappend` retains its selected read and list-validation
behavior and stores the completed list once. A compiler-selected list-update
operation can have a different captured receiver contract; its admission
receipt is separate from generic command dispatch.

No-value `append` and `lappend` forms remain adapter operations. `append`
fires its read observer and propagates a read failure. `lappend` retains its
selected failed-read handling and empty-result validation before storing an
absent value as an empty list. Reached traces, immutable-cell checks and result
ownership remain part of those native operation contracts.

Both concrete value models implement the byte-exact `as_bytes`/`new_bytes`
rung. `append` concatenates those bytes; it does not request a Unicode view.
`lappend` manipulates list element objects, retaining their byte values and
native representations. `binary` uses the same byte rung while keeping native
string-to-byte conversion separate from an existing byte-array payload.

## 4. Known contract gaps

- The array-element methods (`get_elem`/`set_elem`/`unset_elem`/`exists_elem`)
  honour the `FrameId` they are given on both runtimes. `VarStore::array_keys`
  does not: `runtime/rust` resolves it against the active frame regardless of the
  frame passed. No current consumer needs cross-frame array enumeration.
- The enumeration surface is complete for the shared listing subcommands:
  `VarStore::array_keys` (array elements), `Namespaces::commands_in`/`procs_in`/
  `vars_in`/`consts_in` (a namespace's commands/procs/variables/constants), and
  the active-frame `Frames::var_names`/`const_names`/`in_proc` (frame locals,
  links, and constant-visible bindings). These back `info commands`/`procs`/
  `vars`/`locals`/`globals`/`consts` and `array names`. Constant enumeration is
  binding-specific: automatic TclOO instance projections are visible, while
  ordinary links are not; the singular `info constant` query follows both.
- `regexp`/`regsub` divergence from tclsh is confined to the engine (the shared
  layer supplies the shared command interfaces). Both runtimes use the same
  pure-Rust ARE engine (`tcl-regex`), so ARE-only syntax — `\m`/`\M`/`[[:<:]]`
  word edges, POSIX longest-match submatches — behaves alike on both and matches
  tclsh. The engine drives *slices* `text[offset..]`
  (+ `REG_NOTBOL`) rather than tclsh's whole-string+offset, so a **truly-empty**
  pattern at end-of-string differs (`regsub -all {} abc X` → `XaXbXcX` on both
  runtimes vs tclsh 8.6.18/9.0.4's `XaXbXc`).
- `regexp -about` and `regsub -command` are served on both runtimes (#2124).
  `-about` is an ordinary answer on every release. `-command` is versioned by
  the interpreter's pinned release, because its option table is: a `bad switch
  "-command"` through 8.5, a `bad option "-command"` on 8.6, served from 9.0
  (tclsh 8.4.20/8.5.19/8.6.18 vs 9.0.4/9.1b0). Serving it *is* Family-B — the
  prefix is a command invocation — so the core takes the evaluator as a
  closure (`tcl_cmd_core::regex::regsub_eval`, the `lsort -command` shape) and
  each adapter supplies its own dispatch. A prefix that fails propagates its
  completion code unchanged and, for an error, gains C's
  `\n    (-command substitution computation script)` `errorInfo` frame. The
  evaluator-less `tcl_cmd_core::regex::regsub` remains for the registry's
  const-folder, which cannot run script and declines `-command`.
