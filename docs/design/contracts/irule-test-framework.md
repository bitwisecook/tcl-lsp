# iRule Event Orchestrator test framework

How iRule logic — pool selection, header manipulation, data group lookups,
persistence, logging, and connection control — is exercised without a BIG-IP
device. Read it before changing the simulation, its mocks, or the Rust harness
that drives them.

The simulation itself is Tcl: an orchestrator, a TMM shim, protocol state
layers, and command mocks, shipped under `rust/tcl-irule-test/tcl/`. A test
script is plain Tcl. Persistent connection activations require a host with
coroutine support (Tcl 8.6+ or a measured equivalent); earlier hosts report
that capability as unsupported.

`rust/tcl-irule-test` is the Rust side: it embeds those Tcl assets, stands the
orchestrator up **in-process on the bytecode VM** (no `tclsh` subprocess),
loads an iRule, fires events, and reads back the pool/node decisions, captured
logs, and assertion results.

## Architecture

```
test script (Tcl)  |  LiveSession (Rust, drives tcl-vm in-process)
    -> orchestrator.tcl  (event ordering, flow chains, assertions)
    -> itest_core.tcl    (iRule loader, event firer)
    -> command_mocks.tcl (hand-written mocks)
    -> _mock_stubs.tcl   (generic stub action table for registry-only commands)
    -> state_layers.tcl  (protocol state namespaces)
    -> tmm_shim.tcl      (disabled commands, info override)
    -> expr_ops.tcl      (contains/starts_with/ends_with/…)
    -> compat84.tcl      (8.4 compatibility shims)
    -> profiler.tcl      (optional, off by default)
```

The Rust modules are:

| Module | Role |
|---|---|
| `embedded.rs` | the Tcl assets, materialised so `[file dirname [info script]]` sibling lookups resolve |
| `live.rs` | `LiveSession` — bootstrap, compile, fire, read back; sources `FRAMEWORK_FILES` in the same order `runner.tcl` does |
| `session.rs` | `SessionPlan` — assembles the bootstrap script |
| `sim.rs` | `simulate_irule` — the config-agnostic entry point behind `f5-query explain-flow --simulate` |
| `topology.rs` | turn a parsed BIG-IP config into the `::orch::` setup commands for a virtual server |

## Decision rules / contracts

