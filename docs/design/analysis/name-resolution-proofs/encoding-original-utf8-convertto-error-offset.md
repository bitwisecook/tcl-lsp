# naming.encoding.original-utf8-convertto-error-offset

Kind: `native-observation`

## Problem statement

A UTF-8 conversion error has two coordinates. After the fixed multibyte U+00E9 prefix, copying the byte offset into the message character index or the character index into errorCode gives different data.

## Question

For the original counted String C3A9EDA0BD, what completion, result storage, diagnostic character index and errorCode byte offset does encoding convertto utf-8 produce in each pinned C release, and what independent current-image availability answer does Jim supply?

## Conclusion

All six fresh host processes exit0 with empty stderr. C8.4, C8.5 and C8.6 return0 with bytearray primary and no resident string before the getter, then expose the five output bytes C3A9EDA0BD. C9.0 and C9.1 return1 with string primary and resident bytes; their message says unexpected character at index1 U+00D83D while the complete errorCode is TCL ENCODING ILLEGALSEQUENCE2. The retained String input has none primary and resident bytes before and after each C call. The exact initialized Jim image supplies command count0 plus caught code1 invalid command name encoding, with outer completion0; it supplies no C object-storage or error-coordinate observation. This one fixed prefix distinguishes the two C9 coordinates without generalizing to arbitrary inputs or encodings.

## Scope

One original counted Tcl_NewStringObj operand C3A9EDA0BD is invoked through the unchanged four-word object vector and Tcl_EvalObjv/TCL_EVAL_GLOBAL in each fresh C process. INPUT and RETURN/INPUT_AFTER storage observations precede independent RAW or MESSAGE/ERROR_CODE getters. Jim executes its own unchanged current command-count and caught invocation control, with no C input constructor substitute. Exact request/probe/capture launcher, six actual compile receipts/ELFs, six whole process receipts/stdout/stderr, actual versions and required before/after pins are retained. Five whole tclCmdAH/tclEncoding/tclUtf source sets and six independently identified Native292 compile receipts remain source/build evidence. Native306 ten-input windows are a separate question. No arbitrary offset, other encoding/profile, private object identity, Jim worker or software assertion result follows. Shared software conversion requires actual Native C authority, its original string getter and an independently authenticated backend binary result constructor. The pure byte codec and structured error presenter do not issue physical storage. Runtime and VM comparator definitions retain all fifty original Native306 C windows and the five separate Native307 C windows, using exact counted constructor inputs and every non-version storage/output/error row. These definitions establish no executed software pass, private pointer/refcount identity or Jim encoding capability; Jim current-image availability remains its own observed channel. Independent exact C9 Tcl_UtfToExternalDStringEx source forms the message character position with Tcl_NumUtfChars over the processed prefix, while the errorCode decimal field comes from nBytesProcessed. The reached formatter source explains why the fixed observed U+00E9 prefix produces character index1 and byte offset2. Source inspection itself is not another native execution, a generic encoding-profile result or a backend pass.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual compiled ELF, original compile metadata and exact archive 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47 / public header 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf with complete required source/build pins.. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

The one original counted String C3A9EDA0BD returns0 with bytearray primary and no resident string before the binary getter. RAW retains length5 and bytes C3A9EDA0BD. Original input none/resident storage remains unchanged. No error coordinate is produced by this successful call. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual compiled ELF, original compile metadata and exact archive 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc / public header c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5 with complete required source/build pins.. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

The one original counted String C3A9EDA0BD returns0 with bytearray primary and no resident string before the binary getter. RAW retains length5 and bytes C3A9EDA0BD. Original input none/resident storage remains unchanged. No error coordinate is produced by this successful call. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual compiled ELF, original compile metadata and exact archive 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb / public header aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245 with complete required source/build pins.. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

