# KCS: feature — Test Generation

> **Audience:** User
> **Type:** Functionality

## Summary

Analyse an iRule and generate a complete test script using the Event Orchestrator test framework, with event mocks, command stubs, and assertions.

## Applies to

MCP, Copilot Chat, Claude skill

## Question

How do I auto-generate a test script for my iRule?

## How to use

### MCP

Call the `generate_irule_test` tool with the iRule source:

```json
{"tool": "generate_irule_test", "arguments": {"source": "when HTTP_REQUEST { ... }"}}
```

### VS Code Copilot Chat

Run `@irule /test` on the open iRule.

### Claude Code

Run `/generate-test`.  The skill enumerates the control-flow paths first, then
generates one case per path and adds a multi-TMM scenario when `static::` or
`table` state is affected by which TMM handles the connection.

## Example

Given an iRule that routes `/api` traffic to `api_pool`:

```tcl
when HTTP_REQUEST {
    if {[HTTP::uri] starts_with "/api"} {
        pool api_pool
    }
}
```

The generated script sources the framework, declares the iRule and its
fixtures, and adds one case per path:

```tcl
::orch::configure_tests \
    -profiles {TCP HTTP} \
    -irule { when HTTP_REQUEST { if {[HTTP::uri] starts_with "/api"} { pool api_pool } } } \
    -setup { ::orch::add_pool api_pool {10.0.1.1:80} }

::orch::test "routing-1.0" "/api routes to api_pool" -body {
    ::orch::run_http_request -uri /api/v1/users
    ::orch::assert_that pool_selected equals api_pool
}

::orch::run_and_exit
```

The generator extracts events, the commands called inside each event,
referenced pools and data groups, and variable flow, then produces one test
case per significant decision path.

## Related

- [KCS feature index](README.md)
- [iRule Test Framework](../../design/contracts/irule-test-framework.md) — the Event Orchestrator architecture
- [Control-Flow Diagrams](kcs-feature-control-flow-diagrams.md) — the path enumeration the generator uses
- [Diagnostics](kcs-feature-diagnostics.md) — the analysis engine behind event and command extraction
