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

- **VS Code**: Open an iRule file and run `Tcl: Translate iRule to F5 XC`. Two scratch tabs open beside the file — the Terraform HCL and the ves.io JSON — and a notification reports the per-command coverage.
- **VS Code chat**: `@irule /xc` translates the current iRule with AI explanations.
- **MCP**: `xc_translate` accepts the source and returns the Terraform HCL, the ves.io JSON, and the per-command coverage.
- **Claude Code**: `/irule-xc` translates with detailed commentary.

## Operational context

The translator maps iRule event handlers and commands to XC route and service-policy equivalents, and renders them two ways: Terraform HCL for the `volterra` provider, and ves.io JSON-API objects. Some iRule patterns have no XC equivalent and are reported as untranslatable rather than guessed at.

## Failure modes

- Unsupported iRule patterns silently dropped.
- Emitted HCL or JSON that the XC API rejects.

## Example

### Before (iRule)

```tcl
when HTTP_REQUEST {
    if { [HTTP::uri] starts_with "/api" } {
        pool api_pool
    }
}
```

### After (Terraform HCL, route excerpt)

```hcl
    simple_route {
      path {
        prefix = "/api"
      }
      origin_pools {
        pool {
          name      = volterra_origin_pool.api_pool.name
          namespace = volterra_origin_pool.api_pool.namespace
        }
      }
    }
```

The matching `volterra_origin_pool` resource is emitted alongside the load
balancer, with `TODO` markers where the origin servers have to be filled in.
The same translation is also rendered as ves.io JSON-API objects.

Patterns without a direct equivalent — for example a `HTTP::header insert`
that mutates response headers — are counted as untranslatable and listed with
the command that produced them.

## Discoverability

- [KCS feature index](README.md)
