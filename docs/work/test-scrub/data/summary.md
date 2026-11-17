# Test inventory

Written by `tools/scrub_inventory.py`; do not edit by hand.

- Tests read: 37,567; marked failing from the lists: 3,743 (list entries with no matching test: 20).
- Bodies that are the same text as another test's: 179 tests in 88 groups.
- Tests running the same Tcl sources through the same helpers against the same codes as another test: 844 in 385 groups (157 groups span files).
- Tcl sources used from more than one file: 4,313.

Classes: **A** runs Tcl source and cites tclsh, Jim or a release; **B** runs Tcl source and cites an issue, a contract or a KCS page; **C** runs Tcl source and cites nothing; **D** runs no Tcl source (it holds an internal structure or API).

## By class

| Class | Tests | Failing | Failing share of class |
| --- | ---: | ---: | ---: |
| A-oracle | 4,045 | 564 | 14% |
| B-cited | 2,962 | 497 | 17% |
| C-uncited | 18,182 | 2,333 | 13% |
| D-internal | 12,378 | 349 | 3% |

## By crate

| Crate | Tests | Failing | A | B | C | D | Failing A | Failing B | Failing C | Failing D |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `tcl-compiler` | 13,171 | 2,213 | 1,640 | 1,316 | 7,651 | 2,564 | 336 | 335 | 1,405 | 137 |
| `tcl-lsp-core` | 4,911 | 530 | 650 | 578 | 2,962 | 721 | 66 | 97 | 344 | 23 |
| `tcl-vm` | 3,184 | 101 | 767 | 170 | 1,007 | 1,240 | 19 | 2 | 37 | 43 |
| `tcl-runtime` | 2,715 | 58 | 258 | 291 | 955 | 1,211 | 10 | 7 | 20 | 21 |
| `tcl-lsp-server` | 2,682 | 409 | 218 | 134 | 1,636 | 694 | 69 | 25 | 247 | 68 |
| `tcl-registry` | 2,453 | 1 | 141 | 119 | 712 | 1,481 | 0 | 0 | 0 | 1 |
| `tcl-codegen` | 1,538 | 186 | 174 | 92 | 898 | 374 | 33 | 9 | 134 | 10 |
| `tcl-syntax` | 925 | 0 | 20 | 32 | 210 | 663 | 0 | 0 | 0 | 0 |
| `tcl-lexer` | 815 | 0 | 60 | 26 | 304 | 425 | 0 | 0 | 0 | 0 |
| `tcl-spectcl` | 622 | 21 | 15 | 3 | 224 | 380 | 2 | 1 | 13 | 5 |
| `tcl-spec-studio` | 399 | 3 | 3 | 14 | 152 | 230 | 0 | 0 | 0 | 3 |
| `xtask` | 388 | 0 | 0 | 0 | 133 | 255 | 0 | 0 | 0 | 0 |
| `tcl-bigip` | 362 | 0 | 0 | 1 | 89 | 272 | 0 | 0 | 0 | 0 |
| `tcl-cli` | 287 | 41 | 31 | 38 | 139 | 79 | 14 | 5 | 16 | 6 |
| `tcl-cmd-core` | 258 | 0 | 13 | 9 | 24 | 212 | 0 | 0 | 0 | 0 |
| `tcl-lsp-db` | 254 | 76 | 19 | 54 | 142 | 39 | 11 | 11 | 44 | 10 |
| `tcl-dialect` | 216 | 0 | 2 | 0 | 9 | 205 | 0 | 0 | 0 | 0 |
| `f5-cli` | 185 | 2 | 0 | 2 | 31 | 152 | 0 | 0 | 0 | 2 |
| `tcl-bigip-query` | 179 | 0 | 0 | 0 | 26 | 153 | 0 | 0 | 0 | 0 |
| `tcl-mcp` | 146 | 3 | 6 | 18 | 86 | 36 | 0 | 2 | 1 | 0 |
| `tcl-explorer` | 142 | 13 | 4 | 10 | 103 | 25 | 1 | 1 | 10 | 1 |
| `bigip-report-gen-rust` | 126 | 5 | 0 | 0 | 44 | 82 | 0 | 0 | 2 | 3 |
| `tcl-cshim` | 124 | 15 | 3 | 2 | 44 | 75 | 0 | 0 | 7 | 8 |
| `bpf-tcl` | 113 | 0 | 0 | 0 | 89 | 24 | 0 | 0 | 0 | 0 |
| `tcl-fuzz` | 113 | 0 | 0 | 1 | 30 | 82 | 0 | 0 | 0 | 0 |
| `tcl-runtime-api` | 105 | 0 | 0 | 0 | 12 | 93 | 0 | 0 | 0 | 0 |
| `tcl-irules` | 104 | 3 | 3 | 11 | 71 | 19 | 1 | 0 | 1 | 1 |
| `tcl-sslictcl` | 89 | 0 | 0 | 1 | 14 | 74 | 0 | 0 | 0 | 0 |
| `tcl-engine-tclvm` | 85 | 8 | 3 | 6 | 44 | 32 | 1 | 1 | 2 | 4 |
| `bpf-tcl-ir` | 82 | 0 | 1 | 0 | 39 | 42 | 0 | 0 | 0 | 0 |
| `tcl-pkg` | 70 | 0 | 0 | 0 | 25 | 45 | 0 | 0 | 0 | 0 |
| `tcl-irule-test` | 61 | 27 | 2 | 3 | 36 | 20 | 0 | 0 | 27 | 0 |
| `f5-xc` | 57 | 11 | 0 | 1 | 37 | 19 | 0 | 0 | 10 | 1 |
| `tcl-core-types` | 53 | 0 | 0 | 0 | 3 | 50 | 0 | 0 | 0 | 0 |
| `tcl-pkg-model` | 53 | 0 | 0 | 0 | 28 | 25 | 0 | 0 | 0 | 0 |
| `tcl-bytecode` | 52 | 0 | 1 | 0 | 2 | 49 | 0 | 0 | 0 | 0 |
| `tcl-vm-cli` | 49 | 1 | 6 | 3 | 29 | 11 | 1 | 0 | 0 | 0 |
| `tcl-spec-hooks` | 39 | 2 | 1 | 1 | 15 | 22 | 0 | 0 | 1 | 1 |
| `tcl-cli-support` | 38 | 0 | 0 | 5 | 7 | 26 | 0 | 0 | 0 | 0 |
| `tcl-diagram` | 36 | 14 | 1 | 9 | 22 | 4 | 0 | 1 | 12 | 1 |
| `tcl-spec-studio-wasm` | 34 | 0 | 0 | 0 | 12 | 22 | 0 | 0 | 0 | 0 |
| `tcl-engine-wasm` | 33 | 0 | 1 | 11 | 13 | 8 | 0 | 0 | 0 | 0 |
| `tcl-regex` | 33 | 0 | 1 | 0 | 3 | 29 | 0 | 0 | 0 | 0 |
| `bpf-tcl-codegen` | 31 | 0 | 0 | 0 | 29 | 2 | 0 | 0 | 0 | 0 |
| `tcl-bigip-io` | 28 | 0 | 0 | 0 | 2 | 26 | 0 | 0 | 0 | 0 |
| `tcl-lsp-server-wasi` | 27 | 0 | 0 | 0 | 0 | 27 | 0 | 0 | 0 | 0 |
| `tcl-debugger` | 19 | 0 | 0 | 0 | 13 | 6 | 0 | 0 | 0 | 0 |
| `tcl-sandbox` | 16 | 0 | 0 | 0 | 1 | 15 | 0 | 0 | 0 | 0 |
| `tcl-test-support` | 14 | 0 | 0 | 1 | 4 | 9 | 0 | 0 | 0 | 0 |
| `tcl-engine-api` | 9 | 0 | 0 | 0 | 3 | 6 | 0 | 0 | 0 | 0 |
| `tcl-host-native` | 9 | 0 | 1 | 0 | 3 | 5 | 0 | 0 | 0 | 0 |
| `bigip-query-wasm` | 6 | 0 | 0 | 0 | 5 | 1 | 0 | 0 | 0 | 0 |
| `tcl-f5mku` | 6 | 0 | 0 | 0 | 4 | 2 | 0 | 0 | 0 | 0 |
| `tcl-platform` | 5 | 0 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 0 |
| `tcl-vm-wasm` | 4 | 0 | 0 | 0 | 2 | 2 | 0 | 0 | 0 | 0 |
| `tcl-host-c-abi` | 3 | 0 | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 |
| `tcl-explorer-wasm` | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 |
| `tcl-userdirs` | 2 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 |
| `tcl-version` | 2 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 |
| `zed-query-check` | 2 | 0 | 0 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |
| `bigip-report-wasm` | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |

