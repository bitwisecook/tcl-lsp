---
name: irule-convert
description: >
  Modernise legacy iRule patterns to current best practices.
  Detects unbraced expressions, string concat for lists, deprecated
  matchclass, ungated logs, and other convertible patterns.
allowed-tools: Bash, Read, Edit
---

# iRule Convert (Modernise)

Detect and convert legacy iRule patterns to modern best practices.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file to modernise
3. Run the legacy pattern detection:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py convert $FILE
   ```
4. If no legacy patterns are found, report the iRule already follows best practices
5. Otherwise, apply these conversions using the Edit tool:
   - `matchclass` -> `class match` (IRULE2001)
   - Unbraced expr -> braced expr (W100)
   - String concat for lists -> `lappend` (W104)
   - `==` / `!=` for strings -> `eq` / `ne` (W110)
   - Missing `--` option terminator -> add `--` (W304)
   - Ungated log in hot event -> add `CLIENT_ACCEPTED { set debug 0 }` and wrap with `if {$debug}` (IRULE5001)
6. After editing, re-run diagnostics to verify the changes are clean:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
7. If new issues appeared, fix them (up to 3 iterations)
8. Report what was converted and the final validation status

$ARGUMENTS
