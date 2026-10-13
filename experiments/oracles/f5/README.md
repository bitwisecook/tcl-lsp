# F5 oracle evidence

Start with the [question index](questions/index.json). Each listed item has one
Markdown answer and adjacent JSON containing the context matrix, selected
measurements, exact source, command/capture selectors and replay recipes.
[F5-N01](questions/F5-N01.md) through [F5-N16](questions/F5-N16.md) retain the
broader requested questions and explicitly scoped witnesses and gaps.

```text
base/                 placeholder topology and shared deployment configuration
groups/<group>/       deployment setup and minimal reconstruction deltas
questions/<id>.md     question, answer, filtered results and evidence
questions/<id>.json   structured context, source and execution references
runs/<run>/           observations and shared provider/cleanup descriptors
captures/             one indexed, deduplicated capture store
tools/                maintained capture, parsing and review entry points
fixtures/             product fixture index; separate from oracle observations
materialise.py        local inspection, rewriting and reconstruction
```

The stored base network is **192.168.1.0/24**, a placeholder. Materialisation
accepts an aligned user-supplied IPv4 `/24` and individual host overrides.
Existing lab routing, self-IP and client/backend reachability are explicit
preconditions; this application-level base does not replace appliance management
or interface configuration.

```sh
# Show every declared/configured replay address and its proposed target.
python3 experiments/oracles/f5/materialise.py addresses --all \
  --network 192.168.9.0/24 --map .33=.75

# Find the exact case recipe for a retained configuration.
python3 experiments/oracles/f5/materialise.py cases --group native-resolution

# Produce local configuration and command plans; this does not access BIG-IP.
python3 experiments/oracles/f5/materialise.py materialise \
  --group native-resolution --case CASE \
  --network 192.168.9.0/24 --map .33=.75 --output /tmp/f5-case \
  --appliance-directory /var/tmp/f5-case
```

The output contains `base.scf`, `group.scf`, `test.scf`, the object comparison,
command plan, client/server bindings, address inventory and an F5-compatible
TOML map. `topology.json` records actual configured ports, route domains and
pool relationships alongside the external roles. The command plan names an
owned appliance staging directory: check absence/ownership and copy the SCFs
there before invoking the load commands. The F5 CLI owns inspection, rewriting and
object comparisons. Address rewriting is atomic, including host exchanges;
ports and route domains survive. Distinct source hosts cannot map to the same
target. Reserved hosts, absent sources and duplicate overrides are rejected.

The current naming capture configurations contain LTM endpoint fields and no
`net self` or `net route` objects. The address inventory queries their virtual,
node and pool-member fields; masks are not endpoint addresses. Declared client
and self-IP roles are labelled separately from addresses actually configured
in the selected files. New object families need explicit inventory/binding
support before being added to this corpus.

Known endpoint literals are projected in configuration and literal references
using the F5 address-map engine. Encoded Tcl test payloads are unchanged.
The command manifest records the tool version, selected case and generated
filenames. Existing proof hashes and content-store identifiers are retained
for compatibility and deduplication; replay plans add no trust ceremony.

Group deployment setup and reconstruction templates have distinct roles.
`group.scf` contains passive owned scaffolding. `template.scf` is the group's
shared source/configuration model, with placeholder addressing. The JSON
descriptor identifies its historical input and stores small per-test source
deltas; the object comparison shows changes from that model. **Do not load that
template as passive setup**, because its `RULE_INIT` bodies can execute tests.
Its first programme is a test in its own right. A future measured group can
share more active setup only where doing so preserves the question's procedure
owner, loader, context and lifecycle premises.
The selected historical driver owns absence checks, actual invocation, ordered
phases and teardown. Shared event frames, suspended frames, persistent cells
and activated native procedure catalogues cannot be reset by assuming that
configuration deletion retired them.

For a fresh RULE_INIT invocation, generate a comment change inside the selected
iRule's source:

```sh
python3 experiments/oracles/f5/materialise.py trigger --input /tmp/f5-case/test.scf \
  --rule /Common/RULE --nonce 42 --output /tmp/f5-trigger
```

The generated `trigger.scf` contains just that rule and a trailing Tcl comment.
`trigger.json` gives the verify/load commands and required measurement receipts.
Choose a new numeric nonce for each change. The changed source is a new execution
input; source introspection can see it. F5 documents that saving an iRule triggers
[RULE_INIT](https://clouddocs.f5.com/api/irules/RULE_INIT.html). Confirm the actual
worker/invocation receipts rather than treating load success as an observation.
Use fresh capture identities and log cursors: a comment change can leave the
probe's emitted markers unchanged, so repeated invocations must remain distinct.

`--trigger-kind reload` instead produces a `tmsh load sys config` plan. This loads
the appliance's saved configuration and can replace unsaved probe changes or
invoke other rules. Record the saved/live configuration and observed executions.
Neither trigger establishes a clean variable state or native catalogue reset.

Read retained source, commands, queries or measured output by logical path:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read \
  --path RECORD_PATH --json-pointer /ROW
python3 experiments/oracles/f5/materialise.py verify
# Restore an historical driver and its companion files outside the checkout.
python3 experiments/oracles/f5/materialise.py extract \
  --path HISTORICAL_DIRECTORY --output /tmp/f5-original-driver
```

The logical paths are historical identities, not expanded checkout files.
The proof reader resolves them directly from the indexed store and verifies
their existing hashes. [Historical reports](reports.md) can also be read this
way. Capture completeness, duplicate/missing chunks, full return options and
raw phase receipts remain available even where the question view filters them.

The 50 original questions retain 400 context rows. Coverage is complete for
93 reached, reviewed lookup rows. The other 307 retain partial prerequisite,
capability or attempt evidence; their final lookup gaps remain explicit.
The historical 307 investigation records are retained independently of this
coverage; their closure labels do not override the current question matrix.
The broader F5-N questions are partial or unanswered where variants remain open.
No new appliance measurement is implied by reconstruction or compaction.

The naming oracle trees contain no PCAP captures. Their log observations do
not answer packet-level or backend-flow questions. Product flow fixtures and
the [flow lab](../../../scripts/explain_flow_test_harness/README.md) are separate.
For future wire measurements, retain the relevant complete conversations,
configuration, client/backend correlation and versioned decoding commands.

Retained sanitisation notes remain authoritative. Display text is decoded as
UTF-8 only when the recorded encoding says UTF-8; Tcl-value/binary encodings
remain explicit hex. Preserve the distinction between execution-time hashes
and redacted or replay-projected bytes.
