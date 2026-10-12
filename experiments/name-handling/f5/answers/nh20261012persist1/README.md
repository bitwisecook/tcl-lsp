# Native visibility and state persistence measured on BIG-IP

BIG-IP 21.1.0.1 build 0.0.26 accepted the owned provider rule, caller rule and TCP/HTTP virtual server. This capture contains 287 reached measurements: 28 RULE_INIT, 124 native-procedure and 135 HTTP_REQUEST event measurements. The native procedure executes 16 times as case measurements within RULE_INIT on four units and 108 within HTTP_REQUEST on unit `0:0`. These are new bounded controls, not rewritten answers for the original portable corpus.

The three context folders for the main job describe one configuration and one execution. `evidence-alias.json` marks the RULE_INIT and native-procedure folders as context-filtered byte-identical copies of `tmm-traffic-event-0`. The applied main configuration includes the same `tcp` and `http` profiles throughout. The independently executed preflight and cleanup rules have their own receipts.

| Measured surface | Actual result |
| --- | --- |
| Ordinary computed bare head naming a native provider procedure | Code 1, invalid command name, in RULE_INIT, native-procedure and HTTP_REQUEST contexts. |
| `info commands`, `info procs`, `namespace which -command`, each for bare and F5-qualified native names | All six catalogue entries empty; the adjoining F5-qualified `call` returns code 0 and `NATIVE CATALOG`. |
| Computed ordinary `rename` of the F5-qualified native name | Nested code 1, command does not exist. The old F5 `call` still succeeds; `call` of the moved name returns nested code 1. No successful rename or restore is claimed. |
| Explicit namespace cell, regular global and `static::` cell seeded during RULE_INIT | All three read `SEED` during RULE_INIT and its native procedure on all four units; first HTTP_REQUEST and native procedure read the same values on actual unit `0:0`. |
| Owned dynamic procedure created during RULE_INIT | Returns `DYNAMIC` in the later event/procedure measurements, including after the three cells are unset. |

Sixteen ordinary request captures and eleven paired keepalive responses give 27 actual HTTP requests, all with status 200 and actual responding unit `0:0`. The paired responses, NHO invocation identifiers and server-side peer ports identify two continuing TCP connections; they do not rely solely on reusing a client object.

| Controlled request | Peer port | Measured state |
| --- | --- | --- |
| `keep-a-write` | 51992 | A's connection cell becomes `WRITE keep-a-write`. |
| `keep-b-read` | 51993 | B's connection cell remains its own `SEED` value. |
| `keep-a-read` | 51992 | A still reads `WRITE keep-a-write`. |
| A `keep-worker-write`, then B `keep-worker-written` | 51992, then 51993 | B sees all three explicit shared cells as `WRITE`, both directly and through the native procedure. |
| A `keep-worker-unset`, then B `keep-worker-missing` | 51992, then 51993 | Each of B's three cell reads has nested code 1/no such variable; the outer result remains a successful list of these measurements. |
| A `keep-worker-recreate`, then B `keep-worker-recreated` | 51992, then 51993 | B sees all three cells as `RECREATE`, both directly and through the native procedure. |
| Ordinary request invocation `8`, after the paired actions | New peer port 51994 | Shared cells still read `RECREATE`; the new connection cell is freshly seeded. |

The mutations run once in the actual HTTP_REQUEST operation. The native procedure then reads those cells without reseeding them. This proves the measured distinction between explicit shared cells and the bare connection cell on unit `0:0`; it does not establish propagation to a different TMM unit. The recorded `namespace current` is `::`; recorded `info level` is 0 for init reads, 1 for the init-called native procedure, 2 for the event evaluation and 3 for the event-called native procedure. These are the captured evaluation frames.

Preflight proved the unique namespace, ordinary cells and candidate ordinary command slots absent on all four actual units (`0:0`–`0:3`). Cleanup acquired the exact run marker on each unit and returned `owner 1 namespace 0 regular 0 static 0 marker 0 fallback-dynamic {} namespace-dynamic {}`. It removes only the owned dynamic namespace, fallback dynamic command and explicitly owned regular/static cells. Exact configuration queries after deletion return the named objects as not found. Closing the owned client connections supplies their lifetime boundary; no additional read after close is claimed. Configuration deletion does not prove retirement of an activated native procedure catalogue, and this run makes no such claim.

The retained metadata shares dictionaries between context arrays, so it also contains an HTTP_REQUEST wildcard event map on some RULE_INIT items. The report applies that invocation map only to native-procedure records. RULE_INIT and traffic contexts retain their actual capture event; `init-procedure` explicitly selects RULE_INIT. Executed source and metadata snapshots remain unchanged.

`independent-review.json` records checks of every measurement hash and raw line, operation materialisation, aliases, exact event projection, paired ports and mutation readouts. `reviewed-proofs.json` selects 95 narrow records: every init identity, every init-called native procedure identity, event/procedure baselines before and after the paired mutations, and every paired shared/connection cell observation. All other exact records remain in `records.json`.

Results use the retained `binary-scan-tcl-value` encoding. The readable ASCII values here are reversible byte projections; no Tcl Unicode or UTF-8 encoding is inferred. Nested results are captured Tcl list data; inner error options are unavailable unless separately recorded. Provider, configuration, input, invocation, raw logs and owned cleanup receipts remain under this directory.
