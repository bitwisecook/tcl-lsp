# `f5 query` — examples cookbook

A walkthrough of the `f5 query` DSL on small, self-contained SCF
fixtures. Every example shows the input config, the exact command, and
the literal output from the build.

The doc is organised in three layers:

1. **LTM** — single-config queries on `ltm.conf`.
2. **GTM** — `gtm.conf` alone, then **GTM + LTM together** using
   `$ltm` / `$gtm` variables (separate documents, addressed by name).
3. **APM + LTM merged** — APM access profiles live in their own SCF
   but are referenced by LTM virtuals; `--merge` unifies the
   namespace so references walk transparently.

---

## Configs used

### `ltm.conf`

```scf
ltm node /Common/web1    { address 10.0.1.10 }
ltm node /Common/web2    { address 10.0.1.11 }
ltm node /Common/api1    { address 10.0.2.20 }
ltm node /Common/api2    { address 10.0.2.21 }
ltm node /Common/legacy1 { address 192.168.50.10 }

ltm pool /Common/web_pool {
    members {
        /Common/web1:80 { address 10.0.1.10 }
        /Common/web2:80 { address 10.0.1.11 }
    }
    monitor /Common/http
}
ltm pool /Common/api_pool {
    members {
        /Common/api1:8443 { address 10.0.2.20 }
        /Common/api2:8443 { address 10.0.2.21 }
    }
    monitor /Common/https
}
ltm pool /Common/legacy_pool {
    members {
        /Common/legacy1:80 { address 192.168.50.10 }
    }
    monitor /Common/http
}
ltm pool /Common/unused_pool {
    monitor /Common/tcp
}

ltm rule /Common/log_rule {
when HTTP_REQUEST {
    log local0. "[HTTP::host] [HTTP::uri]"
}
}
ltm rule /Common/maintenance_rule {
when HTTP_REQUEST {
    HTTP::respond 503 content "down for maintenance"
}
}

ltm virtual /Common/web_vs {
    destination /Common/10.0.0.10:80
    pool /Common/web_pool
    rules { /Common/log_rule }
}
ltm virtual /Common/web_secure_vs {
    destination /Common/10.0.0.10:443
    pool /Common/web_pool
}
ltm virtual /Common/api_vs {
    destination /Common/10.0.0.20:443
    pool /Common/api_pool
    rules { /Common/log_rule }
}
ltm virtual /Common/legacy_vs {
    destination /Common/192.168.50.100:80
    pool /Common/legacy_pool
}
ltm virtual /Common/forwarder_vs {
    destination /Common/0.0.0.0:0
}
ltm virtual /Common/vpn_vs {
    destination /Common/10.0.0.30:443
    pool /Common/api_pool
    profiles { /Common/employee_login_profile { } }
}
```

### `gtm.conf`

```scf
gtm datacenter /Common/dc-east { }
gtm datacenter /Common/dc-west { }

gtm server /Common/bigip-east {
    datacenter /Common/dc-east
    addresses { 10.0.0.1 { } }
    virtual-servers {
        /Common/web_vs        { destination 10.0.0.10:80 }
        /Common/web_secure_vs { destination 10.0.0.10:443 }
        /Common/api_vs        { destination 10.0.0.20:443 }
    }
}
gtm server /Common/bigip-west {
    datacenter /Common/dc-west
    addresses { 10.1.0.1 { } }
    virtual-servers {
        /Common/web_vs { destination 10.1.0.10:80 }
    }
}

gtm pool a /Common/example_app_pool {
    members {
        /Common/bigip-east:/Common/web_vs { order 0 }
        /Common/bigip-west:/Common/web_vs { order 1 }
    }
    monitor /Common/gateway_icmp
}
gtm pool a /Common/api_app_pool {
    members {
        /Common/bigip-east:/Common/api_vs { order 0 }
    }
    monitor /Common/gateway_icmp
}

gtm wideip a /Common/www.example.com {
    pools { /Common/example_app_pool { } }
}
gtm wideip a /Common/api.example.com {
    pools { /Common/api_app_pool { } }
}
```

### `apm.conf`

