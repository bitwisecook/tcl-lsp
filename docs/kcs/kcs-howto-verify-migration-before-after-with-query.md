# KCS: How do I verify a migration looks the same before and after with `f5 query`?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

I am moving a tenant onto new hardware (or a new BIG-IP version). I
have a UCS from the old box and a UCS from the new box. How do I
confirm — straight from the archives — that the IPs, self-IP
lockdowns, monitors, and certificates did not drift, and then probe
the live VIPs to prove they still listen, still serve the same cert,
and still answer `GET /` the same way?

## Before you start

- A UCS (or `bigip.conf` / SCF) from each side: `old.ucs` and
  `new.ucs`. Encrypted archives are fine — see
  [reading an encrypted UCS](kcs-howto-read-encrypted-ucs-archives.md).
- For the live probes: network reach to the VIPs **from where you run
  the command**, and `--enable-probes` (probes are gated off by
  default so an offline audit never touches the network).

## Answer

`f5 query` loads both archives in one invocation and binds each to a
`$`-variable, so the before/after comparison lives in the query — no
shell `diff`, no temp files. Name them so the variables are stable:

```
--name old=old.ucs --name new=new.ucs old.ucs new.ucs
```

### Config parity, with a match column

One query reads both archives, joins each object by its full path, and
prints `OK` / `FAIL` per object across virtuals, self-IPs, monitors,
and certificates:

```
f5 query --name old=old.ucs --name new=new.ucs --table '$old
  | [ $old.ltm.virtual[]."full-path" ]          as $vok
  | [ $old.net.self[]."full-path" ]             as $sok
  | [ $old.ltm.monitor[]."full-path" ]          as $mok
  | [ $old.sys["file-ssl-cert"][]."full-path" ] as $cok
  | ( $new.ltm.virtual[] as $n | ($n."full-path") as $fp | select(contains($vok,$fp))
      | $old.ltm.virtual[$fp] as $o
      | {check:"VIP", object:$fp, old:$o.destination, new:$n.destination,
         match:(if $o.destination==$n.destination then "OK" else "FAIL" end)} ),
    ( $new.net.self[] as $n | ($n."full-path") as $fp | select(contains($sok,$fp))
      | $old.net.self[$fp] as $o
      | {check:"self-ip", object:$fp, old:join($o."allow-service",","), new:join($n."allow-service",","),
         match:(if join($o."allow-service",",")==join($n."allow-service",",") then "OK" else "FAIL" end)} ),
    ( $new.ltm.monitor[] as $n | ($n."full-path") as $fp | select(contains($mok,$fp))
      | $old.ltm.monitor[$fp] as $o
      | {check:"monitor", object:$fp, old:($o.send+" => "+$o.recv), new:($n.send+" => "+$n.recv),
         match:(if tsv($o.send,$o.recv,$o.interval,$o.timeout)==tsv($n.send,$n.recv,$n.interval,$n.timeout) then "OK" else "FAIL" end)} ),
    ( $new.sys["file-ssl-cert"][] as $n | ($n."full-path") as $fp | select(contains($cok,$fp))
      | ucs_cert($old.sys["file-ssl-cert"][$fp]) as $o | ucs_cert($n) as $nn
      | {check:"cert", object:$fp, old:$o.fingerprint_sha256, new:$nn.fingerprint_sha256,
         match:(if x509_eq($o,$nn) then "OK" else "FAIL" end)} )' \
  old.ucs new.ucs | grep -v '^#'
```

```
+---------+----------------------+--------------------------+--------------------------+-------+
| check   | object               | old                      | new                      | match |
+---------+----------------------+--------------------------+--------------------------+-------+
| VIP     | /Common/app_http_vs  | /Common/203.0.113.10:80  | /Common/203.0.113.10:80  | OK    |
| VIP     | /Common/app_https_vs | /Common/203.0.113.10:443 | /Common/203.0.113.11:443 | FAIL  |
| self-ip | /Common/self-ext     | none                     | all                      | FAIL  |
| self-ip | /Common/self-int     | all                      | all                      | OK    |
| monitor | /Common/http_health  | GET / => 200             | GET / => 200             | OK    |
| cert    | /Common/app.crt      | 80083D43…A6DE8271        | 80083D43…A6DE8271        | OK    |
+---------+----------------------+--------------------------+--------------------------+-------+
```

The cert check uses **`ucs_cert`**, which reads the *real* PEM out of the
UCS filestore and parses it. That matters: a real `sys file ssl-cert`
stanza usually records only `cache-path` / `revision`, no fingerprint or
serial, so `x509_from_config` (stanza metadata) would compare two empty
projections. `ucs_cert` recovers the true identity, and `x509_eq`
compares it (by fingerprint, or subject + issuer + serial). Add
`| select(.match=="FAIL")` to show only the drift. Run single checks by
keeping just one arm of the query.

