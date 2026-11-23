# Threads and the newer subsystems

How the `Thread` package (`thread::*`, `tsv::*`), `tcl::process`, `zipfs`, `tcl::build-info`, `info patchlevel` and the `tcl::unsupported::*` commands are executed by the two engines and described to analysis. Eleven fixtures in [`oo-dispatch-probe/threads-subsystems/`](handover/oo-dispatch-probe/threads-subsystems/) were run under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, Jim, the VM (`tclvm --tcl-version 8.6|9.0`) and the Rust runtime (`run_script --quiet --tcl-version 8.6|9.0`) on `afe2f2d8` (`engines.tsv`), and through the analyser's MCP tools, a cached build that may lag that commit (`analyser-mcp.out`).

**Thread availability.** None of the five tclsh builds on `PATH` ships Thread (`package require Thread` fails on every release; Jim has no Thread either). Thread 3.0.4 for Tcl 9 is bundled with the Google Cloud SDK at `/opt/google-cloud-sdk/platform/bundledpythonunix/lib/thread3.0.4`; with that directory on `auto_path` it loads into tclsh 9.0.4 and 9.1.0 (both are non-threaded builds, `tcl::build-info threaded` is `0`, yet Thread works). It does not load into 8.6 (it is a Tcl 9 binary), and 8.4 cannot even read its `pkgIndex.tcl`. So Thread rows have a 9.0/9.1 oracle only.

**Verdict:** the VM has a real shared-nothing Thread implementation, but it is installed as built-in commands rather than as the `Thread` package, so the same script behaves differently depending on whether it guards with `info commands` or `package require`, and one control-flow bug leaks a caught error. The runtime has no threads. Neither engine has `tcl::process`, `zipfs` or the bytecode `tcl::unsupported` commands; `tcl::build-info` is shared and correct. Analysis knows `zipfs`, `tcl::process` and `tcl::build-info` from the registry with correct release gates, knows nothing about `thread::*` or `tsv::*`, and stays silent on commands that do not exist in the selected release (`thread::id` without the package, `tcl::unsupported::timerate` in 9.0).

## Measured

| Fixture | tclsh 9.0/9.1 (8.x where relevant) | VM 8.6 and 9.0 | Runtime 8.6 and 9.0 |
| --- | --- | --- | --- |
| `t01`–`t03` `package require Thread` (SDK path) | loads; `send`, `send -async`, `tsv::*` work | `package require` runs the SDK `pkgIndex.tcl`, which needs `load`: `invalid command name "load"` (20–54 s) | `can't find package Thread` |
| `t09` no `package require` | `thread::create` absent | present, `thread::id` returns `1` | absent |
| `t10` built-in thread commands | (package) `send 42`, isolation errors, `async 2`, `tsv` shared, `errorcode {MY CODE}` | `package provide Thread` is empty; isolation right; **then a caught error resurfaces** | no threads |
| `t11` `catch {thread::send $t {no-such-cmd}}`, then `send -async … ::res; vwait ::res` | `caught=1`, `next-sync=5`, `async=2` | `caught=1`, `next-sync=5`, then the caught `invalid command name "no-such-cmd"` is raised from `vwait`, exit 1 | — |
| `t02`/`t10` isolation: the creator's globals and procs in a sent script | `can't read "shared"`, `invalid command name "local"` | same | — |
| `t04` `tcl::process list/status/purge/autopurge` with `exec … &` | works on 9.0, 9.1; absent before | absent | absent |
| `t05` `zipfs mkzip/mount/list/info/canonical`, read through `//zipfs:/` | works on 9.0, 9.1 | absent | absent |
| `t06` `tcl::build-info` | `version 9.0`, `patchlevel 9.0.4`, `commit git-…`, `compiler gcc-1303`; absent before 9.0 | present (9.0 pin only) with an all-zero commit and no `git-` prefix | same as VM |
| `t06` `info exists tcl_platform(threaded)` | `1` on 8.6 (threaded build); `0` on these 9.0/9.1 (non-threaded builds) | `1` on both pins; `build-info threaded` says `0` | `1`; `build-info threaded` `0` |
| `t07` `info commands ::tcl::unsupported::*` | 8.6: `assemble corotype disassemble getbytecode inject representation timerate`; 9.0: `assemble corotype disassemble getbytecode loadIcu representation`; 9.1 adds `grapheme` | `corotype representation` | `corotype representation` |
| `t08` `timerate` | 8.6: only `tcl::unsupported::timerate`; 9.0+: only `::timerate` | neither | `::timerate` on both pins (8.6 should not have it) and no `tcl::unsupported::timerate` |

The debug VM also takes 7–11 s to start any of these fixtures and about 10 s for each unknown command (the `unknown`/`auto_load` path), which turns otherwise fast fixtures into timeouts at 20 s.

