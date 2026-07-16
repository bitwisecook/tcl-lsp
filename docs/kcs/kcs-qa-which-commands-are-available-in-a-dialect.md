# KCS: Which commands does tcl-lsp consider available in a dialect?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp CLI, mcp

## Question

Why does the analyser accept `dict` and `lassign` in an iApp or Expect
script, but flag them in an iRule? Which commands count as "available" in
each dialect?

## Answer

Every dialect resolves to a **dialect profile** that owns the answer. A
profile combines two things:

1. **An embedded Tcl base version.** A vendor dialect is a real Tcl
   interpreter with extra commands, so its profile composes the vendor
   surface *with* that base. iApps embed Tcl 8.5, so 8.5 core (`dict`,
   `lassign`, `apply`) is available; Expect embeds Tcl 8.6, so 8.6 core
   (`lmap`, `coroutine`, TclOO) is too; tmsh scripts (`f5-tmsh`) run the
   tmsh shell's Tcl 8.5 host plus the `tmsh::` surface; the bpf framework
   dialect embeds Tcl 9.0. Commands introduced *after* the embedded base
   (`lmap` in an iApp or tmsh script, `zipfs` anywhere below Tcl 9.0) are
   reported as unavailable
   ([W123](../design/dialect-profile-model.md)).
2. **A disable list, for subtractive dialects.** F5 iRules embed a genuine
   Tcl 8.4.6, but the data-plane sandbox *removes* commands (`exec`,
   `file`, `socket`, `open`, `glob`, and the rest of F5's K36322151 list).
   The iRules profile bans exactly those, so they warn (W002) even though
   they are ordinary Tcl 8.4 core. Commands newer than 8.4 (`dict`,
   `lmap`) are never present in an iRule at any BIG-IP version.

A third axis covers **library versions**: the F5 surfaces are keyed on
the BIG-IP (TMOS) release, defaulting to the oldest supported version —
the conservative choice, so a command introduced in a later TMOS (for
example `HTTP2::header`, BIG-IP 16.1.0) is only offered once the target
version covers it, and using it below that pin draws W135 naming the
runtime as the guarantor. Likewise, a plain-Tcl host pins the Tk it
ships: on `tcl8.6`, a `package require Tk` guarantees Tk 8.6, so an
8.7-introduced widget option is not offered even when the require names
no version.

Unknown or misspelled dialect names fall back to a permissive profile that
accepts every standard Tcl version, so a typo in configuration never
floods a file with false warnings.

The same profile drives the analyser, the [command-line
tools](../design/dialect-profile-model.md) (`tcl registry-dump --dialect`,
`tcl lookup`), and editor highlighting, so they can never disagree about
availability.

## See also

- [`docs/design/dialect-profile-model.md`](../design/dialect-profile-model.md)
  — the compositional profile model (design doc).
- [`docs/design/compiler/dialects-events.md`](../design/compiler/dialects-events.md)
  — per-dialect base versions and the iRules event model.
