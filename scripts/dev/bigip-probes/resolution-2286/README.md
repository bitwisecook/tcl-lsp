# BIG-IP appliance verification for resolution #2286

This handoff measures **real iRule compilation and execution on BIG-IP**. The
fixtures cover original command and variable names, procedure ownership, scope,
mutation, traces, and the `static::` namespace across initialization and actual
TMM events. All appliance outcomes in the manifest are **UNMEASURED**. The goal
is a reproducible evidence bundle, including rejections and incomplete coverage,
not a predetermined pass report.

Use an isolated lab appliance or an explicitly exclusive lab partition/VIP.
Create only uniquely named temporary probe objects. Do not attach probes to an
existing virtual server, alter production objects, change global CMP settings,
load a replacement configuration, restart TMM, or run `save sys config`. The
scripts do not require any of those actions. Runtime `namespace`, `rename`, and
`trace` probes operate only on their generated names; they do not rename stock
commands or delete stock namespaces.

## What each execution context proves

| Context | Evidence it supplies | Evidence it cannot supply |
| --- | --- | --- |
| Rule merge/load | Actual iRule compiler acceptance, warning text, rejection and load-time context restrictions | Successful execution, event validity, TMM locality, or procedure reachability |
| Attached iRule plus real traffic | Reached event, actual responding TMM, result, error, mutation visibility, caller scope | Behavior on an unreached TMM or an event without a transcript |
| `RULE_INIT` log | Actual initialization execution on this appliance/configuration | An assumed once-per-TMM schedule or guaranteed broadcast without later per-TMM observations |
| Appliance `/usr/bin/tclsh*` | Standalone Tcl behavior of that binary | iRule compiler, F5 `call`, event restrictions, CMP, or `static::` broadcast |
| `tmsh` CLI script/iApp | Behavior of that particular execution context | TMM behavior or equivalence to iRule runtime |
| Local C Tcl/Jim controls | Independent language comparison | F5 behavior, appliance version, or physical TMM state |

Keep these contexts separate in the report. A standalone Tcl success does not
repair an iRule load rejection. A literal command may reject the entire iRule
at load even inside `catch`; a dynamically constructed command can therefore
provide a different, separately recorded runtime result. Do not replace one
result with the other.

The initialization contract under examination is: a `static::` value assigned
in `RULE_INIT` is available to subsequent TMM instances, while an assignment in
an ordinary event changes the instance executing that event. Measure that
contract. Do not assume how many times `RULE_INIT` itself runs. Ordinary global
variables are a separate experiment because their use can change CMP eligibility.
A single-TMM result cannot establish multi-TMM locality or broadcast.

## Files and exact source

| File | Use |
| --- | --- |
| `generate.py` | Offline canonical source generator; no appliance connection |
| `irules/*.conf` | Materialized standalone iRules using run identifier `lab2286` |
| `irules/*.conf.hex` | Full original config-file bytes as hexadecimal |
| `irules/*.runtime.tcl` | Exact script value evaluated by an optional dynamic probe |
| `irules/manifest.json` | Object name, source size, SHA-256, line ending and runtime-script bytes/hash |
| `irules/rules.tsv` | Exact filename-to-object mapping for the rule-only driver |
| `appliance-rules.sh` | Collision-checked rule create and ownership-checked rule cleanup |
| `render-lab.py` | Offline pool/VIP config renderer for exclusive temporary objects |
| `traffic.py` | Bounded fresh TCP/HTTP flows with raw response bytes and actual TMM counters |
| `backend.py` | External disposable HTTP backend for forwarded-event tests |
| `result-template.json` | Evidence report schema; fill unknowns explicitly |

Prefer a new run identifier for every independent execution, particularly when
repeating a collision/load-order or dynamic-definition case. Object names and
runtime names include the identifier. The committed `lab2286` fixtures are
ready to inspect, but they must not overwrite a same-named object or runtime
state left by another operator.

From the repository checkout:

