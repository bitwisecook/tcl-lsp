# naming.string.original-counted-index-range-storage-and-results

Kind: `native-observation`

## Problem statement

Which original counted storage, native unit, constructor, lazy cache and alias branches do actual string index/range reach, and how do those differ from replacement and a Rust Unicode projection?

## Question

For the exact 20 counted original object inputs, what command completion, public original/result primary and resident-byte fields, alias identity and result bytes do string index and string range produce on each measured C Tcl 8.4–9.1 and current Jim library? Which original constructor cases are explicitly unavailable in Jim?

## Conclusion

Index and range are distinct native purposes: index may encode a selected storage unit while range preserves original counted bytes. Raw supplementary bytes differ across C8.4/8.5, C8.6 and C9/Jim; explicit surrogate units remain a separate constructor/input axis. Native binary and resident-String branches differ by release. Jim returns the exact subject for two whole ranges and accepts the aliased index expression that all five C builds reject. Counted index acceptance differs by release and syntax. These are 116 observed command outcomes and four explicit Jim constructor skips, not 120 commands.

## Scope

120 fresh case processes (20 per selected library), all process exit 0: 116 actual command outcomes (108 code 0, eight code 1) and four explicit Jim constructor skips. Before-command, reached-before-result-getter, materialised-result and materialised-subject observations remain separate. Probe byte inputs are counted C arrays, not Tcl source decoding. Original reference fields and alias identities are snapshots under this exact EvalObjv schedule. The proof does not expose C private Unicode/ByteArray caches, private cache epochs, callback or teardown chronology, general lifetime ownership, arbitrary platforms/builds, direct-helper sharing, replacement semantics, Rust assertions or BIG-IP. Earlier CLI and six independent same-library version processes are separate evidence.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual recorded compiler/link vector and source/header/configuration/static-library/CLI bytes; independent same-static-library version driver. Historical library build and complete system toolchain/load closure are unrecorded.. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands.. Dialect: tcl8.4.

Twenty commands: 18 code 0 and two code 1 (cases 14 and 18). Raw four-byte index 1 returns efbfb0 and index 2 efbe9f, while range 1..2 preserves f09f. Raw invalid index returns efbe80 but range preserves 80ff. Counted NUL index emits c080 while whole range preserves 410042. Pure ByteArray index/range results remain ByteArray before their getters; the already materialised shared ByteArray in case 16 also remains ByteArray. Original Unicode-surrogate range has no resident String bytes before the result getter. All observed results are distinct from the original operands. Counted ordinary index with NUL tail fails, but counted end-1 with NUL tail succeeds.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual recorded compiler/link vector and source/header/configuration/static-library/CLI bytes; independent same-static-library version driver. Historical library build and complete system toolchain/load closure are unrecorded.. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands.. Dialect: tcl8.5.

Twenty commands: 19 code 0 and one code 1 (case 14). Raw four-byte/invalid/NUL index and range bytes match the measured 8.4 cases, but ordinary counted index with NUL tail succeeds. Pure ByteArray cases stay binary before getters. The already materialised shared ByteArray in case 16 reaches String primary and a fresh lazy String result. All results are distinct from original operands. Counted end-1 with NUL tail succeeds. Surrogate range, same-first/last and original Unicode ranges have absent resident result bytes before materialisation.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual recorded compiler/link vector and source/header/configuration/static-library/CLI bytes; independent same-static-library version driver. Historical library build and complete system toolchain/load closure are unrecorded.. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands.. Dialect: tcl8.6.

Twenty commands: 19 code 0 and one code 1 (case 14). Raw four-byte index 1 and 2 emit eda0bd and edb880; their range emits eda0bdedb880. Raw invalid index emits c280 but range preserves 80ff. Counted NUL index emits c080 while whole range preserves 410042. Pure ByteArray index/range stay binary; the already materialised shared ByteArray in case 16 reaches String primary and a fresh lazy String result. All results are distinct from the original operands. Both counted ordinary and end-1 indices with NUL tails succeed.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual recorded compiler/link vector and source/header/configuration/static-library/CLI bytes; independent same-static-library version driver. Historical library build and complete system toolchain/load closure are unrecorded.. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands.. Dialect: tcl9.0.

Twenty commands: 18 code 0 and two code 1 (cases 14 and 19). Raw four-byte index 1 emits f09f9880, index 2 emits 5a, and range 1..2 emits f09f98805a. Explicit CESU surrogate input and original Unicode surrogate units remain separately observed eda0bdedb880. Invalid index emits c280 while range preserves 80ff; NUL index emits c080 while range preserves 410042. Pure ByteArray and already materialised shared ByteArray case 16 stay binary before result getters. All results are distinct from original operands. Counted ordinary index succeeds; counted end-1 with NUL tail fails. The failed aliased subject/index case 14 keeps the subject primary none at its reached snapshot.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual recorded compiler/link vector and source/header/configuration/static-library/CLI bytes; independent same-static-library version driver. Historical library build and complete system toolchain/load closure are unrecorded.. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands.. Dialect: tcl9.1.

Twenty commands: 18 code 0 and two code 1 (cases 14 and 19). These 20 byte outputs, result alias identities and public primary/resident-byte fields match the separately measured 9.0.4 cases. Raw supplementary bytes are one selected index character, whereas explicit CESU units remain separate. Pure and materialised shared ByteArrays remain binary on the observed branches. Counted ordinary index succeeds and counted end-1 with NUL tail fails. No result reuses an original operand header.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual recorded compiler/link vector and source/header/configuration/static-library/CLI bytes; independent same-static-library version driver. Historical library build and complete system toolchain/load closure are unrecorded.. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands.. Dialect: jim.

Sixteen commands, all code 0, and four explicit constructor-unavailable skips (12, 13, 16, 17). Raw supplementary index/range bytes match C9 for these inputs, but NUL index preserves 00 and raw invalid index preserves 80. Whole ASCII range and whole counted-NUL range return the exact original subject header (cases 1 and 9); partial ranges are distinct. The aliased subject-as-index expression 2*1 succeeds and returns 31, with the same original subject reaching String primary. Both counted ordinary and end-1 index objects with NUL tails succeed. Jim charLength is observed only on actual String primary; no native binary/Unicode constructor result is substituted for the four skips.

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance process for these original counted object inputs.. Channel: No BIG-IP original object driver capture.. Dialect: F5 iRules.

Measured C Tcl and Jim objects do not establish a BIG-IP/TMM storage or execution protocol.

## Exact evidence

- `original-counted-probe` (input): [runtime/rust/tests/data/native_original_index_range635/probe.c](../../../../runtime/rust/tests/data/native_original_index_range635/probe.c). SHA-256 `81b03dfd9e5f3763c7cae44aca295c2c9b1c37a172d18f097ea262a378445bde`. Exact counted constructors and original alias roles. Reached snapshots precede both result and subject getters.
- `original-request` (input): [runtime/rust/tests/data/native_original_index_range635/request.json](../../../../runtime/rust/tests/data/native_original_index_range635/request.json). SHA-256 `c875e21028dc0bc95855caaa46f56486db138ab60c9a41d5da39f591b06c8bf6`. Original problem, full 20-case requirements and four explicit Jim constructor skip indices; no expected answers.
- `actual-case-projection` (observation): [runtime/rust/tests/data/native_original_index_range635/interpretation.json](../../../../runtime/rust/tests/data/native_original_index_range635/interpretation.json). SHA-256 `1a30139d1f3b85958ec9c0f56a1d9c20e43bea26ee1a9c8858ba8ea22a37e42b`. All original process rows: 116 command outcomes and four constructor skips. Raw fields and raw stream/receipt hashes are retained.
- `exact-input-map` (provider): [runtime/rust/tests/data/native_original_index_range635/pinned-input-map.json](../../../../runtime/rust/tests/data/native_original_index_range635/pinned-input-map.json). SHA-256 `fef9e14559249204b03380cccf73546265e8c88bb3acc49d3716117b6c342e98`. Every actual input pin joined to complete independently verified archive members.
- `exact-input-bytes` (provider): [runtime/rust/tests/data/native_original_index_range635/pinned-input-bytes.tar.gz](../../../../runtime/rust/tests/data/native_original_index_range635/pinned-input-bytes.tar.gz). SHA-256 `dd3b095e11e37ac7e3065aa6c2fa18454e0c96e2f85ede0b0507eb5af3bf95c7`. Whole measured source/header/configuration/library/compiler/CLI/input bytes, losslessly compressed. No complete build/load closure is inferred.
- `all-original-retention` (provider): [runtime/rust/tests/data/native_original_index_range635/retention.json](../../../../runtime/rust/tests/data/native_original_index_range635/retention.json). SHA-256 `909878efd74ae2b97c61aa15c5fec3b7b7370ff78039be471f463c9d3bee0bfb`. Every original receipt, raw stream and all 12 original driver/version ELFs retained and round-trip verified.
- `source-window-0` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/0/snippet`. Tcl_StringObjCmd STR_INDEX case: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-1` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/1/snippet`. Tcl_StringObjCmd STR_RANGE case: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-2` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/2/snippet`. Tcl_GetUniChar: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-3` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/3/snippet`. Tcl_GetRange: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-4` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/4/snippet`. Tcl_GetCharLength: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-5` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/5/snippet`. Tcl_NewUnicodeObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-6` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/6/snippet`. TclGetIntForIndex: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-7` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/7/snippet`. TclUtfToUniChar macro: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-8` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/8/snippet`. StringIndexCmd: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-9` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/9/snippet`. StringRangeCmd: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-10` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/10/snippet`. Tcl_GetUniChar: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-11` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/11/snippet`. Tcl_GetRange: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-12` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/12/snippet`. Tcl_GetCharLength: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-13` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/13/snippet`. Tcl_NewUnicodeObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-14` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/14/snippet`. TclGetIntForIndex: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-15` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/15/snippet`. TclUtfToUniChar macro: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-16` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/16/snippet`. StringIndexCmd: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-17` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/17/snippet`. StringRangeCmd: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-18` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/18/snippet`. Tcl_GetUniChar: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-19` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/19/snippet`. Tcl_GetRange: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-20` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/20/snippet`. Tcl_GetCharLength: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-21` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/21/snippet`. Tcl_NewUnicodeObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-22` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/22/snippet`. TclGetIntForIndex: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-23` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/23/snippet`. GetEndOffsetFromObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-24` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/24/snippet`. StringIndexCmd: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-25` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/25/snippet`. StringRangeCmd: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-26` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/26/snippet`. Tcl_GetUniChar: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-27` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/27/snippet`. Tcl_GetRange: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-28` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/28/snippet`. Tcl_GetCharLength: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-29` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/29/snippet`. Tcl_NewUnicodeObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-30` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/30/snippet`. Tcl_GetIntForIndex: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-31` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/31/snippet`. GetEndOffsetFromObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-32` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/32/snippet`. StringIndexCmd: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-33` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/33/snippet`. StringRangeCmd: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-34` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/34/snippet`. Tcl_GetUniChar: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-35` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/35/snippet`. Tcl_GetRange: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-36` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/36/snippet`. Tcl_GetCharLength: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-37` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/37/snippet`. Tcl_NewUnicodeObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-38` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/38/snippet`. Tcl_GetIntForIndex: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-39` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/39/snippet`. GetEndOffsetFromObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-40` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/40/snippet`. JimStringGetRange: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-41` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/41/snippet`. Jim_StringRangeObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-42` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/42/snippet`. Jim_StringByteRangeObj: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-43` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/43/snippet`. Jim_GetIndex: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-44` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/44/snippet`. SetIndexFromAny: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-45` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/45/snippet`. Jim_Utf8Length: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-46` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/46/snippet`. Jim_StringCoreCommand OPT_RANGE case: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-47` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/47/snippet`. Jim_StringCoreCommand OPT_INDEX case: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-48` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/48/snippet`. utf8_strlen: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-49` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/49/snippet`. utf8_index: decoded exact snippet with physical-LF lines and whole source/archive join.
- `source-window-50` (source-anchor): [runtime/rust/tests/data/native_original_index_range635/source-windows.json](../../../../runtime/rust/tests/data/native_original_index_range635/source-windows.json). SHA-256 `d6dad86f78d371871b997885c1cc4cd997ecc1ea7da6517bc2f8b1c77a9f487f`. JSON pointer `/anchors/50/snippet`. utf8_tounicode: decoded exact snippet with physical-LF lines and whole source/archive join.
- `tcl8.4-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/8.4.20/receipt.json). SHA-256 `7d5968d1296a1d1d20e7e1a162744e266b0c877edd44409d6a15ca54398721a5`. Actual original compile/link/launch recipes, CLI version, all input pins and exact 20 process receipts.
- `tcl8.4-cli-version-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/8.4.20/cli-version.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/8.4.20/cli-version.stdout). SHA-256 `c4a268983bec029286482df6caa3aaae81571c33d467c0a13f258e335da5954f`. Actual selected CLI version output, independently bound by the original receipt.
- `tcl8.4-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/8.4.20/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/8.4.20/same-library-version/receipt.json). SHA-256 `608d5a953410d8eea25f87d0e229f9899ab65cb16b4acebb8a8d17ec7fef74d5`. Independent version driver using the identical selected static library and headers; original case receipts remain unchanged.
- `tcl8.4-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/8.4.20/same-library-version/execute.stdout). SHA-256 `ef18d09813e9c05fbd397154e6f786ea0221f2ce8981597665e917a08a5c5d7f`. Actual linked-library patchlevel and, for C, Tcl_GetVersion fields.
- `tcl8.5-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/8.5.19/receipt.json). SHA-256 `a1c653cc554c12e08cad4b1c26ea66a32d7c619dd695a754c8d0225500dff5c2`. Actual original compile/link/launch recipes, CLI version, all input pins and exact 20 process receipts.
- `tcl8.5-cli-version-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/8.5.19/cli-version.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/8.5.19/cli-version.stdout). SHA-256 `6b58793c92a3db92926bdacbe0a52a520d417fe876aaa7cf8c29fed54a1fea71`. Actual selected CLI version output, independently bound by the original receipt.
- `tcl8.5-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/8.5.19/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/8.5.19/same-library-version/receipt.json). SHA-256 `9bb46ab46df34d98fb0627d01f98e6f1017090aa20e33f75bfabdb0422fb99f1`. Independent version driver using the identical selected static library and headers; original case receipts remain unchanged.
- `tcl8.5-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/8.5.19/same-library-version/execute.stdout). SHA-256 `1bd253489464d3ed255f286efdc8deb3768100a5b873e0a062f5026291fcf9bb`. Actual linked-library patchlevel and, for C, Tcl_GetVersion fields.
- `tcl8.6-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/8.6.18/receipt.json). SHA-256 `8fc8c5cc6544723b25b8321c36e7c5872dde482c8df4a33044ea30818148bb88`. Actual original compile/link/launch recipes, CLI version, all input pins and exact 20 process receipts.
- `tcl8.6-cli-version-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/8.6.18/cli-version.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/8.6.18/cli-version.stdout). SHA-256 `7523e113df6c321e18099a10ec22086db746e2fcc31a879e16d961e5d3878d01`. Actual selected CLI version output, independently bound by the original receipt.
- `tcl8.6-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/8.6.18/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/8.6.18/same-library-version/receipt.json). SHA-256 `c5d885481beca67b265fab250d5df48062e483ca01ea251d6bdb579b5be3256b`. Independent version driver using the identical selected static library and headers; original case receipts remain unchanged.
- `tcl8.6-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/8.6.18/same-library-version/execute.stdout). SHA-256 `dd12d5dc76bfc80ae3033ac87b1932541f275a888adf9e03dabbcb5878b0df8c`. Actual linked-library patchlevel and, for C, Tcl_GetVersion fields.
- `tcl9.0-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/9.0.4/receipt.json). SHA-256 `3f75cdc6e7992bf672875324dc2c658f3f19f96aed8c3c3cfe7b0698252d302c`. Actual original compile/link/launch recipes, CLI version, all input pins and exact 20 process receipts.
- `tcl9.0-cli-version-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/9.0.4/cli-version.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/9.0.4/cli-version.stdout). SHA-256 `b83920bac1b2d47c3840f588f95bee9dc69b85e26117f471541cdf85d7da52a2`. Actual selected CLI version output, independently bound by the original receipt.
- `tcl9.0-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/9.0.4/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/9.0.4/same-library-version/receipt.json). SHA-256 `3bd5d016dbf36529d40e394d64e3778b2566d432b655c8e555c23ecc47240fd5`. Independent version driver using the identical selected static library and headers; original case receipts remain unchanged.
- `tcl9.0-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/9.0.4/same-library-version/execute.stdout). SHA-256 `d016aed0da5636f83bfe6d76f7c26b06e30ce7951203b5d3543001780eb51a2c`. Actual linked-library patchlevel and, for C, Tcl_GetVersion fields.
- `tcl9.1-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/9.1.0/receipt.json). SHA-256 `9bcc1603df7fe0c0794694d490d7a780a8b2862b04b5f1c894a4e31fb0021899`. Actual original compile/link/launch recipes, CLI version, all input pins and exact 20 process receipts.
- `tcl9.1-cli-version-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/9.1.0/cli-version.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/9.1.0/cli-version.stdout). SHA-256 `077ee5b3a3a7fc2622ceff6466fea0c28f3fb08b24653dcc7f4baab29b9aef36`. Actual selected CLI version output, independently bound by the original receipt.
- `tcl9.1-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/9.1.0/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/9.1.0/same-library-version/receipt.json). SHA-256 `532263333aaa91817a8ae6c19744d9906a6b3b590cc094a465923f7cdb4f4612`. Independent version driver using the identical selected static library and headers; original case receipts remain unchanged.
- `tcl9.1-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/9.1.0/same-library-version/execute.stdout). SHA-256 `b225f15b75641002abfa4f29d4d1be66051123c85bc897b9a7234b22317ea071`. Actual linked-library patchlevel and, for C, Tcl_GetVersion fields.
- `jim-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/Jim/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/Jim/receipt.json). SHA-256 `6f0f000c9d61a79fce94bc94f13e1c8533aa7b9862b1d61e74ac47ee01327f94`. Actual original compile/link/launch recipes, CLI version, all input pins and exact 20 process receipts.
- `jim-cli-version-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/Jim/cli-version.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/Jim/cli-version.stdout). SHA-256 `f9752899fa012cf994a528e0250402609067a6f67a144a1172644a9329386803`. Actual selected CLI version output, independently bound by the original receipt.
- `jim-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_index_range635/Jim/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_index_range635/Jim/same-library-version/receipt.json). SHA-256 `ba0f7b68a6e45a047488a9e4878c48606d7fdaf0a7ddffae5ec975e24b28dfa0`. Independent version driver using the identical selected static library and headers; original case receipts remain unchanged.
- `jim-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_index_range635/Jim/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_index_range635/Jim/same-library-version/execute.stdout). SHA-256 `d7ce62ac8c979a247ad4edf0a43c9133dc8b547cdfc8e5b540fccd9bbe848166`. Actual linked-library patchlevel and, for C, Tcl_GetVersion fields.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_StringObjCmd STR_INDEX case`, lines 1456–1500. Line unit: physical LF only. Original byte range [40373, 41584). Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `b119f73d1d3441e0239f186e4a4c49606df2171c034014e8a55055721efb6b79`; retained evidence `source-window-0`.

