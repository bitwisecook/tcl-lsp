# Scalar alias references before following quoted text

Question: which scalar-source spelling preserves the alias value before a
namespace, identifier, Unicode or array-looking suffix under the selected
source grammar?

[Controlled observations](oracles/controlled/manifest.json) retain the exact
ASCII driver, provider and original init hashes, invocation, completion, raw
stdout/stderr and before/after integrity records for C Tcl 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0 and the identified pinned Jim build. Every provider
process exited zero, with empty stderr. This is ordinary interpreter evaluation;
Tcltest was not used. The manifests retain their original capture paths.

`${a}::tail`, `${a}x` and `${a}(k)` preserve the scalar alias followed by
literal text; the corresponding bare forms select another namespace variable,
scalar or array element. A single colon remains literal for both spellings.
`${a}é` preserves the suffix in all six providers. Bare `$aé` preserves it
on the five C providers, while the captured Jim grammar selects the longer
Unicode name.

[The initial trial](oracles/initial-excluded/manifest.json) is retained but
excluded as array/Unicode authority: shared scalar state prevented its array
setup, and its binary UTF-8 value was evaluated without the required C source
conversion. Controlled cases independently unset `a` and form Unicode source
with the original interpreter's `format %c 233`.

These observations prove source-reference/value behaviour only. They do not
prove alias freshness, movement, whole minifier equivalence, F5 execution,
physical frames/cells, edit authority or Rust acceptance. The Jim capture is
bounded to the recorded revision/build and does not establish latest-upstream
behaviour.

`python3 replay.py --repo /absolute/checkout --out /absolute/new-directory`
checks the original provider/init hashes and writes a separate replay. Missing
or changed providers are an unavailable replay, never permission to substitute
another interpreter or overwrite captured observations. No replay was performed
when copying these byte-identical evidence files into this durable location.
