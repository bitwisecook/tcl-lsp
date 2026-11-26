---
name: irule-hud
description: "Analyse the HUD chain of a BIG-IP virtual server: its profiles (HUD filters), the events they raise, and every attached iRule's handlers merged into one ordered handler table, then report conflicts between iRules — shared connection variables, double responses, priority ordering, handlers that never fire, commands with no matching profile. Use when reviewing all the iRules on a virtual server together, checking whether iRules conflict, debugging iRule order or priority, or asking which iRules run for an event."
allowed-tools: mcp__tcl-lsp__hud_chain, mcp__tcl-lsp__analyze, mcp__tcl-lsp__irule_with_context, Read
---

# iRule HUD Chain

Treat every iRule on a virtual server as one dispatcher and check them
together.

## Steps

1. Read `../_prompts/irules_system.md`, especially "Execution model: the HUD
   chain".
2. Call `mcp__tcl-lsp__hud_chain` with `config_paths` (or `config_text`) and
   `virtual`; given only a rule, pass `rule_path` to get every virtual
   server that attaches it. With no config, ask for one; if there is none,
   analyse the iRules supplied, state the profiles and rule order you
   assume, and skip the profile checks.
3. Read the result per chain:
   - `filters`: profiles client to server, with `profileType`, `layer`,
     `side`, `context`. `typeSource` `baseObject` is a shipped profile
     (`/Common/http`); `unresolved` is defined outside the supplied files,
     so the stack is incomplete. `impliedFilters` lists the assumed TCP.
     Two typings are unmeasured: http2, http-compression and
     web-acceleration count as HTTP, and a virtual without a transport
     profile gets TCP; say so when a finding depends on either.
   - `rules`: rule-list order; `handlersKnown: false` means the source
     could not be read.
   - `events`: each event's `handlers` in execution order (priority, then
     rule position, then source order: F5's documented order). `tiedWith`
     names other iRules at the same priority, whose rule-list order is
     documented but not measured on a device. `raised`
     is `false` when the profiles cannot raise the event, `null` when
     undecided.
4. For conflicts, read each rule's source (`mcp__tcl-lsp__irule_with_context`
   with `rule_path`) and `mcp__tcl-lsp__analyze` it. Report, each with rule,
   event and line:
   - handlers whose event is not `raised`, and commands whose profile is
     missing
   - a bare connection variable written in one iRule and read or written in
     another, unless the sharing is clearly intended
   - more than one handler that can `HTTP::respond` or `HTTP::redirect` in
     the same transaction without an `HTTP::has_responded` guard
   - `return` or `event disable all` used as if it stopped other iRules
   - `tiedWith` handlers whose relative order matters
   - a `call`ed proc whose commands are invalid in a calling event, or whose
     `upvar 1` / `uplevel 1` touches another iRule's variables
   - the same `static::` name initialised by more than one iRule
   - per-request state that keep-alive carries into the next request

## Output

The HUD chain on one line (`tcp → client-ssl → http | rules: r1, r2`), the
handler table (event, priority, iRule, one-line summary), then conflicts
grouped by severity with a fix for each, then assumptions: unresolved
profiles, missing config, and documented-but-unmeasured behaviour (tie
order, shared variables, `event disable` scope).

$ARGUMENTS
