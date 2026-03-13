# KCS: feature — XC Translation

## Summary

Translate F5 BIG-IP iRules to F5 Distributed Cloud (XC) routes and service policies.

## Surface

vscode-command, vscode-chat, mcp, claude-code

## Availability

| Context | How |
|---------|-----|
| VS Code command | `Tcl: Translate iRule to F5 XC` |
| VS Code chat | `@irule /xc` |
| MCP | `xc_translate` tool |
| Claude Code | `/irule-xc` |

## How to use

- **VS Code**: Open an iRule file and run `Tcl: Translate iRule to F5 XC`. The output shows the equivalent XC configuration.
- **VS Code chat**: `@irule /xc` translates the current iRule with AI explanations.
- **MCP**: `xc_translate` tool accepts source code and returns XC config.
- **Claude Code**: `/irule-xc` translates with detailed commentary.

## Operational context

The translator maps iRule event handlers and commands to XC route and service policy equivalents. Some iRule patterns have no XC equivalent and are flagged as manual migration items.

## File-path anchors

- `core/xc/translate.py`
- `editors/vscode/src/extension.ts`

## Failure modes

- Unsupported iRule patterns silently dropped.
- XC output not valid YAML/JSON.

## Test anchors

- `tests/test_xc_translate.py`

## Discoverability

- [KCS feature index](README.md)
