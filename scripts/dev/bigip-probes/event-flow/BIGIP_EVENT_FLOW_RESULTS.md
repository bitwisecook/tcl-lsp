# BIG-IP iRule event-flow results

> **Appliance under test: BIG-IP 21.1.0.1, build 0.0.26, Point Release 1, dated 14 July 2026. FIPS Module: Cryptographic Module for BIG-IP.**

These are measured TMM results. Literal rule acceptance, virtual-server
attachment, event reachability, event order, and client/backend outcomes are
reported separately. An accepted event name is not described as reachable
unless it appeared in the continuous TMM log.

## Appliance, topology, and coverage

| Item | Measured value |
| --- | --- |
| Platform | BIG-IP Virtual Edition, platform Z100, standalone ACTIVE device |
| Provisioning | LTM nominal; APM, ASM, AFM, GTM and the other optional modules unprovisioned |
| TMM process | PID 11604, one process using four CPUs |
| Actual TMM roster | `0:0`, `0:1`, `0:2`, `0:3` |
| Roster proof | `RULE_INIT` logged once on all four units for each trace rule. The 64-flow TCP baseline produced exactly 16 `CLIENT_ACCEPTED` events on each unit. |
| Client and backend | `dev.bragi0.com`, `192.168.9.80` |
| VIP address | `192.168.9.24`, with per-protocol ports 18500-18515 |
| Data-plane prerequisite | The post-relicence configuration lacked the prior `/Common/self_1nic`; it was restored as `192.168.9.24/24` on `/Common/internal`, local-only traffic group. Before that restoration the first TCP request reached TMM and ended in `LB_FAILED`; afterwards every intended backend was reachable. |
| Return path | Every test virtual used SNAT automap. All 88 recorded backend TCP/HTTP/UDP/DNS connections and datagrams with a peer saw `192.168.9.24`, never the client address. |

The complete version output is [sys-version.txt](evidence/evtflow_20261006a/appliance/sys-version.txt), and the complete roster is [sys-tmm-info.txt](evidence/evtflow_20261006a/appliance/sys-tmm-info.txt). The raw continuous log is [ltm-continuous.log](evidence/evtflow_20261006a/appliance/ltm-continuous.log).

```text
Sys::Version
Main Package
  Product      BIG-IP
  Version      21.1.0.1
  Build        0.0.26
  Edition      Point Release 1
  Date         Tue Jul 14 05:03:24 PDT 2026
  FIPS Module  Cryptographic Module for BIG-IP
```

## Source and byte identity

The tested base was `https-origin/rust` commit
`6d214350b9f899a1399fa841be7f6fc5edbf225a`. Generated files were ASCII with
LF line endings and were checked with `sha256sum -c` on the appliance before
load. A recursive `scp` returned success while omitting files, so no transfer
was trusted until the complete manifest passed on the destination. The final
single-file evidence archives were independently hashed after excluding the
licence record and disposable TLS private key.

| Source/artifact | SHA-256 |
| --- | --- |
| `event-candidates.txt` | `59274e8deab11ba9be2864b1612e31995c4983e01988e564935478ab59f7ea03` |
| `generate-event-fixtures.py` (committed) | `b3585ae23dd57b535b9a156ed7eaa14ba3f83932a0ac0ea28463df4af6a83752` |
| `generate-event-fixtures.py` (executed, before source-only formatting) | `5a9656a9f8e13b8f343c1bf9644c1ebcb6ca6b5327ca35c2b7985eb478e5c5f1` |
| `event-flow-lab.py` (committed) | `7306df9d12e039028c233f898a72a0a8acb33a5018a35532f72461a01965c62b` |
| `event-flow-lab.py` (executed, before source-only formatting) | `8f189ad83a8e2475ed6628bd81a4a0b6b72a6d32033ab9c14a72c019dae8d55c` |
| `run-event-loader.sh` | `8d26145b886991a14be267957f45722e3c30af06a90787ac9e5ab59b8b8a24c5` |
| Final generated manifest | `91b67c031956c11cf224784ea12a3a0aab9b9c5cda4283179f9400dbd1601fd2` |
| Appliance evidence archive | `a8b4cd3c13286ceb7bd525480e75457a24134dccc7438e17d58df76b916bbc12` |
| Client/backend evidence archive | `d77ef3f671f655389de67955c9a47a5e8cf0430175ff405af02f000f566ae785` |

