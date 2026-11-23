# Channels and the `chan` command

**Question:** what is the channel surface of each Tcl release (the `chan` ensemble, the legacy commands, options and their values, handler protocols for reflected channels and transforms, standard channels, naming, transfer between interpreters), what do the VM and the Rust runtime implement of it, and what does every analysis consumer know about channels? This page builds on [encodings and channels](encodings-channels-design.md) (the input codec and the script reader), [event loop](event-loop-design.md) (`fileevent`/`chan event`), [exec](exec-design.md) (`open |`, `chan pipe`) and [interpreters](interps-design.md) (`interp share/transfer`), and does not repeat their findings. **Method:** 23 fixtures in [`oo-dispatch-probe/chan/`](handover/oo-dispatch-probe/chan/), run by `run.sh` from a fresh temporary directory with stdin from `/dev/null` under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0, Jim, `tclvm --tcl-version 8.6|9.0` and `run_script --quiet --tcl-version 8.6|9.0` (`engines.tsv`; c23 was run with `hi` piped to stdin; `notes.txt` has one ad-hoc row). Only temporary files and one `socket -server` on 127.0.0.1 with port 0 were used. Static behaviour was probed through the MCP tools (`analyser-mcp.out`; a cached build that may lag `35f11afc`). The code was then read for every owner.

**Verdict:** the registry describes the `chan` surface well, with correct release gates, and C Tcl is consistent across releases apart from documented additions. Neither engine implements a channel layer. The VM has no `chan`, `fcopy`, `fileevent`, `socket`, `pid` or `zlib`. It reads files as a snapshot taken at `open` and never reads the real stdin. The runtime has `chan` only as a dispatcher onto ten legacy commands; everything else in it refuses, citing a "WASM runtime". Both engines accept and then discard `-buffering`, `-buffersize`, `-blocking` and `-eofchar`: `fconfigure $f -buffering line` returns `full` afterwards, and `-buffering sideways` is not an error. This happens because the shared `tcl-cmd-core` setter drops them. No engine has reflected channels, transforms, `fcopy`, sockets, pipes or channel transfer. On the analysis side the worst fault is a **false E002 on every `socket -server` call**. Handler and callback prefixes are navigable but their protocols are not checked. There is no channel instance model, so leaks, use after close and writes to read-only channels go unreported. The option list is written three times in the registry.

## Measured

Wrong answers first. "VM" and "runtime" mean both pins unless one is named.

| Case (fixture) | tclsh | VM | runtime |
| --- | --- | --- | --- |
| `fconfigure $f -buffering line; fconfigure $f -buffering` (c20) | `line` | `full` | `full` |
| `-buffering sideways`, `-buffersize abc` (c20) | errors | accepted silently | accepted silently |
| `-buffersize 99999999` then query (c20) | `1048576` (clamped) | `4096` | `4096` |
| 9.0 `-eofchar {a b}` (c16) | error "must be non-NUL ASCII character" | accepted | accepted |
| `gets stdin` with `hi` piped in (c23) | `hi` | empty (stdin is always at EOF, `tcl-vm/src/cmd_chan.rs:24–27`) | `can not find channel named "stdin"`, script aborts |
| read channel opened, then file appended by another channel (c23) | sees `two` | `{}` (read channels snapshot the file at `open`, `cmd_chan.rs:184`) | right |
| `file channels` (c02, c22) | `stderr stdin stdout` | `unknown or ambiguous subcommand "channels": must be …, channels, …` | empty list (no standard channels) |
| `fblocked` with no argument (c17) | wrong # args | returns `0` (stub, `cmd_chan.rs:128`) | right |
| `interp transfer {} $f k` (c21) | moved; child writes | silent no-op, child `can not find channel` | run refused, exit 2 |
| `vwait` with no event source (`notes.txt`) | blocks | exits 1 from inside `catch` | refused, exit 2 |
| 8.6 `read $f nonewline` (c19) | accepted | right | `expected non-negative integer` |
| 8.6 `puts $f x nonewline` (c19) | accepted | wrong # args (plan 157) | wrong # args |
| 8.6/9.0 `close $f write`, `chan close` direction (c05) | half-close | wrong # args | wrong # args |
| channel names (c03) | `file3`… (8.x), `file5`… (9.x), number reused after close | `file2`, never reused | `file1`, never reused |
| 9.0 `fconfigure` bad-option list (c04) | includes `-stat` | no `-stat` | no `-stat` |
| 9.0 socket options (c13) | adds `-peername -sockname -keepalive -nodelay` | no `socket` | `"socket" is not supported under the WASM runtime` |
| `chan create`, `push/pop`, `copy`, `fcopy`, `pipe`, `postevent`, `pending`, `truncate`, `names`, `isbinary`, `zlib push` (c06–c09, c11, c12, c14–c16, c18) | work | `invalid command name "chan"`/`"zlib"`/`"fcopy"` | refused "not supported under the WASM runtime" |

