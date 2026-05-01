# WASM runtime — compiler-to-interpreter bridge primitives

The Zig runtime (`runtime/zig/tcl_runtime.wasm`) is the execution
partner of the WASM code we emit from `core/compiler/codegen/wasm/`.
Compiled code calls Zig-exported helpers for the primitives it cannot
statically compile (list parsing, string operations, the full Tcl
interpreter for untypable constructs).  This doc lists the boundary
primitives added to support Tcl 9 compatibility, grouped by the
contract each one encodes.

Source: [`runtime/zig/`](../../../runtime/zig/),
[`core/compiler/codegen/wasm/`](../../../core/compiler/codegen/wasm/)

## Expression evaluation

### `tcl_expr_order_cmp(a, b) → TclObj(i64)`

`tcl_string.zig`.  Returns `-1`, `0`, or `1`.  Tries a numeric
comparison of `a` and `b` first via `try_parse_int`; falls back to a
bytewise string comparison when either operand is non-numeric.

Emitted by `_emit_expr_order_cmp` for `BinOp.LT`, `GT`, `LE`, `GE` so
that Tcl 9 expressions like `{"a" < "b"}` evaluate to `1` rather than
raising `expected floating-point number but got "a"` (Tcl 8.x
behaviour).  The compiler does not currently thread a target-dialect
flag through to the emitter — the runtime targets Tcl 9.0 only.

## Call-frame interop

Compiled procs keep their locals in WASM locals for speed.  When a
compiled body calls `tcl_eval` for a fallback, the Zig interpreter
needs to read/write the same variables via its frame hash table.  The
bridge is a sync-then-eval-then-readback sequence:

### `tcl_local_set(name_obj, value) → TclObj`

`tcl_frames.zig`.  Writes `value` to the current frame's bucket for
`name_obj`, following `ALIAS_GLOBAL` / `ALIAS_EXT` redirection.
Emitted by `_emit_frame_sync` to mirror WASM locals into the frame.

### `tcl_local_get(name) → TclObj`

Read the frame bucket or return `0` (unset).  Emitted by
`_emit_frame_readback` to pull post-eval values back into WASM locals.

Together they form the eval-fallback bridge:

```
_emit_frame_sync()        # local_set(name, x) for each WASM local
tcl_ns_set(ns, len)       # stamp current namespace
result = tcl_eval(script) # interpreter runs with our frame visible
tcl_ns_restore(saved)     # unwind namespace context
_emit_frame_readback()    # local_get(name) → WASM local
```

## Namespace context

### `ns_set(name_ptr, name_len) → i64 saved`

`tcl_interp.zig`.  Pushes `(name_ptr, name_len)` into the interpreter's
`current_ns_ptr` / `current_ns_len` globals and packs the previous
values into a single i64 save token.  Emitted just before a
compiled-in-namespace proc calls `tcl_eval`.

### `ns_restore(saved)`

Unwinds a saved namespace.  Must be called after every `ns_set`.  The
pair is balanced by the Python emitter (see
`_emit_eval_fallback`) — the interpreter never produces an unbalanced
stack mid-eval.

## Catch-result separation

### `catch_set_ok_result(val)`

`tcl_catch.zig`.  Records the success-path value of the catch body's
last statement.  Called by compiled catch bodies in "keep result" mode
after the last statement.  `catch_result` then returns this value on
success (code 0) or `error_msg` on error (code 1).  The old
one-slot-for-both design returned the error message on success, which
broke `catch body resultVar` when the body succeeded.

## Frame aliasing (`upvar` / `variable`)

The frame bucket encoding is:

| Value                 | Meaning                                        |
| --------------------- | ---------------------------------------------- |
| `>= 0`                | TclObj pointer (0 = unset)                     |
| `-1`                  | `ALIAS_GLOBAL` — same-name global alias        |
| `<= -65536`           | `ALIAS_EXT` — heap descriptor at `-value`      |

### ALIAS_EXT descriptor layout

12 bytes at the recovered heap address:

| Offset | Field         | Notes                                       |
| ------ | ------------- | ------------------------------------------- |
| `0`    | `kind`        | `0 = KIND_GLOBAL_NAMED`, `1 = KIND_FRAME_VAR` |
| `4`    | `param`       | For `KIND_FRAME_VAR`: absolute target depth |
| `8`    | `target_name` | TclObj\* for the target variable name       |

