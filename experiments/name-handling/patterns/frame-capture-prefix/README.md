# Does namespace capture change a forwarded caller-relative link?

The exact plain prefix returns `changed 0` on C Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4 and 9.1.0. Adding only `namespace code` returns `original 1` on those
same providers: the caller local remains unchanged and the spaced namespace
cell exists. Pinned Jim refuses the unchanged alias setup, so the final
question is unreached. F5 host/context questions remain unanswered.

These supplemental inputs have no canonical proof registration or measured
static-consumer outcome. They establish neither future callback/edit authority
nor whole-application conformance.

## What source and exact question are retained?

[Source proposal receipts](proposal.json) reuse exactly three reviewed contexts:
[ruff `parse_preformatted_state`](https://github.com/georgtree/ruff/blob/b63da69eed625bc544d1f4da76371e8bd6b02d29/src/ruff.tcl#L1137)
(`corpus-state-upvar`, manifest index 3, revision
`b63da69eed625bc544d1f4da76371e8bd6b02d29`),
[pave `window`](https://github.com/aplsimple/pave/blob/875de1f1539d62e46e48e206f34f82848bca1736/apavebase.tcl#L3673)
(`corpus-frame-forward`, index 5, revision
`875de1f1539d62e46e48e206f34f82848bca1736`), and the existing
[Wiki namespace-code snapshot](https://wiki.tcl-lang.org/page/namespace+code)
(index 2, retrieved 2026-10-12). Corpus full-file/range hashes match local pinned
bytes. Existing licence coordinates/hashes are retained; no source or licence
text is copied. The Wiki receipt has no inspected licence or verified revision.

The sites supply separate caller-link, forwarding and namespace-capture
ingredients. None contains this exact composition. Ruff's state array and
pave's OO receiver/expanded arguments remain unmeasured. This selection adds no
source-site count or prevalence claim. [Original proposal notes](proposal-notes.md)
retain the pre-execution hypotheses separately from the results below.

The question is: **After exact successful setup, which caller-local value and
namespace-cell-existence result does `::writer::caller` produce?**
[Plain input](plain-prefix.tcl) forwards `[list ::dispatch {a b}]` through
`uplevel 1`; the same-interpreter alias targets `::writer::assign`, which uses
`upvar 1`. [Captured input](namespace-captured-prefix.tcl) adds only
`namespace code` around that prefix. The source's final physical line is the
question; preceding lines are exact setup. Neither input catches guest errors.
Their final result observes the caller-local value and namespace-cell existence;
it does not read that namespace cell's value.

The existing `frame-wrapper`/near-miss, `wiki-upvar-3..4`, `wiki-code-1..2` and
`wiki-alias-1..2` inputs do not compose this namespace wrapper with an
uplevel-forwarded aliased relative link. Existing original questions and
answers therefore retain their identities; these are separate authored inputs.

## What actual provider outcomes were captured?

| Provider | Plain prefix | Namespace-captured prefix |
| --- | --- | --- |
| C8.4.20 | setup `0`; question `0: changed 0` | setup `0`; question `0: original 1` |
| C8.5.19 | setup `0`; question `0: changed 0` | setup `0`; question `0: original 1` |
| C8.6.18 | setup `0`; question `0: changed 0` | setup `0`; question `0: original 1` |
| C9.0.4 | setup `0`; question `0: changed 0` | setup `0`; question `0: original 1` |
| C9.1.0 | setup `0`; question `0: changed 0` | setup `0`; question `0: original 1` |
| Jim `0.84-9-g5bac7c9` | setup `1: wrong # args: should be "interp"`; question unreached | same original alias setup refusal; question unreached |
| BIG-IP/F5 | each native host/context unanswered | each native host/context unanswered |

[C8.5–9.1 and Jim records](results/records.json),
[their provenance](results/provenance.json),
[final C8.4 records](c84-final-results/records.json) and
[C8.4 provenance](c84-final-results/provenance.json) retain exact source copies,
wrapper bytes, raw stdout/stderr, setup/question completions, return options
where supported and error globals where present. No background errors or
nonzero process exits occurred in the reached final C captures. Successful
driver exit is separate from guest completion and supplies no additional
semantic authority.

The first C8.4 capture attempts failed in the driver before the final question:
its bytecode compiler rejected three-variable `catch` inside a skipped nested
branch. Those raw failures remain in `results/`, with the exact
[first driver](driver-first-attempt.py). The
[corrected driver](run.py) invokes `catch` indirectly and selects its native
arity by provider; a fresh C8.4-only output directory preserves the unchanged
source/setup/question bytes. The correction changes no guest input.

Each stock binary has its matching `TCL_LIBRARY`, init and namespace-source
hashes plus measured patchlevel. Jim matches the previously retained binary and
upstream revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, native embedded
init/support source and configuration hashes. No C startup library substitutes
for Jim; its support library loads from the pinned tree. No Tcltest test is
invoked by either probe. The prior Jim HEAD check is reused without a new
remote revision assertion. No guest alias syntax is adapted to repair refusal.

For a fresh reproduction, retain a new output directory:

```sh
python3 experiments/name-handling/patterns/frame-capture-prefix/run.py --dest /tmp/frame-capture-new
```

`--provider jim` and `--provider tcl8.4.20` select bounded channels. Do not
overwrite retained evidence. The [F5 source/context request](F5-REQUEST.md)
requires actual native-host acceptance and reached execution; desktop results
or a mock supply no appliance answer. No network, message or appliance load
was performed here.
