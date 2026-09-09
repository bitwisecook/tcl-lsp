# KCS: How do I verify a migration looks the same before and after with `f5 query`?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

I am moving a tenant onto new hardware (or a new BIG-IP version). I
have a config from the old box and one from the new box. How do I
confirm the virtuals, pools, and iRules did not drift, catch anything
that appeared or vanished, and then prove the new VIPs still listen?

## Before you start

- A `bigip.conf` / SCF from each side: `old.conf` and `new.conf`. A
  UCS works too — encrypted archives are fine, see
  [reading an encrypted UCS](kcs-howto-read-encrypted-ucs-archives.md).
- For the live checks: network reach to the VIPs **from where you run
  the command**, and `--enable-probes` (probes are gated off by
  default so an offline audit never touches the network).

## Answer

`f5 query` loads both sides in one invocation and binds each to a
`$`-variable, so the before/after comparison lives in the query — no
shell `diff`, no temp files:

```
--name old=old.conf --name new=new.conf old.conf new.conf
```

Each name must also appear as a positional input; the runner needs
the source text behind it.

### Config parity, with a match column

One query joins each object by full path and prints `OK` / `FAIL` per
object across virtuals, pools, and iRules. A pool is three rows, because
a pool that keeps its members can still change how it load-balances them
or what it monitors them with:

```
f5 query --name old=old.conf --name new=new.conf --table '$old
  | [ $old.ltm.virtual[]."full-path" ] as $vok
  | [ $old.ltm.pool[]."full-path" ]    as $pok
  | [ $old.ltm.rule[]."full-path" ]    as $rok
  | ( $new.ltm.virtual[] as $n | ($n."full-path") as $fp | select(contains($vok,$fp))
      | $old.ltm.virtual[$fp] as $o
      | {check:"VIP", object:$fp, old:$o.destination, new:$n.destination,
         match:(if $o.destination==$n.destination then "OK" else "FAIL" end)} ),
    ( $new.ltm.pool[] as $n | ($n."full-path") as $fp | select(contains($pok,$fp))
      | $old.ltm.pool[$fp] as $o
      | join([$o.members[] | basename(.name)+"="+tostring(.address)+"/"+.state+"/"+.ratio],",") as $om
      | join([$n.members[] | basename(.name)+"="+tostring(.address)+"/"+.state+"/"+.ratio],",") as $nm
      | ( {check:"monitor", object:$fp, old:$o.monitor, new:$n.monitor,
           match:(if $o.monitor==$n.monitor then "OK" else "FAIL" end)},
          {check:"lb-mode", object:$fp, old:$o."load-balancing-mode", new:$n."load-balancing-mode",
           match:(if $o."load-balancing-mode"==$n."load-balancing-mode" then "OK" else "FAIL" end)},
          {check:"members", object:$fp, old:$om, new:$nm,
           match:(if $om==$nm then "OK" else "FAIL" end)} ) ),
    ( $new.ltm.rule[] as $n | ($n."full-path") as $fp | select(contains($rok,$fp))
      | $old.ltm.rule[$fp] as $o
      | {check:"irule", object:$fp,
         old:($o.refs.pools|count|tostring), new:($n.refs.pools|count|tostring),
         match:(if $o.body==$n.body then "OK" else "FAIL" end)} )' \
  old.conf new.conf | awk '/^# ===/{n++} n<2' | grep -v '^#'
```

```
+---------+-----------------------+-------------------------------------------+-------------------------------------------+-------+
| check   | object                | old                                       | new                                       | match |
+---------+-----------------------+-------------------------------------------+-------------------------------------------+-------+
| VIP     | /Common/web_vs        | /Common/10.0.0.10:80                      | /Common/10.0.0.10:80                      | OK    |
| VIP     | /Common/api_vs        | /Common/10.0.0.20:443                     | /Common/10.0.0.99:443                     | FAIL  |
| monitor | /Common/web_pool      | /Common/http_health                       | /Common/http_health                       | OK    |
| lb-mode | /Common/web_pool      | round-robin                               | round-robin                               | OK    |
| members | /Common/web_pool      | web1:80=10.0.1.10//1,web2:80=10.0.1.11//1 | web1:80=10.0.1.10//1,web2:80=10.0.1.11//1 | OK    |
| monitor | /Common/api_pool      | /Common/https_health                      | /Common/http_health                       | FAIL  |
| lb-mode | /Common/api_pool      | least-connections-member                  | round-robin                               | FAIL  |
| members | /Common/api_pool      | api1:443=10.0.2.10//1                     | api1:443=10.0.2.10//1                     | OK    |
| irule   | /Common/api_auth_rule | 0                                         | 0                                         | OK    |
+---------+-----------------------+-------------------------------------------+-------------------------------------------+-------+
```

`api_pool` above is the case a member-name comparison misses: the same
member on both sides, a different monitor and a different
load-balancing mode. Each `members` cell renders one member as
`name=address/state/ratio` from the projected `.members[]` fields, so a
re-pointed address, a member left disabled, or a changed weight shows up
as a `FAIL` rather than hiding behind a matching name; an empty segment
is a field the config does not set. `basename` keeps the column narrow —
drop it to see full paths.

