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

A virtual's `profiles` entries are raw strings, and a built-in
client-SSL profile has no `ltm profile client-ssl` stanza to resolve
against. Collect the client-SSL stanza paths, name the built-ins, and
keep a profile that matches either list:

```
f5 query --json '
  . as $cfg
  | [ $cfg.ltm.profile[] | select(.type == "ProfileType.CLIENT_SSL") | ."full-path" ] as $ssl
  | [ $cfg.ltm.profile[]."full-path" ] as $known
  | [ "/Common/clientssl", "/Common/clientssl-insecure-compatible",
      "/Common/clientssl-secure", "/Common/splitsession-default-clientssl",
      "/Common/wom-default-clientssl", "/Common/crypto-server-default-clientssl" ] as $builtin
  | .ltm.virtual[] as $v
  | $v.profiles[] as $ref
  | sub($ref, " .*$", "") as $name
  | select(contains($ssl, $name)
           or contains($builtin, $name)
           or ((contains($known, $name) | not) and endswith($name, "clientssl")))
  | { vs: $v.name, host: host($v.destination), port: port($v.destination),
      profile: $name,
      cert: (if contains($known, $name) then $cfg.ltm.profile[$name].cert else "(device default)" end) }
' bigip.conf
```

```json
[
  {
    "vs": "www_vs",
    "host": "10.0.0.1",
    "port": 443,
    "profile": "/Common/my_clientssl",
    "cert": "/Common/default.crt"
  },
  {
    "vs": "legacy_vs",
    "host": "10.0.0.2",
    "port": 443,
    "profile": "/Common/clientssl",
    "cert": "(device default)"
  }
]
```

The filter has three arms: a profile whose stanza projects as
`ProfileType.CLIENT_SSL`, one of the built-in names, and any name with
no stanza that ends in `clientssl`. `legacy_vs` above needs the second
arm — it attaches `/Common/clientssl`, which ships with the device and
has no stanza, so resolving the reference against `.ltm.profile` and
selecting on `.type` reads `null` and drops the virtual. Subscripting
`.ltm.profile` with that path is an error outright, which is why the
`cert` lookup is guarded by `$known`. A
built-in carries no `cert` in the config; the device serves its own
default, so step 2 is the only way to read it.

`host(...)` and `port(...)` split the destination — `.destination` is
a string, so `.destination.host` is an error.

### 2. Fetch what each endpoint is actually serving

`tls_handshake(host, port[, sni])` returns the negotiated protocol,
cipher, ALPN selection, peer certificate, and a `reason` dict:

```
f5 query --enable-probes --json '
  . as $cfg
  | [ $cfg.ltm.profile[] | select(.type == "ProfileType.CLIENT_SSL") | ."full-path" ] as $ssl
  | [ $cfg.ltm.profile[]."full-path" ] as $known
  | [ "/Common/clientssl", "/Common/clientssl-insecure-compatible",
      "/Common/clientssl-secure", "/Common/splitsession-default-clientssl",
      "/Common/wom-default-clientssl", "/Common/crypto-server-default-clientssl" ] as $builtin
  | .ltm.virtual[] as $v
  | $v.profiles[] as $ref
  | sub($ref, " .*$", "") as $name
  | select(contains($ssl, $name)
           or contains($builtin, $name)
           or ((contains($known, $name) | not) and endswith($name, "clientssl")))
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
  . as $cfg
  | cert_load("/etc/pki/app.pem") as $baseline
  | [ $cfg.ltm.profile[] | select(.type == "ProfileType.CLIENT_SSL") | ."full-path" ] as $ssl
  | [ $cfg.ltm.profile[]."full-path" ] as $known
  | [ "/Common/clientssl", "/Common/clientssl-insecure-compatible",
      "/Common/clientssl-secure", "/Common/splitsession-default-clientssl",
      "/Common/wom-default-clientssl", "/Common/crypto-server-default-clientssl" ] as $builtin
  | .ltm.virtual[] as $v
  | $v.profiles[] as $ref
  | sub($ref, " .*$", "") as $name
  | select(contains($ssl, $name)
           or contains($builtin, $name)
           or ((contains($known, $name) | not) and endswith($name, "clientssl")))
  | tls_handshake(host($v.destination), port($v.destination)) as $tls
  | select(x509_eq($baseline, $tls.peer_cert) == false)
  | { vs: $v.name,
      baseline_serial: $baseline.serial,
      live_serial: $tls.peer_cert.serial,
      reason: $tls.reason.kind }
' bigip.conf
```

Every row is an endpoint serving something other than the cert you
pushed. The client-SSL filter from step 1 is what keeps that true: a
plain-HTTP virtual has no peer cert to compare, so probing one reports
a mismatch that is really an absence. Filter first, then probe.

## How to tell it worked

Step 1 lists a row per client-SSL virtual, `cert` naming the configured
certificate or reading `(device default)` for a built-in profile. A
virtual you expect to see and do not attaches a profile that is neither
a projected client-SSL stanza nor a recognised built-in name — read its
`profiles` and add the name to `$builtin`. With
`--enable-probes`, step 2 fills in `subject` and `expires` from the live
handshake, and `verify` reads `ok` for an endpoint whose chain and
hostname check out.

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
| "What cert is this virtual configured with?" | `.ltm.profile[…].cert`, empty for a built-in profile |
| "What is the device actually serving?" | `tls_handshake(host, port)` |
| "What does this PEM on disk contain?" | `cert_load("/path/to/cert.pem")` |

## Related

- [`kcs-howto-find-objects-by-query.md`](kcs-howto-find-objects-by-query.md) — the base query patterns.
- [`builtins.md`](../references/f5_query/builtins.md) — full builtins reference (`tls_handshake`, `x509_eq`, `cert_load`, …).
