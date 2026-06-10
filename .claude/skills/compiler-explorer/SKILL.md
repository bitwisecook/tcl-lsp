---
name: compiler-explorer
description: >
  Debug the Tcl compiler pipeline (lexer → green tree → segmenter → IR → CFG →
  SSA → codegen) running from source — never a built pyz/zipapp. Wraps the
  tcl-explorer CLI and adds source-slice and raw-token views that make
  off-by-one range bugs obvious. Use when investigating a wrong diagnostic
  position, a stray-brace / unmatched-delimiter false positive, a mis-tokenised
  word, an IR statement whose range looks wrong, or any time you need to see
  what the compiler produces for a snippet of Tcl.
allowed-tools: Bash, Read
---

# Compiler Explorer Skill

Inspect every stage of the Tcl compiler for a snippet of source, **running from
the working tree** (`python -m tooling.explorer`) so what you see is the live
code, not a stale `.pyz`. Built on the `tcl-explorer` CLI (24 views) plus two
local views — `slices` and `tokens` — that the raw CLI does not provide and that
catch range / offset bugs at a glance.

## Usage

Run from the repo root:

```bash
python .claude/skills/compiler-explorer/explore.py <view> [--source S | FILE | -] [--dialect D] [--opt off|on|diff]
```

Input is taken from `--source "..."`, a file path, or stdin (`-`), exactly like
the explorer CLI.

## Views

### Local views (added by this skill)

| View | What it shows | Why it helps |
|---|---|---|
| `slices` | Every IR statement's range as offsets **and `[line:col]`**, plus the literal `repr(source[start:end])` the range covers | A one-byte range overshoot reads as an extra delimiter in the slice (`'return {}}'` vs `'return {}'`). This is how issue #527 was found. |
| `tokens` | The raw lexer token stream: `TokenType`, text, absolute offsets, and the source slice each token covers | A mis-placed `Token.end.offset` (e.g. the empty-`{}` convention) shows up directly. |
| `lowlevel` | `tokens` + `greentree` + `slices` back to back | One-shot low-level overview for a quick triage. |

### Forwarded `tcl-explorer` views

Any other `<view>` is forwarded to `python -m tooling.explorer --show <view>`
with `--text --no-colour`. The full catalogue (see `tooling/explorer/pipeline.py`
`ALL_VIEWS`):

`greentree`, `ir`, `cfg`, `ssa`, `loops`, `intervals`, `bounds`, `interproc`,
`types`, `rendered`, `dataflow`, `opt`, `gvn`, `shimmer`, `taint`, `irules`,
`event-order`, `callouts`, `asm`, `wasm`, `asm-opt`, `wasm-opt`, plus the groups
`all`, `compiler`, `optimiser`.

The `--opt off|on|diff` lens applies to the `ir`/`cfg`/`ssa`/`asm`/`wasm` views
(original path, optimised path, or a diff of the two).

## Examples

```bash
# Issue #527 reproducer — the slice exposes the overshoot instantly
python .claude/skills/compiler-explorer/explore.py slices --source 'if {1} {return {}}'
#   IRReturn   off 8-16  [1:9-1:17]  slice='return {}'      <- correct
#   (a bug would read slice='return {}}' with a trailing brace)

# Raw tokens with offsets + slices
python .claude/skills/compiler-explorer/explore.py tokens --source 'set x {}'

# Green (red-green) tree for a brace-matching question
python .claude/skills/compiler-explorer/explore.py greentree --source 'if {1} {return {}}'

# Lowered IR / control-flow graph / bytecode
python .claude/skills/compiler-explorer/explore.py ir  --source 'set x 1; puts $x'
python .claude/skills/compiler-explorer/explore.py cfg --source 'if {$x > 0} {set a 1}'
python .claude/skills/compiler-explorer/explore.py asm --source 'return 42'

# Optimiser diff
python .claude/skills/compiler-explorer/explore.py ir --opt diff --source 'expr {$x + $x + $x}'

# A specific dialect (F5 iRules)
python .claude/skills/compiler-explorer/explore.py ir --dialect f5-irules --source 'when CLIENT_ACCEPTED {set x 1}'

# File or stdin input
python .claude/skills/compiler-explorer/explore.py lowlevel samples/foo.tcl
echo 'set x 1' | python .claude/skills/compiler-explorer/explore.py ir -
```

## Debugging a range / position bug (the #527 workflow)

When a diagnostic fires on the wrong characters, or a stray-brace / unmatched
delimiter is reported on valid code:

1. **`slices`** — find the IR statement whose `slice=` reads wider or narrower
   than the actual command. An extra trailing `}` / `]` means the statement
   range overshoots its last argument.
2. **`tokens`** — confirm where the over/undershoot starts. Empty braced/bracket
   words `{}` / `[]` are the usual suspect: the lexer reports `end.offset` *on*
   the closer, so any `end.offset + 1` arithmetic overshoots by one (see
   `shared/ranges.range_from_word_token`).
3. **`greentree`** — confirm the green tree itself is correct; a correct tree
   with a wrong IR range points at the segmenter / range helpers, not the lexer.

## Running the explorer directly

The skill is a convenience wrapper; the underlying tool is always available
from source for anything the wrapper does not cover (TUI, JSON, multi-view):

```bash
python -m tooling.explorer --source 'set x 1' --show ir,types,opt --text --no-colour
python -m tooling.explorer --source 'set x 1' --show all --json     # machine-readable
python -m tooling.explorer samples/foo.tcl --tui                    # interactive
```

## When to use

- A diagnostic highlights the wrong span, or fires on valid code.
- Investigating lexer / green-tree / segmenter / IR-range correctness.
- Checking what IR, CFG, SSA, or bytecode the compiler emits for a snippet.
- Comparing original vs optimised IR/bytecode (`--opt diff`).

$ARGUMENTS