```scf
apm policy policy-item /Common/employee_login_ent          { caption Start ;        agents { } }
apm policy policy-item /Common/employee_login_logon_page   { caption "Logon Page" ; agents { /Common/employee_login_logon_page_ag   { type logon-page } } }
apm policy policy-item /Common/employee_login_localdb_auth { caption "LocalDB Auth";agents { /Common/employee_login_localdb_auth_ag { type aaa-localdb } } }
apm policy policy-item /Common/employee_login_end_allow    { caption Allow ;        agents { } }
apm policy policy-item /Common/employee_login_end_deny     { caption Deny ;         agents { } }

apm policy access-policy /Common/employee_login {
    default-ending /Common/employee_login_end_deny
    items {
        /Common/employee_login_ent          { }
        /Common/employee_login_logon_page   { }
        /Common/employee_login_localdb_auth { }
        /Common/employee_login_end_allow    { }
        /Common/employee_login_end_deny     { }
    }
    start-item /Common/employee_login_ent
}

apm profile access /Common/employee_login_profile {
    access-policy /Common/employee_login
}
```

The LTM file's `vpn_vs` attaches the APM profile via its `profiles` list.

---

# Part 1 — LTM

## L1. List every virtual server

```
$ f5 query --raw '.ltm.virtual[].name' ltm.conf
```

```
web_vs
web_secure_vs
api_vs
legacy_vs
forwarder_vs
vpn_vs
```

`--raw` prints scalar values one per line, no quoting.

## L2. VS + destination + pool (one row per VS)

```
$ f5 query --raw '.ltm.virtual[] | tsv(.name, .destination, .pool)' ltm.conf
```

```
web_vs          /Common/10.0.0.10:80        /Common/web_pool
web_secure_vs   /Common/10.0.0.10:443       /Common/web_pool
api_vs          /Common/10.0.0.20:443       /Common/api_pool
legacy_vs       /Common/192.168.50.100:80   /Common/legacy_pool
forwarder_vs    /Common/0.0.0.0:0
vpn_vs          /Common/10.0.0.30:443       /Common/api_pool
```

`tsv(...)` joins arguments with tabs. `forwarder_vs` has no default
pool, so the last cell is empty.

## L3. VS × pool member joined table

Stream broadcast: when one argument to `tsv()` is a stream, scalars are
replicated for each element.

```
$ f5 query --raw '.ltm.virtual[]
                  | select(.pool != "")
                  | tsv(.name, .destination, .pool,
                        .pool.members[].address,
                        port(.pool.members[].name))' ltm.conf
```

```
web_vs           /Common/10.0.0.10:80       /Common/web_pool     10.0.1.10       80
web_vs           /Common/10.0.0.10:80       /Common/web_pool     10.0.1.11       80
web_secure_vs    /Common/10.0.0.10:443      /Common/web_pool     10.0.1.10       80
web_secure_vs    /Common/10.0.0.10:443      /Common/web_pool     10.0.1.11       80
api_vs           /Common/10.0.0.20:443      /Common/api_pool     10.0.2.20       8443
api_vs           /Common/10.0.0.20:443      /Common/api_pool     10.0.2.21       8443
legacy_vs        /Common/192.168.50.100:80  /Common/legacy_pool  192.168.50.10   80
vpn_vs           /Common/10.0.0.30:443      /Common/api_pool     10.0.2.20       8443
vpn_vs           /Common/10.0.0.30:443      /Common/api_pool     10.0.2.21       8443
```

## L4. Same join as JSON, via let-binding (`as $vs`)

```
$ f5 query '.ltm.virtual[] | select(.pool != "") as $vs
            | .pool.members[]
            | {vs: $vs.name, dest: $vs.destination, pool: $vs.pool,
               addr: .address, port: port(.name)}' ltm.conf
```

