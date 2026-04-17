# WASM runtime — compiler-to-interpreter bridge primitives

The Zig runtime (`runtime/zig/tcl_runtime.wasm`) is the execution
partner of the WASM code we emit from `core/compiler/codegen/wasm.py`.
Compiled code calls Zig-exported helpers for the primitives it cannot
statically compile (list parsing, string operations, the full Tcl
interpreter for untypable constructs).  This doc lists the boundary
primitives added to support Tcl 9 compatibility, grouped by the
contract each one encodes.

Source: [`runtime/zig/`](../../../runtime/zig/),
[`core/compiler/codegen/wasm.py`](../../../core/compiler/codegen/wasm.py)

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