The one original counted String C3A9EDA0BD returns0 with bytearray primary and no resident string before the binary getter. RAW retains length5 and bytes C3A9EDA0BD. Original input none/resident storage remains unchanged. No error coordinate is produced by this successful call. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual compiled ELF, original compile metadata and exact archive dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4 / public header eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a with complete required source/build pins.. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

The one original counted String C3A9EDA0BD returns1 with string primary and resident bytes before getters. The complete message identifies character index1 U+00D83D; the complete errorCode is TCL ENCODING ILLEGALSEQUENCE2. The original input remains none/resident. This fixed multibyte prefix distinguishes the character index from the byte offset. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual compiled ELF, original compile metadata and exact archive 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db / public header 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950 with complete required source/build pins.. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

The one original counted String C3A9EDA0BD returns1 with string primary and resident bytes before getters. The complete message identifies character index1 U+00D83D; the complete errorCode is TCL ENCODING ILLEGALSEQUENCE2. The original input remains none/resident. This fixed multibyte prefix distinguishes the character index from the byte offset. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual compiled ELF, original compile metadata and exact archive a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da / public header d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d with complete required source/build pins.. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only.. Dialect: Pinned original provider API/source purpose.

The fresh original availability/invocation control records actual version0.84-9-g5bac7c9, command count0 and caught code1 invalid command name encoding in this exact fullInit image. Its outer completion is0; it creates no C operand and measures no result primary, UTF-8 worker or error coordinates. This bounded image-specific answer does not establish absence in other Jim images or user command stores. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Exact evidence