```json
[
  { "vs": "web_vs",         "dest": "/Common/10.0.0.10:80",       "pool": "/Common/web_pool",     "addr": "10.0.1.10", "port": 80 },
  { "vs": "web_vs",         "dest": "/Common/10.0.0.10:80",       "pool": "/Common/web_pool",     "addr": "10.0.1.11", "port": 80 },
  { "vs": "web_secure_vs",  "dest": "/Common/10.0.0.10:443",      "pool": "/Common/web_pool",     "addr": "10.0.1.10", "port": 80 },
  { "vs": "web_secure_vs",  "dest": "/Common/10.0.0.10:443",      "pool": "/Common/web_pool",     "addr": "10.0.1.11", "port": 80 },
  { "vs": "api_vs",         "dest": "/Common/10.0.0.20:443",      "pool": "/Common/api_pool",     "addr": "10.0.2.20", "port": 8443 },
  { "vs": "api_vs",         "dest": "/Common/10.0.0.20:443",      "pool": "/Common/api_pool",     "addr": "10.0.2.21", "port": 8443 },
  { "vs": "legacy_vs",      "dest": "/Common/192.168.50.100:80",  "pool": "/Common/legacy_pool",  "addr": "192.168.50.10", "port": 80 },
  { "vs": "vpn_vs",         "dest": "/Common/10.0.0.30:443",      "pool": "/Common/api_pool",     "addr": "10.0.2.20", "port": 8443 },
  { "vs": "vpn_vs",         "dest": "/Common/10.0.0.30:443",      "pool": "/Common/api_pool",     "addr": "10.0.2.21", "port": 8443 }
]
```

JSON output is also `{ name, destination, pool }` — field-name
shorthand is sugar for `{ name: .name, destination: .destination, ... }`.

## L5. VSes listening on port 443

```
$ f5 query --raw '.ltm.virtual[] | select(port(.destination) == 443) | .name' ltm.conf
```

```
web_secure_vs
api_vs
vpn_vs
```

## L6. VSes whose destination lies in a CIDR

```
$ f5 query --raw '.ltm.virtual[] | select(in_cidr(.destination, "192.168.0.0/16")) | tsv(.name, .destination)' ltm.conf
```

```
legacy_vs   /Common/192.168.50.100:80
```

## L7. Regex subscript — VSes whose path matches a pattern

```
$ f5 query --raw '.ltm.virtual["~^/Common/web"] | .name' ltm.conf
```

```
web_vs
web_secure_vs
```

`["~..."]` is a regex subscript; the pattern is matched against the
full path.

## L8. VSes with a specific iRule attached

```
$ f5 query --raw '.ltm.virtual[] | select(contains(.rules, "/Common/log_rule")) | .name' ltm.conf
```

```
web_vs
api_vs
```

## L9. Orphan pools (zero references)

```
$ f5 query --raw '.ltm.pool[] | select(referenced_by(.) | count == 0) | .name' ltm.conf
```

```
unused_pool
```

## L10. What references a pool?

```
$ f5 query --raw 'references_to("/Common/web_pool")' ltm.conf
```

```
/Common/web_secure_vs
/Common/web_vs
```

## L11. Count VSes

```
$ f5 query '[.ltm.virtual[]] | count' ltm.conf
```

```
6
```

`[ ... ]` collects a stream into a list; `count` is then a list builtin.

## L12. Distinct pools attached to a VS, sorted

```
$ f5 query '[.ltm.virtual[].pool | select(. != "")] | unique | sort' ltm.conf
```

```
/Common/api_pool
/Common/legacy_pool
/Common/web_pool
```

## L13. Pool sizes as JSON

```
$ f5 query '.ltm.pool[] | {name, member_count: ([.members[]] | count)}' ltm.conf
```

```json
[
  { "name": "web_pool",     "member_count": 2 },
  { "name": "api_pool",     "member_count": 2 },
  { "name": "legacy_pool",  "member_count": 1 },
  { "name": "unused_pool",  "member_count": 0 }
]
```

---

## Rewrites (LTM)

All rewriting examples below print a **unified diff** by default —
dry-run preview. Pass `--write` to print the rewritten SCF to stdout,
or `--in-place` to overwrite the input file.

## L14. Change a VS port

```
$ f5 query '.ltm.virtual["/Common/web_vs"].destination |= with_port(., 8080)' ltm.conf
```

```diff
--- ltm.conf
+++ ltm.conf (modified)
@@ -57,7 +57,7 @@
 }
 ltm virtual /Common/web_vs {
-    destination /Common/10.0.0.10:80
+    destination /Common/10.0.0.10:8080
     ip-protocol tcp
     mask 255.255.255.255
     pool /Common/web_pool
```

`|=` re-binds `.` to the current field, runs the right-hand expression,
and stores the result back.

