---
name: irule-create
description: "Create a new F5 iRule from a natural-language description. Generates the code following security best practices, validates with the LSP analyser, and iterates until clean. Use when creating new iRules, generating F5 iRule code from descriptions, writing iRule event handlers, or scaffolding iRule logic."
allowed-tools: mcp__tcl-lsp__analyze, mcp__tcl-lsp__event_info, Read, Write
---

# iRule Create

Generate an iRule from a description, validate with the LSP, iterate until
clean.

## Steps

1. Read `../_prompts/irules_system.md`.
2. Establish the HUD chain it will run in: the virtual server's profiles
   (tcp, client-ssl, http, …) and whether other iRules share the virtual
   server. If the request does not say, assume the smallest chain that
   supports it, state the assumption, and check each event's
   `implied_profiles` with `mcp__tcl-lsp__event_info`.
3. Generate the iRule: the right `when` handlers, braced expressions, `--`
   terminators, no eval on user data, K&R braces, 4-space indent, comments
   on non-obvious logic. Write it to share a virtual server safely:
   rule-specific prefixes on connection variables, `HTTP::has_responded`
   before `HTTP::respond` / `HTTP::redirect`, explicit `priority` only when
   order against other iRules matters.
4. Write it to a `.tcl` file (ask for a name or pick a sensible default),
   then call `mcp__tcl-lsp__analyze` with the contents as `source`. On a
   tool error report it and adjust the code.
5. Fix errors and warnings and re-validate, up to 5 iterations; then report
   what remains and why.
6. Show the final iRule in a ```tcl fence with the profiles it needs, the
   validation result and iteration count.

$ARGUMENTS