1. **Registry-backed generated data has a native owner.**
   `_event_data.tcl` is generated from `EventRegistry` and `_mock_stubs.tcl`
   from the resolved F5 iRules profile registry plus the hand-written-mock boundary
   in `command_mocks.tcl`. `_user_surface_data.tcl` projects modern core
   command availability, including individually callable qualified ensemble
   members, from the same registry. They must never be edited by hand: `cargo xtask
   in `command_mocks.tcl`, and only for a command whose declared
   `runtime_backing` is `None` or `HostNative`: a shipped builtin or a Tcl
   body is supplied by the interpreter, so it needs no mock and the table
   holds none. They must never be edited by hand: `cargo xtask
   gen-irule-test-data` regenerates them and `make xtask-check` detects drift.
   `_mock_stubs.tcl` is a single data table consumed by the generic
   `::itest::cmd::_stub` proc, which records the shared decision-log triples.
   `_registry_data.tcl` is
   a hand-maintained fixture outside this generated-asset contract.

2. **Decision log over state inspection**: Tests assert on the
   decision log (`{category action args}` triples) rather than raw
   state.  This makes tests more readable and less brittle.

3. **State layers mirror TMM**: The `::state::` namespace hierarchy
   (`connection`, `tls`, `http`, `dns`, `lb`, `table`, `datagroup`,
   `persist`, `event_ctl`, `log_capture`, `vars`) mirrors the TMM
   protocol stack.  Connection state persists across keep-alive
   requests; per-request state resets.

4. **`unknown` handler dispatch**: Commands unavailable as installed host
   commands route through the simulation's Tcl `unknown` handler.
   The `_command_map` array maps iRule names to mock procs. This authored
   dispatch establishes no native TMM command-resolver or compiler protocol.

5. **Mock proc naming convention**:
   `NS::sub` -> `::itest::cmd::ns_sub`
   `toplevel` -> `::itest::cmd::cmd_toplevel`
   Hyphens/dots in names become underscores.

6. **Fluent assertion DSL**: `::orch::assert_that subject verb value`.
   Assertions share one pass/fail counter set, which `::orch::summary` and
   `::orch::done` read.

7. **Keep-alive lifecycle**: `run_http_request` fires full event
   chain (CLIENT_ACCEPTED -> HTTP_REQUEST -> ...).
   `run_next_request` fires only per-request events.
   `close_connection` fires CLIENT_CLOSED.

8. **Cell and worker ownership**: a real retained coroutine activation owns
   connection locals across events and keep-alive requests. Resetting a
   connection retires that activation. `RULE_INIT` executes in the root
   namespace frame. Each CMP worker owns a real child interpreter, including
   `::static::`, global cells, aliases, arrays, traces and user command tables.
   Worker selection never snapshots user values. The declarations are published
   to every worker. Creating a rule executes that rule's `RULE_INIT` handlers
   across the configured worker roster before loading returns. Rule recreation
   executes only the recreated owner's initialisers. Worker selection does not
   rerun initialisation. Static names share each worker's namespace across rule
   owners, so the last initialisation store wins. Full orchestrator
   reset retires the multi-worker interpreters and clears their initialisation
   markers. Reusing a worker index creates a new interpreter lifetime. The mock
   `table` and data groups share parent-orchestrator
   state through command bridges. Legacy global CMP demotion itself remains a
   separately documented platform behaviour, not a simulator scheduling proof.

   The rule loader executes Tcl declarations through `when`, so top-level
   procedure declarations and runtime-defined procedures belong to the proper
   interpreter. The framework's host command surface provides private coroutine
   and interpreter control; user event code retains the F5 grammar and command
   availability surface.

   `RuleIdentity` supplies an explicit absolute configuration path; `RuleSource`
   distinguishes attached rules from unattached procedure libraries.
   `LiveSession::load_rule`, `SessionPlan::with_rules`, and `simulate_rules`
   preserve this identity. Existing `load_irule`/`simulate_irule` convenience
   inputs remain unnamed. The loader never invents a rule owner from source.
   Named rule procedures have private command-table owners in every worker.
   Their actual execution namespace is `::`, independently of the rule path.
   `call` selects a local procedure, a partition-root `rule::proc`, or an
   absolute `/Partition/folder/rule::proc`; nested calls enter the target owner.
   A relative rule name does not search the caller's folder. Public F5 call
   targets do not become Tcl namespace commands, so `namespace which -command`
   does not expose them. Dispatch uses the real caller activation, so `upvar 1`
   reaches its cells. Topology inputs load unattached rules as libraries without
   registering their events and refuse ambiguous short-name object resolution.

   These contracts match the [recorded BIG-IP 21.1.0.1 build 0.0.26 controls](../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md).
   `Vm::install_irules_rule_callable_simulation` is an explicit authored
   capability. Physical Tcl procedure declarations, bodies, command epochs and
   entered activations retain their real host owners; no F5 CPP or ABI is inferred.
   Named procedure declarations and calls on an external host require an
   equivalent capability. Anonymous execution and named event-only rules remain
   available without this rule-owner provider.

   `configure_static` is an explicit test configuration broadcast. It stores
   into each worker's existing `::static` cell and therefore runs that cell's
   write traces. Deleting and recreating a cell retires its old trace
   registrations; a broadcast does not restore them or replace ordinary globals.
   A guest event error retains writes already performed and the connection's
   local cells, while the framework restores its selected rule/execution state.
   Event results retain the handler's error instead of rolling back its stores.

   The default single-worker mode executes in the orchestrator interpreter;
   separate child-interpreter isolation is enabled by `-tmm_count` greater than
   one. The simulator manages one active connection at a time. Selecting a worker
   starts a new connection and retires the previously selected connection frame
   when that frame is next reset. Explicit `fire_event RULE_INIT` can fire a
   handler again. Automatic initialisation belongs to rule creation, including
   library rules without attached events. Inspect handler results when testing
   failed initialisation.

   `LiveSession` explicitly installs an authored timer simulator. Scripted
   `after milliseconds ?-periodic? script` callbacks retain their original
   interpreter and connection frame, or the interpreter root for `RULE_INIT`.
   `LiveSession::advance_time` advances a deterministic logical clock and returns
   each callback's full guest completion or a separate host/activation refusal.
   This is a simulator contract, not a measured F5 hardware timer protocol.
   Ordinary VMs retain Tcl's existing global `after` event loop.

   Timers are ordered by deadline and registration identity. A callback may
   register or cancel timers, including `after cancel -current`; newly registered
   callbacks wait for the next advance. Missed periodic intervals coalesce into
   one dispatch per advance. Periodic delays must be positive. IDs are never
   reused, and cancellation is confined to the owning worker interpreter.
   Deleted workers, replaced rules, reset interpreter epochs and destroyed
   connection frames invalidate their callbacks. A reused frame level or command
   name does not revive an old callback. `after info` lists the worker's queued
   IDs or returns one ID's script and timer kind. Bare delays require a resumable
   event continuation and produce an explicit host refusal; they do not block a
   host thread or fabricate a resumed event. Standalone Tcl framework mocks still
   record scripts without supplying this retained-activation provider.

   Callback execution preserves the original rule/event metadata for helper
   dispatch and restores framework bookkeeping through its retained cells. An
   error retains user writes already performed. Complete guest return options
   remain available in the callback report; host refusals stop the drain before
   later callbacks execute. Synchronous trace callbacks and `call` helpers use
   their actual interpreter cells and caller frames.

   Private framework builtin capabilities retain the original command identity
   and the host dialect explicitly. Framework completion capture can therefore
   use host `catch`/`return` options while user event commands retain the selected
   TMM grammar. An alias spelling alone never grants this capability.

   `LiveSession` installs the authored Tcl 8.4 parser, numeric model and separate
   core math-function provider for user expressions. The orchestrator uses a
   explicitly installed Tcl 9.0 native core. Host activation restores the user expression
   policy on return; a logical dialect label alone does not install native
   commands or compiler registrations.

   Every worker interpreter owns its authored random stream and lazy double
   formatter. Their initial seed of one and precision of twelve are reproducible
   simulator choices. Precision changes affect doubles without an existing
   String representation; an already rendered value retains its String.
   Recreating a worker creates fresh state. These initialisation choices are
   independent of BIG-IP measurements.

   Native `if`, `while`, `for`, expression command substitutions, quoted
   expression strings, math-function calls and array-index substitutions retain
   continuation state on the VM activation stack. They preserve actual Tcl
   frames across coroutine suspension; expression arithmetic and traversal share
   the `tcl-syntax` evaluator with synchronous consumers. Read-trace callbacks
   remain synchronous, matching measured C Tcl rejection of yielding through
   its busy C callback stack.

9. **In-process execution.** `LiveSession` compiles and runs the
   orchestrator on `tcl-vm` directly, so a caller needs no external
   interpreter. Every VM or orchestrator failure is captured into the
   outcome rather than panicking, so a caller (for example
   `f5-query explain-flow --simulate`) can still render its static analysis
   when the simulation cannot complete.

10. **Byte-accurate `*::payload`**: payloads are wire bytes, so the
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
- `rust/tcl-irule-test/tcl/orchestrator.tcl` — event orchestrator, flow chains, assertion DSL, `_fakecmp_hash`
- `rust/tcl-irule-test/tcl/command_mocks.tcl` — hand-written command mocks
- `rust/tcl-irule-test/tcl/state_layers.tcl` — protocol state namespaces
- `rust/tcl-irule-test/tcl/itest_core.tcl` — iRule loader and event firer
- `rust/tcl-irule-test/tcl/tmm_shim.tcl` — TMM environment simulation
- `rust/tcl-irule-test/tcl/expr_ops.tcl` — TMM expression operators
- `rust/tcl-irule-test/tcl/compat84.tcl` — 8.4 compatibility shims
- `rust/tcl-irule-test/tcl/runner.tcl` — the standalone runner's source order
- `rust/tcl-irule-test/tcl/scf_loader.tcl` — SCF / `bigip.conf` parser
- `rust/tcl-irule-test/tcl/example_test.tcl`, `example_multi_tmm_test.tcl`, `binary_payload_test.tcl` — worked examples

### Generated data (Tcl)
- `rust/tcl-irule-test/tcl/_event_data.tcl` — `MASTER_ORDER`, `FLOW_CHAINS`
- `rust/tcl-irule-test/tcl/_registry_data.tcl` — disabled commands, operators, command list
- `rust/tcl-irule-test/tcl/_mock_stubs.tcl` — the generic stub action table

### Rust driver
- `rust/tcl-irule-test/src/{embedded,live,session,sim,topology}.rs`

### AI and editor integration
- `rust/tcl-mcp/src/irule_gen.rs` — the `generate_irule_test` MCP tool
- `rust/tcl-mcp/src/fakecmp.rs` — `fakecmp_which_tmm` / `fakecmp_suggest_sources`, kept byte-for-byte in sync with the Tcl `_fakecmp_hash` (agreement is tested)
- `editors/vscode/src/chat/commands/test.ts` — the VS Code `/test` chat command
- The `generate-test` skill drives test generation from an iRule.

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

Add a build system (`Tools → Build System → New Build System…`):

```json
{
    "cmd": ["tclsh", "$file"],
    "file_regex": "^FAILED: (.*)$",
    "selector": "source.tcl"
}
```

### Helix / Zed

Use the built-in `:sh` command:

```
:sh tclsh %{filename}
```

## Multi-TMM simulation

The simulator gives each worker its own `static::` cells. Rule creation runs
that rule's `RULE_INIT` across the configured roster. Names in `::static` share
storage across rule owners within a worker, and creation order determines the
last writer. Removing a rule configuration removes its event registration but
retains activated procedures. Recreating a rule publishes its declarations and
runs only its initialisers. A full framework reset retires all callable owners. An event write, unset or
recreation affects the
executing worker; `configure_static` explicitly broadcasts a configuration store.
Worker selection does not initialise rules. The mock `table` shares state across
workers.

This authored lifecycle matches the measured BIG-IP 21.1.0.1 build 0.0.26
single-group, four-TMM controls. The measurement does not establish behaviour
for other builds, blades or worker groups. Ordinary-global CMP demotion is
measured platform behaviour; the simulator does not schedule or demote flows
from that observation. The
[BIG-IP resolution probes](../../../scripts/dev/bigip-probes/resolution-2286/README.md)
record actual worker coverage rather than inferring it from request counts.

`LiveSession` installs a separate authored Tcl84 package capability. Logical
package selectors, abbreviations, loaders, unknown handlers and mutations use
an isolated table per interpreter. Host initialization selects the physical
package table and restores the logical dispatch policy afterwards. The logical
table starts with `Tcl 8.4`; it grants no appliance ABI, native package header or
compiler authority. External Tcl hosts require an explicit equivalent provider.

Build-specific original loader checks are opt-in. The measured profile rejects
literal source NUL. Exact complete source images in the measured Unicode corpus
retain their observed ACCEPT/REJECT outcomes; all other non-ASCII source is
unsupported by this loader profile. Three emoji literal probes were accepted,
while the ten other probes were rejected. These observations establish no
general Unicode acceptance rule, identifier normalization or hash-key identity.
ASCII LF and CRLF source remain accepted. Runtime-generated NUL values and names
remain separate from the original loader policy. Dynamic `A\0B` success does not
prove a distinct counted-name key from `A`. The separately installed
`Vm::install_irules_counted_string_simulation` counts original encoded string
bytes and pure bytearray bytes in user event code. It grants no native StringLength
CPP, Unicode indexing or physical object-header authority; host string operations
retain their actual protocol. Literal compiler-refused
commands use the shared load-phase diagnostics; dynamic evaluation sees the
measured runtime surface. Namespace selectors use the Tcl84 member roster,
including prefix ambiguity, and exclude `namespace path`.

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
| `static::` | Yes; shared across rule owners | Rule creation/recreation initialisation | Per-TMM memory |
| `table` | No (CMP shared) | `reset_all` | Shared session DB |
| `data groups` | No (shared config) | `reset_all` | Config partition |
| `connection` | Per-TMM (one conn per TMM select) | `tmm_select` | CMP connection affinity |

### API

- `::orch::tmm_select N` — switch to TMM N without rerunning rule initialisers
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
| `Missing generated file` error | A generated data file is absent | Restore `_event_data.tcl` / `_mock_stubs.tcl` / `_user_surface_data.tcl` from the tree — they are checked in |
| iRule command returns an empty string | It is being served by the generic stub | Write a hand-written mock in `command_mocks.tcl` |
| Event not firing | Profile not configured | Check `::orch::configure -profiles {...}` |