```sh
PROBES=scripts/dev/bigip-probes/resolution-2286
RUN=r2286_lab01
python3 "$PROBES/generate.py" --run "$RUN" --out /tmp/r2286-fixtures
sha256sum /tmp/r2286-fixtures/manifest.json
```

The generator refuses an existing output directory. It writes UTF-8 bytes
without a BOM. `physical_words_lf.conf` contains physical backslash-LF
continuations. `physical_words_crlf.conf` differs by replacing each LF with
CRLF, including the continuation bytes. The literal two characters `\n` are a
separate value, not a physical newline. Precomposed `é` is U+00E9, UTF-8
`c3 a9`; decomposed `é` is U+0065 U+0301, UTF-8 `65 cc 81`. Do not normalize
those names, convert newline styles, paste through an editor, or retype braces
and backslashes. Transfer binary file bytes by `scp` or an equivalent exact
file transfer. Verify SHA-256 on the appliance before loading.

```sh
scp -r /tmp/r2286-fixtures bigip-lab:/var/tmp/
scp "$PROBES/appliance-rules.sh" bigip-lab:/var/tmp/
ssh bigip-lab 'sha256sum /var/tmp/r2286-fixtures/*.conf /var/tmp/r2286-fixtures/manifest.json'
```

Compare each source digest to the manifest. If a transport changes bytes,
regenerate/retransfer and retain the mismatched transcript; do not interpret
that run as the intended source case. `.conf.hex` can reconstruct the exact
file offline with Python `bytes.fromhex`; avoid a shell command that interprets
backslash escapes. The runtime `.tcl` files contain only the optional script
payload and **are not complete iRules**.

## Appliance inventory and log capture

Capture these read-only outputs before creating anything:

```sh
date -u
hostname
id
tmsh show sys version
tmsh show sys hardware
tmsh show sys tmm-info
tmsh show cm failover-status
tmsh list ltm virtual one-line
tmsh list ltm rule one-line
ls -l /usr/bin/tclsh*
```

Retain complete stdout/stderr and exit codes. If a command is unavailable on
this version, record that; do not invent the missing value. Note VE/hardware,
licensed vCPUs, blades/guests, HA active device, route domain, client VLAN,
HTTP profile, relevant existing default profile behavior, and time zone.
Identify the actual active TMM roster from the available platform tools.
`TMM::cmp_count` alone is not proof that all instances received traffic.
`TMM::cmp_unit` may repeat on different blades/groups; use group/unit pairs
when that capability is available. If group identity is unavailable, describe
that limit rather than claiming chassis-wide coverage.

Capture `/var/log/ltm` continuously into a run-specific file, including load
and runtime messages. Do not truncate or clear the system log:

```sh
# Dedicated shell; retain its PID so only this capture process is stopped.
tail -n 0 -F /var/log/ltm > /var/tmp/r2286-ltm-raw.log 2>&1 &
CAPTURE_PID=$!
printf '%s\n' "$CAPTURE_PID" > /var/tmp/r2286-capture.pid
```

Start capture **before** rule creation. Preserve rotated-log context if the
capture reports rotation, disconnect, or loss. Keep the raw file; a filtered
`R2286` view is only a convenience. Every traffic request has a `probe_id` so
messages are not identical, making suppression visible. Initialization messages
can still be coalesced; correlate count/timestamps with actual TMM identities
and traffic observations rather than treating duplicate-count suppression as
absence of execution.

## Rule creation and loader classifications

Start with `minimal`, then separately `identity`, `identity_group`, and
`encoding`. Run optional initialization identity separately: its literal TMM
commands may be illegal in `RULE_INIT` on some versions. Its rejection must
not prevent the minimal/static cases from being attempted.

```sh
bash /var/tmp/appliance-rules.sh load /var/tmp/r2286-fixtures \
  /var/tmp/r2286-minimal-journal minimal
cat /var/tmp/r2286-minimal-journal/load-minimal.log
cat /var/tmp/r2286-minimal-journal/load-status.tsv
```

