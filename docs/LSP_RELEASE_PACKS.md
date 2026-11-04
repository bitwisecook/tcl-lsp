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
| #2309 | Pending audit | Reproduce the `socket -server` arity case and check the registry form/option model. |
| #2365 | Pending audit | Check diagnostics for stub directives outside an active stub block; current docs still say these directives are ignored. |
| #2366 | Pending audit | Compare document stubs passed to `tcl opt` with `tcl diag` and the LSP compilation-unit inputs. |
| #2372 | Pending audit | Compare the W210 KCS claim about `info vars` / `catch {set _ $X}` with analyser behavior and the SCCP contract. |
| #2422 | Pending audit | Measure and inspect role assignment in `CommandRegistry::callback_script_indices` / `script_timing` for large literal commands. |
| #2446 | Pending docs correction | `kcs-qa-tcl-lsp-annotations.md` says stub arity is checked; verify and align it with `kcs-howto-annotate-commands-with-stubs.md`. |
| #2447 | Pending docs correction | `kcs-howto-suppress-diagnostics.md` reload guidance conflicts with project-config watching. |
| #2448 | Pending docs correction | `kcs-qa-how-tcl-lsp-loads-configuration.md` refers to a nonexistent local-only ignore list. |
| #2449 | Pending docs correction | `kcs-howto-write-a-tclspec-pack.md` describes vocabulary 2.0 as current; loader currently reports 2.2 as newest. |

## Verification

A fresh E2E build was started for #2181, then stopped at the coordinator's request to keep concurrent builds within the machine's memory budget. Before network access was enabled, a second attempt could not fetch `inventory`; no product test ran. Re-run the focused tests after the coordinator opens a build slot.