```text
	case STR_INDEX: {
	    if (objc != 4) {
	        Tcl_WrongNumArgs(interp, 2, objv, "string charIndex");
		return TCL_ERROR;
	    }

	    /*
	     * If we have a ByteArray object, avoid indexing in the
	     * Utf string since the byte array contains one byte per
	     * character.  Otherwise, use the Unicode string rep to
	     * get the index'th char.
	     */

	    if (objv[2]->typePtr == &tclByteArrayType) {
		string1 = (char *) Tcl_GetByteArrayFromObj(objv[2], &length1);

		if (TclGetIntForIndex(interp, objv[3], length1 - 1,
			&index) != TCL_OK) {
		    return TCL_ERROR;
		}
		if ((index >= 0) && (index < length1)) {
		    Tcl_SetByteArrayObj(resultPtr,
			    (unsigned char *)(&string1[index]), 1);
		}
	    } else {
		/*
		 * Get Unicode char length to calulate what 'end' means.
		 */
		length1 = Tcl_GetCharLength(objv[2]);

		if (TclGetIntForIndex(interp, objv[3], length1 - 1,
			&index) != TCL_OK) {
		    return TCL_ERROR;
		}
		if ((index >= 0) && (index < length1)) {
		    char buf[TCL_UTF_MAX];
		    Tcl_UniChar ch;

		    ch      = Tcl_GetUniChar(objv[2], index);
		    length1 = Tcl_UniCharToUtf(ch, buf);
		    Tcl_SetStringObj(resultPtr, buf, length1);
		}
	    }
	    break;
	}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_StringObjCmd STR_RANGE case`, lines 2084–2134. Line unit: physical LF only. Original byte range [57197, 58454). Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `6077fb2fb30a674c6a706adaedac4273776a8249bc59099792c5be6026703d66`; retained evidence `source-window-1`.

```text
	case STR_RANGE: {
	    int first, last;

	    if (objc != 5) {
	        Tcl_WrongNumArgs(interp, 2, objv, "string first last");
		return TCL_ERROR;
	    }

	    /*
	     * If we have a ByteArray object, avoid indexing in the
	     * Utf string since the byte array contains one byte per
	     * character.  Otherwise, use the Unicode string rep to
	     * get the range.
	     */

	    if (objv[2]->typePtr == &tclByteArrayType) {
		string1 = (char *)Tcl_GetByteArrayFromObj(objv[2], &length1);
		length1--;
	    } else {
		/*
		 * Get the length in actual characters.
		 */
		string1 = NULL;
		length1 = Tcl_GetCharLength(objv[2]) - 1;
	    }

	    if ((TclGetIntForIndex(interp, objv[3], length1, &first) != TCL_OK)
		    || (TclGetIntForIndex(interp, objv[4], length1,
			    &last) != TCL_OK)) {
		return TCL_ERROR;
	    }

	    if (first < 0) {
		first = 0;
	    }
	    if (last >= length1) {
		last = length1;
	    }
	    if (last >= first) {
		if (string1 != NULL) {
		    int numBytes = last - first + 1;
		    resultPtr = Tcl_NewByteArrayObj(
			(unsigned char *) &string1[first], numBytes);
		    Tcl_SetObjResult(interp, resultPtr);
		} else {
		    Tcl_SetObjResult(interp,
			    Tcl_GetRange(objv[2], first, last));
		}
	    }
	    break;
	}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 482–522. Line unit: physical LF only. Original byte range [16457, 17473). Full-source SHA-256 `a86bf9f649ce6de16363984f1d8af398c464984e7d5677543f279ed87e35e62c`; snippet SHA-256 `2fa8da230e5d84d5c47792180f78ccdbd46236efde83d03494fd596e674402a9`; retained evidence `source-window-2`.

```text
Tcl_GetUniChar(objPtr, index)
    Tcl_Obj *objPtr;	/* The object to get the Unicode charater from. */
    int index;		/* Get the index'th Unicode character. */
{
    Tcl_UniChar unichar;
    String *stringPtr;
    
    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->numChars == -1) {

	/*
	 * We haven't yet calculated the length, so we don't have the
	 * Unicode str.  We need to know the number of chars before we
	 * can do indexing.
	 */

	Tcl_GetCharLength(objPtr);

	/*
	 * We need to fetch the pointer again because we may have just
	 * reallocated the structure.
	 */
	
	stringPtr = GET_STRING(objPtr);
    }
    if (stringPtr->hasUnicode == 0) {

	/*
	 * All of the characters in the Utf string are 1 byte chars,
	 * so we don't store the unicode char.  We get the Utf string
	 * and convert the index'th byte to a Unicode character.
	 */

	unichar = (Tcl_UniChar) objPtr->bytes[index];
    } else {
	unichar = stringPtr->unicode[index];
    }
    return unichar;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 651–704. Line unit: physical LF only. Original byte range [21100, 22520). Full-source SHA-256 `a86bf9f649ce6de16363984f1d8af398c464984e7d5677543f279ed87e35e62c`; snippet SHA-256 `e43875efbb18819c72e458f8fabcc5949f3bc35c642a794f4294cce5f5a11dc0`; retained evidence `source-window-3`.

