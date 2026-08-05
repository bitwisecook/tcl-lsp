# KCS: feature — Refactoring Tools

> **Audience:** User
> **Type:** Functionality

## Summary

Mechanical code refactorings: extract/inline variables, extract/inline procs, if-to-switch, switch-to-dict, brace expr, and data-group extraction with type-aware IP/CIDR support.

## Applies to

all-editors, MCP, Claude skill, refactoring

## How to use

### Editor (all editors via LSP)

Place the cursor on the target construct and trigger code actions (Ctrl+. in VS Code, `<leader>ca` in Neovim, etc.). Available refactorings appear in the lightbulb menu:

- **Extract variable**: select an expression → "Extract into variable '$result'"
- **Inline variable**: cursor on `set var value` with a single use → "Inline variable '$var'"
- **Extract into proc**: select whole commands → "Extract selection into proc" (caller-frame writes are carried through with `upvar`)
- **Inline proc**: cursor on a call → "Inline proc 'name'" (parameters are bound to the call's argument values, defaults included)
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

All refactorings are implemented as pure functions in `tooling/refactoring/` that accept source text and return edit objects. The LSP code actions layer, MCP server, and Claude AI skills all consume these functions identically, ensuring consistent behaviour across all surfaces.

The AI-enhanced data-group tool (`suggest_datagroup_extractions`) returns structured context including pattern type, inferred value type, CIDR detection, body shape analysis (identical/set_mapping/return_mapping/complex), and confidence level (high/medium/low). This enables an LLM to make intelligent decisions about naming, consolidation across events, and coverage.

## File-path anchors

- `tooling/refactoring/__init__.py`
- `tooling/refactoring/_extract_variable.py`
- `tooling/refactoring/_inline_variable.py`
- `tooling/refactoring/_if_to_switch.py`
- `tooling/refactoring/_switch_to_dict.py`
- `tooling/refactoring/_brace_expr.py`
- `tooling/refactoring/_extract_datagroup.py`
- `server/features/code_actions.py`
- `ai/mcp/tcl_mcp_server.py`

## Refusals

A refactoring that finds its subject but cannot preserve behaviour is offered **greyed out**, with a plain-English reason (LSP's `disabled.reason`), rather than silently omitted. A missing menu entry tells you nothing; "the body calls 'return', which acts on the call frame" tells you what to change first. The extract-proc and inline-proc refactorings both work this way.

## Failure modes

- Refactoring produces code with different semantics (e.g. inlining a variable whose value has side effects).
- Data-group type inference guesses wrong (e.g. string that looks like an integer).
- if-to-switch misidentifies the equality chain (e.g. mixed operators).

## Test anchors

- `tests/test_refactoring.py`

## Example

This page is an index — for a concrete before/after, open any of
the individual refactoring notes linked below. As a quick taste,
the if-to-switch refactoring turns this:

```tcl
if {$method eq "GET"} {
    set action read
} elseif {$method eq "POST"} {
    set action create
} else {
    set action unknown
}
```

into this:

```tcl
switch -exact -- $method {
    "GET"   { set action read }
    "POST"  { set action create }
    default { set action unknown }
}
```

See [if/elseif → switch](kcs-feature-refactor-if-to-switch.md)
for the full walkthrough.

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
- [Extract into proc](kcs-feature-refactor-extract-proc.md)
- [Inline proc](kcs-feature-refactor-inline-proc.md)
- [if/elseif → switch](kcs-feature-refactor-if-to-switch.md)
- [switch → dict lookup](kcs-feature-refactor-switch-to-dict.md)
- [Brace expr](kcs-feature-refactor-brace-expr.md)
- [Extract to data-group](kcs-feature-refactor-extract-datagroup.md)

## Discoverability

- [KCS feature index](README.md)
- [Code actions](kcs-feature-code-actions.md)
