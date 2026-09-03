---
name: f5-query
description: "Translate a natural-language question about an F5 BIG-IP configuration into the jq-flavoured `f5 query` DSL and run it. Use when the user asks anything like 'find all virtuals in /Common with default pool', 'rename pool X to Y across this config', 'which iRules reference data-group Z', 'list profiles attached to vs1', 'find unreferenced objects', or any other read-only or mutating query against a `bigip.conf` / `.scf` file. The DSL is jq-shaped on purpose so jq idioms transfer; the divergences live in `docs/references/f5_query/dsl.md` and `docs/references/f5_query/builtins.md`."
allowed-tools: Bash, Read
---

# F5 BIG-IP Query (natural language → DSL)

Turn the user's question into an `f5 query` expression and run it against
the file(s) they name.

## Workflow

1. **Input.** `.conf` and `.scf` load directly; `.ucs` needs
   `f5 ucs-extract` first. No file named → ask.
2. **Translate** from the recipes below; grammar in
   `docs/references/f5_query/dsl.md` (`f5 query --help-dsl`), every builtin
   in `docs/references/f5_query/builtins.md` (`f5 query --help-builtins
   [NAME]`), cookbook via `--help-examples`.
3. **Run.** Read-only by default. A mutating query (`=`, `+=`, `-=`, `|=`,
   `rename`, `rename_partition`) prints a unified diff; `--write` sends the
   rewritten config to stdout, `--in-place` overwrites the source.
4. **Answer** from the output: `--raw` one value per line, `--json`
   structured, `--paths-only` full paths, default renders SCF stanzas for
   object streams.

## DSL primer

