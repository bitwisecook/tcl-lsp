# KCS: iRule Event Orchestrator test framework

## Symptom

Need to test iRule logic (pool selection, header manipulation, data
group lookups, persistence, logging, connection control) without a
BIG-IP device.

## Operational context

`core/irule_test/` provides a complete TMM simulation: event
orchestration, protocol state machines, command mocks, assertion DSL,
and optional Python bridge.  Tests run with just `tclsh` (no Python
needed for pure-Tcl usage) or via `tkinter.Tcl()` / subprocess from
Python.

## Architecture

```
User test script (Tcl or Python)
    -> orchestrator.tcl  (event ordering, flow chains, assertions)
    -> itest_core.tcl    (iRule loader, event firer)
    -> command_mocks.tcl (73 hand-written + 1188 auto-generated stubs)
    -> state_layers.tcl  (10 protocol state namespaces)
    -> tmm_shim.tcl      (disabled commands, info override)
    -> expr_ops.tcl      (contains/starts_with/ends_with/etc.)
```

## Decision rules / contracts

1. **Single source of truth**: Python registries are authoritative.
   Tcl data files (`_event_data.tcl`, `_registry_data.tcl`,
   `_mock_stubs.tcl`) are generated from Python and must never be
   edited by hand.  Staleness tests enforce this.

2. **Decision log over state inspection**: Tests assert on the
   decision log (`{category action args}` triples) rather than raw
   state.  This makes tests more readable and less brittle.

3. **State layers mirror TMM**: The `::state::` namespace hierarchy
   (`connection`, `tls`, `http`, `dns`, `lb`, `table`, `datagroup`,
   `persist`, `event_ctl`, `log_capture`, `vars`) mirrors the TMM
   protocol stack.  Connection state persists across keep-alive
   requests; per-request state resets.

4. **`unknown` handler dispatch**: iRule commands resolve through the
   Tcl `unknown` handler, matching TMM's C-level command resolver.
   The `_command_map` array maps iRule names to mock procs.

5. **Mock proc naming convention**:
   `NS::sub` -> `::itest::cmd::ns_sub`
   `toplevel` -> `::itest::cmd::cmd_toplevel`
   Hyphens/dots in names become underscores.

6. **Codegen pipeline**: Three scripts generate three Tcl files:
   - `codegen_event_data.py` -> `_event_data.tcl`
   - `codegen_registry_data.py` -> `_registry_data.tcl`
   - `codegen_mock_stubs.py` -> `_mock_stubs.tcl`

7. **Fluent assertion DSL**: `::orch::assert_that subject verb value`
   in Tcl; `result.assert_pool()` / `result.assert_decision()` in
   Python.  Both share pass/fail counters.

8. **Keep-alive lifecycle**: `run_http_request` fires full event
   chain (CLIENT_ACCEPTED -> HTTP_REQUEST -> ...).
   `run_next_request` fires only per-request events.
   `close_connection` fires CLIENT_CLOSED.

9. **Static variables**: `::static::` namespace persists across
   connections.  `RULE_INIT` fires once.  `reset_all` clears them;
   `reset_connection_state` does not.

10. **Two Python backends**: `_InProcessBackend` (tkinter.Tcl, preferred)
    and `_SubprocessBackend` (tclsh + runner.tcl JSON protocol).
    Backend auto-selection: tkinter first, subprocess fallback.

## File-path anchors

### Framework core (Tcl)
- `core/irule_test/tcl/orchestrator.tcl` — event orchestrator, flow chains, assertion DSL
- `core/irule_test/tcl/command_mocks.tcl` — hand-written command mocks (73 procs)
- `core/irule_test/tcl/state_layers.tcl` — 10 protocol state namespaces
- `core/irule_test/tcl/itest_core.tcl` — iRule loader and event firer
- `core/irule_test/tcl/tmm_shim.tcl` — TMM environment simulation
- `core/irule_test/tcl/expr_ops.tcl` — TMM expression operators
- `core/irule_test/tcl/runner.tcl` — JSON protocol for subprocess bridge
- `core/irule_test/tcl/scf_loader.tcl` — SCF/bigip.conf parser
- `core/irule_test/tcl/example_test.tcl` — 4-scenario example test

### Generated data (Tcl)
- `core/irule_test/tcl/_event_data.tcl` — MASTER_ORDER, FLOW_CHAINS
- `core/irule_test/tcl/_registry_data.tcl` — disabled commands, operators, command list
- `core/irule_test/tcl/_mock_stubs.tcl` — 1188 auto-generated stub mocks

### Python bridge
- `core/irule_test/bridge.py` — IruleTestSession, RequestResult, backends
- `core/irule_test/topology.py` — SCF -> Tcl setup generator
- `core/irule_test/codegen_event_data.py` — generates _event_data.tcl
- `core/irule_test/codegen_registry_data.py` — generates _registry_data.tcl
- `core/irule_test/codegen_mock_stubs.py` — generates _mock_stubs.tcl