Add `| select(.match=="FAIL")` to show only the drift. Run a single
check by keeping just one arm.

### Inventory parity (added or removed objects)

The join above compares objects present in **both** sides. To catch
objects that appeared or vanished, compare the key sets. Do it for every
object kind the parity table covers, plus data groups — a pool or an
iRule can be added or dropped as easily as a virtual, and only the kinds
you name here are checked:

```
f5 query --name old=old.conf --name new=new.conf --table '$old
  | ["virtual","pool","rule","data-group"][] as $kind
  | [ $old.ltm[$kind][]."full-path" ] as $ok
  | [ $new.ltm[$kind][]."full-path" ] as $nk
  | ( $nk[] | select(contains($ok,.)|not) | {kind:$kind, object:., status:"ADDED (not in old)"} ),
    ( $ok[] | select(contains($nk,.)|not) | {kind:$kind, object:., status:"REMOVED (gone in new)"} )' \
  old.conf new.conf | awk '/^# ===/{n++} n<2' | grep -v '^#'
```

```
+------------+------------------------+-----------------------+
| kind       | object                 | status                |
+------------+------------------------+-----------------------+
| virtual    | /Common/vpn_vs_renamed | ADDED (not in old)    |
| virtual    | /Common/vpn_vs         | REMOVED (gone in new) |
| pool       | /Common/cache_pool     | ADDED (not in old)    |
| pool       | /Common/legacy_pool    | REMOVED (gone in new) |
| rule       | /Common/cache_rule     | ADDED (not in old)    |
| rule       | /Common/legacy_rule    | REMOVED (gone in new) |
| data-group | /Common/cache_hosts    | ADDED (not in old)    |
| data-group | /Common/legacy_hosts   | REMOVED (gone in new) |
+------------+------------------------+-----------------------+
```

An empty result means the inventory matches across all four kinds — and
only those four. `.ltm[$kind]` takes the object kind from the list, so
add a kind to the list to widen the sweep, and swap `.ltm` for `.gtm` or
`.security` to sweep another module; those three are what the projection
covers.

### Live checks — does the migrated box answer?

Config parity proves the two files agree. The probes prove the
running box agrees with the outside world. Each VIP's address and
port come straight from the config:

```
f5 query --enable-probes --table '
  .ltm.virtual[]
  | host(.destination) as $h | port(.destination) as $p
  | { vs: .name, target: ($h + ":" + ($p|tostring)),
      listening: (if portping($h, $p).ok then "UP" else "DOWN" end) }
' new.conf
```

```
+---------+-----------------+-----------+
| vs      | target          | listening |
+---------+-----------------+-----------+
| app_vs  | 127.0.0.1:28080 | UP        |
| dead_vs | 127.0.0.1:28099 | DOWN      |
+---------+-----------------+-----------+
```

For a TLS VIP, add `tls_handshake($h, $p)` and read `.protocol`,
`.peer_cert`, and `.reason.kind` — that is the cert half of the
check, covered in
[auditing server certs](kcs-howto-audit-server-certs-with-query.md).
The `url_*` builtins do not make a live request, so drive HTTP checks
with `curl` instead.

## How to tell it worked

Every row in the parity tables reads `OK`, the inventory query
returns nothing for all four kinds, and every migrated VIP reads `UP`.
In a change gate, fail the step when any `FAIL` row appears, or when the
inventory query prints at all. The gate is only as wide as the fields
and kinds the two queries name: a virtual's profiles, rules, and
persistence are joined by neither, so add an arm for anything else the
migration was meant to preserve.

## Operational context

Three rules make the cross-file queries behave:

- **Root every statement at `$old` or `$new`.** A bare path would be
  read from whichever file the runner is currently on.
- **Guard cross-file lookups with `select(contains($keys,$fp))`.** A
  subscript on a missing key is an error, so iterate one side and keep
  only the objects present in both; the inventory query reports the
  rest.
- **The query runs once per input file**, so a two-file invocation
  prints the same report twice, under a `# === file… ===` banner each
  time. `awk '/^# ===/{n++} n<2'` keeps the first copy and `grep -v
  '^#'` drops the banner. (`--merge` runs once but refuses
  old-versus-new, because both sides define the same object paths.)

The probe builtins (`ping`, `portping`, `tls_handshake`) always go to
the network, so they stay behind `--enable-probes`. Pass a trust
anchor with `--ca-bundle` and the SNI name as
`tls_handshake(host, port, "name")` when the default trust store or
hostname is not what a real client would use. In a real migration the
VIP address is preserved, so "old" and "new" are the same IP probed
before and after cutover.

Both archives may be encrypted; set `F5_UCS_PASSPHRASE` once and it
unlocks every input in the invocation.

## Related

- [KCS index](README.md)
- [kcs-howto-read-encrypted-ucs-archives.md](kcs-howto-read-encrypted-ucs-archives.md)
  — supply the passphrase for an encrypted UCS.
- [kcs-howto-audit-server-certs-with-query.md](kcs-howto-audit-server-certs-with-query.md)
  — compare configured certs against the certs live endpoints serve.
- [kcs-howto-find-objects-by-query.md](kcs-howto-find-objects-by-query.md)
  — the base query patterns these checks build on.
