# KCS: Which commands does tcl-lsp consider available in a dialect?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp CLI, mcp

## Question

Which commands count as "available" in each dialect, and why does `dict`
resolve in an Expect script but not in an iApp or an iRule?

## Answer

Every dialect resolves to a **dialect profile** that owns the answer. Two
things decide availability, and a command needs both.

1. **The profile's point** — the Tcl release the dialect's interpreter
   really is, plus whatever vendor packages sit on it. Expect embeds Tcl
   8.6, so 8.6 core (`lmap`, `coroutine`, TclOO) is available; the bpf
   framework dialect embeds Tcl 9.0. All three F5 script dialects — iRules,
   iApps, and tmsh — ride F5's `f5-tcl` trunk, a fork of Tcl at **8.4.6**,
   so `dict`, `lassign`, and `apply` are absent from every one of them, at
   any BIG-IP version. A command newer than the point (`zipfs` below Tcl
   9.0) is reported unavailable
   ([W123](codes/kcs-diagnostic-w123-unresolved-command.md),
   [W002](codes/kcs-diagnostic-w002-command-disabled-in-dialect.md)).

   The `::tcl::` namespace is one of these later additions: it does not
   exist at 8.4, so `tcl::mathop::+`, `tcl::build-info`,
   `tcl::unsupported::corotype`, and `tcl::tm::path` are all disabled in
   the F5 dialects — each admitted at its own real introduction release
   (8.5, 8.6, or 9.0) once a dialect's point reaches it. Individual
   `tcl::mathop` operators are gated more finely than the namespace: most
   need only its own Tcl 8.5, but `lt`/`le`/`gt`/`ge` (TIP 461, the
   string-ordering counterparts to `eq`/`ne`) need Tcl 9.0.

2. **The command's own declared surface** — where the command exists at
   all. There is no ban list anywhere in the model. F5's data-plane sandbox
   removes about fifty commands from iRules (`exec`, `file`, `socket`,
   `open`, `glob`, and the rest of F5's K36322151 list); each of those
   carries a surface naming Tcl but *not* iRules, so it reports as disabled
   (W002) rather than unknown, and no query path can re-admit it by
   accident.

The ladder reaches into **argument mini-languages** too: a command can be
available while one of its argument values is not. `format %b` (binary)
needs Tcl 8.6, `format %llu` needs 9.0, and `string is dict` names a
class that only exists on 9.0 — using them below those releases draws
W138/W137 naming the dialect's effective Tcl version, which a
`package require Tcl` line can raise above the ambient dialect.

A third axis covers **library versions**. The F5 surfaces are keyed on the
BIG-IP (TMOS) release, taken from `tclLsp.bigipVersion`, `--bigip-version`
on `f5 irule event-info`, or the oldest supported release by default — the
conservative choice, so a command introduced in a later TMOS
(`HTTP2::header`, BIG-IP 16.1.0) is only offered once the target covers it.
The floor applies below the command level too: `SSL::c3d cert_lifespan`,
`SSL::c3d cert_start_date`, and the `mcp` forms of `persist` require BIG-IP
21.1. Using a command, subcommand, or gated enumerated value below its pin
draws W135.

The data itself is declared against a **BIG-IP 15.0 baseline with an open
maximum**: an F5 command, iRules event, LTM profile type, or config-schema
entry with no explicit introduction release is asserted present since 15.0
and not yet removed, so a target below 15.0 flags the whole declared
surface. A plain-Tcl host pins its Tk the same way: on `tcl8.6`, a
`package require Tk` guarantees Tk 8.6, so an 8.7-introduced widget option
is not offered even when the require names no version.

Unknown or misspelled dialect names fall back to a permissive profile that
accepts every standard Tcl version, so a typo in configuration never
floods a file with false warnings.

The same profile drives the analyser, the [command-line
tools](../design/dialect-profile-model.md) (`tcl registry-dump --dialect`,
`tcl command-info`), and editor highlighting, so they can never disagree
about availability.

## See also

- [`docs/design/dialect-profile-model.md`](../design/dialect-profile-model.md)
  — the compositional profile model (design doc).
- [`docs/design/compiler/dialects-events.md`](../design/compiler/dialects-events.md)
  — per-dialect base versions and the iRules event model.
