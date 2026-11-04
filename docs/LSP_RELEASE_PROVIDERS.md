# LSP provider release issue evidence

This page records the source and test evidence for the provider issues in the
release sweep. The linked issue descriptions are historical reports; the status
below refers to the current `work` branch.

| Issue | Status and evidence |
| --- | --- |
| [#2265](https://github.com/bitwisecook/tcl-lsp/issues/2265) | Fixed before this sweep. `collect_expr` uses the lexer's `expression_term` range to descend into a nested command. `nested_command_head_substitution_in_an_expression_keeps_word_extents_2265` passed and checks the exact `length` and `abcdef` spans in the reported source, with no token overlap. |
| [#2497](https://github.com/bitwisecook/tcl-lsp/issues/2497) | Fixed before this sweep. `lens_counts_superclass_my_dispatch_to_a_subclass_method_2497` passed and asserts one reference for `YAxis::CalculateAxisPosition` from `BaseAxis::preparePosition`; this sweep also checks the Find References result against that count. The shared method-reference result is also used by Rename. The call returns `reached` in Tcl 8.6, 9.0 and 9.1. |
| [#2143](https://github.com/bitwisecook/tcl-lsp/issues/2143) | Open. The W123 regression passes on every `TclVersion`, including a control where removing the factory call makes `port` unresolved. The W214 regression fails: the analyser emits only `Configure`'s `description` W214 at byte span 29–40, with no hint for `::port`. The actual analyser compilation unit has one named procedure and no retained body units or Installed cards, so an Installed-only hint has no body/liveness authority. The original fixture prints `8080` under Tcl 8.4–9.1 and JimTcl; removing the factory call raises `invalid command name "port"` in each. |
| [#2254](https://github.com/bitwisecook/tcl-lsp/issues/2254) | Fixed in this sweep. Removed dead, test-only whitespace recognisers and their `#[allow(dead_code)]` attributes from `hover.rs`. Existing production hover tests cover clock, sprintf, binary, regsub, glob and regex; added quoted format, binary scan and unrelated-quoted-word cases through the production entry point. All added tests passed. |
| [#2365](https://github.com/bitwisecook/tcl-lsp/issues/2365) | Closed after the W153 stub diagnostic fix, policy/suppression controls, generated catalogue and [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2365#issuecomment-6631444932). |
| [#2426](https://github.com/bitwisecook/tcl-lsp/issues/2426) | Open. Core context-action deduplication now uses complete operation identity: kind, edit ranges/replacements, command and structured data, excluding rendered titles. The direct repeated-edit operation regression and source-metadata control pass on sealed source `5d1bee18b`; the unchanged Logical T100 producer fixture still fails with `missing T100: []`. The unused extract-proc rename text scanner was retired, and its analysis-backed positive/refusal controls passed on sealed source `27f401f70` ([receipt](LSP_RELEASE_PROVIDER_EXTRACT_RENAME_RECEIPT_2026-11-04.json)). The broader producer-to-action path remains unverified. |
| [#1956](https://github.com/bitwisecook/tcl-lsp/issues/1956) | Open. The fixed Tcllib 2.0 source creates `::clay::define` in `modules/clay/clay.tcl` through `::clay::dialect::create`; `modules/httpd/httpd.tcl` uses it to declare `::httpd::server` and `method connect`. Loading Tcllib 2.0 under Tcl 9.0 returned Clay 0.8.8 and HTTPD 4.3.6; `info class methods ::httpd::server -all` includes `connect`. The registry has no Clay definition grammar, so the analyser records no class or methods for this form. A fix needs a registry-owned member grammar and a definer-family path that preserves source identity. |
| [#1655](https://github.com/bitwisecook/tcl-lsp/issues/1655) | Open. The current Tcllib corpus test documents two abstention limits in the `::clay::dialect::create` prefix: conditional `return` through the live command table and `eval $ninja` without caller argument binding. Under Tcl 9.0 with exact Tcllib 2.0, `::clay::class` exists, inherits `::clay::dialect::MotherOfAllMetaClasses`, and answers `create` and `new`. Lifting the limits requires source-derived completion and argument-binding proofs; a command-name shortcut would not establish the factory's execution. |
| [#2243](https://github.com/bitwisecook/tcl-lsp/issues/2243) | Open; exact-source coverage blocker. The pinned Tcllib 2.0 IMAP4 input (SHA-256 `5837f0296ab3a8efe67d4a385ea7da722099abcc406937141589d75628154b66`) passed 24 earlier quiet memo/uncached comparisons, but the strengthened nonvacuity audit fails: the checked provider returns zero checks, no `function_lattice` executes, and the actual diagnostic solve has no retained original read at the first reported `$argc` span, 43921–43926. The second read assertion is not reached. Existing source tracing records no procedures and unknown command targets after the first `package require`, including the outer `if` containing the reads. Fixed quiet and active `gets`, tainted `argc` rebind and unset/rebind controls pass. The shared byte checker refuses missing or different inputs; The CI DB differential shard provisions the pinned source. Native [startup](../evidence/tcl/lsp-2243-argc-stdin-startup.md) and [unset/rebind](../evidence/tcl/lsp-2243-argc-unset-rebind-stdin.md) captures establish separate Tcl behaviours. Native [header and loader controls](../experiments/name-handling/lsp-2243-package-provision/README.md) distinguish the already provided C core package from external or forgotten package loaders; Tcl 8.4 and Jim reject the exact multi-requirement header. No product repair is claimed; the [archived coverage audit](../experiments/name-handling/lsp-2243-coverage/README.md) and [exact receipt](LSP_RELEASE_PROVIDER_2243_RECEIPT_2026-11-04.json) keep closure blocked. |

Fresh validation on `0f847a7ca991` passed the exact #2265 token-span test and
the #2497 lens/reference-agreement test. The #2265 nested-closer and #2497
unrelated-class callback controls also passed. The inherited-method tests now
assert the truthful partial-coverage titles `at least 1 reference` and `at least
2 references`, check their counts against Find References, and retain an exact
title control for complete source coverage. All four focused Core tests passed
on sealed source `d7c5bb04ae650d43b9f4a59bf2cf763db8b5ac07`; the alignment
was pushed as `f05d0df38`.

The broader `hover::tests` run had 150 passes and 30 failures in existing
production assertions; the failure log is `/tmp/lsp-providers-hover.log`. The
removed recognisers existed only under `#[cfg(test)]` and were never called by
production hover. The focused #2265 and #2497 regressions and the added #2254
tests passed. On source `537b1a48cf64e87b2f2842a7973e3c2c7271c11e`, the
strengthened #2143 W123 test passed, and its W214 test failed. The exact source
hashes are in `/tmp/lsp-providers-2143-trace-seal.txt`; the analyser trace in
`/tmp/lsp-providers-2143-owner-trace.log` reports `procedures=1`,
`body_units=0`, `installed=0`. The ineffective Installed-only candidate is
archived at `/tmp/lsp-2143-installed-candidate.patch` and was removed from
the source. No W214 fix is claimed. Repository gates remain incomplete.