## Families with the most failing tests

A family is a crate, a test target and the first module under it.

| Family | Tests | Failing | Cites an oracle | Runs no Tcl source | In an overlap group |
| --- | ---: | ---: | ---: | ---: | ---: |
| `tcl-compiler/lib/analyser` | 3,512 | 408 | 12% | 15% | 2% |
| `tcl-compiler/lib/command_binding` | 1,371 | 283 | 3% | 14% | 0% |
| `tcl-compiler/lib/optimiser` | 576 | 147 | 8% | 12% | 2% |
| `tcl-compiler/it_4/analyser` | 514 | 112 | 18% | 8% | 3% |
| `tcl-compiler/it_2/value_transfer_witnesses` | 180 | 109 | 76% | 3% | 0% |
| `tcl-compiler/lib/registry_invocation` | 355 | 95 | 1% | 6% | 0% |
| `tcl-lsp-server/lib/tests` | 611 | 83 | 4% | 29% | 2% |
| `tcl-compiler/lib/cfg_builder` | 269 | 79 | 10% | 16% | 0% |
| `tcl-compiler/lib/lowering` | 348 | 72 | 6% | 14% | 3% |
| `tcl-compiler/it_3/taint` | 373 | 68 | 24% | 10% | 7% |
| `tcl-lsp-core/lib/workspace_index` | 251 | 62 | 15% | 2% | 2% |
| `tcl-lsp-core/lib/refactor` | 231 | 59 | 8% | 4% | 1% |
| `tcl-lsp-core/lib/references` | 174 | 52 | 16% | 4% | 6% |
| `tcl-compiler/lib/sccp` | 203 | 46 | 17% | 45% | 17% |
| `tcl-compiler/lib/shimmer` | 214 | 44 | 15% | 18% | 4% |
| `tcl-compiler/lib/ssa` | 171 | 44 | 4% | 33% | 2% |
| `tcl-compiler/it_3/optimiser` | 110 | 44 | 69% | 0% | 1% |
| `tcl-compiler/lib/taint` | 216 | 43 | 6% | 19% | 1% |
| `tcl-lsp-core/lib/minify` | 164 | 43 | 9% | 3% | 1% |
| `tcl-compiler/lib/interprocedural` | 124 | 38 | 5% | 12% | 0% |
| `tcl-lsp-core/lib/rename` | 136 | 38 | 18% | 6% | 13% |
| `tcl-compiler/lib/irules_checks` | 117 | 36 | 0% | 7% | 2% |
| `tcl-lsp-server/e2e/diagnostics` | 273 | 36 | 16% | 5% | 3% |
| `tcl-lsp-core/lib/code_actions` | 110 | 33 | 0% | 17% | 2% |
| `tcl-lsp-db/lib/tests` | 133 | 33 | 5% | 8% | 2% |
| `tcl-lsp-server/e2e/tcloo_navigation` | 52 | 29 | 44% | 0% | 4% |
| `tcl-codegen/lib/codegen` | 535 | 28 | 3% | 48% | 4% |
| `tcl-codegen/it_2/compiler_analysis_residual` | 119 | 28 | 37% | 18% | 4% |
| `tcl-lsp-core/lib/definition` | 242 | 28 | 18% | 5% | 3% |
| `tcl-compiler/lib/unit_scope` | 78 | 27 | 10% | 6% | 0% |
| `tcl-irule-test/lib/live` | 34 | 27 | 3% | 12% | 0% |
| `tcl-compiler/it_5/optimiser_coverage` | 96 | 25 | 78% | 0% | 1% |
| `tcl-compiler/lib/compilation_unit` | 104 | 24 | 10% | 14% | 0% |
| `tcl-codegen/it_3/optimiser_codegen_residual` | 67 | 23 | 30% | 1% | 0% |
| `tcl-lsp-core/lib/completion` | 184 | 23 | 7% | 21% | 1% |
| `tcl-lsp-server/e2e/irules` | 68 | 22 | 1% | 12% | 0% |
| `tcl-lsp-server/e2e/rename_safety` | 27 | 21 | 56% | 11% | 0% |
| `tcl-lsp-core/lib/rename_safety` | 51 | 20 | 27% | 0% | 4% |
| `tcl-vm/native_compilation_conformance/(root)` | 51 | 20 | 4% | 37% | 0% |
| `tcl-lsp-core/lib/hover` | 217 | 18 | 2% | 29% | 1% |

## Failing tests and overlap

- Failing tests that share sources, helpers and codes with another test: 36.
- Overlap groups where every member fails (one cause, several tests): 12.
- Overlap groups where some members pass and some fail (same input, different verdicts: the first place to look for a wrong expectation): 12.

## Name smells among failing tests

| Word in the module path or name | Tests | Failing |
| --- | ---: | ---: |
| `rust_behaviour` | 5 | 3 |
| `residual` | 809 | 78 |
| `coverage` | 284 | 57 |
| `depth` | 416 | 31 |
| `gap` | 29 | 1 |
| `kill` | 36 | 5 |
| `snapshot` | 163 | 10 |
| `golden` | 15 | 4 |
| `regression` | 2 | 0 |
| `pin` | 367 | 29 |
