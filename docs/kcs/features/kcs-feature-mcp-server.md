# KCS: feature — MCP Server

> **Audience:** User
> **Type:** Functionality

## Summary

Model Context Protocol server exposing analysis/refactoring tools for AI agent integration.

## Applies to

MCP

## Availability

| Context | How |
|---------|-----|
| Claude Desktop | Add to `claude_desktop_config.json` |
| Claude Code / Codex | Registered via the repo `.mcp.json`, which launches `scripts/tcl-mcp` |
| Any MCP client | Run the native `tcl-mcp` binary directly (build with `make rust-mcp`, or let `scripts/tcl-mcp` fetch the prebuilt release asset) |

## How to use

The MCP server communicates over stdio using JSON-RPC 2.0. Connect any MCP-compatible AI client and the tools become available:

<!-- BEGIN GENERATED: tcl-mcp tool table (TCL_MCP_BLESS=1 cargo test -p tcl-mcp kcs_note) -->
| Tool | Description | Arguments |
|------|-------------|-----------|
| `call_graph` | Proc caller→callee graph with call sites, roots, and leaf procs. | `source`*, `dialect` |
| `symbol_graph` | Scope hierarchy with proc/variable definitions, references, and package requires. | `source`*, `dialect` |
| `dataflow_graph` | Taint warnings, tainted variables, and per-proc side-effect classification. | `source`*, `dialect` |
| `def_use_chains` | SSA def-use chains + memory-SSA aliases; optional variable filter. | `source`*, `dialect`, `variable` |
| `memory_aliases` | Memory-SSA alias sets (upvar/global/variable) with reasons and locations. | `source`*, `dialect` |
| `diagram` | Control-flow diagram data ({events, procedures}) from the IR. | `source`*, `dialect` |
| `detect_dialect` | Detect the Tcl dialect from source (+ optional filename). | `source`*, `filename` |
| `event_order` | iRule events in canonical firing order with multiplicity. | `source`*, `dialect` |
| `format_source` | Format Tcl/iRules source. | `source`*, `dialect`, `indent_size`, `indent_style`, `max_line_length` |
| `optimize` | Find optimisation opportunities and produce rewritten source. Only the rewrites the source's policy shows are applied: a `# noqa` on a command, a top-of-file `# tcl-lsp: disable=`, the profile and the global config.ini keep a rewrite off exactly as they keep a diagnostic off. | `source`*, `dialect`, `profile`, `disable`, `enable` |
| `compile_wasm` | Compile through the canonical analysis-aware WebAssembly pipeline and return the per-script WAT module and counts. | `source`*, `dialect` |
| `inline_variable` | Inline a single-use variable at the cursor. | `source`*, `line`*, `character`*, `dialect` |
| `if_to_switch` | Convert an if/elseif chain testing one variable to a switch. | `source`*, `line`*, `character`*, `dialect` |
| `switch_to_dict` | Convert a switch whose arms set one variable to a dict lookup. | `source`*, `line`*, `character`*, `dialect` |
| `brace_expr` | Brace an unbraced expr argument for safety/performance. | `source`*, `line`*, `character`*, `dialect` |
| `analyze` | Full analysis: diagnostics (+category), document symbols, detected events, and event firing order. Diagnostics are the source's shown set, including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and the source-style pass (W111, W112, W115, W118) — inline `# noqa`, top-of-file `# tcl-lsp: disable=`, the global config.ini and the call's disable/enable arguments all apply, as in the editor; and a `suppressed` array: every finding the policy hides, with its reason. | `source`*, `dialect`, `disable`, `enable` |
| `validate` | Diagnostics grouped by category (security, taint, thread-safety, control-flow, performance, style, …), from the source's shown set, including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and the source-style pass (W111, W112, W115, W118) (directives, the global config.ini and disable/enable apply); and a `suppressed` array: every finding the policy hides, with its reason. | `source`*, `dialect`, `disable`, `enable` |
| `review` | Security, taint, and thread-safety diagnostics for a focused review, from the source's shown set, including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and the source-style pass (W111, W112, W115, W118) (directives, the global config.ini and disable/enable apply); and a `suppressed` array: every finding the policy hides, with its reason. | `source`*, `dialect`, `disable`, `enable` |
| `find-legacy` | Auto-convertible legacy patterns with a modernisation hint per finding, from the source's shown set, including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and the source-style pass (W111, W112, W115, W118) (directives, the global config.ini and disable/enable apply); and a `suppressed` array: every finding the policy hides, with its reason. | `source`*, `dialect`, `disable`, `enable` |
| `event_info` | iRules event metadata: multiplicity, side, transport, implied profiles, valid commands. | `event_name`* |
| `command_info` | Registry metadata for a command: summary, synopsis, switches, valid events. | `command_name`*, `bigip_version` |
| `symbols` | Document symbol tree (procs, classes, methods, variables) with ranges. | `source`*, `dialect` |
| `hover` | Hover documentation (markdown) for the symbol at the cursor. | `source`*, `line`*, `character`*, `dialect` |
| `complete` | Completion items at the cursor (variables, procs, switches, subcommands, snippets). | `source`*, `line`*, `character`*, `dialect` |
| `goto_definition` | Definition location(s) for the symbol at the cursor. | `source`*, `line`*, `character`*, `dialect` |
| `find_references` | All references to the symbol at the cursor (including its declaration). | `source`*, `line`*, `character`*, `dialect` |
| `rename` | Text edits to rename the symbol at the cursor; null when not renameable. | `source`*, `line`*, `character`*, `new_name`*, `dialect` |
| `code_actions` | Code actions (quick-fixes + refactors) for a selection range: a fix for every finding the source shows — analyser, compiler-check and optimiser rewrite alike — and for no other. | `source`*, `start_line`*, `start_character`*, `end_line`*, `end_character`*, `dialect`, `disable`, `enable` |
| `extract_variable` | Extract a selected expression into a new `set var …` binding. | `source`*, `start_line`*, `start_character`*, `end_line`*, `end_character`*, `dialect`, `var_name` |
| `extract_datagroup` | Extract an if/switch over literals into an iRules data-group + lookup. | `source`*, `dialect`, `line`*, `character`*, `dg_name` |
| `refactor` | List the refactorings available at a selection (probe of all refactors). | `source`*, `start_line`*, `start_character`*, `end_line`*, `end_character`*, `dialect` |
| `generate_docstring` | Generate a docstring stub for a named proc. | `source`*, `proc_name`*, `style`, `decoration`, `dialect` |
| `read_proc_docs` | Structured docs for every proc: params, parsed docstring, inferred param traits. | `source`*, `dialect` |
| `update_docstrings` | Insert docstring stubs above every undocumented proc; returns rewritten source. | `source`*, `style`, `decoration`, `dialect` |
| `unminify_error` | Translate a minified error back to original symbol names / line numbers. | `error_message`*, `symbol_map`*, `minified_source`, `original_source` |
| `set_dialect` | Set the session default dialect used when a tool's dialect is auto-detected. | `dialect`* |
| `fakecmp_which_tmm` | Which TMM a connection 4-tuple hashes to under the fakeCMP disaggregator. | `tmm_count`*, `src_addr`*, `src_port`*, `dst_addr`*, `dst_port`* |
| `fakecmp_suggest_sources` | Source tuples that spread test traffic across every TMM. | `tmm_count`*, `count`, `dst_addr`, `dst_port` |
| `irule_with_context` | Bundle each iRule in a BIG-IP config with the objects it references (pools, data-groups, profiles, monitors, …). | `config_text`, `config_paths`, `rule_path`, `format`, `transitive` |
| `help` | Search the KCS feature knowledge base (empty topic → full catalogue). | `topic`, `dialect`, `limit` |
| `xc_translate` | Translate an iRule to F5 Distributed Cloud (XC) config — terraform HCL, ves.io JSON, or pasteable XC Console documents, with per-command coverage. | `source`*, `output_format` |
| `explain_flow` | Narrate a captured session (pcap) against a BIG-IP config: matched VS, profiles, HTTP/TLS, ordered iRule events, pool decision, optional simulation. | `pcap_path`*, `config_text`, `config_paths`, `use_tshark`, `keylog_path`, `tshark_filter`, `max_event_body_lines`, `simulate` |
| `tk_layout` | Build the shared, versioned static Tk UI model without executing source. Returns registry-resolved widgets, literal options, geometry evidence, source spans, conflicts, and explicit uncertainty. | `source`*, `dialect` |
| `irule_cfg_paths` | Enumerate CFG paths through an iRule to terminal actions (pool/reject/redirect/…), each annotated with priority, relevant taint warnings, and test-generation questions. | `source`* |
| `suggest_datagroup_extractions` | Scan iRules source for if/switch patterns extractable to data-groups; returns per-candidate value type, CIDR flag, body-shape, confidence, and whether the static extractor applies. | `source`* |
| `generate_irule_test` | Generate a runnable iRule test scaffold (per-event + per-CFG-path cases, multi-TMM scenario) with extracted events/profiles/commands/pools/datagroups. | `source`* |
| `spectcl_check` | Validate a SpecTcl (.tclspec) pack by evaluating it in the deterministic sandbox (no clock/IO, budgets, transactional registration — safe on generated packs). Returns per-command draft fields, loader notices (dropped/unknown words), hooks with family and cacheability, hooks reading past their `-inputs`, and collisions with the shipped registry; plus `load_error` (determinism denial, budget, or Tcl error — nothing loads), `target_dependent` (`available?` was queried), `provenance` (what `tier`/`trust` resolve to), `untrusted_tier_refusal` (what an untrusted install at `tier` would refuse the pack for), `dormant_hooks` (hook bodies that stay dormant under that provenance — declarative facts install regardless; empty when that provenance refuses the pack, since nothing of it loads), and `stamp_refusals` (codegen_hook, inline_codegen_hook, or semantic_operation Intrinsic rows that install drops: only a bundled pack may stamp, and only as its `alias_of` target's own). Use spectcl_expand to see what a templated pack registered. | `source`*, `dialect`, `tier` (bundled, user, workspace, studio-override), `trust` (trusted, untrusted) |
| `spectcl_expand` | Expand a SpecTcl (.tclspec) pack to canonical form: evaluate it in the sandbox and return the straight-line source of every registration it made (no proc/foreach/set). Review the expansion, not the loop, before shipping a templated pack and after every template edit; a straight-line pack comes back as itself. Also reports target-dependence (`available?` queried) and load notices. Hand the pack to spectcl_check for field-level validation. | `source`* |
| `spec_import` | Derive version ranges for a Tcl package's commands from LOCAL release snapshots (directories, .zip, or .tar.gz — never fetches): drafts each release, diffs them, and returns the rendered .tclspec pack (evidence in its header), per-command introduced/retired versions, and every contradiction. Obtain sources first with `tcl spec import --github OWNER/REPO` or git; hand the pack to spectcl_check. | `snapshots`*, `dialect`, `package`, `complete_history` |
<!-- END GENERATED: tcl-mcp tool table -->

`analyze`, `validate`, `review`, and `find-legacy` also return a
`suppressed` array: every finding the policy hid, with its reason, so an
agent can tell a clean report from one that hid something (see
[how do I turn a diagnostic off?](../kcs-howto-suppress-diagnostics.md)).

## Operational context

Native Rust implementation — a single self-contained `tcl-mcp` binary, no runtime dependencies. Uses the same analysis engine (`tcl-compiler`/`tcl-lsp-core`) as the LSP server.

## Failure modes

- Stdin/stdout hijacked by other tools.

## Test anchors

- `rust/tcl-mcp/` crate tests

## Example

An AI agent calls the `analyze` tool over stdio using JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "analyze",
    "arguments": {
      "source": "when HTTP_REQUEST {\n  log local0. $unset_var\n}\n",
      "dialect": "f5-irules"
    }
  }
}
```

The server responds with a structured report listing the read-before-set
variable diagnostic (`W210`), the single `HTTP_REQUEST` event, and
the full symbol table for the script. The agent can then follow up
with `code_actions` or `optimize` on the same source.

## Discoverability

- [KCS feature index](README.md)