Use a different journal directory for the next independent selection. There is
no load-all mode. The driver proves each exact rule absent before loading,
records the full merge output, and records only objects it created in its
ownership journal. It stops on a failed merge. A nonzero merge may still create
an object: inspect the presence record and clean journal-owned creations.

Classify each loader result using both the raw exit status and raw output:

* `ACCEPT`: object present, loader successful, no warning observed.
* `WARN`: object present and warning observed; record every warning exactly.
* `REJECT`: intended object absent or loader explicitly rejects it; keep the
  message, source line/offset and any partially created object record.
* `INDETERMINATE`: permission failure, unavailable command, changed input,
  incomplete output, interrupted loader, or ambiguous result.

Do not label a rejected rule “runtime unsupported”; it did not execute. Do not
strip warnings or parse only the final line. A rule that loads but fails in
`HTTP_REQUEST` has **two** results: load acceptance and runtime failure.

## Exclusive lab VIP and backend

Choose an unused routable VIP and distinct **external** backend/client host.
Do not use the BIG-IP self IP, loopback, another BIG-IP VIP, or a secondary
address with no valid traffic path as the backend. The backend must be reachable
through the chosen lab VLAN. Confirm routing/firewall availability without
changing production routes. The default rendered VIP uses TCP+HTTP, port 18086,
and automap SNAT to provide a symmetric return path.

On the external backend host:

```sh
python3 backend.py --bind 192.0.2.20 --port 18080 > backend-r2286.log 2>&1
```

Addresses here are documentation placeholders: replace them with the lab's
actual routable addresses. Capture the backend process PID; stop only that
process after testing. Do not kill all Python processes or all listeners on a
port. A second external host is suitable as client. If client and backend share
a host, ensure both real network paths work and report that topology.

Render the config offline after loading the selected rule:

```sh
python3 "$PROBES/render-lab.py" --run "$RUN" --vip 192.0.2.100 \
  --backend 192.0.2.20 --rule /Common/__tcl_lsp_probe_2286_r2286_lab01_minimal \
  --out /tmp/r2286-lab.conf > /tmp/r2286-lab-objects.json
scp /tmp/r2286-lab.conf bigip-lab:/var/tmp/
```

Before loading this **two-object** config, independently prove both exact names
absent. A failed `tmsh list` is only an absence proof if the error explicitly
says not found; permission/connection failures are not absence:

```sh
tmsh list ltm pool /Common/__tcl_lsp_probe_2286_r2286_lab01_pool
tmsh list ltm virtual /Common/__tcl_lsp_probe_2286_r2286_lab01_vs
# Continue only when BOTH are explicitly not found.
sha256sum /var/tmp/r2286-lab.conf
tmsh load sys config merge file /var/tmp/r2286-lab.conf
```

Retain the loader transcript and a separate ownership record for pool and
virtual. Capture their complete post-create configurations. On any partial
failure, inspect and clean only those exact names proved absent before this
creation. Do not proceed with an existing same-named object.

Rule selection, CMP controls, and order changes may modify **only this owned
lab VIP**. Record before/after config and actual status for every such change:

```sh
tmsh list ltm virtual /Common/__tcl_lsp_probe_2286_r2286_lab01_vs all-properties
tmsh show ltm virtual /Common/__tcl_lsp_probe_2286_r2286_lab01_vs detail
```

For each isolated rule, replace the owned VIP's complete rule list with that
exact rule, using `tmsh modify ltm virtual <OWNED-VIP> rules { <EXACT-RULE> }`.
Do not use `rules add` and accidentally retain a previous responder. Capture
that command, its status, and the resulting complete rule list. Every rule with
`HTTP::respond` is a terminal responder; do not attach multiple terminal rules
in one batch. The collision/procedure batches specify their single responder.

## Actual TMM coverage and CMP controls

Use fresh TCP connections. A keepalive connection, a repeated source tuple, or
many HTTP requests does not imply many TMMs. The traffic driver records the
actual response header, source tuple, timestamp, raw response and SHA-256 for
every request:

```sh
python3 traffic.py --vip 192.0.2.100 --port 18086 --run "$RUN" \
  --source-ip 192.0.2.30 --source-port-start 20000 --requests 256 \
  --expect-units 0,1,2,3 --out traffic-identity.jsonl > coverage-identity.json
```

Supply the actual observed roster, not the example `0,1,2,3`. Omit
`--expect-units` until that roster is known; the driver then reports coverage as
unspecified. Source binding uses a real client IP already owned by that host.
Change source port ranges, use additional real client addresses/hosts, or use a
lab-approved load generator to obtain missing instances. Do not add arbitrary
IP aliases or change production CMP hashing to force coverage. Keep attempts
that fail to reach the roster. A result is complete only when the actual
responding identities cover the supplied active roster for **that case and
configuration**. Inventory coverage from another rule is insufficient if this
rule demotes CMP.

Run two configurations of the same owned VIP:

1. CMP disabled on **only** the owned VIP (`cmp-enabled no`), with actual one-TMM
   identity demonstrated. This controls language/event semantics independently
   of distributed state.
2. CMP enabled (`cmp-enabled yes`), with requested config, actual CMP status and
   complete reached active roster. Compare the same cases and original bytes.

Record whether the global rule changes eligibility or observed distribution.
Do not infer “globals shared across TMMs” when globals instead caused all
requests to execute on one TMM. The static and global fixtures must have separate
traffic and coverage files. If only one TMM is available, report locality and
broadcast as **NOT TESTED**, while retaining single-TMM results.

For chassis/multi-group systems use `static_group` plus an active `group:unit`
roster, after separately proving `TMM::cmp_group` is valid in `HTTP_REQUEST`.
The `static` fixture's unit-only label cannot distinguish equal units on
different groups. If the group command rejects, keep that result and restrict
the claim to the measurable topology.

## Static/global initialization, mutation, removal and recreation

Attach only `static` (or `static_group`) to the owned VIP. Creating/loading its
rule sets the generated cell to `INIT_<run>` in `RULE_INIT`. Keep all init logs.
Use the following matrix without modifying the rule between observations:

| Step | Request action | Required evidence |
| --- | --- | --- |
| Initialization visibility | `read` on every active instance | Before/after existence and seed on each actual identity |
| Selected event write | `write&target=<one-identity>` | The designated instance reports `EVENT_<identity>`; other reached identities perform a read |
| Locality observation | `read` on every instance | Only the written identity changes, or record the exact contrary result |
| Selected unset | `unset&target=<same-identity>` | Unset status/result and existence false on designated instance |
| Removal visibility | `read` on every instance | Missing versus seeded state separately by identity |
| Selected recreate | `recreate&target=<same-identity>` | Recreated cell, exact result and existence on designated identity |
| Recreation visibility | `read` on every instance | Recreated value versus untouched seeds |
| New initialization | Clean owned runtime state, delete/recreate only the rule | New init log and re-seed observations per instance |

For example, after reaching unit 1:

```sh
python3 traffic.py --vip 192.0.2.100 --port 18086 --run "$RUN" \
  --path '/?action=write&target=1' --requests 128 \
  --expect-units 0,1,2,3 --out traffic-static-write1.jsonl > coverage-static-write1.json
```

The `target` filter causes other TMMs to read rather than write. Confirm that at
least one response actually reached the target and performed the requested
action; a full-roster read-only attempt with no targeted mutation is incomplete.
Use a different filename for every request set; the driver refuses overwriting.
The same filtering applies to unset and recreate. `static_group` targets are
strings such as `0:1`; supply that exact string in the query.

Repeat the complete matrix with `global`, keeping its own CMP result and source
hash. Do not copy static observations into global rows. Ordinary connection
locals are measured by `event_frames`, not by assuming event-local assignments
are ordinary globals. No fixture uses a shared distributed `table` to smuggle
state across TMMs: introducing one would change the question.

## Cross-rule collision, logical procedure ownership and load order

### Same static spelling

Use `collision_a`, `collision_b`, and `collision_observer`. A and B set **the
same generated static name** in their own `RULE_INIT` handlers, with distinct
`INIT_a`/`INIT_b` seeds. Their request handlers have priorities 101 and 102;
the single observer responds at priority 110.

