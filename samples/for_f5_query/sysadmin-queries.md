# BIG-IP sysadmin queries

A working cookbook of `f5 query` recipes for the questions
sysadmins actually ask: pool-member rosters, orphan cleanup,
iRule + data-group cross-references, APM policy tracing,
GTM-to-backend chains, log-driven timelines, and live device
probes. Every scenario has a description, the exact command,
and the **real output** captured from running it against the
fixtures in this directory.

## Setup

All examples run against the SCF / log fixtures already in this
directory:

| Path                                                     | What it is                                                       |
|----------------------------------------------------------|------------------------------------------------------------------|
| [`ltm.conf`](ltm.conf)                                   | Single LTM (nodes, pools, VSes, iRules, **data-groups**)         |
| [`gtm.conf`](gtm.conf)                                   | GTM datacenters, servers, pools, wide-IPs                        |
| [`apm.conf`](apm.conf)                                   | APM access-policy + items, attached via LTM `vpn_vs`             |
| [`multitier/`](multitier/)                               | GTM → tier-1 LTM → 12× tier-2 LTM → tier-3 reaggregator          |
| [`multitier/logs/`](multitier/logs/)                     | Syslog samples for every device                                  |
| [`sysadmin/`](sysadmin/)                                 | Reusable `.f5q` scripts                                          |

CLI:

```
$ f5 query [flags] '<expression>' [paths...]
$ f5 query [flags] -f <script.f5q> [paths...]
```

Frequently-used flags appear in the cheat sheet at the end.

