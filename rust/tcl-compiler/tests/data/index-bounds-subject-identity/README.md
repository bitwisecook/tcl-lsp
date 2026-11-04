# Typed index-access diagnostic identity

Frozen joint source: `f5b2f7d05ea1a322ef8f3f018e1af04394b36477` plus
combined patch `da4e3730ceb54fb84cebe0801e4f7e9699041e881ce623050950d5831030b4b1`.
The separate index patch is
`3b41d14b86e0c644e66cb7aa8a78c213c11db00b7e39ed26825d1a7f4341084e`;
the accompanying cfg(test)-only query fixture is
`f1fb0491898f9a884b825a66917f504121eee1e25891cf39bde4985de20cca8a`.
All 42,465 tracked source hashes and the joint patch remain unchanged.

The five source-value projection tests pass, including three new actual-finding
controls. The same original index access deduplicates across advisory values
and changed presentation, independently issued or absent subjects refuse, and
the production analyser emits one W230 for the written/proven access. Code,
span, source-word allocation, operand/list ancestry and package metadata must
agree. Message text and supplemental values are not identity.

The joint unit build, production compiler --lib Clippy and normal library build
pass. The new cfg(test) cloning lint was corrected by static review; no full
all-test-target Clippy run is claimed. Full workspace gates remain centrally
tracked. The nine query-owner and seven guard tests also pass on the same unit
binary; their separate receipt records that test-only change. No Native entry,
execution, value, completion or edit permission follows from deduplication.
