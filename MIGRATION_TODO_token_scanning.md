# TEMP: token-scanning / green-tree dedup migration — remaining work

> **Temporary working note.** Delete when the repo-wide sweep is complete.
> Full design + rationale: `/root/.claude/plans/plan-out-a-full-delightful-badger.md`.

## Goal

Eliminate redundant re-lexing / re-segmentation by routing every raw
`TclLexer(...)` site through the canonical green-tree helpers in
`compiler/parsing/token_scanning.py`, so each `(region, mode)` is tokenised
once per `green_tree_scope`.

## Done (on branch `claude/happy-darwin-tjFfB`, on top of `da8fe31`)

| commit | what |
|---|---|
| `cc7f66b` | W1 — per-scope **segmentation memo** in `GreenTreeScope` + `segment_commands` |
| `5423964` | W1 fix — **copy cached commands on return** (mutation-aliasing safety; see Lessons) |
| `6def413` | W2a — **`word_piece` centralised** (3 divergent copies → one two-flag canonical, golden-tested) |
| `2975f64` | W2b — green-tree-backed **scanners** + routed `parse_command_words` / `extract_single_expr_argument` / `is_simple_scalar_var_word` |
| `544f1e4` | W3.1a — `proc_arg_traits` / `execution_intent` / `interprocedural` |
| `e6b3fbb` | W3.1b — `core_analyses` / `taint/_propagation` / `taint/_sinks` / `analyser/checks/_style` |

**Result so far:** the `analyse()` path has **0 raw-lexer bypasses** and **0
per-key segmentation redundancy** (was 131 redundant lexes + 35 redundant
segments on the sample). All commits byte-identical across 300 files
(`bench/phase0_descend.py`) + `ci-fast` green. `test-slow` passes once the
aliasing fix (`5423964`) is included — re-run before pushing.

## Helpers available (in `compiler/parsing/token_scanning.py`)

`word_piece(tok, *, array_codegen_marker=, normalise_var_braces=)`,
`extract_scalar_var_name`, `scan_command_substitutions(text, base_offset=, …)`,
`single_command_substitution`, `parse_single_command`,
`extract_single_expr_argument`, `iter_region_words`.

## Remaining sites (≈ all the same mechanical swap)

### compiler/ codegen-path (run under `optimise=True`; verify with `test-slow`)
- `rendered_properties.py:217`
- `gvn.py:464` (custom argv), `:504` (→ `scan_command_substitutions`), `:1029` (STR-recursion, absolute)
- `irules_flow.py:201` (absolute), `:250`, `:1115` (`tokenise_all`)
- `optimiser/_helpers.py:223,484` (absolute), `:543` (uses `_word_piece`)
- `optimiser/_pattern_recognition.py:933` (absolute), `_propagation.py:807`, `_tail_call.py:269` (absolute, uses `parse_command_words`), `_elimination.py:69` (`tokenise_all` + try/except)
- `lowering.py:247,254` (`_switch_body_elements`; uses marker `word_piece` → swap source, don't use `iter_region_words`)
- `registry/runtime.py:1073` (absolute, deferred import)
- `cfg.py:87,362,399,403,664,672` (mix of `tokenise_all` + custom loops, some nested)
- `stdlib_prelude.py:118,152` (`tokenise_all`, deferred import)

### analyser/ (consistency; in-scope)
- `_analyser/_proc.py` switch-pair extractor, `checks/_helpers.py` switch-pair extractor

### server/ (consistency; **no active scope** → wrap hot handlers in `green_tree_scope()` to actually dedup; `document_state.py:827` is already cached — LEAVE)
- `features/{hover.py:482, inlay_hints.py:228, code_actions.py:1262, symbol_resolution.py:58, minimize.py:130}`, `_semantic_tokens/_collect.py:554,563` (keep `virtual_insertions` + thread `line_starts`), `_semantic_tokens/_format_args.py:41`

### tooling/ (consistency; explorer/serialise are in-scope) — `formatter/engine.py:164,456`, `minifier/minifier.py:804,1117,1263,1604`, `cli/_utils.py:664,716`, `vm/{substitution.py:129,compiler.py:109}`; **LEAVE** `fuzzing/harness.py` (tests the lexer)

### dialects/ — `f5/bigip/irules_refs.py:297` (absolute; import from `compiler.parsing.token_scanning`)

## Migration patterns

1. **Exact helper match** → call the helper (`extract_scalar_var_name`,
   `scan_command_substitutions`, `single_command_substitution`, …).
2. **Custom reconstruction / error handling** → swap only the lexing source:
   `tokens, _ = tokenise(text, base_offset, base_line, base_col)` + `for tok in
   tokens:`; or keep an exact `while True` with `tok = next(_it, None)` where
   `_it = iter(tokenise(...)[0])`.
3. **Absolute-offset sites** → pass the owning token's content base through.
4. Drop now-unused `TclLexer` imports (ruff `--fix` for sort).

## Verification per batch

- `bench/phase0_descend.py --capture A; … --compare tmp/p0_golden.json A` → **BYTE-IDENTICAL**.
- `/tmp/measure_dup3.py` → confirm the migrated callers leave the bypass list.
- `make ci-fast` (lint-imports + ty + fast tests).
- **Before pushing:** `SKIP_TEST_EXT=1 make test-slow` — `ci-fast` does NOT run
  the full `test-py` / `test-opt`; those catch optimiser/recovery regressions
  the diagnostic corpus misses.

## Lessons / hazards

- **Mutation aliasing (caused the one regression so far).** `segment_commands`
  results are mutated in place by `analyser/_analyser/_recovery.py`
  (`cmd.argv/texts/all_tokens.append`). The seg memo therefore **copies on
  return** (`_copy_commands`). When migrating, do NOT introduce new shared
  caches of `SegmentedCommand` / token lists without the same defensive copy.
- `tokenise(text,0,0,0)` is byte-identical to `TclLexer(text)` (token stream),
  so swap-lexing-source is provably output-preserving for ANY consumer —
  the only risk is changing the loop logic, so preserve it exactly.
- Keep `is_simple_scalar_var_word` physically in `lexer.py` (deferred import)
  and `token_scanning` registry-free at module scope (import-cycle avoidance).
