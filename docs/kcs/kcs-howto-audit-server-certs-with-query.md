# KCS: How do I audit the certs my virtual servers serve with `f5 query`?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

I have hundreds of client-SSL virtual servers across one or more
BIG-IPs. How do I find which cert each one is configured with, check
what it is actually serving, and spot the endpoints where a cert push
never landed?

## Before you start

- One or more `bigip.conf` / SCF files (per device or merged).
- Network reach to the virtuals you want to probe.
- `--enable-probes`, which unlocks `tls_handshake`. Without it the
  probe builtins raise rather than touching the network, so an offline
  audit stays offline.

## Answer

### 1. List every client-SSL virtual and the cert it is configured with

A virtual's `profiles` entries are raw strings, so resolve each one
against `.ltm.profile` and keep the client-SSL ones:

```
f5 query --json '
  . as $cfg
  | .ltm.virtual[] as $v
  | $v.profiles[] as $ref
  | $cfg.ltm.profile[sub($ref, " .*$", "")] as $prof
  | select($prof.type == "ProfileType.CLIENT_SSL")
  | { vs: $v.name, host: host($v.destination), port: port($v.destination),
      profile: $prof.name, cert: $prof.cert }
' bigip.conf
```

```json
[
  {
    "vs": "www_vs",
    "host": "10.0.0.1",
    "port": 443,
    "profile": "my_clientssl",
    "cert": "/Common/default.crt"
  }
]
```

`host(...)` and `port(...)` split the destination — `.destination` is
a string, so `.destination.host` is an error.

### 2. Fetch what each endpoint is actually serving

`tls_handshake(host, port[, sni])` returns the negotiated protocol,
cipher, ALPN selection, peer certificate, and a `reason` dict:

```
f5 query --enable-probes --json '
  . as $cfg
  | .ltm.virtual[] as $v
  | $v.profiles[] as $ref
  | $cfg.ltm.profile[sub($ref, " .*$", "")] as $prof
  | select($prof.type == "ProfileType.CLIENT_SSL")
  | tls_handshake(host($v.destination), port($v.destination)) as $tls
  | { vs: $v.name,
      subject: $tls.peer_cert.subject,
      expires: $tls.peer_cert.not_after,
      sans: $tls.peer_cert.sans,
      verify: $tls.reason.kind }
' bigip.conf
```

Filter with `select(.verify == "expired")` for expired certs, or
`"self_signed"`, `"hostname_mismatch"`, `"untrusted_ca"` for the
other verification outcomes. Pass a trust anchor with `--ca-bundle`
and an SNI name as `tls_handshake(host, port, "name")` when the
default trust store or hostname is not what the client would use.

### 3. Compare the served cert against a known-good PEM

`cert_load(path[, password])` reads a PEM, DER, or PKCS#12 file from
disk into the same dict shape, and `x509_eq` compares two of them by
identity rather than field-by-field:

```
f5 query --enable-probes --json '
  cert_load("/etc/pki/app.pem") as $baseline
  | .ltm.virtual[] as $v
  | tls_handshake(host($v.destination), port($v.destination)) as $tls
  | select(x509_eq($baseline, $tls.peer_cert) == false)
  | { vs: $v.name,
      baseline_serial: $baseline.serial,
      live_serial: $tls.peer_cert.serial,
      reason: $tls.reason.kind }
' bigip.conf
```

Every row is an endpoint serving something other than the cert you
pushed.

## Operational context

### Reason taxonomy

Every TLS-touching builtin emits a `reason` dict with three fields:

- **`kind`** — `"ok"`, `"expired"`, `"not_yet_valid"`,
  `"self_signed"`, `"hostname_mismatch"`, `"untrusted_ca"`,
  `"other_verification"`, `"connection_error"`.
- **`message`** — the verification text, so you can quote it
  verbatim in a ticket.
- **`fatal`** — `true` only when the connection itself did not
  complete (DNS failure, refused, timeout). Verification failures are
  `false`, because the cert data is still there to inspect.

### How `x509_eq` decides

1. `fingerprint_sha256` — the canonical X.509 identity. When both
   sides carry it, that is the only check.
2. `subject` + `issuer` + `serial` — the X.509 primary key, used when
   one side has no fingerprint.

Plain `==` on the dicts is stricter: it compares every field,
including ones a projection leaves `null`. Use `x509_eq` for "same
cert", `==` for "identical projection".

### Which source answers which question

| Question | Source |
|---|---|
| "What cert is this virtual configured with?" | `.ltm.profile[…].cert` |
| "What is the device actually serving?" | `tls_handshake(host, port)` |
| "What does this PEM on disk contain?" | `cert_load("/path/to/cert.pem")` |

## Related

- [`kcs-howto-find-objects-by-query.md`](kcs-howto-find-objects-by-query.md) — the base query patterns.
- [`f5-query-dsl-builtins.md`](../references/f5_query/builtins.md) — full builtins reference (`tls_handshake`, `x509_eq`, `cert_load`, …).
