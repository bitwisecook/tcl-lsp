# KCS: feature — `f5` CLI overview

> **Audience:** User
> **Type:** Functionality

## Summary

`f5` is a stdlib-only CLI for working with BIG-IP configurations.  It
ships ~20 verbs covering the operator workflow: pull a config from a
device, analyse and lint it, transform it (rename, redact, split,
emit `tmsh`, convert to AS3), share it safely (redaction is
reversible via a sidecar map file), and push edits back.

## Applies to

`f5` CLI (zipapp + `python -m explorer.f5_cli`)

## Question

I have a BIG-IP — what can I do with `f5`?

## How to use

The CLI groups verbs by lifecycle phase.

### 1.  Acquire

| Verb | Purpose |
| --- | --- |
| `f5 fetch` | Pull SCF/UCS from a live device via iControl REST or SSH. |
| `f5 extract` (alias `ucs2scf`) | Unpack a local UCS file to SCF text. |

```sh
# REST (default), credentials via env / XDG / prompt
F5_HOST=bigip01 F5_USER=admin f5 fetch --transport rest

# SSH with explicit creds
f5 fetch --transport ssh --host bigip01 --user admin

# Convert a UCS that was scp'd off a device
f5 extract device.ucs > device.scf
```

`fetch` defaults to caching under `$XDG_CACHE_HOME/f5/<host>/<UTC-timestamp>/`
with a `latest` symlink alongside.  `--output -` streams the SCF to
stdout.  `--format ucs|both` keeps the UCS bytes too.

### 2.  Analyse

| Verb | Purpose |
| --- | --- |
| `f5 stats` (alias `summary`) | Counts per object kind, partition breakdown, top-references, orphan count. |
| `f5 graph` (alias `deps`) | Emit the reference graph as DOT / JSON / Mermaid (with `--seed PATH` for subgraphs). |
| `f5 explain {virtual\|pool\|auto} <name>` | Resolve the profile chain, iRule chain, persistence, SNAT, and pool members for one object. |
| `f5 diff old.scf new.scf` | Object-aware diff (ignores property ordering and iRule whitespace). |
| `f5 grep` | Find every object related to a name, regex, or CIDR. |
| `f5 query` (alias `q`) | jq-flavoured DSL for filtering and projecting object properties; see [`kcs-feature-bigip-query.md`](kcs-feature-bigip-query.md). |
| `f5 cleanup` | Generate `tmsh delete` commands for objects no virtual references. |
| `f5 validate` (alias `lint`) | Best-practice / structural checks (orphan monitors, empty pools, deprecated iRule commands, unknown events, …). |

```sh
f5 stats bigip.conf
f5 explain virtual /Common/vs_app bigip.conf
f5 diff before.conf after.conf --json
f5 graph bigip.conf --seed /Common/vs_app --format mermaid
f5 validate bigip.conf --format sarif > findings.sarif
```

`f5 validate` exits 0 on no findings or info-only, 1 on warning, 2 on
error.  Multi-file inputs are merged before rules run, so cross-file
references don't trigger false-positive orphan findings.

### 3.  Transform

| Verb | Purpose |
| --- | --- |
| `f5 rename` (alias `mv`) | Rename a full-path and update every reference (dry-run by default). |
| `f5 query` (alias `q`) | DSL-driven property edits and identity renames; readdressing, bulk field rewrites, and iRule reference edits all land here.  See [`kcs-feature-bigip-query.md`](kcs-feature-bigip-query.md). |
| `f5 redact` (alias `sanitize`) | Strip secrets and remap public IPs into a configurable CIDR pool. |
| `f5 unredact` (alias `unmap`) | Reverse a `redact` using its sidecar map file. |
| `f5 pcap-remap` (alias `pcapmap`) | Apply a redaction map to a PCAP capture. |
| `f5 split` | Write one `.conf` per partition under a directory (suitable for git). |
| `f5 merge` | Concatenate per-partition `.conf`s back into one SCF. |
| `f5 convert` | UCS↔SCF and SCF→AS3 declaration. |
| `f5 tmsh` (alias `scf2tmsh`) | Emit `tmsh create` / `--modify` commands in dependency order. |

