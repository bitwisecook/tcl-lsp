# Token-scanning / green-tree dedup migration — COMPLETE

> **Status: complete.** Every migratable raw-`TclLexer` site now routes through
> the canonical green-tree helpers, so each `(region, mode)` is tokenised once
> per `green_tree_scope`. This is the record of what changed; the live design
> lives in `docs/design/compiler/green-token-tree.md`.

## Why

Tcl tokenises context-dependently, so historically every subsystem re-lexed the
bytes it cared about on a fresh `TclLexer`. Measured on a 322-char sample driven
through the full `analyse()` pipeline: **131 redundant raw lexes** (52 distinct
of 183) and **35 redundant segmentations** (15 distinct of 50), on top of the
already-memoised green-tree token lexing (0 redundant). This migration removes
that duplication and collapses the duplicated helper logic into one place.

## Result

- The analyse path has **zero raw-lexer bypasses** and **zero per-key
  segmentation redundancy**.
- Codegen-path sites dedup wherever a `green_tree_scope` is active.
- Every commit is **byte-identical** across the 300-file / 2842-diagnostic
  corpus (`bench/phase0_descend.py --compare`), and `test-slow` (full
  `test-py`/`test-opt`) is green.

## Central helper surface — `compiler/parsing/token_scanning.py`

Lives under `compiler.parsing` so every concern (incl. `dialects`) may import it
without an import-linter carve-out; it must not import the registry at module
scope (cycle avoidance — deferred inside functions, like `descend_command`).

| helper | replaces |
|---|---|
| `word_piece(tok, *, array_codegen_marker=, normalise_var_braces=)` | the 3 divergent `word_piece`/`_word_piece` copies |
| `extract_scalar_var_name(text)` | per-site `$var` name extraction |
| `scan_command_substitutions(text, base_offset=, base_line=, base_col=)` | `TclLexer(...).tokenise_all()` + CMD-filter loops |
| `single_command_substitution(text)` | "is this exactly one `[cmd]`?" checks |
| `parse_single_command(text)` | `token_helpers.parse_command_words` (now re-exported) |
| `extract_single_expr_argument(text, *, expr_aliases=)` | `command_shapes` impl (now re-exported) |
| `iter_region_words(text, base...)` | hand-rolled switch-body / case-list word splitters |

Façades kept for stable import paths: `compiler/token_helpers.py`
(`word_piece`, `parse_command_words`, marker helpers),
`compiler/parsing/command_shapes.py` (`extract_single_expr_argument`),
`compiler/parsing/lexer.py::is_simple_scalar_var_word` (delegates, deferred
import to dodge the `lexer ↔ green_tree ↔ token_scanning` cycle).

## Commits

| commit | scope |
|---|---|
| `cc7f66b` | W1 — per-scope segmentation memo (`GreenTreeScope` + `segment_commands`) |
| `5423964` | W1 fix — **copy cached commands on return** (mutation-aliasing safety) |
| `6def413` | W2a — `word_piece` centralised (3 → 1, golden-tested) |
| `2975f64` | W2b — scanner helpers + routed `parse_command_words` / `extract_single_expr_argument` / `is_simple_scalar_var_word` |
| `544f1e4` | proc_arg_traits / execution_intent / interprocedural |
| `e6b3fbb` | core_analyses / taint / style |
| `329bed4` | compiler codegen-path (gvn, cfg, optimiser ×5, stdlib, rendered, irules, lowering, registry) |
| `5e375df` | analyser switch-pairs / server / tooling / dialects |
| `599eded` | removed the temporary TODO scratchpad |

## Migrated sites, by tier