### Inventory parity (added or removed objects)

The join above compares objects present in **both** archives. To catch
objects that appeared or vanished, compare the key sets (an empty
result means the inventory matches):

```
f5 query --name old=old.ucs --name new=new.ucs --table '$old
  | [ $old.ltm.virtual[]."full-path" ] as $ok
  | [ $new.ltm.virtual[]."full-path" ] as $nk
  | ( $nk[] | select(contains($ok,.)|not) | {object:., status:"ADDED (not in old)"} ),
    ( $ok[] | select(contains($nk,.)|not) | {object:., status:"REMOVED (gone in new)"} )' \
  old.ucs new.ucs | grep -v '^#'
```

### Live external probes — does the migrated box behave the same?

Config parity proves the archives agree. The probes prove the running
box agrees with the outside world. Each VIP's address and port come
straight from the UCS; `--ca-bundle` makes TLS verification reflect
your trust chain. This report lists, per VIP, whether it listens, the
`GET /` status, and the served certificate:

```
f5 query --enable-probes --ca-bundle app-ca.pem --table '
  .ltm.virtual[]
  | (host(.destination)) as $h | (port(.destination)) as $p
  | (any(.profiles[] | .context=="clientside") or $p==443) as $tls
  | (if $tls then "https" else "http" end) as $scheme
  | (url_get($scheme + "://" + $h + ":" + ($p|tostring) + "/")) as $r
  | { vs:.name, target:($h+":"+($p|tostring)), scheme:$scheme,
      listening:(if portping($h,$p).ok then "UP" else "DOWN" end),
      "GET /":$r.status, verify:$r.reason.kind }
' new.ucs | grep -v '^#'
```

```
+--------------+-----------------+--------+-----------+-------+--------+
| vs           | target          | scheme | listening | GET / | verify |
+--------------+-----------------+--------+-----------+-------+--------+
| app_http_vs  | 127.0.0.1:28080 | http   | UP        | 200   | ok     |
| app_https_vs | 127.0.0.1:28443 | https  | UP        | 200   | ok     |
+--------------+-----------------+--------+-----------+-------+--------+
```

A virtual counts as HTTPS when it carries a clientside SSL profile *or*
listens on `443`. The port test matters: BIG-IP attaches a client-SSL
profile with an empty `{ }` body (no explicit `context`), so a
context-only predicate misses those VIPs and probes them as plain HTTP.
For TLS on a non-standard port, add it to the `$p==443` test.

To compare the two boxes in one shot, probe `$old`'s VIPs and `$new`'s
VIPs and match every externally visible signal — status, content type,
body size, TLS protocol, and the served cert (`x509_eq`):

```
f5 query --enable-probes --ca-bundle app-ca.pem --name old=old.ucs --name new=new.ucs --table '$old
  | [ $old.ltm.virtual[]."full-path" ] as $ok
  | $new.ltm.virtual[] as $nv | ($nv."full-path") as $fp | select(contains($ok,$fp))
  | $old.ltm.virtual[$fp] as $ov
  | (any($nv.profiles[] | .context=="clientside") or port($nv.destination)==443) as $tls
  | (if $tls then "https" else "http" end) as $sch
  | (url_get($sch+"://"+host($ov.destination)+":"+(port($ov.destination)|tostring)+"/")) as $or
  | (url_get($sch+"://"+host($nv.destination)+":"+(port($nv.destination)|tostring)+"/")) as $nr
  | (if $tls then tls_handshake(host($ov.destination),port($ov.destination),"app.example.com") else null end) as $ot
  | (if $tls then tls_handshake(host($nv.destination),port($nv.destination),"app.example.com") else null end) as $nt
  | { vs:$fp, scheme:$sch,
      status:(if $or.status==$nr.status then "OK "+($nr.status|tostring) else "FAIL "+($or.status|tostring)+"->"+($nr.status|tostring) end),
      ctype:(if http_header($or,"content-type")==http_header($nr,"content-type") then "OK" else "FAIL" end),
      body_len:(if ($or.body|length)==($nr.body|length) then "OK" else "FAIL" end),
      tls_proto:(if $tls then (if $ot.protocol==$nt.protocol then "OK "+$nt.protocol else "FAIL" end) else "n/a" end),
      cert:(if $tls then (if x509_eq($ot.peer_cert,$nt.peer_cert) then "OK" else "FAIL" end) else "n/a" end),
      cert_valid:(if $tls then $nt.reason.kind else "n/a" end) }
' old.ucs new.ucs | grep -v '^#'
```

```
+----------------------+--------+--------+-------+----------+------------+------+------------+
| vs                   | scheme | status | ctype | body_len | tls_proto  | cert | cert_valid |
+----------------------+--------+--------+-------+----------+------------+------+------------+
| /Common/app_http_vs  | http   | OK 200 | OK    | OK       | n/a        | n/a  | n/a        |
| /Common/app_https_vs | https  | OK 200 | OK    | OK       | OK TLSv1.3 | OK   | ok         |
+----------------------+--------+--------+-------+----------+------------+------+------------+
```