- `native_encoding_utf8_offset307-8.4.20-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20-original-compile.receipt.json). SHA-256 `452a5b3105d74a21fa8a19b63c576a8b9776acb138dc40046f08a2d7fe5bd0cc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/compile.receipt.json). SHA-256 `de9d46a4e3e06ffdced3839f320bcdda95238c88de217bc817a0afc17bed674d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/probe.elf). SHA-256 `c40f0b58c2801bf4c00ddfb996ceb1a1242788e6b6b5d511bba31f99c0ba14a5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-string-eacute-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate/receipt.json). SHA-256 `26038a3d0409f755c5979719ea4642ef6e0aac01415efc2a7040ed1b0fd634c5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-string-eacute-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-string-eacute-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate/stdout). SHA-256 `ef78ee15df67f896a1d523a3744522f1a5be612b6c0ceabd173f88bd98f81e93`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/tclCmdAH.c). SHA-256 `4119b635cc3677728baa21df8ee8c683c3f37aa87bf9ca80fcab74eec948fd75`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/tclEncoding.c). SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.4.20-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/tclUtf.c). SHA-256 `ddef5409a1278d84f479f8b0c9c11dd8d348bfbdcc4db15941d4d7e2e1a2bbcb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19-original-compile.receipt.json). SHA-256 `92b36c7426600ed77ff8d022f07173b8bafc164203032ff4f4437f242269b5eb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/compile.receipt.json). SHA-256 `51ef8152f002397850beb420774bdd8b189d68bbbea9248cd20c52f667c98a3f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/probe.elf). SHA-256 `9125ace6c14af8fa78fd174c234105338997fcb705c091a4f3d1637a848430dc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-string-eacute-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate/receipt.json). SHA-256 `6b06ade5336dde447ce9723cd2335e841893eb9d8698a36bd17dd3cb685f2990`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-string-eacute-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-string-eacute-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate/stdout). SHA-256 `5648e3f4adb815024c1231e7d327b3856bb79d80652ea222850a913872be7212`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/tclCmdAH.c). SHA-256 `cdd328ee0a0d0c3ec7b67da50c90ae22b40591cc52e9492f19d7b257355fcf43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/tclEncoding.c). SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.5.19-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/tclUtf.c). SHA-256 `3352c62e891cff1d131260183af69bd2c2334e1014bfe2874c25722be871b580`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18-original-compile.receipt.json). SHA-256 `6e9433414d044125e139dfa184aa375aad7f62d1f91c687c918889c221bc50f2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/compile.receipt.json). SHA-256 `3922809dd8f8a369eb8a59d8dc5ea9665945aa69a8550a6281fa358933b4174a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/probe.elf). SHA-256 `f0581433de94329c3665d2575250c384b75b99aec2391c23706efb8916f2ee64`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-string-eacute-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate/receipt.json). SHA-256 `aacbd817d18c8d40d30844be83de2a20bc8b515e21422e1db144793f2b9f4086`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-string-eacute-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-string-eacute-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate/stdout). SHA-256 `140bb0ca966017b80eb8b4872b79fef778f5dcc1eceb59e45741eddcfc94fbaf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/tclCmdAH.c). SHA-256 `752473fb5c05d166602ed00b2afff8e123dafd34f4c585628e941b8c5691ae88`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/tclEncoding.c). SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-8.6.18-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/tclUtf.c). SHA-256 `95d8a272d4c96a4462c5e406d4fef3073b3bd7489691d839615bfaef394ac7f5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4-original-compile.receipt.json). SHA-256 `592f0da1fe061ceb61facfee8eb8a40ba7fe0cd86103f185cc54de51d5cdfff7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/compile.receipt.json). SHA-256 `c014489ed3c14afe4296f364eb7d2f54dc73c985f9258d4b238e8ce9af086f01`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/probe.elf). SHA-256 `4efcd7faf270fb9d0b1afb1271044457a7a54f7bb9555c46ce6cc251688b4cc9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-string-eacute-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate/receipt.json). SHA-256 `bfbf17070c6ddc731d3961da8b47864def7b2d776c487a176543f448efb6c914`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-string-eacute-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-string-eacute-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate/stdout). SHA-256 `8f0ab1ef69f2470f8d9bc7b98f12c04b3873ffa87844d4116f7ddc35df3f0836`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/tclCmdAH.c). SHA-256 `b1fd4aec514b23bbca0a43d99e25a613ec570752e50fadf1caf5c499b1a54ec1`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/tclEncoding.c). SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.0.4-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/tclUtf.c). SHA-256 `1574311b441c745dac0c1cc4f0e4ca0e7cbc09b86261d463a62d9ee3e317027c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0-original-compile.receipt.json). SHA-256 `914e2aae7f2e087f41d42d956c4e1e5ae0305e2ae638a81c4d828f5c16814a5b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/compile.receipt.json). SHA-256 `c0c33c27175e90ab6bb14129117c7678e663f61a38dfcbece05a6fde2783860b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/probe.elf). SHA-256 `1a5c68a6bae6bd3a93f6226448a09914b07dfb0252682a28e43b87ffcd501137`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-string-eacute-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate/receipt.json). SHA-256 `682fcd9bec7056638cdd5c718e484b57ac52fce114dea583072938f5902b00bc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-string-eacute-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-string-eacute-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate/stdout). SHA-256 `b859264a6eb2ccae4834f4bcf472abd7743a65a54dd7a7658c20990a110bcd42`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/tclCmdAH.c). SHA-256 `56db5fdd8c4bd688cb0beda3b91ad60111adb802fd63c28821055140b582fe04`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/tclEncoding.c). SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-9.1.0-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/tclUtf.c). SHA-256 `f388adf112e56d3d07bd86b6878e73a1f8c56a2d751f2bb5d860d3464f4db265`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-capture.py` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/capture.py](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/capture.py). SHA-256 `733bd275eb9fd20e863ff478b4dbfae51c7f4101e46dcd53840aeafb6a68d2b6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-jim-original-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim-original-compile.receipt.json). SHA-256 `0e6488a3572aa9b5f197343a7024b79f39dde8082887700faca5b6526e9c07ff`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/compile.receipt.json). SHA-256 `dd14faf051db4ef230b6aa6b0a331e784a8cc05f5b13ce583a449fd7859f575c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/probe.elf). SHA-256 `f6227c055e0214327c43d9c81f960e497a4f0b80e0081bb05e06d306511a40fe`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-jim-string-eacute-single-high-surrogate-receipt.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/string-eacute-single-high-surrogate/receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/string-eacute-single-high-surrogate/receipt.json). SHA-256 `d973c03a81b2f2922b171dd23158060c366f0f8d6131138d6188801bb8137a15`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-jim-string-eacute-single-high-surrogate-stderr` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/string-eacute-single-high-surrogate/stderr](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/string-eacute-single-high-surrogate/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-jim-string-eacute-single-high-surrogate-stdout` (observation): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/string-eacute-single-high-surrogate/stdout](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/string-eacute-single-high-surrogate/stdout). SHA-256 `e2203213036a1c3f2d9e5f74348e24efbdd7f4ed4cc390992bf92c6d2d10af43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-probe.c` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/probe.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/probe.c). SHA-256 `9c31cd70428ddfe871d8f1f28ee11a422bae6beb0ecbf6dec5ee2b40494d807b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request.json). SHA-256 `e09732a749b29eae6897bcd6c87433f0844818e367b5d291dd27c6303aaea08a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.4.20-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.4.20-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.4.20-original-compile.receipt.json). SHA-256 `452a5b3105d74a21fa8a19b63c576a8b9776acb138dc40046f08a2d7fe5bd0cc`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.4.20-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.4.20/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.4.20/tclCmdAH.c). SHA-256 `4119b635cc3677728baa21df8ee8c683c3f37aa87bf9ca80fcab74eec948fd75`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.4.20-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.4.20/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.4.20/tclEncoding.c). SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.4.20-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.4.20/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.4.20/tclUtf.c). SHA-256 `ddef5409a1278d84f479f8b0c9c11dd8d348bfbdcc4db15941d4d7e2e1a2bbcb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.5.19-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.5.19-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.5.19-original-compile.receipt.json). SHA-256 `92b36c7426600ed77ff8d022f07173b8bafc164203032ff4f4437f242269b5eb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.5.19-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.5.19/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.5.19/tclCmdAH.c). SHA-256 `cdd328ee0a0d0c3ec7b67da50c90ae22b40591cc52e9492f19d7b257355fcf43`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.5.19-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.5.19/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.5.19/tclEncoding.c). SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.5.19-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.5.19/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.5.19/tclUtf.c). SHA-256 `3352c62e891cff1d131260183af69bd2c2334e1014bfe2874c25722be871b580`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.6.18-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.6.18-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.6.18-original-compile.receipt.json). SHA-256 `6e9433414d044125e139dfa184aa375aad7f62d1f91c687c918889c221bc50f2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.6.18-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.6.18/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.6.18/tclCmdAH.c). SHA-256 `752473fb5c05d166602ed00b2afff8e123dafd34f4c585628e941b8c5691ae88`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.6.18-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.6.18/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.6.18/tclEncoding.c). SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-8.6.18-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.6.18/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/8.6.18/tclUtf.c). SHA-256 `95d8a272d4c96a4462c5e406d4fef3073b3bd7489691d839615bfaef394ac7f5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-9.0.4-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.0.4-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.0.4-original-compile.receipt.json). SHA-256 `592f0da1fe061ceb61facfee8eb8a40ba7fe0cd86103f185cc54de51d5cdfff7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-9.0.4-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.0.4/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.0.4/tclCmdAH.c). SHA-256 `b1fd4aec514b23bbca0a43d99e25a613ec570752e50fadf1caf5c499b1a54ec1`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-9.0.4-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.0.4/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.0.4/tclEncoding.c). SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-9.0.4-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.0.4/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.0.4/tclUtf.c). SHA-256 `1574311b441c745dac0c1cc4f0e4ca0e7cbc09b86261d463a62d9ee3e317027c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-9.1.0-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.1.0-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.1.0-original-compile.receipt.json). SHA-256 `914e2aae7f2e087f41d42d956c4e1e5ae0305e2ae638a81c4d828f5c16814a5b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-9.1.0-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.1.0/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.1.0/tclCmdAH.c). SHA-256 `56db5fdd8c4bd688cb0beda3b91ad60111adb802fd63c28821055140b582fe04`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-9.1.0-tclEncoding.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.1.0/tclEncoding.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.1.0/tclEncoding.c). SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-9.1.0-tclUtf.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.1.0/tclUtf.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/9.1.0/tclUtf.c). SHA-256 `f388adf112e56d3d07bd86b6878e73a1f8c56a2d751f2bb5d860d3464f4db265`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-jim-original-compile.receipt.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/jim-original-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/jim-original-compile.receipt.json). SHA-256 `0e6488a3572aa9b5f197343a7024b79f39dde8082887700faca5b6526e9c07ff`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-probe.c` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/probe.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/probe.c). SHA-256 `9c31cd70428ddfe871d8f1f28ee11a422bae6beb0ecbf6dec5ee2b40494d807b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-request-request.json` (input): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/request.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/request/request.json). SHA-256 `e09732a749b29eae6897bcd6c87433f0844818e367b5d291dd27c6303aaea08a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_encoding_utf8_offset307-summary.json` (provider): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/summary.json](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/summary.json). SHA-256 `eba0dcf7a61848d2ea7d483590b547f8f55c4e5d1ae0f2033ceaa3b60a74405f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