```text
Tcl_GetRange(objPtr, first, last)
    Tcl_Obj *objPtr;		/* The Tcl object to find the range of. */
    int first;			/* First index of the range. */
    int last;			/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to find the range of. */
    String *stringPtr;
    
    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->numChars == -1) {
    
	/*
	 * We haven't yet calculated the length, so we don't have the
	 * Unicode str.  We need to know the number of chars before we
	 * can do indexing.
	 */

	Tcl_GetCharLength(objPtr);

	/*
	 * We need to fetch the pointer again because we may have just
	 * reallocated the structure.
	 */
	
	stringPtr = GET_STRING(objPtr);
    }

    if (objPtr->bytes && stringPtr->numChars == objPtr->length) {
	char *str = Tcl_GetString(objPtr);

	/*
	 * All of the characters in the Utf string are 1 byte chars,
	 * so we don't store the unicode char.  Create a new string
	 * object containing the specified range of chars.
	 */
	
	newObjPtr = Tcl_NewStringObj(&str[first], last-first+1);

	/*
	 * Since we know the new string only has 1-byte chars, we
	 * can set it's numChars field.
	 */
	
	SetStringFromAny(NULL, newObjPtr);
	stringPtr = GET_STRING(newObjPtr);
	stringPtr->numChars = last-first+1;
    } else {
	newObjPtr = Tcl_NewUnicodeObj(stringPtr->unicode + first,
		last-first+1);
    }
    return newObjPtr;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 402–462. Line unit: physical LF only. Original byte range [14356, 15994). Full-source SHA-256 `a86bf9f649ce6de16363984f1d8af398c464984e7d5677543f279ed87e35e62c`; snippet SHA-256 `8a0345123a3d160661109480c1af6ac553c74297ec37d58b5f5533aadfa90ee6`; retained evidence `source-window-4`.

```text
Tcl_GetCharLength(objPtr)
    Tcl_Obj *objPtr;	/* The String object to get the num chars of. */
{
    String *stringPtr;
    
    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    /*
     * If numChars is unknown, then calculate the number of characaters
     * while populating the Unicode string.
     */
    
    if (stringPtr->numChars == -1) {
	register int i = objPtr->length;
	register unsigned char *str = (unsigned char *) objPtr->bytes;

	/*
	 * This is a speed sensitive function, so run specially over the
	 * string to count continuous ascii characters before resorting
	 * to the Tcl_NumUtfChars call.  This is a long form of:
	 stringPtr->numChars = Tcl_NumUtfChars(objPtr->bytes, objPtr->length);
	*/

	while (i && (*str < 0xC0)) { i--; str++; }
	stringPtr->numChars = objPtr->length - i;
	if (i) {
	    stringPtr->numChars += Tcl_NumUtfChars(objPtr->bytes
		    + (objPtr->length - i), i);
	}

 	if (stringPtr->numChars == objPtr->length) {

	    /*
	     * Since we've just calculated the number of chars, and all
	     * UTF chars are 1-byte long, we don't need to store the
	     * unicode string.
	     */

	    stringPtr->hasUnicode = 0;

	} else {
    
	    /*
	     * Since we've just calucalated the number of chars, and not
	     * all UTF chars are 1-byte long, go ahead and populate the
	     * unicode string.
	     */

	    FillUnicodeRep(objPtr);

	    /*
	     * We need to fetch the pointer again because we have just
	     * reallocated the structure to make room for the Unicode data.
	     */
	    
	    stringPtr = GET_STRING(objPtr);
	}
    }
    return stringPtr->numChars;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 371–382. Line unit: physical LF only. Original byte range [13600, 13926). Full-source SHA-256 `a86bf9f649ce6de16363984f1d8af398c464984e7d5677543f279ed87e35e62c`; snippet SHA-256 `d81d8c46a110edd94eeea725b80b3b2efb68139bb08d35828c12857839c0a6cf`; retained evidence `source-window-5`.

```text
Tcl_NewUnicodeObj(unicode, numChars)
    CONST Tcl_UniChar *unicode;	/* The unicode string used to initialize
				 * the new object. */
    int numChars;		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclUtil.c`, function `TclGetIntForIndex`, lines 2331–2381. Line unit: physical LF only. Original byte range [62656, 64092). Full-source SHA-256 `26a0bb5644999077e8a6a82a7ab1bbcf73787f889a35d7ede5369788c66d2e4c`; snippet SHA-256 `c5e3e71cdf92aebc7d1b3d26977965108fe533b9b65063cf629c812bb80709a2`; retained evidence `source-window-6`.

```text
TclGetIntForIndex(interp, objPtr, endValue, indexPtr)
    Tcl_Interp *interp;		/* Interpreter to use for error reporting. 
				 * If NULL, then no error message is left
				 * after errors. */
    Tcl_Obj *objPtr;		/* Points to an object containing either
				 * "end" or an integer. */
    int endValue;		/* The value to be stored at "indexPtr" if
				 * "objPtr" holds "end". */
    int *indexPtr;		/* Location filled in with an integer
				 * representing an index. */
{
    if (Tcl_GetIntFromObj(NULL, objPtr, indexPtr) == TCL_OK) {
	return TCL_OK;
    }

    if (SetEndOffsetFromAny(NULL, objPtr) == TCL_OK) {
	/*
	 * If the object is already an offset from the end of the
	 * list, or can be converted to one, use it.
	 */

	*indexPtr = endValue + objPtr->internalRep.longValue;

    } else {
	/*
	 * Report a parse error.
	 */

	if (interp != NULL) {
	    char *bytes = Tcl_GetString(objPtr);
	    /*
	     * The result might not be empty; this resets it which
	     * should be both a cheap operation, and of little problem
	     * because this is an error-generation path anyway.
	     */
	    Tcl_ResetResult(interp);
	    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
				   "bad index \"", bytes,
				   "\": must be integer or end?-integer?",
				   (char *) NULL);
	    if (!strncmp(bytes, "end-", 3)) {
		bytes += 3;
	    }
	    TclCheckBadOctal(interp, bytes);
	}

	return TCL_ERROR;
    }
	    
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h`, function `TclUtfToUniChar macro`, lines 2604–2623. Line unit: physical LF only. Original byte range [106143, 106974). Full-source SHA-256 `f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a`; snippet SHA-256 `ff3d12fec44694ddea1f35014764627fceb043f0776786a22960fc401b18fa61`; retained evidence `source-window-7`.

```text
#define TclUtfToUniChar(str, chPtr) \
	((((unsigned char) *(str)) < 0xC0) ? \
	    ((*(chPtr) = (Tcl_UniChar) *(str)), 1) \
	    : Tcl_UtfToUniChar(str, chPtr))

/*
 *----------------------------------------------------------------
 * Macro used by the Tcl core to compare Unicode strings.  On
 * big-endian systems we can use the more efficient memcmp, but
 * this would not be lexically correct on little-endian systems.
 * The ANSI C "prototype" for this macro is:
 *
 * EXTERN int TclUniCharNcmp _ANSI_ARGS_((CONST Tcl_UniChar *cs,
 *         CONST Tcl_UniChar *ct, unsigned long n));
 *----------------------------------------------------------------
 */
#ifdef WORDS_BIGENDIAN
#   define TclUniCharNcmp(cs,ct,n) memcmp((cs),(ct),(n)*sizeof(Tcl_UniChar))
#else /* !WORDS_BIGENDIAN */
#   define TclUniCharNcmp Tcl_UniCharNcmp

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `StringIndexCmd`, lines 1331–1381. Line unit: physical LF only. Original byte range [32691, 34104). Full-source SHA-256 `fe2c67a2097ece2e07d9345a5edf416b5fbc891276349e360f9c25da1cc8f348`; snippet SHA-256 `cf43d50641dd32ef8c6ba51ce2762089cf71a970e74be22e8f25363c6d068b9d`; retained evidence `source-window-8`.

```text
StringIndexCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int length, index;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string charIndex");
	return TCL_ERROR;
    }

    /*
     * If we have a ByteArray object, avoid indexing in the Utf string since
     * the byte array contains one byte per character. Otherwise, use the
     * Unicode string rep to get the index'th char.
     */

    if (TclIsPureByteArray(objv[1])) {
	const unsigned char *string =
		Tcl_GetByteArrayFromObj(objv[1], &length);

	if (TclGetIntForIndexM(interp, objv[2], length-1, &index) != TCL_OK){
	    return TCL_ERROR;
	}
	string = Tcl_GetByteArrayFromObj(objv[1], &length);
	if ((index >= 0) && (index < length)) {
	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(string + index, 1));
	}
    } else {
	/*
	 * Get Unicode char length to calulate what 'end' means.
	 */

	length = Tcl_GetCharLength(objv[1]);

	if (TclGetIntForIndexM(interp, objv[2], length-1, &index) != TCL_OK){
	    return TCL_ERROR;
	}
	if ((index >= 0) && (index < length)) {
	    char buf[TCL_UTF_MAX];
	    Tcl_UniChar ch;

	    ch = Tcl_GetUniChar(objv[1], index);
	    length = Tcl_UniCharToUtf(ch, buf);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(buf, length));
	}
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `StringRangeCmd`, lines 2071–2128. Line unit: physical LF only. Original byte range [51436, 52881). Full-source SHA-256 `fe2c67a2097ece2e07d9345a5edf416b5fbc891276349e360f9c25da1cc8f348`; snippet SHA-256 `9c60c1a66b64e366234348ff685ef65caed36f33654b1d5ae6ff0cd74c61e476`; retained evidence `source-window-9`.

```text
StringRangeCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const unsigned char *string;
    int length, first, last;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last");
	return TCL_ERROR;
    }

    /*
     * If we have a ByteArray object, avoid indexing in the Utf string since
     * the byte array contains one byte per character. Otherwise, use the
     * Unicode string rep to get the range.
     */

    if (TclIsPureByteArray(objv[1])) {
	string = Tcl_GetByteArrayFromObj(objv[1], &length);
	length--;
    } else {
	/*
	 * Get the length in actual characters.
	 */

	string = NULL;
	length = Tcl_GetCharLength(objv[1]) - 1;
    }

    if (TclGetIntForIndexM(interp, objv[2], length, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], length, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    if (first < 0) {
	first = 0;
    }
    if (last >= length) {
	last = length;
    }
    if (last >= first) {
	if (string != NULL) {
	    /*
	     * Reread the string to prevent shimmering nasties.
	     */

	    string = Tcl_GetByteArrayFromObj(objv[1], &length);
	    Tcl_SetObjResult(interp,
		    Tcl_NewByteArrayObj(string+first, last - first + 1));
	} else {
	    Tcl_SetObjResult(interp, Tcl_GetRange(objv[1], first, last));
	}
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 478–516. Line unit: physical LF only. Original byte range [16509, 17507). Full-source SHA-256 `d014bc85fc8f0d12abbd60116f403f9131dd4e933580833d2b02610e211ab449`; snippet SHA-256 `aa3356a708540a9fd641aba2d9ca112310fe86d924098eb6f70bb437e762d66d`; retained evidence `source-window-10`.

```text
Tcl_GetUniChar(
    Tcl_Obj *objPtr,		/* The object to get the Unicode charater
				 * from. */
    int index)			/* Get the index'th Unicode character. */
{
    Tcl_UniChar unichar;
    String *stringPtr;

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->numChars == -1) {
	/*
	 * We haven't yet calculated the length, so we don't have the Unicode
	 * str. We need to know the number of chars before we can do indexing.
	 */

	Tcl_GetCharLength(objPtr);

	/*
	 * We need to fetch the pointer again because we may have just
	 * reallocated the structure.
	 */

	stringPtr = GET_STRING(objPtr);
    }
    if (stringPtr->hasUnicode == 0) {
	/*
	 * All of the characters in the Utf string are 1 byte chars, so we
	 * don't store the unicode char. We get the Utf string and convert the
	 * index'th byte to a Unicode character.
	 */

	unichar = (Tcl_UniChar) objPtr->bytes[index];
    } else {
	unichar = stringPtr->unicode[index];
    }
    return unichar;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 645–696. Line unit: physical LF only. Original byte range [21100, 22483). Full-source SHA-256 `d014bc85fc8f0d12abbd60116f403f9131dd4e933580833d2b02610e211ab449`; snippet SHA-256 `f21bc1e9040b3317a3cae4978a935112f7d5e3c38877bf1549957a3fdd0fb427`; retained evidence `source-window-11`.