## L15. Attach an iRule to every pool-backed VS missing it

```
$ f5 query '.ltm.virtual[]
            | select(.pool != "" and not contains(.rules, "/Common/log_rule"))
            | .rules += "/Common/log_rule"' ltm.conf
```

```diff
--- ltm.conf
+++ ltm.conf (modified)
@@ -78,6 +78,7 @@
         /Common/http { }
         /Common/tcp { }
     }
+    rules { /Common/log_rule }
 }
 ltm virtual /Common/api_vs {
@@ -96,6 +97,7 @@
     destination /Common/192.168.50.100:80
     ip-protocol tcp
     pool /Common/legacy_pool
+    rules { /Common/log_rule }
 }
 ltm virtual /Common/forwarder_vs {
@@ -114,4 +116,5 @@
         /Common/http { }
         /Common/tcp { }
     }
+    rules { /Common/log_rule }
 }
```

`+=` on a list field appends; `+=` on a scalar concatenates.

## L16. Subnet renumber across the whole config (one query, three pipelines)

`;` separates pipelines that share the same root.

```
$ f5 query '
    .ltm.virtual[]
      | select(in_cidr(.destination, "192.168.0.0/16"))
      | .destination |= ip("10.50.0.0/16", .) ;
    .ltm.node[]
      | select(in_cidr(.address, "192.168.0.0/16"))
      | .address |= ip("10.50.0.0/16", .) ;
    .ltm.pool[] | .members[]
      | select(in_cidr(.address, "192.168.0.0/16"))
      | .address |= ip("10.50.0.0/16", .)
  ' ltm.conf
```

```diff
--- ltm.conf
+++ ltm.conf (modified)
@@ -11,7 +11,7 @@
     address 10.0.2.21
 }
 ltm node /Common/legacy1 {
-    address 192.168.50.10
+    address 10.50.50.10
 }
@@ -38,7 +38,7 @@
 ltm pool /Common/legacy_pool {
     members {
         /Common/legacy1:80 {
-            address 192.168.50.10
+            address 10.50.50.10
         }
     }
@@ -93,7 +93,7 @@
 }
 ltm virtual /Common/legacy_vs {
-    destination /Common/192.168.50.100:80
+    destination /Common/10.50.50.100:80
     ip-protocol tcp
     pool /Common/legacy_pool
```

Host bits and ports preserved by `ip(net, .)`.

## L17. Rename — three equivalent forms

```
$ f5 query 'rename("/Common/web_pool", "/Common/web_primary_pool")' ltm.conf
$ f5 query '.ltm.pool["/Common/web_pool"].name = "/Common/web_primary_pool"' ltm.conf
$ f5 query '.ltm.pool["/Common/web_pool"].name |= with_name(., "web_primary_pool")' ltm.conf
```

All three produce the same diff (stderr first, stdout second):

```
renamed '/Common/web_pool' -> '/Common/web_primary_pool' (3 occurrence(s))
```

```diff
--- ltm.conf
+++ ltm.conf (modified)
@@ -13,7 +13,7 @@
 ltm node /Common/legacy1 {
     address 192.168.50.10
 }
-ltm pool /Common/web_pool {
+ltm pool /Common/web_primary_pool {
     members {
         /Common/web1:80 {
             address 10.0.1.10
@@ -60,7 +60,7 @@
     destination /Common/10.0.0.10:80
     ip-protocol tcp
     mask 255.255.255.255
-    pool /Common/web_pool
+    pool /Common/web_primary_pool
     profiles {
         /Common/http { }
         /Common/tcp { }
@@ -73,7 +73,7 @@
     destination /Common/10.0.0.10:443
     ip-protocol tcp
     mask 255.255.255.255
-    pool /Common/web_pool
+    pool /Common/web_primary_pool
```

Identity-field writes auto-route through the rename engine, so the
two VS `pool` references follow the header. `rename()` is the most
direct form; `.name = ...` and `with_name(., ...)` exist for cases
where you compute the new name as part of a larger pipeline.

## L18. Move a family of objects by prefix

```
$ f5 query 'rename_prefix("/Common/legacy_", "/Legacy/legacy_")' ltm.conf
```