Run independent object-creation orders A→B and B→A with full cleanup and fresh
initialization between runs. Capture init ordering, rule creation status,
complete VIP attachment order, and response/log values on every TMM. Separately
reverse VIP attachment order while holding creation order fixed. Event priority
remains explicit, so attachment order must not be mistaken for priority order.

For each order: read the seed, send `write_a` on a single fresh connection,
record its actual TMM, then read every instance. Repeat `write_b`. Unlike the
static fixture, collision writes have no target parameter: send one request at
a time and use the returned TMM identity to identify the mutated instance.
Do not run a broad write burst when testing one-instance locality.

Record whether same-name state collides across rules, which initialization wins,
and whether deletion/recreation of one rule reinitializes another rule's view.
Do not assume rule ownership isolates static variables. Do not assume a
consistent observed winner defines a guaranteed ordering without the opposite
creation/attachment controls. The run prefix prevents collisions with other
operators, but intentional A/B collision is part of this experiment.

### Same procedure spelling

Attach `procedure_a`, `procedure_b`, and `procedure_caller`, with the caller as
the only responder. A and B each define a top-level F5 procedure named `same`.
Their own `call same` results reveal local rule ownership. The caller exercises
same-folder `ruleName::same` and absolute `/Common/ruleName::same` routing.
A also uses `call caller_link x` to test whether its real procedure's `upvar 1`
reaches the original local call-site word. Record `x` before/after and the exact
returned result/error. Do not map F5 rule names onto ordinary Tcl namespaces.

Run creation/attachment order controls as above. Attempt `ordinary_literal`
separately: its ordinary command head `same` inside `catch` may be rejected by
the iRule compiler despite a top-level procedure. Dynamic Tcl proc tests have
another implementation/entry protocol and cannot substitute for F5 `call`.

If the appliance supports additional folders, optionally create a uniquely
named **owned** folder under an approved lab partition, generate distinct
rule identities there, and compare local, same-folder, absolute-folder and
incorrect-folder calls. The generator intentionally defaults to `/Common` and
does not create folders. Preserve the full object paths in the report. Do not
reuse a production folder or claim cross-folder coverage from a same-folder
run. A relative route with no explicit owning rule in a standalone Tcl control
has no F5 ownership evidence.

## Optional runtime resolution cases

Load and attach one `dynamic_*` rule at a time. These rules store the script in
an escaped quoted variable and evaluate that original value at runtime. The
manifest supplies the exact evaluated script bytes as well as the original
outer iRule bytes. The iRule compiler can still reject `eval` or another outer
literal command; record that result independently.

`binary scan` converts the returned value and observed `errorCode` to hex for
transport. Validate the `encoding` capability first. Hex is the byte result of
that Tcl conversion, **not a dump of native Tcl_Obj structure, refcounts,
internal representation, or original source bytes**. An absent/unset errorCode
is reported as `UNAVAILABLE`; success may retain a previous errorCode, so do not
attribute a stale value to the successful command. Decode hex offline and keep
the raw bytes alongside a display-friendly string.