## Source inspection

tcl9.0 9.0.4; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Tcl_UtfToExternalDStringEx`, lines 1495–1613. Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `7e64155927baff592f6a483955a442c01acf66b5e0500082670330889f5e4526`; retained evidence `native_encoding_utf8_offset307-9.0.4-tclEncoding.c`.

```text
Tcl_UtfToExternalDStringEx(
    Tcl_Interp *interp,		/* For error messages. May be NULL. */
    Tcl_Encoding encoding,	/* The encoding for the converted string, or
				 * NULL for the default system encoding. */
    const char *src,		/* Source string in UTF-8. */
    Tcl_Size srcLen,		/* Source string length in bytes, or < 0 for
				 * strlen(). */
    int flags,			/* Conversion control flags. */
    Tcl_DString *dstPtr,	/* Uninitialized or free DString in which the
				 * converted string is stored. */
    Tcl_Size *errorLocPtr)	/* Where to store the error location
				 * (or TCL_INDEX_NONE if no error). May
				 * be NULL. */
{
    char *dst;
    Tcl_EncodingState state;
    const Encoding *encodingPtr;
    int result;
    const char *srcStart = src;
    Tcl_Size dstLen, soFar;

    /* DO FIRST - must always be initialized on return */
    Tcl_DStringInit(dstPtr);

    dst = Tcl_DStringValue(dstPtr);
    dstLen = dstPtr->spaceAvl - 1;

    if (encoding == NULL) {
	encoding = systemEncoding;
    }
    encodingPtr = (Encoding *) encoding;

    if (src == NULL) {
	srcLen = 0;
	src = "";
    } else if (srcLen < 0) {
	srcLen = strlen(src);
    }

    flags &= ~TCL_ENCODING_END;
    flags |= TCL_ENCODING_START;
    while (1) {
	int srcChunkLen, srcChunkRead;
	int dstChunkLen, dstChunkWrote, dstChunkChars;

	if (srcLen > INT_MAX) {
	    srcChunkLen = INT_MAX;
	} else {
	    srcChunkLen = srcLen;
	    flags |= TCL_ENCODING_END; /* Last chunk */
	}
	dstChunkLen = dstLen > INT_MAX ? INT_MAX : dstLen;

	result = encodingPtr->fromUtfProc(encodingPtr->clientData, src,
		srcChunkLen, flags, &state, dst, dstChunkLen,
		&srcChunkRead, &dstChunkWrote, &dstChunkChars);
	soFar = dst + dstChunkWrote - Tcl_DStringValue(dstPtr);

	/* Move past the part processed in this go around */
	src += srcChunkRead;

	/*
	 * Keep looping in two case -
	 *   - our destination buffer did not have enough room
	 *   - we had not passed in all the data and error indicated fragment
	 *     of a multibyte character
	 * In both cases we have to grow buffer, move the input source pointer
	 * and loop. Otherwise, return the result we got.
	 */
	if ((result != TCL_CONVERT_NOSPACE) &&
		(result != TCL_CONVERT_MULTIBYTE || (flags & TCL_ENCODING_END))) {
	    Tcl_Size nBytesProcessed = (src - srcStart);
	    Tcl_Size i = soFar + encodingPtr->nullSize - 1;
	    /* Loop as DStringSetLength only stores one nul byte at a time */
	    while (i >= soFar) {
		Tcl_DStringSetLength(dstPtr, i--);
	    }
	    if (errorLocPtr) {
		/*
		 * Do not write error message into interpreter if caller
		 * wants to know error location.
		 */
		*errorLocPtr = result == TCL_OK
			? TCL_INDEX_NONE : nBytesProcessed;
	    } else {
		/* Caller wants error message on failure */
		if (result != TCL_OK && interp != NULL) {
		    Tcl_Size pos = Tcl_NumUtfChars(srcStart, nBytesProcessed);
		    int ucs4;
		    char buf[TCL_INTEGER_SPACE];

		    TclUtfToUniChar(&srcStart[nBytesProcessed], &ucs4);
		    snprintf(buf, sizeof(buf), "%" TCL_SIZE_MODIFIER "d",
			    nBytesProcessed);
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "unexpected character at index %" TCL_SIZE_MODIFIER
			    "u: 'U+%06X'",
			    pos, ucs4));
		    Tcl_SetErrorCode(interp, "TCL", "ENCODING", "ILLEGALSEQUENCE",
			    buf, (char *)NULL);
		}
	    }
	    if (result != TCL_OK) {
		errno = (result == TCL_CONVERT_NOSPACE) ? ENOMEM : EILSEQ;
	    }
	    return result;
	}

	flags &= ~TCL_ENCODING_START;
	srcLen -= srcChunkRead;

	if (Tcl_DStringLength(dstPtr) == 0) {
	    Tcl_DStringSetLength(dstPtr, dstLen);
	}
	Tcl_DStringSetLength(dstPtr, 2 * Tcl_DStringLength(dstPtr) + 1);
	dst = Tcl_DStringValue(dstPtr) + soFar;
	dstLen = Tcl_DStringLength(dstPtr) - soFar - 1;
    }
}