The appliance-side checks are [fixture-sha256-check.txt](evidence/evtflow_20261006a/appliance/fixture-sha256-check.txt) and [fixture-EFB-sha256-check.txt](evidence/evtflow_20261006a/appliance/fixture-EFB-sha256-check.txt). Exact generated fixtures and manifests are retained under [appliance/fixtures](evidence/evtflow_20261006a/appliance/fixtures/) and [appliance/fixtures-EFB](evidence/evtflow_20261006a/appliance/fixtures-EFB/).
The repository formatter changed only Python source layout after execution. The
committed generator reproduces the final manifest hash above; loaded payload
identity is established by the retained appliance-side manifests and checks.

## Literal event-name inventory

Each of the 176 candidate names was the literal event in an otherwise
independent rule. There were 163 accepts, 13 rejects, and no warnings. The
accepted count establishes only compiler recognition.

The following names were rejected with `unknown event`:

```text
HTTP_CLASS_FAILED
HTTP_CLASS_SELECTED
IP_GTM
SIP_REQUEST_DONE
SIP_RESPONSE_DONE
TCP_GTM
UDP_GTM
XML_BEGIN_DOCUMENT
XML_BEGIN_ELEMENT
XML_CDATA
XML_END_DOCUMENT
XML_END_ELEMENT
XML_EVENT
```

This is a counterexample to treating the current 176-name registry list as the
event set for this build. In particular, `SIP_REQUEST` and
`SIP_REQUEST_SEND` loaded and ran, while the two `*_DONE` names did not compile.
The authoritative table and every raw loader diagnostic are
[event-load-results.tsv](evidence/evtflow_20261006a/appliance/event-load-results.tsv)
and [event-load-details](evidence/evtflow_20261006a/appliance/event-load-details/).

`RULE_INIT` ran on all four actual units. That observation is direct; it is not
inferred from CMP configuration or request volume.

## Reached event matrix

The following event names were reached by valid traffic. Counts are retained in
the raw log; the order rows below describe the correlated transactions.

| Flow or condition | Measured event sequence and outcome |
| --- | --- |
| Standard TCP echo | `CLIENT_ACCEPTED -> CLIENT_DATA -> LB_SELECTED -> SA_PICKED -> SERVER_CONNECTED -> SERVER_DATA -> SERVER_CLOSED -> CLIENT_CLOSED`; 64/64 correlated responses, 16 flows on each TMM |
| HTTP/1.1 GET | `CLIENT_ACCEPTED -> HTTP_REQUEST -> LB_SELECTED -> SA_PICKED -> SERVER_CONNECTED -> HTTP_REQUEST_SEND -> HTTP_REQUEST_RELEASE -> HTTP_RESPONSE -> HTTP_RESPONSE_RELEASE`, then close events |
| HTTP/1.1 POST with 12-byte body | `HTTP_REQUEST_DATA` followed `HTTP_REQUEST`; release preceded load balancing |
| Collected HTTP response | `HTTP_RESPONSE_DATA` followed `HTTP_RESPONSE`; `HTTP_RESPONSE_RELEASE` followed payload release |
| Origin 404 and 401 | Normal HTTP sequence; `HTTP_RESPONSE` exposed statuses 404 and 401. Neither response caused an F5 `AUTH_*` event. |
| Client TLS plus HTTP/1.1 | `CLIENT_ACCEPTED -> CLIENTSSL_CLIENTHELLO -> CLIENTSSL_HANDSHAKE -> HTTP_REQUEST`; TLS 1.3 with `TLS13-AES128-GCM-SHA256`. `CLIENTSSL_SERVERHELLO_SEND` did not fire. |
| HTTP/2 | ALPN selected `h2`; client request and response were HTTP/2, backend request was HTTP/1.1. The ordinary `HTTP_REQUEST`/`HTTP_RESPONSE` events fired with `HTTP::version == 2`. No separate HTTP/2 event name was observed. |
| Server-side TLS | After `SERVER_CONNECTED`: `SERVERSSL_CLIENTHELLO_SEND -> SERVERSSL_SERVERHELLO -> SERVERSSL_SERVERCERT -> SERVERSSL_HANDSHAKE -> HTTP_REQUEST_SEND`; TLS 1.3 with `TLS13-AES256-GCM-SHA384` |
| UDP echo | `CLIENT_ACCEPTED -> LB_SELECTED -> CLIENT_DATA -> SERVER_CONNECTED -> SERVER_DATA`; correlated 11-byte request and 20-byte response |
| DNS over UDP | `CLIENT_ACCEPTED -> CLIENT_DATA -> DNS_REQUEST -> LB_SELECTED -> SERVER_CONNECTED -> SERVER_DATA -> DNS_RESPONSE`; ID 59137, A question, one A answer |
| Cache miss | `HTTP_REQUEST -> CACHE_REQUEST ->` backend events `-> HTTP_RESPONSE -> CACHE_UPDATE -> HTTP_RESPONSE_RELEASE` |
| Cache hit on another TMM | `HTTP_REQUEST -> CACHE_REQUEST -> CACHE_RESPONSE -> HTTP_RESPONSE_RELEASE`; no load-balancing, server, backend, or ordinary `HTTP_RESPONSE` event |
| FastL4 | `CLIENT_ACCEPTED -> LB_SELECTED -> SERVER_CONNECTED -> SERVER_CLOSED -> CLIENT_CLOSED`; no `CLIENT_DATA`, `SERVER_DATA`, or `SA_PICKED` in the valid echo transaction |
| MPTCP-profile VIP, ordinary TCP | Same standard TCP event sequence |
| MPTCP-profile VIP, native MPTCP socket | Successful echo; Linux reported `tcp-ulp-mptcp flags:mc` and an MPTCP control connection with one subflow, but also `fallback`. This is not evidence of a successfully negotiated multipath data path. |
| SIP/UDP incomplete request | Only UDP `CLIENT_ACCEPTED` and `CLIENT_DATA`; no SIP event or backend forwarding |
| SIP/UDP complete OPTIONS | `SIP_REQUEST -> LB_SELECTED -> SERVER_CONNECTED -> SIP_REQUEST_SEND -> SERVER_DATA -> SIP_RESPONSE -> SIP_RESPONSE_SEND`; correlated 200 response returned |
| RTSP OPTIONS | `RTSP_REQUEST` before load balancing and `RTSP_RESPONSE` after `SERVER_DATA`; correlated 200 response returned |
| MQTT CONNECT/CONNACK | `MQTT_CLIENT_INGRESS` before load balancing and `MQTT_SERVER_INGRESS` after `SERVER_DATA`; no `*_DATA` or `*_EGRESS` event fired for these complete packets |
| Closed backend port | `HTTP_REQUEST -> LB_SELECTED -> SA_PICKED -> LB_FAILED -> SERVER_CLOSED -> CLIENT_CLOSED`; no `SERVER_CONNECTED` |

