# naming.numeric.c84-integer-updater-build-pair

Kind: `native-observation`

## Problem statement

Integer updater authority depends on separately pinned native producer builds and reached objects, not a release label or shared header alone. The complete finite output and the actual archive versus loaded-image producer must be retained independently.

## Question

Does the unchanged original C8.4 integer-updater probe produce byte-identical complete 812-row observations from the separately pinned archive and its independently linked/loaded image?

## Conclusion

All four actual link/compile/archive-probe/loaded-image-probe commands exit 0 with empty stderr. Both complete 812-row outputs are byte-identical to the unchanged observations.tsv SHA cc19ad501bb96674935c4dd855ec055b3095b97673f1dca64f5cdc8a06a6929c. For each exact fresh long/wide specimen the measured type name stays unchanged, the exposed string-bytes flag changes0→1, errno is unchanged, and the complete counted byte result remains recorded. The archive SHA 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47, header SHA 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf, linked image SHA 66b79b3754bae00916073872f0eb2e812c45dd21cb24be0d092d6e17820c53c2 and probe SHA ec96f46e8f86c1dc7ca87506214fa33c18c30f4afdd0365452dade027e4f1c24 remain distinct identities. Equal finite outputs do not establish universal integer formatting, another archive/image, ABI compatibility, native object identity across producers, compiler/frame/admission or an executed Rust pass. C8.5/C8.6/C9.0/C9.1/Jim/BIG-IP are not tested.

A separately measured GNU17 build has archive SHA `dc67eba353eeb83cbfb9ce5e0f6ff52972297374799156c09cef1ac06cc5cfff`, the same exact header SHA `824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf`, configuration SHA `33d929abbd1102f01624ca3aa217af1bcdc7e6023be2ea094e82e5228f8880e9`, and independently linked/loaded image SHA `b381688d8da3cee9d558eef7a38503980a10635782a28f56141c7e5bc503892f`. Its archive and loaded-image probes each preserve all 812 original rows byte-for-byte. This directly measured pair is eligible for exact build admission; the Rust loader/updater test remains a separate execution obligation. The older `44b069f1` archive is a different build, its mismatch cause remains unknown, and neither that build nor a future different build is admitted by source/configuration agreement.

A marked pure software control links all three complete digest pairs and mismatch/empty refusals to the formatter build-admission owner.

## Scope

The unchanged original probe.c SHA 2c1e656d0a1bdc6a86f84a08a8601cb414eef16ce7f53ec9b1d882c99f91ff6e creates fresh public Tcl long/wide objects, attaches each to its interpreter result, reads exposed Tcl_Obj type/string fields around reached Tcl_GetStringFromObj, and records pre/post errno and counted result hex. One probe uses linked archive functions; the other independently dlopens the exact image and obtains the stated public symbols with original flags. The version label identifies the pinned source/header/build; this original probe does not query a runtime version. No Tcl_Eval source/channel or private bytecode claim is involved.

This definition performs no native linking, ABI loading or getter/updater call; source linkage proves neither library availability nor passing assertion. The two original812-row updater operations and their separate build identities remain the native evidence. Admission-table membership alone supplies no modern release, foreign image, numeric formatter or default C ABI authority.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original C API fresh Tcl_NewLongObj/Tcl_NewWideIntObj and reached Tcl_GetStringFromObj; direct archive versus original independently loaded image symbols. Version: tcl8.4 8.4.20.

Archive and independently loaded image each return the unchanged complete 812 observations with process exit0/empty stderr. All type/string flag/errno/hex fields are retained byte-for-byte; finite equality grants no other build or universal formatting claim.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No execution answers this exact formatter build-pair question.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [rust/tcl-test-support/src/native_integer_formatter.rs](../../../../rust/tcl-test-support/src/native_integer_formatter.rs), `native_integer_formatter::tests::loaded_c84_updater_matches_812_original_archive_and_image_windows` (linked): Strict admitted archive/header pair and independently loaded image compare all 812 unchanged original type/string-flag/errno/counted-byte rows. Linked test source supplies no executed pass or authority for another build/header alone.

- [rust/tcl-test-support/src/native_integer_formatter.rs](../../../../rust/tcl-test-support/src/native_integer_formatter.rs), `admitted_reference_build`: Match the complete recorded archive/header pair rather than admitting a formatter from a release or either digest alone.
- [rust/tcl-test-support/src/native_integer_formatter.rs](../../../../rust/tcl-test-support/src/native_integer_formatter.rs), `load_pinned_c84_integer_formatter`: Independently verify complete build inputs, produce a linked image, recheck the inputs and supply that separate image digest to the actual ABI loader; table admission is only one prerequisite.
- [rust/tcl-test-support/src/native_integer_formatter.rs](../../../../rust/tcl-test-support/src/native_integer_formatter.rs), `native_integer_formatter::tests::formatter_admission_requires_a_complete_verified_build_pair` (linked): A pure admission-table control accepts each of three exact original archive/header digest pairs, rejects either substituted digest and rejects the empty pair. It does not run either812-row native updater experiment, link a formatter, load an image or call the ABI.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_integer_formatter216](../../../../rust/tcl-registry/tests/data/native_integer_formatter216)
