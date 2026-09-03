---
name: generate-test
description: >
  Generate iRule test scripts for the Event Orchestrator framework
  (rust/tcl-irule-test). Extracts events, commands, pools, data groups, and
  variables, enumerates CFG paths, and produces a test file with one case per
  path plus a fakeCMP multi-TMM scenario when static:: or table state is
  CMP-sensitive.
allowed-tools: mcp__tcl-lsp__generate_irule_test, mcp__tcl-lsp__irule_cfg_paths, mcp__tcl-lsp__fakecmp_suggest_sources, mcp__tcl-lsp__fakecmp_which_tmm, Bash, Read, Write, Glob, Grep
---

# Generate iRule Test

Framework: `rust/tcl-irule-test/tcl/orchestrator.tcl`; contract
`docs/design/contracts/irule-test-framework.md`; worked examples
`example_test.tcl` and `example_multi_tmm_test.tcl` beside the framework.

## Steps

1. Read the iRule. Call `mcp__tcl-lsp__irule_cfg_paths` with its contents as
   `source`: every path to a terminal action (pool / reject / redirect …)
   with its branch conditions, priority, taint warnings, and
   test-generation questions. Prioritise security-sensitive actions and the
   `else` / `default` paths — those are the under-tested ones.
2. Call `mcp__tcl-lsp__generate_irule_test` with the same `source`. It
   returns a runnable scaffold: the framework source chain,
   `::orch::configure_tests` with profiles, iRule, and setup, one
   `::orch::test` per event and per CFG path with request parameters derived
   from the conditions, pool / data-group / header assertions, and a
   multi-TMM scenario when it sees `static::` writes outside `RULE_INIT`,
   `static::` counters, or `table incr` / `table set` shared state.
3. Tighten the scaffold: answer the generator's questions, replace
   placeholders with values the conditions actually select, delete cases the
   iRule cannot reach, add assertions the scaffold could not infer.
4. Multi-TMM: plan sources with `mcp__tcl-lsp__fakecmp_suggest_sources`
   (`tmm_count`, `count`) and confirm a tuple with `fakecmp_which_tmm`. Write
   the test for the *desired* behaviour — a CMP bug (a `static::` counter
   with 4 TMMs allows 4× the limit) should fail it.
5. Write the file and run it: `tclsh test_<name>.tcl`; `::orch::done` exits
   non-zero on failure.

## Framework API

| Command | Purpose |
|---|---|
| `::orch::configure_tests -profiles {TCP HTTP} -irule {…} -setup {…} [-tmm_count N -tmm_select auto]` | defaults for every test; `-tmm_select auto` hashes with fakeCMP |
| `::orch::test "name" "desc" -body {…}` | isolated case, state reset |
| `::orch::run_http_request -host … -uri … -method …`, `::orch::run_next_request` | simulate a request, keep-alive follow-up |
| `::orch::assert_that <subject> <verb> <value>` | subjects `pool_selected`, `http_uri`, `http_host`, `http_path`, `http_method`, `http_status`, `http_header "Name"`, `decision <cat> <action>`, `log`, `event`, `var <name>`; verbs `equals`, `not_equals`, `contains`, `starts_with`, `ends_with`, `matches`, `was_called`, `was_called_with`, `was_not_called` |
| `::orch::assert_pool_selected`, `assert_equal`, `assert {expr} msg` | classic assertions |
| `::orch::fakecmp_suggest_sources -count N`, `fakecmp_which_tmm a p da dp`, `tmm_ids` | multi-TMM planning |
| `::orch::done` | summary and exit code |

$ARGUMENTS
