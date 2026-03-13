# KCS: feature — Refactoring Tools

## Summary

Mechanical code refactorings: extract/inline variables, if-to-switch, switch-to-dict, brace expr, and data-group extraction with type-aware IP/CIDR support.

## Surface

lsp, mcp, claude-code, all-editors

## How to use

### Editor (all editors via LSP)

Place the cursor on the target construct and trigger code actions (Ctrl+. in VS Code, `<leader>ca` in Neovim, etc.). Available refactorings appear in the lightbulb menu:

- **Extract variable**: select an expression → "Extract into variable '$result'"
- **Inline variable**: cursor on `set var value` with a single use → "Inline variable '$var'"
- **if/elseif → switch**: cursor on `if` with equality chain → "Convert to switch on $var"
- **switch → dict lookup**: cursor on `switch` where every arm sets the same variable → "Convert to dict lookup"
- **Brace expr**: cursor on `expr "..."` → "Brace expr for safety and performance"
- **Extract to data-group** (iRules): cursor on `if` or `switch` with literal values → "Extract to data-group" (type-aware: IP/CIDR, integer, string)

### MCP tools

Individual tools for programmatic use:

| Tool | Description |
|------|-------------|
| `extract_variable` | Extract selection into a named variable |
| `inline_variable` | Inline a single-use variable |
| `if_to_switch` | Convert if/elseif chain to switch |
| `switch_to_dict` | Convert switch to dict lookup |
| `brace_expr` | Brace unbraced expr arguments |
| `extract_datagroup` | Static data-group extraction with tmsh definition |
| `suggest_datagroup_extractions` | AI-enhanced: scan for all data-group candidates with confidence scores |
| `refactor` | List all available refactorings at a position |

### Claude Code skills

- `/irule-datagroup <file>` — AI-enhanced data-group analysis using both static extraction and LLM reasoning

### Data-group type inference

The extract-to-datagroup refactoring automatically detects value types:

- **IP addresses**: `10.0.0.0`, `192.168.1.0/24`, `::ffff:10.0.0.0` → `type ip`
- **CIDR ranges**: `10.0.0.0/8`, `172.16.0.0/12` → `type ip` (preserves prefix length)
- **Integers**: `80`, `443`, `8080` → `type integer`
- **Strings**: `"/api"`, `"example.com"` → `type string`

## Operational context

All refactorings are implemented as pure functions in `core/refactoring/` that accept source text and return edit objects. The LSP code actions layer, MCP server, and Claude AI skills all consume these functions identically, ensuring consistent behaviour across all surfaces.

The AI-enhanced data-group tool (`suggest_datagroup_extractions`) returns structured context including pattern type, inferred value type, CIDR detection, body shape analysis (identical/set_mapping/return_mapping/complex), and confidence level (high/medium/low). This enables an LLM to make intelligent decisions about naming, consolidation across events, and coverage.

## File-path anchors

- `core/refactoring/__init__.py`
- `core/refactoring/_extract_variable.py`
- `core/refactoring/_inline_variable.py`
- `core/refactoring/_if_to_switch.py`
- `core/refactoring/_switch_to_dict.py`
- `core/refactoring/_brace_expr.py`
- `core/refactoring/_extract_datagroup.py`
- `lsp/features/code_actions.py`
- `ai/mcp/tcl_mcp_server.py`

## Failure modes

- Refactoring produces code with different semantics (e.g. inlining a variable whose value has side effects).
- Data-group type inference guesses wrong (e.g. string that looks like an integer).
- if-to-switch misidentifies the equality chain (e.g. mixed operators).

## Test anchors

- `tests/test_refactoring.py`

## Samples

- `samples/for_screenshots/25-extract-variable-{before,after}.tcl`
- `samples/for_screenshots/26-inline-variable-{before,after}.tcl`
- `samples/for_screenshots/27-if-to-switch-{before,after}.tcl`
- `samples/for_screenshots/28-switch-to-dict-{before,after}.tcl`
- `samples/for_screenshots/29-brace-expr-{before,after}.tcl`
- `samples/for_screenshots/30-extract-datagroup-{before,after}.irul`
- `samples/for_screenshots/31-extract-datagroup-ip-{before,after}.irul`
- `samples/for_screenshots/32-extract-datagroup-mapping-{before,after}.irul`

## Individual refactoring docs

- [Extract variable](kcs-feature-refactor-extract-variable.md)
- [Inline variable](kcs-feature-refactor-inline-variable.md)
- [if/elseif → switch](kcs-feature-refactor-if-to-switch.md)
- [switch → dict lookup](kcs-feature-refactor-switch-to-dict.md)
- [Brace expr](kcs-feature-refactor-brace-expr.md)
- [Extract to data-group](kcs-feature-refactor-extract-datagroup.md)

## Discoverability

- [KCS feature index](README.md)
- [Code actions](kcs-feature-code-actions.md)