| Case | Exact question | Discriminating observations |
| --- | --- | --- |
| `dynamic_namespace` | Namespace variable and procedure resolution | Qualified proc result, namespace variable binding |
| `dynamic_path_shadow` | Local procedure versus fallback/path lookup | Actual support of `namespace path`, chosen LOCAL/GLOBAL; rejection is not an ordering answer |
| `dynamic_global_alias` | Proc `global` aliases original global cell | Global seed reaches proc; separate CMP consequences |
| `dynamic_upvar` | Actual caller-frame formal name | Original caller local changes to 9 |
| `dynamic_upvar_unset` | Alias lifetime across target unset/recreate | Caller observes recreated target or exact error |
| `dynamic_uplevel` | Actual relative frame selection | Selected caller local changes to 42 |
| `dynamic_alias` | Alias prefix arguments and callable selection | FIXED reaches real target; not leaf-name guessing |
| `dynamic_alias_rename` | Alias lifetime when target is renamed | Alias status and renamed direct target separately |
| `dynamic_rename_epoch` | Repeated command word after rename/recreate | OLD cached first call, NEW same-name call, OLD moved call |
| `dynamic_namespace_import` | Export/import callable ownership | Imported proc result and actual import availability |
| `dynamic_namespace_delete` | Namespace incarnation and stale command word | OLD before delete, NEW after recreate, or exact stale lookup failure |
| `dynamic_trace_scalar` | Read/write/unset ordering and recreation | Exact callback name/index/op sequence; post-recreate traces must be measured |
| `dynamic_trace_upvar_array` | Whole-array trace through element upvar | Exact callback coverage and element/name arguments |
| `dynamic_trace_static` | Static cell trace and recreation on real event TMM | Per-TMM callback sequence, unset/free/recreate behavior |
| `dynamic_package` | Actual TMM package surface | `package provide/require Tcl` results or unavailable command |
| `dynamic_package_resolution` | Registered ifneeded/provider identity | Exact own package require/provide outcome; clean only own package |
| `dynamic_unicode_names` | Byte-distinct precomposed/decomposed names | Literal `set` retrieval, distinct stored values and equality result, without normalization |
| `dynamic_unicode_variable_syntax` | Original `$name` variable token boundaries | Separate catch results for precomposed/decomposed spellings; combining-mark tokenization is measured |
| `dynamic_colon_names` | Multi-colon name handling | Qualified original spellings and resulting cell/error |
| `dynamic_read_failure` | Abrupt argument read before later statement | Missing read error; no claim that later assignment executed |
| `dynamic_exists` | Name presence query versus required contents read | Both existence results without manufacturing variable contents |
| `dynamic_expr` | Pooled/normalized numeric result language surface | 3.5, shift, abs and unknown function result independently |
| `dynamic_word_bytes` | Runtime parser's backslash-newline and literal escape | Decoded results, paired with physical original-word files |

Run each accepted dynamic probe first on CMP-disabled single-TMM control,
then with CMP enabled and measured coverage. Definition, alias, namespace and
package state may persist on the executing TMM between requests; record first
and subsequent requests separately. Do not assume a rule removal deletes
runtime-created Tcl commands. Use `runtime_cleanup` on every previously reached
instance between independent cases, and retain its per-name success/error
records. Its names are an exact generated roster, not `info commands` wildcard
deletions. If a cleanup command itself is unavailable, record the residue and
use a fresh unique run for subsequent cases rather than deleting unrelated state.

For stronger frame/trace coverage, extend a **new isolated generated case** with
absolute `upvar #0`, `uplevel #0`, nested procedures, array-element local alias,
namespace frame, `global ::qualified`, and alias retargeting. Keep the full
original source/hash and native results. Treat a command's presence and each
subcommand/version syntax as independent: for example support for `trace
variable` does not prove support for `trace add`, and support for `namespace
eval` does not prove support for `namespace path`.

The generated absolute/zero-frame controls are `dynamic_upvar_absolute`,
`dynamic_upvar_zero`, and `dynamic_uplevel_absolute`. `dynamic_static_global_link`
tests `global static::name` separately from a direct qualified static access.
`dynamic_path_provider` distinguishes namespace-path provider selection from
root fallback; `dynamic_path_shadow` uses the **same** generated procedure
spelling locally and globally. Keep each capability's result separate.

## Event restrictions and forwarded-flow control

Attach only `event_frames` to the owned VIP with the external backend pool.
This rule does not respond locally, so a real successful backend exchange can
reach `LB_SELECTED`, `SERVER_CONNECTED`, and `HTTP_RESPONSE`. Correlate the
client request, backend receipt and complete event log. `event_local` is set
in `CLIENT_ACCEPTED`; each later event catches its own read and records the
status/value on the actual event TMM.

