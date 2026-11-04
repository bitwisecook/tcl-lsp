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
| #2365 | Pending audit | Check diagnostics for stub directives outside an active stub block; current docs still say these directives are ignored. |
| #2366 | Pending audit | Compare document stubs passed to `tcl opt` with `tcl diag` and the LSP compilation-unit inputs. |
| #2372 | Documentation corrected | Removed the unsupported `info vars` / `info locals` and `catch {set _ $X}` narrowing claims from the W210 page; it now links to the SCCP contract, which states only `info exists` and `array exists` are recognised. |
| #2422 | Pending audit | Measure and inspect role assignment in `CommandRegistry::callback_script_indices` / `script_timing` for large literal commands. |
| #2446 | Documentation corrected | `kcs-qa-tcl-lsp-annotations.md` now states that stub roles guide analysis but stub arity is not checked; this matches `kcs-howto-annotate-commands-with-stubs.md` and `flush_arity_diagnostics` in `rust/tcl-compiler/src/analyser/diagnostics/validity.rs`. |
| #2447 | Documentation corrected | `kcs-howto-suppress-diagnostics.md` now says project `.tcl-lsp.ini` is watched and global config changes require a restart, matching `register_file_watchers` and `did_change_watched_files` in `rust/tcl-lsp-server/src/lib.rs`. |
| #2448 | Documentation corrected | Removed the nonexistent project "local-only ignore list" workaround from `kcs-qa-how-tcl-lsp-loads-configuration.md`; it now explains project precedence and advises coordinating with the team. |
| #2449 | Documentation corrected | `kcs-howto-write-a-tclspec-pack.md` no longer calls vocabulary 2.0 current; the loader's `NEWEST_VOCABULARY_VERSION` is 2.2, while the separate upgrade-to-2.0 wording remains. |

## Verification

A fresh E2E build was started for #2181, then stopped at the coordinator's request to keep concurrent builds within the machine's memory budget. The #2299 and #2270 test invocations were also stopped when their dependency graph began compiling `tcl-compiler`; neither reached a test result. Re-run the focused tests after the coordinator opens a build slot.
