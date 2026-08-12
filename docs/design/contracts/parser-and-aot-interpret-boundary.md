# Contract: canonical parser & the AOT/interpret boundary

> The semantic boundary the AOT compiler and the runtime agree on: what one
> grammar owns, and where "I can compile this" ends and "I must interpret
> this" begins. As-built notes: [parsing.md](parsing.md),
> [lexing.md](lexing.md), and the WASM codegen pipeline under
> [../compiler/](../compiler/).

## The two non-negotiable ideas

1. **Parse once, canonically.** Word splitting, quoting, substitution,
   `{*}` expansion, comments, and command separation are *one* grammar. It is
   used by the compiler's front end, the runtime evaluator, `subst`, the list
   parser, and `expr`'s argument handling. Re-implementing it in N places
   guarantees N drifts — every place that re-derives "where does this word
   end" is a future off-by-one or a mismatched error. One scanner; many
   clients.

2. **Tcl is homoiconic and late-bound, so the AOT compiler is half of a
   pair.** `eval`, `uplevel`, `subst`, `source`, `apply`, `proc f $a $body`,
   `$dynamic_cmd …`, and `{*}$computed` all produce code that does not exist
   at compile time. A *complete runtime parser + tree-walking evaluator* must
   exist as a peer to the compiled path, and the two must be observably
   identical: same result, same return code/options, same `errorInfo`
   frames, same line numbers, same trace firings. The boundary between "I can
   compile this" and "I must interpret this" is a designed contract, not an
   accident of which cases got optimised.

## The grammar (the parts everyone gets subtly wrong)

A *script* is a sequence of *commands* separated by **newline** or `;`. A
command is a sequence of *words* separated by unescaped whitespace. Within a
word, four things happen — and *which* happen depends on how the word is
introduced:

| Word form | `$var` subst | `[cmd]` subst | `\` subst | Notes |
|---|---|---|---|---|
| **bare** `abc` | yes | yes | yes | whitespace ends it unless escaped/grouped |
| **double-quoted** `"…"` | yes | yes | yes | whitespace is literal; ends at the matching `"` |
| **braced** `{…}` | **no** | **no** | **only** `\<newline>`→space | fully literal; nesting counts balanced `{}` |

Hard edges that must be in the spec, not discovered later:

* **Comments** (`#`) are comments **only where a command is expected** —
  i.e. at the start of a command. `set x #y` has a literal `#y`. After a
  word, `#` is ordinary.
* **`;` and newline** both terminate a command; inside `{}`/`"…"`/`[…]`
  they do not. A `;` inside a bare word is *not* special only if escaped.
* **Backslash-newline** (line continuation) collapses to a single space —
  including inside braces (the one substitution braces allow). This changes
  the *length* of a braced body (`{a\<NL>b}` has length 3, "a b").
* **`{*}`** triggers argument expansion **only** when the three chars are a
  complete leading prefix of a word *immediately* followed by a non-blank,
  non-terminator character. A standalone `{*}` (end of command, or followed
  by whitespace/`;`/newline) is the literal braced word `*`. (Getting this
  wrong silently drops or duplicates arguments.)
* **Nesting** is independent per bracket type: `[` … `]` command
  substitution, `{` … `}` braces, `"` … `"` quotes; a `]` inside `{}` is
  literal, a `}` inside `[…]` belongs to the inner command, etc.
* **`$` variable forms:** `$name`, `${name}` (any chars), `$arr(idx)` (the
  index is itself substituted), `$name` stops at the first non-name char.
* The result of parsing is **tokens with source spans**, not strings. Spans
  are needed for `info frame`, error carets, and the LSP — thread them from
  byte zero.

## Where the boundary falls

| Construct | Default disposition |
|---|---|
| Static command name + static body (`proc`, `if {…} {…}`, `while`, `foreach`, `set x 1`, braced `expr`) | **Compile** to native code. |
| `eval $x`, `uplevel`, `subst`, `apply {…}`, `proc f $a $body`, `$cmd …`, `namespace eval` of a computed body, `source` | **Interpret** at runtime via the shared parser+evaluator. |
| `{*}$computed` | Compile the *call site*, but the expanded words are materialised at runtime (must grow unbounded — never a fixed-size argument array). |
| `switch`/`expr`/`dict for`/`try` bodies | Sub-grammars; compile when the body is a literal brace word, else interpret. |

The interpreter is therefore not a fallback "slow path" bolted on late — it
is a co-equal back end. Budget for it from the start, and make every dynamic
construct route through *one* `eval_script(tokens, frame)` entry so behaviour
cannot fork.

## The identity contract (compiled ≡ interpreted)

For any script that *can* be run both ways, the following must be
byte-identical:

* the result string and the return code + options dict
  (`-errorcode`, `-errorinfo`, `-errorstack`, `-level`);
* the `errorInfo` traceback text, including the "invoked from within …"
  vs. "while executing …" distinction. In the as-built runtime that is the
  `Interp::error_info` accumulator plus the `error_logged` flag (C's
  `ERR_ALREADY_LOGGED`): the *first* frame logged selects "while executing",
  `error_logged` stops the same bytecode frame being re-logged, and it is
  cleared at a real frame boundary — a nested `eval` / `subst`, a proc or
  control body — so the enclosing command contributes its own "invoked from
  within" frame. `error_line` (C's `iPtr->errorLine`) carries the innermost
  logged command's 1-based source line, which is what the `(procedure … line
  N)` and `("while" body line N)` frames report;
* `info script` and `info level`/`info frame` *command* content;
* the order and arguments of variable/command/execution trace callbacks.

Line numbers and PC tables from `info frame` are **incompatible-by-design**
against a from-scratch codegen and are classified, not chased.

## `source` / `package` / auto-load are this boundary + a VFS

`source FILE` is "read bytes, parse, evaluate in the caller's namespace,
with `info script` set to FILE and relative paths resolved against it."
`package require` → `package ifneeded`/`pkgIndex.tcl` → `source`, and
`unknown`/`auto_load`/`tclIndex` are a lazy command-discovery layer on top.
All of it is "interpret code discovered at runtime, from a filesystem."

**Design rule:** the filesystem and module discovery sit behind one VFS +
loader interface, so a constrained host (WASI: no writable fs, no real stat)
is a *missing implementation of a defined interface* rather than a missing
design. Capabilities a backend genuinely lacks are declared as such — see
[../runtime/backend-constraints.md](../runtime/backend-constraints.md).

## Contract vs. incompatible-by-design

| Behaviour | Class |
|---|---|
| Word/quote/brace/`{*}`/comment/`;`-newline grammar, line-continuation length | **Contract** |
| Substitution rules per word form, `$arr(idx)` index substitution | **Contract** |
| Result + return code + options dict as the universal command return | **Contract** |
| `errorInfo` text incl. "invoked from within"/"while executing" | **Contract** |
| Parse errors (`missing close-brace`, `unbalanced open bracket`, …) verbatim | **Contract** |
| `info frame` line/PC tables, bytecode disassembly | **Incompatible-by-design** (W9-internal) |
| `info script` value, `info level` command lists | **Contract** |

## See also

- [runtime-variable-frame-model.md](runtime-variable-frame-model.md) — the
  frame the interpreter injects for `uplevel`/`eval`.
- [numeric-tower-and-expr-semantics.md](numeric-tower-and-expr-semantics.md)
  — `expr` is a second grammar parsed by the same "parse once" discipline.
- [parsing.md](parsing.md), [lexing.md](lexing.md) — as-built segmentation.
