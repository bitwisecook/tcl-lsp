# naming.encoding.original-utf8-convertto-object-storage

Kind: `native-observation`

## Problem statement

Counted String input bytes, ByteArray octets and external UTF-8 output are different operands. Treating an unchanged Unicode input object as a converted external binary object can expose E9 instead of C3A9 to a later binary getter. Release-specific surrogate/astral interpretation and result storage require their own original object-vector controls.

## Question

For ten fixed counted String or ByteArray operands, what original completion, result primary/string residency before getters, input materialisation and raw UTF-8 octets does encoding convertto produce on each pinned C release, and what separate availability/invocation answer does the exact initialized Jim image supply?

## Conclusion

All sixty fresh host processes exit0 with empty stderr. The fifty original C object-vector calls retain46 code0 results whose primary is bytearray with no resident string before the getter, and4 C9 code1 results whose primary is string with resident bytes. C9 rejects the fixed surrogate-pair and isolated-high-surrogate String inputs with the retained illegal-sequence diagnostic/errorCode. The successful eacute operand returns C3A9; both fixed zero forms return00. A ByteArray containing C3A9 octets returns C383C2A9 rather than the unchanged octets. C8.4/8.5 retain the fixed surrogate pair as six output bytes and interpret the fixed astral input as eight output bytes; C8.6 combines the pair and preserves the astral four bytes, while C9 accepts the astral input and rejects the two surrogate cases. ByteArray input retains its primary while acquiring resident string bytes; String input keeps its none primary and resident bytes in these windows. Each Jim availability control returns a zero command count and caught code1 invalid-command diagnostic for encoding in this exact fullInit image; it supplies no C input/object-storage observation. These fixed fields establish neither arbitrary encoding behavior nor private object identity or software execution success.

## Scope

The unchanged ASCII C probe constructs each exact counted Tcl_NewStringObj or Tcl_NewByteArrayObj operand and invokes the original four-word object vector with Tcl_EvalObjv/TCL_EVAL_GLOBAL in a fresh initialized process. INPUT and RETURN/INPUT_AFTER primary and string-residency fields precede the result binary/string getter; RAW, MESSAGE and ERROR_CODE are independent subsequent channels. No CLI/source parser, input replacement or Unicode rendering substitutes the fixed constructor bytes. Jim has its own original info commands count plus caught invocation and no C object constructor/type substitute. Complete unchanged request/probe/capture launcher, six actual compile receipts/ELFs, sixty process receipts/stdout/stderr, actual versions and required before/after pins are retained. The five complete tclCmdAH/tclEncoding/tclUtf source sets and six separately identified Native292 compile receipts are independent source/build evidence, not additional executions. No pointer/refcount/table/name/parent lifetime, arbitrary codepage/profile/failindex/streaming behavior, BIG-IP or software assertion result is established. Shared software conversion requires actual Native C authority, its original string getter and an independently authenticated backend binary result constructor. The pure byte codec and structured error presenter do not issue physical storage. Runtime and VM comparator definitions retain all fifty original Native306 C windows and the five separate Native307 C windows, using exact counted constructor inputs and every non-version storage/output/error row. These definitions establish no executed software pass, private pointer/refcount identity or Jim encoding capability; Jim current-image availability remains its own observed channel. Independent exact command source explains the observed result boundary: C8.4 reads the original counted String and writes binary bytes with Tcl_SetByteArrayObj on the interpreter result; C8.5/C8.6 read the original String and publish Tcl_NewByteArrayObj. C9 reads the original String through Tcl_UtfToExternalDStringEx, publishes ByteArray after successful conversion, and takes its separate error return before that constructor when no fail-index receiver is requested. Exact UtfToUtfProc bodies retain their release-selected counted UTF conversion, modified-zero and surrogate handling. Those source explanations do not extend the finite observed cases to arbitrary profiles/streaming/codepages or prove compiled/loaded private identity.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual compiled ELF, original compile metadata and exact archive 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47 / public header 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf with complete required source/build pins.. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

Ten original object vectors retain10 code0 bytearray/no-resident-string results and0 code1 string/resident results before getters. Input and INPUT_AFTER fields retain the original constructor/type/materialisation boundary. Exact zero/eacute/FF/ByteArray-octet and surrogate/astral RAW bytes, or complete MESSAGE/ERROR_CODE, remain independent. Fixed surrogate-pair output is eda0bdedb880; fixed astral input outputs c3b0c29fc298c280. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual compiled ELF, original compile metadata and exact archive 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc / public header c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5 with complete required source/build pins.. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

Ten original object vectors retain10 code0 bytearray/no-resident-string results and0 code1 string/resident results before getters. Input and INPUT_AFTER fields retain the original constructor/type/materialisation boundary. Exact zero/eacute/FF/ByteArray-octet and surrogate/astral RAW bytes, or complete MESSAGE/ERROR_CODE, remain independent. Fixed surrogate-pair output is eda0bdedb880; fixed astral input outputs c3b0c29fc298c280. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual compiled ELF, original compile metadata and exact archive 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb / public header aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245 with complete required source/build pins.. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

Ten original object vectors retain10 code0 bytearray/no-resident-string results and0 code1 string/resident results before getters. Input and INPUT_AFTER fields retain the original constructor/type/materialisation boundary. Exact zero/eacute/FF/ByteArray-octet and surrogate/astral RAW bytes, or complete MESSAGE/ERROR_CODE, remain independent. Fixed pair and astral inputs both output f09f9880; isolated high-surrogate output remains eda0bd. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual compiled ELF, original compile metadata and exact archive dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4 / public header eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a with complete required source/build pins.. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

Ten original object vectors retain8 code0 bytearray/no-resident-string results and2 code1 string/resident results before getters. Input and INPUT_AFTER fields retain the original constructor/type/materialisation boundary. Exact zero/eacute/FF/ByteArray-octet and surrogate/astral RAW bytes, or complete MESSAGE/ERROR_CODE, remain independent. The two fixed surrogate String cases return1 with String/resident result and TCL ENCODING ILLEGALSEQUENCE0; fixed astral input outputs f09f9880. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual compiled ELF, original compile metadata and exact archive 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db / public header 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950 with complete required source/build pins.. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

Ten original object vectors retain8 code0 bytearray/no-resident-string results and2 code1 string/resident results before getters. Input and INPUT_AFTER fields retain the original constructor/type/materialisation boundary. Exact zero/eacute/FF/ByteArray-octet and surrogate/astral RAW bytes, or complete MESSAGE/ERROR_CODE, remain independent. The two fixed surrogate String cases return1 with String/resident result and TCL ENCODING ILLEGALSEQUENCE0; fixed astral input outputs f09f9880. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual compiled ELF, original compile metadata and exact archive a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da / public header d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d with complete required source/build pins.. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

Ten fresh availability/invocation controls each observe actual version0.84-9-g5bac7c9, command count0 and caught code1 invalid command name encoding in this exact fullInit image. The outer AVAILABILITY completion is0; no String/ByteArray constructor, result primary/residency or UTF-8 worker is substituted. This is a bounded current-image answer, not a statement about other Jim builds, user commands or source configurations. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Exact evidence

