# Incoming external-global seed repair

The baseline public native-entry probe reproduced
four seed-map failures on genuine retained top-level reads: a reachable global
writer masks independent `argv` and `env` provenance, while a qualified `argv`
read and a root alias miss the seed. T100 still emits from independently unknown
contents, so this does not establish a missing diagnostic or a cause of #2243.

The shared `VariableCellKey` owner now projects an incoming root-global address
only against the same retained, closed native entry. It keeps array element
identity and refuses recreated lifetimes, authored addresses, local cells and
foreign or non-root namespace storage. Taint reads the special-variable surface
from the same retained input's profile and joins declared external provenance
with an existing conservative seed. A display name or separately supplied
dialect cannot select that physical source. Missing physical entry contributes
no seed; existing authored formal and unknown-content transfers remain separate.

On sealed source `82ff2b798c86b54bb95d3f7bb1142d868cb71157` plus patch
`f9c132b4bf1ce4e22a3cae040e85e5660968e7729ef904be9b1c0376ffa7110a`,
production compiler Clippy and the compiler library test build both exited 0.
All 42,917 hydrated tracked files retained identical hashes across the build.
The receipt records commands, actual exits, source hashes,
binary hash and the earlier failed/interrupted attempts. The raw Clippy log,
build log and six process streams remain alongside it.

Twelve counted tests passed in six fresh processes: four new fixed regressions,
four existing provenance/formal/global-writer controls and four interruption
and fresh-retry controls. The positive tests require the original retained
read and matching native-root cell before checking incoming provenance. The
formal guard asserts a genuine retained activation cell without claiming an
exact diagnostic read. Recreated-lifetime wrappers test projection refusal
only. Native capture attests command and namespace registration, not a bound
incoming variable, its value or a successful execution.

The frozen test executable has SHA-256
`b8fab38db77645ebf5691d00af8c38da105fcfd2f16bf33348a73128e21069fb`.
It and the complete source manifests remain under ignored
`tmp/lsp-verified-artifacts/2243/seed-candidate-attempt3/`.
`make rust-check` and `make prep-pr` have not run for this checkpoint.
Both exact IMAP4 `argc` reads remain unavailable; #2243 stays open.
