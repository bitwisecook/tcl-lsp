# Sprawl outside the areas already studied

**Question:** after the studies of names, frames, namespaces, object systems, Tk, expressions, packages, words, containers, caching, data structures, contracts, constants, parsers, crates and editor actions, what sprawl remains unexamined?

**Method:**
- A survey of the whole repository (`rust/`, `runtime/rust`, `editors/`), excluding the areas above, looking for one concern implemented several times with different rules.
- The concrete faults below marked *observed* were checked in the code.
- Crate and client sizes were measured.

**Sizes that framed the search:**
- `tcl-registry`, 606,000 lines: 185,000 in `commands/`, 126,000 in `bigip/`, 42,000 in `base_objects/`.
- `runtime/rust`, 343,000; `tcl-vm`, 281,000; `tcl-lsp-core`, 276,000; `tcl-lsp-server`, 175,000.
- The `xtask` gates, 46,000.
- The VS Code client, 57,000 lines of TypeScript, mostly UI, tests and generated catalogues.

## Ranked findings

1. **Diagnostic severity and category have no owner.**
   - The policy layer (`tcl-lsp-core/src/diagnostic_policy.rs`: noqa, disables, profiles) is shared by the LSP, `tcl diag` and MCP.
   - Severity, though, is chosen by matching code strings inside each producer (`compiler_checks.rs` and the explorer's `serialise/warnings.rs`), and the generated catalogue has no severity field. *Observed:* the explorer renders S102 and taint findings as errors, while the LSP and CLI report warnings.
   - Three outputs skip the policy entirely: the BIG-IP query WASM `analyze.rs` (raw severity, no noqa, positions on a reformatted copy), Spec Studio samples, and the explorer's iRules view (hard-coded "warning").
   - There are nine `Severity` enums.
   - Two vocabularies are used for `category`: the compiler checks' "shimmer", "gvn" and "irules" against MCP's prefix rules.
2. **Dialect selection: one detector, different precedence per caller.** The owner is `tcl_registry::dialects::detect_dialect` (directive > shebang > content > extension > default).
   - The LSP's closed-document path lets the extension-derived language override the directive.
   - The CLI falls back to a hard-coded `tcl8.6` and ignores `dialect=` in `.tcl-lsp.ini`.
   - MCP passes no filename, so it has no extension tier, and does not normalise lone CRs.
   - *Observed in code:* a `.irul` file with `# tcl-lsp: dialect=tcl8.4` is iRules in the LSP workspace index but Tcl 8.4 in `tcl diag`.
3. **Positions and line endings.**
   - Two line rules (`LineIndex::new` ignores a lone CR; `new_lsp` breaks on it) are chosen per call site: 214 against 48 in `tcl-lsp-core`, and `new` only in MCP, the explorer, `tcl-bigip`, `f5-xc` and Spec Studio. `f5-xc` builds LSP ranges from `new`.
   - Separate `Range` types: `tcl-bigip` has an inclusive end and code-point offsets, and documents 1-based UTF-8 while the code computes 0-based UTF-16.
   - Hand-written LF-only line-start helpers in four crates; two identical copies of `char_col_to_utf16`.
   - Diagnostic JSON uses five unit conventions: 1-based UTF-16 (CLI), start only (report-gen), 1-based bytes (BIG-IP query), 0-based UTF-16 under a key named `byte_range` (MCP), and an inclusive end plus one (explorer).
4. **BIG-IP and iRules duplicates, with observed wrong answers:**
   - **Monitor expressions:** `lint.rs`'s `orphan-monitor`, `convert.rs` and `irule_context.rs` take the first word of a pool monitor, though `value::monitor_expression::try_parse` exists. *Observed:* in `monitor /Common/a and /Common/b`, `b` is reported as orphaned, and `min 1 of {…}` looks up a monitor called `min`.
   - **Event order:** `f5-cli`'s `explain_flow` lists `CLIENTSSL_HANDSHAKE` before `CLIENTSSL_CLIENTHELLO`; the registry has the opposite order. *Observed.*
   - **Profiles:** MCP's `irule_gen` puts `SERVERSSL_*` events in its SSL set and adds a `CLIENTSSL` profile for them. *Observed.*
   - **Further hand-written event tables:** the report generator's front end, VS Code's iRule skeleton (4 HTTP events against MCP's 7) and `f5-xc`'s mapping.
   - **Partition formatting:** two implementations with different outputs.
   - **Object graphs:** built three times (`tcl-bigip`, `bigip-report-gen`, `tcl-bigip-query`).
5. **Tcl error messages.**
   - The C-matching value describer is `tcl_value_syntax::list::describe_bad_value`, which reports "a list" for multi-word values. Five other describers exist, and at least nine VM, runtime and `tcl-cmd-core` sites always quote the value (*inferred* to differ on multi-word values).
   - "wrong # args" is built by at least six helpers, and the literal appears in 34 VM and 21 runtime files.
   - Return options are handled by three implementations around the shared `completion_options`.
6. **Configuration layering.** One ini parser (`config_ini.rs`), but three layer stacks:
   - LSP: global < editor < project.
   - CLI: global + invocation + project.
   - MCP: global + invocation with no project layer, and the global file re-read on every call.

   `invocation_layer` is copied between the CLI and MCP.
7. **Gate waivers.** Five waiver matchers, with comment-block lookback or a fixed five-line window. *Observed:* `resolution_drift` matches the bare substring `drift-ok:`, so a `number-drift-ok:`, `dialect-drift-ok:` or `segmentation-drift-ok:` waiver within four lines also silences it.
8. **Tracing and timing.**
   - 54 `TCL_LSP_TRACE_*` and about 51 other `TCL_LSP_*` environment variables, read directly in 151 files, mostly on every call.
   - 341 `eprintln!` calls in `tcl-compiler`.
   - `Instant::now` timing scattered over about 100 files, with no shared facade.
9. **Editor clients.** Mostly generated and gated. Still hand-written:
   - VS Code's iRule skeleton;
   - 16 static snippets that are not dialect-aware (`tcl-try` offered for 8.4) beside the server's snippets;
   - extension maps in the Neovim, Emacs and Helix READMEs (Neovim maps `irul` to `tcl`).
10. **Already well owned:** call, symbol and dataflow graphs (`tcl-lsp-core/src/graphs.rs`, used by the CLI and MCP); clock (`tcl_cmd_core::clock`). The per-crate dialect environment modules (`profile_for_dialect` in seven crates) are thin faces of the shared `tcl_registry::model::ingress` and are fine.

## How these fit the architecture

The same pattern as elsewhere: an owner exists, but its *policy* (severity, precedence, line rule, layer stack, waiver syntax) is applied by each caller. The fixes follow the architecture plan's rules:

- policy lives with the owner and is exposed as one call per surface;
- the agreement harness compares the surfaces (LSP, CLI, MCP, explorer, WASM, report generator);
- gates forbid the hand-written copies.

Severity and category belong in the registry's diagnostic catalogue with the code, so the generated TypeScript and Kotlin catalogues carry them too.

The BIG-IP domain crates fall under the architecture plan's rule that domain crates contribute data and hooks through interfaces. Their event tables come from the registry, and their monitor and partition logic from `tcl-bigip`'s typed values.

## Plan

Items 107–115 in [plan.md](plan.md). The quick fixes (item 107) are small, observed and independent of the phases.
