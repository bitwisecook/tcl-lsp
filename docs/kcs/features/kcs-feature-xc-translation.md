# KCS: feature — XC Translation

> **Audience:** User
> **Type:** Functionality

## Summary

Translate F5 BIG-IP iRules to F5 Distributed Cloud (XC) routes and service policies.

## Applies to

VS Code, Copilot Chat, MCP, Claude skill

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

- `editors/vscode/src/extension.ts`

## Failure modes

- Unsupported iRule patterns silently dropped.
- XC output not valid YAML/JSON.

## Example

### Before (iRule)

```tcl
when HTTP_REQUEST {
    if { [HTTP::uri] starts_with "/api" } {
        pool api_pool
    }
}
```

### After (XC route policy)

```yaml
routes:
  - match:
      path:
        prefix: /api
    route_destination:
      pool:
        name: api_pool
```

Patterns without a direct equivalent — for example, a `HTTP::header
insert` that mutates response headers — are emitted as a comment in
the YAML output flagged as a manual migration item.

## Discoverability

- [KCS feature index](README.md)
