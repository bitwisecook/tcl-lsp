# KCS: iRule Event Orchestrator test framework

## Symptom

Need to test iRule logic (pool selection, header manipulation, data
group lookups, persistence, logging, connection control) without a
BIG-IP device.

## Operational context

The `tcl-irule-test` crate provides a complete TMM simulation: event
orchestration, protocol state machines, command mocks, and an assertion
DSL.  The simulation itself is the body of Tcl under
`rust/tcl-irule-test/tcl/`; the crate's Rust `src/` is the driver that
stands it up on the bytecode VM, plus a BIG-IP topology generator.

A test script therefore has two ways to run.  Written as Tcl, it runs
under plain `tclsh` with no other tooling.  Driven from Rust, a
`LiveSession` compiles and runs the same framework on `tcl-vm`
in-process — no `tclsh` subprocess, no external interpreter.

## Architecture

```
User test script (Tcl)          Rust driver (tcl-irule-test/src)
    |                               |
    |                               +-> session.rs   (bootstrap script)
    |                               +-> live.rs      (LiveSession on tcl-vm)
    |                               +-> embedded.rs  (framework bundled via include_str!)
    |                               +-> sim.rs       (simulate_irule facade)
    |                               +-> topology.rs  (bigip.conf -> ::orch:: setup)
    v                               v
    -> orchestrator.tcl  (event ordering, flow chains, assertions)
    -> itest_core.tcl    (iRule loader, event firer)
    -> command_mocks.tcl (~97 hand-written mocks + a 1473-entry stub table)
    -> state_layers.tcl  (the ::state:: protocol namespaces)
    -> tmm_shim.tcl      (disabled commands, info override)
    -> expr_ops.tcl      (contains/starts_with/ends_with/etc.)
    -> compat84.tcl      (8.4-compatible shims for the framework itself)
    -> profiler.tcl      (optional, off by default: emits rule-profiler
                          occurrence logs for profile-guided optimisation)
```

The framework files are also embedded in the binary with `include_str!`
and materialised to a temporary directory on demand, so a consumer such
as `f5 explain-flow --simulate` does not need the source tree beside it.

## Decision rules / contracts