```
renamed 'prefix /Common/legacy_' -> '/Legacy/legacy_' (3 occurrence(s))
```

```diff
--- ltm.conf
+++ ltm.conf (modified)
@@ -35,7 +35,7 @@
     monitor /Common/https
 }
-ltm pool /Common/legacy_pool {
+ltm pool /Legacy/legacy_pool {
     members {
         /Common/legacy1:80 {
             address 192.168.50.10
@@ -92,10 +92,10 @@
         /Common/log_rule
     }
 }
-ltm virtual /Common/legacy_vs {
+ltm virtual /Legacy/legacy_vs {
     destination /Common/192.168.50.100:80
     ip-protocol tcp
-    pool /Common/legacy_pool
+    pool /Legacy/legacy_pool
 }
```

Note `legacy1` (the node) is *not* renamed — the prefix
`/Common/legacy_` has a trailing underscore, and `legacy1` doesn't
match. Choose the prefix to fit the boundary you want.

For whole-partition moves (every object under `/Common/`), use
`rename_partition("Tenant_A", "Tenant_B")` instead.

## L19. `--strict` — fail on zero-match writes

```
$ f5 query --strict 'rename("/Common/does_not_exist", "/Common/x")' ltm.conf
error: --strict: mutating query produced no textual change (no matches).  Check the path / predicate.
$ echo $?
2
```

Without `--strict`, the same query exits `1` silently. Use `--strict`
in CI to catch broken queries.

---

## Output formats (LTM)

The same rename, four ways:

```
RENAME='rename("/Common/legacy_pool", "/Common/legacy_app_pool")'
```

### L20a. Default — SCF unified-diff preview

```
$ f5 query "$RENAME" ltm.conf
```

```
renamed '/Common/legacy_pool' -> '/Common/legacy_app_pool' (2 occurrence(s))
--- ltm.conf
+++ ltm.conf (modified)
@@ -35,7 +35,7 @@
     monitor /Common/https
 }
-ltm pool /Common/legacy_pool {
+ltm pool /Common/legacy_app_pool {
     members {
         /Common/legacy1:80 {
@@ -95,7 +95,7 @@
 ltm virtual /Common/legacy_vs {
     destination /Common/192.168.50.100:80
     ip-protocol tcp
-    pool /Common/legacy_pool
+    pool /Common/legacy_app_pool
 }
```

### L20b. `--write` — full rewritten SCF on stdout

```
$ f5 query --write "$RENAME" ltm.conf
```

Prints the entire rewritten file (117 lines). Original layout,
whitespace, and comments are preserved — token-bounded rewrite, not a
re-render. Use `--in-place` to overwrite the file directly.

### L20c. `--format tmsh` — full tmsh re-render

```
$ f5 query --write --format tmsh "$RENAME" ltm.conf
```

```
tmsh modify ltm node /Common/web1 { address 10.0.1.10 }
tmsh modify ltm node /Common/web2 { address 10.0.1.11 }
tmsh modify ltm node /Common/api1 { address 10.0.2.20 }
tmsh modify ltm node /Common/api2 { address 10.0.2.21 }
tmsh modify ltm node /Common/legacy1 { address 192.168.50.10 }
tmsh modify ltm pool /Common/web_pool { monitor /Common/http members { ... } }
tmsh modify ltm pool /Common/api_pool { monitor /Common/https members { ... } }
tmsh modify ltm pool /Common/legacy_app_pool { monitor /Common/http members { /Common/legacy1:80 { address 192.168.50.10 } } }
...
```

(26 lines total — every object becomes a `tmsh modify`. Best for
re-baselining on a device that already has the same skeleton.)

### L20d. `--format tmsh-delta` — only changed objects

```
$ f5 query --write --format tmsh-delta "$RENAME" ltm.conf
```

```
tmsh create ltm pool /Common/legacy_app_pool { monitor /Common/http members { /Common/legacy1:80 { address 192.168.50.10 } } }
tmsh modify ltm virtual /Common/legacy_vs { destination /Common/192.168.50.100:80 pool /Common/legacy_app_pool }
tmsh delete ltm pool /Common/legacy_pool
```

A rename is modelled at the tmsh layer as `create new + delete old`;
the `legacy_vs` whose `pool` ref changed gets a `modify`. The
unchanged `web_pool`, `api_pool`, etc. are not emitted.

