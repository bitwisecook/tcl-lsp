# explain-flow test harness — data gathering for the flow explainer

End-to-end test lab for `f5 explain-flow` and (by extension) anyone who
needs to capture *enough information* about a real BIG-IP flow that the
flow explainer (CLI + MCP tool + Claude skill) can narrate what
happened.  Use this harness either as a self-contained reproducer
against a lab BIG-IP, or as a shopping list of the pieces you need to
gather by hand from production:

* the **bigip.conf / SCF** for every partition the flow touches;
* a **pcap** that captures the packets the BIG-IP saw, including the
  F5 Ethernet trailer so per-packet peer-IP and reset-cause info is
  preserved;
* a **TLS keylog file** (`SSLKEYLOGFILE` from curl, openssl, the
  client's browser, or BIG-IP's TLS keylog provider) so HTTPS
  payloads are decryptable;
* enough **server-side context** that the captured response side
  can be matched against what the origin actually did.

The harness automates all four of those.

## Data the explainer needs (and where each piece comes from)

| Data piece | Source | How the harness gathers it |
|------------|--------|----------------------------|
| BIG-IP configuration (VS, profiles, iRules, pools, LTM policies) | `bigip.conf` / SCF | `tmsh save sys config file=…` (full export) or `tmsh load sys config merge file=… verify` of `bigip_test_lab.scf` for the lab |
| Per-packet wire data + F5 trailer | `tcpdump` on the BIG-IP, with `:nnnp` so the host-side trailer (peer IP, reset cause TLVs, optional TLS keylog provider data) is included | `run_tests.py` SSHes in and runs `tcpdump -i 0.0:nnnp -s 0 -w …` per scenario |
| TLS master secrets / pre-master keys | NSS keylog format (`SSLKEYLOGFILE` env var on the client; or `db tmm.tls.keylogger.enabled true` on TMOS 16+) | `run_tests.py` exports `SSLKEYLOGFILE=<out>/sslkeys.log` for every curl invocation that needs decryption |
| Server-side request/response | The origin's access log + a side-by-side pcap on the backend | `test_server.py` logs every request to stdout; deploy on the pool member or run with `--server-here` so the same host does both |
| iRule source | Already inside the SCF | Captured automatically when the SCF is loaded |
| Reset cause text | F5 Ethernet trailer LOW/MED TLVs — only present in `:nnnp` captures from BIG-IP | Same `tcpdump` invocation as above |

If any of those pieces is missing the explainer falls back gracefully
(e.g. a pcap without keylogs still gives 5-tuple + TLS handshake
metadata + reset analysis), but the richer the input, the richer the
narrative.

## What this lab adds on top of a raw capture

This harness is *more* than a capture grabber: it deliberately
exercises specific iRule / TLS / pool / policy code paths so you have
known-good ground truth to compare the explainer's output against.
Useful both as a regression test for `f5 explain-flow` itself, and as
a worked example for users learning how to gather production data.

## Layout

| File | Purpose |
|------|---------|
| `bigip_test_lab.scf`  | BIG-IP SCF: 6 virtual servers, several iRules, an LTM policy, profiles, pools (incl. one with no reachable members) |
| `gen_certs.sh`        | OpenSSL-driven cert factory: CA + valid / expired / mismatch / self-signed leafs + client cert |
| `test_server.py`      | Multi-port HTTP/HTTPS origin; deployable on a backend or runnable on the same host as `run_tests.py` when the SCF uses `automap` SNAT |
| `scenarios.py`        | Declarative scenarios table (curl args, expected status, expected substrings in the explain-flow report) |
| `run_tests.py`        | The orchestrator (deploys, captures, drives traffic, fetches pcaps, runs `f5 explain-flow`) |

## Prerequisites

On the host running `run_tests.py`:

* Python 3.10+
* `openssl`
* `ssh`, `scp`
* `curl`
* `nc` (or `ncat`) — only for the `payload_reset` scenario
* `sshpass` — optional, only needed if you authenticate with `--password`
* `faketime` — optional; if installed, `gen_certs.sh` produces a properly
  back-dated expired cert.  Without it the expired-cert scenario uses a
  cert with `notAfter` 1 day out, so you must rerun the harness after
  the wallclock advances or supply your own backdated PEM.

On the BIG-IP:

* `tmsh` (always present)
* `tcpdump` (preinstalled on TMOS)
* SSH access for the user passed via `--user`

