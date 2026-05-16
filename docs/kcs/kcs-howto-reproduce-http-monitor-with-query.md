# KCS: How do I reproduce an `ltm monitor http(s)` from the device using `f5 query`, including the 5,120-byte response-check ceiling?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

A pool member is being marked **down** by an `ltm monitor http`
(or `https`) and I want to reproduce what the device sees from my
laptop — the exact same `send` string, the exact same `recv`
match, and the exact same response-size ceiling — without
SSHing into the device.

## Before you start

- A `bigip.conf` / SCF file containing the monitor and the pool
  member you want to investigate.
- Network reach to the pool-member IP + port from where you're
  running `f5 query`.
- `f5 query --enable-probes` (network builtins are gated).

## Answer

To reproduce a BIG-IP HTTP / HTTPS monitor faithfully you need
to know two things the device handles for you:

1. **Send-string escape semantics.**  TMSH-form `\r` is the CR
   byte (0x0d) and `\n` is the LF byte (0x0a).  The device
   auto-appends a double CR/LF to the send string if it doesn't
   already terminate the headers, and inserts an `Authorization`
   header when basic-auth is enabled on the monitor.  Full rules
   are in F5 KB **K2167** ("Constructing HTTP requests for use
   with the HTTP or HTTPS application health monitor") — also
   the reference for HTTP-version defaults (HTTP 0.9 unless the
   send string explicitly specifies otherwise; HTTP 1.0+ when
   basic-auth is enabled), CONNECT-method tunnels (K000148880),
   NTLM handshakes, and CR/LF handling edge cases (K10655: "CR/LF
   characters appended to the HTTP monitor Send string").
2. **Response-check ceiling.**  Per F5 KB **K3451** ("Content
   length limits for HTTP and HTTPS health monitors") the HTTP /
   HTTPS monitor reads up to **5,120 bytes** of the HTTP response
   — and that ceiling **includes the response headers**, so the
   body window is even smaller.  Anything past the 5,120-byte
   ceiling is invisible to the monitor regardless of whether the
   server actually sent it, which is exactly the failure K3224
   ("HTTP health checks may fail even though the node is
   responding correctly") catalogues.  The documented remediation
   is an **External monitor** (cURL-based or similar) whose limit
   is set by the external program rather than the device.  When
   you're chasing a "the server is up, why is the monitor down?"
   mystery, K3451 + K3224 + the umbrella troubleshooting guide
   **K12531** ("Troubleshooting health monitors") are the first
   stop.

Reproducing the monitor in `f5 query` means honouring both
rules: translate `\r` / `\n` the same way the device does on the
way out, then truncate the response to 5,120 bytes (headers +
body together) on the way back before testing the `recv` regex.

The `f5 query` recipe below pulls the monitor's `send` / `recv` /
`recv-disable` / `interval` from the parsed config, fires the
request via `url_get`, then runs the same regex match against the
first 5,120 bytes of the response body.

### Walk every HTTP / HTTPS monitor on a pool and compare

```
f5 query --enable-probes '
  .ltm.pool[]
  | { pool: .name, members: .members[].full-path, mon: .monitor.monitors[] }
  | .mon as $monpath
  | (.ltm.monitor.http[$monpath] // .ltm.monitor.https[$monpath]) as $m
  | select($m != null)
  | { pool, member: .members,
      send: $m.send, recv: $m.recv,
      probe: url_get(
        ("http" + (if $m | kind | contains("https") then "s" else "" end)
         + "://" + .members + $m.send)) }
  | { pool, member,
      sent: .send, expected_recv: .recv,
      truncated_body: (.probe.body[0:5120]),
      match: (.probe.body[0:5120] | test(.expected_recv)),
      status: .probe.status,
      reason: .probe.reason.kind }
' --json bigip.conf
```

Each row tells you:

- `truncated_body` — the same byte window the device evaluates
  (`body[0:5120]`).
- `match` — `true` iff the `recv` regex hits inside that 5,120-byte
  window.  Pool members where `match == false` would be marked
  **down** by the monitor.
- `reason.kind` — `"ok"` / `"self_signed"` / `"expired"` /
  `"hostname_mismatch"` / `"untrusted_ca"` / `"connection_error"`,
  matching the verification-status taxonomy `url_get` / `tls_handshake`
  emit.  Use `select(.reason.kind == "ok")` to filter to clean
  probes only.

### Reproduce a single monitor against a single member

When you already know which monitor and member to test:

```
f5 query --enable-probes '
  .ltm.monitor.http["/Common/http_health"] as $m
  | url_get("http://10.0.0.42:8080" + $m.send) as $probe
  | { sent: $m.send,
      recv: $m.recv,
      response: ($probe.body[0:5120]),
      device_would_mark_up: ($probe.body[0:5120] | test($m.recv)),
      status: $probe.status,
      reason: $probe.reason.kind }
' --json bigip.conf
```

Output (example):

```json
{
  "sent": "GET /health HTTP/1.0\\r\\n\\r\\n",
  "recv": "HTTP/1\\\\.[01] 200",
  "response": "HTTP/1.1 200 OK\\r\\nContent-Type: application/json\\r\\n...",
  "device_would_mark_up": true,
  "status": 200,
  "reason": "ok"
}
```

When `device_would_mark_up` is `false` and `status` looks healthy,
the `recv` regex usually doesn't match what the server is actually
returning — that's a config drift, not a server outage.

### Find every monitor whose `recv` pattern depends on bytes past 5,120

Some monitors look for content the server emits after the
5,120-byte ceiling (a sitemap, a large JSON listing, a sentinel
deep in an HTML page).  These monitors fail silently — the device
marks the member down even though the response is healthy.  This
query flags every member whose full response contains the `recv`
match but the truncated 5,120-byte window does **not**:

```
f5 query --enable-probes '
  .ltm.monitor.http[] as $m
  | select($m.recv != "" and $m.recv != null)
  | { mon: $m.name, recv: $m.recv,
      probe: url_get("http://" + .destination + $m.send) }
  | { mon, recv,
      hit_full: (.probe.body | test(.recv)),
      hit_truncated: (.probe.body[0:5120] | test(.recv)) }
  | select(.hit_full == true and .hit_truncated == false)
' --json bigip.conf
```

Every row is a monitor whose `recv` pattern is unreachable from
the device's 5,120-byte read window.  The documented remediation
is an **External monitor** — see K3451 — whose limit is set by
the external program (cURL, etc.) rather than the device.

## Operational context

### Why 5,120 bytes?

The HTTP / HTTPS monitor reads up to **5,120 bytes** of the HTTP
response into its internal buffer.  That cap **includes the
response headers** — a chunky `Server`, `Set-Cookie`, and CORS
header bag can eat several hundred bytes of the window before the
body even starts.  Documented in F5 KB **K3451** ("Content length
limits for HTTP and HTTPS health monitors").

When you need to match deeper into the response, the documented
path is an External monitor — a custom EAV that shells out to
cURL or a similar tool whose own buffer governs the read.  See
the External-monitors section of the BIG-IP LTM configuration
guide and DevCentral's "LTM External Monitors: The Basics".

### Why `recv` is a regex

The `recv` field is treated as a POSIX regular expression — `test(...)`
in `f5 query` uses Python's regex engine which is a strict
superset, so what tests positive in `f5 query` will also match on
the device.  The opposite is **not** true: Python supports
features (named groups, look-behind) that the device rejects.
Pin patterns to plain POSIX when you can.

### What about `recv-disable`?

`recv-disable` is a second regex; when it matches, the member is
forced **down** regardless of the `recv` outcome.  Same 5,120-byte
ceiling applies.  Replace `$m.recv` with `$m."recv-disable"` in
the queries above to audit disable patterns.

### Reproducing HTTPS monitors

Swap `url_get` for `url_get` on an `https://` URL — `f5 query`
captures the peer cert and verification status in the same
response object (see
[`kcs-howto-audit-server-certs-with-query.md`](kcs-howto-audit-server-certs-with-query.md)).
`reason.kind == "self_signed"` typically means the monitor is
configured against a backend with a self-signed cert; that's a
soft failure unless the monitor's `cert` / `ca-file` overrides
explicitly trust it.

## Related

- [`kcs-howto-audit-server-certs-with-query.md`](kcs-howto-audit-server-certs-with-query.md) — cert audit across the fleet.
- [`f5-query-dsl-builtins.md`](../references/f5_query/builtins.md) — `url_get`, `test`, monitor walkers.
- F5 KB:
  - **K2167** — Constructing HTTP requests for use with the HTTP or HTTPS application health monitor (send-string format, CR/LF, basic-auth + NTLM).
  - **K3451** — Content length limits for HTTP and HTTPS health monitors (the 5,120-byte ceiling).
  - **K3224** — HTTP health checks may fail even though the node is responding correctly (the receive-string-too-late / version-mismatch / redirect cases).
  - **K12531** — Troubleshooting health monitors (umbrella guide).
  - **K10655** — CR/LF characters appended to the HTTP monitor Send string.
  - **K000148880** — Creating HTTP Monitors for Backend Servers using HTTP CONNECT on F5 BIG-IP.
