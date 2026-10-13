# Scoped MCP cache-provenance amendment

Root confirmed that the coordinated team kept the actual default Cargo target
exclusive during the selected MCP build, 2026-10-13 00:33:33–00:36:41 UTC.
Root ran no default-target build; the coordinated Core/Registry/Runtime builds
used explicit `tmp/name-target-runtime` targets under their separate leases.
This is a scoped coordination attestation. External host processes were not
independently attested, so absolute host exclusivity is not claimed.

The [original capture](../validation-01/README.md) honestly records that the
runner omitted explicit `CARGO_TARGET_DIR` and Cargo used
`/workspace/tcl-lsp/target` rather than the granted issues cache. Its initial
exclusivity uncertainty is resolved for the coordinated team by the
[annotation](annotation.json). That capture's raw outputs, derived files,
source hashes, original failure and SHA256 manifest remain unchanged.
No source edit, repeat build or control accompanies this amendment.

The existing result remains: selected MCP build passed; inventory/helper
controls passed; generator control failed on `[]` versus `["helper_dg"]` after
its single-capture assertion passed. The datagroup failure remains unresolved.