- `native_encoding_utf8_object306-8.4.20-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20-original-compile.receipt.json). SHA-256 `452a5b3105d74a21fa8a19b63c576a8b9776acb138dc40046f08a2d7fe5bd0cc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-ff/receipt.json). SHA-256 `e5979e4efeab3d51b2124c1bd087e69ef51bd241f09ddde02b1e012a9e01deaf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-ff/stdout). SHA-256 `7e72c1eb8d27779edcc28328a38314765776e40b714a08fe44141542cfa1f2aa`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-utf8-eacute-octets-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-utf8-eacute-octets/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-utf8-eacute-octets/receipt.json). SHA-256 `924b541c395c4ce1189714e998cc8c3a2c745f2c80c576d18456a2e261de2c40`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-utf8-eacute-octets-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-utf8-eacute-octets/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-utf8-eacute-octets/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-utf8-eacute-octets-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-utf8-eacute-octets/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-utf8-eacute-octets/stdout). SHA-256 `177bdc09c3b0540a66f19a456df58ee353b3a58fde3ebe86d10fe75939b3afda`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-zero/receipt.json). SHA-256 `67ed438e1079660764b35b0e46189fe3414eddf52bcc6db48ec4dcb4e8427952`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-byte-array-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/byte-array-zero/stdout). SHA-256 `99cc9401ce0c272aeb414cf2a2d959fc496f71b147f615ee4864ff5d87c5c5d2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/compile.receipt.json). SHA-256 `e2426f037963e8c79a0594fb0c8af89240df284d3de08ee9f40a30537a48a9bb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/probe.elf). SHA-256 `0f024927665e49b91a4425fe01a5933125a842e43da1f469b89e692009d9e0a6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-astral-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-astral/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-astral/receipt.json). SHA-256 `291ae57d54bf4e98b38096862caa730eede4b73beb0badf426a5515d4c5ef0ed`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-astral-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-astral/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-astral/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-astral-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-astral/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-astral/stdout). SHA-256 `6269a719b9d320f0baf33d1bbdda427e0c6b9bd9882c9717448f301e421e19d6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-eacute-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-eacute/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-eacute/receipt.json). SHA-256 `e1a7f24fea4a5775b45f743b874df7c068205070207e0202ae79fba1b3c576b9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-eacute-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-eacute/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-eacute/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-eacute-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-eacute/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-eacute/stdout). SHA-256 `e5d490050e35b65d905a1d55e3a75430a9fe8f051ff4c56c78c8bcbf98316e8f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-ff/receipt.json). SHA-256 `aad35dcfec21c81866a7ba4c5380323b976fe53ac138a02d8b81d5ca9a441e55`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-ff/stdout). SHA-256 `eb8f2558e1d37fa831ac1a5ee2f3351af58a3fd84846383770c6514ff33c4cc0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-literal-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-literal-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-literal-zero/receipt.json). SHA-256 `d7c2ae521098caf7cfbd85e9fe9efa609fe5b6a96245147540cd0997f9337794`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-literal-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-literal-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-literal-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-literal-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-literal-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-literal-zero/stdout). SHA-256 `02e1cd55e2385dff251553f57a8fa0fdb63d466b9282e1e286f6e33f97edeaef`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-modified-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-modified-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-modified-zero/receipt.json). SHA-256 `40aa61afc6c3743617a1edc08b181d88d73ad4a28da5b359fa9f610010250342`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-modified-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-modified-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-modified-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-modified-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-modified-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-modified-zero/stdout). SHA-256 `ac50503c4f364e44d0745aabc5c917efce4dff713dcc25a4bc988b468091614d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-single-high-surrogate/receipt.json). SHA-256 `7987250cd0260664d4d46b8cd902039dd715b247cbf1f3b3ac6ab9fb573301d9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-single-high-surrogate/stdout). SHA-256 `e648409ff17b4952400f5b2243cd5517e755a65e10609952787602f9254c38ba`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-surrogate-pair-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-surrogate-pair/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-surrogate-pair/receipt.json). SHA-256 `74ae97ec16ac889672954a66415390dc13a8eca51b96153fd8183f337c6d9867`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-surrogate-pair-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-surrogate-pair/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-surrogate-pair/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-string-surrogate-pair-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-surrogate-pair/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/string-surrogate-pair/stdout). SHA-256 `4781be8625e2185c7599783871496dfe2eda70ca91c6fa438f666dd79364b54e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/tclCmdAH.c). SHA-256 `4119b635cc3677728baa21df8ee8c683c3f37aa87bf9ca80fcab74eec948fd75`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/tclEncoding.c). SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.4.20-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20/tclUtf.c). SHA-256 `ddef5409a1278d84f479f8b0c9c11dd8d348bfbdcc4db15941d4d7e2e1a2bbcb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19-original-compile.receipt.json). SHA-256 `92b36c7426600ed77ff8d022f07173b8bafc164203032ff4f4437f242269b5eb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-ff/receipt.json). SHA-256 `7d39cc719c9a7d9a7e7eef90f6504c0a7fe48c35303f16b7260e204fb5c10fa0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-ff/stdout). SHA-256 `cdd0c7d7589b9b19473ab306f83069739702d6b2c87fa6e15e31eabaffdcaa27`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-utf8-eacute-octets-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-utf8-eacute-octets/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-utf8-eacute-octets/receipt.json). SHA-256 `e6e7500f4ee56f070089683f2b3f1353c77ac9698e2be09ef4a0b0d7f295beaf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-utf8-eacute-octets-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-utf8-eacute-octets/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-utf8-eacute-octets/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-utf8-eacute-octets-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-utf8-eacute-octets/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-utf8-eacute-octets/stdout). SHA-256 `e7bfce1f6009b9869e0bbaf9146a07b3cf1ecbb5e2c28d87cd23d7b23b6ea4a2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-zero/receipt.json). SHA-256 `6d49606426cbc3cb6d6e17b9a2e2fad4165be1e0729ac5e5be9fefd49620d023`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-byte-array-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/byte-array-zero/stdout). SHA-256 `3a0ab86c58687b44796fc70166e7f5da72215b7593089edadd967132882b1929`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/compile.receipt.json). SHA-256 `4b5ac9f1f243a3d563d237575d5a48a541959fb0dd8988e4c674bcc2a5665b40`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/probe.elf). SHA-256 `96accad0d57eff8e96fe960f06f1bac7632aae06e376288f964d3082574305dc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-astral-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-astral/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-astral/receipt.json). SHA-256 `70e69aaabcca935cceb42e06104ce14890993509bf867c221224e36520b3934a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-astral-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-astral/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-astral/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-astral-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-astral/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-astral/stdout). SHA-256 `ab1be825e339410ff0a1fb8111d639805f2e7fff2a6214fe5f2c6135c0d15252`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-eacute-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-eacute/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-eacute/receipt.json). SHA-256 `f7400bb51f9da8712630e7854e26b9ae17cd9a56f2c099a8bc9653cb4b59084a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-eacute-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-eacute/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-eacute/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-eacute-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-eacute/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-eacute/stdout). SHA-256 `e5b176339c0215e86ff05731cb68cb111cd455c63b678e146b2884d2c37f8b21`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-ff/receipt.json). SHA-256 `bbf27657014a732382eb853b70e711010f7efdf60d64509b75d488170bd96c7a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-ff/stdout). SHA-256 `bb5be57931dada1fc0e87ef5c0326166ef7b0d9257062c8bca0f967063317aa0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-literal-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-literal-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-literal-zero/receipt.json). SHA-256 `6b6bb11bc1907ccc5a14226ddc5861f5b5c38d3c3422ea88c98fc3e962f9b3e7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-literal-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-literal-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-literal-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-literal-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-literal-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-literal-zero/stdout). SHA-256 `ded4ef33b7aa7517226c2b078249b406604f102257df1145493d21dab5587888`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-modified-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-modified-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-modified-zero/receipt.json). SHA-256 `69811797041189077e48e8a98431e63b4f7e224bc933e904616fa81ed61a62a9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-modified-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-modified-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-modified-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-modified-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-modified-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-modified-zero/stdout). SHA-256 `f678eca5bd90cc746e8c02f845ae7ff67b3769549c15cb0c2c7c0e589f3b6449`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-single-high-surrogate/receipt.json). SHA-256 `4afee9b0c200121d17e2156b82c9e137ed7095dcd56cfd1d44fdd9c62b917d89`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-single-high-surrogate/stdout). SHA-256 `c58e7aa60d8104147a2f24919ba6ecfb1c26c6268939fab40afa46785eaf8b06`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-surrogate-pair-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-surrogate-pair/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-surrogate-pair/receipt.json). SHA-256 `018009720f8da5ff9065ac7c0a2f4e41cf9a5ecddfeaeb974c6f652bdfda6eae`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-surrogate-pair-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-surrogate-pair/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-surrogate-pair/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-string-surrogate-pair-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-surrogate-pair/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/string-surrogate-pair/stdout). SHA-256 `2772d54d27a611aebddf84b0b7174ed087e2223afe2c73ec5abf80d9ba3deaad`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/tclCmdAH.c). SHA-256 `cdd328ee0a0d0c3ec7b67da50c90ae22b40591cc52e9492f19d7b257355fcf43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/tclEncoding.c). SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.5.19-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19/tclUtf.c). SHA-256 `3352c62e891cff1d131260183af69bd2c2334e1014bfe2874c25722be871b580`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18-original-compile.receipt.json). SHA-256 `6e9433414d044125e139dfa184aa375aad7f62d1f91c687c918889c221bc50f2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-ff/receipt.json). SHA-256 `fb634036aed06b8328bb6972bb81ccc8030a68fc7d85b568a347850f48468694`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-ff/stdout). SHA-256 `f1b203592cffddca29dd5d394a7c3b97351583cda977ba4b610592bcedad3937`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-utf8-eacute-octets-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-utf8-eacute-octets/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-utf8-eacute-octets/receipt.json). SHA-256 `898a11d090140c3dc6ce84035c62bc76483a2247224252532e0ef5f7c9ac7339`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-utf8-eacute-octets-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-utf8-eacute-octets/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-utf8-eacute-octets/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-utf8-eacute-octets-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-utf8-eacute-octets/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-utf8-eacute-octets/stdout). SHA-256 `b75543e18d4c3e07563117706603aec5dfbf9dd8e8bc5f450596d725e6611bab`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-zero/receipt.json). SHA-256 `6785e0d0c6b8d64f25f95877e2280dd81c69179101eae1b9da1eafc6bb97b72b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-byte-array-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/byte-array-zero/stdout). SHA-256 `5594ee976c21a28713b788ecfd523b006f14d5dc0005431fc36bde02aad38720`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/compile.receipt.json). SHA-256 `8b8941ecfa6fc6a6cd816fdef6c553eb6a449198924193a21322d5fbb4cf32f9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/probe.elf). SHA-256 `40956a323cd6aaf403cff0fd0624ebc7a65c3373f6c746e792d0607d3fe60d2a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-astral-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-astral/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-astral/receipt.json). SHA-256 `1675c4ca0d1c33a0a82edfa8b022d57cd64f47cc8dcffe7b6296fa4214e27209`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-astral-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-astral/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-astral/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-astral-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-astral/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-astral/stdout). SHA-256 `2e049cfa0fa330fe55d226f8ae6121bfe79ee6c67e1c641414d7af432d208740`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-eacute-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-eacute/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-eacute/receipt.json). SHA-256 `07726877af0f433cfca6accdd7f24b5df66cd52f8492496bda2abbc1b92d52c0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-eacute-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-eacute/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-eacute/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-eacute-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-eacute/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-eacute/stdout). SHA-256 `ef17521912ce1580f36e70c7aa335617fccee09a0dfcd92bb0ca470b08a8a56e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-ff/receipt.json). SHA-256 `25ef2f12a12977ffb6085171e1e3b693f5a5a11d01baef0f69eb457d54e2973e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-ff/stdout). SHA-256 `56c6ade9563adb2a6322c2eb4b9d5b03d94bb7b6f2fe336a142502559f8e415a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-literal-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-literal-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-literal-zero/receipt.json). SHA-256 `82580b37a3c2b253cc7f1e4362b78626262f1ab4808e9cdd96dd3232c2281272`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-literal-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-literal-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-literal-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-literal-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-literal-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-literal-zero/stdout). SHA-256 `ae55e491c9c5ffb03578a7bbc4e3daa9801a29eb16e6c3e00789b7865634c6b4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-modified-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-modified-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-modified-zero/receipt.json). SHA-256 `97556d07fe543872678866966e355f39f91f0015c70d1fe9d68dfef90214096f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-modified-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-modified-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-modified-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-modified-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-modified-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-modified-zero/stdout). SHA-256 `6b08bb44c67bc42e83255b3263ff443b8431fb184dcff063f837cbc41f709031`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-single-high-surrogate/receipt.json). SHA-256 `e31c52032381ce3977c1b0f0be7aa265ef30b1b8e123eb3fbf57349ef561d978`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-single-high-surrogate/stdout). SHA-256 `2add5b458f263b5dd860f19072c8eb789aa2601f8a3a7a1244a4f40e4df411c0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-surrogate-pair-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-surrogate-pair/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-surrogate-pair/receipt.json). SHA-256 `d7f5072c3833dc1e3e1cec179b2f4797da073f2a717ba5de59eace303fe02b0b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-surrogate-pair-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-surrogate-pair/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-surrogate-pair/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-string-surrogate-pair-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-surrogate-pair/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/string-surrogate-pair/stdout). SHA-256 `e41a932ce11e38a058fe164baec80073cc980e9ad5c1e9a7f2b4ab85507dc3fb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/tclCmdAH.c). SHA-256 `752473fb5c05d166602ed00b2afff8e123dafd34f4c585628e941b8c5691ae88`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/tclEncoding.c). SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-8.6.18-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18/tclUtf.c). SHA-256 `95d8a272d4c96a4462c5e406d4fef3073b3bd7489691d839615bfaef394ac7f5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4-original-compile.receipt.json). SHA-256 `592f0da1fe061ceb61facfee8eb8a40ba7fe0cd86103f185cc54de51d5cdfff7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-ff/receipt.json). SHA-256 `fdfc3a7c82e7c657b516d8ba2cb614629ed6e8585bb543b7b64fd87d1c940e76`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-ff/stdout). SHA-256 `b8aee9e2e8e7bf9225500d3fc939463571fbefb56825c2472c76ad96d7677176`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-utf8-eacute-octets-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-utf8-eacute-octets/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-utf8-eacute-octets/receipt.json). SHA-256 `281003bc0bb6edb7160a7207cff43e0a8330cd543dc131c81d645d4981f214d3`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-utf8-eacute-octets-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-utf8-eacute-octets/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-utf8-eacute-octets/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-utf8-eacute-octets-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-utf8-eacute-octets/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-utf8-eacute-octets/stdout). SHA-256 `4e56044278ea7cabab2910eca7d557353463ac929974df43a58febf2c60af63e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-zero/receipt.json). SHA-256 `9b29f22847599a8a619f9b8e8e7a82bdeec09ef522d161a251ddf76e94af3bdd`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-byte-array-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/byte-array-zero/stdout). SHA-256 `0c11a1f4c4ebe11ca819ce89e091b2fd171f16c52f72ba1d9cf7921f6905d72e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/compile.receipt.json). SHA-256 `11931d65d679b8f061556923df2ccccbca6c2084c1a0059e4c11b11aeb27bf02`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/probe.elf). SHA-256 `d466e89ba708cfb1cf121ad67762e08c5a72da57cfaa757cc17284cdf8a2308d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-astral-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-astral/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-astral/receipt.json). SHA-256 `271ef85054cedafab13cff334922a20633779ac4bfeadb504a8f35f066711bdc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-astral-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-astral/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-astral/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-astral-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-astral/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-astral/stdout). SHA-256 `e9af05c46b9d6d688af590a99955047d4b4141a44f3e885b0ba6a8f890b74c93`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-eacute-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-eacute/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-eacute/receipt.json). SHA-256 `60f2bbe2888c02dab697d71512a4ba833f947bac0179cb009df612cba79b04ec`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-eacute-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-eacute/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-eacute/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-eacute-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-eacute/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-eacute/stdout). SHA-256 `dc4f40488892fbbc262072aae742a9edabedf6b9322d0a109f5df4d864f05dda`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-ff/receipt.json). SHA-256 `32833a4f5cd759f6d11b98828cec1d6e9b9b7bbb912a31631c8f8cb7567cfc1b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-ff/stdout). SHA-256 `14ab656dbb2ada6808844867aa61c4c5d697f8b339e4dc9b25dc745121e3089e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-literal-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-literal-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-literal-zero/receipt.json). SHA-256 `8eaf9ad247ddae39a2e0c8c1357632a70f1c11c013a2989b1169e6d8e154661c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-literal-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-literal-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-literal-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-literal-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-literal-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-literal-zero/stdout). SHA-256 `15910772f9d2fddfb30652c8ca4e63b9c2d11881b06efd7da96a7b872cafce45`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-modified-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-modified-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-modified-zero/receipt.json). SHA-256 `3e90c0231a23c080af12e3c435a7e92a2a3e03b8a8e0a5bc356be297db879041`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-modified-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-modified-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-modified-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-modified-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-modified-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-modified-zero/stdout). SHA-256 `7070df2089a08a38777696e77d751d9d13c410cce10bb51d6895d47a6eda58ba`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-single-high-surrogate/receipt.json). SHA-256 `c8a126697db7d3c7d7d7131cd58116afeccad1e409e9c2f66148b73c145e5458`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-single-high-surrogate/stdout). SHA-256 `9e2e1e9c96313de8f9a414dfd4b1f6309c6069e61cf0239ffd40552618528dd9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-surrogate-pair-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-surrogate-pair/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-surrogate-pair/receipt.json). SHA-256 `c9d15663f972240648f5567f2d3e3c56637e0d66dd6b0032b334b636b351baad`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-surrogate-pair-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-surrogate-pair/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-surrogate-pair/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-string-surrogate-pair-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-surrogate-pair/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/string-surrogate-pair/stdout). SHA-256 `9373b38c0ca25563a71de710d1ad2daa6830d123e6ea6aff92be8379db795909`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/tclCmdAH.c). SHA-256 `b1fd4aec514b23bbca0a43d99e25a613ec570752e50fadf1caf5c499b1a54ec1`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/tclEncoding.c). SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.0.4-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4/tclUtf.c). SHA-256 `1574311b441c745dac0c1cc4f0e4ca0e7cbc09b86261d463a62d9ee3e317027c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0-original-compile.receipt.json). SHA-256 `914e2aae7f2e087f41d42d956c4e1e5ae0305e2ae638a81c4d828f5c16814a5b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-ff/receipt.json). SHA-256 `e45a40c4b4851c6481690759b38a00204a6ea4e99a9d3f0448f0b084b421b1ca`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-ff/stdout). SHA-256 `5f428d8842aa330298250122e405abae625d41606dc53835c1cddc8bd7de9d3f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-utf8-eacute-octets-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-utf8-eacute-octets/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-utf8-eacute-octets/receipt.json). SHA-256 `3a64745f430d4715c82010c67d7d5e7d8194ed89d0e5d67f2d4ad95f5fc7c173`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-utf8-eacute-octets-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-utf8-eacute-octets/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-utf8-eacute-octets/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-utf8-eacute-octets-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-utf8-eacute-octets/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-utf8-eacute-octets/stdout). SHA-256 `57c551adacff120351eedf28cf3499e9236e51f7e08a725e1f783168d15cda61`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-zero/receipt.json). SHA-256 `155d22bb9f6544c8f97c6bb7ea1bab8c8c9e657c257c6a13548b1c3c17d25431`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-byte-array-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/byte-array-zero/stdout). SHA-256 `c492e04aab61af15ea7b7e348e0dd3b2fff456be3109af310528e8651dbb15ef`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/compile.receipt.json). SHA-256 `10b098dbf3a39205e8edf5148fda08c8a1d763e98ad22289ac02f16dabe03308`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/probe.elf). SHA-256 `6af3c849b249dafa0483fda77ef3882b7ce27cb412262f473fbb30ae0931d6d4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-astral-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-astral/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-astral/receipt.json). SHA-256 `00abc9c276f6fda24d1430a967479688eee3326a93744f83ac4b80f44dda9a94`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-astral-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-astral/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-astral/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-astral-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-astral/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-astral/stdout). SHA-256 `8d84570c0c080c650450d183079b5314f2e09f3b1c8c5bfb4ff9db24e0a46e27`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-eacute-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-eacute/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-eacute/receipt.json). SHA-256 `7caebaa8d0ab05f593127f6565693ccbc723185d9547f9fe848d9b0c6df8c2df`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-eacute-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-eacute/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-eacute/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-eacute-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-eacute/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-eacute/stdout). SHA-256 `45e1f5baad3b2af1791c18ae982bdb1539aea83b138074b24a2e3189a5b741ff`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-ff/receipt.json). SHA-256 `e333eb4f5983d5dced069c1f1a90b080b6c0a8f498ab07a90dd2cdd7c2be12bc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-ff/stdout). SHA-256 `bb1120f47dee7ad287192a279fdb2fd90ddad20cafeb3fc1d89de91a820ce3e4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-literal-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-literal-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-literal-zero/receipt.json). SHA-256 `fda796f201f4bfa2afbfe1d2b6f4790429fe1aac33d0a66032cf9ef28f2390f0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-literal-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-literal-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-literal-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-literal-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-literal-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-literal-zero/stdout). SHA-256 `4e88f651b01fa4e047de9d4216971718b1d5b17d99cec1bec49a333323223c7c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-modified-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-modified-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-modified-zero/receipt.json). SHA-256 `f50db364e769a29e00c9dbfce491d4a3741a4d7c02e429d5c6dfb52097158d9b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-modified-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-modified-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-modified-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-modified-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-modified-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-modified-zero/stdout). SHA-256 `0f9f37fdd46dc1c2c5dd64792548d3eeaab08c93078225be3a18ce81a73c94fc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-single-high-surrogate/receipt.json). SHA-256 `09f34c66077aa9bfdb5472e81e19d65948cee7e5f2b75b9ca5448e1999c5551a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-single-high-surrogate/stdout). SHA-256 `9f4676385ac7cdc702d05bb9fd2aa126d4d9e3a7487475dd4b658be981ed8c03`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-surrogate-pair-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-surrogate-pair/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-surrogate-pair/receipt.json). SHA-256 `0b04e617453830c939e21840ed0595e8864a14e0dc5171b96d8b76236cf14403`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-surrogate-pair-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-surrogate-pair/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-surrogate-pair/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-string-surrogate-pair-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-surrogate-pair/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/string-surrogate-pair/stdout). SHA-256 `b3c3ebee4293a950bd8ed7630dc7b19325fddcc8d874fa0863d81d655e879e20`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/tclCmdAH.c). SHA-256 `56db5fdd8c4bd688cb0beda3b91ad60111adb802fd63c28821055140b582fe04`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/tclEncoding.c). SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-9.1.0-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0/tclUtf.c). SHA-256 `f388adf112e56d3d07bd86b6878e73a1f8c56a2d751f2bb5d860d3464f4db265`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-capture.py` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/capture.py](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/capture.py). SHA-256 `efa40a253c81580d3949ffbc11d0a70793011db7c9762a687774711b11f5c5c9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim-original-compile.receipt.json). SHA-256 `0e6488a3572aa9b5f197343a7024b79f39dde8082887700faca5b6526e9c07ff`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-ff/receipt.json). SHA-256 `4c34c52f9314ac8daea3010a86688cd0586977dc225f7f89f04430be74a63881`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-ff/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-utf8-eacute-octets-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-utf8-eacute-octets/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-utf8-eacute-octets/receipt.json). SHA-256 `44e7f0a436f3500f89c223805e2d72af91b067ab62b15cc789d87bea92fae955`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-utf8-eacute-octets-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-utf8-eacute-octets/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-utf8-eacute-octets/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-utf8-eacute-octets-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-utf8-eacute-octets/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-utf8-eacute-octets/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-zero/receipt.json). SHA-256 `c6611df39d8046d42931be2a40ef07c781ec99864d4983176c990f75b8bae0eb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-byte-array-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/byte-array-zero/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/compile.receipt.json). SHA-256 `feaf4b8b6b02580fa311cfaf5f15ef2ce2bbd3ae13a2214f99d6c595bfeed26d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/probe.elf). SHA-256 `f9b7696c0f8029d21717aba89730453b737586d8001987a27960d9e06b419800`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-astral-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-astral/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-astral/receipt.json). SHA-256 `b2798c276cd653c65542b45bd1133a4166852863e9e93fe295ba40dedf48418f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-astral-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-astral/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-astral/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-astral-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-astral/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-astral/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-eacute-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-eacute/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-eacute/receipt.json). SHA-256 `b1ad01296d1f822318ca43ce29165625585acdcaf18bc4e820a70b08e241ce1c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-eacute-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-eacute/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-eacute/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-eacute-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-eacute/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-eacute/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-ff-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-ff/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-ff/receipt.json). SHA-256 `fce31824d0e6618e41629d0121b77eb321f23a3ae7150f0865b3ea392fa3ec22`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-ff-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-ff/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-ff/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-ff-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-ff/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-ff/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-literal-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-literal-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-literal-zero/receipt.json). SHA-256 `b1f87819f872499c75fc13e501bf479e38b0847972dcf44c0d91c3adc36ad2e0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-literal-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-literal-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-literal-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-literal-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-literal-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-literal-zero/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-modified-zero-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-modified-zero/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-modified-zero/receipt.json). SHA-256 `64b6e1d9988d824eaa69128374c0d6836442c10d375ec6a6dd75e4fcd3622608`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-modified-zero-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-modified-zero/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-modified-zero/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-modified-zero-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-modified-zero/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-modified-zero/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-single-high-surrogate/receipt.json). SHA-256 `1c4d9103b835d32e33400b0a60270bf0f95f7463b2d4f22410c9b4a8e2dc0889`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-single-high-surrogate/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-surrogate-pair-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-surrogate-pair/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-surrogate-pair/receipt.json). SHA-256 `6940a966f0ead97cdfc1dac0a76ebe7308f09969f4be340effcf7edabd4be7ab`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-surrogate-pair-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-surrogate-pair/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-surrogate-pair/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-jim-string-surrogate-pair-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-surrogate-pair/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim/string-surrogate-pair/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-probe.c` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/probe.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/probe.c). SHA-256 `4209e7d447c59412bc4b0b576d3efe890fd08de75ddab991060ccdd7a675a427`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request.json). SHA-256 `fa52426a47920f82b850eb32dde48bc8cdf3ee7dfdbd9816333339804f82c853`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.4.20-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.4.20-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.4.20-original-compile.receipt.json). SHA-256 `452a5b3105d74a21fa8a19b63c576a8b9776acb138dc40046f08a2d7fe5bd0cc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.4.20-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.4.20/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.4.20/tclCmdAH.c). SHA-256 `4119b635cc3677728baa21df8ee8c683c3f37aa87bf9ca80fcab74eec948fd75`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.4.20-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.4.20/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.4.20/tclEncoding.c). SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.4.20-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.4.20/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.4.20/tclUtf.c). SHA-256 `ddef5409a1278d84f479f8b0c9c11dd8d348bfbdcc4db15941d4d7e2e1a2bbcb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.5.19-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.5.19-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.5.19-original-compile.receipt.json). SHA-256 `92b36c7426600ed77ff8d022f07173b8bafc164203032ff4f4437f242269b5eb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.5.19-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.5.19/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.5.19/tclCmdAH.c). SHA-256 `cdd328ee0a0d0c3ec7b67da50c90ae22b40591cc52e9492f19d7b257355fcf43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.5.19-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.5.19/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.5.19/tclEncoding.c). SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.5.19-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.5.19/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.5.19/tclUtf.c). SHA-256 `3352c62e891cff1d131260183af69bd2c2334e1014bfe2874c25722be871b580`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.6.18-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.6.18-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.6.18-original-compile.receipt.json). SHA-256 `6e9433414d044125e139dfa184aa375aad7f62d1f91c687c918889c221bc50f2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.6.18-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.6.18/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.6.18/tclCmdAH.c). SHA-256 `752473fb5c05d166602ed00b2afff8e123dafd34f4c585628e941b8c5691ae88`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.6.18-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.6.18/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.6.18/tclEncoding.c). SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-8.6.18-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.6.18/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/8.6.18/tclUtf.c). SHA-256 `95d8a272d4c96a4462c5e406d4fef3073b3bd7489691d839615bfaef394ac7f5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-9.0.4-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.0.4-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.0.4-original-compile.receipt.json). SHA-256 `592f0da1fe061ceb61facfee8eb8a40ba7fe0cd86103f185cc54de51d5cdfff7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-9.0.4-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.0.4/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.0.4/tclCmdAH.c). SHA-256 `b1fd4aec514b23bbca0a43d99e25a613ec570752e50fadf1caf5c499b1a54ec1`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-9.0.4-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.0.4/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.0.4/tclEncoding.c). SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-9.0.4-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.0.4/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.0.4/tclUtf.c). SHA-256 `1574311b441c745dac0c1cc4f0e4ca0e7cbc09b86261d463a62d9ee3e317027c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-9.1.0-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.1.0-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.1.0-original-compile.receipt.json). SHA-256 `914e2aae7f2e087f41d42d956c4e1e5ae0305e2ae638a81c4d828f5c16814a5b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-9.1.0-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.1.0/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.1.0/tclCmdAH.c). SHA-256 `56db5fdd8c4bd688cb0beda3b91ad60111adb802fd63c28821055140b582fe04`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-9.1.0-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.1.0/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.1.0/tclEncoding.c). SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-9.1.0-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.1.0/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/9.1.0/tclUtf.c). SHA-256 `f388adf112e56d3d07bd86b6878e73a1f8c56a2d751f2bb5d860d3464f4db265`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-jim-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/jim-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/jim-original-compile.receipt.json). SHA-256 `0e6488a3572aa9b5f197343a7024b79f39dde8082887700faca5b6526e9c07ff`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-probe.c` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/probe.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/probe.c). SHA-256 `4209e7d447c59412bc4b0b576d3efe890fd08de75ddab991060ccdd7a675a427`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-request-request.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/request.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/request/request.json). SHA-256 `fa52426a47920f82b850eb32dde48bc8cdf3ee7dfdbd9816333339804f82c853`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_object306-summary.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_object306/summary.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/summary.json). SHA-256 `824939b3948478050e75083138da322a154b2172f72ecb86f47a4ec608911cf1`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