**Analysis** (`analyser-mcp.out`):

| Case | Answer |
| --- | --- |
| `zipfs`, `tcl::process`, `tcl::build-info` under 8.6 | W002 "available in Tcl 9.0, 9.1" (right); `zipfs` also gets W123, the other two do not |
| the same under 9.0 | nothing (right) |
| `thread::id`, `tsv::get` under 9.0 with no `package require Thread` | nothing (tclsh: invalid command) |
| `tcl::unsupported::timerate` under 9.0 | nothing (tclsh 9.0: invalid command) |
| `thread::create`, `tcl::unsupported::timerate` hover under 9.0 | `null`; `zipfs mount` hover is "Subcommand of `zipfs`" only |
| `open $mp/hello.txt` with `mp` under `//zipfs:/` | W103 dynamic-path warning |
| a script sent with `thread::send` | treated as an opaque string: no body unit, no W-codes inside, no navigation |
| `package require Thread` | accepted with no W120 (no Thread roster exists to check against) |

## Census

- **Threads.**
  - VM: `rust/tcl-vm/src/cmd_thread.rs` (1,575 lines) implements `thread::create/send/wait/release/id/exists/names/errorproc` and `tsv::set/get/exists/unset/incr/append/lappend/keys/names` with a worker `Vm` per OS thread (`:19-40`); `Vm::enable_threads` (`:179`) installs them and sets `tcl_platform(threaded)` to `1` (`:199`, `:312`). The CLI always enables them (`rust/tcl-vm-cli/src/main.rs:259`). No `package provide Thread` is registered, and the module documents Thread 2.8.13 on a threaded 8.6.18 as its oracle, which is not installed here.
  - Runtime: none (`runtime/rust/src/cmd_coro.rs:958` calls itself "the single-threaded wasm build").
  - Registry: no `thread::*`, `tsv::*` or `tpool::*` spec in `rust/tcl-registry` or `specs/`.
  - Analysis: one hint function, `is_unregistered_thread_command` (`rust/bpf-tcl-ir/src/lower.rs:1216`), a namespace-prefix test that the comment says must not drive semantics. The `thread` hits in `tcl-lsp-server` and `tcl-spectcl` are Rust threads, not Tcl.
- **`tcl::process`:** registry only (`rust/tcl-registry/src/commands/tcl/tcl_process.rs`, `TCL90_PLUS`, subcommands `autopurge`, `list`, `purge`, `status` with `-wait`/`--`). No engine implements it.
- **`zipfs`:** registry (`rust/tcl-registry/src/commands/tcl/tcl_zipfs.rs`, `TCL90_PLUS`, twelve subcommands) and the shared path grammar in `tcl-cmd-core` (`rust/tcl-cmd-core/src/path.rs:93-199`: the `//zipfs:/` volume and 9.0 reading), used by both engines' `file` commands. No engine implements the filesystem or the command; the safe-interpreter code in both engines mentions `zipfs` only as a release-gated hidden command.
- **`tcl::build-info`:** one owner, `rust/tcl-dialect/src/build_info.rs` (compose and `query`), called by the VM (`rust/tcl-vm/src/command.rs:561`) and the runtime (`runtime/rust/src/version.rs`, `cmd_misc.rs:42`); the registry spec (`tcl__build_info.rs`) holds the field vocabulary separately as `ArgValue`s for completion. The engines' string has no `git-` prefix and no compiler tag, so `commit` and `compiler` differ from tclsh in form.
- **`tcl_platform`:** built in `rust/tcl-platform/src/lib.rs:703` (`threaded` = `0`), overwritten to `1` by the VM's thread installer; the runtime also reports the key (measured), while `tcl::build-info threaded` answers `0` in both.
- **`tcl::unsupported`:** registry specs for `representation`, `corotype` and `grapheme` (`tcl_unsupported_*.rs`) and for bare `timerate` (`timerate.rs`, `TCL86_PLUS`), but not for `tcl::unsupported::timerate` (the only spelling in a stock 8.6), `assemble`, `disassemble`, `getbytecode`, `inject` or `loadIcu`. `tcl-cmd-core/src/representation.rs` serves both engines' `representation`; `corotype` is per engine (`rust/tcl-vm/src/cmd_coro.rs`, runtime `cmd_coro.rs`). The runtime registers `::timerate` regardless of pin (`runtime/rust/src/cmd_control.rs:58`).

Duplicates are few because there is little implementation: the one real duplication is the release roster of these commands, written as registry surfaces, as the VM's and the runtime's registration lists, and as each engine's safe-hidden narrowing, and the three disagree (`timerate` in runtime 8.6, `tcl::unsupported::timerate` absent everywhere but tclsh).