### L20e. `--format tmsh-delta --transaction` — atomic surgical apply

```
$ f5 query --write --format tmsh-delta --transaction "$RENAME" ltm.conf
```

```
cli transaction
tmsh create ltm pool /Common/legacy_app_pool { monitor /Common/http members { /Common/legacy1:80 { address 192.168.50.10 } } }
tmsh modify ltm virtual /Common/legacy_vs { destination /Common/192.168.50.100:80 pool /Common/legacy_app_pool }
tmsh delete ltm pool /Common/legacy_pool
submit-transaction
```

The combo for change-controlled surgical apply: a single atomic block
containing only the lines that changed.

---

# Part 2 — GTM, alone and with LTM

## G1. GTM wide-IPs

```
$ f5 query --raw '.gtm.wideip[].name' gtm.conf
```

```
www.example.com
api.example.com
```

## G2. GTM pools and their members

GTM pool members are compound `<server>:<vs>` path strings, not nested
objects. Stream them and split when you need the parts.

```
$ f5 query --raw '.gtm.pool[] | tsv(.name, .members[])' gtm.conf
```

```
example_app_pool    /Common/bigip-east:/Common/web_vs
example_app_pool    /Common/bigip-west:/Common/web_vs
api_app_pool        /Common/bigip-east:/Common/api_vs
```

## G3. Wide-IP → GTM pool (auto-deref)

```
$ f5 query --raw '.gtm.wideip[] | tsv(.name, .pools[].name)' gtm.conf
```

```
www.example.com   example_app_pool
api.example.com   api_app_pool
```

`.pools[]` on a wide-IP auto-derefs from a path-ref into the target
`gtm pool` object, so `.pools[].name` reads the pool's name field
without an explicit second lookup.

---

## GTM + LTM together with `$ltm` / `$gtm`

LTM and GTM are separate ownership domains. The DSL addresses them by
name: pass each config with `--name <var>=<path>` and reference it as
`$<var>.<path>`. Each file must also appear as a positional argument
so the engine has the source text.

> **Caveat.** Without `--merge`, the query is evaluated once per
> positional input, so output appears twice when both files are
> positional. Pipe through `sort -u` or use `--merge` when output
> shape allows.

## LG1. Side-by-side directory of LTM and GTM names

```
$ f5 query --name ltm=ltm.conf --name gtm=gtm.conf --raw \
    '$gtm.gtm.wideip[].name' gtm.conf
```

```
www.example.com
api.example.com
```

(Passing only the file you need as positional avoids the per-file
duplication for simple inspection queries.)

## LG2. Full chain — wideip → GTM pool → LTM VS → LTM pool → pool member

The flagship cross-document join.

```
$ f5 query --name ltm=ltm.conf --name gtm=gtm.conf --merge --raw '
    $gtm.gtm.wideip[] as $w
    | $w.pools[] as $gp
    | $gp.members[]
    | last(split(., ":")) as $vspath
    | $ltm.ltm.virtual[]
    | select(."full-path" == $vspath) as $vs
    | $vs.pool.members[]
    | tsv($w.name, $gp.name, $vs.name, $vs.pool, .address, port(.name))
  ' ltm.conf gtm.conf | sort -u
```

```
api.example.com    api_app_pool        api_vs   /Common/api_pool   10.0.2.20    8443
api.example.com    api_app_pool        api_vs   /Common/api_pool   10.0.2.21    8443
www.example.com    example_app_pool    web_vs   /Common/web_pool   10.0.1.10    80
www.example.com    example_app_pool    web_vs   /Common/web_pool   10.0.1.11    80
```

Read top-to-bottom:

1. `$gtm.gtm.wideip[]` streams every wide-IP (named source binding so
   the query is unambiguous).
2. `$w.pools[]` auto-derefs from a wide-IP's pool path-ref to the
   `gtm pool` object — bound to `$gp`.
3. `$gp.members[]` streams pool members (compound strings like
   `/Common/bigip-east:/Common/web_vs`).
4. `last(split(., ":"))` peels off the LTM VS path from the compound.
5. `$ltm.ltm.virtual[] | select(."full-path" == $vspath)` looks the VS
   up by full path in the LTM document — bound to `$vs`.
