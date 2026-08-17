# `f5 query` — Comprehensive Reference Manual

The complete reference for the `f5 query` DSL, organised by topic
so individual items (an operator, a builtin family, a probe
behaviour, a KB cross-reference) can be looked up directly via
anchor.  Each section has a stable `{#anchor-id}` so external
tools (the MCP server, AI skills, IDE quick-lookups) can deep-link
into a single concept without dragging the whole document.

This manual is the canonical source.  The `f5 query --help-manual`
CLI flag emits the auto-generated grammar + builtins + examples
trio; this file is the curated long-form companion that adds the
operational context, the probe / audit taxonomy, and the F5 KB
cross-references that the auto-generated content can't carry.

## Quick lookup index

| Looking for | Section |
|---|---|
| Grammar / parser rules | [Grammar](#grammar-grammar) |
| Path access (`.foo.bar`, `[]`, subscripts) | [Path access](#path-access-path-access) |
| Streams vs lists | [Streams and lists](#streams-and-lists-streams-and-lists) |
| Pipe / assignment operators | [Operators](#operators-operators) |
| Variables and bindings | [Variables and let-bindings](#variables-and-let-bindings-variables-and-let-bindings) |
| Object literals | [Object construction](#object-construction-object-construction) |
| Conditionals and comma streams | [Control flow](#control-flow-control-flow) |
| All builtins (alphabetical) | [Builtin catalogue](#builtin-catalogue-builtin-catalogue) |
| Probes (`url_get`, `tls_handshake`, …) | [Network probes](#network-probes-network-probes) |
| Cert audit shape | [X.509 cert dict shape](#x509-cert-dict-shape-x509-cert-dict-shape) |
| Reason taxonomy on failed probes | [Reason taxonomy](#reason-taxonomy-reason-taxonomy) |
| Read-from-file inputs (JSON, JSONL, CSV, f5log) | [External inputs](#external-inputs-external-inputs) |
| TLS / monitor F5 KB articles | [F5 KB cross-reference](#f5-kb-cross-reference-f5-kb-cross-reference) |
| Output formats (raw / paths / json / scf / tmsh) | [Output modes](#output-modes-output-modes) |
| Mutating queries (assignments, renames) | [Mutating queries](#mutating-queries-mutating-queries) |
| Edit-plan semantics + apply order | [Edit planning](#edit-planning-edit-planning) |
| Worked example cookbook | [Examples](#examples-examples) |

## Grammar {#grammar}

`f5 query` is a jq-flavoured language whose grammar is documented
in detail in [`docs/references/f5_query/dsl.md`](dsl.md).
The shipped CLI emits the same grammar via `f5 query --help-dsl`.
Highlights below; see the linked reference for the full EBNF.

```
program     := pipeline (';' pipeline)*
pipeline    := comma_expr ('|' comma_expr)*
comma_expr  := pipe_stage (',' pipe_stage)*
pipe_stage  := or_expr (ASSIGN_OP pipe_stage)?
            |  or_expr 'as' '$' IDENT '|' pipeline
ASSIGN_OP   := '=' | '|=' | '+=' | '-='
primary     := literal | call | path | variable
            |  list_literal | object_literal | if_expr | '(' pipeline ')'
call        := IDENT ('(' (pipe_stage (',' pipe_stage)*)? ')')?
```

**Divergences from jq:** function arguments separated by `,` (not
`;`); assignment is a trailing operator on a pipe-stage, not a
top-level statement; identifiers may contain `-` so `.source-address-translation`
is a single field name without quoting.

## Path access {#path-access}

| Form | Meaning |
|---|---|
| `.` | Identity — the current value. |
| `.foo` | Field access. |
| `.foo.bar` | Nested field access; chained. |
| `.foo[]` | Stream every value of the container `foo`. |
| `.foo[0]` | Numeric index (zero-based) into a list. |
| `.foo["/Common/x"]` | Exact subscript by full-path. |
| `.foo["~^/Common/vs_"]` | Regex subscript (regex applied to full-path keys). |
| `.foo?` | Suppress missing-field errors; emit nothing if `foo` doesn't exist. |
| `.foo[]?` | Stream-safe variant — empty stream for missing containers. |
| `."hyphen-name"` | Quoted field for names that collide with operators. |
| `.ltm.virtual.web_vs` | Partition shorthand: bare names resolve to `/Common/<name>` when unambiguous. |

## Streams and lists {#streams-and-lists}

Pipelines iterate **streams**, not plain lists.  `[]` produces a
stream; stream-returning builtins do too.  Plain lists (the value
of `.rules`) pass through `|` whole.  To fold a stream into a
list — for aggregators like `sort`, `unique`, `count` over a
collection — wrap it: `[.ltm.virtual[].name] | sort`.

| Construct | Emits | Use case |
|---|---|---|
| `.x[]` | Stream | iterate values |
| `[.x[]]` | List | collect for aggregators |
| `[1, 2, 3]` | List literal | inline collection |
| `a, b, c` | Stream concat | jq's comma operator |
| `(.x, .y)` | Stream concat | parenthesised when used as one argument |

## Operators {#operators}

### Pipe and assignment

| Op | Effect |
|---|---|
| `|` | Pass each value on the left into the right. |
| `=` | Replace the LHS field with the RHS value. |
| `|=` | Replace the LHS with `RHS` evaluated against the LHS value. |
| `+=` | List append; string concat; numeric add. |
| `-=` | List remove; numeric subtract. |
| `;` | Sequence statements; each statement sees the SAME root. |

### Boolean / comparison

| Op | Effect |
|---|---|
| `and`, `or`, `not` | Logical connectives. |
| `==`, `!=`, `<`, `<=`, `>`, `>=` | Comparison (numeric, string, structural). |

### Arithmetic

| Op | Effect |
|---|---|
| `+`, `-`, `*`, `/` | Numeric arithmetic.  `+` doubles as string concat. |

## Variables and let-bindings {#variables-and-let-bindings}

```
.ltm.virtual[] as $vs | $vs.pool.members[] | $vs.name + " -> " + .name
```

- `$name` — the root container of a named source loaded with
  `--name N=PATH` (or auto-derived from filename stem).
- `as $x | body` — bind each value of the LHS to `$x` and evaluate
  `body` with that binding.  Right-associative.

## Object construction {#object-construction}

```
.ltm.virtual[] | { name, pool, destination }
```

- `{ k1: v1, k2: v2 }` — explicit entries.
- `{ name }` — shorthand for `{ name: .name }`.
- Stream-valued fields broadcast element-wise so one stream field
  becomes one row per item.

## Control flow {#control-flow}

```
.ltm.virtual[] |
  if .pool == "" then "no pool"
  elif .pool | startswith("/Common/") then "common"
  else "tenant"
  end
```

- `if cond then body [elif cond then body]* [else body] end` —
  stream-aware: when `cond` is a stream every item branches
  independently and the bodies are concatenated.
- `select(predicate)` — filter; emits the input when predicate
  is truthy, empty otherwise.
- `map(body)` — apply `body` to every item of a list, returning a
  list.

## Builtin catalogue {#builtin-catalogue}

The full alphabetical catalogue lives in the hand-maintained
[`docs/references/f5_query/builtins.md`](builtins.md), kept in sync
by hand against the registry in `rust/tcl-bigip-query/src/builtins/`.
`f5 query --help-builtins` emits a metadata-only summary from the
same registry (name / category / arity / flags, not the full prose).
Each builtin has its own anchor in `builtins.md` — to look one up:

- `f5 query --help-builtins NAME` — that builtin's metadata from the
  CLI.
- `docs/references/f5_query/builtins.md#NAME` — the full prose +
  examples, in the Markdown file.

Major families:

- **value introspection** — `kind`, `path`, `length`, `defined`, `type`, `has`, `in`, `to_entries`, `from_entries`, `with_entries`, `env`
- **tree manipulation** — `paths`, `leaf_paths`, `getpath`, `setpath`, `del`, `delpaths`, `pick`, `walk`, `recurse`, `recurse_down`, `until`, `repeat`
- **string** — `match` / `test`, `sub`, `gsub` (all flag-aware), `scan`, `capture`, `splits`, `startswith`, `endswith`, `contains`, `upcase` / `ascii_upcase`, `downcase` / `ascii_downcase`, `ltrimstr`, `rtrimstr`, `tonumber`, `tostring`, `tojson`, `fromjson`, `explode`, `implode`, `ascii`, `utf8bytelength`, `csv`, `tsv`, `join`, `split`
- **encoding** — `uri`, `base64`, `base64d`, `html`, `sh` (jq's `@`-prefix format strings as plain functions)
- **list / stream** — `count`, `unique`, `dupes`, `sort`, `sort_by`, `unique_by`, `group_by`, `min_by`, `max_by`, `min_max`, `max_min`, `first`, `last`, `nth`, `limit`, `reverse`, `flatten`, `range`, `add`, `min`, `max`, `any`, `all`, `map`, `map_values`, `select`, `keys`, `keys_unsorted`, `values`, `empty`, `error`, `not`, `inside`, `IN`, `INDEX`, `combinations`, `halt`, `halt_error`, `debug`, `stderr`
- **math** — `floor`, `ceil`, `round`, `trunc`, `rint`, `nearbyint`, `abs`, `fabs`, `copysign`, `fdim`, `sqrt`, `cbrt`, `pow`, `pow10`, `exp`, `exp2`, `exp10`, `expm1`, `log`, `log10`, `log2`, `log1p`, `logb`, `hypot`, `fma`, `fmax`, `fmin`, `fmod`, `remainder`/`drem`, `sin`/`cos`/`tan`/`asin`/`acos`/`atan`/`atan2`, `sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`, `gamma`/`tgamma`/`lgamma`/`lgamma_r`, `j0`/`j1`/`y0`/`y1`/`jn`/`yn`, `frexp`/`ldexp`/`modf`/`significand`, `nan`, `infinite`, `isnan`, `isinfinite`, `isnormal`
- **time** — `now`, `todate`/`todateiso8601`/`date`, `fromdate`/`fromdateiso8601`, `gmtime`, `localtime`, `mktime`, `strftime`, `strptime`, `dateadd`, `datesub`
- **network types** — `ip`, `port`, `host`, `net`, `in_cidr`, `ip_translate`, `ip_range_*`, `port_set_*`
- **graph** — `refs`, `referenced_by`
- **mutating** — `rename`, `rename_partition`, `rename_prefix`
- **HTTP-response helpers** — `http_ok`, `http_client_error`,
  `http_header`, `http_body_json`
- **network probes** (need `--enable-probes`) — `url_get`,
  `url_head`, `url_options`, `url_post` (not yet implemented —
  see [Network probes](#network-probes-network-probes)), `tls_handshake`, `ping`,
  `portping`, `traceroute`.  `dns` / `rev_dns` are ungated (they
  resolve without `--enable-probes`).
- **cert / X.509** — `x509_parse`, `cert_load`,
  `x509_from_config`, `x509_eq`
- **external inputs** — `json_load`, `jsonl_load`, `csv_load`,
  `f5log_load`

## Network probes {#network-probes}

Most probes need `f5 query --enable-probes` (the gate keeps
read-only queries from accidentally reaching out) — `dns` /
`rev_dns` are the exception, ungated as benign name resolution.
Probe results are **not** cached or memoised — each call is a fresh
network round-trip, so referencing the same probe repeatedly in one
query repeats the work.

**`url_get` / `url_head` / `url_options` / `url_post` are not
currently implemented.**  The live HTTP request path was deferred
(non-deterministic / not golden-testable); every call returns the
shape below with `status: null` and an explanatory `error`,
regardless of the target URL.  `tls_handshake`, `dns`, `rev_dns`,
`ping`, `portping`, and `traceroute` are fully implemented.

| Builtin | Returns |
|---|---|
| `url_get(url, [headers])` — *not implemented* | `{ status, headers, body, body_json, peer_cert, error }` |
| `url_head(url, [headers])` — *not implemented* | same shape, no body |
| `url_options(url, [headers])` — *not implemented* | same shape |
| `url_post(url, [body], [headers])` — *not implemented* | same shape — note the argument order differs from `url_get` / `url_head` / `url_options` |
| `tls_handshake(host, port, [sni])` | `{ protocol, cipher, peer_cert, alpn_selected, verify_status, reason, error }` |
| `dns(name)` | `list[string]` — sorted, unique A + AAAA addresses (not cached; see [`dns`](builtins.md#dns)) |
| `rev_dns(ip)` | `list[string]` — reverse-DNS name(s), best-effort |
| `ping(ip)` | `{ ok, rtt_ms, error }` |
| `portping(ip, port, [protocol])` | `{ ok, rtt_ms, error }` — *protocol* is `tcp` (default) or `udp` |

### Reason taxonomy {#reason-taxonomy}

Every HTTPS / TLS probe emits a structured `reason` field describing
the verification status.  Probes **do not** abort on cert errors —
they retry with verification disabled so the response body + peer
cert are always available.  Filter audit results on `reason.kind`:

| `reason.kind` | Meaning | `reason.fatal` |
|---|---|---|
| `ok` | Verified clean against the trust store. | `false` |
| `expired` | `X509_V_ERR_CERT_HAS_EXPIRED` (code 10). | `false` |
| `not_yet_valid` | `X509_V_ERR_CERT_NOT_YET_VALID` (code 9). | `false` |
| `self_signed` | `X509_V_ERR_DEPTH_ZERO_SELF_SIGNED_CERT` (codes 18 / 19). | `false` |
| `untrusted_ca` | Chain doesn't terminate at a trusted CA (codes 20 / 21 / 24). | `false` |
| `hostname_mismatch` | SNI / SAN doesn't match (code 62). | `false` |
| `other_verification` | Any other OpenSSL verify code. | `false` |
| `connection_error` | Connection couldn't complete (DNS, refused, timeout). | `true` |

`reason.message` is the OpenSSL verify-text verbatim so you can
file a ticket with it untouched.  `reason.fatal == true` is the
only case where the response body / peer cert may be absent.

## X.509 cert dict shape {#x509-cert-dict-shape}

Every cert-emitting builtin returns the same dict shape so
`x509_eq(a, b)` works across sources:

| Field | Type | Notes |
|---|---|---|
| `subject` | string | RFC-4514 format. |
| `issuer` | string | RFC-4514 format. |
| `serial` | string | Uppercase hex (no `0x` prefix). |
| `fingerprint_sha256` | string | Uppercase hex; canonical identity. |
| `sans` | list[string] | DNS / IP names; `DNS:` / `IP:` prefixes stripped. |
| `not_before` | string \| null | ISO-8601 UTC; null on BIG-IP file projection. |
| `not_after` | string \| null | ISO-8601 UTC. |
| `key_alg` | string \| null | `RSAPublicKey`, `EllipticCurvePublicKey`, … |
| `key_size` | int \| null | Modulus / curve bit count. |
| `sig_alg` | string \| null | `sha256WithRSAEncryption`, … |
| `version` | string \| null | `v1` / `v2` / `v3`. |
| `public_key_pem` | string \| null | Full PEM of the public key. |

`x509_eq(a, b)` compares by `fingerprint_sha256` first; falls
back to `subject` + `issuer` + `serial` when one side lacks the
SHA-256 hash (BIG-IP's TMSH surface).

### Cert source builtins

- `x509_parse(pem)` — parse a PEM string in memory.
- `cert_load(path[, password])` — read PEM / DER / PKCS#12
  (`.pem`, `.crt`, `.cer`, `.der`, `.pfx`, `.p12`) from disk.
- `x509_from_config(cert)` — project any BIG-IP config object
  that carries cert metadata into the same dict shape.  Works
  on `sys file ssl-cert` (cert / chain / bundle store, the
  target of every `cert-key-chain` and `ltm monitor https.cert`
  PathRef) and `cm cert` (device-trust certs, the target of
  `cm device.cert` / `cm trust-domain.ca-cert`).  For PathRef
  fields elsewhere (`ltm monitor https.cert`, `cm device.cert`,
  …), index into the referent first then project.  `sys crypto
  cert` is a minimal projection without cert metadata — load
  its PEM with `cert_load` instead.
- `tls_handshake(host, port).peer_cert` — capture during a live
  TLS handshake.
- `url_get(url).peer_cert` — always `null` today: the live HTTP
  request path is not yet implemented (see
  [Network probes](#network-probes-network-probes)); use `tls_handshake` for a
  live peer certificate.

## External inputs {#external-inputs}

Beyond `bigip.conf` / SCF files, `f5 query` reads structured
external sources that bind into the DSL as named roots:

| Builtin | Format |
|---|---|
| `json_load(path)` | Read a JSON file; return the parsed value. |
| `jsonl_load(path)` | Read a JSON-Lines file; return a list of values. |
| `csv_load(path[, options])` | Read a CSV; return a list of dicts keyed by header row. |
| `f5log_load(path)` | Parse an F5 `/var/log/ltm` style log; emit structured events. |

CLI form for binding to a named root:

```
f5 query --name dc1=dc1-bigip.conf --name pcap=net-pcap.json \
  '$dc1.ltm.virtual[] | $pcap.connections[. == .name]' \
  bigip.conf
```

## Output modes {#output-modes}

| Flag | Output |
|---|---|
| (default) | `auto` — JSON for objects, raw scalars otherwise. |
| `--raw` | One scalar per line; no quoting.  Refuses non-scalars. |
| `--paths-only` | Full-paths only (for piping into `xargs`). |
| `--json` | JSON document per file (no per-file banner). |
| `--scf` | Re-emit the rewritten config as SCF. |
| `--in-place` | Overwrite the input file. |
| `--write` | Print rewritten config to stdout. |
| `--format scf\|tmsh` | Pick the rendered output dialect. |

## Mutating queries {#mutating-queries}

`f5 query` is dry-run by default.  Mutating queries (`=`, `|=`,
`+=`, `-=`, `rename(...)`, `rename_partition(...)`, etc.) print
a unified diff; pass `--write` to commit to stdout or
`--in-place` to overwrite.

| Operator | Common use |
|---|---|
| `.x = ...` | Replace `.x` with the RHS. |
| `.x \|= f` | Replace `.x` with `f` evaluated against `.x`. |
| `.x += ...` | List append / string concat / numeric add. |
| `.x -= ...` | List remove / numeric subtract. |
| `rename(old, new)` | Rename one object + every reference. |
| `rename_partition(old, new)` | Cascade-rename every object in a partition. |
| `rename_prefix(old, new)` | Cascade-rename a prefix across the config. |

### Edit planning {#edit-planning}

Edits don't apply during evaluation.  Each `Assignment` node
emits an `EditOp` into the runner's edit plan; the runner applies
the collected ops AFTER evaluation finishes so an edit's view of
the world stays stable across the whole query.  Multi-statement
mutating queries (`a ; b ; c`) drive `evaluate_statement` one
statement at a time and apply each statement's edits before
moving to the next.

## F5 KB cross-reference {#f5-kb-cross-reference}

Linked reference doc — see
[`f5-kb-monitor-articles.md`](f5-kb-monitor-articles.md) for the
expanded form.  Quick lookup table:

| Article | Topic | Section |
|---|---|---|
| K2167 | HTTP monitor send-string format | request format |
| K3451 | HTTP monitor 5,120-byte response ceiling | response handling |
| K3224 | Receive string / version / redirect failures | response handling |
| K10655 | CR/LF behaviour on send strings | request format |
| K12531 | Top-level monitor troubleshooting | umbrella |
| K16526 | HTTPS monitor cipher strength | TLS specifics |
| K29224049 | HTTP/2 monitor (`http2`) overview | HTTP/2 |
| K000148880 | HTTP CONNECT-method monitors | CONNECT |

## Examples {#examples}

The cookbook ships separately at
[`docs/references/f5_query/builtins.md`](builtins.md)
(each builtin carries its own example block) and is also
available via `f5 query --help-examples`.  The KCS HOW-TOs cover
the long-form recipes:

- [Bulk readdress virtuals](../../kcs/kcs-howto-readdress-virtuals-with-query.md)
- [Migrate a partition](../../kcs/kcs-howto-migrate-partition-with-query.md)
- [Compose query streams](../../kcs/kcs-howto-compose-query-streams.md)
- [Audit a config](../../kcs/kcs-howto-audit-config-with-query.md)
- [Audit server certs](../../kcs/kcs-howto-audit-server-certs-with-query.md)
- [Reproduce an HTTP monitor](../../kcs/kcs-howto-reproduce-http-monitor-with-query.md)
- [Cross-config transforms](../../kcs/kcs-howto-cross-config-transforms-with-query.md)

## Related documents

- [`docs/references/f5_query/dsl.md`](dsl.md) — the
  full DSL grammar reference.
- [`docs/references/f5_query/builtins.md`](builtins.md)
  — hand-maintained catalogue of every builtin (one section per
  function, every one with its own anchor for direct linking).
- [`docs/references/f5_query/f5-kb-monitor-articles.md`](f5-kb-monitor-articles.md)
  — F5 KB articles cross-referenced by the cert / monitor probe
  guidance.
- the `f5-query` AI skill that turns natural-language questions into
  DSL queries (exposed as a native `tcl-mcp` MCP tool).

## Programmatic access

The same content backing every `--help-*` flag is reachable from the
CLI and from the native `tcl-mcp` MCP server:

- `f5 query --help-dsl` — full grammar reference.
- `f5 query --help-builtins [NAME]` — every builtin (or one named
  function), with category, arity, and dispatch flags — metadata
  only; for full signatures, prose, and examples see
  [`builtins.md`](builtins.md).
- `f5 query --help-examples` — the worked-example cookbook.
- `f5 query --help-manual` — the whole reference concatenated.

MCP / agent contexts enumerate every callable and its signature through
the `tcl-mcp` tool surface (which wraps the same registry), for
auto-completion or grounded answer-building.

## Operator handbook {#operator-handbook}

End-to-end recipes for reproducing a BIG-IP scenario on your
laptop so the `f5 query` audit queries have something real to
talk to.  Every recipe is self-contained — copy-paste, run,
query.

### Sample SCF / conf fragments {#sample-configs}

Drop these into a file and feed it to `f5 query` for any of the
examples below.  Every fragment is valid SCF syntax; combine them
in any order.

**Single virtual + pool + monitor + iRule:**

```
ltm node /Common/web1 { address 10.0.0.11 }
ltm node /Common/web2 { address 10.0.0.12 }
ltm pool /Common/web_pool {
    members {
        /Common/web1:80 { address 10.0.0.11 }
        /Common/web2:80 { address 10.0.0.12 }
    }
    monitor /Common/http
}
ltm monitor http /Common/http {
    defaults-from /Common/http
    interval 5
    timeout 16
    send "GET /health HTTP/1.0\r\n\r\n"
    recv "HTTP/1\\.[01] 200"
}
ltm rule /Common/log_request {
    when HTTP_REQUEST {
        log local0. "[HTTP::uri]"
    }
}
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5:443
    pool /Common/web_pool
    profiles {
        /Common/clientssl { context clientside }
        /Common/http { }
    }
    rules { /Common/log_request }
}
```

**SSL profile + sys file ssl-cert (for the cert-audit recipes):**

```
sys file ssl-cert /Common/example.crt {
    source-path "file:///config/ssl/ssl.crt/example.crt"
    subject "CN=www.example.com,O=Acme Corp"
    issuer "CN=Example CA,O=Example"
    subject-alternative-name "DNS:www.example.com,DNS:example.com"
    expiration-string "Jan 1 00:00:00 2099 GMT"
    fingerprint "SHA256/AB:CD:EF:12:34"
    key-size 2048
    key-type rsa-public
    serial-number 1002
    version 3
}
sys file ssl-key /Common/example.key {
    source-path "file:///config/ssl/ssl.key/example.key"
    key-size 2048
    key-type rsa-private
}
ltm profile client-ssl /Common/clientssl_example {
    defaults-from /Common/clientssl
    cert-key-chain {
        default { cert /Common/example.crt key /Common/example.key }
    }
}
```

**GTM wide-IP + pool (for cross-module recipes):**

```
gtm pool a /Common/web_gtm_pool {
    members { /Common/dc1-bigip:/Common/web_vs }
    monitor /Common/gateway_icmp
}
gtm wideip a /Common/www.example.com {
    pools { /Common/web_gtm_pool }
    aliases { www.example.com }
}
```

**Firewall rule-list (for security-* recipes):**

```
security firewall address-list /Common/web_servers {
    addresses { 10.0.0.0/24 10.0.1.0/24 }
}
security firewall rule-list /Common/allow_web {
    rules {
        allow_https {
            action accept
            ip-protocol tcp
            destination {
                address-lists { /Common/web_servers }
                ports { 443 }
            }
        }
    }
}
```

### Backend server one-liners {#backend-servers}

Spin a local backend so the network-probe recipes in the
[reproduce-http-monitor](../../kcs/kcs-howto-reproduce-http-monitor-with-query.md)
HOW-TO have something to talk to.

**Plain HTTP — Python stdlib:**

```sh
# Serves the current directory.  Hit Ctrl-C to stop.
python3 -m http.server 8080
```

```sh
# Custom response (200 with JSON body):
cat > /tmp/health-server.py <<'PY'
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(b'{"status":"ok","uptime_s":42}')
HTTPServer(("127.0.0.1", 8080), H).serve_forever()
PY
python3 /tmp/health-server.py
```

**HTTPS — `openssl s_server` (no real backend, just for TLS):**

```sh
# Listen on 8443 with the cert you just generated below.
openssl s_server -accept 8443 -cert server.crt -key server.key \
  -WWW -CAfile ca.crt
```

**HTTPS — `nginx` (drop-in proxy with TLS termination):**

```nginx
# /tmp/nginx.conf — minimal HTTPS reverse proxy listening on 8443.
events {}
http {
    server {
        listen       8443 ssl;
        ssl_certificate     /tmp/server.crt;
        ssl_certificate_key /tmp/server.key;

        location / {
            proxy_pass http://127.0.0.1:8080;
            proxy_set_header Host $host;
        }
    }
}
```

```sh
nginx -p /tmp/ -c /tmp/nginx.conf &
# Stop with: nginx -p /tmp/ -c /tmp/nginx.conf -s stop
```

### Generating certificates {#cert-generation}

For the cert-audit recipes you need PEM / PFX certs on disk that
`cert_load` / `x509_parse` / `tls_handshake` can chew through.

**Self-signed RSA cert with SANs (OpenSSL):**

```sh
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout server.key \
  -out server.crt \
  -days 365 \
  -subj "/CN=test.local/O=Lab" \
  -addext "subjectAltName=DNS:test.local,DNS:www.test.local,IP:127.0.0.1"
```

**Self-signed ECDSA cert (smaller key, modern curve):**

```sh
openssl req -x509 -newkey ec:<(openssl ecparam -name prime256v1) -nodes \
  -keyout server.key -out server.crt \
  -days 365 -subj "/CN=test.local"
```

**Bundle a cert + key as PKCS#12 (for `cert_load("bundle.pfx", "pw")`):**

```sh
openssl pkcs12 -export \
  -inkey server.key -in server.crt \
  -out bundle.pfx -passout pass:trustno1
```

**Chain PEM (leaf + intermediate + root, in that order):**

```sh
cat server.crt intermediate.crt root.crt > chain.pem
# cert_load("chain.pem") returns [leaf, intermediate, root]
```

**Inspect a cert from the shell (matches the `f5 query` shape):**

```sh
openssl x509 -in server.crt -noout -subject -issuer -dates -fingerprint -sha256
```

### Running a self-signed HTTPS server in Python {#python-https}

For the cert-audit `tls_handshake` recipes (`url_get` is not yet
implemented — see [Network probes](#network-probes-network-probes)):

```python
# https_server.py
import http.server, ssl
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain("server.crt", "server.key")
srv = http.server.HTTPServer(("127.0.0.1", 8443), http.server.SimpleHTTPRequestHandler)
srv.socket = ctx.wrap_socket(srv.socket, server_side=True)
srv.serve_forever()
```

```sh
python3 https_server.py &
# Now run the cert-audit recipes against https://127.0.0.1:8443
```

### End-to-end audit walkthrough {#walkthrough}

Put the pieces together to reproduce the
[cert-audit HOW-TO](../../kcs/kcs-howto-audit-server-certs-with-query.md):

```sh
# 1. Generate a cert
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout /tmp/test.key -out /tmp/test.crt \
  -days 365 -subj "/CN=test.local" \
  -addext "subjectAltName=DNS:test.local"

# 2. Spin a TLS server using it
python3 - <<'PY' &
import http.server, ssl
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain("/tmp/test.crt", "/tmp/test.key")
srv = http.server.HTTPServer(("127.0.0.1", 8443), http.server.SimpleHTTPRequestHandler)
srv.socket = ctx.wrap_socket(srv.socket, server_side=True)
srv.serve_forever()
PY
sleep 1

# 3. Build a tiny SCF with a matching sys file ssl-cert
cat > /tmp/lab.conf <<'SCF'
sys file ssl-cert /Common/test.crt {
    subject "CN=test.local"
    subject-alternative-name "DNS:test.local"
    expiration-string "Jan 1 00:00:00 2099 GMT"
    fingerprint "SHA256/AB:CD"
    key-size 2048
    key-type rsa-public
    serial-number 1
    version 3
}
SCF

# 4. Query: confirm the cert the server is serving matches what
#    the device thinks it has.
f5 query --enable-probes '
  x509_eq(
    .sys["file-ssl-cert"]["/Common/test.crt"] | x509_from_config(.),
    tls_handshake("127.0.0.1", 8443).peer_cert)
' /tmp/lab.conf
# => false (fingerprints differ — the SCF carries a placeholder
#    SHA256/AB:CD; the live cert has the real fingerprint).

# 5. Inspect the live cert's full shape:
f5 query --enable-probes '
  tls_handshake("127.0.0.1", 8443).peer_cert
' --json /tmp/lab.conf

# 6. Cleanup
kill %1
```

### Probe gating and CA bundles {#probe-controls}

| Behaviour | Knob |
|---|---|
| Gate (refuses by default) | `--enable-probes` — `dns` / `rev_dns` are ungated exceptions |
| Pin a CA bundle | `--ca-bundle /path/to/ca.crt` |
| Result caching | **none** — every probe call is a fresh network round-trip; nothing is cached or memoised across calls in a query |
| Cert-verification failures | reported, not retried insecurely — `tls_handshake` sets `verify_status` / `reason.kind` and still captures `peer_cert` when the handshake reached the certificate message |
| Per-call timeout | not configurable — `tls_handshake` hardcodes 5s connect/read/write timeouts; `ping` / `portping` hardcode 2s |

### Per-builtin lookup {#per-builtin-lookup}

To pull a single builtin's full documentation:

```sh
f5 query --help-builtins x509_parse
f5 query --help-builtins url_get
f5 query --help-builtins rename_partition
```

To dump every builtin at once (for piping into a doc generator
or model context):

```sh
f5 query --help-builtins
```

To see the full self-contained manual (grammar + every builtin +
every example concatenated with section banners):

```sh
f5 query --help-manual
```

The F5 KB cross-reference doc has no dedicated CLI help flag — read it
directly: [`f5-kb-monitor-articles.md`](f5-kb-monitor-articles.md).

## 100% coverage map {#coverage-map}

Every behaviour in the query engine has documentation; this table
is the source-of-truth index for what lives where.

| Behaviour | Canonical reference |
|---|---|
| Grammar (parser, precedence, EBNF) | [`dsl.md`](dsl.md) + `f5 query --help-dsl` |
| Every builtin with examples | [`builtins.md`](builtins.md) + `f5 query --help-builtins NAME` |
| jq divergences | [`dsl.md`](dsl.md) §"Divergences from jq" |
| Probe gate + reason taxonomy | [Reason taxonomy](#reason-taxonomy-reason-taxonomy) section above |
| Cert dict shape | [X.509 cert dict shape](#x509-cert-dict-shape-x509-cert-dict-shape) section above |
| Mutating-query apply order | [Edit planning](#edit-planning-edit-planning) section above |
| Stream vs list semantics | [Streams and lists](#streams-and-lists-streams-and-lists) section above |
| Multi-source / `$name` bindings | [Variables and let-bindings](#variables-and-let-bindings-variables-and-let-bindings) section above |
| Object literals | [Object construction](#object-construction-object-construction) section above |
| If / elif / else | [Control flow](#control-flow-control-flow) section above |
| External inputs (JSON / CSV / f5log) | [External inputs](#external-inputs-external-inputs) section above |
| Output rendering | [Output modes](#output-modes-output-modes) section above |
| End-to-end cookbook | [`builtins.md`](builtins.md) + KCS HOW-TOs |
| Operational recipes (setup, certs, servers) | [Operator handbook](#operator-handbook-operator-handbook) section above |
| F5 KB articles | [`f5-kb-monitor-articles.md`](f5-kb-monitor-articles.md) |
| AI / MCP integration | the native `tcl-mcp` MCP server (`f5-query` skill) |
| Programmatic / MCP access | [Programmatic access](#programmatic-access) section above |
