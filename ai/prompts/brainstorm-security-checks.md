# Brainstorm: Six New Automatic Security Checks

Status: proposal
Date: 2026-02-28

## Context

The LSP already has strong coverage of Tcl injection patterns (W100–W309,
T100–T106) and iRules-specific taint sinks (IRULE3001–IRULE3004, IRULE3101–
IRULE3102).  The checks below target gaps that remain — credential hygiene,
channel safety, timing oracles, scope escapes, file-system abuse, and SSRF.

---

## 1. W310 — Hardcoded credentials in `socket`, `http::geturl`, and SMTP helpers

**What it catches:**
String literals that look like passwords, API keys, or tokens passed directly
to network-facing commands — `socket`, `http::geturl -headers`, `smtp::sendmessage`,
or any `HTTP::header insert Authorization` in iRules.

**Detection approach (syntactic, single-pass check):**
Scan arguments of network/auth-related commands for braced or quoted string
literals that match high-entropy patterns (base64 blocks, hex runs ≥ 32 chars,
`password`, `secret`, `bearer`, `basic` followed by a base64 blob).  Flag
when the value is a literal, *not* a `$variable` or `[command]` substitution —
the point is to catch secrets baked into source.

**Why it matters:**
Secrets committed in iRules or Tcl scripts are the #1 post-incident finding
in F5 config reviews.  Even a low-confidence heuristic catches obvious
offenders before they reach version control.

**Diagnostic severity:** WARNING

**iRules variant (IRULE3005):**
Additionally flag `HTTP::header insert Authorization "Basic …"` or
`HTTP::cookie insert` with a literal value that looks like a session token.

---

## 2. W311 — Unsafe channel encoding (`fconfigure`/`chan configure -encoding binary`)

**What it catches:**
Setting a channel to `-encoding binary` or `-translation binary` then using
`gets`/`puts` for text I/O — a latent bug that silently corrupts multibyte
characters and can enable encoding-differential attacks (e.g. overlong UTF-8
smuggling past validation).

**Detection approach (taint-phase, SSA-aware):**
Track channel variables through `open`/`socket` → `fconfigure $ch -encoding binary`
→ subsequent `gets $ch` / `puts $ch`.  When the channel is configured for binary
but consumed by a text-oriented command, emit the warning.  Also flag the
inverse: binary data (from `read -nonewline`) piped into a channel still in
`utf-8` encoding, which can trigger `Tcl_ExternalToUtf` errors at runtime.

**Why it matters:**
Encoding mismatches are a classic source of bypass vulnerabilities (WAF evasion,
XML external entity expansion).  In iRules, `TCP::payload` is already binary,
so mixing it with `HTTP::header insert` without re-encoding is a real-world
bug class.

**Diagnostic severity:** WARNING

---

## 3. T104 — Tainted data in `socket` host/port (SSRF)

**What it catches:**
Server-side request forgery — when user-controlled input flows into the host
or port argument of `socket`, `http::geturl`, or iRules `connect` /
`HTTP::redirect` to a back-end origin.

**Detection approach (taint-phase):**
Add `socket` host (arg index 0–1 depending on options) and `http::geturl`
URL (arg 0) as taint sinks.  The existing colour lattice already provides
the right suppression: values proven to be `IP_ADDRESS`, `PORT`, or `FQDN`
from an allowlist lookup would have those colours and suppress the warning
automatically.

**iRules variant (IRULE3006):**
Flag `connect` and `sideband::open` when host/port arguments carry generic
taint.  These are the primary mechanisms for iRules-initiated back-end
connections.

**Why it matters:**
SSRF is consistently in the OWASP Top 10.  Tcl scripts used as web
back-ends (or iRules that proxy to internal services) can reach internal
metadata endpoints or admin interfaces when the destination is attacker-
controlled.

**Diagnostic severity:** WARNING

---

## 4. W312 — `interp eval` / `interp invokehidden` with tainted arguments

**What it catches:**
Code injection across interpreter boundaries — when `interp eval $child $script`
or `interp invokehidden` passes a dynamically constructed script into a child
interpreter, the safe-interp sandbox is only as strong as the sanitisation of
that script.

**Detection approach (syntactic check + taint sink):**

- **Syntactic (W312):** In the single-pass checker, flag `interp eval` and
  `interp invokehidden` where the script argument contains `$` or `[`
  substitutions (same pattern as `check_uplevel_injection`).