**compiler/** — `proc_arg_traits.py` (`_extract_var_name`, `_parse_subst`,
`_scan_value_text`); `execution_intent.py` (`_parse_command_substitution`);
`interprocedural.py` (`_scan_script_text`, `_scan_embedded_commands`, return-expr
resolver); `core_analyses.py` (`_parse_cmd_subst`, `_parse_cmd_subst_command`,
constant/segment scanners, `_word_mutation_free`, existence scan,
`_split_command_args`); `taint/_propagation.py` ×4, `taint/_sinks.py`;
`gvn.py` (`_extract_command_argv`, `_find_cmd_tokens_in_text`,
`_collect_cmd_tokens_recursive`); `cfg.py` ×6; `stdlib_prelude.py` ×2;
`rendered_properties.py`; `irules_flow.py` ×3; `lowering.py`
(`_switch_body_elements`, `_expr_arg_from_expr_command` via `command_shapes`);
`registry/runtime.py` (case-list); `optimiser/` (`_helpers` ×3,
`_pattern_recognition`, `_propagation`, `_tail_call`, `_elimination`).
Plus the W1/W2 infra: `parsing/green_tree.py`, `parsing/command_segmenter.py`,
`parsing/command_shapes.py`, `parsing/lexer.py`, `token_helpers.py`.

**analyser/** — `_analyser/_proc.py` (switch pattern/body pairs),
`checks/_helpers.py` (switch pattern/body pairs), `checks/_style.py`
(`_find_nested_expr_subst`).

**server/** — `features/hover.py`, `features/symbol_resolution.py`,
`features/code_actions.py`, `features/minimize.py`, `features/inlay_hints.py`,
`features/_semantic_tokens/_format_args.py`.

**tooling/** — `minifier/minifier.py` ×4, `formatter/engine.py` ×2,
`cli/_utils.py` ×2, `vm/substitution.py`, `vm/compiler.py`.

**dialects/** — `f5/bigip/irules_refs.py`.

## Migration patterns used

1. **Exact helper match** → call the helper (e.g.
   `gvn._find_cmd_tokens_in_text` → `scan_command_substitutions`).
2. **Custom reconstruction / error handling** → swap only the lexing source,
   preserving the loop: `tokens, _ = tokenise(text, base...)` + `for tok in
   tokens:` for `tokenise_all` loops, or an exact `while True` fed by
   `next(iter(tokenise(...)[0]), None)`.
3. **Absolute-offset sites** → pass the owning token's content base through.
4. Drop now-unused `TclLexer` imports (`ruff --fix` for sort/format).

`tokenise(text, 0, 0, 0)` yields a token stream byte-identical to
`TclLexer(text)`, so swap-lexing-source is provably output-preserving for any
downstream consumer — the only risk is changing the loop logic, which we did not.

## Intentionally left on raw `TclLexer`

- `server/features/_semantic_tokens/_collect.py` — hot path using `line_starts`
  (perf hint) + `virtual_insertions`; runs with no active scope (no dedup
  benefit), and routing through `tokenise` would drop the `line_starts`
  optimisation.
- `server/workspace/document_state.py` — already result-cached at the snapshot
  level (`snap._tokens`), once per document version.
- `tooling/fuzzing/harness.py` — deliberately exercises the lexer.
- `compiler/parsing/green_tree.py` / `lexer.py` — the canonical lexer itself.

## Verification

- Per batch: `bench/phase0_descend.py --capture/--compare` (BYTE-IDENTICAL),
  `/tmp/measure_dup3.py` (migrated callers leave the bypass list), `make ci-fast`.
- Final: `SKIP_TEST_EXT=1 make test-slow` (full `test-py`/`test-opt`,
  optimiser/gvn ranges, server/tooling/vm).
- Unit tests: `tests/test_token_scanning_word_piece.py` (golden table pinning the
  three `word_piece` flag combos), `tests/test_token_scanning_scanners.py`.

## Lessons / hazards (keep these in mind for future changes)

- **Mutation aliasing — the one regression.** `segment_commands` results are
  mutated in place by `analyser/_analyser/_recovery.py`
  (`cmd.argv/texts/all_tokens.append`). The seg memo therefore **copies on
  return** (`_copy_commands`); `ci-fast` missed this, the full `test-py`
  (`TestUnmatchedCloseBracket`) caught it. Do NOT add new shared caches of
  `SegmentedCommand` / token lists without the same defensive copy.
- **Import cycles.** `token_scanning` stays registry-free at module scope;
  `is_simple_scalar_var_word` stays in `lexer.py` with a deferred import.
- **`word_piece` is not one behaviour.** The two flags
  (`array_codegen_marker`, `normalise_var_braces`) reproduce codegen-marker /
  segmenter-normalised / verbatim reconstructions exactly — see the golden test.
