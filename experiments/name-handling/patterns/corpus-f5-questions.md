# Corpus F5 naming questions

Source coordinates, exact revisions and file/range hashes are in
[sites.json](sites.json), `corpus-f5-partition-call` and
`corpus-f5-sideband-call`. Both are actual procedure-body calls to
partition-qualified rule procedures; commented introductory examples were not
counted. No BIG-IP input has been accepted or run for this supplement.

The maintained [wild-pattern F5 bundle](../../oracles/f5/reports.md#report-9eb804dd1e931e16) maps these five
questions and every portable corpus/Wiki case to separate context records in
[questions.json](../../oracles/f5/captures/README.md) (retained logical path: `experiments/name-handling/patterns/f5/questions.json`). Use its
[answer template](../../oracles/f5/reports.md#report-871ae73990c26b0c), together with
[the existing appliance request](../../oracles/f5/reports.md#report-5f42849056598f7d).
Give these observations stable IDs prefixed `corpus-f5-` and retain exact
materialised rule definitions, attachments, input, version/build/hotfix,
context, compile/load results, runtime results and error/return options.

| ID | Semantic question | Controls and near misses |
| --- | --- | --- |
| `corpus-f5-rule-path` | For a known procedure in a separately installed rule, how do `/partition/rule::proc`, `rule::proc` and a bare local procedure resolve through `call`? Which partition/current rule determines the relative cases? | Same procedure spelling in two rules and partitions; explicit qualified and bare local controls; absent rule versus absent procedure. Keep device object names isolated using the main protocol. |
| `corpus-f5-computed-target` | Does a dynamically built `call` target preserve the same object/procedure identity as its literal spelling? | Build exact target bytes with `set`/list-safe construction; observe compile rejection separately from runtime lookup; unknown target must not be tested only as a literal load-time failure. |
| `corpus-f5-prefix-arguments` | How are fixed and expanded arguments passed to a rule procedure, especially one argument containing spaces, braces or namespace separators? | Literal argument, list-built prefix/expansion where accepted, deliberately space-joined near miss; feature availability and syntax acceptance are separate observations. |
| `corpus-f5-rule-lifetime` | What fixes the binding: compilation, rule attachment, or each runtime call? What happens after replacing or detaching the target rule in a controlled disposable configuration? | Capture old/new implementations and exact administrative operation; use the main protocol's permitted isolation and cleanup. Do not change existing application objects to obtain this answer. |
| `corpus-f5-storage-context` | Do caller local, `static::` and rule/procedure qualified variable spellings address different storage contexts during `call`? | Distinct sentinel values, read/write pairs and independent connections where required. Treat TMM instance/connection lifetime as context, not a C Tcl namespace assumption. |

The JSON source uses `/json/test/json::json_get`; the sideband source uses
`/Common/simple_sideband::request`. These are motivating public examples,
not names to deploy on the appliance. The administrative rule object path is
a separate identity axis from the procedure's Tcl component. Record which
part of the path is a BIG-IP object and whether namespace operations can
observe any part of the procedure lookup; do not assume ordinary C Tcl
namespace behaviour.

[Latest Jim observations](jim-results-5bac7c9/README.md) cover the portable
reductions, retaining setup failures and different APIs explicitly. A rejected
input cannot answer a later lookup; a mock implementation supplies no oracle
result. See the [catalogue](README.md) for provider-specific outcomes.