```text
Tcl_GetRange(
    Tcl_Obj *objPtr,		/* The Tcl object to find the range of. */
    int first,			/* First index of the range. */
    int last)			/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to find the range of. */
    String *stringPtr;

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->numChars == -1) {
	/*
	 * We haven't yet calculated the length, so we don't have the Unicode
	 * str. We need to know the number of chars before we can do indexing.
	 */

	Tcl_GetCharLength(objPtr);

	/*
	 * We need to fetch the pointer again because we may have just
	 * reallocated the structure.
	 */

	stringPtr = GET_STRING(objPtr);
    }

    if (objPtr->bytes && (stringPtr->numChars == objPtr->length)) {
	char *str = TclGetString(objPtr);

	/*
	 * All of the characters in the Utf string are 1 byte chars, so we
	 * don't store the unicode char. Create a new string object containing
	 * the specified range of chars.
	 */

	newObjPtr = Tcl_NewStringObj(&str[first], last-first+1);

	/*
	 * Since we know the new string only has 1-byte chars, we can set it's
	 * numChars field.
	 */

	SetStringFromAny(NULL, newObjPtr);
	stringPtr = GET_STRING(newObjPtr);
	stringPtr->numChars = last-first+1;
    } else {
	newObjPtr = Tcl_NewUnicodeObj(stringPtr->unicode + first,
		last-first+1);
    }
    return newObjPtr;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 395–458. Line unit: physical LF only. Original byte range [14378, 16047). Full-source SHA-256 `d014bc85fc8f0d12abbd60116f403f9131dd4e933580833d2b02610e211ab449`; snippet SHA-256 `dc5437a179e5f3dc28b8590347693274e9a4d531f82b9194c164ff41d0985f7a`; retained evidence `source-window-12`.

```text
Tcl_GetCharLength(
    Tcl_Obj *objPtr)		/* The String object to get the num chars
				 * of. */
{
    String *stringPtr;

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    /*
     * If numChars is unknown, then calculate the number of characaters while
     * populating the Unicode string.
     */

    if (stringPtr->numChars == -1) {
	register int i = objPtr->length;
	register unsigned char *str = (unsigned char *) objPtr->bytes;

	/*
	 * This is a speed sensitive function, so run specially over the
	 * string to count continuous ascii characters before resorting to the
	 * Tcl_NumUtfChars call. This is a long form of:
	 stringPtr->numChars = Tcl_NumUtfChars(objPtr->bytes,objPtr->length);
	 *
	 * TODO: Consider macro-izing this.
	 */

	while (i && (*str < 0xC0)) {
	    i--;
	    str++;
	}
	stringPtr->numChars = objPtr->length - i;
	if (i) {
	    stringPtr->numChars += Tcl_NumUtfChars(objPtr->bytes
		    + (objPtr->length - i), i);
	}

	if (stringPtr->numChars == objPtr->length) {
	    /*
	     * Since we've just calculated the number of chars, and all UTF
	     * chars are 1-byte long, we don't need to store the unicode
	     * string.
	     */

	    stringPtr->hasUnicode = 0;
	} else {
	    /*
	     * Since we've just calucalated the number of chars, and not all
	     * UTF chars are 1-byte long, go ahead and populate the unicode
	     * string.
	     */

	    FillUnicodeRep(objPtr);

	    /*
	     * We need to fetch the pointer again because we have just
	     * reallocated the structure to make room for the Unicode data.
	     */

	    stringPtr = GET_STRING(objPtr);
	}
    }
    return stringPtr->numChars;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 364–375. Line unit: physical LF only. Original byte range [13641, 13949). Full-source SHA-256 `d014bc85fc8f0d12abbd60116f403f9131dd4e933580833d2b02610e211ab449`; snippet SHA-256 `5106fe8044b31c6e7e3f121b0787b6fd7dc564e2b5f60cf07f18d4c0a05fb007`; retained evidence `source-window-13`.

```text
Tcl_NewUnicodeObj(
    const Tcl_UniChar *unicode,	/* The unicode string used to initialize the
				 * new object. */
    int numChars)		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclUtil.c`, function `TclGetIntForIndex`, lines 3234–3325. Line unit: physical LF only. Original byte range [92509, 94743). Full-source SHA-256 `67be2e4aed576288b31069a98a1ddacf8354c87ca1a8c8e85e53421364a989cd`; snippet SHA-256 `a7968238557112ae80779f5b5ade88692a05399b891743a618062eb81edf7b56`; retained evidence `source-window-14`.

```text
TclGetIntForIndex(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting. If
				 * NULL, then no error message is left after
				 * errors. */
    Tcl_Obj *objPtr,		/* Points to an object containing either "end"
				 * or an integer. */
    int endValue,		/* The value to be stored at "indexPtr" if
				 * "objPtr" holds "end". */
    int *indexPtr)		/* Location filled in with an integer
				 * representing an index. */
{
    int length;
    char *opPtr, *bytes;

    if (TclGetIntFromObj(NULL, objPtr, indexPtr) == TCL_OK) {
	return TCL_OK;
    }

    if (SetEndOffsetFromAny(NULL, objPtr) == TCL_OK) {
	/*
	 * If the object is already an offset from the end of the list, or can
	 * be converted to one, use it.
	 */

	*indexPtr = endValue + objPtr->internalRep.longValue;
	return TCL_OK;
    }

    bytes = TclGetStringFromObj(objPtr, &length);

    /*
     * Leading whitespace is acceptable in an index.
     */

    while (length && TclIsSpaceProc(*bytes)) {
	bytes++;
	length--;
    }

    if (TclParseNumber(NULL, NULL, NULL, bytes, length, (const char **)&opPtr,
	    TCL_PARSE_INTEGER_ONLY | TCL_PARSE_NO_WHITESPACE) == TCL_OK) {
	int code, first, second;
	char savedOp = *opPtr;

	if ((savedOp != '+') && (savedOp != '-')) {
	    goto parseError;
	}
	if (TclIsSpaceProc(opPtr[1])) {
	    goto parseError;
	}
	*opPtr = '\0';
	code = Tcl_GetInt(interp, bytes, &first);
	*opPtr = savedOp;
	if (code == TCL_ERROR) {
	    goto parseError;
	}
	if (TCL_ERROR == Tcl_GetInt(interp, opPtr+1, &second)) {
	    goto parseError;
	}
	if (savedOp == '+') {
	    *indexPtr = first + second;
	} else {
	    *indexPtr = first - second;
	}
	return TCL_OK;
    }

    /*
     * Report a parse error.
     */

  parseError:
    if (interp != NULL) {
	char *bytes = Tcl_GetString(objPtr);

	/*
	 * The result might not be empty; this resets it which should be both
	 * a cheap operation, and of little problem because this is an
	 * error-generation path anyway.
	 */

	Tcl_ResetResult(interp);
	Tcl_AppendResult(interp, "bad index \"", bytes,
		"\": must be integer?[+-]integer? or end?[+-]integer?", NULL);
	if (!strncmp(bytes, "end-", 4)) {
	    bytes += 4;
	}
	TclCheckBadOctal(interp, bytes);
    }

    return TCL_ERROR;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h`, function `TclUtfToUniChar macro`, lines 3670–3708. Line unit: physical LF only. Original byte range [141500, 143213). Full-source SHA-256 `72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa`; snippet SHA-256 `fccd790ea7cdcbc2cd7a1a0518a75f44fa6f0f8a0a717f5b9ac0c9c175b8cfc4`; retained evidence `source-window-15`.

```text
#define TclUtfToUniChar(str, chPtr) \
	((((unsigned char) *(str)) < 0xC0) ?		\
	    ((*(chPtr) = (Tcl_UniChar) *(str)), 1)	\
	    : Tcl_UtfToUniChar(str, chPtr))

/*
 *----------------------------------------------------------------
 * Macro that encapsulates the logic that determines when it is safe to
 * interpret a string as a byte array directly. In summary, the object must be
 * a byte array and must not have a string representation (as the operations
 * that it is used in are defined on strings, not byte arrays). Theoretically
 * it is possible to also be efficient in the case where the object's bytes
 * field is filled by generation from the byte array (c.f. list canonicality)
 * but we don't do that at the moment since this is purely about efficiency.
 * The ANSI C "prototype" for this macro is:
 *
 * MODULE_SCOPE int	TclIsPureByteArray(Tcl_Obj *objPtr);
 *----------------------------------------------------------------
 */

#define TclIsPureByteArray(objPtr) \
	(((objPtr)->typePtr==&tclByteArrayType) && ((objPtr)->bytes==NULL))

/*
 *----------------------------------------------------------------
 * Macro used by the Tcl core to compare Unicode strings. On big-endian
 * systems we can use the more efficient memcmp, but this would not be
 * lexically correct on little-endian systems. The ANSI C "prototype" for
 * this macro is:
 *
 * MODULE_SCOPE int	TclUniCharNcmp(const Tcl_UniChar *cs,
 *			    const Tcl_UniChar *ct, unsigned long n);
 *----------------------------------------------------------------
 */

#ifdef WORDS_BIGENDIAN
#   define TclUniCharNcmp(cs,ct,n) memcmp((cs),(ct),(n)*sizeof(Tcl_UniChar))
#else /* !WORDS_BIGENDIAN */
#   define TclUniCharNcmp Tcl_UniCharNcmp

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `StringIndexCmd`, lines 1381–1423. Line unit: physical LF only. Original byte range [34868, 35899). Full-source SHA-256 `c2fe2deb95449c94c2719dca830d4e57ec4a436741199da39122d772888e849e`; snippet SHA-256 `8f56884c0c122c9c107a1fa785cda8e51ae78ff2e6d53c7aee8f19ab01789316`; retained evidence `source-window-16`.

```text
StringIndexCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int length, index;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string charIndex");
	return TCL_ERROR;
    }

    /*
     * Get the char length to calculate what 'end' means.
     */

    length = Tcl_GetCharLength(objv[1]);
    if (TclGetIntForIndexM(interp, objv[2], length-1, &index) != TCL_OK) {
	return TCL_ERROR;
    }

    if ((index >= 0) && (index < length)) {
	int ch = TclGetUCS4(objv[1], index);

	/*
	 * If we have a ByteArray object, we're careful to generate a new
	 * bytearray for a result.
	 */

	if (TclIsPureByteArray(objv[1])) {
	    unsigned char uch = UCHAR(ch);

	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(&uch, 1));
	} else {
	    char buf[8] = "";

	    length = TclUCS4ToUtf(ch, buf);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(buf, length));
	}
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `StringRangeCmd`, lines 2161–2196. Line unit: physical LF only. Original byte range [54494, 55397). Full-source SHA-256 `c2fe2deb95449c94c2719dca830d4e57ec4a436741199da39122d772888e849e`; snippet SHA-256 `3af80b13196f9690c9f503fcdebe658ebb4735233f43dedccf88577cdb366d7b`; retained evidence `source-window-17`.

```text
StringRangeCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int length, first, last;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last");
	return TCL_ERROR;
    }

    /*
     * Get the length in actual characters; Then reduce it by one because
     * 'end' refers to the last character, not one past it.
     */

    length = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], length, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], length, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    if (first < 0) {
	first = 0;
    }
    if (last >= length) {
	last = length;
    }
    if (last >= first) {
	Tcl_SetObjResult(interp, Tcl_GetRange(objv[1], first, last));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 515–570. Line unit: physical LF only. Original byte range [16001, 17254). Full-source SHA-256 `d5cae88e9008d6b9a6101f4c00d7e489fc66fb3d51c86b3b0eb13f7fd8e469b9`; snippet SHA-256 `eb4561c41cbc60ce9bc68c63fc943cca7d3545f31a682cdd9309336f2f43f0a4`; retained evidence `source-window-18`.

```text
Tcl_GetUniChar(
    Tcl_Obj *objPtr,		/* The object to get the Unicode charater
				 * from. */
    int index)			/* Get the index'th Unicode character. */
{
    String *stringPtr;

    if (index < 0) {
	return 0xFFFD;
    }

    /*
     * Optimize the case where we're really dealing with a ByteArray object
     * we don't need to convert to a string to perform the indexing operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	int length;
	unsigned char *bytes = Tcl_GetByteArrayFromObj(objPtr, &length);
	if (index >= length) {
		return 0xFFFD;
	}

	return bytes[index];
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == -1) {
	    TclNumUtfChars(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (index >= stringPtr->numChars) {
	    return 0xFFFD;
	}
	if (stringPtr->numChars == objPtr->length) {
	    return (unsigned char) objPtr->bytes[index];
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }

    if (index >= stringPtr->numChars) {
	return 0xFFFD;
    }
    return stringPtr->unicode[index];
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 741–829. Line unit: physical LF only. Original byte range [21606, 23971). Full-source SHA-256 `d5cae88e9008d6b9a6101f4c00d7e489fc66fb3d51c86b3b0eb13f7fd8e469b9`; snippet SHA-256 `3233dcf7e82eabc6d832cbd7cc7afc8e9f57871d1b08de1a46353eb9458dc80e`; retained evidence `source-window-19`.

```text
Tcl_GetRange(
    Tcl_Obj *objPtr,		/* The Tcl object to find the range of. */
    int first,			/* First index of the range. */
    int last)			/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to find the range of. */
    String *stringPtr;
    int length;

    if (first < 0) {
	first = 0;
    }

    /*
     * Optimize the case where we're really dealing with a bytearray object
     * we don't need to convert to a string to perform the substring operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	unsigned char *bytes = Tcl_GetByteArrayFromObj(objPtr, &length);

	if (last < 0 || last >= length) {
	    last = length - 1;
	}
	if (last < first) {
	    TclNewObj(newObjPtr);
	    return newObjPtr;
	}
	return Tcl_NewByteArrayObj(bytes + first, last - first + 1);
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == -1) {
	    TclNumUtfChars(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (stringPtr->numChars == objPtr->length) {
	    if (last < 0 || last >= stringPtr->numChars) {
		last = stringPtr->numChars - 1;
	    }
	    if (last < first) {
		TclNewObj(newObjPtr);
		return newObjPtr;
	    }
	    newObjPtr = Tcl_NewStringObj(objPtr->bytes + first, last - first + 1);

	    /*
	     * Since we know the char length of the result, store it.
	     */

	    SetStringFromAny(NULL, newObjPtr);
	    stringPtr = GET_STRING(newObjPtr);
	    stringPtr->numChars = newObjPtr->length;
	    return newObjPtr;
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }
    if (last < 0 || last >= stringPtr->numChars) {
	last = stringPtr->numChars - 1;
    }
    if (last < first) {
	TclNewObj(newObjPtr);
	return newObjPtr;
    }
#if TCL_UTF_MAX == 4
    /* See: bug [11ae2be95dac9417] */
    if ((first > 0) && ((stringPtr->unicode[first] & 0xFC00) == 0xDC00)
	    && ((stringPtr->unicode[first-1] & 0xFC00) == 0xD800)) {
	++first;
    }
    if ((last + 1 < stringPtr->numChars)
	    && ((stringPtr->unicode[last+1] & 0xFC00) == 0xDC00)
	    && ((stringPtr->unicode[last] & 0xFC00) == 0xD800)) {
	++last;
    }
#endif
    return Tcl_NewUnicodeObj(stringPtr->unicode + first, last - first + 1);
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 402–453. Line unit: physical LF only. Original byte range [13279, 14616). Full-source SHA-256 `d5cae88e9008d6b9a6101f4c00d7e489fc66fb3d51c86b3b0eb13f7fd8e469b9`; snippet SHA-256 `2eb391d5c09386d8ab2c714a78697dee02310bdded451a8cb9400f76c4c3d07f`; retained evidence `source-window-20`.