jq's model: `.` is the current value, `.foo.bar` reads fields, `.X[]`
streams a container, `|` pipes, `[ ... ]` collects a stream into a list for
aggregators (`[.X[].name] | sort | first`), bare builtins act on the current
value, `select(pred)` filters, `map(body)` transforms. Builtins include
`count`, `unique`, `sort`, `any`, `all`, `contains`, `startswith`,
`endswith`, `match` (boolean — jq's `test()`), `str`. `$name` is the root of
one loaded source (below); reads and writes both route to it.

| Divergence from jq | f5 query |
|---|---|
| function args | `,` separated, not `;` |
| stream concat `,` | absent — use `[ ... ]` lists or `;` statements |
| `test()` | `match()` is boolean; capture groups via `sub` / `gsub` |
| truthiness | empty string / list / stream / PathRef and numeric 0 are also falsey |
| object literals | `{name, dest: .destination}` (bareword key = `key: .key`; stream fields broadcast one row per item) |
| `expr as $x \| body` | supported — streams iterate, plain lists bind whole |
| string interpolation | not in v1 — concat with `+` (scalars auto-coerce) |
| `,` inside `[...]` | not in v1 — parse error names the comma |

## Multi-config queries

Several positional configs are auto-bound by filename stem (`ltm.conf` →
`$ltm`, `/path/bigip-tier2.conf` → `$bigip-tier2`); `--name N=PATH`
overrides. Default mode runs the query once per source with `.` bound to
each; `$name` reaches across files, but `refs` / `referenced_by` stay
scoped to the originating source. `--merge` makes every source one
namespace: `.ltm.virtual[]` spans all inputs, `refs` cross files, edits
route back; it refuses when two sources define the same `(kind, full-path)`.

| User asks | Query |
|---|---|
| list virtuals from ltm.conf with gtm.conf loaded | `$ltm.ltm.virtual[].name` |
| every LTM pool a GTM pool references | `--merge .gtm.pool[] \| refs(.)` |
| rename a pool in tier1 only | `$tier1.ltm.pool["/Common/old"].name = "/Common/new"` |
| every object across both tiers | `--merge .ltm[][].name` |

## Network probes and cert audit

Live checks need `--enable-probes`. Every cert-emitting builtin returns the
same dict (`subject`, `issuer`, `serial`, `fingerprint_sha256`, `sans`,
`not_after`, `key_size`, `key_alg`, `version`) so any two compose with
`x509_eq(a, b)` (fingerprint first, then subject + issuer + serial):
`x509_parse(pem)`, `cert_load(path[, pw])` (PEM / DER / PKCS#12, a chain
comes back as `[leaf, ...]`), `x509_from_config(cert)` (`sys file ssl-cert`
or `cm cert`), `tls_handshake(host, port).peer_cert`,
`url_get(url).peer_cert`.

`url_get` / `url_head` / `tls_handshake` do not abort on cert errors: they
retry unverified so the audit still gets body and peer cert, and report
`reason.kind` (`ok`, `expired`, `not_yet_valid`, `self_signed`,
`hostname_mismatch`, `untrusted_ca`, `other_verification`,
`connection_error`), `reason.message` (OpenSSL text), and `reason.fatal`
(true only when nothing was captured).

| User asks | Query |
|---|---|
| expiry of every cert on the device | `.sys["file-ssl-cert"][] \| { name, expires: x509_from_config(.).not_after }` |
| certs expiring within 30 days | `.sys["file-ssl-cert"][] \| x509_from_config(.) \| select(.not_after < "2026-06-15") \| .subject` |
| expiry of every device-trust cert | `.cm.cert[] \| { name, expires: x509_from_config(.).not_after }` |
| does each VS serve the cert its client-ssl profile names | walk `.ltm.virtual[]` → `.profiles[]` → client-ssl → `cert-key-chain` → `sys file ssl-cert`, compare with `tls_handshake(host, port).peer_cert` via `x509_eq`; full pattern in [`kcs-howto-audit-server-certs-with-query.md`](../../../../docs/kcs/kcs-howto-audit-server-certs-with-query.md) |
| endpoints with an expired / self-signed cert | `.ltm.virtual[] \| { vs: .name, probe: tls_handshake(.destination.host, .destination.port) } \| select(.probe.reason.kind == "expired")` (or `"self_signed"`) |
| would this HTTP monitor mark the member up | `.ltm.monitor.http[$mon] as $m \| url_get($url + $m.send) \| .body[0:5120] \| test($m.recv)` — BIG-IP reads at most 5,120 bytes (K3451); full pattern in [`kcs-howto-reproduce-http-monitor-with-query.md`](../../../../docs/kcs/kcs-howto-reproduce-http-monitor-with-query.md) |

## Recipes

| Read-only | Query |
|---|---|
| every virtual server | `.ltm.virtual[].name` |
| pools in /Common | `.ltm.pool[] \| select(partition(."full-path") == "Common") \| .name` |
| virtuals using pool X | `.ltm.virtual[] \| select(.pool == "/Common/X") \| .name` |
| vs1's full config / its iRules | `.ltm.virtual["/Common/vs1"]` / `.ltm.virtual["/Common/vs1"].rules[]` |
| pools without monitors | `.ltm.pool[] \| select(.monitor == "") \| .name` |
| virtuals on port 443 | `.ltm.virtual[] \| select(endswith(.destination, ":443")) \| .name` |
| any iRule referencing the missing pool /Common/X | `any(.ltm.rule[].refs.pools[] \| (. == "/Common/X"))` |
| each pool's member count | `.ltm.pool[] \| .name + ": " + count(.members)` |
| persistence profiles inheriting from cookie | `.ltm.persistence[] \| select(."defaults-from" == "/Common/cookie") \| .name` |
| data-groups whose body contains 'foo' | `.ltm["data-group"][] \| select(contains(.records, "foo")) \| .name` |
| every kind of object | `[.[]] \| count` |
| unreferenced pools | use `f5 cleanup` |

| Mutating (preview the diff first) | Query |
|---|---|
| rename pool old → new | `rename("/Common/old", "/Common/new")` |
| move every object /Tenant_A → /Tenant_B | `rename_partition("Tenant_A", "Tenant_B")` |
| set vs1's description | `.ltm.virtual["/Common/vs1"].description = "production HTTPS"` |
| attach iRule X to every VS in /Common | `.ltm.virtual[] \| select(partition(."full-path") == "Common") \| .rules += "/Common/X"` |
| remove iRule Y from vs1 | `.ltm.virtual["/Common/vs1"].rules -= "/Common/Y"` |
| rename pool to match its destination | not scriptable — `rename` per VS |

Flags: `--raw`, `--paths-only`, `--json` (multi-file: one envelope
`[{"uri", "values"}, ...]`), `--scf`, `--format scf|tmsh` (`tmsh` emits a
`tmsh modify` script; refused with `--in-place`, which always preserves the
source format and reads strict UTF-8), `--write`, `--in-place`,
`--from-file FILE`.

## Gotchas

1. `map(body)` is many-to-many and flattens like the pipe;
   `map(select(...) | .field)` is the filter+transform idiom.
2. No `,` inside `[ ... ]` — one pipeline expression per bracket.
3. `--in-place --format tmsh` is refused; use `--write` for tmsh.
4. Field-edit strings are SCF-encoded on write; newlines and braces raise an
   `EditError`.
5. Multi-file output carries `# === <uri> ===` banners except under
   `--json`.

## Defer to

`f5 cleanup` (orphans, reverse-topological), `f5 lint` (BIG-IP rule
findings), `f5 grep` (transitive reference graph), `f5 diff`
(round-trip-aware whole-config diff), `f5 trace` / `f5 explain` (ad-hoc Tcl
analysis).

## Etiquette

Show the exact query you ran; dry-run every mutation and show the diff
before suggesting `--in-place`; summarise long results with counts and a
sample; say explicitly when a query returns nothing and suggest the next
step (a partition filter, usually).
