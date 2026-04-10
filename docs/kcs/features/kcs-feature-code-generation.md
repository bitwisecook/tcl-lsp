# KCS: feature — Test Generation

> **Audience:** User
> **Type:** Functionality

## Summary

Analyse an iRule and generate a complete test script using the Event Orchestrator test framework, with event mocks, command stubs, and assertions.

## Applies to

tcl-lsp CLI, Claude skill

## Question

How do I auto-generate a test script for my iRule?

## How to use

### tcl-lsp CLI

```
tcl generate-test my_irule.irul
tcl generate-test my_irule.irul -o test_my_irule.tcl
```

### Claude Code

The `/generate-test` skill analyses the iRule and produces a test file with commentary.

## Example

Given an iRule that routes `/api` traffic to `api_pool`:

```tcl
when HTTP_REQUEST {
    if {[HTTP::uri] starts_with "/api"} {
        pool api_pool
    }
}
```

The generated test contains:

```tcl
# Test: HTTP_REQUEST routes /api to api_pool
setup_event HTTP_REQUEST
mock_command HTTP::uri "/api/v1/users"
run_event
assert_pool api_pool
```

The generator extracts events, the commands called inside each event, referenced pools and data groups, and variable flow, then produces one test case per significant decision path.

## Related

- [KCS feature index](README.md)
- [iRule Test Framework](../../design/contracts/irule-test-framework.md) — the Event Orchestrator architecture
- [Control-Flow Diagrams](kcs-feature-control-flow-diagrams.md) — the CFG path enumeration the generator uses
- [Diagnostics](kcs-feature-diagnostics.md) — the analysis engine behind event and command extraction
