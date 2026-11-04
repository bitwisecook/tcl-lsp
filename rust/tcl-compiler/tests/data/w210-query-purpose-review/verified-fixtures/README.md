# Verified conditional query fixtures

The nine existence-query owner tests pass on frozen base
`f5b2f7d05ea1a322ef8f3f018e1af04394b36477` plus combined patch
`da4e3730ceb54fb84cebe0801e4f7e9699041e881ce623050950d5831030b4b1`.
The separate cfg(test)-only fixture patch is
`f1fb0491898f9a884b825a66917f504121eee1e25891cf39bde4985de20cca8a`.
The coordinator's independent typed-index patch is recorded in
[`index-bounds-subject-identity`](../../index-bounds-subject-identity/README.md),
along with passing build, production Clippy and normal-library emission.
All 42,465 file hashes and the combined patch remain unchanged through gates
and runtime. Seven guard tests, including fourteen guard cases, also pass.

The two strict-None assertions are unchanged. Their old ordinary uncalled
procedure fixture intentionally selects the `world after load` model and was
therefore not a Conditional preview. The new child f exists only in outer's
deferred outcome. Its root witness retains may-absence and no proved target;
possible candidates are not discarded. The original conditional name query,
local context, empty required-read set and name dependency remain positive.
The original missing/foreign metadata refusals now execute and pass.

Separate ordinary after-load and actual `outer; f` controls preserve selected
model metadata, while unknown-history controls refuse strict and diagnostic
queries. Options supply no Native entry/source point; complete-normal proof
remains absent. No production owner, parser, source purpose or permission was
changed to repair the tests. These Rust software checks are separate from Tcl
execution evidence. Full workspace gates remain centrally tracked.