## Source inspection

tcl8.4 8.4.20; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdAH.c`, function `Tcl_EncodingObjCmd`, lines 423–527. Full-source SHA-256 `4119b635cc3677728baa21df8ee8c683c3f37aa87bf9ca80fcab74eec948fd75`; snippet SHA-256 `2199d10f6fa5dbab66299360fc4b966c60dcce589e50384d535bcf3709670489`; retained evidence `native_encoding_utf8_object306-8.4.20-tclCmdAH.c`.

```text
Tcl_EncodingObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int index, length;
    Tcl_Encoding encoding;
    char *string;
    Tcl_DString ds;
    Tcl_Obj *resultPtr;

    static CONST char *optionStrings[] = {
	"convertfrom", "convertto", "names", "system",
	NULL
    };
    enum options {
	ENC_CONVERTFROM, ENC_CONVERTTO, ENC_NAMES, ENC_SYSTEM
    };

    if (objc < 2) {
    	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
        return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], optionStrings, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum options) index) {
	case ENC_CONVERTTO:
	case ENC_CONVERTFROM: {
	    Tcl_Obj *data;
	    if (objc == 3) {
		encoding = Tcl_GetEncoding(interp, NULL);
		data = objv[2];
	    } else if (objc == 4) {
		if (TclGetEncodingFromObj(interp, objv[2], &encoding)
			!= TCL_OK) {
		    return TCL_ERROR;
		}
		data = objv[3];
	    } else {
		Tcl_WrongNumArgs(interp, 2, objv, "?encoding? data");
		return TCL_ERROR;
	    }
	    
	    if ((enum options) index == ENC_CONVERTFROM) {
		/*
		 * Treat the string as binary data.
		 */

		string = (char *) Tcl_GetByteArrayFromObj(data, &length);
		Tcl_ExternalToUtfDString(encoding, string, length, &ds);

		/*
		 * Note that we cannot use Tcl_DStringResult here because
		 * it will truncate the string at the first null byte.
		 */

		Tcl_SetStringObj(Tcl_GetObjResult(interp),
			Tcl_DStringValue(&ds), Tcl_DStringLength(&ds));
		Tcl_DStringFree(&ds);
	    } else {
		/*
		 * Store the result as binary data.
		 */

		string = Tcl_GetStringFromObj(data, &length);
		Tcl_UtfToExternalDString(encoding, string, length, &ds);
		resultPtr = Tcl_GetObjResult(interp);
		Tcl_SetByteArrayObj(resultPtr, 
			(unsigned char *) Tcl_DStringValue(&ds),
			Tcl_DStringLength(&ds));
		Tcl_DStringFree(&ds);
	    }

	    Tcl_FreeEncoding(encoding);
	    break;
	}
	case ENC_NAMES: {
	    if (objc > 2) {
		Tcl_WrongNumArgs(interp, 2, objv, NULL);
		return TCL_ERROR;
	    }
	    Tcl_GetEncodingNames(interp);
	    break;
	}
	case ENC_SYSTEM: {
	    if (objc > 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "?encoding?");
		return TCL_ERROR;
	    }
	    if (objc == 2) {
		Tcl_SetStringObj(Tcl_GetObjResult(interp),
			Tcl_GetEncodingName(NULL), -1);
	    } else {
	        return Tcl_SetSystemEncoding(interp,
			Tcl_GetStringFromObj(objv[2], NULL));
	    }
	    break;
	}
    }
    return TCL_OK;
}