### `frame_alias_named(local_name, target_name)`

Registers `local_name` as a global alias to `target_name` (for
`upvar #0 other local` and `variable`).

### `frame_alias_frame_var(local_name, abs_depth, target_name)`

Registers `local_name` as an alias to `target_name` in the frame at
1-indexed absolute depth `abs_depth` (for `upvar N other local`).

Both register a descriptor at a fresh heap allocation and store the
negated descriptor address in the current frame bucket.

## List element encoding

### `list_elem_quote(buf, off, ptr, len) → new_off` (shared)

`tcl_obj.zig`.  Writes one list element into `buf` starting at `off`.
Chooses between three forms:

1. **Bare** when the element has no special characters, no backslash,
   no braces, and no leading `{`.
2. **Braced** `{…}` when internal braces balance and the content
   doesn't end in an odd number of backslashes (which would escape
   the closing `}` per `TclFindElement`).
3. **Backslash-escaped** — each whitespace, brace, backslash, quote,
   `$`, `[`, or `;` byte is prefixed with `\`.

`tcl_string.zig`'s `list_quote_elem` is a thin alias; both modules use
the canonical implementation.

### `copy_unbraced_elem(dst, src_ptr, src_len) → written`

`tcl_obj.zig`.  Decodes backslash sequences in an unbraced list element
using the shared `consume_bs_escape` helper, handling the full Tcl
backslash table (`\n \t \r \a \b \f \v`, `\xNN`, `\uNNNN`,
`\UNNNNNNNN`, octal `\NNN`, `\<whitespace>` folding).

### `consume_bs_escape(src, si, len, out) → {next_si, written}`

Shared escape decoder used by `subst_flagged` (interpreter word
substitution) and `copy_unbraced_elem` (list element decoding).  Writes
up to 4 UTF-8 bytes to `out` for `\uNNNN` / `\UNNNNNNNN`; 1 byte for
all other escapes.

## `lappend` fast path

`tcl_cmd_lappend` trims trailing whitespace from the existing list
representation, appends a single space, and appends the quoted new
element via `list_elem_quote`.  Existing element encodings are
preserved verbatim — no re-parse, no re-quote — so repeated `lappend`
is O(1) per call instead of O(existing_elems).  A fallback path
(`lappend_reparse`) is used only when the existing list ends in an
unpaired backslash that would eat the separator space.

## Argument expansion (`{*}`)

Tcl 8.5+'s `{*}word` prefix is parsed into a per-word `expand` flag by
`parse_command` in `tcl_interp.zig`.  `eval_script` then splits the
flagged word's value as a Tcl list and inserts each element as a
separate argument, up to `MAX_EXPANDED_WORDS = 128`.  Compiled
callers route `{*}` through `_emit_eval_fallback` with a
`script_override` that reconstructs the original `{*}word` prefix so
the interpreter handles the expansion.

## Variadic `args` parameter

When a proc's last formal parameter is literally named `args`, surplus
call-site arguments are packed into a single Tcl list and bound to
that slot.  The compiler tracks this per-proc in
`_proc_args_tail: set[str]` and emits `_emit_args_list(tail_args)` at
the call site; the runtime (`eval_proc_call` in `tcl_interp.zig`) does
the same packing for interpreter-dispatched calls.

## Clock + timezone resolution

The `clock` ensemble in the WASM runtime supports the four practical
subcommands a typical Tcl script uses: `seconds` / `clicks` /
`milliseconds` / `microseconds` (WASI-clock-backed) plus `format` /
`scan` / `add` (real timezone-aware implementations).  The split is
across three Zig modules:

- [`runtime/zig/io/tcl_tz.zig`](../../../runtime/zig/io/tcl_tz.zig) —
  TZif (RFC 8536) parser, 8-slot LRU cache, and the host-tzdata
  resolver.  Pure-parse + libc fopen — no compiler bridge needed.
- [`runtime/zig/io/tcl_clock.zig`](../../../runtime/zig/io/tcl_clock.zig) —
  strftime renderer (`render_format`), broken-down time helper
  (`break_down`), epoch repacker (`pack_epoch`), and the four
  exports the interpreter dispatcher calls: `clock_format`,
  `clock_format_tz`, `clock_scan_obj`, `clock_add_pair`.
- [`runtime/zig/cmds/stubs.zig`](../../../runtime/zig/cmds/stubs.zig) —
  the interpreter-side `eval_clock` parses `-format` / `-gmt` /
  `-timezone` flags and routes to the right export.

### `clock_format_tz(secs, fmt, zone) → TclObj`

Renders `secs` (Unix epoch) under the timezone identified by the
TclObj `zone`.  Empty `zone` falls through to
`tz.resolve_default()` — `$TZ` first, then `/etc/localtime`, then
synthetic UTC.  `fmt == 0` selects the same default as tclsh
(`%a %b %e %H:%M:%S %Z %Y`).

### `clock_scan_obj(text, zone, gmt) → TclObj(i64)`

Recognises ISO date / RFC 3339 date+time / Zulu / `±HHMM` / `±HH:MM`
suffixes.  Inputs that don't match fall through to the integer
parser so `clock scan 12345` returns `12345` (matches the
counter::init pattern referenced in the KCS issue).  Free-form
`"next thursday"`-style grammar from `library/clock.tcl::GetDate`
is *not* supported and is unlikely to land — the C parser is
~3 KSLOC of yacc grammar.

### `clock_add_pair(base, count, unit) → TclObj(i64)`

Fixed-second units (`seconds` / `minutes` / `hours` / `days` /
`weeks`) are pure i64 multiplies.  Calendar units (`months` /
`years`) re-pack via UTC broken-down time with day-of-month
clamping that matches tclsh — adding 1 month to 2025-01-31 lands
on 2025-02-28 rather than spilling into March.  The interpreter
loop iterates `(count, unit)` pairs so multi-unit inputs
(`clock add $t 1 day 2 hours`) accumulate correctly.

### TZ resolver lookup order

Implemented in `tcl_tz.resolve()`:

1. Synthetic UTC for `UTC` / `GMT` / `:UTC` / `:GMT` / `Etc/UTC` /
   `Etc/GMT` / `Universal` / `Zulu` — no I/O, always succeeds.
2. 8-slot LRU cache keyed on the requested zone name.
3. Host filesystem probe via wasi-libc `open()`:
   `/usr/share/zoneinfo/<zone>`, `/usr/share/lib/zoneinfo/<zone>`,
   `/etc/zoneinfo/<zone>`.  Path validation rejects `..` walks
   and absolute-path overrides as defence in depth — the WASI
   sandbox already blocks anything outside the embedder's
   preopens, but the validator keeps a buggy / malicious script
   from probing arbitrary host files via the resolver.
4. (`resolve_default()` only) `/etc/localtime` for the
   system-default zone.
5. Last-ditch synthetic UTC with a non-zero `last_error` flag so
   callers can surface a warning if they care.

### Bundled trimmed-tzdata fallback (deferred)

The task spec calls for a bundled blob compiled into the wasm
binary so a host with no preopened tzdata still produces real
local-time output.  This is *not* implemented yet — the host
filesystem path covers the common case (every Linux container in
the repo's CI image has `/usr/share/zoneinfo`).  When the bundle
lands it will plug into `tz.resolve()` between the host probe and
the synthetic-UTC fallback.

The intended trim policy:

- **Decade ± 5 years**.  Real-world callers care about timestamps
  near "now"; transitions older than ~5 years and newer than
  ~5 years (rounded to whole-decade boundaries) can be omitted.
  This trims `/usr/share/zoneinfo` from ~3 MB to a few hundred KB.
- **Drop unused zones**.  Aliases like `US/Eastern` (which `link`
  to `America/New_York`) cost only the symlink entry; we keep
  them.  But obscure zones with no living users (e.g. abolished
  jurisdictions like `America/Buenos_Aires` superseded by
  `America/Argentina/Buenos_Aires`) can be dropped from the
  trimmed bundle if size becomes a concern.
- **Strip leap-second tables**.  POSIX time pretends leap seconds
  don't exist and Tcl follows that convention; the leap-second
  records in TZif are dead weight for our renderer.

The trimmer should live in `scripts/trim_tzdata.py` and run at
runtime build time (idempotent, like `fetch_tcl_regex.sh`).
Output goes to `runtime/zig/data/tzdata.bin` and is pulled in
via `@embedFile` from `tcl_tz.zig`.

## Capability-gated commands

Three commands — `exec`, `exit`, and `glob` — escape the WASM
sandbox by reaching the host process or the host filesystem.  All
three live behind a per-runtime capability bitset
(`runtime/zig/interp/tcl_caps.zig`); the default sandboxed posture
refuses each call with a Tcl-catchable
`permission denied: <cmd> requires CAP_<NAME>` until the embedder
flips the matching bit.

### Bit layout

| Constant | Bit | Gates |
|---|---|---|
| `CAP_EXEC` | `1 << 0` | `exec` |
| `CAP_EXIT` | `1 << 1` | `exit` |
| `CAP_FS_GLOB` | `1 << 2` | `glob` |

### Host entry points

#### `tcl_set_capabilities(bits: u32)` → void

`tcl_caps.zig`.  Overwrites the active capability mask.  Called by
the embedder before `tcl_eval` to grant whichever subset of dangerous
primitives the deployment needs.  Passing `0` resets to the default
sandboxed posture mid-run, which is honoured for every subsequent
call.

#### `tcl_get_capabilities() → u32`

Read-only mirror of the active mask.  Exposes the policy for an
embedder that wraps the runtime in additional guards.

### `host_spawn` (new host import)

`exec` cannot use WASI directly — `wasi_snapshot_preview1` exposes
`proc_exit` only, not `proc_spawn`.  The runtime declares a single
new host import:

```
extern "env" fn host_spawn(
    argv_ptr: u32,
    argv_len: u32,
    stdin_ptr: u32,
    stdin_len: u32,
) i32;
```

`argv_ptr` / `argv_len` point to a NUL-separated UTF-8 buffer in
linear memory: every argument is followed by a NUL byte, including
the last, so the host walks `argv_len` bytes splitting on NUL.  The
`stdin_*` pair follows the same convention; `(0, 0)` means "no
stdin".  Return value is a TclObj handle for the captured stdout (a
string TclObj) on success, or `0` on failure with the host expected
to have raised a Tcl error through the catch path before returning.

Embedders **must** satisfy `env.host_spawn` even when they never
intend to grant `CAP_EXEC` — the WASM linker rejects modules with
unsatisfied imports.  The recommended sandboxed-default stub raises
`host_spawn: not configured` and returns `0`; the test harness in
`tests/test_wasm_real_tcl.py` ships with that behaviour and lets
opt-in tests swap in a real implementation.

### `proc_exit` (existing WASI import)

`exit` calls `std.os.wasi.proc_exit` directly — no new host wiring.
wasmtime surfaces this as an `Exit` trap to the embedder
(`wasmtime.ExitTrap` in the Python binding); other embedders see
their environment-specific termination signal.  When `CAP_EXIT` is
not granted, control never reaches `proc_exit` — the capability
gate raises `permission denied` first.

### Failure paths through `catch`

Capability denial routes through `stubs.tcl_stubs.raise` so a
`catch` around the call sees `code == 1` with the permission-denied
message available via `$::errorInfo`.  Bare invocations (no `catch`)
write the same message to stderr with the standard
`tcl trap: site=<id>` prefix and trap.

## Known limitations

- **Dialect gating** — the runtime targets Tcl 9.0.  `tcl_expr_order_cmp`
  produces Tcl 9 semantics; no 8.x fallback path exists.
- **Short-circuit evaluation** — `expr_or` / `expr_and` now thread a
  `skip` flag through the recursive-descent evaluator so `||` / `&&`
  do not run side-effecting `[cmd]` substitutions on the discarded
  branch.
- **Ensembles, coroutines, OO, zipfs** — not supported in the
  interpreter dispatch.  Compiled code that reaches these commands
  produces `unsupported command:` at runtime.
- **Auto-loading (`unknown` proc)** — not implemented; missing commands
  produce `unknown command:` errors immediately.

## Related design docs

- [codegen-internals.md](codegen-internals.md) — bytecode LVT and
  emitter architecture (sibling target).
- [namespace-resolution.md](namespace-resolution.md) — qualified name
  handling used by `qualify_name` in the interpreter.
