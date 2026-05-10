# explain-pcap test harness

End-to-end test lab for `f5 explain-pcap`.  Builds TLS certs, deploys an
SCF onto a BIG-IP via tmsh, runs `tcpdump -i 0.0:nnnp` so the F5
Ethernet trailer (legacy + DPT formats) is captured, drives a curated
set of curl scenarios with `SSLKEYLOGFILE` set so HTTPS payloads are
decryptable, pulls every pcap back, and finally runs `f5 explain-pcap`
over each pcap to produce a text + JSON narrative of what happened.

## Layout

| File | Purpose |
|------|---------|
| `bigip_test_lab.scf`  | BIG-IP SCF: 6 virtual servers, several iRules, an LTM policy, profiles, pools (incl. one with no reachable members) |
| `gen_certs.sh`        | OpenSSL-driven cert factory: CA + valid / expired / mismatch / self-signed leafs + client cert |
| `test_server.py`      | Multi-port HTTP/HTTPS origin; deployable on a backend or runnable on the same host as `run_tests.py` when the SCF uses `automap` SNAT |
| `scenarios.py`        | Declarative scenarios table (curl args, expected status, expected substrings in the explain-pcap report) |
| `run_tests.py`        | The orchestrator (deploys, captures, drives traffic, fetches pcaps, runs `f5 explain-pcap`) |

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
scripts/explain_pcap_test_harness/gen_certs.sh

# full run: deploy SCF, capture every scenario, run f5 explain-pcap
scripts/explain_pcap_test_harness/run_tests.py \
    --bigip 10.0.0.5 \
    --user admin \
    --ssh-key ~/.ssh/id_rsa \
    --server-here \
    --out runs/$(date +%s)
```

Single scenario:

```bash
scripts/explain_pcap_test_harness/run_tests.py \
    --bigip 10.0.0.5 --user admin --ssh-key ~/.ssh/id_rsa \
    --scenarios https_sni_route_api \
    --out /tmp/just-sni
```

Skip BIG-IP deployment (useful when iterating on `f5 explain-pcap`
itself):

```bash
scripts/explain_pcap_test_harness/run_tests.py ... --skip-deploy
```

Skip running `f5 explain-pcap` (capture only):

```bash
scripts/explain_pcap_test_harness/run_tests.py ... --skip-explain
```

## Output

After a run, `<out>/` contains:

```
<out>/
├── manifest.json                   ← every scenario + its outcome
├── sslkeys.log                     ← NSS keylog file (-> tshark/Wireshark/explain-pcap)
├── http_block_by_host.pcap
├── http_block_by_host.log
├── https_sni_route_api.pcap
├── ...
└── explain/
    ├── http_block_by_host.text     ← `f5 explain-pcap` text report
    ├── http_block_by_host.json     ← same, JSON
    └── ...
```

Feed the JSON into the `explain_pcap` MCP tool or the `/explain-pcap`
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

## Cleaning up

The SCF lives entirely in the `explain_pcap_lab` admin partition, so:

```bash
ssh admin@<bigip> "tmsh delete auth partition explain_pcap_lab"
```

removes everything the harness deployed.  The orchestrator never
modifies `Common`.

## Adding a scenario

1. Edit `bigip_test_lab.scf` and add the new VS / iRule / pool you want.
2. Add a `Scenario(...)` entry to `scenarios.py` describing what curl
   should send and what `f5 explain-pcap` should be able to say about
   the resulting pcap.
3. Update this table.