Right in both engines: `fconfigure` read-out of a file and of the standard channels, including 9.0's `-profile strict` (c02, c04); `seek`/`tell`/`eof`/`read N` on files (c11); `gets` with a variable, `read -nonewline` (c19); `can not find channel named` after close (c10); `-translation` and `-encoding` validation (c20); `fconfigure` defaults of `-buffering` per standard channel. The runtime's `chan` subcommand table and unknown-subcommand message are right for 8.6 and 9.0 (c01), because they come from the registry's release table.

C Tcl by release (c01, c05, c07, c16, c17):

- `chan` arrives in 8.5 with `blocked close configure copy create eof event flush gets names pending postevent puts read seek tell truncate`.
- 8.6 adds `pipe pop push` and the `close` direction (8.5's `chan close` takes exactly one argument).
- 9.0 adds `isbinary`, `-profile`, the `-stat` option on files, `-keepalive`/`-nodelay` on sockets and the refchan `truncate` method. 8.5 and 8.6 reject a handler that lists `truncate`: `returned bad method "truncate"`.
- 9.0 changes the error wording from `channelId` to `channel`, makes `-eofchar` a single character (8.x reports `{{} {}}` for a read-write file), rejects `-encoding binary` (it reads back `utf-8`), and makes the legacy `read $f nonewline` and `puts $f x nonewline` errors.
- 9.1 is the same as 9.0 on every row.
- Jim has no `chan`, `fcopy`, `fblocked`, `file channels` or `zlib push`. Its channels are commands (`::aio.handle4`), and its `fconfigure` takes `-buffering`, `-translation binary|text` and nothing else.

Handler protocols as measured (c06, c07, c09, c18):

- A reflected channel's handler is called as `cmd method id ?args?`: `initialize id mode` must return the method list; then `read id count`, `write id data`, `seek id offset base`, `configure id option value`, `cget id option`, `cgetall id`, `blocking id mode`, `truncate id length` (9.0), `watch id events` and `finalize id`.
- `initialize`, `finalize`, `watch` and `read` or `write` are required, with fixed errors for a missing method (c08).
- `tell` is `seek 0 current`.
- A transform is called `initialize, clear, write, drain, flush, finalize` on a write-and-pop sequence, and `initialize, read, drain, flush` on a read-and-close sequence.
- `chan postevent` from inside `watch` reaches `chan event`.

Analysis (`analyser-mcp.out`):

| Case | Report |
| --- | --- |
| `socket -server acc 8080`, `socket -server acc -myaddr 127.0.0.1 0` (8.6, 9.0) | **E002 "expected at least 2, got 1"**: false, at error severity |
| `puts stdout x nonewline` (8.6) | E003, false (plan 123) |
| `-buffersize abc`, `-blocking maybe`, `close $f sideways`, `chan create sideways rc`, `chan postevent $c sideways`, `fcopy … -size` with no value, odd option/value counts | nothing |
| `chan puts a b c d` | nothing (plan 129) |
| handler with too few parameters (`proc rc {cmd}` for `chan create`, `proc one {a}` for `socket -server`/`chan push`, `proc cb {}` for `fcopy -command`) | nothing, although the registry declares the appended arity |
| protocol-fixed but unused parameters (`proc acc {s a p}`) | W214 "unused parameter" hints (noise) |
| `close $f` then `gets $f`, through an alias, or in a procedure; channel opened in a procedure and never closed; `puts` on a read-only `open` | nothing |
| `gets $n` where `n` is an integer, `puts $l` where `l` is a list | W126 (right) |
| `-buffering sideways`, `chan pending sideways` | W127 (right) |
| release gates: `chan` in 8.4, `push` in 8.5, `-profile` in 8.5, direction in 8.5 | W002/W004/W137/W135 (right; 8.4 `chan` gives W002 and W123; `close … write` is W135, `chan close … write` is W137) |
| `exec sh -c [gets $sock]` | no taint report (plan 183) |
| definition of the handler word in `chan create`, `chan push`, `socket -server`, `fcopy -command`, `chan event` | right |
| hover on a handler word; on a channel variable | proc signature only; "Inferred intrep: channel" |
| optimiser over `fconfigure`/`tell`/`eof`/`chan names` | no rewrites (safe) |

## Census

**Shared kernel.** `tcl_cmd_core::channel` (`rust/tcl-cmd-core/src/channel.rs`) owns `ChannelConfig` (`:305`: encoding, profile, direction, input and output translation only), `ConfigOption` (`:42`), open-mode parsing (`resolve_open_access_mode`, `:483`), standard-channel configs (`:615`), output encoding (`encode_output`, `:764`) and the `fconfigure` read-out:

- `config_value` (`:664–673`) hard-codes `-blocking 1`, `-buffersize 4096` and an empty `-eofchar`, and derives `-buffering` from the channel name (`:717`);
- `set_config_value` (`:700–714`) returns `Ok(())` for `-blocking`, `-buffering`, `-buffersize` and `-eofchar` without validating or storing them.

That one function causes every c20 wrong answer in both engines. The option names reach the engines from the registry (`tcl-registry/src/native_ensemble.rs:126`).

**VM.**

- `rust/tcl-vm/src/cmd_chan.rs` registers `open close gets read eof flush seek tell fconfigure` and a constant `fblocked` (`:118–128`).
- `Channel` (`:49–67`) is either a whole-file snapshot (`Read`) or an `std::fs::File` (`Write`).
- The standard channels are special-cased by name (`is_std`, `:974`): stdin is always at EOF, and stdout follows the captured output.
- The table is a per-VM `HashMap<String, Channel>` (`interp.rs:1271`), named from a counter (`interp/configuration/host_integration.rs:344`).
- Files are opened with `std::fs`, not through the host.
- There is no `chan`, `fcopy`, `fileevent`, `socket`, `pid` or `zlib`.
- `cmd_file.rs:451–461` answers every unimplemented `file` subcommand (`channels`, `tempfile`, `copy`, …) with an "unknown or ambiguous subcommand" message whose list contains that subcommand.

**Runtime.**

- `runtime/rust/src/cmd_chan.rs`: `ChanState`/`ChannelTable` (`:52`, `:60`, per interpreter, `interp.rs:1063`), named `file{n}` (`:188`), opened with `std::fs`.
- `install` (`:211–224`) registers the legacy commands plus `chan`.
- `chan_cmd` (`:232–299`) forwards ten subcommands to the legacy handlers, so their wrong-argument messages name the legacy command (`chan close` says `close channelId`). The rest are refused with "not supported under the WASM runtime" (`:288–292`).
- `fileevent`, `fcopy` and `socket` are refusals in `cmd_misc.rs` (exec page).
- `file channels` (`cmd_fs.rs:744`) lists the table, which has no standard channels.
- Interpreter transfer is refused by the worker policy (interps page).

**Host.** `tcl-platform/src/lib.rs`:

- `Filesystem` (`:269`) is whole-file only. Its comment says streaming channels "layer on top separately", but no such layer exists.
- `Sockets` (`:451`) is an empty marker trait.
- `Process` (`:455`) runs a program to completion.
- `tcl-host-native` implements `Process` (`:653`) and no `Sockets`.
- `tcl-runtime-api` has no channel trait.

**Registry.**

- `commands/tcl/chan_.rs` (943 lines) has 21 subcommands with release surfaces (`pipe`/`pop`/`push` 8.6, `isbinary` 9.0), value tables for direction, mode, pending and postevent, the configure options, and `command_prefixes` with `AppendedArity::AtLeast(2)` for `create` and `push`.
- `close` and `chan close` have `Arity::new(1, 2)` with the direction value gated to 8.6. 8.5's arity is not windowed (plan 127).
- The option list exists three times: `chan_.rs` `CONFIGURE_OPTIONS` (`:100`, with value grammars) and `CMD_OPTIONS` (`:831`, plain `value("mode")`), and `fconfigure_.rs` (`:180`).
- `socket_.rs` declares the server form in `INVOCATION_FORMS` (`:98–111`), but the command-level `arity: Arity::at_least(2)` (`:183`) is what the arity check reads, hence the false E002. `-server` declares `AppendedArity::Exactly(3)` (`:57`).
- `fcopy.rs` (`-command`, `AtLeast(1)`, `:50`) and `fileevent.rs` are live; `fcopy_.rs` and `fileevent_.rs` are dead duplicates (plans 123, 169).
- `zlib.rs` covers `zlib push`.
- `channel_configuration.rs` describes the encoding/translation subset for the source-channel rule.
- `TclType::Channel` is the return type of `open`, `socket`, `chan create`, `push` and `pipe`. `TAINT_SOURCE` is on `gets`, `read`, `chan gets`, `chan read` and `socket`.

**Analysis.**

- `ArgRole::Channel` drives W126 (`tcl-compiler/src/analyser/diagnostics/dataflow/value_domains.rs`, `original_roles.rs`), the taint rule that a value reaching only a channel slot is not a sink (`taint/sink_warnings/sink_emission.rs:198`), and the optimiser's closed-consumer pass (`optimiser/closed_consumer/application.rs:421`).
- W127 for channel options comes from `analyser/diagnostics/usage/source_channel.rs`.
- W103 is the word-shape rule in `security/source_paths.rs`.
- Callback heads are recorded with their appended arity (`signature_scan/walker.rs:745–785`), which feeds navigation, but no diagnostic compares that arity with the target procedure's parameters.
- Hover and inlay hints show `channel` as a type (`tcl-lsp-core/src/inlay_hints.rs:301`).
- Nothing models a channel's identity, mode, configuration or open/closed state.
- Codegen compiles channel commands as ordinary runtime invocations (*inference*: no channel-specific code in `tcl-codegen`).
- iRules has no Tcl channels; the registry excludes `chan` and the legacy commands there.

**Duplicated rules and disagreements.**

1. Channel tables and read/write paths: two (VM and runtime), with different representations (snapshot against stream), naming (`file2` against `file1`) and standard-channel handling.
2. The `fconfigure` option list: three registry copies.
3. Half-close direction gate: two codes (W135, W137).
4. `chan` dispatch: the runtime forwards to the legacy commands, and the VM has none. The registry, the runtime table and C agree on the roster; the engines disagree with all three on behaviour.
5. Pipeline detection: three copies (exec page).
6. Encoding decoders: seven readers (encodings page).
7. Standard-channel defaults: `ChannelConfig::standard` and `channel_buffering` agree with each other and with tclsh.

## Design

One channel model, stated once per layer:

- **Registry (descriptors).**
  - One `ChannelOptions` table per channel type (generic, file, socket, serial, console, refchan), with value grammars and release gates. `-buffering` is one of `full line none`; `-buffersize` is an integer, clamped to 1 MiB; `-blocking` is a boolean; `-eofchar` is a pair until 8.6 and one non-NUL ASCII character from 9.0; `-profile` (9.0), `-stat` (9.0, files), `-inputmode` (9.0, console). `chan configure`, `fconfigure` and `socket` read it, and the engines take their names from it (they already do through `native_ensemble`).
  - Handler protocols become **fixed-signature callback descriptors**: `refchan` (methods, appended words per method, required set, release gate for `truncate`) and `transchan` (`initialize finalize read write drain flush clear limit?`). `chan create`/`chan push` point at them instead of a bare `AtLeast(2)`, `socket -server` keeps `Exactly(3)`, and `fcopy -command` keeps `AtLeast(1)`.
  - Arity comes from the invocation forms, never a command-level minimum that contradicts them.
- **Engines (`tcl-runtime-api` + `tcl-cmd-core`).**
  - A `Channels` trait: create from a host stream, look up by name, configure, read, gets, write, flush, seek, tell, truncate, eof, blocked, pending, close with direction, names, transfer and share between interpreters, register an event handler.
  - One implementation in `tcl-cmd-core` of the buffer, translation, eofchar and codec (the input path of plan 186 plus the output path that exists now), `-buffering`/`-buffersize`/`-blocking` state, stacking of transforms, the refchan and transchan dispatchers (calling back into the engine through a small `invoke(prefix, words)` hook), and C's naming (`file<fd>` on POSIX).
  - The engines keep only the handle table and the command registration.
  - The standard channels are ordinary entries opened from the host's stdin, stdout and stderr streams.
  - Unimplemented operations raise a catchable Tcl error naming the engine and operation.
- **Host (`tcl-platform`).** A streaming `Filesystem::open(path, access, perms) -> Box<dyn Stream>` (read, write, seek, truncate, readiness), with `Sockets` given methods (`connect`, `listen`, `accept`, local and peer names, `keepalive`, `nodelay`) and `Process` given pipelines (plan 182). `chan pipe` is a host pair of streams. WASM hosts return `Unsupported` for what they lack.
- **Analysis (instance values, L4).**
  - A channel is an instance in the instance-value lattice: identity (the creating site), kind (file, socket, server, pipe, refchan, std), mode (read, write or both, from the `open` access value), configuration fields, state (open, half-closed, closed) and owning interpreter.
  - Created by `open`, `socket`, `chan create`, `chan pipe` (two instances), `file tempfile` and `zlib push`/`chan push` (same identity, stacked).
  - Destroyed by `close`, `chan close` (or half-closed with a direction) and `interp transfer`.
  - It flows through variables, lists, dicts and aliases as `Answer<ChannelRef>` with receipts, and is `Unknown` across unproven calls and `uplevel`.
  - Consumers:
    - W126 reads the kind;
    - new diagnostics for use after close, writing to a read-only channel, and a leak (a procedure-local channel neither closed, returned, stored nor given to `fileevent`/`chan event`/`fcopy -command`);
    - W103 reads the `open` argument's value (plan 188);
    - taint treats socket instances as sources;
    - hover shows kind, mode and creation site;
    - definition on a channel variable goes to its creating command.
  - The optimiser keeps every channel command impure (`FileIo`); none is foldable.
- **Events and copies.** `fileevent`/`chan event`, `fcopy -command`, `socket -server` and `chan postevent` handlers are deferred global bodies (plan 169's descriptor). A refchan's `watch` and `postevent` connect to the event driver of plan 171.
- **Interpreters.** `interp share`/`transfer` are `Channels` operations, and the instance's owning interpreter changes accordingly (plan 178 owns the interpreter side).

**Scope for the LSP release:** the false `socket -server` E002, the registry option tables and value grammars, handler protocol descriptors with arity checks, and use after close and read-only writes inside one body where the instance is proven. **Later:** the engine `Channels` trait and kernel, host streams and sockets, refchans and transforms in the engines, transfer, leak detection across procedures, and the WASM decisions.

## Plan

- **Fix the false `socket -server` E002.** Owner: `rust/tcl-registry/src/commands/tcl/socket_.rs` (drop the command-level `Arity::at_least(2)` in favour of the `INVOCATION_FORMS` client and server forms) and the arity checker that reads it. Done when `socket -server acc 8080` and `socket -server acc -myaddr 127.0.0.1 0` are clean under 8.4–9.1, `socket host` still gives E002, and the a03 rows are regression tests.
- **Stop the engines discarding channel options.** Owner: `rust/tcl-cmd-core/src/channel.rs` (`ChannelConfig`, `config_value`, `set_config_value`). Store and validate `-blocking`, `-buffering`, `-buffersize` (clamped as in C) and `-eofchar` (pair until 8.6, single character from 9.0), and return them from `config_value` and `config_list`. Implement line and none buffering in both engines' write paths. Done when c16 and c20 match tclsh 8.6 and 9.0 in the VM and the runtime.
- **Real stdin and live read channels in the VM.** Owner: `rust/tcl-vm/src/cmd_chan.rs`. Replace the `Read` snapshot with a stream and read stdin from the host instead of a constant EOF. Done when c23 matches tclsh. Make the runtime's `gets`/`read` find `stdin`, done when c23 matches there too. Give `fblocked` an arity check and a channel lookup (c17).
- **`file channels` and honest refusals.** Owners: `rust/tcl-vm/src/cmd_file.rs:451–461` and `runtime/rust/src/cmd_fs.rs:744`. Implement `file channels` (with standard channels) in both engines. Make unimplemented `file` subcommands say they are unimplemented, naming the engine, instead of "unknown or ambiguous". Replace "WASM runtime" in `runtime/rust/src/cmd_chan.rs:288–292` and `cmd_misc.rs` with the engine's name. Done when c02 and c22 match tclsh, and c01–c18 refusals name the engine.
- **One option table per channel type.** Owner: `rust/tcl-registry/src/commands/tcl/chan_.rs` and `fconfigure_.rs`. Keep `CONFIGURE_OPTIONS` (with value grammars) as the only list, delete `CMD_OPTIONS` and the `fconfigure_.rs` copy, and add release-gated `-stat` (files, 9.0), socket options (`-peername`, `-sockname`, `-keepalive`, `-nodelay`, `-error`, `-connecting`), serial and console (`-inputmode`) options as typed tables. Overlaps 128 and 188, which own the odd-count and value-grammar checks. Done when the a01 and a03 value rows each give one diagnostic and a registry test asserts one table.
- **Handler protocol descriptors.** Owner: `rust/tcl-registry` (new `channel_protocols.rs` beside `body_execution.rs`), used by `chan_.rs` (`create`, `push`), `socket_.rs` (`-server`) and `fcopy.rs`/`chan copy` (`-command`). Describe the refchan methods (required set, appended words, `truncate` from 9.0) and transchan methods. Done when completion offers the method names inside a handler's `switch`, hover on the handler word lists the protocol, and a registry test pins c06–c09's measured call order.
- **Callback arity diagnostics.** Owner: the W214-family check over `signature_scan` callback rows (`rust/tcl-compiler/src/signature_scan/walker.rs:745`), shared with traces (plan 160). Report a procedure that cannot accept the protocol's appended words. Stop reporting "unused parameter" for parameters a protocol fixes. Done when the a03 handler rows report and `proc acc {s a p}` gives no unused-parameter hint.
- **Missing value checks.** Owner: `chan_.rs`: `chan create` mode (`read`, `write` or both, as a list), `chan postevent` event spec, `close`/`chan close` direction (closed value), `chan puts` maximum (plan 129). Use one code for the half-close release gate (W135 and W137 now). Done when a01/a03 give one report each.
- **Channel instance values.** Owner: the instance-value work in `tcl-compiler` (with plan 188's channel item; this extends it). Record creation sites, kind, mode from the `open` access word, and the closed or half-closed state. Report use after close and a write to a read-only channel where the instance is proven. Report a leak only within one procedure body with no escape. Hover shows kind, mode and site. Done when the a04 rows report and nothing new appears in the analysis-performance corpus without a matching tclsh error.
- **`Channels` trait and one kernel.** Owner: `rust/tcl-runtime-api` (trait) and `rust/tcl-cmd-core/src/channel.rs` (buffers, translation, eofchar, codec, stacking, refchan and transchan dispatch, naming). Port the VM's and the runtime's `cmd_chan.rs` to registration over it. Extends plan 186 (input path) to the whole channel. Done when c02–c05, c10, c11, c19 and c20 pass in both engines from the shared code, and each `cmd_chan.rs` is under 300 lines.
- **Host streams, sockets and pipes.** Owner: `rust/tcl-platform/src/lib.rs` (streaming `open`, `Sockets` methods) and `rust/tcl-host-native`; pipelines belong to plan 182. Done when c13 and c14 pass in both engines on the native host, and WASM hosts refuse with the documented error.
- **Engine roster: `chan` (VM), `fcopy`/`chan copy`, `chan create/push/pop/postevent/pending/truncate/names/isbinary`, `zlib push`, `pid`, `socket`.** Implement them over the kernel, or record each as a non-goal in the engine contract. This overlaps plans 171, 186 and 193, which list the VM `chan` and `fileevent`; this item adds the rest. Done when c06–c09, c12, c15, c16 and c18 pass, or are marked expected-unsupported in the vector runner with the refusal text.
- **Interpreter transfer.** `interp share`/`transfer` implemented as `Channels` operations (plan 178 owns the interpreter side). Done when c21 matches tclsh in both engines.
- **Conformance rows.** Add `oo-dispatch-probe/chan/` to the plan 198 vectors, with the standard-input and socket fixtures marked as needing the harness to pipe input and allow loopback. Done when `engines.tsv` is regenerated by the runner rather than by `run.sh`.

## Status on the current source

Confirmed: `fconfigure $f -buffering line` reads back `full` in the VM (tclsh `line`), from the shared setter in `tcl-cmd-core/src/channel.rs`. Not reproduced: the false E002 on `socket -server acc 8080` (clean under `tcl8.6` and `tcl9.0` on the current source; a cached-build artefact), so the first plan item below becomes a regression row. Analyser findings on handler arity, value checks and channel state were not re-run. Plan items 208–216.