Reached event totals, including baseline repetitions, were:

```text
CACHE_REQUEST 2       CACHE_RESPONSE 1      CACHE_UPDATE 1
CLIENTSSL_CLIENTHELLO 2  CLIENTSSL_HANDSHAKE 2
CLIENT_ACCEPTED 99    CLIENT_CLOSED 95      CLIENT_DATA 76
DNS_REQUEST 1         DNS_RESPONSE 1
HTTP_REQUEST 15       HTTP_REQUEST_DATA 1   HTTP_REQUEST_RELEASE 12
HTTP_REQUEST_SEND 12  HTTP_RESPONSE 12      HTTP_RESPONSE_RELEASE 12
LB_FAILED 2           LB_SELECTED 87        SA_PICKED 81
MQTT_CLIENT_INGRESS 1 MQTT_SERVER_INGRESS 1
RTSP_REQUEST 1        RTSP_RESPONSE 1
SERVER_CONNECTED 86   SERVER_DATA 73        SERVER_CLOSED 87
SERVERSSL_CLIENTHELLO_SEND 1  SERVERSSL_SERVERHELLO 1
SERVERSSL_SERVERCERT 1      SERVERSSL_HANDSHAKE 1
SIP_REQUEST 2         SIP_REQUEST_SEND 2    SIP_RESPONSE 1
SIP_RESPONSE_SEND 1
```

These counts do not imply that every listed protocol case covered every TMM.
Only the initialization and 64-flow TCP coverage phase deliberately covered
the complete roster for the same configuration. Single-transaction protocol
rows establish order on the identified TMM only.

## Priority, suppression, and terminal actions

Separate iRules at priorities 100, 300, 500, 700, and 900 ran in ascending
numeric order in `CLIENT_ACCEPTED`, `HTTP_REQUEST`, and `HTTP_RESPONSE`. The
trace rule at default priority 500 ran according to attachment order among the
priority-500 handlers.