```

tcl9.1 9.1.0; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Tcl_UtfToExternalDStringEx`, lines 1745–1863. Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `e5b5ace1a7ce815e12fd97478754b0060cfd86bb44f3e0fb0d435aafe34fe325`; retained evidence `native_encoding_utf8_offset307-9.1.0-tclEncoding.c`.

```text
Tcl_UtfToExternalDStringEx(
    Tcl_Interp *interp,		/* For error messages. May be NULL. */
    Tcl_Encoding encoding,	/* The encoding for the converted string, or
				 * NULL for the default system encoding. */
    const char *src,		/* Source string in UTF-8. */
    Tcl_Size srcLen,		/* Source string length in bytes, or < 0 for
				 * strlen(). */
    int flags,			/* Conversion control flags. */
    Tcl_DString *dstPtr,	/* Uninitialized or free DString in which the
				 * converted string is stored. */
    Tcl_Size *errorLocPtr)	/* Where to store the error location
				 * (or TCL_INDEX_NONE if no error). May
				 * be NULL. */
{
    char *dst;
    Tcl_EncodingState state;
    const Encoding *encodingPtr;
    int result;
    const char *srcStart = src;
    Tcl_Size dstLen, soFar;

    /* DO FIRST - must always be initialized on return */
    Tcl_DStringInit(dstPtr);

    dst = Tcl_DStringValue(dstPtr);
    dstLen = dstPtr->spaceAvl - 1;

    if (encoding == NULL) {
	encoding = systemEncoding;
    }
    encodingPtr = (Encoding *) encoding;

    if (src == NULL) {
	srcLen = 0;
	src = "";	/* Avoid ABSAN warnings about ops on NULL pointers */
    } else if (srcLen < 0) {
	srcLen = strlen(src);
    }

    flags &= ~TCL_ENCODING_END;
    flags |= TCL_ENCODING_START;
    while (1) {
	int srcChunkLen, srcChunkRead;
	int dstChunkLen, dstChunkWrote, dstChunkChars;

	if (srcLen > INT_MAX) {
	    srcChunkLen = INT_MAX;
	} else {
	    srcChunkLen = srcLen;
	    flags |= TCL_ENCODING_END; /* Last chunk */
	}
	dstChunkLen = dstLen > INT_MAX ? INT_MAX : dstLen;

	result = encodingPtr->fromUtfProc(encodingPtr->clientData, src,
		srcChunkLen, flags, &state, dst, dstChunkLen,
		&srcChunkRead, &dstChunkWrote, &dstChunkChars);
	soFar = dst + dstChunkWrote - Tcl_DStringValue(dstPtr);

	/* Move past the part processed in this go around */
	src += srcChunkRead;

	/*
	 * Keep looping in two case -
	 *   - our destination buffer did not have enough room
	 *   - we had not passed in all the data and error indicated fragment
	 *     of a multibyte character
	 * In both cases we have to grow buffer, move the input source pointer
	 * and loop. Otherwise, return the result we got.
	 */
	if ((result != TCL_CONVERT_NOSPACE) &&
		(result != TCL_CONVERT_MULTIBYTE || (flags & TCL_ENCODING_END))) {
	    Tcl_Size nBytesProcessed = (src - srcStart);
	    Tcl_Size i = soFar + encodingPtr->nullSize - 1;
	    /* Loop as DStringSetLength only stores one nul byte at a time */
	    while (i >= soFar) {
		Tcl_DStringSetLength(dstPtr, i--);
	    }
	    if (errorLocPtr) {
		/*
		 * Do not write error message into interpreter if caller
		 * wants to know error location.
		 */
		*errorLocPtr = result == TCL_OK
			? TCL_INDEX_NONE : nBytesProcessed;
	    } else {
		/* Caller wants error message on failure */
		if (result != TCL_OK && interp != NULL) {
		    Tcl_Size pos = Tcl_NumUtfChars(srcStart, nBytesProcessed);
		    int ucs4;
		    char buf[TCL_INTEGER_SPACE];

		    TclUtfToUniChar(&srcStart[nBytesProcessed], &ucs4);
		    snprintf(buf, sizeof(buf), "%" TCL_SIZE_MODIFIER "d",
			    nBytesProcessed);
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "unexpected character at index %" TCL_SIZE_MODIFIER
			    "u: 'U+%06X'",
			    pos, ucs4));
		    Tcl_SetErrorCode(interp, "TCL", "ENCODING", "ILLEGALSEQUENCE",
			    buf, (char *)NULL);
		}
	    }
	    if (result != TCL_OK) {
		errno = (result == TCL_CONVERT_NOSPACE) ? ENOMEM : EILSEQ;
	    }
	    return result;
	}

	flags &= ~TCL_ENCODING_START;
	srcLen -= srcChunkRead;

	if (Tcl_DStringLength(dstPtr) == 0) {
	    Tcl_DStringSetLength(dstPtr, dstLen);
	}
	Tcl_DStringSetLength(dstPtr, 2 * Tcl_DStringLength(dstPtr) + 1);
	dst = Tcl_DStringValue(dstPtr) + soFar;
	dstLen = Tcl_DStringLength(dstPtr) - soFar - 1;
    }
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
