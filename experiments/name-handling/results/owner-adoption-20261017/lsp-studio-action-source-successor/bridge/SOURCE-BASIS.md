# Current Main to joint source bridge

Final patch SHA-256: `ac7a976b047564d5a202f7bf27cd0c519a16526e46a106f9affa44e635c3ccf6`.

This31-file patch starts from actual Main with the admitted six-ingredient
composition96e54992 applied. Its six additional ingredients are reviewed Studio21,
703 retained-string successor94dd8a4a, validation current-source-v4, validation
preview-v2, validation irules-v1 and619/623 source-rewrite obligations. Every
ingredient exact patch hash, manifest and review receipt hash is retained.
Studio manifest is exactly8404ed03d533bd439e282ef1c2aa1d544f41f2de583a50ba0e0590f13185a619.
703 source review is exactly669508afddce4e710464655cca03b6331f9956d84b59ee96492b55e528368340.

Every declared original before image matched the preserved original commonbase;
actual Main matched all admitted sixpack after images. Each ingredient was
replayed with fuzz0 and its expected after hashes checked. v4 has no independent
manifest: its exact reviewed patch681dcbb4 and independent-review.json are the
authorities, and its retained v3 parent manifest is explicitly historical. The
v4 helper after hash is recorded as derived from that exact patch replay.

Composition uses disjoint original-commonbase line edits and fails on overlapping
non-insertion changes. Only exact insertion concatenations were needed: contract
appends retain admitted sixpack then Studio then703 where coincident, and the
Server module declarations retain Studio then703. composition.json records every
interval, ingredient and replacement hash. No conflict repair or code semantics
were introduced. Both old bridge-v1 and ingredient packets remain unchanged.

original-commonbase, current-before and joint-after contain all captured union
files, including unchanged admitted sixpack files. manifest.json lists only31
files changed by this bridge. Fuzz0 joint forward replay matched every exact
after image; inverse restored current-before files and removed newly created
files. Main remained byte-identical throughout. No Cargo, test, provider, Git or
Main mutation was performed. Root owns actual application, mechanical formatting,
independent integration review and validation. The reviewed irules multiline
call remains byte-exact; its mechanical formatting is deliberately deferred to
Root's separately recorded actual after-image.

Scoped rustfmt --check exit 1 across 13 Rust files; exact differences are in format-check.log.
