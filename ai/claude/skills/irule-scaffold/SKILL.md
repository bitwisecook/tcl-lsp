---
name: irule-scaffold
description: "Generate an iRule skeleton from selected event names. Creates a template with proper log gating, local variable extraction, and placeholder sections for each event handler. Use when scaffolding new iRules, creating iRule templates, generating F5 iRule boilerplate, or starting a new iRule from event names."
allowed-tools: mcp__tcl-lsp__analyze, mcp__tcl-lsp__event_info, mcp__tcl-lsp__command_info, Read, Write
---

# iRule Scaffold

## Steps

1. Read `../_prompts/irules_system.md`.
2. Take the event names from the request. If none were given, offer the
   common events from the prompt with when each fires, and ask.
3. For each event call `mcp__tcl-lsp__event_info`: its multiplicity, side,
   `implied_profiles` (the HUD filters the virtual server needs) and
   `valid_commands`. Use only commands it reports, checking
   `mcp__tcl-lsp__command_info` for the target BIG-IP version when one is
   given; where metadata is missing, leave a placeholder instead of
   asserting a command.
4. Generate the skeleton: a comment on when each handler fires, guarded
   logging when requested, values extracted only where needed, commented
   placeholder sections (`# --- Request routing ---`), K&R braces, 4-space
   indent, connection variables with a rule-specific prefix.
5. Write the file, call `mcp__tcl-lsp__analyze` with the contents as
   `source`, fix and re-validate up to 3 iterations, then report the status,
   the profiles the skeleton needs, and anything remaining.

$ARGUMENTS
