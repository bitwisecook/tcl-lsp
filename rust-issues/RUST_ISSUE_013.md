# RUST_ISSUE_013: the VM and runtime diverge on the options dict: a bare builtin error loses `-errorcode`/`-errorinfo`/`-errorstack` in the VM

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `catch` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

catch — the VM and runtime diverge on the options dict: a bare builtin error loses `-errorcode`/`-errorinfo`/`-errorstack` in the VM.
Completion codes 0-4 agree. But VM `completion_options` (command.rs:1107-1114) falls back to only `-code`/`-level` when the carried dict is empty, and a bare builtin error carries an empty dict (interp.rs:146); the VM never emits `-errorstack` even for user `error` (command.rs:1390-1394). Runtime always builds all five (cmd_error.rs:107-144). `catch {llength} m opts; dict get $opts -errorcode` → "key not known" in the VM, works in the runtime. Confidence: high