### AI integration
- `ai/claude/tcl_ai.py` — `cmd_generate_test` CLI subcommand
- `ai/mcp/tcl_mcp_server.py` — `generate_irule_test` MCP tool
- `editors/vscode/src/chat/commands/test.ts` — VS Code `/test` chat command

### Tests
- `tests/test_irule_test_framework.py` — 177 tests (54 require tkinter)

## Example: minimal test (Tcl)

```tcl
::orch::init
::orch::configure -profiles {TCP HTTP}
::orch::add_pool api_pool {10.0.1.1:8080 10.0.1.2:8080}

::orch::load_irule {
    when HTTP_REQUEST {
        if { [HTTP::host] eq "api.example.com" } {
            pool api_pool
        }
    }
}

::orch::run_http_request -host "api.example.com" -uri "/v1/users"
::orch::assert_that pool_selected equals "api_pool"

::orch::run_http_request -host "other.example.com" -uri "/"
::orch::assert_that decision lb pool_select was_not_called

::orch::summary
```

## Example: Python integration

```python
async with IruleTestSession(backend="inprocess") as session:
    await session.start()
    await session.load_irule(irule_source)
    result = await session.run_http_request(host="api.example.com")
    result.assert_pool("api_pool")
    result.assert_no_decision("connection", "reject")
```

## Test runner: structured test cases

The `::orch::test` command provides tcltest-style named test cases:

```tcl
::orch::configure_tests \
    -profiles {TCP HTTP} \
    -irule { when HTTP_REQUEST { pool web_pool } } \
    -setup { ::orch::add_pool web_pool {10.0.0.1:80} }

::orch::test "routing-1.0" "routes to web_pool" -body {
    ::orch::run_http_request -host example.com
    ::orch::assert_that pool_selected equals web_pool
}

exit [::orch::done]
```

Each `test` call automatically resets state, re-inits the framework,
re-loads the iRule, and runs shared setup.  Options: `-body`, `-setup`,
`-cleanup`, `-constraints`.

### Output format

```
==== routing-1.3 Unknown host gets rejected FAILED
---- decision connection reject: was not called

Total	6	Passed	5	Skipped	0	Failed	1
```

The `FAILED:` lines and `Total\tN\tPassed\t...` summary match tcltest
conventions.  Exit code is 0 (all pass) or 1 (failures).

## Editor integration

### VS Code

A `$irule-test` problemMatcher is registered in `package.json`.
Create a task in `.vscode/tasks.json`:

```json
{
    "label": "Run iRule Tests",
    "type": "shell",
    "command": "tclsh ${file}",
    "problemMatcher": "$irule-test",
    "group": "test"
}
```

### Neovim

Use `:make` with `makeprg` and `errorformat`:

```lua
vim.opt_local.makeprg = 'tclsh %'
vim.opt_local.errorformat = '%EFAILED: %m'
```

Or bind to a key:
```lua
vim.keymap.set('n', '<leader>rt', ':!tclsh %<CR>', { desc = 'Run iRule tests' })
```

### Emacs

Use `compilation-mode` with a custom regexp:

```elisp
(add-to-list 'compilation-error-regexp-alist
  '("^FAILED:\\s+\\(.+\\)" nil nil nil nil 1))

(defun run-irule-test ()
  (interactive)
  (compile (concat "tclsh " (buffer-file-name))))
```

### Sublime Text

The `iRule-Test.sublime-build` file is provided.  Use
`Ctrl+B` / `Cmd+B` to run the current test file.

### Helix / Zed

Use the built-in `:sh` command:

```
:sh tclsh %{filename}
```

## Multi-TMM simulation

On real BIG-IP, each TMM core maintains its own copy of `static::`
variables (RULE_INIT fires independently per TMM).  The `table` command
is CMP-shared across all TMMs.  This is a common source of bugs: a
static variable updated on one TMM is stale on others.

Enable multi-TMM mode with `-tmm_count`.  Write the test for the
*desired* behavior — if the iRule has a CMP bug, the test fails:

```tcl
::orch::configure_tests -tmm_count 4 -profiles {TCP HTTP} \
    -irule {
        when RULE_INIT { set static::req_count 0; set static::rate_limit 100 }
        when HTTP_REQUEST {
            incr static::req_count
            if { $static::req_count > $static::rate_limit } { reject }
            pool web_pool
        }
    } \
    -setup { ::orch::add_pool web_pool {10.0.1.1:80} }

# This test FAILS -- proving the bug exists.
# static:: is per-TMM: each TMM only sees 30 of the 120 requests.
::orch::test "rate-1.0" "global rate limit enforced across TMMs" -body {
    set total_rejects 0
    for {set tmm 0} {$tmm < 4} {incr tmm} {
        ::orch::tmm_select $tmm
        for {set i 0} {$i < 30} {incr i} {
            ::orch::run_http_request -host app.example.com
        }
        foreach d [::itest::get_decisions connection] {
            if {[lindex $d 1] eq "reject"} { incr total_rejects }
        }
    }
    # 120 requests, limit 100 → at least 20 should be rejected
    ::orch::assert {$total_rejects >= 20} \
        "total $total_rejects rejected (expected >= 20)"
}
```