The probe-based scenarios in [Section 8](#8-live-probes) need
`--enable-probes` and reach the network. All other scenarios are
offline-only.

---

## Contents

1. [Inventory & export](#1-inventory--export)
2. [Find / search](#2-find--search)
3. [Orphans, duplicates, audit](#3-orphans-duplicates-audit)
4. [iRules + data-groups](#4-irules--data-groups)
5. [APM access policies](#5-apm-access-policies)
6. [GTM & cross-tier joins](#6-gtm--cross-tier-joins)
7. [Logs & timeline](#7-logs--timeline) (including an ASCII Gantt of monitor up/down)
8. [Live probes](#8-live-probes)
9. [Migration & bulk edits](#9-migration--bulk-edits)
10. [Localhost / self-IP security audits](#10-localhost--self-ip-security-audits)
11. [Training-question crosswalk](#11-training-question-crosswalk)
12. [Cheat sheet](#12-cheat-sheet)

Most scenarios came from real questions on DevCentral, the F5
community forum, and `r/F5Networks` — links to the originals are
inline where applicable.

---

## 1. Inventory & export

### 1.1 List every VS with destination, pool, port (CSV)

**Question.** *["What tmsh command do I use to view virtual servers
and their addresses?"][q11]* — recurring forum thread; the
sysadmin wants a spreadsheet-friendly dump, not the multi-line
`tmsh list` output.

[q11]: https://community.f5.com/discussions/technicalforum/what-tmsh-command-do-i-use-to-view-pool-members-and-their-addresses/199927

**Query.** Stream every VS, emit one CSV row per VS. `csv(...)`
quotes per RFC 4180.

```
$ f5 query --raw '
    "vs,destination,host,port,pool",
    (.ltm.virtual[]
     | csv(.name, .destination, host(.destination), port(.destination), .pool))
  ' ltm.conf
```

```
vs,destination,host,port,pool
web_vs,/Common/10.0.0.10:80,10.0.0.10,80,/Common/web_pool
web_secure_vs,/Common/10.0.0.10:443,10.0.0.10,443,/Common/web_pool
api_vs,/Common/10.0.0.20:443,10.0.0.20,443,/Common/api_pool
legacy_vs,/Common/192.168.50.100:80,192.168.50.100,80,/Common/legacy_pool
forwarder_vs,/Common/0.0.0.0:0,0.0.0.0,0,
vpn_vs,/Common/10.0.0.30:443,10.0.0.30,443,/Common/api_pool
```

The first stream element (`"vs,destination,host,port,pool"`) is
the header row — `,` at the top level of a pipeline concatenates
streams, so the literal flows out before the per-VS rows.

### 1.2 Pool member rollup

**Question.** *["What tmsh command do I use to view pool members
and their addresses?"][q11]* — the most-asked F5 ops question on
DevCentral. The accepted answer was `tmsh show ltm pool ... members
field-fmt | grep`, which is brittle.

**Query.** Per pool, broadcast over its members; `as $p` carries
the outer pool context into each member row.

```
$ f5 query --raw '
    "pool,member,host,port,monitor",
    (.ltm.pool[] as $p
     | $p.members[]
     | csv($p.name, .name, .address, port(.name),
           join($p.monitor.monitors, ",")))
  ' ltm.conf
```

```
pool,member,host,port,monitor
web_pool,/Common/web1:80,10.0.1.10,80,/Common/http
web_pool,/Common/web2:80,10.0.1.11,80,/Common/http
api_pool,/Common/api1:8443,10.0.2.20,8443,/Common/https
api_pool,/Common/api2:8443,10.0.2.21,8443,/Common/https
legacy_pool,/Common/legacy1:80,192.168.50.10,80,/Common/http
```

The pool's `.monitor` is a *monitor expression* (it can carry
the `min N of {...}` quorum form). `.monitor.monitors` is the
list of monitor names inside; `join` collapses it to a single
CSV-safe cell. `unused_pool` (zero members) does not contribute
a row — `.members[]` on an empty list emits nothing.

### 1.3 Every IP the box advertises

**Question.** *"Give me one IP-address column that combines VSes,
nodes, and self-IPs"* — generic CMDB / firewall-rule audit.

**Query.** Three statements sharing one root, concatenated by `;`.
`sort -u` outside the query collapses duplicates across them.

```
$ f5 query --raw '
    .ltm.virtual[] | host(.destination) ;
    .ltm.node[]    | .address           ;
    .net.self[]    | .address
  ' ltm.conf | sort -u
```

```
0.0.0.0
10.0.0.10
10.0.0.20
10.0.0.30
10.0.1.10
10.0.1.11
10.0.2.20
10.0.2.21
192.168.50.10
192.168.50.100
```

The fixture has no `net self` stanzas, so the third statement is
empty here — on a real box the self-IPs appear too.

### 1.4 Public vs. private destinations

**Question.** *"Did anything public-routable leak onto a tenant
partition?"* — pre-audit before a partition move.

```
$ f5 query --raw '
    .ltm.virtual[]
    | tsv(.name, .destination,
          (if is_public(host(.destination)) then "PUBLIC" else "private" end))
  ' ltm.conf
```

```
web_vs	/Common/10.0.0.10:80	private
web_secure_vs	/Common/10.0.0.10:443	private
api_vs	/Common/10.0.0.20:443	private
legacy_vs	/Common/192.168.50.100:80	private
forwarder_vs	/Common/0.0.0.0:0	private
vpn_vs	/Common/10.0.0.30:443	private
```

`is_public` is the inverse of `is_private` / `is_loopback` /
`is_reserved`. Swap in `is_documentation` to find `192.0.2.0/24`
/ `203.0.113.0/24` (TEST-NET) leaks. Returns `false` for the
`0.0.0.0` wildcard.

### 1.5 Pool sizes as JSON

**Question.** *"For a quarterly capacity review, dump every pool
with its monitor and member list"* — feeds straight into a JSON
column in BigQuery / Snowflake.

```
$ f5 query '.ltm.pool[]
            | {name,
               monitor,
               member_count: ([.members[]] | count),
               members: [.members[].address]}' ltm.conf
```

```json
[
  {
    "name": "web_pool",
    "monitor": "/Common/http",
    "member_count": 2,
    "members": [
      "10.0.1.10",
      "10.0.1.11"
    ]
  },
  {
    "name": "api_pool",
    "monitor": "/Common/https",
    "member_count": 2,
    "members": [
      "10.0.2.20",
      "10.0.2.21"
    ]
  },
  {
    "name": "legacy_pool",
    "monitor": "/Common/http",
    "member_count": 1,
    "members": [
      "192.168.50.10"
    ]
  },
  {
    "name": "unused_pool",
    "monitor": "/Common/tcp",
    "member_count": 0,
    "members": []
  }
]
```

`{name, monitor, ...}` bareword keys desugar to `{name: .name,
monitor: .monitor, ...}` — jq's shortcut, supported here.

### 1.6 Persistence profiles per VS

**Question.** *["List all virtual servers with persistence
profiles"][q16]* — common CSV deliverable for change reviews.

[q16]: https://www.middlewareinventory.com/blog/f5-list-virtual-servers-persistence-profiles/

```
$ f5 query --raw '
    .ltm.virtual[]
    | tsv(.name, join(.profiles[]."full-path", ","))
  ' ltm.conf
```

```
web_vs	/Common/http,/Common/tcp
web_secure_vs	/Common/http,/Common/tcp
api_vs	/Common/http,/Common/tcp
legacy_vs	
forwarder_vs	/Common/fastL4
vpn_vs	/Common/employee_login_profile,/Common/http,/Common/tcp
```

This fixture doesn't carry an explicit `persist` profile on any
VS; the typical real-world query is the same shape with
`.persist[]` substituted for `.profiles[]`. `legacy_vs` has no
profiles at all (an unusual config — typically forwarders only).

### 1.7 The full inventory as one CSV (via a saved script)

The longer "one CSV that joins VSes and members in a single
table" recipe is checked in at
[`sysadmin/inventory_csv.f5q`](sysadmin/inventory_csv.f5q):

```
$ f5 query --raw -f sysadmin/inventory_csv.f5q ltm.conf
```

```
kind,name,destination,host,port,pool,member,member_addr,member_port,monitor
ltm virtual,/Common/web_vs,/Common/10.0.0.10:80,10.0.0.10,80,/Common/web_pool,,,,
ltm virtual,/Common/web_secure_vs,/Common/10.0.0.10:443,10.0.0.10,443,/Common/web_pool,,,,
ltm virtual,/Common/api_vs,/Common/10.0.0.20:443,10.0.0.20,443,/Common/api_pool,,,,
ltm virtual,/Common/legacy_vs,/Common/192.168.50.100:80,192.168.50.100,80,/Common/legacy_pool,,,,
ltm virtual,/Common/forwarder_vs,/Common/0.0.0.0:0,0.0.0.0,0,,,,,
ltm virtual,/Common/vpn_vs,/Common/10.0.0.30:443,10.0.0.30,443,/Common/api_pool,,,,
pool member,web_pool,,,,web_pool,/Common/web1:80,10.0.1.10,80,/Common/http
pool member,web_pool,,,,web_pool,/Common/web2:80,10.0.1.11,80,/Common/http
pool member,api_pool,,,,api_pool,/Common/api1:8443,10.0.2.20,8443,/Common/https
pool member,api_pool,,,,api_pool,/Common/api2:8443,10.0.2.21,8443,/Common/https
pool member,legacy_pool,,,,legacy_pool,/Common/legacy1:80,192.168.50.10,80,/Common/http
```

The `kind` column lets a spreadsheet pivot table separate VS
rows from member rows. Pipe through `column -t -s,` for an
aligned terminal view.

---

## 2. Find / search

### 2.1 What references a pool? (forward audit)

**Question.** *["How to get virtual servers associated with a
specific SNAT pool?"][q21]* — the accepted forum answer was a
brittle `tmsh list ... | grep | awk` pipeline.

[q21]: https://community.f5.com/discussions/technicalforum/how-to-get-virtual-servers-associated-with-a-specific-snat-pool/320047

**Query.** `references_to(path)` is the same engine `f5 grep`
uses, so iRule body references and compound values are caught
too — not just plain field references.

```
$ f5 query --raw 'references_to("/Common/web_pool")' ltm.conf
```

```
/Common/api_router_rule
/Common/web_secure_vs
/Common/web_vs
```

Note that `/Common/api_router_rule` is in the list — that iRule
contains `pool /Common/web_pool` as a fallback in its body, and
the same token-bounded grep catches it. Plain `grep` against the
SCF would also find it, but `references_to` won't false-positive
on a coincidental string match.

### 2.2 Find every VS in a subnet

**Question.** *"Which VIPs are in 192.168.0.0/16 — every one of
those is going to move next quarter."*

```
$ f5 query --raw '
    .ltm.virtual[]
    | select(in_cidr(.destination, "192.168.0.0/16"))
    | tsv(.name, .destination)
  ' ltm.conf
```

```
legacy_vs	/Common/192.168.50.100:80
```

`in_cidr` strips the `/Common/` prefix and `:port` from the
destination before testing. Combine with `or` to test an IPv4
range and the equivalent IPv6 prefix in one shot.

### 2.3 Find the F5 that holds a specific URL

**Question.** *["How to find a particular URL across hundreds of
F5s"][q23]* — the community-accepted answer was "use DNS, then
trace the IP." We can do it directly: resolve the FQDN, then
search every loaded SCF for a VS that destinations-to it.

[q23]: https://community.f5.com/discussions/technicalforum/f5-related-questionhow-to-find-how-particular-url-in-f5-if-we-have-hundreds-of-f/308040

```
$ f5 query --raw '
    .ltm.virtual[]
    | select(host(.destination) == "10.0.0.20")
    | tsv(.name, .destination, .pool)
  ' ltm.conf
```

```
api_vs	/Common/10.0.0.20:443	/Common/api_pool
```

Plug a glob into the positional args and the same query scans
every conf at once — no SSH-and-grep loop required:

```
$ f5 query --raw '
    .ltm.virtual[]
    | select(host(.destination) == "10.2.0.10")
    | tsv(source_file(.), .name, .destination)
  ' multitier/tier2-*-ltm-ha.conf
```

```
# === file:///home/user/tcl-lsp/samples/for_f5_query/multitier/tier2-c01-ltm-ha.conf ===
file:///home/user/tcl-lsp/samples/for_f5_query/multitier/tier2-c01-ltm-ha.conf	app_vs	/Common/10.2.0.10:443
…
```

`source_file(.)` is the URI of the file the matched object came
from — invaluable when sweeping a fleet's worth of saved SCFs at
once.

### 2.4 Regex subscript — VSes whose path matches a pattern

```
$ f5 query --raw '.ltm.virtual["~^/Common/web"] | .name' ltm.conf
```

```
web_vs
web_secure_vs
```

`["~..."]` is a Python regex matched against the full-path. The
typical operational use is partition / app-prefix filtering
(`/Common/iApps/Tenant.app/...`).

### 2.5 Distinct hosts and their VS count

**Question.** *"Find duplicate destination IPs — every duplicate
is two services fighting for the same VIP."*

```
$ f5 query --raw '
    [.ltm.virtual[] | host(.destination)] | sort
  ' ltm.conf | uniq -c | sort -rn
```

```
      2 10.0.0.10
      1 192.168.50.100
      1 10.0.0.30
      1 10.0.0.20
      1 0.0.0.0
```

`10.0.0.10` carries two VSes — `web_vs:80` and `web_secure_vs:443`.
Same host, different ports — that's fine. The same query with
`tsv(.name, .destination)` and `sort | uniq -d -w 30` finds
host+port collisions.

---

## 3. Orphans, duplicates, audit

### 3.1 Orphan pools, iRules, monitors, data-groups

**Question.** *["How to identify unused objects on the F5
device?"][q31]* — F5's own iHealth Portal flags these; the same
list is one query away.

[q31]: https://www.kareemccie.com/2020/05/how-to-identify-unused-objects-in-f5.html

```
$ f5 query --raw '
    .ltm.pool[]          | select(referenced_by(.) | count == 0) | .name ;
    .ltm.rule[]          | select(referenced_by(.) | count == 0) | .name ;
    .ltm["data-group"][] | select(referenced_by(.) | count == 0) | .name ;
    .ltm.node[]          | select(referenced_by(.) | count == 0) | .name
  ' ltm.conf
```

```
unused_pool
maintenance_rule
```

`unused_pool` was created but never attached; `maintenance_rule`
is a typical "kept around for emergencies" rule with no VS
attached *right now*. Both data-groups and nodes in the fixture
are referenced.

To get the **SCF stanza** of each orphan (ready to paste into a
`tmsh delete` script), swap `--raw` for `--scf`.

### 3.2 VSes without a default pool

```
$ f5 query --raw '
    .ltm.virtual[]
    | select(.pool == "")
    | tsv(.name, .destination, ([.profiles[]] | count))
  ' ltm.conf
```

```
forwarder_vs	/Common/0.0.0.0:0	1
```

`forwarder_vs` has one profile (`fastL4`) and no default pool —
correct for a forwarder. A poolless `http` VS with no iRule
would be an empty listener and is almost always a bug.

### 3.3 Cross-partition leaks

`check_partition_visibility()` walks every reference and reports
any that violates F5's directional visibility rules
(`/Tenant_A/` may reference `/Common/`, but not vice versa, and
not `/Tenant_B/`):

```
$ f5 query --raw 'check_partition_visibility()' ltm.conf
```

(no output)

Empty result = clean. On a leaky config it prints a list of
`"<referrer> -> <target>"` strings. Pair with
`rename_partition` (Section 9) to plan the cleanup.

---

## 4. iRules + data-groups

### 4.1 Every iRule and what it references

**Question.** *"For a quarterly cleanup, I need every iRule
along with the pools, persistence profiles, and data-groups it
touches."*

**Query.** `.refs.pools` / `.refs.persists` / `.refs."data-groups"`
are the edges `core.bigip.irules_refs.extract_irules_object_references`
extracts from the iRule body — same engine `f5 grep` walks.

```
$ f5 query --raw '
    .ltm.rule[]
    | tsv(.name,
          join(.refs.pools, ","),
          join(.refs.persists, ","),
          join(.refs."data-groups", ","))
  ' ltm.conf
```

```
log_rule			
maintenance_rule			
ip_blocklist_rule			/Common/banned_ips
api_router_rule	/Common/web_pool		/Common/routing_map
api_auth_rule			/Common/api_keys
```

`api_router_rule` is the most interesting: it has a `pool` body
command pointing at `/Common/web_pool` *and* a `class match`
against `routing_map`. Both edges are caught.

### 4.2 Which data-groups is each iRule consuming?

```
$ f5 query --raw '
    .ltm.rule[]
    | .refs."data-groups"[] as $dg
    | tsv(.name, $dg)
  ' ltm.conf
```

```
ip_blocklist_rule	/Common/banned_ips
api_router_rule	/Common/routing_map
api_auth_rule	/Common/api_keys
```

Inverse — "which iRules reference this data-group" — comes
from `referenced_by`:

```
$ f5 query --raw '
    .ltm["data-group"][] as $dg
    | tsv($dg.name, $dg.type,
          join(referenced_by($dg), ","))
  ' ltm.conf
```

```
banned_ips	ip	/Common/ip_blocklist_rule
api_keys	string	/Common/api_auth_rule
routing_map	string	/Common/api_router_rule
```

`referenced_by` is fed by the same edges, so the two views are
guaranteed to agree.

### 4.3 The full chain: VS → iRule → data-group

**Question.** *"If I touch `banned_ips`, which client traffic
does it affect?"* — i.e., trace the data-group back to the
listener.

```
$ f5 query --raw '
    .ltm.virtual[] as $vs
    | $vs.rules[] as $rref
    | str($rref) as $r
    | .ltm.rule[$r].refs."data-groups"[] as $dg
    | tsv($vs.name, $r, $dg)
  ' ltm.conf
```

```
api_vs	/Common/ip_blocklist_rule	/Common/banned_ips
api_vs	/Common/api_auth_rule	/Common/api_keys
api_vs	/Common/api_router_rule	/Common/routing_map
```

Three rows — every data-group `api_vs` touches, by way of one
of its iRules. `str($rref)` coerces the path-ref so it can
subscript `.ltm.rule[...]`.

### 4.4 iRules that use a given Tcl command

**Question.** *["Find every iRule that does HTTP::respond"][q44]* —
common before tightening a corporate-style policy.

[q44]: https://community.f5.com/discussions/technicalforum/check-via-irule-if-data-group-exists/257590

```
$ f5 query --raw '
    .ltm.rule[]
    | match(.body, "HTTP::respond") as $usesresp
    | match(.body, "class match")   as $usesclass
    | match(.body, "pool ")         as $usespool
    | match(.body, "HTTP::redirect") as $usesredir
    | tsv(.name, $usesresp, $usesclass, $usespool, $usesredir)
  ' ltm.conf
```

```
log_rule	false	false	false	false
maintenance_rule	true	false	false	false
ip_blocklist_rule	false	true	false	false
api_router_rule	false	true	true	false
api_auth_rule	true	true	false	false
```

`match(.body, regex)` runs Python's `re.search` against the iRule
body. Same regex flavour as Tcl's POSIX, **except** for the few
features the device doesn't support (named groups, look-behind);
patterns that work here will work on the device but not vice
versa.

### 4.5 Data-group records, expanded

```
$ f5 query --raw '
    .ltm["data-group"][]
    | tsv(.name, .type, .records[])
  ' ltm.conf
```

```
banned_ips	ip	10.99.0.0/16
banned_ips	ip	198.51.100.7/32
api_keys	string	deadbeef
api_keys	string	cafebabe
routing_map	string	/api
routing_map	string	/legacy
```

For an audit of `banned_ips` you'd typically pipe through
`in_cidr` to spot a typo (a host accidentally written as
`/16`):

```
$ f5 query --raw '
    .ltm["data-group"]["/Common/banned_ips"].records[]
    | select(in_cidr("10.99.42.7", .))
  ' ltm.conf
```

```
10.99.0.0/16
```

The record `10.99.0.0/16` is the entry that would block
`10.99.42.7`.

---

## 5. APM access policies

### 5.1 Summary of every access-policy

**Query.** Auto-deref: `.start-item.caption` walks
`access-policy → policy-item → caption` in one chain.

```
$ f5 query --raw '
    .apm["access-policy"][]
    | tsv(.name, .start-item.caption, .default-ending,
          ([.items[]] | count))
  ' apm.conf
```

```
employee_login	Start	/Common/employee_login_end_deny	5
```

One policy, starting at the "Start" item, defaulting to the deny
ending, made up of 5 items.

### 5.2 Trace a policy — every item in order

```
$ f5 query --raw '
    .apm["access-policy"]["/Common/employee_login"]
    | .items[] as $item
    | tsv($item.name, $item.caption,
          join([$item.agents[]], ","))
  ' apm.conf
```

```
employee_login_ent	Start	
employee_login_logon_page	Logon Page	/Common/employee_login_logon_page_ag
employee_login_localdb_auth	LocalDB Auth	/Common/employee_login_localdb_auth_ag
employee_login_end_allow	Allow	
employee_login_end_deny	Deny	
```

Columns: item name → caption → agent path-refs. The flow
arrows between items (branch-rules) aren't projected in v1, but
the agent type per step is enough for a "what does the policy
do?" walk-through.

### 5.3 Find every LTM VS that attaches a given APM profile

**Question.** *"Before retiring `employee_login_profile`, which
VSes still attach it?"*

`--merge` is the natural mode: LTM and APM are separate SCFs but
the profile reference crosses files.

```
$ f5 query --merge --raw '
    .ltm.virtual[]
    | select(any(.profiles[]."full-path" == "/Common/employee_login_profile"))
    | tsv(.name, .destination)
  ' ltm.conf apm.conf
```

```
# === file:///home/user/tcl-lsp/samples/for_f5_query/ltm.conf ===
vpn_vs	/Common/10.0.0.30:443
```

The `# === <uri> ===` header appears when more than one file
contributes. Pipe through `grep -v '^#'` for a clean list.

---

## 6. GTM & cross-tier joins

### 6.1 Wide-IPs and their pool chains

```
$ f5 query --raw '
    .gtm.wideip[]
    | tsv(.name, .pools[].name,
          join(.pools[].members[], ","))
  ' gtm.conf
```

```
www.example.com	example_app_pool	/Common/bigip-east:/Common/web_vs,/Common/bigip-west:/Common/web_vs
api.example.com	api_app_pool	/Common/bigip-east:/Common/api_vs
```

`.pools[]` is a PathRef list; auto-deref makes `.pools[].members[]`
walk wide-IP → GTM pool → member in one chain. Pool members are
strings of the form `<gtm-server>:<vs-fullpath>`.

### 6.2 GTM server inventory

```
$ f5 query --raw '
    .gtm.server[]
    | tsv(.name, .datacenter, join(."virtual-servers", ","))
  ' gtm.conf
```

```
bigip-east	/Common/dc-east	10.0.0.10:80,10.0.0.10:443,10.0.0.20:443
bigip-west	/Common/dc-west	10.1.0.10:80
```

### 6.3 Wide-IP → backend chain across LTM + GTM

The flagship cross-file join — wide-IP through GTM pool, into
the LTM VS the GTM pool member references, through that VS's
pool, all the way to the backend node:

```
$ f5 query --name ltm=ltm.conf --name gtm=gtm.conf --merge --raw '
    $gtm.gtm.wideip[] as $w
    | $w.pools[] as $gp
    | $gp.members[]
    | last(split(., ":")) as $vspath
    | ($ltm.ltm.virtual[]
       | select(."full-path" == $vspath)) as $vs
    | $vs.pool.members[]
    | tsv($w.name, $gp.name, $vs.name, $vs.pool, .address, port(.name))
  ' ltm.conf gtm.conf | sort -u
```

```
# === file:///.../samples/for_f5_query/ltm.conf ===
api.example.com	api_app_pool	api_vs	/Common/api_pool	10.0.2.20	8443
api.example.com	api_app_pool	api_vs	/Common/api_pool	10.0.2.21	8443
www.example.com	example_app_pool	web_vs	/Common/web_pool	10.0.1.10	80
www.example.com	example_app_pool	web_vs	/Common/web_pool	10.0.1.11	80
```

Reading top-to-bottom: every wide-IP (`$w`) → its GTM pools
(`$gp`) → pool-member strings (`<server>:<vspath>`) → peel
`$vspath` off → look up the LTM virtual → walk its pool
members → emit one row per backend address. `sort -u` collapses
the per-file iteration duplication.

### 6.4 Wide-IP-only DNS map (GTM standalone)

```
$ f5 query --raw '
    .gtm.wideip[]
    | tsv(.name, ."record-type")
  ' gtm.conf
```

```
www.example.com	a
api.example.com	a
```

For a forward-DNS audit, this is the "what is GTM authoritative
for?" answer. Live verification — *does* DNS actually return
that record? — is in [Section 8](#8-live-probes).

---

## 7. Logs & timeline

The `multitier/logs/` directory carries representative syslog
from every device in the multitier. Standard message-code
families used below (KB **K9970** lists every one):

| Code         | Meaning                                                 |
|--------------|---------------------------------------------------------|
| `01340011:4` | Pool member monitor status **down**                      |
| `01340012:6` | Pool member monitor status **up**                        |
| `01010013:3` | Monitor failing for a node                               |
| `01340038:6` | Device entered **Active** state (HA failover)            |
| `01340039:6` | Device entered **Standby** state                         |
| `01070417:6` | AUDIT — config change via tmsh / GUI                     |
| `01071a08:6` | Configuration loaded                                     |
| `01071db9:6` | Boot complete                                            |
| `01260009:4` | SSL handshake failed                                     |
| `011a4001:4` | GTM pool member transitioned to **down**                 |
| `011a4002:6` | GTM pool member transitioned to **up**                   |

### 7.1 When did pool member X go down/up?

**Question.** *["Gather a list of virtuals/pools that are
offline and provide duration"][q71]* — the community answer
was "TMOS doesn't track downtime, use Splunk." `f5log_load`
makes the answer "use your existing log file."

[q71]: https://community.f5.com/t5/technical-forum/gather-a-list-of-virtuals-and-or-pools-that-are-offline-state/td-p/26436

```
$ f5 query --raw '
    f5log_load("multitier/logs/t1-a.log")[]
    | select(.module == "01340011" or .module == "01340012")
    | select(contains(.message, "/Common/t2_c01_vip:443"))
    | tsv(.timestamp, .module,
          (if .module == "01340011" then "DOWN" else "UP" end),
          .message)
  ' multitier/tier1-ltm-ha.conf
```

```
Mar 14 10:11:09	01340011	DOWN	Pool /Common/t2_fanout_pool member /Common/t2_c01_vip:443 monitor status down.
Mar 14 10:20:15	01340012	UP	Pool /Common/t2_fanout_pool member /Common/t2_c01_vip:443 monitor status up.
```

That member was down for **~9 minutes** between 10:11 and 10:20.
The same shape, with `select(contains(.message, "..."))` swapped
out, drills into any other member.

### 7.2 Flap top-N

```
$ f5 query --raw --input-f5log lt=multitier/logs/t1-a.log '
    [$lt[]
     | select(.module == "01340011")
     | sub(.message, "^.*member ", "")
     | sub(., " monitor.*$", "")]
    | sort
  ' multitier/tier1-ltm-ha.conf | grep -v '#' | uniq -c | sort -rn | head -5
```

```
      1 /Common/t2_c12_vip:443
      1 /Common/t2_c11_vip:443
      1 /Common/t2_c10_vip:443
      1 /Common/t2_c09_vip:443
      1 /Common/t2_c08_vip:443
```

The generated multitier log only has one down event per member,
so counts are flat at 1. On a real box with flap-prone members
you'll see counts in the tens / hundreds — that's exactly when
this query earns its keep.

`--input-f5log lt=PATH` binds the parsed log to `$lt`. Inside
the query, `$lt[]` streams each event dict
(`{timestamp, host, severity, daemon, pid, code, module, level,
message, raw}`).

### 7.3 HA failover events

```
$ f5 query --raw '
    f5log_load("multitier/logs/t1-a.log")[],
    f5log_load("multitier/logs/t1-b.log")[],
    f5log_load("multitier/logs/t2-c01-a.log")[],
    f5log_load("multitier/logs/t2-c01-b.log")[]
    | select(.module == "01340038" or .module == "01340039")
    | tsv(.timestamp, .host,
          (if .module == "01340038" then "ACTIVE" else "STANDBY" end))
  ' multitier/tier1-ltm-ha.conf
```

```
Mar 14 08:26:50	t1-a.example.test	ACTIVE
Mar 14 07:23:21	t1-b.example.test	STANDBY
Mar 14 07:46:29	t2-c01-a.example.test	ACTIVE
Mar 14 06:20:23	t2-c01-b.example.test	STANDBY
```

Each pair is one HA group; the device that's `ACTIVE` is the
one currently handling traffic. A `STANDBY` entry that follows
an `ACTIVE` from the same host **after** an `01010013:3`
monitor-failing event is the kind of evidence a postmortem
wants — pair the two log queries to build that timeline.

### 7.4 SSL-handshake failures clustered by client

```
$ f5 query --raw --input-f5log lt=multitier/logs/t2-c01-a.log '
    $lt[]
    | select(.module == "01260009")
    | sub(.message, "^.*for client ", "")
    | sub(., " on .*$", "")
  ' multitier/tier2-c01-ltm-ha.conf | grep -v '#' | sort | uniq -c | sort -rn
```

```
      1 198.51.100.7
```

If one client IP keeps showing up, that's stuck ciphersuite
negotiation, missing SNI, or stale OpenSSL on the client. KB
**K15212** is the umbrella troubleshooting guide.

### 7.5 Audit trail — who changed what?

```
$ f5 query --raw --input-f5log lt=multitier/logs/t1-a.log '
    $lt[]
    | select(.module == "01070417")
    | tsv(.timestamp, .host, sub(.message, "^AUDIT - ", ""))
  ' multitier/tier1-ltm-ha.conf
```

```
# === file:///home/user/tcl-lsp/samples/for_f5_query/multitier/tier1-ltm-ha.conf ===
Mar 14 08:54:06	t1-a.example.test	jadmin - tmsh - tmsh -c modify ltm virtual /Common/t1_ingress_vs description "managed by netops"
```

Filter by username with `select(contains(.message, "jadmin"))`
or by target object with `select(contains(.message, "/Common/t1_ingress_vs"))`.

### 7.6 Boot and config-load timeline

```
$ f5 query --raw --input-f5log lt=multitier/logs/t1-a.log '
    $lt[]
    | select(.module == "01071db9" or .module == "01071a08")
    | tsv(.timestamp,
          (if .module == "01071db9" then "BOOT" else "CFG-LOAD" end),
          .message)
  ' multitier/tier1-ltm-ha.conf
```

```
Mar 14 06:32:36	BOOT	Boot complete on t1-a.example.test
Mar 14 07:40:19	CFG-LOAD	Configuration loaded from /config/bigip.conf successfully.
```

Cross-reference with 7.3: if a device went `ACTIVE` *before*
its config-load completed, that's the kind of race that turns
into a brief outage during a reboot.

### 7.7 ASCII Gantt timeline of monitor events

**Question.** *["Provide a list of virtuals/pools offline along
with how long they've been offline"][q71]* — the standard answer
("export the log and graph it in Splunk") works, but for ad-hoc
incident-response we can render the timeline straight from
`f5log_load` to a terminal.

[`sysadmin/monitor_timeline.py`](sysadmin/monitor_timeline.py)
consumes the `f5 query` output and renders a Gantt-style ASCII
chart — `#` is "marked down," `v` is the DOWN transition, `^` is
the UP transition.

```
$ f5 query --raw '
    f5log_load("multitier/logs/t1-a.log")[]
    | select(.module == "01340011" or .module == "01340012")
    | tsv(.timestamp,
          (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
          (if .module == "01340011" then "DOWN" else "UP" end))
  ' multitier/tier1-ltm-ha.conf | grep -v '^#' \
  | python3 sysadmin/monitor_timeline.py
```

```
members down/up over time (1 char = 5 min)
                      10          11          12          13          14          15          16          17          18
                      +----------------------------------------------------------------------------------------------------------
t2_c01_vip:443        |v#^
t2_c02_vip:443        |   v######^
t2_c03_vip:443        |          v#######^
t2_c04_vip:443        |                  v#########^
t2_c05_vip:443        |                                     v###^
t2_c06_vip:443        |                                           v######^
t2_c07_vip:443        |                                                     v####^
t2_c08_vip:443        |                                                           v^
t2_c09_vip:443        |                                                                v###########^
t2_c10_vip:443        |                                                                               v#^
t2_c11_vip:443        |                                                                                        v^
t2_c12_vip:443        |                                                                                               ^
```

Reading the chart: the t2 fanout pool's members went down in a
rolling sequence — `t2_c01` first at ~10:11, recovering within
~9 minutes; `t2_c04` was down for ~46 minutes between 11:45 and
12:31; `t2_c09` for ~60 minutes between 15:31 and 16:31. None of
them were down at the same time, which is exactly what you want
to see — the rollout was sequential rather than a stampede.

The same query against another device's log shows the same pool
from a different observer; line-up the two timelines and HA
disagreement (one device says UP, the other says DOWN) shows
up as missing characters.

---

## 8. Live probes

These queries reach the network and need `--enable-probes`.
The outputs below were captured against a tiny local Python
HTTP server on `127.0.0.1:8001` so the responses are real —
substitute your own backend addresses when running these
against a real box. (`ping` / `portping` semantics are the
same regardless.)

### 8.1 HTTP probe with response capture

**Question.** *"Send the device's HTTP health check at the live
backend, save the response body in the report so the next
operator can read it."*

```
$ f5 query --enable-probes --json '
    url_get("http://127.0.0.1:8001/health") as $r
    | {url: "http://127.0.0.1:8001/health",
       status: $r.status,
       server: http_header($r, "server"),
       content_type: http_header($r, "content-type"),
       body: $r.body,
       reason: $r.reason.kind}
  ' ltm.conf
```

```json
[
  {
    "url": "http://127.0.0.1:8001/health",
    "status": 200,
    "server": "nginx/1.24.0",
    "content_type": "application/json",
    "body": "{\"status\":\"ok\",\"build\":\"v2.4.1\"}\n",
    "reason": "ok"
  }
]
```

`reason.kind` ∈ `ok` / `self_signed` / `expired` /
`hostname_mismatch` / `untrusted_ca` / `connection_error` —
same taxonomy as the device's HTTPS-monitor failure modes.

### 8.2 Reproduce an HTTP monitor with the 5,120-byte ceiling

**Question.** *["Server's healthy, but the monitor still marks
it down. Why?"][q82]* — KB **K3451**: HTTP monitors read at
most 5,120 bytes including headers. If your `recv` pattern lives
past byte 5120, the device never sees it.

[q82]: https://my.f5.com/manage/s/article/K3451

```
$ f5 query --enable-probes --json '
    url_get("http://127.0.0.1:8001/health") as $r
    | { sent: "GET /health",
        recv: "status.*ok",
        truncated_body: $r.body,
        device_would_mark_up:
            (match($r.body, "status.*ok") and $r.status == 200),
        status: $r.status,
        reason: $r.reason.kind }
  ' ltm.conf
```

```json
[
  {
    "sent": "GET /health",
    "recv": "status.*ok",
    "truncated_body": "{\"status\":\"ok\",\"build\":\"v2.4.1\"}\n",
    "device_would_mark_up": true,
    "status": 200,
    "reason": "ok"
  }
]
```

For the full device-faithful version that applies the
**5,120-byte truncation** (`$r.body[0:5120]`) and walks every
`ltm monitor http` against the live pool member, see the KCS
note linked below — this query is the offline-only-tool subset.

The recipe applies the device's 5,120-byte truncation **before**
running the recv regex, so the verdict reflects what the device
sees, not the full response. Detailed walkthrough in the KCS
note [`kcs-howto-reproduce-http-monitor-with-query.md`][q82-kcs].

[q82-kcs]: ../../docs/kcs/kcs-howto-reproduce-http-monitor-with-query.md

### 8.3 Ping / port-ping every pool member

```
$ f5 query --enable-probes --raw '
    "pool,member,host,port,reachable,rtt_ms",
    (.ltm.pool[] as $p
     | $p.members[]
     | csv($p.name, .name, .address, port(.name),
           portping(.address, port(.name)).ok,
           portping(.address, port(.name)).rtt_ms))
  ' ltm.conf
```

Sample output (the fixture's IPs aren't reachable from this
host — the values on a real box are the real reachability):

```
pool,member,host,port,reachable,rtt_ms
web_pool,/Common/web1:80,10.0.1.10,80,true,0.74
web_pool,/Common/web2:80,10.0.1.11,80,true,0.50
api_pool,/Common/api1:8443,10.0.2.20,8443,false,
api_pool,/Common/api2:8443,10.0.2.21,8443,false,
legacy_pool,/Common/legacy1:80,192.168.50.10,80,true,0.60
```

(In this container `127.0.0.1`'s TCP stack accepts on most
ports, so the lab values look optimistic; the real-machine
values would distinguish "the port is genuinely closed" from
"the host is unreachable".)

Standalone form for a CI gate — *fail* the build if any member
can't be reached:

```
$ f5 query --enable-probes --strict --raw '
    .ltm.pool[].members[]
    | select(not portping(.address, port(.name)).ok)
    | tsv(.name, .address, port(.name))
  ' ltm.conf
```

`--strict` exits 2 on any non-empty result (failed probes) and
1 on empty (everything reachable). A green CI run is exit 0 —
i.e., no failed probes.

### 8.4 DNS round-trip on every wide-IP

```
$ f5 query --enable-probes --raw '
    .gtm.wideip[] | tsv(.name, join(dns(.name), ","))
  ' gtm.conf
```

(Outputs the empty list if the FQDN isn't published — the
fixture FQDNs are `www.example.com` / `api.example.com` and
will return whatever IANA's example domains currently resolve
to.)

---

## 9. Migration & bulk edits

All mutating queries print a **unified diff** by default
(dry-run). Add `--write` to print the rewritten SCF to stdout
or `--in-place` to overwrite the input file.

### 9.1 Add an iRule to every pool-backed VS that's missing it

```
$ f5 query '
    .ltm.virtual[]
    | select(.pool != "" and not contains(.rules, "/Common/log_rule"))
    | .rules += "/Common/log_rule"
  ' ltm.conf
```

```diff
--- samples/for_f5_query/ltm.conf
+++ samples/for_f5_query/ltm.conf (modified)
@@ -125,6 +125,7 @@
         /Common/http { }
         /Common/tcp { }
     }
+    rules { /Common/log_rule }
 }
 ltm virtual /Common/api_vs {
     destination /Common/10.0.0.20:443
@@ -146,6 +147,7 @@
     destination /Common/192.168.50.100:80
     ip-protocol tcp
     pool /Common/legacy_pool
+    rules { /Common/log_rule }
 }
 ltm virtual /Common/forwarder_vs {
     destination /Common/0.0.0.0:0
@@ -164,4 +166,5 @@
         /Common/http { }
         /Common/tcp { }
     }
+    rules { /Common/log_rule }
 }
```

Three VSes get the rule attached: `web_secure_vs`, `legacy_vs`,
`vpn_vs`. `api_vs` and `web_vs` already had it; `forwarder_vs`
has no pool so the predicate skipped it.

### 9.2 Subnet renumber across the box

```
$ f5 query '
    .ltm.virtual[]
      | select(in_cidr(.destination, "192.168.0.0/16"))
      | .destination |= ip("10.50.0.0/16", .) ;
    .ltm.node[]
      | select(in_cidr(.address, "192.168.0.0/16"))
      | .address |= ip("10.50.0.0/16", .) ;
    .ltm.pool[].members[]
      | select(in_cidr(.address, "192.168.0.0/16"))
      | .address |= ip("10.50.0.0/16", .)
  ' ltm.conf
```

```diff
--- samples/for_f5_query/ltm.conf
+++ samples/for_f5_query/ltm.conf (modified)
@@ -11,7 +11,7 @@
     address 10.0.2.21
 }
 ltm node /Common/legacy1 {
-    address 192.168.50.10
+    address 10.50.50.10
 }
 ltm pool /Common/web_pool {
     members {
@@ -38,7 +38,7 @@
 ltm pool /Common/legacy_pool {
     members {
         /Common/legacy1:80 {
-            address 192.168.50.10
+            address 10.50.50.10
         }
     }
     monitor /Common/http
@@ -143,7 +143,7 @@
     }
 }
 ltm virtual /Common/legacy_vs {
-    destination /Common/192.168.50.100:80
+    destination /Common/10.50.50.100:80
     ip-protocol tcp
     pool /Common/legacy_pool
 }
```

`ip(net, .)` rebases each source address into `net` while
preserving the host bits, partition prefix, route domain, and
port. The node, the pool-member entry, and the VS destination
all move in one pass.

### 9.3 Rename one object, every reference follows

```
$ f5 query 'rename("/Common/web_pool", "/Common/web_primary_pool")' ltm.conf
```

```
renamed '/Common/web_pool' -> '/Common/web_primary_pool' (4 occurrence(s))
--- samples/for_f5_query/ltm.conf
+++ samples/for_f5_query/ltm.conf (modified)
@@ -13,7 +13,7 @@
 ltm node /Common/legacy1 {
     address 192.168.50.10
 }
-ltm pool /Common/web_pool {
+ltm pool /Common/web_primary_pool {
     members {
         /Common/web1:80 {
             address 10.0.1.10
@@ -88,7 +88,7 @@
 when HTTP_REQUEST {
     set tgt [class match -value [HTTP::path] starts_with routing_map]
     if { $tgt ne "" } {
         pool $tgt
     } else {
-        pool /Common/web_pool
+        pool /Common/web_primary_pool
     }
 }
…
```

Four occurrences: the pool header, two VS `pool` properties, and
the **iRule body** fallback inside `api_router_rule`. Identity
writes auto-route through the rename engine so iRule body refs
move with the header.

### 9.4 Emit a tmsh-delta script for change control

```
$ f5 query --write --format tmsh-delta --transaction \
    '.ltm.virtual["/Common/api_vs"].destination |= with_port(., 8443)' \
    ltm.conf
```

```
cli transaction
tmsh modify ltm virtual /Common/api_vs { destination /Common/10.0.0.20:8443 pool /Common/api_pool profiles replace-all-with { /Common/http { } /Common/tcp { } } rules replace-all-with { /Common/ip_blocklist_rule /Common/api_auth_rule /Common/api_router_rule /Common/log_rule } }
submit-transaction
```

Pipe to `tmsh load /sys config from-terminal merge` on the
device for an atomic apply — see KB **K13030** for transaction
semantics.

---

## 10. Localhost / self-IP security audits

A security check pattern popularised by the open-source
**BIG-IP Localhost Security Checker** ([bigipck][bigipck-repo]):
review `net self` allow-services for risk; flag any object,
pool member, or iRule body that references a loopback /
wildcard address; build a data-group usage tree.

[bigipck-repo]: https://github.com/example/bigipck

The fixture [`sysadmin/lab_localhost.conf`](sysadmin/lab_localhost.conf)
carries the typical mix of "safe", "risky", and "default" `net
self` objects, plus a pool member on `127.0.0.1` and an iRule
that does `node 127.0.0.1 8080`.

### 10.1 `net self` allow-service audit

**Rule.** F5 best practice (KB **K17333**, "BIG-IP best practice
for self IP allow services") is `allow-service none` on
public-facing self-IPs, and only the management services on
internal self-IPs. `allow-service all` opens the management
plane on that interface.

```
$ f5 query --raw '
    .net.self[]
    | join(."allow-service", ",") as $svc
    | tsv(.name, .address, .vlan, $svc,
          (if $svc == "none" then "OK"
           elif $svc == "all" then "RISK (open)"
           elif $svc == "default" then "default (review)"
           else "custom" end))
  ' sysadmin/lab_localhost.conf
```

```
198.51.100.5	198.51.100.5/24	/Common/external	none	OK
10.1.0.5	10.1.0.5/24	/Common/internal	all	RISK (open)
10.2.0.5	10.2.0.5/24	/Common/internal	default	default (review)
```

`10.1.0.5` opens every management service on an internal VLAN
— almost certainly a misconfig. `10.2.0.5` uses `default`,
which is "tcp:domain udp:domain tcp:f5-iquery tcp:snmp tcp:https"
on most platforms — review whether each of those genuinely
needs to be reachable here.

### 10.2 Loopback / wildcard pool members

**Rule.** A pool member on `127.0.0.1` typically points at a
control-plane daemon (`bigd`, `iControl`); a wildcard
`0.0.0.0` member usually means "this pool isn't load-balancing,
it's used as a tag on a wildcard VS." Both are legitimate but
**should** be deliberate.

```
$ f5 query --raw '
    .ltm.pool[].members[]
    | select(is_loopback(.address) or is_unspecified(.address))
    | tsv(.name, .address, port(.name),
          (if is_loopback(.address) then "LOOPBACK"
           else "WILDCARD" end))
  ' sysadmin/lab_localhost.conf
```

```
/Common/loopback_n:8080	127.0.0.1	8080	LOOPBACK
/Common/anyhost:80	0.0.0.0	80	WILDCARD
```

### 10.3 iRule bodies that touch a loopback / wildcard address

**Rule.** F5 KB **K05413010** notes that `node 127.0.0.1` /
`pool { 127.0.0.1 8080 }` etc. require
`tmm.tcl.rule.node.allow_loopback_addresses=true` on modern
TMOS — flagging every such iRule lets you decide whether the
db-key change is intentional.

```
$ f5 query --raw '
    .ltm.rule[]
    | select(match(.body, "127\\.0\\.0\\.|::1|localhost"))
    | tsv(.name, "uses localhost in body")
  ' sysadmin/lab_localhost.conf
```

```
loopback_node_rule	uses localhost in body
```

Combine with `referenced_by(.)` to trace each match back to the
VSes that attach the rule — i.e., the listeners whose traffic
hits the loopback path.

### 10.4 Data-group usage tree

**Rule.** Before deleting / re-keying a data-group, build the
full reach: which iRules reference it, which VSes attach those
iRules. This recipe runs against `ltm.conf`:

```
$ f5 query --raw '
    .ltm["data-group"][] as $dg
    | (.ltm.rule[]
       | select(any(.refs."data-groups"[] == $dg."full-path"))) as $r
    | (.ltm.virtual[]
       | select(any(.rules[] == $r."full-path"))) as $vs
    | tsv($dg.name, $r.name, $vs.name, $vs.destination)
  ' ltm.conf
```

```
banned_ips	ip_blocklist_rule	api_vs	/Common/10.0.0.20:443
api_keys	api_auth_rule	api_vs	/Common/10.0.0.20:443
routing_map	api_router_rule	api_vs	/Common/10.0.0.20:443
```

Three rows: each of the three data-groups is reachable from one
iRule, which is attached to one VS (`api_vs`). Pipe through
`column -t -s$'\t'` for a tree-shaped terminal view.

---

## 11. Training-question crosswalk

The 301A LTM Specialist labs and the F5 Agility 2018
"Beginning LTM Implementation" labs ask students to answer
questions about a running BIG-IP. Many of those questions
become one-liners with `f5 query`. A representative mapping:

| Lab module                                  | Lab question                                                          | Equivalent `f5 query` recipe        |
|---------------------------------------------|-----------------------------------------------------------------------|-------------------------------------|
| 301A — Monitors and Status, Lab 1, Q2-Q3 ([link][lab041]) | "What are the node / pool / VS statuses after applying a monitor?" | §7.1, plus live §8.3 portping       |
| 301A — Load Balancing and Pools ([link][lab071]) | "How many connections has each pool member taken? Is the ratio correct?" | Stats are live-only on the device, but §1.2 + §1.5 give the static "what's configured" view that the question depends on |
| 301A — Load Balancing and Pools ([link][lab071]) | "Which member isn't taking connections (priority groups)?"          | §1.5 pool JSON shows `priority-group` / `ratio` per member |
| 301A — Load Balancing and Pools ([link][lab071]) | "Are there any persistence records?"                                | §1.6 persistence rollup             |
| Agility 2018 — Load Balancing & Monitoring  | "How do I see which monitor each pool member uses?"                  | §1.2 pool/member CSV                |
| Agility 2018 — Traffic Management           | "Which iRule routes which URI?"                                      | §4.1 + §4.2 iRule edges; §4.4 body-regex audit |
| 201 TMOS Administration — VS / pool basics ([link][lab201]) | "List every VS along with its pool and destination."         | §1.1                                |
| LTM 201 study guide ([link][lab201]) | "Identify VSes without a default pool."                          | §3.2                                |
| F5 community thread ([link][q21])           | "Get VSes associated with a SNAT pool."                              | §2.1 `references_to(SNAT)`          |
| F5 community thread ([link][q23])           | "Find which F5 has a particular URL across hundreds of boxes."       | §2.3 (with glob)                    |
| F5 community thread ([link][q71])           | "Gather virtuals/pools offline and how long."                        | §7.1, §7.2, §7.7 timeline           |
| F5 KB **K3451**                             | "Monitor's down even though the backend is healthy" — 5,120-byte rule | §8.2                                |
| Kareem CCIE blog ([link][q31])              | "Identify unused objects (orphans)."                                 | §3.1                                |
| `bigipck` security audit                    | "Find net-self objects whose allow-service isn't `none`."            | §10.1                               |
| `bigipck` security audit                    | "Find iRule bodies / pool members that touch loopback / wildcard."   | §10.2, §10.3                        |
| `bigipck` security audit                    | "Build a data-group usage tree (DG → iRule → VS)."                   | §10.4                               |

[lab041]: https://clouddocs.f5.com/training/community/f5cert/html/class8/module04/lab1.html
[lab071]: https://clouddocs.f5.com/training/community/f5cert/html/class8/module07/lab1.html
[lab201]: https://f5-201-certification.readthedocs.io/en/latest/class1/module7/lab1.html

The 301A answer keys live in Appendix I of each lab module; for
the runtime statistics questions (connection counts, current
persistence records) you still need `tmsh show ltm pool / VS`
on a live device. `f5 query` is the offline-config / log /
probe tool — pair it with `tmsh show` for the full picture.

---

## 12. Cheat sheet

### Common flags

| Flag                       | Purpose                                                                  |
|----------------------------|--------------------------------------------------------------------------|
| `--raw`                    | Scalars one per line, no quoting (TSV-friendly)                          |
| `--json`                   | JSON array output                                                        |
| `--paths-only`             | Print only the full-path of each result                                  |
| `--scf`                    | Render results as SCF stanzas when possible                              |
| `--enable-probes`          | Enable live network builtins (`ping`, `portping`, `url_get`, `tls_handshake`, ...) |
| `--input-json N=PATH`      | Bind a JSON sidecar to `$N`                                              |
| `--input-jsonl N=PATH`     | Bind a JSON-Lines sidecar to `$N`                                        |
| `--input-csv N=PATH`       | Bind a CSV sidecar to `$N`                                               |
| `--input-f5log N=PATH`     | Bind a parsed BIG-IP log to `$N`                                         |
| `--name V=PATH`            | Bind a positional source to `$V` for cross-source addressing              |
| `--merge`                  | Treat every positional source as one namespace                           |
| `-f FILE.f5q`              | Read the query from a file                                               |
| `--strict`                 | Exit 1 if read-only produced no values / 2 if mutating didn't match      |
| `--write` / `--in-place`   | For mutating queries: print rewritten SCF / overwrite the file           |
| `--format tmsh-delta`      | Emit just changed objects as `tmsh create / modify / delete`             |
| `--transaction`            | Wrap tmsh output in `cli transaction ... submit-transaction`             |

### Common builtins

| Builtin                            | What it does                                                     |
|------------------------------------|------------------------------------------------------------------|
| `host(d)` / `port(d)`              | Pull the address / port half of a destination                    |
| `partition(p)` / `basename(p)`     | Partition / leaf name of a full-path                             |
| `in_cidr(addr, net)`               | Membership test against a CIDR                                   |
| `ip(net, src)`                     | Rebase `src` into `net`, preserving host bits / partition / RD / port |
| `is_public / is_private / is_loopback / is_reserved` | Address-class predicates                       |
| `is_unspecified` / `is_wildcard_port` | Wildcard `0.0.0.0` / `:0` predicates                          |
| `referenced_by(o)` / `references_to(p)` | Reverse references                                           |
| `refs(o)`                          | Forward references                                               |
| `ping(ip)` / `portping(ip, port)`  | Live L3 / L4 probes (`--enable-probes`)                          |
| `url_get(url)` / `url_head(url)`   | Live HTTP — `{status, headers, body, reason, error}`             |
| `tls_handshake(host, port[, sni])` | Live TLS handshake — peer cert + verify status                   |
| `dns(name)` / `rev_dns(ip)`        | Forward / reverse DNS                                            |
| `f5log_load(path)`                 | Parse `/var/log/ltm`-style file into events                      |
| `x509_from_config(c)` / `x509_eq(a, b)` | Cert projection / cert-identity equality                    |
| `tsv(...)` / `csv(...)`            | One row per stream broadcast; CSV is RFC-4180-quoted             |
| `match(s, regex)` / `sub` / `gsub` | Regex predicate / single-sub / global-sub                        |
| `rename(old, new)` / `rename_partition(old, new)` | Cascading rename across header + every ref         |

### When to reach for which verb

- **`f5 query`** — "for every object matching this filter, project / set / append this field."
- **`f5 grep`** — "which objects are related to X?" (multi-hop walks).
- **`f5 rename`** — "swap object name X for Y everywhere" (shorthand for one query).
- **`f5 explain` / `f5 explain-flow`** — "describe how the device handles this request / PCAP flow." Different shape of answer; see the dedicated KCS notes.

## Further reading

- [`docs/f5-query-examples.md`](../../docs/f5-query-examples.md) — the full DSL examples cookbook.
- [`docs/kcs/kcs-howto-audit-config-with-query.md`](../../docs/kcs/kcs-howto-audit-config-with-query.md) — orphan / leak / policy audits.
- [`docs/kcs/kcs-howto-reproduce-http-monitor-with-query.md`](../../docs/kcs/kcs-howto-reproduce-http-monitor-with-query.md) — full HTTP-monitor reproduction (5,120-byte rule).
- [`docs/kcs/kcs-howto-audit-server-certs-with-query.md`](../../docs/kcs/kcs-howto-audit-server-certs-with-query.md) — cert-fleet audit.
- [`docs/references/f5_query/dsl.md`](../../docs/references/f5_query/dsl.md) — grammar reference.
- [`docs/references/f5_query/builtins.md`](../../docs/references/f5_query/builtins.md) — every builtin with signature + examples.
- F5 KBs referenced: **K9970** (log-code reference), **K2167**, **K3451**, **K12531**, **K15212**, **K13030**.

[Q-Forum-Pool]: https://community.f5.com/discussions/technicalforum/what-tmsh-command-do-i-use-to-view-pool-members-and-their-addresses/199927
[Q-Forum-SNAT]: https://community.f5.com/discussions/technicalforum/how-to-get-virtual-servers-associated-with-a-specific-snat-pool/320047
[Q-Forum-URL]:  https://community.f5.com/discussions/technicalforum/f5-related-questionhow-to-find-how-particular-url-in-f5-if-we-have-hundreds-of-f/308040
[Q-Forum-Offline]: https://community.f5.com/t5/technical-forum/gather-a-list-of-virtuals-and-or-pools-that-are-offline-state/td-p/26436
[Q-Forum-Orphan]: https://www.kareemccie.com/2020/05/how-to-identify-unused-objects-in-f5.html
[Q-Forum-DG]:   https://community.f5.com/discussions/technicalforum/check-via-irule-if-data-group-exists/257590
