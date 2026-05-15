# KCS: How do I audit BIG-IP server certs against the live endpoints they protect with `f5 query`?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

I have hundreds of `sys file ssl-cert` entries across one or more
BIG-IPs.  How do I confirm the cert installed on each device
matches the cert each virtual server is actually serving — and
how do I find devices where a cert *should* have been pushed but
the update never landed?

## Before you start

- One or more `bigip.conf` / SCF files (per device or merged).
- Network reach to the virtuals you want to probe.
- `f5 query --enable-probes` so the network builtins (`url_get`,
  `tls_handshake`) are unlocked.

## Answer

Every cert source in `f5 query` ships the same dict shape:
`subject` / `issuer` / `serial` / `fingerprint_sha256` / `sans` /
`not_after` / `key_size` / `key_alg` / `version`.  That lets you
compare a cert from any source against a cert from any other
source via the `x509_eq` builtin, which collapses field-by-field
noise (different ISO time offsets, missing `not_before` on the
device side) into pure cert-identity semantics.

The four cert sources `f5 query` recognises:

| Source | Builtin | Shape |
|---|---|---|
| `sys file ssl-cert` on the device | `x509_from_sys_file(cert)` | parsed dict |
| Live TLS handshake | `tls_handshake(host, port).peer_cert` | parsed dict |
| Any HTTPS endpoint | `url_get(url).peer_cert` | parsed dict |
| PEM / DER / PKCS#12 on disk | `cert_load(path[, password])` | parsed dict |

Every one of these is pre-parsed — no `x509_parse` step required
once you have the source.  Pass any two of them to `x509_eq(a, b)`
and you get a boolean.

### Verify the cert each VS is serving matches the cert in `sys file ssl-cert`

For a single device, walk every virtual that uses a client-SSL
profile, fetch its certificate via `tls_handshake`, and assert
it matches the cert the device file references:

```
f5 query --enable-probes '
  .ltm.virtual[]
  | select(.profiles[] | select(.context == "clientside") | true)
  | { name,
      dest: .destination,
      device_cert: (
        .profiles[] | select(.context == "clientside")
        | .full-path | .ltm.profile."client-ssl"[.]
        | ."cert-key-chain"[0].cert | .sys."file-ssl-cert"[.]
        | x509_from_sys_file(.)),
      live_cert: tls_handshake(.destination.host, .destination.port).peer_cert,
      matches: false }
  | .matches = x509_eq(.device_cert, .live_cert)
' --json bigip.conf
```

Each row of output is one virtual.  `matches == false` means the
running cert is *not* the cert the device thinks it has — the
device probably reloaded against a stale source or the file lives
on disk but didn't get installed cleanly.

### Find every device in a multi-tier deployment where a cert push failed

Load every device's SCF as a named source (`--name`), run the
same comparison, and filter to the rows where the live cert
disagrees with the file cert:

```
f5 query --enable-probes \
  --name dc1=dc1-bigip.conf \
  --name dc2=dc2-bigip.conf \
  --name dc3=dc3-bigip.conf \
  '
  ($dc1, $dc2, $dc3) as $dev
  | $dev.ltm.virtual[]
  | select(.profiles[] | select(.context == "clientside") | true)
  | {
      device: ($dev | kind),
      vs: .name,
      file_cert: (
        .profiles[] | select(.context == "clientside")
        | .full-path | $dev.ltm.profile."client-ssl"[.]
        | ."cert-key-chain"[0].cert | $dev.sys."file-ssl-cert"[.]
        | x509_from_sys_file(.)),
      live_cert: tls_handshake(.destination.host, .destination.port).peer_cert
    }
  | select(x509_eq(.file_cert, .live_cert) == false)
  | { device, vs,
      file_serial: .file_cert.serial,
      live_serial: .live_cert.serial,
      live_expires: .live_cert.not_after,
      reason: ("file cert serial " + .file_cert.serial
               + " does not match live serial " + .live_cert.serial) }
' --json
```

Output is the punch list: every (device, VS) pair where the cert
update didn't take effect.  Pair with `f5 grep` to confirm the
`sys file ssl-cert` block on disk actually changed in the latest
config sync, or pipe to the network-ops Slack channel.

### Just find every expired cert across the fleet

`tls_handshake` and `url_get` collect data even when the cert
fails strict verification — the `reason.kind` field tags WHY
verification failed, and `reason.fatal` is true only for
connection-level errors.  That lets you query for expired certs
specifically:

```
f5 query --enable-probes '
  .ltm.virtual[]
  | select(.profiles[] | select(.context == "clientside") | true)
  | { vs: .name,
      probe: tls_handshake(.destination.host, .destination.port) }
  | select(.probe.reason.kind == "expired")
  | { vs,
      expired_on: .probe.peer_cert.not_after,
      subject: .probe.peer_cert.subject,
      sans: .probe.peer_cert.sans }
' --json bigip.conf
```

The same pattern works for `self_signed` (devices stuck on the
default `default.crt`), `hostname_mismatch` (cert installed on the
wrong virtual), and `untrusted_ca` (intermediate / root chain
isn't present on the device).

## Operational context

### Reason taxonomy

Every TLS-touching builtin emits a `reason` dict with three
fields:

- **`kind`** — `"ok"`, `"expired"`, `"not_yet_valid"`,
  `"self_signed"`, `"hostname_mismatch"`, `"untrusted_ca"`,
  `"other_verification"`, `"connection_error"`.
- **`message`** — the OpenSSL verification text (e.g. `"unable to
  get local issuer certificate"`) so you can copy/paste it
  verbatim when filing tickets.
- **`fatal`** — `true` only when the connection itself didn't
  complete (DNS failure, refused, timeout).  Verification
  failures are `false` because the cert data is still available
  for inspection.

### Reading the comparison rules

`x509_eq` compares certs by identity, in this order:

1. `fingerprint_sha256` — the canonical X.509 identity.  When both
   sides have it, that's the only check.
2. `subject` + `issuer` + `serial` — the X.509-defined primary
   key.  Used when one side is a `sys file ssl-cert` (the TMSH
   surface doesn't carry a SHA-256 fingerprint).

Plain `==` on the dicts is stricter — it compares every field
including `not_before` and `sig_alg`, which the device side
leaves `null`.  Use `x509_eq` for "same cert" semantics, `==` for
"identical projection".

### When to use which source

| Question | Source |
|---|---|
| "What does the device think is installed?" | `x509_from_sys_file` against `sys file ssl-cert` |
| "What is the device actually serving?" | `tls_handshake` to the virtual's destination |
| "What does this PEM on disk contain?" | `cert_load("/path/to/cert.pem")` |
| "What cert is a peer endpoint using?" | `url_get(url).peer_cert` |

## Related

- [`kcs-howto-find-objects-by-query.md`](kcs-howto-find-objects-by-query.md) — for the base query patterns.
- [`f5-query-dsl-builtins.md`](../references/f5_query/builtins.md) — full builtins reference (`tls_handshake`, `x509_from_sys_file`, `x509_eq`, `cert_load`, …).
