# LSP release packs and registry status

Updated 4 November 2026. Branch: `work`. Test execution remains pending where noted; source inspection is not a passing test result.

## Findings

| Issue | Status | Source evidence / next check |
| --- | --- | --- |
| #2181 | Existing implementation and regression coverage; execution pending | `rust/tcl-lsp-server/tests/e2e/spec_packs.rs` tests pack `Body` roles in nested semantic tokens and pack-declared extensions. VS Code also has `src/test/packFileAssociations.test.ts`. Run these after a build slot opens. |
| #2242 | Existing implementation and regression coverage; execution pending | `rust/tcl-lsp-server/tests/e2e/spec_packs.rs::a_workspace_pack_subcommand_arity_error_is_published` checks the exact workspace-pack subcommand arity case over LSP. |
| #2270 | Fix drafted; execution pending | `CommandRegistry::best_visible` now ranks installed pack provenance before shipped surface specificity. `codegen_stamps.rs` removes the dialect workaround so the existing pack override path exercises the unscoped case. |
| #2299 | Fix drafted; execution pending | `SecurityFloor::apply` carries compiler and dispatch stamps to same-route shipped subcommands, command forms, and nested subcommand forms. `i6_security_floor.rs` covers array-for lowering, string-length intrinsic, form lowering, renamed form routes, and removed/added routes. |
| #2130 | Existing implementation appears to resolve native const-fold hooks and reject unknown or wrongly scoped identifiers; full family audit pending | `rust/tcl-spectcl/src/loader.rs::native_fold`, `native_hook_tables_cover_their_catalogues`, and `docs/design/spec-dsl-examples/string.tclspec`. |
| #2137 | Existing implementation and round-trip coverage | `render_spectcl.rs` no longer lists `semantic_operation` in `GAPS`; `spectcl_roundtrip.rs::stamp_windows_survive_the_round_trip` covers the operation and its lifecycle. |
| #2138 | Existing implementation documented; focused compiler check pending | `dialect-stubs.md` documents all seven flags and their consumers; `DocumentCommandSurface` carries declared traits and side effects. |
| #2259 | Pending audit | Check role support for variable-cell names in `upvar` and `namespace upvar`, and whether role-driven rename and semantic-token consumers use it. |
| #2303 | Pending audit | Verify discovery follows installed dependency symlinks without broadening workspace traversal. |
| #2309 | Fix drafted; execution pending | `socket_.rs` declares client and server invocation forms separately. `rust/tcl-compiler/src/analyser/diagnostics/tests.rs::socket_server_form_counts_only_its_port_as_positional` checks valid and missing-operand client/server shapes. |
| #2365 | Closed after [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2365#issuecomment-6631444932) | W153 now marks a valid `# tcl-lsp:` stub directive outside an active block on its comment line while the unknown call retains W123. The focused `stub_directive_outside_block_reports_its_own_line_2365` test passed after a clean build: the original issue input has one W153 and W123, the bounded form has neither, a comment-shaped line inside a braced word has no W153, and out-of-block `expr-func`/`expr-op` forms each get W153 while a malformed stub does not. Existing bounded-stub control passed. The policy test passed for editor and file suppression, and targeted production Clippy passed. Generated diagnostic catalogues and their drift checks pass; `diag-emission-check` passes. The per-code W153 KCS page is indexed. Full repository gates remain pending; sparse-checkout `kcs-index-links` reports unrelated missing paths and no W153 error. |
| #2366 | Pending audit | Compare document stubs passed to `tcl opt` with `tcl diag` and the LSP compilation-unit inputs. |
| #2372 | Closed after [source-verification proof](https://github.com/bitwisecook/tcl-lsp/issues/2372#issuecomment-6636790281) | Removed the unsupported `info vars` / `info locals` and `catch {set _ $X}` narrowing claims from the W210 page; it now links to the SCCP contract, which states only `info exists` and `array exists` are recognised. |
| #2422 | Pending audit | Measure and inspect role assignment in `CommandRegistry::callback_script_indices` / `script_timing` for large literal commands. |
| #2446 | Closed after [source-verification proof](https://github.com/bitwisecook/tcl-lsp/issues/2446#issuecomment-6636792651) | `kcs-qa-tcl-lsp-annotations.md` now states that stub roles guide analysis but stub arity is not checked; this matches `kcs-howto-annotate-commands-with-stubs.md` and `flush_arity_diagnostics` in `rust/tcl-compiler/src/analyser/diagnostics/validity.rs`. |
| #2447 | Closed after [source-verification proof](https://github.com/bitwisecook/tcl-lsp/issues/2447#issuecomment-6636794767) | `kcs-howto-suppress-diagnostics.md` now says project `.tcl-lsp.ini` is watched and global config changes require a restart, matching `register_file_watchers` and `did_change_watched_files` in `rust/tcl-lsp-server/src/lib.rs`. |
| #2448 | Closed after [source-verification proof](https://github.com/bitwisecook/tcl-lsp/issues/2448#issuecomment-6636796786) | Removed the nonexistent project "local-only ignore list" workaround from `kcs-qa-how-tcl-lsp-loads-configuration.md`; it now explains project precedence and advises coordinating with the team. |
| #2449 | Closed after [source-verification proof](https://github.com/bitwisecook/tcl-lsp/issues/2449#issuecomment-6636798978) | `kcs-howto-write-a-tclspec-pack.md` no longer calls vocabulary 2.0 current; the loader's `NEWEST_VOCABULARY_VERSION` is 2.2, while the separate upgrade-to-2.0 wording remains. |