The orchestrator is dependency-free Python (stdlib only) so it runs out
of the box on any distro that ships Python 3.10+.

## Network topology

The SCF defines six virtual servers in `10.255.42.0/24`:

| VIP                | Port  | Scenario hook |
|--------------------|-------|---------------|
| 10.255.42.100      | 80    | `vs_block` — iRule HTTP block + LTM policy URI rewrite |
| 10.255.42.101      | 443   | `vs_sni` — SNI-based pool routing |
| 10.255.42.102      | 443   | `vs_tls_expired` — server cert past notAfter |
| 10.255.42.103      | 443   | `vs_tls_selfsigned` — no chain to lab CA |
| 10.255.42.104      | 80    | `vs_pool_down` — only pool member is unreachable |
| 10.255.42.105      | 7777  | `vs_payload_reset` — TCP `RESETME` payload triggers `TCP::close` |

The pool members live at `10.255.42.10` / `.11` / `.20`.  Either:

* run `test_server.py` on real hosts at those addresses, or
* run `run_tests.py --server-here` so `test_server.py` listens on the
  same host that drives the curl traffic.  Because every VS uses
  `source-address-translation { type automap }`, BIG-IP sources the
  back-side connection from one of its self-IPs and the response
  travels back through BIG-IP to the same client; no extra plumbing
  needed.

## Running

```bash
# generate certs once (idempotent — re-run safely)
scripts/explain_flow_test_harness/gen_certs.sh

# full run: deploy SCF, capture every scenario, run f5 explain-flow
scripts/explain_flow_test_harness/run_tests.py \
    --bigip 10.0.0.5 \
    --user admin \
    --ssh-key ~/.ssh/id_rsa \
    --server-here \
    --out runs/$(date +%s)
```

Single scenario:

```bash
scripts/explain_flow_test_harness/run_tests.py \
    --bigip 10.0.0.5 --user admin --ssh-key ~/.ssh/id_rsa \
    --scenarios https_sni_route_api \
    --out /tmp/just-sni
```

Skip BIG-IP deployment (useful when iterating on `f5 explain-flow`
itself):

```bash
scripts/explain_flow_test_harness/run_tests.py ... --skip-deploy
```

Skip running `f5 explain-flow` (capture only):

```bash
scripts/explain_flow_test_harness/run_tests.py ... --skip-explain
```

## Output

After a run, `<out>/` contains:

```
<out>/
├── manifest.json                   ← every scenario + its outcome
├── sslkeys.log                     ← NSS keylog file (-> tshark/Wireshark/explain-flow)
├── http_block_by_host.pcap
├── http_block_by_host.log
├── https_sni_route_api.pcap
├── ...
└── explain/
    ├── http_block_by_host.text     ← `f5 explain-flow` text report
    ├── http_block_by_host.json     ← same, JSON
    └── ...
```

Feed the JSON into the `explain_flow` MCP tool or the `/explain-flow`
Claude skill for an LLM-readable narrative; the text reports are
operator-facing.

## Scenarios

| Name                       | What it captures |
|----------------------------|------------------|
| `http_block_by_host`       | iRule `HTTP::respond 403` triggered by `[HTTP::host] equals "blocked.lab.test"`. The HUD annotation tells the LLM exactly which line of the iRule fired and what the captured Host was. |
| `http_block_admin_path`    | Same iRule, different branch (`HTTP::path starts_with "/admin"`). |
| `http_policy_rewrite`      | LTM policy rewrites `/api/v1/health` → `/api/v2/health`; back-side flow shows the rewritten URI. |
| `https_sni_default`        | HTTPS with SNI=`app.lab.test`; iRule does NOT route, traffic stays on default pool. |
| `https_sni_route_api`      | HTTPS with SNI=`api.lab.test`; iRule fires `pool /…/lab_pool_api` → back-side flow lands on a different pool member. |
| `tls_expired_cert`         | Curl rejects expired cert; pcap shows handshake failure + BIG-IP RST + F5 reset cause. |
| `tls_selfsigned_strict`    | Curl rejects self-signed cert (default verify); captured as bad-cert alert. |
| `tls_selfsigned_relaxed`   | Same VS but `curl -k` so handshake completes; HTTPS payload decryptable via SSLKEYLOGFILE. |
| `pool_down`                | Pool with one unreachable member; BIG-IP RSTs with cause text. |
| `payload_reset`            | Plain TCP; iRule resets the connection on `RESETME` byte pattern. |

