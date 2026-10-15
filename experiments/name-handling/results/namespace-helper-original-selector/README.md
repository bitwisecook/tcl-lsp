# Original Jim namespace helper selector

The question is whether a recognised abbreviated namespace selector looks up its
canonical helper spelling or the original selector. The retained C probe observes
12 cases in cold and reuse phases (200 rows) against the pinned original Jim
revision identified in `upstream-check/receipt.json`. It uses the original
`Jim_EvalNamespace` and core/static-extension initialisation. The binary/library,
source, configuration, exact invocation and raw channel hashes are in
`manifest.json`; no new oracle execution is claimed here.

An accepted `upv` selector looks up `namespace upv`; replacing only
`namespace upvar` does not donate that binding. A replacement abbreviated helper
receives the untouched original argument object. An exact selector with a NUL
uses the selected CString extent, while an abbreviated selector with a NUL
fails the counted prefix comparison. The Runtime regression checks ten cases in
20 phases, including that refusal. Public arity diagnostics and the original
subcommand-cache header remain separate recorded stages; the regression grants
no complete native cache/final-free parity.

`initial-tool-*` explicitly records a reconstructed exploratory tool transcript;
the separately compiled C probe and its retained raw channels are the primary
evidence. `capture.py` and `check-upstream.py` are exact historical scripts with
historical paths. Their upstream receipt establishes latest only at its capture
time. Reproduction requires the exact provider hashes from the manifest and a
fresh output directory; a changed provider needs a separate capture.
