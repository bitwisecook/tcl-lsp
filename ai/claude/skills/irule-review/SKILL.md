---
name: irule-review
description: "Security and safety review of an F5 iRule. Combines LSP static analysis (security, taint, thread safety diagnostics) with deep analysis of input validation, information leakage, race conditions, DoS vectors, and conflicts with other iRules on the same virtual server. Use when reviewing iRule security, auditing F5 iRule safety, performing iRule penetration testing, or checking iRule code for vulnerabilities."
allowed-tools: mcp__tcl-lsp__review, Bash, Read
---

# iRule Security Review

## Steps

1. Read `../_prompts/irules_system.md`, then the iRule.
2. Call `mcp__tcl-lsp__review` with the contents as `source`. On a tool
   error report it and suggest fixes.
3. Present the static findings (security, taint, thread safety; codes in
   `docs/generated/diagnostic_codes.md`).
4. Go beyond them: unvalidated HTTP::uri / HTTP::query / HTTP::header /
   HTTP::cookie input; information leakage in logs or HTTP::respond bodies;
   races on static:: or table state; DoS vectors (unbounded loops, expensive
   work in hot events); header injection via user data in HTTP::header
   insert/replace; open redirects via HTTP::redirect; session handling;
   uncaught errors that reset the client.
5. Execution-model risks: logic that assumes `return`, a response or
   `event disable all` stops other iRules; responses without an
   `HTTP::has_responded` guard; generic connection variable names another
   iRule could share; per-request state carried across keep-alive requests;
   procs reaching the caller through `upvar`. With a BIG-IP config, review
   against each attached virtual server's HUD chain
   (`f5-query explain virtual VS CONF --json`) and suggest `/irule-hud` for
   the whole chain.
6. For each finding: description, affected lines, risk, recommended fix.

## Output

`### Static Analysis Findings` from the tool, then `### Deep Analysis`
grouped by severity (critical, high, medium, low).

$ARGUMENTS
