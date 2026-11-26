---
name: irule-diff
description: "Compare two iRule versions and explain the semantic differences, security implications, performance changes, and breaking changes. Uses LSP analysis on both files. Use when comparing iRule versions, diffing F5 iRules, analysing iRule change impact, or reviewing iRule modifications."
allowed-tools: mcp__tcl-lsp__analyze, mcp__tcl-lsp__irule_with_context, mcp__tcl-lsp__hud_chain, Read
---

# iRule Diff

## Steps

1. Read `../_prompts/irules_system.md`, then both files.
2. Call `mcp__tcl-lsp__analyze` on each version's source (for a BIG-IP
   config, take the rule body from `mcp__tcl-lsp__irule_with_context` with
   `config_text` or `config_paths` and `rule_path` first). Compare the
   events, diagnostics and firing order. If a call fails, compare the
   sources by hand and say LSP analysis was unavailable.
3. Explain, concisely and operationally, with a heading each: semantic
   changes (behaviour, not lines); handlers added, removed, or re-prioritised;
   security implications; performance on the hot path; breaking changes to
   traffic handling. Focus on any specific question asked.
4. Changes that reach other iRules on the same virtual server: a new or
   renamed connection variable, a new `HTTP::respond` / `HTTP::redirect`
   without an `HTTP::has_responded` guard, a `priority` change that reorders
   handlers against another iRule, `event disable` or `return` changes, a
   new command whose profile the virtual server may lack. With a config,
   check them against each attached virtual server's HUD chain
   (`mcp__tcl-lsp__hud_chain` with `rule_path`).

$ARGUMENTS