```text
Tcl_GetCharLength(
    Tcl_Obj *objPtr)		/* The String object to get the num chars
				 * of. */
{
    String *stringPtr;
    int numChars;

    /*
     * Quick, no-shimmer return for short string reps.
     */

    if ((objPtr->bytes) && (objPtr->length < 2)) {
	/* 0 bytes -> 0 chars; 1 byte -> 1 char */
	return objPtr->length;
    }

    /*
     * Optimize the case where we're really dealing with a bytearray object;
     * we don't need to convert to a string to perform the get-length operation.
     *
     * NOTE that we do not need the ByteArray to be "pure".  A ByteArray value
     * with a string rep cannot be trusted to represent the same value as the
     * string rep, but it *can* be trusted to have the same character length
     * as the string rep, which is all this routine cares about.
     */

    if (objPtr->typePtr == &tclByteArrayType) {
	int length;

	(void) Tcl_GetByteArrayFromObj(objPtr, &length);
	return length;
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);
    numChars = stringPtr->numChars;

    /*
     * If numChars is unknown, compute it.
     */

    if (numChars == -1) {
	TclNumUtfChars(numChars, objPtr->bytes, objPtr->length);
	stringPtr->numChars = numChars;

    }
    return numChars;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 371–382. Line unit: physical LF only. Original byte range [12542, 12850). Full-source SHA-256 `d5cae88e9008d6b9a6101f4c00d7e489fc66fb3d51c86b3b0eb13f7fd8e469b9`; snippet SHA-256 `5106fe8044b31c6e7e3f121b0787b6fd7dc564e2b5f60cf07f18d4c0a05fb007`; retained evidence `source-window-21`.

```text
Tcl_NewUnicodeObj(
    const Tcl_UniChar *unicode,	/* The unicode string used to initialize the
				 * new object. */
    int numChars)		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclUtil.c`, function `TclGetIntForIndex`, lines 3634–3714. Line unit: physical LF only. Original byte range [101826, 103787). Full-source SHA-256 `452b96a108330de9bf79c52a6266634127b468b704512ffb93e0dc66685df372`; snippet SHA-256 `4afd7808861ad1c1108aded9122cad8a8a0879b96160adb8579cdf9d8b222812`; retained evidence `source-window-22`.

```text
TclGetIntForIndex(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting. If
				 * NULL, then no error message is left after
				 * errors. */
    Tcl_Obj *objPtr,		/* Points to an object containing either "end"
				 * or an integer. */
    int endValue,		/* The value to be stored at "indexPtr" if
				 * "objPtr" holds "end". */
    int *indexPtr)		/* Location filled in with an integer
				 * representing an index. */
{
    int length;
    char *opPtr;
    const char *bytes;

    if (TclGetIntFromObj(NULL, objPtr, indexPtr) == TCL_OK) {
	return TCL_OK;
    }

    if (GetEndOffsetFromObj(objPtr, endValue, indexPtr) == TCL_OK) {
	return TCL_OK;
    }

    bytes = TclGetStringFromObj(objPtr, &length);

    /*
     * Leading whitespace is acceptable in an index.
     */

    while (length && TclIsSpaceProcM(*bytes)) {
	bytes++;
	length--;
    }

    if (TclParseNumber(NULL, NULL, NULL, bytes, length, (const char **)&opPtr,
	    TCL_PARSE_INTEGER_ONLY | TCL_PARSE_NO_WHITESPACE) == TCL_OK) {
	int code, first, second;
	char savedOp = *opPtr;

	if ((savedOp != '+') && (savedOp != '-')) {
	    goto parseError;
	}
	if (TclIsSpaceProcM(opPtr[1])) {
	    goto parseError;
	}
	*opPtr = '\0';
	code = Tcl_GetInt(interp, bytes, &first);
	*opPtr = savedOp;
	if (code == TCL_ERROR) {
	    goto parseError;
	}
	if (TCL_ERROR == Tcl_GetInt(interp, opPtr+1, &second)) {
	    goto parseError;
	}
	if (savedOp == '+') {
	    *indexPtr = first + second;
	} else {
	    *indexPtr = first - second;
	}
	return TCL_OK;
    }

    /*
     * Report a parse error.
     */

  parseError:
    if (interp != NULL) {
	bytes = TclGetString(objPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"bad index \"%s\": must be integer?[+-]integer? or"
		" end?[+-]integer?", bytes));
	if (!strncmp(bytes, "end-", 4)) {
	    bytes += 4;
	}
	TclCheckBadOctal(interp, bytes);
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", (char *)NULL);
    }

    return TCL_ERROR;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclUtil.c`, function `GetEndOffsetFromObj`, lines 3772–3786. Line unit: physical LF only. Original byte range [105138, 105708). Full-source SHA-256 `452b96a108330de9bf79c52a6266634127b468b704512ffb93e0dc66685df372`; snippet SHA-256 `68ea1d9b31b30e07aa171f045c4144ce5be95da386aae27f5e8e5ecb2f489036`; retained evidence `source-window-23`.

```text
GetEndOffsetFromObj(
    Tcl_Obj *objPtr,            /* Pointer to the object to parse */
    int endValue,               /* The value to be stored at "indexPtr" if
                                 * "objPtr" holds "end". */
    int *indexPtr)              /* Location filled in with an integer
                                 * representing an index. */
{
    if (SetEndOffsetFromAny(NULL, objPtr) != TCL_OK) {
	return TCL_ERROR;
    }

    /* TODO: Handle overflow cases sensibly */
    *indexPtr = endValue + (int)objPtr->internalRep.longValue;
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `StringIndexCmd`, lines 1399–1445. Line unit: physical LF only. Original byte range [36397, 37452). Full-source SHA-256 `080e201d1a09760aad4fedd9835e2091fd663d405d32d034f32dd0918d9006cc`; snippet SHA-256 `90cef86ead0e37d95bddb09fb40076f39feac97f6ed36db6453e1bf6c9ea84cc`; retained evidence `source-window-24`.

```text
StringIndexCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Size index, end;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string charIndex");
	return TCL_ERROR;
    }

    /*
     * Get the char length to calculate what 'end' means.
     */

    end = Tcl_GetCharLength(objv[1]) - 1;
    if (TclGetIntForIndexM(interp, objv[2], end, &index) != TCL_OK) {
	return TCL_ERROR;
    }

    if ((index >= 0) && (index <= end)) {
	int ch = Tcl_GetUniChar(objv[1], index);

	if (ch == -1) {
	    return TCL_OK;
	}

	/*
	 * If we have a ByteArray object, we're careful to generate a new
	 * bytearray for a result.
	 */

	if (TclIsPureByteArray(objv[1])) {
	    unsigned char uch = UCHAR(ch);

	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(&uch, 1));
	} else {
	    char buf[4] = "";

	    end = Tcl_UniCharToUtf(ch, buf);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(buf, end));
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `StringRangeCmd`, lines 2286–2315. Line unit: physical LF only. Original byte range [58732, 59522). Full-source SHA-256 `080e201d1a09760aad4fedd9835e2091fd663d405d32d034f32dd0918d9006cc`; snippet SHA-256 `878a80fbc714c3a804e3cd310890e541f7f67c29dcaa18d9f4a38c7bfdaba894`; retained evidence `source-window-25`.

```text
StringRangeCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Size first, last, end;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last");
	return TCL_ERROR;
    }

    /*
     * Get the length in actual characters; Then reduce it by one because
     * 'end' refers to the last character, not one past it.
     */

    end = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    if (last >= 0) {
	Tcl_SetObjResult(interp, Tcl_GetRange(objv[1], first, last));
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 532–589. Line unit: physical LF only. Original byte range [16892, 18182). Full-source SHA-256 `d44c80d7de637da41ac841c6c55c25c4ee302efc774c2c4cfe5d8ff73193b7b4`; snippet SHA-256 `0adf65b1842f583608923976258c985d80a5229ad2257b604978082a4e579e7c`; retained evidence `source-window-26`.

```text
Tcl_GetUniChar(
    Tcl_Obj *objPtr,		/* The object to get the Unicode charater
				 * from. */
    Tcl_Size index)		/* Get the index'th Unicode character. */
{
    String *stringPtr;
    int ch;

    if (index < 0) {
	return -1;
    }

    /*
     * Optimize the case where we're really dealing with a ByteArray object
     * we don't need to convert to a string to perform the indexing operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	Tcl_Size length = 0;
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &length);
	if (index >= length) {
		return -1;
	}

	return bytes[index];
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == TCL_INDEX_NONE) {
	    TclNumUtfCharsM(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (index >= stringPtr->numChars) {
	    return -1;
	}
	if (stringPtr->numChars == objPtr->length) {
	    return (unsigned char) objPtr->bytes[index];
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }

    if (index >= stringPtr->numChars) {
	return -1;
    }
    ch = stringPtr->unicode[index];
    return ch;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 725–802. Line unit: physical LF only. Original byte range [21719, 23746). Full-source SHA-256 `d44c80d7de637da41ac841c6c55c25c4ee302efc774c2c4cfe5d8ff73193b7b4`; snippet SHA-256 `d36a3c13a434922d1818009f1e44331371e6af564d609516708f5db2e60ff251`; retained evidence `source-window-27`.

```text
Tcl_GetRange(
    Tcl_Obj *objPtr,		/* The Tcl object to find the range of. */
    Tcl_Size first,		/* First index of the range. */
    Tcl_Size last)		/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to return that is the new
				 * range. */
    String *stringPtr;
    Tcl_Size length = 0;

    if (first < 0) {
	first = 0;
    }

    /*
     * Optimize the case where we're really dealing with a byte-array object
     * we don't need to convert to a string to perform the substring operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &length);

	if (last < 0 || last >= length) {
	    last = length - 1;
	}
	if (last < first) {
	    TclNewObj(newObjPtr);
	    return newObjPtr;
	}
	return Tcl_NewByteArrayObj(bytes + first, last - first + 1);
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == TCL_INDEX_NONE) {
	    TclNumUtfCharsM(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (stringPtr->numChars == objPtr->length) {
	    if (last < 0 || last >= stringPtr->numChars) {
		last = stringPtr->numChars - 1;
	    }
	    if (last < first) {
		TclNewObj(newObjPtr);
		return newObjPtr;
	    }
	    newObjPtr = Tcl_NewStringObj(objPtr->bytes + first, last - first + 1);

	    /*
	     * Since we know the char length of the result, store it.
	     */

	    SetStringFromAny(NULL, newObjPtr);
	    stringPtr = GET_STRING(newObjPtr);
	    stringPtr->numChars = newObjPtr->length;
	    return newObjPtr;
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }
    if (last < 0 || last >= stringPtr->numChars) {
	last = stringPtr->numChars - 1;
    }
    if (last < first) {
	TclNewObj(newObjPtr);
	return newObjPtr;
    }
    return Tcl_NewUnicodeObj(stringPtr->unicode + first, last - first + 1);
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 378–427. Line unit: physical LF only. Original byte range [12857, 14260). Full-source SHA-256 `d44c80d7de637da41ac841c6c55c25c4ee302efc774c2c4cfe5d8ff73193b7b4`; snippet SHA-256 `c6abedf2bf8abac93b0289e8d2031f2092f8ebbb3d47eb1807f9a63ad7928598`; retained evidence `source-window-28`.