## Verification

A fresh E2E build was started for #2181, then stopped at the coordinator's request to keep concurrent builds within the machine's memory budget. The #2299 and #2270 test invocations were also stopped when their dependency graph began compiling `tcl-compiler`; neither reached a test result. Re-run the focused tests after the coordinator opens a build slot.

## SpecTcl loading correctness

The LSP loads bundled vendor commands eagerly during its ordinary startup and
reload path. It does not activate process-wide demand deferral or schedule a
pack reload from a registry request. Finished-pack snapshots and segmentation
memoisation remain enabled, and native hook bodies use the shared service actor, and browser/WASI hosts use a synchronous thread-local service without OS threads.
The CLI retains its synchronous demand-driven pack loading.

The regression
`spec_packs::the_bundled_eda_loadables_make_their_vendor_commands_known` now checks
that every bundled EDA pack already reports commands before any vendor document
opens. Each vendor's first hover must know its own command; the existing checks
retain shared SDC/UPF commands and reject a rival vendor's command. This avoids
accepting eventual reload completion as proof of first-request completeness.

Prepared cache controls distinguish provider-free and include-aware finished
snapshots, retain warm hits in both domains, and check included-file declaration
order and edits. Manifest identity uses exact source bytes; modified CR/CRLF
sources refuse the shipped manifest and still evaluate in full. A deterministic
hook publication control interleaves a real changed plan with host installation
and checks its installed generation and result.

Current scoped result: the fresh five-ELF SpecTcl/Registry diagnostic passes17/19. Cache domains, coherent publication, retirement, stale-table and quarantine controls pass. Two failures remain: pack display data containing CR becomes LF on an unisolated evaluation branch, and Constraints emitter registration overwrites the LiteralArgumentValidator family in a mixed engine. Neither failure is fixed or waived. `/workspace/tmp/lsp-merged-cohort/hook-publication19-postbuild-diagnostic/result.json` records each result; its parent prebuild refusal on20 late VM fixture inputs remains separate.

Actual Browser and WASI host jobs succeed on e5b98d38, including non-tabulated hook execution and changed-content responses; newer run41919112254 also has successful Browser/WASI jobs. These do not certify every hook family or the final recapture source. The named vendor E2E control, generated manifest drift and full rust-check/prep-pr gates remain open.

Parallel pack work is stopped at the user's request. Investigation notes are `/tmp/spec-hook-two-failures-stop-handoff/HANDOFF.md`; no partial emitter/CR patch is adopted. The active autoload fix is now verified by9/9 selected controls and three consecutive public definitions. All other pack investigations remain stopped; final status and push conclude the session.