Use HTTP commands only in HTTP events. Use `IP::client_addr` and
`TCP::client_port` only in the connection's client event. Do not place HTTP
queries, respond, backend member selection, or connection-specific commands in
`RULE_INIT` to create a synthetic “working” control. A load-time acceptance of
an event-specific command in `RULE_INIT` does not prove a usable context there.
If a target version rejects an event or command in this rule, isolate that
literal capability in a new fixture with exact bytes and retain the rejection;
do not silently delete the rejected event and claim full event coverage.

Distinguish events that did not fire from events that fired and returned an
error. A local `HTTP::respond` case cannot test `HTTP_RESPONSE` from a backend.
A reused server-side connection may not emit `SERVER_CONNECTED` on every
request; the supplied backend closes its response connection, and the client
also closes each flow. Still prove actual events from logs. Do not synthesize
backend events with a Tcl `eval` wrapper.

## Physical byte/parser controls

Run `physical_words_lf` and `physical_words_crlf` independently through rule
load and traffic, with the original appliance file hashes. Each includes:

* A braced word containing an actual backslash followed by the file's physical
  line ending and spaces before `B`.
* The corresponding quoted word.
* A braced literal `\n` (two original characters).
* Precomposed and decomposed Unicode in one source word.

Compare loader diagnostics and decoded runtime hex. Do not equate load-time
parser behavior with dynamic script parsing: `dynamic_word_bytes` passes a
materialized string value to the runtime parser, whereas the physical controls
exercise the actual iRule source file. Keep braces/quote, physical newline,
literal escape, source channel and materialization as distinct axes.

For name comparison use `dynamic_unicode_names`, which assigns different
values to the two distinct generated names. A display font can render them
identically; the manifest/runtime payload hex is authoritative. Capture
`string length`/equality only as outputs of the actual appliance implementation;
those commands do not establish a particular native string header or Unicode
normalization policy beyond the measured operation.

## Cleanup and absence proof

Runtime state and configuration objects have separate lifetimes. Perform
runtime cleanup **before** removing the final lab VIP while traffic is still
available. Attach only `runtime_cleanup`, reach every instance that executed a
probe, and preserve each exact name's status/error. If ordinary-global CMP
demotion prevents reaching a previously used instance, do not claim its runtime
state was cleared. Record the incomplete cleanup and keep the unique run
identifier reserved. Do not restart TMM or delete broad namespaces to hide it.

Then remove configuration in dependency order:

1. Verify the exact owned lab VIP still has the expected lab-only config; stop
   test traffic, delete only that VIP, and prove its exact absence.
2. Delete only the owned pool and any separately journaled newly created lab
   node/folder objects after verifying no remaining references. The rendered
   pool can cause implicit node creation on some platforms. Record node state
   **before** pool creation and after pool deletion; never delete an existing
   shared backend node. Clean an implicit node only if its exact original
   absence, exclusive probe ownership and lack of references are established.
3. Run rule cleanup for each ownership journal; rules must be detached first.

```sh
# Exact names below are the rendered example; use the names from your manifest.
tmsh delete ltm virtual /Common/__tcl_lsp_probe_2286_r2286_lab01_vs
tmsh list ltm virtual /Common/__tcl_lsp_probe_2286_r2286_lab01_vs
tmsh delete ltm pool /Common/__tcl_lsp_probe_2286_r2286_lab01_pool
tmsh list ltm pool /Common/__tcl_lsp_probe_2286_r2286_lab01_pool
bash /var/tmp/appliance-rules.sh cleanup /var/tmp/r2286-fixtures \
  /var/tmp/r2286-minimal-journal
```

Every failed list must explicitly indicate absence, not permissions or a lost
management connection. The rule cleanup driver compares the current exact rule
config digest to its post-create ownership record and refuses a changed
object. Investigate a refusal instead of removing the check. Do not use a
wildcard delete. Delete commands, their exit codes, and subsequent absence
proofs belong in the evidence bundle. Journal cleanup can be rerun after a
partial failure; already absent exact owned rules are recorded as absent.

Stop only the captured backend and log-tail processes, retaining their final
logs. Leave no saved configuration changes. Record all residual probe objects
or runtime state honestly; residue is a cleanup failure, not a language result.

