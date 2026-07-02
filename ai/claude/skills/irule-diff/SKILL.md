---
name: irule-diff
description: "Compare two iRule versions and explain the semantic differences, security implications, performance changes, and breaking changes. Uses LSP context analysis on both files. Use when comparing iRule versions, diffing F5 iRules, analysing iRule change impact, or reviewing iRule modifications."
allowed-tools: mcp__tcl-lsp__irule_with_context, Read
---

# iRule Diff

Compare two iRule versions and analyse the differences.

## Steps

1. Read the domain knowledge from `../_prompts/irules_system.md`
2. Read both iRule files (the user should provide two file paths)
3. Run context analysis on both files: call `mcp__tcl-lsp__irule_with_context` once with the contents of `$FILE_A` as `config_text`, and again with the contents of `$FILE_B` as `config_text`
4. If the analysis tool fails on either file, fall back to manual source comparison and note that LSP analysis was unavailable
5. Compare the two versions and explain:
   - **Semantic changes** -- What changed in behaviour (not just line diffs)?
   - **Events** -- Any events added, removed, or reordered?
   - **Security implications** -- Do the changes introduce or fix security issues?
   - **Performance implications** -- Any changes to hot-path efficiency?
   - **Breaking changes** -- Could these changes affect traffic handling?
6. If the user asked a specific question about the diff, focus on that

## Output format

Focus on what matters operationally. Be concise. Use headings for each
analysis section.

$ARGUMENTS
