# Joint selected source validation

The selected six-library offline build passed. Fifteen exact controls ran
once each on the six pinned current binaries: 13 passed and two failed.

- [Metadata](metadata/README.md): 10 passed; Registry DynamicNonOption bit and
  original braced-expression data-group controls failed.
- [Compiler source selectors](compiler-source-selectors/README.md): both passed.
- [Corrected VM same-position fixture](vm-same-position/README.md): passed.

[Shared build/source provenance](build/README.md) and
[combined result](validation.json) preserve the actual common source scope,
exact build, raw outputs, changed-owner images and pinned artifact identities.
All 11,803 frozen hashes and six binary hashes stayed unchanged. No assertion
was weakened, source repaired, failure repeated, MCP/integration/full-suite
control or interpreter oracle executed in this capture. No blanket proof or
provider status is promoted. All source/cache leases are released.