1. **The registry is the intended source of truth — with a live gap.**
   The three Tcl data files (`_event_data.tcl`, `_registry_data.tcl`,
   `_mock_stubs.tcl`) were generated from the old Python registry.
   Those generators were retired with the Python tree and have **no
   Rust replacement**: there is currently no regeneration step and no
   staleness gate.  The files are checked in, and `_registry_data.tcl`
   carries hand-applied corrections (the bogus
   `tcl::mathop::{&&,||,@}` entries removed and the real 9.0+ TIP 461
   `tcl::mathop::{lt,le,gt,ge}` operators added, issue #984); the rest
   of its data has not been re-audited against `tcl-registry`.  Treat
   these files as data that *should* be derived from the registry and
   is not yet — do not hand-edit them casually, and prefer fixing the
   generation gap to patching them again.

2. **Decision log over state inspection**: Tests assert on the
   decision log (`{category action args}` triples) rather than raw
   state.  This makes tests more readable and less brittle.

3. **State layers mirror TMM**: The `::state::` namespace hierarchy
   (`connection`, `tls` — with `client` / `server` children — `http`,
   `http2`, `dns`, `lb`, `table`, `datagroup`, `persist`, `vars`,
   `event_ctl`, and `log_capture`) mirrors the TMM protocol stack.
   Connection state persists across keep-alive requests; per-request
   state resets.  `::static::` sits alongside it, outside `::state::`.

4. **`unknown` handler dispatch**: iRule commands resolve through the
   Tcl `unknown` handler, matching TMM's C-level command resolver.
   The `_command_map` array maps iRule names to mock procs.

5. **Mock proc naming convention**:
   `NS::sub` -> `::itest::cmd::ns_sub`
   `toplevel` -> `::itest::cmd::cmd_toplevel`
   Hyphens/dots in names become underscores.

6. **One generic stub proc, not 1473 procs**: `_mock_stubs.tcl` is a
   single `_stub_actions` data table mapping each registry-only iRule
   command to its `{category action}` pair.  `::itest::cmd::register_all`
   wires them on demand through one generic `::itest::cmd::_stub` proc.
   The earlier shape — one generated proc per command — cost about
   9.7 s of VM proc-body compilation on every fresh session for
   identical behaviour.  Hand-written mocks in `command_mocks.tcl` are
   probed with the same name and take precedence.

7. **Fluent assertion DSL**: `::orch::assert_that subject verb value`,
   with `::orch::assert` and `::orch::assert_equal` underneath it.  All
   share one pair of pass/fail counters, which `::orch::summary` and
   `::orch::done` report.

8. **Keep-alive lifecycle**: `run_http_request` fires full event
   chain (CLIENT_ACCEPTED -> HTTP_REQUEST -> ...).
   `run_next_request` fires only per-request events.
   `close_connection` fires CLIENT_CLOSED.

9. **Static variables**: `::static::` namespace persists across
   connections.  `RULE_INIT` fires once.  `reset_all` clears them;
   `reset_connection_state` does not.

10. **The Rust driver runs in-process on `tcl-vm`**: `LiveSession`
    sources the framework files in `runner.tcl`'s order, compiles them
    through the VM's compile service, and then exposes a thin Rust API
    over the `::orch::` surface (`load_irule`, `run_http_request`,
    `fire_event`, `fire_sequence`, `add_datagroup`, `pool_selected`,
    `decisions`, `logs`).  `LiveSession::embedded()` runs against the
    bundled copy; `LiveSession::new(dir)` against a checkout.  There is
    no subprocess and no external interpreter.  Every failure — VM
    bootstrap, compile, or orchestrator error — is returned as a
    `SessionError` and, at the `simulate_irule` layer, captured into
    `SimOutcome::error` so a caller can still render its static
    analysis.  `runner.tcl`'s line-oriented JSON protocol survives as an
    out-of-process entry point; it was written for the retired Python
    bridge and nothing in-tree drives it today.

11. **Byte-accurate `*::payload`**: payloads are wire bytes, so the
    `*::payload` mocks treat them as byte arrays — `length`, the
    `<size>` getter, and `replace` are BYTE operations (offsets and
    lengths are byte counts, matching TMM), and `replace` re-wraps the
    spliced result as a byte array so surrounding bytes `>= 0x80` are
    not re-encoded.  The helpers (`_payload_bytes` / `_payload_bytelength`
    / `_payload_first` / `_payload_splice` in `command_mocks.tcl`) force
    a byte-array intrep via `binary format a*`.  This is the runtime
    counterpart to the static `S110` byte-array-corruption diagnostic
    (see [`../compiler/byte-array-corruption.md`](../compiler/byte-array-corruption.md));
    the orchestrator suite `binary_payload_test.tcl` exercises it.
    Limitation: the mock converts a non-byte-array data argument via
    latin-1 (`binary format a*`), so it does not reproduce TMM's UTF-8
    *re-encoding* of a multibyte character string — the S110 diagnostic
    covers that pattern at author time instead.

## File-path anchors

### Framework core (Tcl)
- `rust/tcl-irule-test/tcl/orchestrator.tcl` — event orchestrator, flow chains, assertion DSL, multi-TMM and fakeCMP
- `rust/tcl-irule-test/tcl/command_mocks.tcl` — hand-written command mocks (~97 procs) and `register_all`
- `rust/tcl-irule-test/tcl/state_layers.tcl` — the `::state::` protocol namespaces
- `rust/tcl-irule-test/tcl/itest_core.tcl` — iRule loader and event firer
- `rust/tcl-irule-test/tcl/tmm_shim.tcl` — TMM environment simulation
- `rust/tcl-irule-test/tcl/expr_ops.tcl` — TMM expression operators
- `rust/tcl-irule-test/tcl/compat84.tcl` — 8.4-compatible shims the framework itself uses
- `rust/tcl-irule-test/tcl/profiler.tcl` — optional rule-profiler occurrence emitter (off by default)
- `rust/tcl-irule-test/tcl/runner.tcl` — line-oriented JSON protocol for an out-of-process driver
- `rust/tcl-irule-test/tcl/scf_loader.tcl` — SCF/bigip.conf parser
- `rust/tcl-irule-test/tcl/example_test.tcl` — example test
- `rust/tcl-irule-test/tcl/example_multi_tmm_test.tcl` — multi-TMM bug/fix pair
- `rust/tcl-irule-test/tcl/binary_payload_test.tcl` — byte-accurate payload suite

### Generated data (Tcl)
- `rust/tcl-irule-test/tcl/_event_data.tcl` — `MASTER_ORDER`, `FLOW_CHAINS`
- `rust/tcl-irule-test/tcl/_registry_data.tcl` — disabled commands, operators, command list
- `rust/tcl-irule-test/tcl/_mock_stubs.tcl` — the `_stub_actions` table (1473 entries)

See contract 1: these three files have no regeneration step today.

### Rust driver
- `rust/tcl-irule-test/src/session.rs` — `SessionPlan`, the bootstrap script
- `rust/tcl-irule-test/src/live.rs` — `LiveSession`, `SessionError`, the `::orch::` API
- `rust/tcl-irule-test/src/embedded.rs` — `EmbeddedLib`, framework files bundled with `include_str!`
- `rust/tcl-irule-test/src/sim.rs` — `simulate_irule`, `SimRequest`, `SimOutcome`
- `rust/tcl-irule-test/src/topology.rs` — `Topology`, bigip.conf → `::orch::` setup script

### AI and editor integration
- `rust/tcl-mcp/src/irule_gen.rs` — the `generate_irule_test` MCP tool
- `rust/tcl-mcp/src/fakecmp.rs` — the `fakecmp_which_tmm` / `fakecmp_suggest_sources` MCP tools
- `.claude/skills/generate-test/` — the Claude Code test-generation skill
- `editors/vscode/src/chat/commands/test.ts` — VS Code `/test` chat command

### Tests
- Unit tests live beside the driver in each `rust/tcl-irule-test/src/*.rs` module
- `rust/f5-cli/tests/explain_flow.rs` — the `f5 explain-flow --simulate` path that drives the embedded framework
- The Tcl suites under `rust/tcl-irule-test/tcl/` (`example_test.tcl`, `example_multi_tmm_test.tcl`, `binary_payload_test.tcl`) run directly under `tclsh`

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

## Example: driving it from Rust

```rust
use tcl_irule_test::LiveSession;

let mut session = LiveSession::embedded()?;
session.eval("::orch::init")?;
session.eval("::orch::configure -profiles {TCP HTTP}")?;
session.eval("::orch::add_pool api_pool {10.0.1.1:8080}")?;
session.load_irule(irule_source)?;
session.run_http_request("-host api.example.com -uri /v1/users")?;
assert_eq!(session.pool_selected()?, "api_pool");
# Ok::<(), tcl_irule_test::SessionError>(())
```

For a whole config rather than a hand-written setup, `Topology` turns a
`bigip.conf` into the `::orch::` setup script for one virtual server, and
`simulate_irule` wraps the entire "stand up a session, load, fire, read
back" cycle into one call returning a best-effort `SimOutcome` — that is
what `f5 explain-flow --simulate` uses.

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
*desired* behaviour — if the iRule has a CMP bug, the test fails:

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
| A new registry command is unknown to the framework | The generated data files no longer track the registry (contract 1) | Add its `_stub_actions` entry by hand, or a real mock in `command_mocks.tcl` |
| `iRule-test library not found` (`SessionError::MissingLib`) | `LiveSession::new` pointed at a directory without the framework files | Use `LiveSession::embedded()`, or point at `rust/tcl-irule-test/tcl/` |
| `SimOutcome::error` names a missing command | The VM does not yet implement a command the orchestrator needs | Expected while the VM's command surface grows; the static analysis still renders |
| iRule command returns empty string | It resolved to the generic stub mock | Write a hand-written mock in `command_mocks.tcl` |
| Event not firing | Profile not configured | Check `::orch::configure -profiles {...}` |
