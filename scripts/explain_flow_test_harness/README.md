# explain-flow test lab — data gathering for the flow explainer

A lab BIG-IP configuration and TLS cert factory for `f5 explain-flow`, plus
the recipe for gathering the same artefacts from production. Whichever route
you take, the explainer wants four things:

* the **bigip.conf / SCF** for every partition the flow touches;
* a **pcap** the BIG-IP itself captured, with the F5 Ethernet trailer so
  per-packet peer IP and reset cause survive;
* a **TLS keylog file** (`SSLKEYLOGFILE` from curl, openssl, or the browser;
  or BIG-IP's own keylog provider) so HTTPS payloads decrypt;
* enough **server-side context** to match the captured response against what
  the origin actually did.

Any piece may be missing — a pcap without keylogs still yields 5-tuple, TLS
handshake metadata, and reset analysis — but the richer the input, the richer
the narrative.

## Layout

| File | Purpose |
|------|---------|
| `bigip_test_lab.scf` | BIG-IP SCF: 6 virtual servers, 4 iRules, an LTM policy, profiles, and three pools (one with no reachable member), all in the `explain_flow_lab` partition |
| `gen_certs.sh` | OpenSSL cert factory: CA plus valid / expired / mismatch / self-signed leafs and a client cert, EC P-256, into `./certs/` |

Deploying the SCF and driving traffic through it is manual — there is no
orchestrator. Load the SCF, generate the certs, run curl, capture, and feed
the result to `f5 explain-flow` as in *Gathering the artefacts* below.

## Network topology

The SCF defines six virtual servers in `10.255.42.0/24`:

| VIP | Port | What it exercises |
|--------------------|-------|---------------|
| 10.255.42.100 | 80 | `vs_block` — iRule `HTTP::respond 403` plus an LTM policy URI rewrite |
| 10.255.42.101 | 443 | `vs_sni` — SNI-based pool routing |
| 10.255.42.102 | 443 | `vs_tls_expired` — server cert past `notAfter` |
| 10.255.42.103 | 443 | `vs_tls_selfsigned` — no chain to the lab CA |
| 10.255.42.104 | 80 | `vs_pool_down` — the only pool member is unreachable |
| 10.255.42.105 | 7777 | `vs_payload_reset` — a `RESETME` payload triggers `TCP::close` |

Pool members live at `10.255.42.10:8080`, `.11:8080`, `.20:8443`, and the
deliberately dead `.99:8080`. Every VS uses
`source-address-translation { type automap }`, so BIG-IP sources the back-side
connection from a self-IP and one host can serve both roles.

## Certificates

```bash
scripts/explain_flow_test_harness/gen_certs.sh [output_dir]   # idempotent
```

Without `faketime` installed the "expired" leaf is dated one day out rather
than properly back-dated, so re-run it once the wallclock has advanced or
supply your own back-dated PEM.

## Gathering the artefacts

The same four steps whether the flow is in the lab or in production.

### 1. Pull the BIG-IP config

Only the partitions the flow traverses are needed; the explainer takes
several `.conf` files as positional arguments, so a partial dump is fine.

```bash
ssh admin@<bigip> "tmsh save sys config file=/var/tmp/prod.scf no-passphrase"
scp admin@<bigip>:/var/tmp/prod.scf ./prod.scf

# or, when a full SCF save is not permitted:
ssh admin@<bigip> "tmsh list ltm one-line | tee /var/tmp/prod-ltm.conf"
scp admin@<bigip>:/var/tmp/prod-ltm.conf ./prod-ltm.conf
```

For the lab, load the SCF instead:

```bash
tmsh load sys config merge file=/var/tmp/bigip_test_lab.scf verify
```

### 2. Capture with the F5 Ethernet trailer

The `:nnnp` suffix is what makes BIG-IP write the trailer into each packet —
without it there is no peer-IP pairing, no reset-cause TLV, no decoded TMM
annotation.

```bash
ssh admin@<bigip> "tcpdump -i 0.0:nnnp -s 0 -w /var/tmp/flow.pcap \
    'host <client-ip> and host <vs-ip>'"
# ...reproduce the issue, then Ctrl-C...
scp admin@<bigip>:/var/tmp/flow.pcap ./flow.pcap
```

`-i 0.0` captures across all VLANs; name a VLAN instead when you know which
side matters. `-C 100 -W 5` gives a rolling 5×100 MB ring for an intermittent
issue.

### 3. Get a TLS keylog (optional, high value)

1. **From the client** — set `SSLKEYLOGFILE=/path/to/keys.log` before the
   request (curl, openssl, Firefox, Chrome). No BIG-IP change needed.
2. **From BIG-IP** (TMOS 16+) — `tmsh modify sys db
   tmm.tls.keylogger.enabled value true` rides the master secrets inside the
   F5 trailer (DPT provider 4), parsed straight out of the pcap with no
   external keylog file.
3. **From the backend** — the same env var on the pool member's client
   library, when BIG-IP re-encrypts to the origin.

### 4. Run the explainer

```bash
f5 explain-flow --tshark --keylog ./keys.log --simulate ./flow.pcap ./prod.scf
```

* `--tshark` adds full HTTP/TLS decoding (needs `tshark` on PATH).
* `--keylog` decrypts HTTPS so `HTTP::host` / method / status populate.
* `--simulate` runs the matched VS's iRules under the C-tcl orchestrator with
  the captured state, returning the real pool / respond decisions.

`--json` is the shape the `explain_flow` MCP tool and the `explain-flow`
Claude skill consume; the default text report is operator-facing.

## Cleaning up

Everything the lab deploys lives in one partition:

```bash
ssh admin@<bigip> "tmsh delete auth partition explain_flow_lab"
```