```

tcl8.4 8.4.20; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2008–2105. Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `5e8aafa9ae3c9c2efc9ace8eb5a64079c610933a5192d1ac7abc558db9d07960`; retained evidence `native_encoding_utf8_object306-8.4.20-tclEncoding.c`.

```text
UtfToUtfProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	     srcReadPtr, dstWrotePtr, dstCharsPtr, pureNullMode)
    ClientData clientData;	/* Not used. */
    CONST char *src;		/* Source string in UTF-8. */
    int srcLen;			/* Source string length in bytes. */
    int flags;			/* Conversion control flags. */
    Tcl_EncodingState *statePtr;/* Place for conversion routine to store
				 * state information used during a piecewise
				 * conversion.  Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst;			/* Output buffer in which converted string
				 * is stored. */
    int dstLen;			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr;		/* Filled with the number of bytes from the
				 * source string that were converted.  This
				 * may be less than the original source length
				 * if there was a problem converting some
				 * source characters. */
    int *dstWrotePtr;		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr;		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode;		/* Convert embedded nulls from
				 * internal representation to real
				 * null-bytes or vice versa */

{
    CONST char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;
    Tcl_UniChar ch;

    result = TCL_OK;
    
    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - TCL_UTF_MAX;

    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 &&
	    !(UCHAR(*src) == 0 && pureNullMode == 0)) {
	    /*
	     * Copy 7bit chatacters, but skip null-bytes when we are
	     * in input mode, so that they get converted to 0xc080.
	     */
	    *dst++ = *src++;
	} else if (pureNullMode == 1 &&
		   UCHAR(*src) == 0xc0 &&
		   UCHAR(*(src+1)) == 0x80) {
	    /* 
	     * Convert 0xc080 to real nulls when we are in output mode.
	     */
	    *dst++ = 0;
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /* Always check before using Tcl_UtfToUniChar. Not doing
	     * can so cause it run beyond the endof the buffer!  If we
	     * happen such an incomplete char its bytes are made to
	     * represent themselves.
	     */

	    ch = (unsigned char) *src;
	    src += 1;
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    src += Tcl_UtfToUniChar(src, &ch);
	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    *srcReadPtr  = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.5 8.5.19; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdAH.c`, function `Tcl_EncodingObjCmd`, lines 426–528. Full-source SHA-256 `cdd328ee0a0d0c3ec7b67da50c90ae22b40591cc52e9492f19d7b257355fcf43`; snippet SHA-256 `400fc03cb450abdcd8d861af8f35e3b158847f0b4789698228dc46473a2dca6d`; retained evidence `native_encoding_utf8_object306-8.5.19-tclCmdAH.c`.

```text
Tcl_EncodingObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int index;

    static const char *optionStrings[] = {
	"convertfrom", "convertto", "dirs", "names", "system",
	NULL
    };
    enum options {
	ENC_CONVERTFROM, ENC_CONVERTTO, ENC_DIRS, ENC_NAMES, ENC_SYSTEM
    };

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], optionStrings, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum options) index) {
    case ENC_CONVERTTO:
    case ENC_CONVERTFROM: {
	Tcl_Obj *data;
	Tcl_DString ds;
	Tcl_Encoding encoding;
	int length;
	char *stringPtr;

	if (objc == 3) {
	    encoding = Tcl_GetEncoding(interp, NULL);
	    data = objv[2];
	} else if (objc == 4) {
	    if (Tcl_GetEncodingFromObj(interp, objv[2], &encoding) != TCL_OK) {
		return TCL_ERROR;
	    }
	    data = objv[3];
	} else {
	    Tcl_WrongNumArgs(interp, 2, objv, "?encoding? data");
	    return TCL_ERROR;
	}

	if ((enum options) index == ENC_CONVERTFROM) {
	    /*
	     * Treat the string as binary data.
	     */

	    stringPtr = (char *) Tcl_GetByteArrayFromObj(data, &length);
	    Tcl_ExternalToUtfDString(encoding, stringPtr, length, &ds);

	    /*
	     * Note that we cannot use Tcl_DStringResult here because it will
	     * truncate the string at the first null byte.
	     */

	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    Tcl_DStringValue(&ds), Tcl_DStringLength(&ds)));
	    Tcl_DStringFree(&ds);
	} else {
	    /*
	     * Store the result as binary data.
	     */

	    stringPtr = TclGetStringFromObj(data, &length);
	    Tcl_UtfToExternalDString(encoding, stringPtr, length, &ds);
	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(
		    (unsigned char *) Tcl_DStringValue(&ds),
		    Tcl_DStringLength(&ds)));
	    Tcl_DStringFree(&ds);
	}

	Tcl_FreeEncoding(encoding);
	break;
    }
    case ENC_DIRS:
	return EncodingDirsObjCmd(dummy, interp, objc, objv);
    case ENC_NAMES:
	if (objc > 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	Tcl_GetEncodingNames(interp);
	break;
    case ENC_SYSTEM:
	if (objc > 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?encoding?");
	    return TCL_ERROR;
	}
	if (objc == 2) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    Tcl_GetEncodingName(NULL), -1));
	} else {
	    return Tcl_SetSystemEncoding(interp, TclGetString(objv[2]));
	}
	break;
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2218–2313. Full-source SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`; snippet SHA-256 `4d847f1cbd55c32d64e8636ba3deab5a173cba1e73600711a906b957b9346311`; retained evidence `native_encoding_utf8_object306-8.5.19-tclEncoding.c`.

```text
UtfToUtfProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr,		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode)		/* Convert embedded nulls from internal
				 * representation to real null-bytes or vice
				 * versa. */
{
    const char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;
    Tcl_UniChar ch;

    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - TCL_UTF_MAX;

    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !(UCHAR(*src) == 0 && pureNullMode == 0)) {
	    /*
	     * Copy 7bit chatacters, but skip null-bytes when we are in input
	     * mode, so that they get converted to 0xc080.
	     */

	    *dst++ = *src++;
	} else if (pureNullMode == 1 && UCHAR(*src) == 0xc0 &&
		(src + 1 < srcEnd) && UCHAR(*(src+1)) == 0x80) {
	    /*
	     * Convert 0xc080 to real nulls when we are in output mode.
	     */

	    *dst++ = 0;
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Always check before using Tcl_UtfToUniChar. Not doing can so
	     * cause it run beyond the endof the buffer! If we happen such an
	     * incomplete char its byts are made to represent themselves.
	     */

	    ch = (unsigned char) *src;
	    src += 1;
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    src += Tcl_UtfToUniChar(src, &ch);
	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.6 8.6.18; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdAH.c`, function `EncodingConverttoObjCmd`, lines 712–755. Full-source SHA-256 `752473fb5c05d166602ed00b2afff8e123dafd34f4c585628e941b8c5691ae88`; snippet SHA-256 `494761f60a21317772a04c3a86f16d17566541f8ee6302b29e1e4e0a8a22af3d`; retained evidence `native_encoding_utf8_object306-8.6.18-tclCmdAH.c`.

```text
EncodingConverttoObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *data;		/* String to convert */
    Tcl_DString ds;		/* Buffer to hold the byte array */
    Tcl_Encoding encoding;	/* Encoding to use */
    int length;			/* Length of the string being converted */
    const char *stringPtr;	/* Pointer to the first byte of the string */

    if (objc == 2) {
	encoding = Tcl_GetEncoding(interp, NULL);
	data = objv[1];
    } else if (objc == 3) {
	if (Tcl_GetEncodingFromObj(interp, objv[1], &encoding) != TCL_OK) {
	    return TCL_ERROR;
	}
	data = objv[2];
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?encoding? data");
	return TCL_ERROR;
    }

    /*
     * Convert the string to a byte array in 'ds'
     */

    stringPtr = TclGetStringFromObj(data, &length);
    Tcl_UtfToExternalDString(encoding, stringPtr, length, &ds);
    Tcl_SetObjResult(interp,
		     Tcl_NewByteArrayObj((unsigned char*) Tcl_DStringValue(&ds),
					 Tcl_DStringLength(&ds)));
    Tcl_DStringFree(&ds);

    /*
     * We're done with the encoding
     */

    Tcl_FreeEncoding(encoding);
    return TCL_OK;

}