Output when the bug is present:

```
==== rate-1.0 global rate limit enforced across TMMs FAILED
---- total 0 rejected (expected >= 20)
```

**Fix**: Use `table` (CMP-shared) instead of `static::` for cross-TMM
counters.  Re-run and the test passes:

```tcl
# table incr is CMP-shared -- works correctly across all TMMs
set count [table incr -subtable rate_limits [IP::client_addr]]
if { $count > $static::rate_limit } { reject }
```

See `example_multi_tmm_test.tcl` for the complete bug/fix pair with
passing and failing tests side by side.

### Scoping model

| Scope | Per-TMM? | Reset | Real BIG-IP equivalent |
|-------|----------|-------|----------------------|
| `static::` | Yes | RULE_INIT per TMM | Per-TMM memory |
| `table` | No (CMP shared) | `reset_all` | Shared session DB |
| `data groups` | No (shared config) | `reset_all` | Config partition |
| `connection` | Per-TMM (one conn per TMM select) | `tmm_select` | CMP connection affinity |

### API

- `::orch::tmm_select N` — switch to TMM N (fires RULE_INIT on first use)
- `::orch::tmm_get_static N varname` — read a static var from TMM N
- `::orch::tmm_ids` — list all TMM indices
- `::orch::tmm_current` — current TMM index
- `::orch::assert_that tmm_var N varname verb expected` — fluent assertion
- `-tmm_select auto` — **fakeCMP** auto-select mode (see below)

### fakeCMP: simulated CMP hash

With `-tmm_select auto`, the framework uses **fakeCMP** — a deterministic
simulated hash (NOT the real BIG-IP CMP algorithm) — to pick which TMM
handles each connection based on `(src_ip, src_port, dst_ip, dst_port)`.
Same tuple always lands on the same TMM.

**Planning tools** — figure out the distribution before writing the test:

| Tool | Purpose |
|------|---------|
| `::orch::fakecmp_which_tmm addr port dst_addr dst_port` | Look up which TMM a specific tuple maps to |
| `::orch::fakecmp_which_tmm` (no args) | Uses current `client_addr`/`client_port` config |
| `::orch::fakecmp_suggest_sources -count N` | Find N client addr/port combos per TMM |
| `::orch::fakecmp_plan -count N` | Pretty-print distribution plan |

Example using `fakecmp_suggest_sources` to guarantee all TMMs get traffic:

```tcl
::orch::configure_tests -tmm_count 4 -tmm_select auto \
    -profiles {TCP HTTP} -irule { ... }

# Get 2 source tuples per TMM from fakeCMP
set plan [::orch::fakecmp_suggest_sources -count 2]

# Send traffic using the planned sources
foreach tmm_id [::orch::tmm_ids] {
    set sources [dict get $plan $tmm_id]
    foreach {addr port} $sources {
        ::orch::configure -client_addr $addr -client_port $port
        ::orch::run_http_request -host app.example.com
    }
}
```

Example verifying a prediction:

```tcl
# Predict, then verify
set predicted [::orch::fakecmp_which_tmm 10.0.0.42 54321 192.168.1.100 443]
::orch::configure -client_addr 10.0.0.42 -client_port 54321
::orch::run_http_request -host app.example.com
::orch::assert_equal $predicted [::orch::tmm_current]
```

**MCP tools** for AI-assisted test generation: `fakecmp_which_tmm` and
`fakecmp_suggest_sources` are also exposed as MCP tools so that AI agents
can plan multi-TMM test distributions before generating Tcl code.

## Troubleshooting

| Problem | Cause | Fix |
|---------|-------|-----|
| `Missing generated file` error | Codegen not run | `python -m core.irule_test.codegen_event_data` (or `_registry_data` / `_mock_stubs`) |
| `_mock_stubs.tcl is stale` test failure | New commands added to registry | Regenerate with `python -m core.irule_test.codegen_mock_stubs` |
| `tkinter.Tcl() not available` | Python built without `_tkinter` | Use `backend="subprocess"` or install tkinter |
| iRule command returns empty string | Using stub mock | Write a hand-written mock in `command_mocks.tcl` |
| Event not firing | Profile not configured | Check `::orch::configure -profiles {...}` |