## Standalone comparison controls

The optional `.runtime.tcl` payloads can be run under independently identified
C Tcl 8.4, 8.5, 8.6, 9.0, 9.1 and current Jim Tcl binaries. Supply an explicit
wrapper that catches the script's result and records errorCode after the catch.
Keep engine executable hash/version, original script hash, output and exit
status. Do not stub `unknown` or pretend `call`, `when`, or `TMM::*` are ordinary
Tcl commands. `static::` has F5 semantics only on actual TMM: creating a stock
Tcl namespace called `static` is a language comparison, not a broadcast model.

An appliance shell `tclsh`, CLI script or iApp result must have its own context
label and evidence file. The host's Tcl version is not proof of the TMM's Tcl
version. Record `package provide Tcl` if the dynamic probe can measure it, but
also retain the exact BIG-IP version/build and command-surface limitations.
The broader C/Jim matrix cannot erase an appliance-specific result.

## Evidence bundle and decision criteria

Return a compressed evidence directory containing:

* Branch/commit identifier, original generated fixture directory, manifest and
  both workstation/appliance hashes.
* Appliance inventory, topology, requested and actual CMP config, active roster,
  complete object ownership journals and loader outputs/statuses.
* Traffic JSONL, complete raw HTTP response bytes, per-case coverage summaries,
  backend transcripts, raw `/var/log/ltm` capture and useful filtered copies.
* Filled `result-template.json`, per-case first/repeated request distinction,
  every actual result/error, unsupported load/event surfaces, and unmeasured
  rows with precise limits.
* Cleanup commands, their original statuses, exact-object absence proofs,
  runtime cleanup coverage and explicit residue.

For each observation state **what actually ran**, under **which rule owner,
original source hash, event, CMP mode and actual TMM**. Preserve names and
result/error bytes. A success row requires a reached event and matching input;
a locality/broadcast row additionally requires the relevant complete roster.
Unsupported syntax, load rejection, unavailable runtime command, guest error,
missing traffic, partial coverage and operator/environment failure are distinct
outcomes. Do not classify them all as “FAIL”, fill expected values from this
repository, or silently remove difficult cases.

The most valuable counterexamples include an event write appearing on another
TMM, an init seed absent on a reached instance, cross-rule static collision,
wrong F5 procedure owner, an alias surviving/dying unexpectedly after rename,
a recreated namespace resolving a stale command, a trace firing through an
unexpected alias/frame, and literal/dynamic parser disagreement. Each needs its
original source and complete transcript so the implementation can be corrected
at the shared semantic owner.

## Copy-and-paste agent prompt

The branch handoff is
[resolution-2286/README.md](https://github.com/bitwisecook/tcl-lsp/blob/work/scripts/dev/bigip-probes/resolution-2286/README.md).
Record the checked-out commit alongside the original fixture hashes.

```text
Use https://github.com/bitwisecook/tcl-lsp/blob/work/scripts/dev/bigip-probes/resolution-2286/README.md and scripts/dev/bigip-probes/resolution-2286/README.md as the exact appliance verification handoff.
Run only on an exclusive BIG-IP lab; create uniquely named temporary owned objects, never modify production objects or save sys config.
Generate a fresh run identifier, transfer original fixture bytes, and verify manifest SHA-256 on both workstation and appliance.
Record every loader status/warning/rejection separately from real TMM event execution; standalone Tcl/CLI/iApp is optional comparison only.
Measure static/global initialization, per-TMM write/unset/recreate, cross-rule collisions/load order, F5 calls, scopes, aliases, renames and traces.
Use real external clients/backend, capture raw /var/log/ltm and HTTP bytes, and prove actual CMP/TMM roster coverage for each relevant case.
Keep byte-distinct Unicode and LF/CRLF/backslash/braced controls exact; report unsupported or unmeasured surfaces without inventing expectations.
Return the full raw evidence bundle and honest results matrix, clean only owned exact objects/runtime names, and prove cleanup or report residue.
```
