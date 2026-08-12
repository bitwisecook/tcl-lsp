# F5 KB articles cited by the `f5 query` monitor docs

External references for the F5 Knowledge articles that the `f5
query` HOWTOs and the `f5-query` skill point at.  Centralised here
so the operational guidance can link once rather than restating
the title each time.  All articles are on `my.f5.com`; the slugs
shown are the canonical short ID F5 search recognises.

## HTTP / HTTPS monitor — request format

- **K2167** — *Constructing HTTP requests for use with the HTTP
  or HTTPS application health monitor.*  Send-string format, CR/LF
  translation (`\r` → 0x0d, `\n` → 0x0a), HTTP-version defaults
  (0.9 unless overridden in the send string; 1.0 when basic-auth
  is enabled), the auto-appended terminating CR/LF, and the
  basic-auth / NTLM Authorization-header injection rules.
- **K10655** — *CR/LF characters appended to the HTTP monitor
  Send string.*  Deep dive on the auto-appended `\r\n\r\n`
  behaviour, including the cases where it produces a malformed
  request (basic-auth + body, NTLM + body).
- **K000148880** — *Creating HTTP Monitors for Backend Servers
  using HTTP CONNECT on F5 BIG-IP.*  The CONNECT-method monitor
  pattern (`CONNECT host:443 HTTP/1.1\r\nHost: host\r\nConnection:
  Close\r\n\r\n`) for monitoring explicit-proxy pool members.

## HTTP / HTTPS monitor — response handling

- **K3451** — *Content length limits for HTTP and HTTPS health
  monitors.*  The canonical reference for the **5,120-byte**
  response-read ceiling.  The cap includes the HTTP response
  headers, so a chunky header bag eats into the body window.
  Documented remediation when you need to match deeper: an
  External monitor (EAV) whose limit is set by the external
  program (cURL or similar) rather than the device.
- **K3224** — *HTTP health checks may fail even though the node
  is responding correctly.*  Catalogues the three most common
  "server looks fine but the device marks it down" failure modes:
  receive-string-too-late (the K3451 5,120-byte ceiling in
  action), HTTP version mismatch (0.9 send string against an
  HTTP/1.1-only server), and the device not following meta-refresh
  / 30x redirects.

## HTTP / HTTPS monitor — TLS specifics

- **K16526** — *Configuring the SSL cipher strength for a custom
  HTTPS health monitor.*  Cipher-suite knobs on the
  `ltm monitor https` object.  Relevant when you're reproducing
  an HTTPS monitor and the device-side handshake fails for a
  reason `tls_handshake` doesn't see (the device uses a different
  OpenSSL build than your laptop).
- **K29224049** — *Overview of the BIG-IP HTTP/2 monitor.*  The
  `http2` / `http2_head_f5` monitors shipped from 15.1.0; they
  read response bytes the same way HTTP/1.x monitors do (K3451
  applies).

## Umbrella troubleshooting

- **K12531** — *Troubleshooting health monitors.*  Top-level
  guide: how to identify which monitor is marking a pool member
  down (logs, SNMP traps, `tmsh show /ltm pool`), monitor-logging
  per-pool-member, interval/timeout tuning rules, the `bigd`
  daemon's role, and `tcpdump` capture recipes.

## How `f5 query` honours these articles

The `f5 query` builtins that touch HTTP / HTTPS / TLS implement
the device-correct behaviour these articles describe:

| Article | Honoured by | How |
|---|---|---|
| K2167 (CR/LF translation) | `url_get`, `url_head`, `url_options` | The `ureq` HTTP client (Rust) performs the same `\r\n` byte handling natively. |
| K3451 (5,120-byte ceiling) | the audit recipes in `kcs-howto-reproduce-http-monitor-with-query.md` | Truncate `body[0:5120]` before testing `recv`. |
| K3224 (version mismatch / redirect) | `url_get` follows redirects by default; pin to `--no-follow-redirects` (future) for monitor parity. | Use `url_head` or check `.status` against 3xx. |
| K12531 (umbrella triage) | every probe builtin's `reason` field | The structured `reason.kind` mirrors the failure taxonomy K12531 walks through. |

## Related `f5 query` documentation

- [Comprehensive DSL grammar reference](dsl.md)
- [Full builtins reference](builtins.md)
- [How-to: reproduce an HTTP monitor with `f5 query`](../../kcs/kcs-howto-reproduce-http-monitor-with-query.md)
- [How-to: audit BIG-IP server certs with `f5 query`](../../kcs/kcs-howto-audit-server-certs-with-query.md)
