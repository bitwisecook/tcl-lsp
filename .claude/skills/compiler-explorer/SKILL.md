---
name: compiler-explorer
description: >
  Debug the Tcl compiler pipeline (parser → CST → segmenter → IR → CFG → SSA →
  codegen) running from source — never a stale build. Wraps the `tcl explore`
  CLI and adds source-slice and CST-leaf views that make off-by-one range bugs
  obvious. Use when investigating a wrong diagnostic position, a stray-brace /
  unmatched-delimiter false positive, a mis-parsed word, an IR statement whose
  range looks wrong, or any time you need to see what the compiler produces for
  a snippet of Tcl.
allowed-tools: Bash, Read
---

# Compiler Explorer Skill

Inspect every stage of the Tcl compiler for a snippet of source, **running from
the working tree** so what you see is the live code. Built on `tcl explore` (the
Rust compiler-explorer surface in `rust/tcl-cli`) plus three local views —
`slices`, `tokens`, and `cst` — that catch range / offset bugs at a glance.

By default the wrapper shells out to `cargo run -q -p tcl-cli --bin tcl`, so the
explorer always reflects the current source. Export `TCL_EXPLORE_BIN=/path/to/tcl`
to use a prebuilt binary and skip the rebuild.

## Usage

Run from the repo root:

```bash
python .claude/skills/compiler-explorer/explore.py <view> [--source S | FILE | -] [--dialect D]
```

Input is taken from `--source "..."`, a file path, or stdin (`-`), exactly like
the explorer CLI.

## Offsets are half-open

The explorer's JSON contract reports `[startOffset, endOffset)`. A statement
covering `set x 1` in a 7-byte document reports `off 0-7`. Slices are therefore
`source[start:end]`.

## Views

### Local views (rendered by this skill)

| View | What it shows | Why it helps |
|---|---|---|
| `slices` | Every IR statement's range as offsets **and `[line:col]`**, plus the literal `repr(source[start:end])` the range covers | A one-byte range overshoot reads as an extra delimiter in the slice (`'return {}}'` vs `'return {}'`). This is how issue #527 was found. |
| `tokens` | The CST's **leaf** nodes: kind, absolute offsets, and the source slice each covers | A mis-placed `endOffset` shows up directly. |
| `cst` | The parse tree as an indented tree with offsets and tags | Brace-matching and delimiter questions. |
| `lowlevel` | `tokens` + `cst` + `slices` back to back | One-shot low-level overview for a quick triage. |

The Rust parser builds a single red-green CST directly, so there is **no separate
lexer token stream** and no standalone green tree. `tokens` reports CST leaves —
the terminal spans the compiler actually consumes. `greentree` is accepted as a
deprecated alias for `cst`.

`cst` is rendered locally rather than forwarded because `tcl explore --text` has
no renderer for it; it exists only in the `--json` contract.

### Forwarded `tcl explore` views

Any other `<view>` is forwarded to `tcl explore --show <view> --text --no-colour`.
The full catalogue (`VIEW_META` in `rust/tcl-explorer/src/views.rs`):

`cst`, `segments`, `structuralIndex`, `sourceMap`, `ir`, `cfg`, `ssa`, `loops`,
`types`, `intervals`, `bounds`, `dataflow`, `interproc`, `rendered`, `opt`,
`optimiserPasses`, `gvn`, `shimmer`, `taint`, `irules`, `eventOrder`, `callouts`,
`asm`, `asmOpt`, `wasm`, `wasmOpt`.

`interproc` additionally reports each procedure's **param constants** — the
caller-uniform-literal SCCP seed it was analysed under. That line is the first
thing to check when a condition on a parameter folded and you think it should
not have: its presence names the literal every visible caller passed, and its
*absence* means an indirect call site (a `$cmd` dispatch, a callback prefix, an
`eval $script`) withdrew the seed. See
`docs/design/compiler/interprocedural-call-site-seeding.md`.

Four of those have **no text renderer** and are reachable only via `--json`:
`cst` (rendered locally instead), `segments`, `asmOpt`, `wasmOpt`. Asking for one
prints `compiler explorer: no matching views`.

> [!WARNING]
> `--show` matches view names by **substring**, not exact name. `--show all` does
> not mean "every view" — it renders only `callouts`, the one name containing the
> substring `all`. Likewise `--show ss` matches both `ssa` and `optimiserPasses`.
> Pass exact names.

There is no `--opt` lens. The optimised paths are their own views (`opt`,
`optimiserPasses`, `asmOpt`, `wasmOpt`).

## Examples

```bash
# Issue #527 reproducer — the slice exposes the overshoot instantly
python .claude/skills/compiler-explorer/explore.py slices --source 'if {1} {return {}}'
#   IRIf       off 0-18  [1:1-1:19]  slice='if {1} {return {}}'
#   IRReturn   off 8-17  [1:9-1:18]  slice='return {}'      <- correct
#   (a bug would read slice='return {}}' with a trailing brace)

# CST leaf spans with offsets + slices
python .claude/skills/compiler-explorer/explore.py tokens --source 'set x {}'

# Parse tree for a brace-matching question
python .claude/skills/compiler-explorer/explore.py cst --source 'if {1} {return {}}'

# Lowered IR / control-flow graph / bytecode
python .claude/skills/compiler-explorer/explore.py ir  --source 'set x 1; puts $x'
python .claude/skills/compiler-explorer/explore.py cfg --source 'if {$x > 0} {set a 1}'
python .claude/skills/compiler-explorer/explore.py asm --source 'return 42'

# Optimiser views
python .claude/skills/compiler-explorer/explore.py opt --source 'expr {$x + $x + $x}'

# A specific dialect (F5 iRules)
python .claude/skills/compiler-explorer/explore.py ir --dialect f5-irules --source 'when CLIENT_ACCEPTED {set x 1}'

# Skip the cargo rebuild when the tree is already built
TCL_EXPLORE_BIN=target/release/tcl python .claude/skills/compiler-explorer/explore.py slices --source 'set x 1'
```
