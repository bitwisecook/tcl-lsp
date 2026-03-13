---
name: generate-test
description: >
  Generate iRule test scripts using the Event Orchestrator framework.
  Analyzes an iRule to extract events, commands, pools, data groups,
  and variables, then produces a complete test file with assertions.
  Auto-detects multi-TMM / CMP-sensitive patterns and adds fakeCMP
  distribution tests when appropriate.
allowed-tools: Bash, Read, Write, Glob, Grep
---

# Generate iRule Test

Generates a complete test script for an iRule using the Event Orchestrator
test framework (`core/irule_test/tcl/orchestrator.tcl`).

## Usage

```bash
python3 -m ai.claude.tcl_ai generate-test <irule_file>
```

## What it does

1. **Extracts metadata** from the iRule source:
   - Events used (HTTP_REQUEST, CLIENT_ACCEPTED, etc.)
   - iRule commands (HTTP::uri, pool, table, etc.)
   - Object references (pool names, data groups, nodes)
   - Variable patterns (static::, connection-scoped)
   - Profile requirements (TCP, HTTP, SSL, DNS)

2. **Analyzes control flow** using the compiler IR:
   - Walks the IR to extract all unique paths through if/switch/loop branches
   - Each path records the chain of conditions leading to a terminal action
   - Generates one test case per code path for full branch coverage
   - Falls back to template-based generation if CFG analysis produces no paths

3. **Generates test script** with:
   - Framework source chain (compat84 → orchestrator)
   - `::orch::configure_tests` with profiles, iRule source, and setup
   - CFG-informed `::orch::test` cases targeting each branch path
   - Request parameters derived from branch conditions (host, URI, headers)
   - Pool/datagroup/header assertions
   - Exit with `::orch::done` for pass/fail summary

3. **Detects multi-TMM patterns** automatically:
   - `static::` writes in hot events (not just RULE_INIT)
   - Counter/rate-limiter patterns using `static::`
   - `table incr`/`table set` shared-state patterns
   - When detected, adds a multi-TMM scenario using `fakecmp_suggest_sources`

## Examples

### Generate a test for a simple routing iRule
```
$ python3 -m ai.claude.tcl_ai generate-test samples/for_screenshots/ai-scene.irul
```

### Generate and run immediately
```
$ python3 -m ai.claude.tcl_ai generate-test my_irule.tcl > test_my_irule.tcl
$ tclsh test_my_irule.tcl
```

### Use fakeCMP tools to plan multi-TMM distribution
```tcl
# In a generated or hand-written test:
set plan [::orch::fakecmp_suggest_sources -count 2]
foreach tmm_id [::orch::tmm_ids] {
    set sources [dict get $plan $tmm_id]
    foreach {addr port} $sources {
        ::orch::configure -client_addr $addr -client_port $port
        ::orch::run_http_request -host app.example.com
    }
}
```

## When to use

- When writing tests for a new or modified iRule
- When you need to verify CMP/multi-TMM behavior
- When migrating from manual testing to automated test scripts
- As a starting point that you then customize with specific assertions

## Key framework commands

| Command | Purpose |
|---|---|
| `::orch::configure_tests -profiles ... -irule {...}` | Set up test defaults |
| `::orch::test "name" "desc" -body {...}` | Define a test case |
| `::orch::run_http_request -host ... -uri ...` | Simulate HTTP traffic |
| `::orch::assert_pool_selected pool_name` | Assert pool routing |
| `::orch::assert_equal $actual $expected` | Assert equality |
| `::orch::assert {$expr} "message"` | Assert condition |
| `::orch::fakecmp_suggest_sources -count N` | Plan TMM distribution |
| `::orch::fakecmp_which_tmm addr port dst_addr dst_port` | Query TMM for tuple |
| `::orch::done` | Print summary, return exit code |

## Reference

- Framework source: `core/irule_test/tcl/orchestrator.tcl`
- Example tests: `core/irule_test/tcl/example_test.tcl`, `example_multi_tmm_test.tcl`
- KCS documentation: `docs/kcs/kcs-irule-test-framework.md`
- MCP tools: `generate_irule_test`, `irule_cfg_paths`, `fakecmp_which_tmm`, `fakecmp_suggest_sources`
- CLI: `python3 -m ai.claude.tcl_ai cfg-paths <irule_file>` — extract control flow paths

$ARGUMENTS
