# KCS: feature — BIG-IP Related-Object Grep

> **Audience:** User
> **Type:** Functionality

## Summary

`f5` CLI verb that finds every BIG-IP object related to a given object name, regex, or CIDR — by walking the same forward-and-reverse reference graph the cleanup analysis uses.  CIDR mode also scans IP literals buried inside iRule script bodies.

## Applies to

tcl-lsp CLI

## Question

How do I find every BIG-IP object that is related to a given object — every pool that uses a node, every virtual that depends on a pool, every iRule that references a data-group?

## How to use

`grep` parses one or more `bigip.conf` / SCF files and walks the BIG-IP object reference graph from every object whose full path matches the pattern.  The reference graph is the same one [`f5 cleanup`](kcs-feature-bigip-cleanup.md) walks: configuration-property references plus iRule body references (`pool`, `persist`, `class match ... <data-group>`, `snatpool`, `virtual`, `node`).

By default the walk runs both directions — *forward* (what the match depends on) and *reverse* (what depends on it) — so one command shows the full neighbourhood.

```
f5 grep /Common/web_pool bigip.conf
f5 grep --direction reverse /Common/web1 bigip.conf
f5 grep --regex '^/Common/(web|api)_pool$' bigip.conf
f5 grep --json --max-depth 2 web_pool bigip.conf
f5 grep --cidr 10.0.0.0/8 bigip.conf
f5 grep --cidr '10.0.0.0/8, 192.168.0.0/16' bigip.conf
```

`related` is an alias for the same verb.

## Options

- `-e, --regex` — treat PATTERN as a Rust `regex`-crate regular expression, which has no look-around or backreferences (default: substring match against the object's full path).  Mutually exclusive with `--cidr`.
- `-c, --cidr` — treat PATTERN as one or more whitespace- or comma-separated IPv4/IPv6 addresses or CIDR ranges.  An object qualifies when any IP literal or CIDR mentioned in its full path, header, or body — including iRule script bodies — overlaps any requested network.  Mutually exclusive with `--regex`.
- `-r, --recurse` / `--no-recurse` — walk the related-object graph from each match (the default), or return only the objects that directly match PATTERN.  Applies to every match mode.  With `--no-recurse`, `--direction` and `--max-depth` are ignored and the report's `related` list is empty.
- `--direction {forward,reverse,both}` — which edges to walk from each match.  `forward` follows outgoing references, `reverse` incoming ones, `both` (default) walks both.
- `--max-depth N` — stop after N hops from each match.  Default: unlimited.
- `--max-nodes N` — cap the result at N objects (default: 1000) to keep the output tractable on very large configurations.
- `--full` — print each object's full body, not just its header / path.
- `--json` — emit the report as JSON instead of the text report.
- `--format tmsh` — render each matched object as a one-line `tmsh create` command.
- `-o FILE` — write the report to `FILE` instead of stdout.

The exit code is `0` when at least one object matched the pattern and `1` when none did, mirroring the standard `grep` convention.

## Example

### Input

```
ltm node /Common/n1 { address 10.0.0.1 }
ltm monitor http /Common/m1 { defaults-from /Common/http }
ltm pool /Common/web_pool {
    members { /Common/n1:80 { address 10.0.0.1 } }
    monitor /Common/m1
}
ltm virtual /Common/vs {
    destination /Common/10.0.0.10:80
    pool /Common/web_pool
}
```

### Output (`f5 grep /Common/web_pool bigip.conf`)

```
# tcl-lsp BIG-IP grep
# Pattern: /Common/web_pool
# Direction: both
# Sources: file:///bigip.conf
# Searches: 1 matched object(s)
# Related: 3 object(s)
#   ltm_monitor_http: 1
#   ltm_node: 1
#   ltm_pool: 1
#   ltm_virtual: 1

# Searches (matched by pattern):
* [ltm_pool] /Common/web_pool  (depth 0)

# Related objects (reachable through reference edges):
  [ltm_monitor_http] /Common/m1  (depth 1)
  [ltm_node] /Common/n1  (depth 1)
  [ltm_virtual] /Common/vs  (depth 1)
```

A matched line is prefixed with `*`; related lines start with two spaces.  The `(depth N)` annotation is the distance from the nearest match.

### CIDR mode

Use `--cidr` to ask "which BIG-IP objects touch a given network?".  PATTERN is parsed as one or more IPv4/IPv6 addresses or CIDR ranges, and every object's full path, header, and body is scanned for overlapping IP/CIDR tokens.  Bare host addresses are treated as `/32` (IPv4) or `/128` (IPv6), so `--cidr 10.0.0.5` also matches an object that contains the containing network `10.0.0.0/24`, and vice versa.

```
ltm rule /Common/r_block {
when HTTP_REQUEST {
    if { [matchclass [IP::client_addr] equals "10.10.0.0/16"] } { reject }
    if { [IP::addr [IP::client_addr] equals "10.0.0.5"] } { reject }
}
}
```

`f5 grep --cidr 10.0.0.0/8 bigip.conf` matches `/Common/r_block` because both the literal `10.0.0.5` and the CIDR `10.10.0.0/16` in the iRule body fall inside `10.0.0.0/8`.  This is the only way to surface IP references buried inside Tcl logic — the substring and regex modes only match against an object's full path.

## Out of scope

- The grep verb does not modify the configuration — it only reports.
- It uses *substring* matching by default; pass `--regex` for a regular expression or `--cidr` for IP/CIDR matching.  There is no glob / shell-style matching.
- `--cidr` skips any candidate token that does not parse as an address or network — the token regexes are deliberately permissive and the address parser is the source of truth.
- The reference graph is the same one [`f5 cleanup`](kcs-feature-bigip-cleanup.md) walks; objects unreachable through that graph are not surfaced even if they share a name pattern.

## Related

- [BIG-IP Config Cleanup](kcs-feature-bigip-cleanup.md) — uses the same reference graph to flag unreachable objects.
- [iRule Extraction](kcs-feature-irule-extraction.md) — pulls iRule bodies out of a `bigip.conf` for editing.