A swapped certificate is the classic migration trap — the app still
answers `200`, but the cert column flips to `FAIL` because the two live
handshakes return different serials.

### Probe the VS cert and compare it to the cert in the UCS

The check above compares the two *live* boxes to each other. To tie the
running cert back to the **baseline** — does the new box serve the same
cert the old box's archive holds? — read the real cert out of `old.ucs`
with `ucs_cert` and compare it to the live handshake:

```
f5 query --enable-probes --ca-bundle app-ca.pem --name old=old.ucs --table '$old
  | $old.ltm.virtual[]
  | select(any(.profiles[] | .context=="clientside") or port(.destination)==443) as $vs
  | tls_handshake(host($vs.destination), port($vs.destination), "app.example.com").peer_cert as $live
  | ucs_cert($old.sys["file-ssl-cert"]["/Common/app.crt"]) as $baseline
  | { vs:$vs.name,
      ucs_fp:$baseline.fingerprint_sha256, live_fp:$live.fingerprint_sha256,
      matches_baseline:(if x509_eq($baseline, $live) then "OK" else "FAIL" end) }
' old.ucs | grep -v '^#'
```

```
+--------------+------------------+------------------+------------------+
| vs           | ucs_fp           | live_fp          | matches_baseline |
+--------------+------------------+------------------+------------------+
| app_https_vs | 80083D43…A6DE82… | 80083D43…A6DE82… | OK               |
+--------------+------------------+------------------+------------------+
```

`ucs_cert` reads the actual PEM from the UCS filestore (located by the
stanza's `cache-path`), so this works even when the archive is
encrypted — set `F5_UCS_PASSPHRASE` and the cert is decrypted in memory
along with the rest. Certificates are public, so no key or master key is
involved; that is a separate concern (see
[reading an encrypted UCS](kcs-howto-read-encrypted-ucs-archives.md)).

Point the cert at a profile's `cert-key-chain` to resolve it per VS;
here it is referenced by path because there is one app cert.

## How to tell it worked

Every row in the config-parity and probe tables reads `OK`. Pipe
either table through `select(.match=="FAIL")` (config) or
`select(.cert=="FAIL")` (probes) and get nothing back. In a change
gate, fail the step when any FAIL row appears.

## Operational context

Three rules make the cross-file queries behave, and they are why the
combined queries are shaped the way they are:

- **Lead each query with `$old |`.** Without it, `f5 query` runs the
  body once per input file and the table prints twice. Rooting every
  statement at a `$`-variable tells the engine there is no per-file
  work to do, so it runs once. (`--merge` also runs once but refuses
  old-versus-new because both define the same object paths, so it is
  the wrong tool here.)
- **Guard cross-file lookups with `select(contains($keys,$fp))`.** A
  subscript on a missing key is an error, so iterate one side and keep
  only the objects present in both; the inventory query reports the
  rest.
- **`grep -v '^#'`** drops the `# === file… ===` banner that appears
  whenever there is more than one input.

Use `--name old=… --name new=…` so the `$old` / `$new` variables stay
the same whatever the real filenames are.

The probe builtins (`ping`, `portping`, `url_get`, `tls_handshake`,
and friends) always go to the network, so they stay gated behind
`--enable-probes`. Pass the SNI name to
`tls_handshake(host, port, "name")` and a trust anchor with
`--ca-bundle` so `verify` reads `ok` instead of `hostname_mismatch`.
Without a trust anchor you still get the cert and a `reason.kind`
(`expired`, `self_signed`, `untrusted_ca`), which is useful on its
own. In a real migration the VIP address is preserved, so "old" and
"new" are the same IP probed before and after cutover.

Both archives may be encrypted; set the passphrase once with
`F5_UCS_PASSPHRASE` and it unlocks every input in the invocation — see
[reading an encrypted UCS](kcs-howto-read-encrypted-ucs-archives.md).

## Related

- [KCS index](README.md)
- [kcs-howto-read-encrypted-ucs-archives.md](kcs-howto-read-encrypted-ucs-archives.md)
  — supply the passphrase for an encrypted UCS.
- [kcs-howto-audit-server-certs-with-query.md](kcs-howto-audit-server-certs-with-query.md)
  — compare configured certs against the certs live endpoints serve.
- [kcs-howto-reproduce-http-monitor-with-query.md](kcs-howto-reproduce-http-monitor-with-query.md)
  — reproduce a health monitor's send/recv from your laptop.
- [kcs-howto-find-objects-by-query.md](kcs-howto-find-objects-by-query.md)
  — the base query patterns these checks build on.
