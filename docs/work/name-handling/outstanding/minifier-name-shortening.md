# Open item: minifier name shortening lost until a shared rename planner

Opened 17 November 2026 by the LSP features area, on the owner's ruling of the same day.

## What was given up

The minifier's lexical naming passes stay off (`LEXICAL_NAMING_PASSES = false` in `rust/tcl-lsp-core/src/minify/original.rs`): they allocate names without the shared transactional rename planner, so occupied command slots are not reserved and per-occurrence qualification is lost (review N15). With them off, the compact and aggressive tiers do no local renaming, no proc renaming under `isolated` and no alias insertion. The tests that expected those rewrites were retired in `292a9428`: expectations now match the output with the passes refused, and the ten tests that held nothing else were deleted.

## Measured loss

| Corpus | Bytes in | Out, passes refused | Out, passes on | Loss |
| --- | --- | --- | --- | --- |
| Retired test fixtures (12 inputs, old expected vs new expected) | — | 802 | 594 | 35.0% larger |
| 30 tcllib 2.0 module files under 3 KB, lenient `tcl`, compact tier | 21,524 | 13,021 | 13,021 | none |
| Same files, aggressive tier | 21,524 | 13,021 | 12,840 | 1.4% larger (181 bytes) |

The tcllib measurement flipped the constant locally in a throwaway test (nothing committed). Larger tcllib files were not measured: the minifier did not finish 40 files under 40 KB within 20 minutes, which is its own performance problem. On tcllib the compact tier renamed nothing even with the passes on, so the fixtures overstate the loss on real code; aggressive alias insertion is what real code loses.

## What fixing it needs

A shared transactional rename planner that reserves occupied command and variable slots and keeps each occurrence's qualification, used by the minifier instead of its own allocation. Then the passes come back on, and the retired expectations come back as planner-backed tests. The keyword-abbreviation tests (`abbreviations::*`, `abbreviates_ensemble_subcommand_in_irules`, `keyword_abbreviation_follows_an_aliased_head`) are a separate contract (keyword selection) and were not retired.