6. `$vs.pool.members[]` walks VS → pool (auto-deref) → pool members.
7. `tsv(...)` joins every cell across the broadcast stream into one
   row per LTM pool member.

`sort -u` collapses the per-file-iteration duplication noted above.

## LG3. Cross-file rename — `--merge` cascades across documents

When the *rewrite* needs to span both files, use `--merge`:

```
$ f5 query --merge 'rename("/Common/web_vs", "/Common/web_primary_vs")' ltm.conf gtm.conf
```

```
renamed '/Common/web_vs' -> '/Common/web_primary_vs' (1 occurrence(s))
renamed '/Common/web_vs' -> '/Common/web_primary_vs' (4 occurrence(s))
```

```diff
--- ltm.conf
+++ ltm.conf (modified)
@@ -56,7 +56,7 @@
     HTTP::respond 503 content "down for maintenance"
 }
 }
-ltm virtual /Common/web_vs {
+ltm virtual /Common/web_primary_vs {
     destination /Common/10.0.0.10:80
--- gtm.conf
+++ gtm.conf (modified)
@@ -6,7 +6,7 @@
         10.0.0.1 { }
     }
     virtual-servers {
-        /Common/web_vs {
+        /Common/web_primary_vs {
             destination 10.0.0.10:80
         }
@@ -23,17 +23,17 @@
         10.1.0.1 { }
     }
     virtual-servers {
-        /Common/web_vs {
+        /Common/web_primary_vs {
             destination 10.1.0.10:80
         }
@@ -34,10 +34,10 @@
 gtm pool a /Common/example_app_pool {
     members {
-        /Common/bigip-east:/Common/web_vs {
+        /Common/bigip-east:/Common/web_primary_vs {
             order 0
         }
-        /Common/bigip-west:/Common/web_vs {
+        /Common/bigip-west:/Common/web_primary_vs {
             order 1
         }
     }
```

One command, two files updated. Both the LTM `ltm virtual` header and
every GTM `server.virtual-servers` / `pool.members` reference in the
GTM file moves together.

---

# Part 3 — APM + LTM (merged)

APM access-policies live in their own SCF stanzas, but they're
attached to LTM virtuals via the `apm profile access` profile name in
the VS's `profiles { ... }` list. The two documents are tightly
coupled — `--merge` is the natural mode.

## A1. List access policies

```
$ f5 query --raw '.apm["access-policy"][].name' apm.conf
```

```
employee_login
```

The container kind has a hyphen, so subscript form is needed:
`.apm["access-policy"]`.

## A2. Policy items in execution order

```
$ f5 query --raw '.apm["policy-item"][].name' apm.conf
```

```
employee_login_ent
employee_login_logon_page
employee_login_localdb_auth
employee_login_end_allow
employee_login_end_deny
```

## A3. Access-policy summary (start item, default ending)

`start-item` is a path-ref; `.start-item.caption` auto-derefs into the
target `policy-item` and reads its caption field.

```
$ f5 query --raw '.apm["access-policy"][] | tsv(.name, .start-item.caption, .default-ending)' apm.conf
```

```
employee_login    Start   /Common/employee_login_end_deny
```

---

## APM + LTM merged

## LA1. Which LTM virtuals use an APM profile?

```
$ f5 query --merge --raw '.ltm.virtual[] | select(contains(.profiles, "/Common/employee_login_profile")) | .name' ltm.conf apm.conf
```

```
vpn_vs
```

The query reads `.profiles` (a field of an LTM virtual) and matches a
name defined in `apm.conf`. With `--merge` the engine treats both
documents as one namespace so the predicate works without any
explicit `$ltm`/`$apm` plumbing.

## LA2. Rename an APM profile — the LTM virtual follows

```
$ f5 query --merge 'rename("/Common/employee_login_profile", "/Common/sso_v2_profile")' ltm.conf apm.conf
```

```
renamed '/Common/employee_login_profile' -> '/Common/sso_v2_profile' (1 occurrence(s))
renamed '/Common/employee_login_profile' -> '/Common/sso_v2_profile' (1 occurrence(s))
```