```sh
f5 rename /Common/old_pool /Common/new_pool bigip.conf --in-place

# Redact + reversible map
f5 redact bigip.conf -o bigip.redacted --map-file shared.redact.toml
# … later, recover an IP from a support reply
echo "saw 10.0.0.42 fail" | f5 unredact shared.redact.toml -

# Apply the same map to a packet capture
f5 pcap-remap shared.redact.toml capture.pcap capture.redacted.pcap

# Recreate a config on a fresh device
f5 tmsh bigip.conf > recreate.tmsh
```

#### Redaction model

- **CIDR-preserving**: every public IPv4 literal joins its enclosing
  `/24` (configurable via `--source-cidr`); each unique source CIDR
  gets a same-prefix-length target CIDR out of `--target-cidr` (default
  RFC1918 + `fd00::/8`).  Two real IPs sharing a `/24` always land in
  the same redacted `/24`.
- **Reversible**: a sidecar TOML map (`<output>.redact.toml` by
  default) records every assignment.  `f5 unredact` walks it in reverse
  over any text — config, support email, log line.
- **Stable across runs**: passing the same `--map-file` to a later
  `f5 redact` reuses every prior assignment, so an IP a customer saw
  in week-1 keeps mapping to the same redacted address in week-2.
- **--shuffle**: per-CIDR Fisher-Yates permutation of host bits hides
  patterns; shuffle keys are recorded in the map so reverse direction
  is exact.

`f5 pcap-remap` applies the same map to a libpcap file: rewrites IPv4/
IPv6 src/dst, recomputes IP header + TCP/UDP/ICMP/ICMPv6 checksums,
and **parses** the F5 Ethernet trailer (everything past
`IP total_length` — what `tcpdump -i 0.0:nnnp` adds), rewriting peer-IP
fields at schema-known offsets.  Both legacy (TMOS 9.4–13.x) and DPT
(TMOS 14+) trailer formats are handled; the schema is ported from
Wireshark's `packet-f5ethtrailer.c`.  When a TLV's `(type, version)`
pair has no registered schema, behaviour is controlled by
`--on-unknown`: `error` (default) refuses to write the output, `preserve`
leaves the TLV unchanged, `sweep` falls back to byte-replacement of
known IPs within just that TLV's data section.  Operators can extend
the schema for fleet-specific layouts via `--schema OVERLAY.toml`
(repeatable); `--list-schemas` prints the active registry.  L4 payload
bytes are *not* touched.

### 4.  Round-trip

| Verb | Purpose |
| --- | --- |
| `f5 pull` | GET one object from a device and emit its SCF stanza (or `--json` for raw iControl payload). |
| `f5 push` | PUT (replace) or `--create` POST one object via iControl REST. |
| `f5 irule extract` | Write each rule body to its own `.tcl` file for editing. |
| `f5 irule trace EVENT` | Static event-flow trace from a starting event. |

```sh
f5 pull pool /Common/p1 --host bigip01 --user admin > p1.scf
f5 pull pool /Common/p1 --host bigip01 --user admin --json > p1.json
# … edit p1.json …
f5 push pool p1.json --host bigip01 --user admin
```

### Credentials

Resolution order (highest priority first) for every verb that talks
to a device:

1. CLI flag (`--host`, `--user`, `--password`, `--port`, `--ssh-port`)
2. Environment (`F5_HOST`, `F5_USER`, `F5_PASSWORD`, `F5_PORT`, `F5_SSH_PORT`)
3. XDG hosts file: `$XDG_CONFIG_HOME/f5/hosts.toml` (default
   `~/.config/f5/hosts.toml`).  Use a host alias on the CLI to look up
   that entry:

   ```toml
   [hosts.lab]
   host = "10.0.0.5"
   user = "labadmin"
   password = "labpw"
   port = 8443
   ```

   `f5 fetch --host lab` resolves to the alias.
4. Interactive prompt (`getpass` for passwords); pass `--no-prompt` to
   make missing creds fail instead of prompt.

### Shell completion

`f5 completion bash|fish|zsh` emits a ready-to-install completion
script.  See [`kcs-feature-bigip-cleanup.md`](kcs-feature-bigip-cleanup.md)
for installation paths.
