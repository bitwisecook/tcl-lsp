---
name: irule-hud
description: "Analyse the HUD chain of a BIG-IP virtual server: its profiles (HUD filters), the events they raise, and every attached iRule's handlers merged into one ordered handler table, then report conflicts between iRules — shared connection variables, double responses, priority ordering, handlers that never fire, commands with no matching profile. Use when reviewing all the iRules on a virtual server together, checking whether iRules conflict, debugging iRule order or priority, or asking which iRules run for an event."
allowed-tools: mcp__tcl-lsp__analyze, mcp__tcl-lsp__event_info, mcp__tcl-lsp__irule_with_context, Bash, Read
---

# iRule HUD Chain

Treat every iRule on a virtual server as one dispatcher and check them
together.

## Steps

1. Read `../_prompts/irules_system.md`, especially "Execution model: the HUD
   chain".
2. Get a BIG-IP config and a virtual server. Given only a rule, find the
   virtual servers that attach it:
   `f5-query query '.ltm.virtual[] | select(.rules[] == "/Common/RULE") | .name' CONF...`
   and analyse each. With no config, ask for one; if there is none, analyse
   the iRules supplied, state the profiles and rule order you assume, and
   skip steps 3 and 5's profile checks.
3. HUD chain: `f5-query explain virtual VS CONF... --json` gives the profiles
   and the iRules in rule-list order. A profile marked `(unresolved)` is
   defined outside the supplied files (often a system default such as
   `/Common/http`); infer its type from the name and say so.
4. Sources: `mcp__tcl-lsp__irule_with_context` with `config_paths` and
   `rule_path` per iRule, then `mcp__tcl-lsp__analyze` on each rule's
   `source` for its events and diagnostics.
5. Handler table: for each event, every handler from every iRule, lower
   `priority` first (default 500; a top-level `priority N` sets the default
   for later `when` blocks). `analyze` lists each event once per rule, so
   read each handler and its priority from the `when` lines of the source. Mark equal-priority handlers from different
   iRules as order-unspecified. Use `mcp__tcl-lsp__event_info`
   `implied_profiles` to find handlers whose event no profile in the chain
   raises, and commands whose profile is missing.
6. Conflicts, each with rule, event and line:
   - a bare connection variable written in one iRule and read or written in
     another, unless the sharing is clearly intended
   - more than one handler that can `HTTP::respond` or `HTTP::redirect` in
     the same transaction without an `HTTP::has_responded` guard
   - `return` or `event disable all` used as if it stopped other iRules
   - equal-priority handlers for one event whose order matters
   - a `call`ed proc whose commands are invalid in a calling event, or whose
     `upvar 1` / `uplevel 1` touches another iRule's variables
   - the same `static::` name initialised by more than one iRule
   - per-request state that keep-alive carries into the next request

## Output

The HUD chain on one line (`tcp → client-ssl → http → … | rules: r1, r2`),
the handler table (event, priority, iRule, one-line summary), then conflicts
grouped by severity with a fix for each, then assumptions: profiles inferred
from names, missing config, and order the device decides.

$ARGUMENTS