```diff
--- ltm.conf
+++ ltm.conf (modified)
@@ -110,7 +110,7 @@
     mask 255.255.255.255
     pool /Common/api_pool
     profiles {
-        /Common/employee_login_profile { }
+        /Common/sso_v2_profile { }
         /Common/http { }
         /Common/tcp { }
     }
--- apm.conf
+++ apm.conf (modified)
@@ -37,6 +37,6 @@
     }
     start-item /Common/employee_login_ent
 }
-apm profile access /Common/employee_login_profile {
+apm profile access /Common/sso_v2_profile {
     access-policy /Common/employee_login
 }
```

The header in `apm.conf` and the VS's `profiles` reference in
`ltm.conf` both move in one operation.

The two stderr `renamed ...` lines are one event per document — the
rewrite touched one occurrence in each file.

## LA3. Rename the access-policy itself

```
$ f5 query --merge 'rename("/Common/employee_login", "/Common/employee_sso_v2")' ltm.conf apm.conf
```

```
renamed '/Common/employee_login' -> '/Common/employee_sso_v2' (2 occurrence(s))
```

```diff
--- apm.conf
+++ apm.conf (modified)
@@ -26,7 +26,7 @@
     caption Deny
     agents { }
 }
-apm policy access-policy /Common/employee_login {
+apm policy access-policy /Common/employee_sso_v2 {
     default-ending /Common/employee_login_end_deny
     items {
         /Common/employee_login_ent { }
@@ -38,5 +38,5 @@
     start-item /Common/employee_login_ent
 }
 apm profile access /Common/employee_login_profile {
-    access-policy /Common/employee_login
+    access-policy /Common/employee_sso_v2
 }
```

The `access-policy` field of the `apm profile access` follows the
header rename automatically. (The profile name itself didn't change, so
the LTM VS doesn't need updating in this case.)

---

# Appendix — flag reference cheat sheet

| Flag | Purpose |
|---|---|
| `--raw`        | Scalars one per line, no quoting (TSV-friendly) |
| `--json`       | JSON array output |
| `--paths-only` | Print only the full-path of each result |
| `--scf`        | Render results as SCF stanzas when possible |
| `--write`      | For mutating queries: print rewritten config (default = diff) |
| `--in-place`   | For mutating queries: overwrite the input file |
| `--strict`     | Fail (exit 2) if a mutating query matched nothing |
| `--format scf` | SCF output (default; preserves layout) |
| `--format tmsh`        | Full re-render as `tmsh modify` commands |
| `--format tmsh-delta`  | Only changed objects, as `tmsh create / modify / delete` |
| `--transaction`        | Wrap tmsh output in `cli transaction ... submit-transaction` |
| `--name N=PATH`        | Bind a positional file to DSL variable `$N` |
| `--merge`              | Treat all loaded configs as one namespace; refs walk across files |
| `--help-dsl`           | Print the full grammar reference |
| `--help-builtins [N]`  | Print the builtin catalogue (or one entry) |
| `--help-examples`      | Print a cookbook of examples |

| Common builtin | What it does |
|---|---|
| `select(expr)`      | Drop the current value unless `expr` is truthy |
| `contains(s, x)`    | Substring / list membership |
| `tsv(a, b, ...)`    | Tab-separated row; broadcasts streams |
| `csv(a, b, ...)`    | RFC 4180 CSV row; broadcasts streams |
| `in_cidr(addr, n)`  | True if `addr` is inside CIDR `n` |
| `ip(net, src)`      | Rebase `src` into `net`, preserving host bits |
| `port(dest)`        | Parse port from a destination / member name |
| `with_port(d, p)`   | Replace the port of `d` |
| `with_partition(p, P)` | Move path `p` into partition `P` |
| `with_name(p, n)`   | Replace the leaf name of path `p` |
| `partition(p)`      | Return the partition of full-path `p` |
| `references_to(p)`  | List paths that reference `p` |
| `referenced_by(o)`  | Same, but takes an object |
| `rename(o, n)`      | Rename one object everywhere (header + refs) |
| `rename_prefix(o, n)` | Rename every object whose path starts with `o` |
| `rename_partition(o, n)` | Move every object in partition `o` into `n` |
| `rename_folder(o, n)` | Move a folder's worth of objects |
