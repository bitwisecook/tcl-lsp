# KCS: feature — BIG-IP report Security tab

> **Audience:** User
> **Type:** Functionality

## Summary

A **Security** tab in the standalone BIG-IP HTML report that runs a small,
documented set of offline, high-confidence security-posture checks against a
config export or UCS backup — factory/default credentials, default or weak
SNMP community strings, disabled/weak password-policy enforcement, plaintext
(unencrypted) secrets, unprotected private-key material, and non-administrative
accounts with full shell access — and lists each as a structured, stable-id
finding with a severity, a status, and remediation guidance.

## Applies to

tcl-lsp CLI

## Question

What does the report's Security tab check, and how do I read its findings?

## How to use

Generate a report as usual (`bigip-report-gen device.ucs -o report.html`, or
the in-browser generator). The **Security** tab always appears — even a bare,
LTM-only `bigip.conf` shows it, so you can see *what wasn't checked* and why,
not just what was found.

Each finding has:

- an **id** (e.g. `BIGIP-SEC-001`) — stable across reports, so you can track a
  specific check over time or link to it from a ticket;
- a **severity** — `critical` / `high` / `medium` / `low` / `info`;
- a **status**:
  - **confirmed** — the issue is present; act on it;
  - **clear** — the check ran and found nothing;
  - **not applicable** — there was nothing to check (e.g. no `root` account
    in this source at all);
  - **could not inspect** — the material is present but not in a form this
    generator can verify (e.g. a password hash in an unsupported scheme);
- **evidence** — a short, human-readable explanation, and **source** — the
  object it's about (e.g. `auth user /Common/admin`);
- **remediation** — what to do about it, plus a reference link when an
  authoritative one is confidently known.

Detection is entirely **passive and offline**: nothing in this tab
authenticates to the device or makes a network request, and no password,
hash, salt, master key, private key, or other decrypted secret value is ever
shown in a finding — only object names, field names, and counts.

## What each check looks for

| Id | Check | Confirmed when |
|---|---|---|
| `BIGIP-SEC-001` | Factory/default credentials | the stored `root` or `admin` password hash verifies against the known default (`default` / `admin`) |
| `BIGIP-SEC-002` | Default/weak SNMP community | an `sys snmp` community's `community-name` is a well-known default (`public`, `private`) |
| `BIGIP-SEC-003` | Weak password policy | `auth password-policy` has enforcement disabled, or a minimum length below a conservative baseline |
| `BIGIP-SEC-004` | Plaintext secret | a credential-bearing field BIG-IP normally encrypts under the unit master key is stored in clear text |
| `BIGIP-SEC-005` | Exposed private key | a private-key file in the UCS filestore carries no passphrase-protection marker |
| `BIGIP-SEC-006` | Shell access review | a non-administrative account has `shell bash` (full OS access) |

## Example

An estate with a device still on `admin`/`admin` and a `public` SNMP
community shows:

```
BIGIP-SEC-001  critical  confirmed  Default account credential detected
  the stored `admin` password verifies against the factory default
  source: auth user /Common/admin

BIGIP-SEC-002  medium    confirmed  Default SNMP community string
  community `comm-public` uses the default string "public" with ro access
  source: sys snmp communities comm-public
```

A clean device with a custom SNMP community and a strong password policy
shows the same rules with `clear` status instead — the tab reports both
outcomes, not just alarms.

## Default-credential detection, in a bit more detail

`root`/`default` and `admin`/`admin` are checked against every stored
password representation this generator can verify **without ever calling the
platform `crypt(3)`** (so the native binary, the wasm in-browser generator,
and any future backend agree byte-for-byte):

- `$6$…` SHA-512-crypt and `$5$…` SHA-256-crypt — what a current BIG-IP
  (TMOS 11+) stores;
- `$1$…` MD5-crypt — older estates and migrated accounts.

A hash in an unsupported or unrecognised scheme (classic 13-character Unix
DES-crypt with no `$` prefix, bcrypt, Apache's `$apr1$`, or anything
malformed) is reported **could not inspect**, never guessed at and never
confirmed or cleared — "fail closed". A locked account (`*`, `!`, `!!`) or
one with no password field at all is **not applicable**.

`root` is an OS account, not a `tmsh` object — its credential only appears in
a UCS's `/etc/shadow`, so this check needs a UCS backup (not a bare
`bigip.conf`) to inspect it at all; without one, `root` shows **not
applicable** with that reason.

## Limitations

- **`sys sshd` (remote root / SSH access) and `sys httpd` (management GUI
  TLS/HTTP settings) are not yet checked.** The generated BIG-IP object model
  keeps both as untyped records that drop every property, and the Python
  generator that would normally add the missing fields has been retired from
  this branch (see `AGENTS.md`) — there's no generator left to run. This is
  tracked as a follow-up (either restore codegen for these two kinds or add
  hand-maintained parsing).
- A **partial `bigip.conf`** (no `auth password-policy` block, no UCS
  filestore, no `/etc/shadow`) naturally shows more **not applicable**/
  **could not inspect** results — the report is telling you what it
  *couldn't* check, which is expected, not a bug.
- The **Security tab is Rust-only for now** (native CLI and the wasm
  in-browser generator share the exact same code, so those two always agree).
  `rust/bigip-report-gen/python/python/f5report/report.py` is a parallel,
  hand-maintained Python port of the *rest* of the model — kept only as "the
  PyO3/Python demonstration of driving the query engine as a library" per
  `AGENTS.md`'s "Python has been fully retired on this branch" — and was not
  given a matching Security tab in this change. Porting it (including a
  from-scratch Python MD5-crypt/SHA-crypt implementation, since the platform
  `crypt(3)` module is explicitly out of scope for the same reason it is
  here) is a follow-up if the Python demo needs to stay at parity.
- The report's query console still embeds the entire raw source text
  verbatim (for live queries), the same way it always has for other tabs
  (e.g. the Secrets tab's own reveal-behind-a-button values) — so a stored
  password hash is not literally absent from the HTML file. What this feature
  guarantees is narrower and still meaningful: the *Security tab's own
  findings* never carry the stored hash, salt, candidate password, or any
  other secret value — only the pass/fail verdict.

## Why it is built this way

Findings are declared as a small rule table (one stable id + check function
per row, in `rust/bigip-report-gen/rust/src/security.rs`) rather than
one-off logic scattered through the report — the same shape
`crate::forensics`'s ATT&CK checklist uses. Fields the generated BIG-IP model
doesn't carry (the real SNMP `community-name` value, an `auth user`'s `role`)
are read directly off the config text with the same low-level block/property
helpers the certificates and secrets tabs already use, rather than guessing
from a stanza label or hand-extending a `@generated` model file with no
generator left to keep it honest.
