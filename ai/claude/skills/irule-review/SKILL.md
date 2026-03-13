---
name: irule-review
description: >
  Security and safety review of an iRule. Combines LSP static analysis
  (security, taint, thread safety diagnostics) with deep analysis of
  input validation, information leakage, race conditions, and DoS vectors.
allowed-tools: Bash, Read
---

# iRule Security Review

Perform a comprehensive security review combining static analysis with deep analysis.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file to review
3. Run the security-focused analysis:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py review $FILE
   ```
4. Present the static analysis findings (security, taint, thread safety)
5. Go beyond the static analysis. Check for:
   - Input validation gaps (HTTP::uri, HTTP::query, HTTP::header, HTTP::cookie used without sanitisation)
   - Information leakage in logs or HTTP::respond bodies
   - Race conditions with shared state (static:: variables, table commands)
   - Denial of service vectors (unbounded loops, expensive operations in hot events)
   - Header injection possibilities (user data in HTTP::header insert/replace)
   - Open redirect vulnerabilities (HTTP::redirect with user-controlled values)
   - Session handling issues
6. For each finding, explain the risk and suggest a fix

## Output format

### Static Analysis Findings
(from the review tool output)

### Deep Analysis
(your security analysis beyond what static analysis can detect)

- Group findings by severity (critical, high, medium, low)
- For each finding, include: description, affected line(s), risk level, recommended fix

$ARGUMENTS