| Control | Measured result |
| --- | --- |
| `event disable HTTP_REQUEST` | Literal load rejection: `"invalid 'HTTP_REQUEST'; expected:all"` |
| `event disable HTTP_RESPONSE` | Same rejection with `HTTP_RESPONSE` |
| `event disable all` at priority 400 | The current handler continued and its post-action log ran. Priority 500/700/900 handlers and every later event were suppressed. Traffic still reached the backend and returned 200. |
| `HTTP::disable` at priority 400 | The current handler and a priority-500 `HTTP_REQUEST` observer still ran, followed by `HTTP_DISABLED`; HTTP parsing then stopped, raw traffic reached the backend, and no HTTP response event ran. |
| `reject` alone | The handler continued, and priority-500/700/900 `HTTP_REQUEST` handlers still ran. The client received reset and no backend request arrived. |
| `reject` then `event disable all` | The current handler continued; later handlers/events were suppressed; client received reset. |
| `HTTP::respond 204` | The current handler continued, and a later priority-500 handler in the same `HTTP_REQUEST` ran. No `HTTP_RESPONSE` event fired and no backend request arrived. |
| `HTTP::respond 204` then `event disable all` | Current handler continued, but the later priority-500 handler was suppressed. No `HTTP_RESPONSE` event fired and no backend request arrived. |
| Querying `HTTP::header` after `HTTP::respond` | Runtime error `Can't call after responding - ERR_NOT_SUPPORTED`; the response was reset. This is distinct from the successful safe responder control. |

The exact sources are the suppression rules generated by
`generate-event-fixtures.py`; the logs and raw responses are
[suppression-log.txt](evidence/evtflow_20261006a/appliance/suppression-log.txt),
[suppression-safe-log.txt](evidence/evtflow_20261006a/appliance/suppression-safe-log.txt),
and the `suppress-*.raw` files under [dev evidence](evidence/evtflow_20261006a/dev/).

## LTM policy ordering

An `http-header insert` action matching `/policy` added
`X-Evtflow-Policy: yes`. The inserted value was already visible to an iRule
`HTTP_REQUEST` handler at priority 100, remained visible at priority 900, and
was received by the backend. For this case, LTM request-policy mutation
preceded the earliest tested iRule priority; it did not execute between the
tested priority handlers.

Directly loading the policy required `requires { http }`. Omitting it rejected
the policy with:

```text
Policy '...', rule 'insert_header'; event 'request' is not available,
add the appropriate profile type as required.
```

Specifying the unrelated `request-adaptation` control loaded the policy but
made attachment require a request-adapt profile. `controls { none }` was
itself an invalid value in a config merge. All variants and loader output are
retained as `23-policy*` and `policy-*-load.txt` in the appliance evidence.

## Profile gates, wrong protocols, and unavailable modules

| Case | Measured result |
| --- | --- |
| HTTP rule containing client-SSL event on HTTP-only VIP | Attachment rejected: client-SSL or persistence profile required |
| HTTP/TLS rule containing server-SSL event without server-SSL profile | Attachment rejected: server-SSL profile required |
| `AUTH_FAILURE` rule on HTTP-only VIP | Attachment rejected: AUTH profile required |
| `ACCESS_POLICY_COMPLETED` rule on HTTP-only VIP | Attachment rejected: ACCESS profile required |
| `ASM_REQUEST_DONE` rule without ASM policy/profile | Attachment accepted. A complete HTTP transaction ran, but the ASM event did not fire. |
| Clear HTTP sent to TLS VIP | Only `CLIENT_ACCEPTED` and `CLIENT_CLOSED`; TMM logged SSL alert 40, `not SSL`; curl returned 52 |
| Non-DNS datagram sent to DNS VIP | UDP `CLIENT_ACCEPTED` and `CLIENT_DATA`, no DNS event, no response |
| HTTP/3 profile assembly | QUIC first rejected without `/Common/httprouter`, then rejected until the client-SSL profile prohibited TLS <=1.2 and DTLS. The final configuration loaded. |
| HTTP/3 runtime | Unreached. Installed curl has HTTP/2 but no HTTP/3 feature and rejects `--http3-only`. A non-QUIC datagram elicited no response or iRule event. |
| APM runtime | Not tested. APM is licensed but unprovisioned; no access profile was created. Literal ACCESS event names loaded, and the ACCESS attachment gate was measured. |
| ASM/AWAF runtime | Not tested. ASM is licensed but unprovisioned; no security policy was created. Literal ASM event names loaded, and a no-policy unreached control was measured. |
| Authentication runtime | Not tested with an F5 AUTH profile or AAA provider. An origin 401 is not a substitute and emitted no AUTH event. |