```text
Tcl_GetCharLength(
    Tcl_Obj *objPtr)		/* The String object to get the num chars
				 * of. */
{
    String *stringPtr;
    Tcl_Size numChars = 0;

    /*
     * Quick, no-shimmer return for short string reps.
     */

    if ((objPtr->bytes) && (objPtr->length < 2)) {
	/* 0 bytes -> 0 chars; 1 byte -> 1 char */
	return objPtr->length;
    }

    /*
     * Optimize the case where we're really dealing with a byte-array object;
     * we don't need to convert to a string to perform the get-length operation.
     *
     * Starting in Tcl 9.0, we check for a "pure" byte-array, because the
     * machinery behind that test is using a proper byte-array ObjType.  We
     * could also compute length of an improper byte-array without shimmering
     * but there's no value in that. We *want* to shimmer an improper byte-array
     * because improper byte-arrays have worthless internal reps.
     */

    if (TclIsPureByteArray(objPtr)) {
	(void) Tcl_GetBytesFromObj(NULL, objPtr, &numChars);
	return numChars;
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);
    numChars = stringPtr->numChars;

    /*
     * If numChars is unknown, compute it.
     */

    if (numChars < 0) {
	TclNumUtfCharsM(numChars, objPtr->bytes, objPtr->length);
	stringPtr->numChars = numChars;
    }
    return numChars;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 347–358. Line unit: physical LF only. Original byte range [12110, 12423). Full-source SHA-256 `d44c80d7de637da41ac841c6c55c25c4ee302efc774c2c4cfe5d8ff73193b7b4`; snippet SHA-256 `46108912ae8807a53ab4e976f0544a4cb283f328400dfd84fc84519e26911dd5`; retained evidence `source-window-29`.

```text
Tcl_NewUnicodeObj(
    const Tcl_UniChar *unicode,	/* The unicode string used to initialize the
				 * new object. */
    Tcl_Size numChars)		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclUtil.c`, function `Tcl_GetIntForIndex`, lines 3470–3500. Line unit: physical LF only. Original byte range [97702, 98838). Full-source SHA-256 `13df44c0c262d3445b5c5db138d14c6e45e4993b076387f6e4f272f89d62bcf4`; snippet SHA-256 `78756b3cdf9bac765d88a74ef22ef2de5f69fe66e3db11c68dd4a89c7f548a57`; retained evidence `source-window-30`.

```text
Tcl_GetIntForIndex(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting. If
				 * NULL, then no error message is left after
				 * errors. */
    Tcl_Obj *objPtr,		/* Points to an object containing either "end"
				 * or an integer. */
    Tcl_Size endValue,		/* The value corresponding to the "end" index */
    Tcl_Size *indexPtr)		/* Location filled in with an integer
				 * representing an index. May be NULL.*/
{
    Tcl_WideInt wide;

    if (GetWideForIndex(interp, objPtr, endValue, &wide) == TCL_ERROR) {
	return TCL_ERROR;
    }
    if (indexPtr != NULL) {
	/* Note: check against TCL_SIZE_MAX needed for 32-bit builds */
	if (wide >= 0 && wide <= TCL_SIZE_MAX) {
	    *indexPtr = (Tcl_Size)wide; /* A valid index */
	} else if (wide > TCL_SIZE_MAX) {
	    *indexPtr = TCL_SIZE_MAX;   /* Beyond max possible index */
	} else if (wide < -1-TCL_SIZE_MAX) {
	    *indexPtr = -1-TCL_SIZE_MAX; /* Below most negative index */
	} else if ((wide < 0) && (endValue >= 0)) {
	    *indexPtr = TCL_INDEX_NONE; /* No clue why this special case */
	} else {
	    *indexPtr = (Tcl_Size) wide;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclUtil.c`, function `GetEndOffsetFromObj`, lines 3532–3770. Line unit: physical LF only. Original byte range [99708, 106500). Full-source SHA-256 `13df44c0c262d3445b5c5db138d14c6e45e4993b076387f6e4f272f89d62bcf4`; snippet SHA-256 `8b9c41fda6225b676a897a3b0bc631e4a523655bd99859c8ec877d94fa7d18b1`; retained evidence `source-window-31`.

```text
GetEndOffsetFromObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr,		/* Pointer to the object to parse */
    Tcl_WideInt endValue,	/* The value to be stored at "widePtr" if
				 * "objPtr" holds "end". */
    Tcl_WideInt *widePtr)	/* Location filled in with an integer
				 * representing an index. */
{
    Tcl_ObjInternalRep *irPtr;
    Tcl_WideInt offset = -1;	/* Offset in the "end-offset" expression - 1 */
    void *cd;

    while ((irPtr = TclFetchInternalRep(objPtr, &endOffsetType)) == NULL) {
	Tcl_ObjInternalRep ir;
	Tcl_Size length;
	const char *bytes = TclGetStringFromObj(objPtr, &length);

	if (*bytes != 'e') {
	    int numType;
	    const char *opPtr;
	    int t1 = 0, t2 = 0;

	    /* Value doesn't start with "e" */

	    /* If we reach here, the string rep of objPtr exists. */

	    /*
	     * The valid index syntax does not include any value that is
	     * a list of more than one element. This is necessary so that
	     * lists of index values can be reliably distinguished from any
	     * single index value.
	     */

	    /*
	     * Quick scan to see if multi-value list is even possible.
	     * This relies on TclGetString() returning a NUL-terminated string.
	     */
	    if ((TclMaxListLength(bytes, TCL_INDEX_NONE, NULL) > 1)
		    /* If it's possible, do the full list parse. */
		    && (TCL_OK == TclListObjLength(NULL, objPtr, &length))
		    && (length > 1)) {
		goto parseError;
	    }

	    /* Passed the list screen, so parse for index arithmetic expression */
	    if (TCL_OK == TclParseNumber(NULL, objPtr, NULL, NULL, TCL_INDEX_NONE, &opPtr,
		    TCL_PARSE_INTEGER_ONLY)) {
		Tcl_WideInt w1=0, w2=0;

		/* value starts with valid integer... */

		if ((*opPtr == '-') || (*opPtr == '+')) {
		    /* ... value continues with [-+] ... */

		    /* Save first integer as wide if possible */
		    Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t1);
		    if (t1 == TCL_NUMBER_INT) {
			w1 = (*(Tcl_WideInt *)cd);
		    }

		    if (TCL_OK == TclParseNumber(NULL, objPtr, NULL, opPtr + 1,
			    TCL_INDEX_NONE, NULL, TCL_PARSE_INTEGER_ONLY)) {
			/* ... value concludes with second valid integer */

			/* Save second integer as wide if possible */
			Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t2);
			if (t2 == TCL_NUMBER_INT) {
			    w2 = (*(Tcl_WideInt *)cd);
			}
		    }
		}
		/* Clear invalid internalreps left by TclParseNumber */
		TclFreeInternalRep(objPtr);

		if (t1 && t2) {
		    /* We have both integer values */
		    if ((t1 == TCL_NUMBER_INT) && (t2 == TCL_NUMBER_INT)) {
			/* Both are wide, do wide-integer math */
			if (*opPtr == '-') {
			    if (w2 == WIDE_MIN) {
				goto extreme;
			    }
			    w2 = -w2;
			}

			if ((w1 ^ w2) < 0) {
			    /* Different signs, sum cannot overflow */
			    offset = w1 + w2;
			} else if (w1 >= 0) {
			    if (w1 < WIDE_MAX - w2) {
				offset = w1 + w2;
			    } else {
				offset = WIDE_MAX;
			    }
			} else {
			    if (w1 > WIDE_MIN - w2) {
				offset = w1 + w2;
			    } else {
				offset = WIDE_MIN;
			    }
			}
		    } else {
			/*
			 * At least one is big, do bignum math. Little reason to
			 * value performance here. Re-use code.  Parse has verified
			 * objPtr is an expression. Compute it.
			 */

			Tcl_Obj *sum;

		    extreme:
			if (interp) {
			    Tcl_ExprObj(interp, objPtr, &sum);
			} else {
			    Tcl_Interp *compute = Tcl_CreateInterp();
			    Tcl_ExprObj(compute, objPtr, &sum);
			    Tcl_DeleteInterp(compute);
			}
			Tcl_GetNumberFromObj(NULL, sum, &cd, &numType);

			if (numType == TCL_NUMBER_INT) {
			    /* sum holds an integer in the signed wide range */
			    offset = *(Tcl_WideInt *)cd;
			} else {
			    /* sum holds an integer outside the signed wide range */
			    /* Truncate to the signed wide range. */
			    if (mp_isneg((mp_int *)cd)) {
				offset = WIDE_MIN;
			    } else {
				offset = WIDE_MAX;
			    }
			}
			Tcl_DecrRefCount(sum);
		    }
		    if (offset < 0) {
			offset = (offset == -1) ? WIDE_MIN : WIDE_MIN+1;
		    }
		    goto parseOK;
		}
	    }
	    goto parseError;
	}

	if ((length < 3) || (length == 4) || (strncmp(bytes, "end", 3) != 0)) {
	    /* Doesn't start with "end" */
	    goto parseError;
	}
	if (length > 4) {
	    int t;

	    /* Parse for the "end-..." or "end+..." formats */

	    if ((bytes[3] != '-') && (bytes[3] != '+')) {
		/* No operator where we need one */
		goto parseError;
	    }
	    if (TclIsSpaceProc(bytes[4])) {
		/* Space after + or - not permitted. */
		goto parseError;
	    }

	    /* Parse the integer offset */
	    if (TCL_OK != TclParseNumber(NULL, objPtr, NULL,
		    bytes + 4, length - 4, NULL, TCL_PARSE_INTEGER_ONLY)) {
		/* Not a recognized integer format */
		goto parseError;
	    }

	    /* Got an integer offset; pull it from where parser left it. */
	    Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t);

	    if (t == TCL_NUMBER_BIG) {
		/* Truncate to the signed wide range. */
		if (mp_isneg((mp_int *)cd)) {
		    offset = (bytes[3] == '-') ? WIDE_MAX : WIDE_MIN;
		} else {
		    offset = (bytes[3] == '-') ? WIDE_MIN : WIDE_MAX;
		}
	    } else {
		/* assert (t == TCL_NUMBER_INT); */
		offset = (*(Tcl_WideInt *)cd);
		if (bytes[3] == '-') {
		    offset = (offset == WIDE_MIN) ? WIDE_MAX : -offset;
		}
		if (offset == 1) {
		    offset = WIDE_MAX; /* "end+1" */
		} else if (offset > 1) {
		    offset = WIDE_MAX - 1; /* "end+n", out of range */
		} else if (offset != WIDE_MIN) {
		    offset--;
		}
	    }
	}

    parseOK:
	/* Success. Store the new internal rep. */
	ir.wideValue = offset;
	Tcl_StoreInternalRep(objPtr, &endOffsetType, &ir);
    }

    offset = irPtr->wideValue;

    if (offset == WIDE_MAX) {
	/*
	 * Encodes end+1. This is distinguished from end+n as noted
	 * in function header.
	 * NOTE: this may wrap around if the caller passes (as lset does)
	 * listLen-1 as endValue and listLen is 0. The -1 will be
	 * interpreted as FF...FF and adding 1 will result in 0 which
	 * is what we want. Callers like lset which pass in listLen-1 == -1
	 * as endValue will have to adjust accordingly.
	 */
	*widePtr = (endValue == -1) ? WIDE_MAX : endValue + 1;
    } else if (offset == WIDE_MIN) {
	*widePtr = (endValue == -1) ? WIDE_MIN : -1;
    } else if (offset < 0) {
	/* end-(n-1) - Different signs, sum cannot overflow */
	*widePtr = endValue + offset + 1;
    } else {
	/* 0:WIDE_MAX - plain old index. */
	*widePtr = offset;
    }
    return TCL_OK;

    /* Report a parse error. */
  parseError:
    if (interp != NULL) {
	char * bytes = TclGetString(objPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"bad index \"%s\": must be integer?[+-]integer? or"
		" end?[+-]integer?", bytes));
	if (!strncmp(bytes, "end-", 4)) {
	    bytes += 4;
	}
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", (char *)NULL);
    }

    return TCL_ERROR;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `StringIndexCmd`, lines 1464–1510. Line unit: physical LF only. Original byte range [39626, 40684). Full-source SHA-256 `62be408d1faa29a35d3e30eacc4986fd9f1a973f7efcd474b3a2a9082ded512f`; snippet SHA-256 `221469366523bdb3c10889585247e221941606b04fde1952a008d2521fb39e96`; retained evidence `source-window-32`.

```text
StringIndexCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size index, end;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string charIndex");
	return TCL_ERROR;
    }

    /*
     * Get the char length to calculate what 'end' means.
     */

    end = Tcl_GetCharLength(objv[1]) - 1;
    if (TclGetIntForIndexM(interp, objv[2], end, &index) != TCL_OK) {
	return TCL_ERROR;
    }

    if ((index >= 0) && (index <= end)) {
	int ch = Tcl_GetUniChar(objv[1], index);

	if (ch == -1) {
	    return TCL_OK;
	}

	/*
	 * If we have a ByteArray object, we're careful to generate a new
	 * bytearray for a result.
	 */

	if (TclIsPureByteArray(objv[1])) {
	    unsigned char uch = UCHAR(ch);

	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(&uch, 1));
	} else {
	    char buf[4] = "";

	    end = Tcl_UniCharToUtf(ch, buf);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(buf, end));
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `StringRangeCmd`, lines 2351–2380. Line unit: physical LF only. Original byte range [61981, 62774). Full-source SHA-256 `62be408d1faa29a35d3e30eacc4986fd9f1a973f7efcd474b3a2a9082ded512f`; snippet SHA-256 `0967cdbc006e0fcfb4d8e977f99a5a8b9e9c287d4bc8e623c9a52ac941f9884f`; retained evidence `source-window-33`.

```text
StringRangeCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size first, last, end;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last");
	return TCL_ERROR;
    }

    /*
     * Get the length in actual characters; Then reduce it by one because
     * 'end' refers to the last character, not one past it.
     */

    end = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    if (last >= 0) {
	Tcl_SetObjResult(interp, Tcl_GetRange(objv[1], first, last));
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 535–592. Line unit: physical LF only. Original byte range [16905, 18198). Full-source SHA-256 `d1ecd65e375cee3b1ab86111cf5b90fb6bf4d7b5e71448cd361e4026d32a4bf8`; snippet SHA-256 `461be8e0a8670fac6d1dd2515e8804d1c791026bfe54a177b9781b75fd14af4a`; retained evidence `source-window-34`.

```text
Tcl_GetUniChar(
    Tcl_Obj *objPtr,		/* The object to get the Unicode charater
				 * from. */
    Tcl_Size index)		/* Get the index'th Unicode character. */
{
    String *stringPtr;
    int ch;

    if (index < 0) {
	return -1;
    }

    /*
     * Optimize the case where we're really dealing with a ByteArray object
     * we don't need to convert to a string to perform the indexing operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	Tcl_Size length = 0;
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &length);
	if (index >= length) {
	    return -1;
	}

	return bytes[index];
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == TCL_INDEX_NONE) {
	    TclNumUtfCharsM(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (index >= stringPtr->numChars) {
	    return -1;
	}
	if (stringPtr->numChars == objPtr->length) {
	    return (unsigned char) objPtr->bytes[index];
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }

    if (index >= stringPtr->numChars) {
	return -1;
    }
    ch = stringPtr->unicode[index];
    return ch;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 728–805. Line unit: physical LF only. Original byte range [21720, 23747). Full-source SHA-256 `d1ecd65e375cee3b1ab86111cf5b90fb6bf4d7b5e71448cd361e4026d32a4bf8`; snippet SHA-256 `d36a3c13a434922d1818009f1e44331371e6af564d609516708f5db2e60ff251`; retained evidence `source-window-35`.

```text
Tcl_GetRange(
    Tcl_Obj *objPtr,		/* The Tcl object to find the range of. */
    Tcl_Size first,		/* First index of the range. */
    Tcl_Size last)		/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to return that is the new
				 * range. */
    String *stringPtr;
    Tcl_Size length = 0;

    if (first < 0) {
	first = 0;
    }

    /*
     * Optimize the case where we're really dealing with a byte-array object
     * we don't need to convert to a string to perform the substring operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &length);

	if (last < 0 || last >= length) {
	    last = length - 1;
	}
	if (last < first) {
	    TclNewObj(newObjPtr);
	    return newObjPtr;
	}
	return Tcl_NewByteArrayObj(bytes + first, last - first + 1);
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == TCL_INDEX_NONE) {
	    TclNumUtfCharsM(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (stringPtr->numChars == objPtr->length) {
	    if (last < 0 || last >= stringPtr->numChars) {
		last = stringPtr->numChars - 1;
	    }
	    if (last < first) {
		TclNewObj(newObjPtr);
		return newObjPtr;
	    }
	    newObjPtr = Tcl_NewStringObj(objPtr->bytes + first, last - first + 1);

	    /*
	     * Since we know the char length of the result, store it.
	     */

	    SetStringFromAny(NULL, newObjPtr);
	    stringPtr = GET_STRING(newObjPtr);
	    stringPtr->numChars = newObjPtr->length;
	    return newObjPtr;
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }
    if (last < 0 || last >= stringPtr->numChars) {
	last = stringPtr->numChars - 1;
    }
    if (last < first) {
	TclNewObj(newObjPtr);
	return newObjPtr;
    }
    return Tcl_NewUnicodeObj(stringPtr->unicode + first, last - first + 1);
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 381–430. Line unit: physical LF only. Original byte range [12869, 14272). Full-source SHA-256 `d1ecd65e375cee3b1ab86111cf5b90fb6bf4d7b5e71448cd361e4026d32a4bf8`; snippet SHA-256 `c6abedf2bf8abac93b0289e8d2031f2092f8ebbb3d47eb1807f9a63ad7928598`; retained evidence `source-window-36`.

