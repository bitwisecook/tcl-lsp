# KCS: How do I reproduce an `ltm monitor http(s)` from the device, including the 5,120-byte response-check ceiling?

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
- Network reach to the pool-member IP + port.
- A request tool: `curl`, or `nc`. `f5 query` reads the config; it
  does not make the request (its `url_*` builtins return an
  `error` field instead of a response).

## Answer

Two things the device handles for you have to be reproduced by hand:

1. **Send-string escape semantics.** TMSH-form `\r` is the CR
   byte (0x0d) and `\n` is the LF byte (0x0a). The device
   auto-appends a double CR/LF to the send string if it doesn't
   already terminate the headers, and inserts an `Authorization`
   header when basic-auth is enabled on the monitor. Full rules
   are in F5 KB **K2167** — also the reference for HTTP-version
   defaults (HTTP 0.9 unless the send string says otherwise;
   HTTP 1.0+ when basic-auth is enabled), CONNECT-method tunnels
   (K000148880), NTLM handshakes, and CR/LF handling edge cases
   (K10655).
2. **Response-check ceiling.** Per F5 KB **K3451** the HTTP /
   HTTPS monitor reads up to **5,120 bytes** of the response —
   and that ceiling **includes the response headers**, so the
   body window is smaller still. Anything past it is invisible to
   the monitor however much the server sent, which is the failure
   K3224 catalogues. The documented remediation is an **External
   monitor**, whose limit is set by the external program.

### 1. Read the monitor and its members out of the config

```
f5 query --json '
  . as $cfg
  | .ltm.pool["/Common/web_pool"] as $p
  | $cfg.ltm.monitor[$p.monitor] as $m
  | select($m.type == "http" or $m.type == "https")
  | { pool: $p.name, monitor: $m.name, type: $m.type,
      send: $m.send, recv: $m.recv, disable: $m."recv-disable",
      members: [ $p.members[].name ] }
' bigip.conf
```

```json
[
  {
    "pool": "web_pool",
    "monitor": "http_health",
    "type": "http",
    "send": "GET /health HTTP/1.0\\r\\n\\r\\n",
    "recv": "ok health",
    "disable": "maintenance",
    "members": ["/Common/web1:8080"]
  }
]
```

Drop the `["/Common/web_pool"]` subscript to sweep every pool.
`.monitor` on a pool is the monitor's full path, and the HTTP or
HTTPS flavour is the monitor's `type` field.

### 2. Fire the same request and truncate the response

Send the `send` string verbatim, with the escapes expanded, and cut
the reply to 5,120 bytes before you test `recv`:

```sh
printf 'GET /health HTTP/1.0\r\n\r\n' \
  | nc 10.0.1.10 8080 \
  | head -c 5120 \
  | grep -E 'ok health' && echo UP || echo DOWN
```

`head -c 5120` is the whole point: it is the same window the device
evaluates, headers included. A pattern that only matches without it
is a monitor that will mark a healthy member **down**.

`recv-disable` is a second pattern under the same ceiling; when it
matches, the member is forced down whatever `recv` says.

## How to tell it worked

The truncated response matches `recv` exactly when the device says
the member is up, and fails to match when the device says it is
down. If the full response matches but the truncated one does not,
the `recv` pattern lives past the ceiling — move to an External
monitor (K3451).

## Operational context

`recv` is a POSIX regular expression on the device. `grep -E` is
close but not identical, and neither is `f5 query`'s `test(...)`,
which uses the Rust `regex` crate: it rejects backreferences and
look-behind that some POSIX tools accept. Pin patterns to plain
POSIX and they behave the same everywhere.

## Related

- [`kcs-howto-audit-server-certs-with-query.md`](kcs-howto-audit-server-certs-with-query.md) — the cert half of the same investigation.
- [`builtins.md`](../references/f5_query/builtins.md) — the builtin catalogue.
- F5 KB:
  - **K2167** — Constructing HTTP requests for use with the HTTP or HTTPS application health monitor (send-string format, CR/LF, basic-auth + NTLM).
  - **K3451** — Content length limits for HTTP and HTTPS health monitors (the 5,120-byte ceiling).
  - **K3224** — HTTP health checks may fail even though the node is responding correctly.
  - **K12531** — Troubleshooting health monitors (umbrella guide).
  - **K10655** — CR/LF characters appended to the HTTP monitor Send string.
  - **K000148880** — Creating HTTP Monitors for Backend Servers using HTTP CONNECT on F5 BIG-IP.