## Gathering data from production (without this lab)

If you're trying to explain a flow on a real production BIG-IP rather
than reproduce one in the lab, you don't need `run_tests.py` — you
need the same *artefacts* it collects.  Run these by hand in roughly
this order:

### 1. Pull the relevant BIG-IP config

You only need the partitions the flow actually traverses.  The
explainer accepts multiple `.conf` files via the CLI's positional
`paths` argument, so partial dumps are fine.

```bash
# full SCF export (recommended — includes every referenced object)
ssh admin@<bigip> "tmsh save sys config file=/var/tmp/prod.scf no-passphrase"
scp admin@<bigip>:/var/tmp/prod.scf ./prod.scf

# or, if the box won't let you save full SCF, just the LTM partition
ssh admin@<bigip> "tmsh list ltm one-line | tee /var/tmp/prod-ltm.conf"
scp admin@<bigip>:/var/tmp/prod-ltm.conf ./prod-ltm.conf
```

### 2. Capture with the F5 Ethernet trailer

This is the critical step for the `f5 explain-flow` extras (peer IP
pairing, reset cause TLVs, decoded TMM annotations).  The
`:nnnp` suffix is what makes BIG-IP write the trailer into each
packet:

```bash
# on the BIG-IP, capture both directions of the flow you care about
ssh admin@<bigip> "tcpdump -i 0.0:nnnp -s 0 -w /var/tmp/flow.pcap \
    'host <client-ip> and host <vs-ip>'"
# ...reproduce the issue...
# Ctrl-C the tcpdump
scp admin@<bigip>:/var/tmp/flow.pcap ./flow.pcap
```

`-i 0.0` captures across all VLANs; replace with `-i internal` /
`-i external` / `-i <vlan>` if you know which side is interesting.
Use `-C 100 -W 5` for a rolling 5x100MB ring if the issue is
intermittent.

### 3. Get a TLS keylog (optional but high-value)

Three ways to obtain one, in order of preference:

1. **From the client.**  If the client is curl, openssl, Firefox, or
   Chrome, set `SSLKEYLOGFILE=/path/to/keys.log` in its environment
   *before* the request — every TLS session it negotiates appends
   PMS/TLS-1.3 keys to that file.  No BIG-IP-side change required.
2. **From BIG-IP** (TMOS 16+).  Enable the F5 TLS keylogger so the
   master secrets ride along inside the F5 Ethernet trailer (DPT
   provider 4 — already understood by `dialects.f5.bigip.f5_trailer`):
   ```
   tmsh modify sys db tmm.tls.keylogger.enabled value true
   ```
   The keylog material then gets parsed straight out of the pcap; no
   external `--keylog` file is needed.
3. **From the backend** (when the BIG-IP is acting as the TLS client
   to a re-encrypted pool member).  Same `SSLKEYLOGFILE` env var on
   whatever client library the backend service uses.

### 4. (Optional) Backend-side log + pcap

Useful when the question is "did the pool member ever see the
request?".  `test_server.py` logs every request and is small enough
to drop on any Linux box; or just run `tcpdump` on the member and
correlate timestamps with the BIG-IP capture.

### 5. Feed everything to `f5 explain-flow`

```bash
f5 explain-flow \
    --tshark \
    --keylog ./keys.log \
    --simulate \
    ./flow.pcap ./prod.scf
```

* `--tshark` enriches the report with full HTTP/TLS decoding
  (requires `tshark` on PATH).
* `--keylog` decrypts HTTPS so HTTP::host / HTTP::method / response
  codes are populated for TLS-wrapped sessions.
* `--simulate` actually executes the matched VS's iRules under the
  C-tcl orchestrator with the captured state, returning the real
  pool/respond decisions.

JSON output (`--json`) is what the `explain_flow` MCP tool / Claude
skill consume.

## Cleaning up

The SCF lives entirely in the `explain_flow_lab` admin partition, so:

```bash
ssh admin@<bigip> "tmsh delete auth partition explain_flow_lab"
```

removes everything the harness deployed.  The orchestrator never
modifies `Common`.

## Adding a scenario

1. Edit `bigip_test_lab.scf` and add the new VS / iRule / pool you want.
2. Add a `Scenario(...)` entry to `scenarios.py` describing what curl
   should send and what `f5 explain-flow` should be able to say about
   the resulting pcap.
3. Update this table.