- **Taint (T105):** Register `interp eval` script-position and
  `interp invokehidden` command-position as taint sinks so the colour-aware
  analysis can track whether user data flows into a child interpreter.

**Why it matters:**
`interp eval` is the cross-interpreter equivalent of `eval` — it has the
same injection risks but currently gets no dedicated diagnostic.  Developers
often assume the child interp is sandboxed, but the script itself is
constructed in the *parent* and can carry attacker payloads.

**Diagnostic severity:** WARNING

---

## 5. IRULE3007 — Timing-sensitive comparison of secrets

**What it catches:**
Using `eq`, `==`, or `string equal` to compare a tainted value (e.g.
`HTTP::header Authorization`, `HTTP::cookie`) against a literal or
`static::` secret.  String comparison in Tcl short-circuits on the first
differing byte, leaking information about the secret via timing.

**Detection approach (taint-phase + syntactic heuristic):**

1. In the taint pass, detect `expr { $tainted_var eq $secret }` or
   `string equal $tainted_var $static_value` where one operand is tainted
   and the other is a literal or `static::` variable.
2. The syntactic layer also flags `if { [HTTP::header Authorization] eq "…" }`
   patterns directly, since these are extremely common in iRules auth checks.

**Suggested fix (quick-fix action):**
Offer to wrap the comparison in a constant-time helper:
```tcl
# constant-time compare (iRules)
proc ::ct_compare {a b} {
    if {[string length $a] != [string length $b]} { return 0 }
    binary scan $a c* al; binary scan $b c* bl
    set diff 0
    foreach x $al y $bl { set diff [expr {$diff | ($x ^ $y)}] }
    return [expr {$diff == 0}]
}
```

**Why it matters:**
Timing attacks on authentication tokens are practical even over a network
when the comparison runs inside an F5 BIG-IP data plane processing millions
of requests per second.  A constant-time comparison is the standard
mitigation.

**Diagnostic severity:** WARNING (HINT in plain Tcl dialect, promoted to
WARNING in `f5-irules` where the pattern is actionable)

---

## 6. W313 — `file delete` / `file rename` / `file mkdir` with tainted path

**What it catches:**
Destructive file-system operations where the path argument is derived from
untrusted input without normalisation — enabling path-traversal attacks that
delete, overwrite, or create files outside the intended directory.

**Detection approach (taint-phase):**
Register `file delete`, `file rename` (both source and destination), and
`file mkdir` path arguments as taint sinks.  Suppress when the path carries
the `PATH_NORMALISED` colour (set by `file normalize` or `file join` with a
constant root prefix).  The existing `W201` check catches *string*
concatenation of paths, but this new check covers the case where a variable
carrying tainted data is passed directly to a destructive file command without
any concatenation at all.

**Syntactic companion (W313):**
In the single-pass checker, flag `file delete $var` and `file rename $var …`
where the path argument contains a `$` or `[` substitution and the command
lacks a prior `file normalize` call on the same variable.  This is the
destructive-file analogue of `check_source_variable` (W300).

**Why it matters:**
While `source` with a variable path is already flagged (W300), destructive
operations like `file delete` can cause denial of service or data loss and
deserve their own diagnostic.  In EDA tool flows (where Tcl scripts manage
build artefacts), an attacker-controlled filename can delete critical outputs.

**Diagnostic severity:** WARNING

---

## Implementation priority

| Priority | Check | Effort | Value |
|----------|-------|--------|-------|
| 1 | T104 — SSRF in `socket`/`http::geturl` | Low | High |
| 2 | W312/T105 — `interp eval` injection | Low | High |
| 3 | W313 — destructive file ops with tainted path | Medium | High |
| 4 | W310 — hardcoded credentials | Medium | High |
| 5 | IRULE3007 — timing-sensitive comparison | Medium | Medium |
| 6 | W311 — channel encoding mismatch | High | Medium |

Checks 1–2 are straightforward extensions of existing patterns (add taint
sinks, mirror `check_uplevel_injection`).  Check 3 adds new taint sinks to
`file` subcommands.  Check 4 requires a regex heuristic for credential
patterns.  Check 5 needs SSA-level detection of comparison operands.  Check 6
requires tracking channel state across multiple statements.