```text
Tcl_GetCharLength(
    Tcl_Obj *objPtr)		/* The String object to get the num chars
				 * of. */
{
    String *stringPtr;
    Tcl_Size numChars = 0;

    /*
     * Quick, no-shimmer return for short string reps.
     */

    if ((objPtr->bytes) && (objPtr->length < 2)) {
	/* 0 bytes -> 0 chars; 1 byte -> 1 char */
	return objPtr->length;
    }

    /*
     * Optimize the case where we're really dealing with a byte-array object;
     * we don't need to convert to a string to perform the get-length operation.
     *
     * Starting in Tcl 9.0, we check for a "pure" byte-array, because the
     * machinery behind that test is using a proper byte-array ObjType.  We
     * could also compute length of an improper byte-array without shimmering
     * but there's no value in that. We *want* to shimmer an improper byte-array
     * because improper byte-arrays have worthless internal reps.
     */

    if (TclIsPureByteArray(objPtr)) {
	(void) Tcl_GetBytesFromObj(NULL, objPtr, &numChars);
	return numChars;
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);
    numChars = stringPtr->numChars;

    /*
     * If numChars is unknown, compute it.
     */

    if (numChars < 0) {
	TclNumUtfCharsM(numChars, objPtr->bytes, objPtr->length);
	stringPtr->numChars = numChars;
    }
    return numChars;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 350–361. Line unit: physical LF only. Original byte range [12122, 12435). Full-source SHA-256 `d1ecd65e375cee3b1ab86111cf5b90fb6bf4d7b5e71448cd361e4026d32a4bf8`; snippet SHA-256 `46108912ae8807a53ab4e976f0544a4cb283f328400dfd84fc84519e26911dd5`; retained evidence `source-window-37`.

```text
Tcl_NewUnicodeObj(
    const Tcl_UniChar *unicode,	/* The unicode string used to initialize the
				 * new object. */
    Tcl_Size numChars)		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclUtil.c`, function `Tcl_GetIntForIndex`, lines 3535–3565. Line unit: physical LF only. Original byte range [100090, 101224). Full-source SHA-256 `509c893e8aef8f1ae70d0d1e279bbefc60baa758a56d1be184421794f6868e64`; snippet SHA-256 `8213f51c4bcb466aa11daa1bf3d4ebb4ad64bbb722df17d78952db2224fc3f69`; retained evidence `source-window-38`.

```text
Tcl_GetIntForIndex(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting. If
				 * NULL, then no error message is left after
				 * errors. */
    Tcl_Obj *objPtr,		/* Points to an object containing either "end"
				 * or an integer. */
    Tcl_Size endValue,		/* The value corresponding to the "end" index */
    Tcl_Size *indexPtr)		/* Location filled in with an integer
				 * representing an index. May be NULL.*/
{
    Tcl_WideInt wide;

    if (GetWideForIndex(interp, objPtr, endValue, &wide) == TCL_ERROR) {
	return TCL_ERROR;
    }
    if (indexPtr != NULL) {
	/* Note: check against TCL_SIZE_MAX needed for 32-bit builds */
	if (wide >= 0 && wide <= TCL_SIZE_MAX) {
	    *indexPtr = (Tcl_Size)wide;	/* A valid index */
	} else if (wide > TCL_SIZE_MAX) {
	    *indexPtr = TCL_SIZE_MAX;	/* Beyond max possible index */
	} else if (wide < -1-TCL_SIZE_MAX) {
	    *indexPtr = -1-TCL_SIZE_MAX; /* Below most negative index */
	} else if ((wide < 0) && (endValue >= 0)) {
	    *indexPtr = TCL_INDEX_NONE;	/* No clue why this special case */
	} else {
	    *indexPtr = (Tcl_Size) wide;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclUtil.c`, function `GetEndOffsetFromObj`, lines 3597–3835. Line unit: physical LF only. Original byte range [102089, 108881). Full-source SHA-256 `509c893e8aef8f1ae70d0d1e279bbefc60baa758a56d1be184421794f6868e64`; snippet SHA-256 `8b9c41fda6225b676a897a3b0bc631e4a523655bd99859c8ec877d94fa7d18b1`; retained evidence `source-window-39`.

```text
GetEndOffsetFromObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr,		/* Pointer to the object to parse */
    Tcl_WideInt endValue,	/* The value to be stored at "widePtr" if
				 * "objPtr" holds "end". */
    Tcl_WideInt *widePtr)	/* Location filled in with an integer
				 * representing an index. */
{
    Tcl_ObjInternalRep *irPtr;
    Tcl_WideInt offset = -1;	/* Offset in the "end-offset" expression - 1 */
    void *cd;

    while ((irPtr = TclFetchInternalRep(objPtr, &endOffsetType)) == NULL) {
	Tcl_ObjInternalRep ir;
	Tcl_Size length;
	const char *bytes = TclGetStringFromObj(objPtr, &length);

	if (*bytes != 'e') {
	    int numType;
	    const char *opPtr;
	    int t1 = 0, t2 = 0;

	    /* Value doesn't start with "e" */

	    /* If we reach here, the string rep of objPtr exists. */

	    /*
	     * The valid index syntax does not include any value that is
	     * a list of more than one element. This is necessary so that
	     * lists of index values can be reliably distinguished from any
	     * single index value.
	     */

	    /*
	     * Quick scan to see if multi-value list is even possible.
	     * This relies on TclGetString() returning a NUL-terminated string.
	     */
	    if ((TclMaxListLength(bytes, TCL_INDEX_NONE, NULL) > 1)
		    /* If it's possible, do the full list parse. */
		    && (TCL_OK == TclListObjLength(NULL, objPtr, &length))
		    && (length > 1)) {
		goto parseError;
	    }

	    /* Passed the list screen, so parse for index arithmetic expression */
	    if (TCL_OK == TclParseNumber(NULL, objPtr, NULL, NULL, TCL_INDEX_NONE, &opPtr,
		    TCL_PARSE_INTEGER_ONLY)) {
		Tcl_WideInt w1=0, w2=0;

		/* value starts with valid integer... */

		if ((*opPtr == '-') || (*opPtr == '+')) {
		    /* ... value continues with [-+] ... */

		    /* Save first integer as wide if possible */
		    Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t1);
		    if (t1 == TCL_NUMBER_INT) {
			w1 = (*(Tcl_WideInt *)cd);
		    }

		    if (TCL_OK == TclParseNumber(NULL, objPtr, NULL, opPtr + 1,
			    TCL_INDEX_NONE, NULL, TCL_PARSE_INTEGER_ONLY)) {
			/* ... value concludes with second valid integer */

			/* Save second integer as wide if possible */
			Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t2);
			if (t2 == TCL_NUMBER_INT) {
			    w2 = (*(Tcl_WideInt *)cd);
			}
		    }
		}
		/* Clear invalid internalreps left by TclParseNumber */
		TclFreeInternalRep(objPtr);

		if (t1 && t2) {
		    /* We have both integer values */
		    if ((t1 == TCL_NUMBER_INT) && (t2 == TCL_NUMBER_INT)) {
			/* Both are wide, do wide-integer math */
			if (*opPtr == '-') {
			    if (w2 == WIDE_MIN) {
				goto extreme;
			    }
			    w2 = -w2;
			}

			if ((w1 ^ w2) < 0) {
			    /* Different signs, sum cannot overflow */
			    offset = w1 + w2;
			} else if (w1 >= 0) {
			    if (w1 < WIDE_MAX - w2) {
				offset = w1 + w2;
			    } else {
				offset = WIDE_MAX;
			    }
			} else {
			    if (w1 > WIDE_MIN - w2) {
				offset = w1 + w2;
			    } else {
				offset = WIDE_MIN;
			    }
			}
		    } else {
			/*
			 * At least one is big, do bignum math. Little reason to
			 * value performance here. Re-use code.  Parse has verified
			 * objPtr is an expression. Compute it.
			 */

			Tcl_Obj *sum;

		    extreme:
			if (interp) {
			    Tcl_ExprObj(interp, objPtr, &sum);
			} else {
			    Tcl_Interp *compute = Tcl_CreateInterp();
			    Tcl_ExprObj(compute, objPtr, &sum);
			    Tcl_DeleteInterp(compute);
			}
			Tcl_GetNumberFromObj(NULL, sum, &cd, &numType);

			if (numType == TCL_NUMBER_INT) {
			    /* sum holds an integer in the signed wide range */
			    offset = *(Tcl_WideInt *)cd;
			} else {
			    /* sum holds an integer outside the signed wide range */
			    /* Truncate to the signed wide range. */
			    if (mp_isneg((mp_int *)cd)) {
				offset = WIDE_MIN;
			    } else {
				offset = WIDE_MAX;
			    }
			}
			Tcl_DecrRefCount(sum);
		    }
		    if (offset < 0) {
			offset = (offset == -1) ? WIDE_MIN : WIDE_MIN+1;
		    }
		    goto parseOK;
		}
	    }
	    goto parseError;
	}

	if ((length < 3) || (length == 4) || (strncmp(bytes, "end", 3) != 0)) {
	    /* Doesn't start with "end" */
	    goto parseError;
	}
	if (length > 4) {
	    int t;

	    /* Parse for the "end-..." or "end+..." formats */

	    if ((bytes[3] != '-') && (bytes[3] != '+')) {
		/* No operator where we need one */
		goto parseError;
	    }
	    if (TclIsSpaceProc(bytes[4])) {
		/* Space after + or - not permitted. */
		goto parseError;
	    }

	    /* Parse the integer offset */
	    if (TCL_OK != TclParseNumber(NULL, objPtr, NULL,
		    bytes + 4, length - 4, NULL, TCL_PARSE_INTEGER_ONLY)) {
		/* Not a recognized integer format */
		goto parseError;
	    }

	    /* Got an integer offset; pull it from where parser left it. */
	    Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t);

	    if (t == TCL_NUMBER_BIG) {
		/* Truncate to the signed wide range. */
		if (mp_isneg((mp_int *)cd)) {
		    offset = (bytes[3] == '-') ? WIDE_MAX : WIDE_MIN;
		} else {
		    offset = (bytes[3] == '-') ? WIDE_MIN : WIDE_MAX;
		}
	    } else {
		/* assert (t == TCL_NUMBER_INT); */
		offset = (*(Tcl_WideInt *)cd);
		if (bytes[3] == '-') {
		    offset = (offset == WIDE_MIN) ? WIDE_MAX : -offset;
		}
		if (offset == 1) {
		    offset = WIDE_MAX; /* "end+1" */
		} else if (offset > 1) {
		    offset = WIDE_MAX - 1; /* "end+n", out of range */
		} else if (offset != WIDE_MIN) {
		    offset--;
		}
	    }
	}

    parseOK:
	/* Success. Store the new internal rep. */
	ir.wideValue = offset;
	Tcl_StoreInternalRep(objPtr, &endOffsetType, &ir);
    }

    offset = irPtr->wideValue;

    if (offset == WIDE_MAX) {
	/*
	 * Encodes end+1. This is distinguished from end+n as noted
	 * in function header.
	 * NOTE: this may wrap around if the caller passes (as lset does)
	 * listLen-1 as endValue and listLen is 0. The -1 will be
	 * interpreted as FF...FF and adding 1 will result in 0 which
	 * is what we want. Callers like lset which pass in listLen-1 == -1
	 * as endValue will have to adjust accordingly.
	 */
	*widePtr = (endValue == -1) ? WIDE_MAX : endValue + 1;
    } else if (offset == WIDE_MIN) {
	*widePtr = (endValue == -1) ? WIDE_MIN : -1;
    } else if (offset < 0) {
	/* end-(n-1) - Different signs, sum cannot overflow */
	*widePtr = endValue + offset + 1;
    } else {
	/* 0:WIDE_MAX - plain old index. */
	*widePtr = offset;
    }
    return TCL_OK;

    /* Report a parse error. */
  parseError:
    if (interp != NULL) {
	char * bytes = TclGetString(objPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"bad index \"%s\": must be integer?[+-]integer? or"
		" end?[+-]integer?", bytes));
	if (!strncmp(bytes, "end-", 4)) {
	    bytes += 4;
	}
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", (char *)NULL);
    }

    return TCL_ERROR;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimStringGetRange`, lines 2706–2719. Line unit: physical LF only. Original byte range [79361, 79839). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `a706ac09e3b9a5c1722d4044c1378d854fe033d6e81792f288bbd26ce7f94354`; retained evidence `source-window-40`.

```text
static int JimStringGetRange(Jim_Interp *interp, Jim_Obj *firstObjPtr, Jim_Obj *lastObjPtr,
    int len, int *first, int *last, int *range)
{
    if (Jim_GetIndex(interp, firstObjPtr, first) != JIM_OK) {
        return JIM_ERR;
    }
    if (Jim_GetIndex(interp, lastObjPtr, last) != JIM_OK) {
        return JIM_ERR;
    }
    *first = JimRelToAbsIndex(len, *first);
    *last = JimRelToAbsIndex(len, *last);
    JimRelToAbsRange(len, first, last, range);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_StringRangeObj`, lines 2741–2768. Line unit: physical LF only. Original byte range [80361, 81177). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `beee166c4a381a4041f5a2808af0bdcf2a289589396f338ae16f1a8056b96598`; retained evidence `source-window-41`.

```text
Jim_Obj *Jim_StringRangeObj(Jim_Interp *interp,
    Jim_Obj *strObjPtr, Jim_Obj *firstObjPtr, Jim_Obj *lastObjPtr)
{
#ifdef JIM_UTF8
    int first, last;
    const char *str;
    int len, rangeLen;
    int bytelen;

    str = Jim_GetString(strObjPtr, &bytelen);
    len = Jim_Utf8Length(interp, strObjPtr);

    if (JimStringGetRange(interp, firstObjPtr, lastObjPtr, len, &first, &last, &rangeLen) != JIM_OK) {
        return NULL;
    }

    if (first == 0 && rangeLen == len) {
        return strObjPtr;
    }
    if (len == bytelen) {
        /* ASCII optimisation */
        return Jim_NewStringObj(interp, str + first, rangeLen);
    }
    return Jim_NewStringObjUtf8(interp, str + utf8_index(str, first), rangeLen);
#else
    return Jim_StringByteRangeObj(interp, strObjPtr, firstObjPtr, lastObjPtr);
#endif
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_StringByteRangeObj`, lines 2721–2739. Line unit: physical LF only. Original byte range [79840, 80360). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `bc8b322ce5dd70de6b42cf7246ca7d09ac5e4b629c7f7752307f6be5cf4f4feb`; retained evidence `source-window-42`.

```text
Jim_Obj *Jim_StringByteRangeObj(Jim_Interp *interp,
    Jim_Obj *strObjPtr, Jim_Obj *firstObjPtr, Jim_Obj *lastObjPtr)
{
    int first, last;
    const char *str;
    int rangeLen;
    int bytelen;

    str = Jim_GetString(strObjPtr, &bytelen);

    if (JimStringGetRange(interp, firstObjPtr, lastObjPtr, bytelen, &first, &last, &rangeLen) != JIM_OK) {
        return NULL;
    }

    if (first == 0 && rangeLen == bytelen) {
        return strObjPtr;
    }
    return Jim_NewStringObj(interp, str + first, rangeLen);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetIndex`, lines 8326–8344. Line unit: physical LF only. Original byte range [259686, 260276). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `e69392d845af7ada0d169c8703b7ec2127156f90f1755797a93e0b0ba85152d9`; retained evidence `source-window-43`.

```text
int Jim_GetIndex(Jim_Interp *interp, Jim_Obj *objPtr, int *indexPtr)
{
    /* Avoid shimmering if the object is an integer. */
    if (objPtr->typePtr == &intObjType) {
        jim_wide val = JimWideValue(objPtr);

        if (val < 0)
            *indexPtr = -INT_MAX;
        else if (val > INT_MAX)
            *indexPtr = INT_MAX;
        else
            *indexPtr = (int)val;
        return JIM_OK;
    }
    if (objPtr->typePtr != &indexObjType && SetIndexFromAny(interp, objPtr) == JIM_ERR)
        return JIM_ERR;
    *indexPtr = objPtr->internalRep.intValue;
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `SetIndexFromAny`, lines 8254–8324. Line unit: physical LF only. Original byte range [257892, 259685). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `efadbb2defc05a6d97ac2f56086add8851119ab90cf80ba8ea74824380f8be00`; retained evidence `source-window-44`.

```text
static int SetIndexFromAny(Jim_Interp *interp, Jim_Obj *objPtr)
{
    jim_wide idx;
    int end = 0;
    const char *str;
    Jim_Obj *exprObj = objPtr;

    JimPanic((objPtr->refCount == 0, "SetIndexFromAny() called with zero refcount object"));

    /* Get the string representation */
    str = Jim_String(objPtr);

    /* Try to convert into an index */
    if (strncmp(str, "end", 3) == 0) {
        end = 1;
        str += 3;
        idx = 0;
        switch (*str) {
            case '\0':
                exprObj = NULL;
                break;

            case '-':
            case '+':
                /* Create a temp object here for evaluation, but this only happens
                 * once unless the index object shimmers since the result is kept
                 */
                exprObj = Jim_NewStringObj(interp, str, -1);
                break;

            default:
                goto badindex;
        }
    }
    if (exprObj) {
        int ret;
        Jim_IncrRefCount(exprObj);
        ret = Jim_GetWideExpr(interp, exprObj, &idx);
        Jim_DecrRefCount(interp, exprObj);
        if (ret != JIM_OK) {
            goto badindex;
        }
    }

    if (end) {
        if (idx > 0) {
            idx = INT_MAX;
        }
        else {
            /* end-1 is repesented as -2 */
            idx--;
        }
    }
    else if (idx < 0) {
        idx = -INT_MAX;
    }
    else if (idx > INT_MAX) {
        goto badindex;
    }

    /* Free the old internal repr and set the new one. */
    Jim_FreeIntRep(interp, objPtr);
    objPtr->typePtr = &indexObjType;
    objPtr->internalRep.intValue = idx;
    return JIM_OK;

  badindex:
    Jim_SetResultFormatted(interp,
        "bad index \"%#s\": must be intexpr or end?[+-]intexpr?", objPtr);
    return JIM_ERR;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_Utf8Length`, lines 2491–2503. Line unit: physical LF only. Original byte range [73003, 73369). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `42b7da0bec907380a4163c0c1d18622cacbf844fa007c0a7af4ed7ebd5888076`; retained evidence `source-window-45`.

```text
int Jim_Utf8Length(Jim_Interp *interp, Jim_Obj *objPtr)
{
#ifdef JIM_UTF8
    SetStringFromAny(interp, objPtr);

    if (objPtr->internalRep.strValue.charLength < 0) {
        objPtr->internalRep.strValue.charLength = utf8_strlen(objPtr->bytes, objPtr->length);
    }
    return objPtr->internalRep.strValue.charLength;
#else
    return Jim_Length(objPtr);
#endif
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_StringCoreCommand OPT_RANGE case`, lines 14962–14979. Line unit: physical LF only. Original byte range [473428, 474034). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `7a49dbdc64977f03b18f5cad0defbbfbb51f085bb44d2e238d00d45619965159`; retained evidence `source-window-46`.

```text
        case OPT_RANGE:{
                Jim_Obj *objPtr = Jim_StringRangeObj(interp, argv[2], argv[3], argv[4]);
                if (objPtr == NULL) {
                    return JIM_ERR;
                }
                Jim_SetResult(interp, objPtr);
                return JIM_OK;
            }

        case OPT_BYTERANGE:{
                Jim_Obj *objPtr = Jim_StringByteRangeObj(interp, argv[2], argv[3], argv[4]);
                if (objPtr == NULL) {
                    return JIM_ERR;
                }
                Jim_SetResult(interp, objPtr);
                return JIM_OK;
            }


```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_StringCoreCommand OPT_INDEX case`, lines 15029–15053. Line unit: physical LF only. Original byte range [475600, 476521). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `c33188ea103aa42e528c1e810678099a965926c32fda51a8ebb40b1e21812229`; retained evidence `source-window-47`.

```text
        case OPT_INDEX:{
                int idx;
                const char *str;

                if (Jim_GetIndex(interp, argv[3], &idx) != JIM_OK) {
                    return JIM_ERR;
                }
                str = Jim_String(argv[2]);
                len = Jim_Utf8Length(interp, argv[2]);
                idx = JimRelToAbsIndex(len, idx);
                if (idx < 0 || idx >= len || str == NULL) {
                    Jim_SetResultString(interp, "", 0);
                }
                else if (len == Jim_Length(argv[2])) {
                    /* ASCII optimisation */
                    Jim_SetResultString(interp, str + idx, 1);
                }
                else {
                    int c;
                    int i = utf8_index(str, idx);
                    Jim_SetResultString(interp, str + i, utf8_tounicode(str + i, &c));
                }
                return JIM_OK;
            }


```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/utf8.c`, function `utf8_strlen`, lines 63–77. Line unit: physical LF only. Original byte range [1330, 1628). Full-source SHA-256 `905a2ffed3671332e014f5964560c98462cd45483f5f1b9bdfd03109e0ddf526`; snippet SHA-256 `13cdf8195f1c948c31d8e1758dccd9e35654c319632b64b2db969f49e371a510`; retained evidence `source-window-48`.

```text
int utf8_strlen(const char *str, int bytelen)
{
    int charlen = 0;
    if (bytelen < 0) {
        bytelen = strlen(str);
    }
    while (bytelen > 0) {
        int c;
        int l = utf8_tounicode(str, &c);
        charlen++;
        str += l;
        bytelen -= l;
    }
    return charlen;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/utf8.c`, function `utf8_index`, lines 79–86. Line unit: physical LF only. Original byte range [1629, 1780). Full-source SHA-256 `905a2ffed3671332e014f5964560c98462cd45483f5f1b9bdfd03109e0ddf526`; snippet SHA-256 `7e1134644e87f15b1ccd3f9e89c4e9adc092d35a9430cb1d5f5231f3940e3dba`; retained evidence `source-window-49`.

```text
int utf8_index(const char *str, int index)
{
    const char *s = str;
    while (index--) {
        s += utf8_charlen(*s);
    }
    return s - str;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/utf8.c`, function `utf8_tounicode`, lines 109–148. Line unit: physical LF only. Original byte range [2207, 3429). Full-source SHA-256 `905a2ffed3671332e014f5964560c98462cd45483f5f1b9bdfd03109e0ddf526`; snippet SHA-256 `ae10829cddca29646acfb3c2ffdb42425cdff5e5c6aa3600e5914f4376be7916`; retained evidence `source-window-50`.

```text
int utf8_tounicode(const char *str, int *uc)
{
    unsigned const char *s = (unsigned const char *)str;

    if (s[0] < 0xc0) {
        *uc = s[0];
        return 1;
    }
    if (s[0] < 0xe0) {
        if ((s[1] & 0xc0) == 0x80) {
            *uc = ((s[0] & ~0xc0) << 6) | (s[1] & ~0x80);
            if (*uc >= 0x80) {
                return 2;
            }
            /* Otherwise this is an invalid sequence */
        }
    }
    else if (s[0] < 0xf0) {
        if (((str[1] & 0xc0) == 0x80) && ((str[2] & 0xc0) == 0x80)) {
            *uc = ((s[0] & ~0xe0) << 12) | ((s[1] & ~0x80) << 6) | (s[2] & ~0x80);
            if (*uc >= 0x800) {
                return 3;
            }
            /* Otherwise this is an invalid sequence */
        }
    }
    else if (s[0] < 0xf8) {
        if (((str[1] & 0xc0) == 0x80) && ((str[2] & 0xc0) == 0x80) && ((str[3] & 0xc0) == 0x80)) {
            *uc = ((s[0] & ~0xf0) << 18) | ((s[1] & ~0x80) << 12) | ((s[2] & ~0x80) << 6) | (s[3] & ~0x80);
            if (*uc >= 0x10000) {
                return 4;
            }
            /* Otherwise this is an invalid sequence */
        }
    }

    /* Invalid sequence, so just return the byte */
    *uc = *s;
    return 1;
}

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Use the unchanged original counted-object probe and all 20 indices, including the four explicit Jim skips. Per-provider receipts retain exact compile/link/launch vectors, environments, stream hashes and original image identities. The whole input archive retains every recorded source/header/configuration/static library/compiler/CLI/request/probe/version payload. Verify original selected bytes or treat rebuilt bytes as a new experiment; omitted linker/toolchain/load closure remains outside this evidence. Do not replace invalid native bytes with a Rust Unicode projection or borrow replacement/direct-helper lifetime claims.