## Design

- **Thread is a package, not a built-in.** The VM's implementation should register itself through the package system (`package ifneeded Thread <version> {…}` provided by the engine, so `package require Thread` succeeds without `load` and `package provide Thread` answers) and install no commands until required. The registry gains a Thread profile — commands, arities, `thread::send`'s script argument as a body evaluated in **another interpreter world** (the same world boundary as `interp eval`, see [interpreters](interps-design.md)), `-async` with a result variable written in the caller's world at a later event-loop point, and `tsv::*` as a process-global store outside every interpreter's variable world. Analysis then reports `thread::*` without the package as W120 and analyses sent scripts as isolated body units.
- **Release rosters from the oracle.** Which commands exist at each release (`tcl::process`, `zipfs`, `tcl::build-info`, `timerate` versus `tcl::unsupported::timerate`, each `tcl::unsupported::*`) is one table generated from `info commands` on tclsh 8.4–9.1 and read by the registry surfaces and both engines' registration (cross-cutting fault 4). An engine that does not implement a listed command registers a refusal with a reason naming the engine.
- **Process and zipfs ride on the host layer.** `tcl::process` needs the background-process table that `exec … &` also needs ([exec and pipelines](exec-design.md)); `zipfs` needs a virtual filesystem behind `tcl-platform`'s filesystem trait, with `tcl-cmd-core`'s existing `//zipfs:/` path grammar as its name owner. Both are engine work items, not analysis ones; analysis already has correct release gates for them.
- **Build identity.** `tcl_dialect::build_info` stays the one owner; it should also answer `tcl_platform(threaded)` so the two cannot disagree, and carry the `git-` prefix and a compiler tag so `commit` and `compiler` have tclsh's shape.
- **Conformance.** The fixtures become rows in the engine matrix; Thread rows run against tclsh 9.0/9.1 with the SDK package until a Thread build for 8.6 is installed in the oracle environment (cross-cutting fault 14).

## Plan

- **Fix the VM's leaked thread error** (owner: `rust/tcl-vm/src/cmd_thread.rs`): a synchronous `thread::send` whose error the caller caught must not be raised again by a later `vwait` or `-async` reply. Done when `t10` and `t11` match tclsh 9.0 under both VM pins.
- **Make the VM's Thread a package** (owner: `cmd_thread.rs`, `rust/tcl-vm-cli/src/main.rs:259`): provide `Thread` through `package ifneeded`, install commands on `package require`, answer `package provide Thread`. Done when `t01`–`t03` pass under the VM with the SDK path removed and `t09` shows no `thread::create` before the require.
- **Registry Thread profile** (owner: `tcl-registry`, new `commands/thread/`): `thread::*`, `tsv::*`, `tpool::*` with the package roster, `thread::send`'s script as a cross-world body and `tsv` as global shared state. Done when W120 fires for `thread::id` without the package, hover works, and `is_unregistered_thread_command` (`rust/bpf-tcl-ir/src/lower.rs:1216`) is deleted.
- **One generated release roster** for `tcl::process`, `zipfs`, `tcl::build-info`, `timerate`, `tcl::unsupported::*` (owner: `tcl-dialect` or the registry generator, from tclsh `info commands`). Done when the runtime's `::timerate` under 8.6 is gone, `tcl::unsupported::timerate` is known for 8.6 and unknown for 9.0 (W123 or W002 there), and `zipfs` gives one diagnostic, not W002 plus W123.
- **`tcl::process` and background pids in both engines**, after the host pipeline work in the exec plan. Done when `t04` matches tclsh 9.0.
- **`zipfs` in both engines or an explicit non-goal** (owner: `tcl-platform` filesystem trait plus `tcl-cmd-core/src/path.rs`). Done when `t05` matches tclsh 9.0, or fails with a documented refusal.
- **Build identity in one place** (owner: `rust/tcl-dialect/src/build_info.rs`): derive `tcl_platform(threaded)` and `build-info threaded` from the same fact; emit `git-` and a compiler tag. Done when `t06` agrees with itself under both engines.
- **The missing `tcl::unsupported` commands** (`assemble`, `disassemble`, `getbytecode`, `inject`, `loadIcu`): record them as non-goals in the engines' command inventories with a refusal naming the engine, and add registry specs so analysis knows the per-release set. Done when `t07` lists the same names or a documented refusal for each.
- **Startup and `unknown` cost in the debug VM:** measure why a VM run takes 7–11 s and each unknown command about 10 s (the auto-load path), and fix or document it; the fixture harness should not need 120 s timeouts.
- **Oracle environment:** install Thread for 8.6 (and keep the SDK's 3.0.4 for 9.x) in the reference set, so Thread rows have every release.