No module was provisioned solely for this run. Therefore APM login success,
failure, denied ACL, per-request policy, ASM allow/block/violation, bot defense,
DoS, AFM, GTM, Diameter, GTP, RADIUS, FIX, TDS, WebSocket, JSON, XML and ICAP
events remain unmeasured at runtime. Their literal load acceptance must not be
used as runtime support evidence.

## Reproduction

```sh
PROBES=scripts/dev/bigip-probes/event-flow
RUN=EFB

python3 "$PROBES/generate-event-fixtures.py" \
  --run "$RUN" --out /tmp/evtflow-fixtures \
  --vip 192.168.9.24 --backend 192.168.9.80 \
  --vip-port-base 18500 --backend-port-base 18600 \
  --source-commit 6d214350b9f899a1399fa841be7f6fc5edbf225a

(cd /tmp/evtflow-fixtures && sha256sum -c SHA256SUMS)
scp -r /tmp/evtflow-fixtures bigip:/var/tmp/
ssh bigip 'cd /var/tmp/evtflow-fixtures && sha256sum -c SHA256SUMS'

# Start this before the first rule creation.
ssh bigip 'tail -n 0 -F /var/log/ltm > /var/tmp/evtflow-ltm.log 2>&1 & echo $! > /var/tmp/evtflow-tail.pid'

# Independent literal event loads.
ssh bigip 'bash /var/tmp/run-event-loader.sh /var/tmp/evtflow-fixtures /var/tmp/evtflow-evidence'

# External server and bounded client matrix.
python3 "$PROBES/event-flow-lab.py" serve \
  --bind 192.168.9.80 --base 18600 --log /tmp/backend.jsonl
python3 "$PROBES/event-flow-lab.py" matrix \
  --vip 192.168.9.24 --base 18500 --source-port-base 31200 \
  --repetitions 64 --log /tmp/client.jsonl
```

Load each rule and config independently with `tmsh load sys config merge file`
and retain stdout, stderr, and status. Every generated virtual contains
`source-address-translation { type automap }`. The exact post-load virtuals,
pools, profiles and policy are in
[final-owned-config.txt](evidence/evtflow_20261006a/appliance/final-owned-config.txt).

## Evidence, cleanup, and retained appliance state

Primary evidence:

- [complete appliance archive](evidence/evtflow_20261006a/evtflow_20261006a-appliance.tgz)
- [complete client/backend archive](evidence/evtflow_20261006a/evtflow_20261006a-dev.tgz)
- [continuous raw LTM log](evidence/evtflow_20261006a/appliance/ltm-continuous.log)
- [raw client responses and protocol traces](evidence/evtflow_20261006a/dev/)
- [backend observations](evidence/evtflow_20261006a/dev/backend.jsonl), [revision 2](evidence/evtflow_20261006a/dev/backend-v2.jsonl), [revision 3](evidence/evtflow_20261006a/dev/backend-v3.jsonl), and [TLS backend](evidence/evtflow_20261006a/dev/tls-backend.jsonl)
- [protocol event extraction](evidence/evtflow_20261006a/appliance/protocol-event-log.txt)
- [cleanup transcript](evidence/evtflow_20261006a/appliance/cleanup.txt)

All virtuals, pools, profiles, the LTM policy, the named node, the
`EVTFLOW_EFB` partition, and every Common iRule beginning
`__tcl_lsp_evtflow_` were deleted. The partition lookup returned explicit not
found and the exact rule-prefix search was empty. The two recorded backend
PIDs were stopped and ports 18600-18605 and 18614 were absent. The restored
`/Common/self_1nic` remains because it is the appliance's one-NIC data-plane
configuration, not a test object. See
[post-cleanup-verification.txt](evidence/evtflow_20261006a/appliance/post-cleanup-verification.txt)
and [process-cleanup.txt](evidence/evtflow_20261006a/dev/process-cleanup.txt).

The vendor executables required by FIPS integrity remain restored. Update and
telemetry execution remains disabled through supported state: `auto-check` and
`auto-phonehome` are disabled; `f5_update_checker` is boot-disabled and down;
`/shared/f5_update_action` is absent; exact-name process checks found no
`telemd`, `phonehome_upload`, or `updatecheck` process. The final read-only
check is [post-cleanup-callout-verification.txt](evidence/evtflow_20261006a/appliance/post-cleanup-callout-verification.txt).