```

tcl8.6 8.6.18; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2276–2420. Full-source SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; snippet SHA-256 `be626d5c5b9bf340a757216b2274f6014547a1fa09bb3f0c7bc59afa2a1203de`; retained evidence `native_encoding_utf8_object306-8.6.18-tclEncoding.c`.

```text
UtfToUtfProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr,		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode)		/* Convert embedded nulls from internal
				 * representation to real null-bytes or vice
				 * versa. Also combine or separate surrogate pairs */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars, charLimit = INT_MAX;
    Tcl_UniChar *chPtr = (Tcl_UniChar *) statePtr;

    if (flags & TCL_ENCODING_START) {
    	*statePtr = 0;
    }
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= 6;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - ((pureNullMode == 1) ? 4 : TCL_UTF_MAX);

    for (numChars = 0; src < srcEnd && numChars <= charLimit; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !((UCHAR(*src) == 0) && (pureNullMode == 0))) {
	    /*
	     * Copy 7bit characters, but skip null-bytes when we are in input
	     * mode, so that they get converted to 0xC080.
	     */

	    *dst++ = *src++;
	    *chPtr = 0; /* reset surrogate handling */
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd)
		&& (UCHAR(src[1]) == 0x80) && (pureNullMode == 1)) {
	    /*
	     * Convert 0xC080 to real nulls when we are in output mode.
	     */

	    *dst++ = 0;
	    *chPtr = 0; /* reset surrogate handling */
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Always check before using TclUtfToUniChar. Not doing can so
	     * cause it run beyond the end of the buffer! If we happen such an
	     * incomplete char its bytes are made to represent themselves
	     * unless the user has explicitly asked to be told.
	     */

	    if ((flags & TCL_ENCODING_STOPONERROR) && (pureNullMode == 0)) {
		result = TCL_CONVERT_MULTIBYTE;
		break;
	    }
	    *chPtr = UCHAR(*src);
	    src += 1;
	    dst += Tcl_UniCharToUtf(*chPtr, dst);
	} else {
	    size_t len = TclUtfToUniChar(src, chPtr);
	    if ((len < 2) && (*chPtr != 0) && (flags & TCL_ENCODING_STOPONERROR)
		    && ((*chPtr & ~0x7FF) != 0xD800) && (pureNullMode == 0)) {
		result = TCL_CONVERT_SYNTAX;
		break;
	    }
	    src += len;
	    if ((*chPtr & ~0x7FF) == 0xD800) {
		Tcl_UniChar low;
		/* A surrogate character is detected, handle especially */
#if TCL_UTF_MAX <= 4
	    if ((len < 3) && ((src[3 - len] & 0xC0) != 0x80)) {
	    /* It's invalid. See [ed29806ba] */
		*chPtr = UCHAR(src[-1]);
		dst += Tcl_UniCharToUtf(*chPtr, dst);
		continue;
	    }
#endif
		low = *chPtr;
		len = (src <= srcEnd-3) ? Tcl_UtfToUniChar(src, &low) : 0;
		if (((low & ~0x3FF) != 0xDC00) || (*chPtr & 0x400)) {
		    *dst++ = (char) (((*chPtr >> 12) | 0xE0) & 0xEF);
		    *dst++ = (char) (((*chPtr >> 6) | 0x80) & 0xBF);
		    *dst++ = (char) ((*chPtr | 0x80) & 0xBF);
		    *chPtr = 0; /* reset surrogate handling */
		    continue;
		} else if ((TCL_UTF_MAX > 3) || (pureNullMode == 1)) {
		    int full = (((*chPtr & 0x3FF) << 10) | (low & 0x3FF)) + 0x10000;
		    *dst++ = (char) (((full >> 18) | 0xF0) & 0xF7);
		    *dst++ = (char) (((full >> 12) | 0x80) & 0xBF);
		    *dst++ = (char) (((full >> 6) | 0x80) & 0xBF);
		    *dst++ = (char) ((full | 0x80) & 0xBF);
			*chPtr = 0; /* reset surrogate handling */
		    src += len;
		    continue;
		}
	    }
	    dst += Tcl_UniCharToUtf(*chPtr, dst);
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.0 9.0.4; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdAH.c`, function `EncodingConverttoObjCmd`, lines 620–698. Full-source SHA-256 `b1fd4aec514b23bbca0a43d99e25a613ec570752e50fadf1caf5c499b1a54ec1`; snippet SHA-256 `f41b9ad6cfbd2f05efdafb53b9ec3cfe32a237e00f8093390e32b4de0114f49d`; retained evidence `native_encoding_utf8_object306-9.0.4-tclCmdAH.c`.

```text
EncodingConverttoObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *data;		/* String to convert */
    Tcl_DString ds;		/* Buffer to hold the byte array */
    Tcl_Encoding encoding;	/* Encoding to use */
    Tcl_Size length;		/* Length of the string being converted */
    const char *stringPtr;	/* Pointer to the first byte of the string */
    int result;
    int flags;
    Tcl_Obj *failVarObj;
    Tcl_Size errorLocation;

    if (EncodingConvertParseOptions(interp, objc, objv, &encoding, &data,
	    &flags, &failVarObj) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Convert the string to a byte array in 'ds'
     */

    stringPtr = TclGetStringFromObj(data, &length);
    result = Tcl_UtfToExternalDStringEx(interp, encoding, stringPtr, length, flags,
	    &ds, failVarObj ? &errorLocation : NULL);
    /* NOTE: ds must be freed beyond this point even on error */

    switch (result) {
    case TCL_OK:
	errorLocation = TCL_INDEX_NONE;
	break;
    case TCL_ERROR:
	/* Error in parameters. Should not happen. interp will have error */
	goto done;
    default:
	/*
	 * One of the TCL_CONVERT_* errors. If we were not interested in the
	 * error location, interp result would already have been filled in
	 * and we can just return the error. Otherwise, we have to return
	 * what could be decoded and the returned error location.
	 */
	if (failVarObj == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
	break;
    }
    /*
     * TCL_OK or a TCL_CONVERT_* error where the caller wants back as much
     * data as was converted.
     */
    if (failVarObj) {
	Tcl_Obj *failIndex;

	TclNewIndexObj(failIndex, errorLocation);
	if (Tcl_ObjSetVar2(interp, failVarObj, NULL, failIndex,
		TCL_LEAVE_ERR_MSG) == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
    }

    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(
	    (unsigned char*) Tcl_DStringValue(&ds),
	    Tcl_DStringLength(&ds)));

    result = TCL_OK;

done:
    Tcl_DStringFree(&ds);
    if (encoding) {
	Tcl_FreeEncoding(encoding);
    }
    return result;

}

```

tcl9.0 9.0.4; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2441–2743. Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `b277bf5fb0fe4f34527d9240864b55644807871f8e5bf977f4590c388418f265`; retained evidence `native_encoding_utf8_object306-9.0.4-tclEncoding.c`.

```text
UtfToUtfProc(
    void *clientData,		/* additional flags */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* TCL_ENCODING_* conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars, charLimit = INT_MAX;
    int ch;
    int profile;

    if (flags & TCL_ENCODING_START) {
	/* *statePtr will hold high surrogate in a split surrogate pair */
	*statePtr = 0;
    }
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= 6;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    flags |= PTR2INT(clientData);

    /*
     * If output is UTF-8 or encoding for Tcl's internal encoding,
     * max space needed is TCL_UTF_MAX. Otherwise, need 6 bytes (CESU-8)
     */
    dstEnd = dst + dstLen - ((flags & (ENCODING_INPUT|ENCODING_UTF)) ? TCL_UTF_MAX : 6);

    /*
     * Macro to output an isolated high surrogate when it is not followed
     * by a low surrogate. NOT to be called for strict profile since
     * that should raise an error.
     */
#define OUTPUT_ISOLATEDSURROGATE                                \
    do {                                                        \
	Tcl_UniChar high;                                       \
	if (PROFILE_REPLACE(profile)) {                         \
	    high = UNICODE_REPLACE_CHAR;                        \
	} else {                                                \
	    high = (Tcl_UniChar)(ptrdiff_t) *statePtr;          \
	}                                                       \
	assert(!(flags & ENCODING_UTF)); /* Must be CESU-8 */   \
	assert(HIGH_SURROGATE(high));                           \
	assert(!PROFILE_STRICT(profile));                       \
	dst += Tcl_UniCharToUtf(high, dst);                     \
	*statePtr = 0; /* Reset state */                        \
    } while (0)

    /*
     * Macro to check for isolated surrogate and either break with
     * an error if profile is strict, or output an appropriate
     * character for replace and tcl8 profiles and continue.
     */
#define CHECK_ISOLATEDSURROGATE                                         \
    if (*statePtr) {                                                    \
	if (PROFILE_STRICT(profile)) {                                  \
	    result = TCL_CONVERT_SYNTAX;                                \
	    break;                                                      \
	}                                                               \
	OUTPUT_ISOLATEDSURROGATE;                                       \
	continue; /* Rerun loop so length checks etc. repeated */       \
    } else                                                              \
	(void) 0

    profile = ENCODING_PROFILE_GET(flags);
    for (numChars = 0; src < srcEnd && numChars <= charLimit; numChars++) {

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !((UCHAR(*src) == 0) && (flags & ENCODING_INPUT))) {

	    CHECK_ISOLATEDSURROGATE;
	    /*
	     * Copy 7bit characters, but skip null-bytes when we are in input
	     * mode, so that they get converted to \xC0\x80.
	     */
	    *dst++ = *src++;
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd) &&
		 (UCHAR(src[1]) == 0x80) &&
		 (!(flags & ENCODING_INPUT) || !PROFILE_TCL8(profile))) {
	    /* Special sequence \xC0\x80 */

	    CHECK_ISOLATEDSURROGATE;
	    if (!PROFILE_TCL8(profile) && (flags & ENCODING_INPUT)) {
		if (PROFILE_REPLACE(profile)) {
		    dst += Tcl_UniCharToUtf(UNICODE_REPLACE_CHAR, dst);
		    src += 2;
		} else {
		    /* PROFILE_STRICT */
		    result = TCL_CONVERT_SYNTAX;
		    break;
		}
	    } else {
		/*
		 * Convert 0xC080 to real nulls when we are in output mode,
		 * irrespective of the profile.
		 */
		*dst++ = 0;
		src += 2;
	    }

	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Incomplete byte sequence not because there are insufficient
	     * bytes in source buffer (have already checked that above) but
	     * because the UTF-8 sequence is truncated.
	     */

	    CHECK_ISOLATEDSURROGATE;

	    if (flags & ENCODING_INPUT) {
		/* Incomplete bytes for modified UTF-8 target */
		if (PROFILE_STRICT(profile)) {
		    result = (flags & TCL_ENCODING_CHAR_LIMIT)
			    ? TCL_CONVERT_MULTIBYTE
			    : TCL_CONVERT_SYNTAX;
		    break;
		}
	    }
	    if (PROFILE_REPLACE(profile)) {
		ch = UNICODE_REPLACE_CHAR;
		++src;
	    } else {
		/* TCL_ENCODING_PROFILE_TCL8 */
		char chbuf[2];
		chbuf[0] = UCHAR(*src++);
		chbuf[1] = 0;
		TclUtfToUniChar(chbuf, &ch);
	    }
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    /* Have a complete character */
	    size_t len = TclUtfToUniChar(src, &ch);

	    Tcl_UniChar savedSurrogate = (Tcl_UniChar) (ptrdiff_t)*statePtr;
	    *statePtr = 0; /* Reset surrogate */

	    if (flags & ENCODING_INPUT) {
		if (((len < 2) && (ch != 0)) || ((ch > 0xFFFF) && !(flags & ENCODING_UTF))) {
		    if (PROFILE_STRICT(profile)) {
			result = TCL_CONVERT_SYNTAX;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    }
		}
	    }

	    const char *saveSrc = src;
	    src += len;
	    if (!(flags & ENCODING_UTF) && !(flags & ENCODING_INPUT)
		    && (ch > 0x7FF)) {
		assert(savedSurrogate == 0); /* Since this flag combo
						will never set *statePtr */
		if (ch > 0xFFFF) {
		    /* CESU-8 6-byte sequence for chars > U+FFFF */
		    ch -= 0x10000;
		    *dst++ = 0xED;
		    *dst++ = (char) (((ch >> 16) & 0x0F) | 0xA0);
		    *dst++ = (char) (((ch >> 10) & 0x3F) | 0x80);
		    ch = (ch & 0x03FF) | 0xDC00;
		}
		*dst++ = (char)(((ch >> 12) | 0xE0) & 0xEF);
		*dst++ = (char)(((ch >> 6) | 0x80) & 0xBF);
		*dst++ = (char)((ch | 0x80) & 0xBF);
		continue;
	    } else if (SURROGATE(ch)) {
		if ((flags & ENCODING_UTF)) {
		    /* UTF-8, not CESU-8, so surrogates should not appear */
		    if (PROFILE_STRICT(profile)) {
			result = (flags & ENCODING_INPUT)
			    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
			src = saveSrc;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    } else {
			/* PROFILE_TCL8 - output as is */
		    }
		} else {
		    /* CESU-8 */
		    if (LOW_SURROGATE(ch)) {
			if (savedSurrogate) {
			    assert(HIGH_SURROGATE(savedSurrogate));
			    ch = 0x10000 + ((savedSurrogate - 0xd800) << 10) + (ch - 0xdc00);
			} else {
			    /* Isolated low surrogate */
			    if (PROFILE_STRICT(profile)) {
				result = (flags & ENCODING_INPUT)
				    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
				src = saveSrc;
				break;
			    } else if (PROFILE_REPLACE(profile)) {
				ch = UNICODE_REPLACE_CHAR;
			    } else {
				/* Tcl8 profile. Output low surrogate as is */
			    }
			}
		    } else {
			assert(HIGH_SURROGATE(ch));
			/* Save the high surrogate */
			*statePtr = (Tcl_EncodingState) (ptrdiff_t) ch;
			if (savedSurrogate) {
			    assert(HIGH_SURROGATE(savedSurrogate));
			    if (PROFILE_STRICT(profile)) {
				result = (flags & ENCODING_INPUT)
				    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
				src = saveSrc;
				break;
			    } else if (PROFILE_REPLACE(profile)) {
				ch = UNICODE_REPLACE_CHAR;
			    } else {
				/* Output the isolated high surrogate */
				ch = savedSurrogate;
			    }
			} else {
			    /* High surrogate saved in *statePtr. Do not output anything just yet. */
			    --numChars; /* Cancel the increment at end of loop */
			    continue;
			}
		    }
		}
	    } else {
		/* Normal character */
		CHECK_ISOLATEDSURROGATE;
	    }

	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    /* Check if an high surrogate left over */
    if (*statePtr) {
	assert(!(flags & ENCODING_UTF)); /* CESU-8, Not UTF-8 */
	if (!(flags & TCL_ENCODING_END)) {
	    /* More data coming */
	} else {
	    /* No more data coming */
	    if (PROFILE_STRICT(profile)) {
		result = (flags & ENCODING_INPUT)
		    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
	    } else {
		if (PROFILE_REPLACE(profile)) {
		    ch = UNICODE_REPLACE_CHAR;
		} else {
		    ch = (Tcl_UniChar) (ptrdiff_t) *statePtr;
		}
		if (dst < dstEnd) {
		    dst += Tcl_UniCharToUtf(ch, dst);
		    ++numChars;
		} else {
		    /* No room in destination */
		    result = TCL_CONVERT_NOSPACE;
		}
	    }
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.1 9.1.0; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdAH.c`, function `EncodingConverttoObjCmd`, lines 642–719. Full-source SHA-256 `56db5fdd8c4bd688cb0beda3b91ad60111adb802fd63c28821055140b582fe04`; snippet SHA-256 `e664eecc91d73aa779b196b71376378c12474428e1b1ec39a36f7236b640997d`; retained evidence `native_encoding_utf8_object306-9.1.0-tclCmdAH.c`.

```text
EncodingConverttoObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *data;		/* String to convert */
    Tcl_DString ds;		/* Buffer to hold the byte array */
    Tcl_Encoding encoding;	/* Encoding to use */
    Tcl_Size length;		/* Length of the string being converted */
    const char *stringPtr;	/* Pointer to the first byte of the string */
    int result;
    int flags;
    Tcl_Obj *failVarObj;
    Tcl_Size errorLocation;

    if (EncodingConvertParseOptions(interp, objc, objv, &encoding, &data,
	    &flags, &failVarObj) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Convert the string to a byte array in 'ds'
     */

    stringPtr = TclGetStringFromObj(data, &length);
    result = Tcl_UtfToExternalDStringEx(interp, encoding, stringPtr, length, flags,
	    &ds, failVarObj ? &errorLocation : NULL);
    /* NOTE: ds must be freed beyond this point even on error */

    switch (result) {
    case TCL_OK:
	errorLocation = TCL_INDEX_NONE;
	break;
    case TCL_ERROR:
	/* Error in parameters. Should not happen. interp will have error */
	goto done;
    default:
	/*
	 * One of the TCL_CONVERT_* errors. If we were not interested in the
	 * error location, interp result would already have been filled in
	 * and we can just return the error. Otherwise, we have to return
	 * what could be decoded and the returned error location.
	 */
	if (failVarObj == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
	break;
    }
    /*
     * TCL_OK or a TCL_CONVERT_* error where the caller wants back as much
     * data as was converted.
     */
    if (failVarObj) {
	Tcl_Obj *failIndex;

	TclNewIndexObj(failIndex, errorLocation);
	if (Tcl_ObjSetVar2(interp, failVarObj, NULL, failIndex,
		TCL_LEAVE_ERR_MSG) == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
    }

    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(
	    (unsigned char*) Tcl_DStringValue(&ds),
	    Tcl_DStringLength(&ds)));

    result = TCL_OK;

  done:
    Tcl_DStringFree(&ds);
    if (encoding) {
	Tcl_FreeEncoding(encoding);
    }
    return result;
}

```

tcl9.1 9.1.0; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2965–3152. Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `a84fc5bfd1594815a97f0abfc47991c565ba675bd913ce005cb70879ad4889f0`; retained evidence `native_encoding_utf8_object306-9.1.0-tclEncoding.c`.

```text
UtfToUtfProc(
    void *clientData,
    const char *src,
    int srcLen,
    int flags,
    Tcl_EncodingState *statePtr,
    char *dst,
    int dstLen,
    int *srcReadPtr,
    int *dstWrotePtr,
    int *dstCharsPtr)
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars = 0, charLimit = INT_MAX;
    int ch;
    int profile;

    *statePtr = 0;
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    flags |= PTR2INT(clientData);

    dstEnd = dst + dstLen - TCL_UTF_MAX;

    /* Checks if a source byte can be copied directly to destination */
#define BYTE_COPYABLE(byte_) \
    (UCHAR(byte_) < 0x80 && ((UCHAR(byte_) != 0)))

    profile = ENCODING_PROFILE_GET(flags);

    for (numChars = 0; src < srcEnd && numChars < charLimit; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}

	if (BYTE_COPYABLE(*src)) {
	    /*
	     * Common case fast path for non-nul ASCII bytes.
	     *
	     * Because we resolved any pending surrogate above (before the
	     * loop), *statePtr is guaranteed to be 0 here, so no
	     * CHECK_ISOLATEDSURROGATE() call is needed.
	     */
	    const char *p = src;
	    ptrdiff_t copyCount = srcEnd - src;
	    if (charLimit != INT_MAX) {
		if (copyCount > (charLimit - numChars)) {
		    copyCount = charLimit - numChars;
		}
	    }
	    if (copyCount > (dstEnd - dst + 1)) {
		copyCount = dstEnd - dst + 1;
	    }
	    const char *srcStop = src + copyCount;
	    while (p < srcStop && BYTE_COPYABLE(*p)) {
		*dst++ = *p++;
	    }
	    numChars += (int)(p - src) - 1;
	    src = p;
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd) &&
		(UCHAR(src[1]) == 0x80) &&
		(!(flags & ENCODING_INPUT) || !PROFILE_TCL8(profile))) {
	    /* Special sequence \xC0\x80 */
	    if (!PROFILE_TCL8(profile) && (flags & ENCODING_INPUT)) {
		if (PROFILE_REPLACE(profile)) {
		    dst += Tcl_UniCharToUtf(UNICODE_REPLACE_CHAR, dst);
		    src += 2;
		} else {
		    /* PROFILE_STRICT */
		    result = TCL_CONVERT_SYNTAX;
		    break;
		}
	    } else {
		*dst++ = 0;
		src += 2;
	    }
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Incomplete byte sequence (truncated UTF-8, not just end of
	     * source buffer — that is caught by the srcClose check above).
	     */
	    if (flags & ENCODING_INPUT) {
		if (PROFILE_STRICT(profile)) {
		    result = (flags & TCL_ENCODING_CHAR_LIMIT)
			    ? TCL_CONVERT_MULTIBYTE
			    : TCL_CONVERT_SYNTAX;
		    break;
		}
	    }
	    if (PROFILE_REPLACE(profile)) {
		ch = UNICODE_REPLACE_CHAR;
		++src;
	    } else {
		/* TCL_ENCODING_PROFILE_TCL8 */
		char chbuf[2];
		chbuf[0] = UCHAR(*src++);
		chbuf[1] = 0;
		TclUtfToUniChar(chbuf, &ch);
	    }
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    /* Have a complete character */
	    size_t len = TclUtfToUniChar(src, &ch);

	    /*
	     * For invalid inputs, Tcl_UtfToUniChar will return the byte
	     * itself and len == 1. We know this is invalid because valid
	     * single byte ASCII is already handled above except for 0.
	     * For the output case, Tcl should not have invalid byte sequences
	     * internally but if it does, garbage in, garbage out. This is
	     * historical behavior that should be changed imho. See
	     * ticket [b69e00ecf6]. TODO.
	     */
	    if (flags & ENCODING_INPUT) {
		if ((len < 2) && (ch != 0)) {
		    if (PROFILE_STRICT(profile)) {
			result = TCL_CONVERT_SYNTAX;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    }
		}
	    }

	    const char *saveSrc = src;
	    src += len;

	    if (SURROGATE(ch)) {
		if (PROFILE_STRICT(profile)) {
		    result = (flags & ENCODING_INPUT)
			    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
		    src = saveSrc;
		    break;
		} else if (PROFILE_REPLACE(profile)) {
		    ch = UNICODE_REPLACE_CHAR;
		}
		/* PROFILE_TCL8: fall through and output as-is */
	    }
	    /* Normal character (or surrogate resolved to replacement/as-is) */
	    assert(ch >= 0 && ch <= 0x10FFFF);
	    if (ch == 0) {
		if (flags & ENCODING_INPUT) {
		    *dst++ = (char)0xC0;
		    *dst++ = (char)0x80;
		} else {
		    *dst++ = 0;
		}
	    } else if ((unsigned)ch < 0x800) {
		assert(ch >= 0x80);
		*dst++ = (char)(0xC0 | (ch >> 6));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    } else if ((unsigned)ch < 0x10000) {
		*dst++ = (char)(0xE0 | (ch >> 12));
		*dst++ = (char)(0x80 | ((ch >> 6) & 0x3F));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    } else {
		*dst++ = (char)(0xF0 | (ch >> 18));
		*dst++ = (char)(0x80 | ((ch >> 12) & 0x3F));
		*dst++ = (char)(0x80 | ((ch >> 6)  & 0x3F));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    }
	}
    }

    *srcReadPtr  = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;

#undef BYTE_COPYABLE
}

```


## Consumer bindings

- [rust/tcl-cmd-core/src/encoding.rs](../../../../rust/tcl-cmd-core/src/encoding.rs), `external_utf8_octets`: Own release-selected external UTF-8 byte transformation and structured error coordinates independently of any physical result storage.
- [rust/tcl-cmd-core/src/encoding.rs](../../../../rust/tcl-cmd-core/src/encoding.rs), `convert_to_utf8`: Require actual Native C name authority and original string getter before requesting independently authenticated external binary result storage.
- [rust/tcl-cmd-core/src/encoding.rs](../../../../rust/tcl-cmd-core/src/encoding.rs), `Utf8EncodingError`: Keep the diagnostic character index, original source-byte offset and rejected decoded unit as distinct fields.
- [rust/tcl-syntax/src/value.rs](../../../../rust/tcl-syntax/src/value.rs), `ValueOps::native_external_utf8_result`: Declare fallible external UTF-8 binary result construction; unavailable or foreign issuers supply no result storage.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `native_external_utf8_result`: Authenticate actual Runtime C release/result owner before constructing external binary bytes; chosen version or name recipe alone supplies no object authority.
- [rust/tcl-vm/src/value_ops.rs](../../../../rust/tcl-vm/src/value_ops.rs), `native_external_utf8_result`: Authenticate the VM actual C issuer independently before constructing the binary result; Jim and foreign owner cannot borrow it.
- [rust/tcl-test-support/src/encoding_utf8.rs](../../../../rust/tcl-test-support/src/encoding_utf8.rs), `CONTROLS`: Retain complete immutable original constructor/output columns shared by codec and both backend comparators, keeping constructor kind independent from observed result primary.
- [rust/tcl-cmd-core/src/encoding/native_controls.rs](../../../../rust/tcl-cmd-core/src/encoding/native_controls.rs), `encoding::native_controls::selected_utf8_units_match_all_55_original_c_conversion_windows` (linked): The pure selected C codec compares converted bytes or complete structured message/errorCode against fifty Native306 and five independent Native307 C columns. This tests byte/error presentation only; it owns no physical result header.
- [runtime/rust/src/cmd_misc/native_encoding_tests.rs](../../../../runtime/rust/src/cmd_misc/native_encoding_tests.rs), `cmd_misc::native_encoding_tests::original_utf8_conversion_matches_all_55_c_object_storage_windows` (linked): Authentic actual C original counted String/ByteArray object vectors compare all non-version INPUT, RETURN, INPUT_AFTER and RAW or MESSAGE/ERROR_CODE rows against the complete original fifty storage and five offset columns. Primary/residency is observed before getters; host refusal is excluded. No native pointer/refcount identity or Jim worker is inferred.
- [rust/tcl-vm/src/command/native_encoding_tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_tests.rs), `command::native_encoding_tests::original_utf8_conversion_matches_all_55_c_object_storage_windows` (linked): The actual VM selected C environment executes original counted constructors and object vectors, then compares every non-version input/result storage and output/error field against the complete fifty storage and five offset columns. Result getters follow the independent primary/residency observation; no Native header identity or Jim worker is granted.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
