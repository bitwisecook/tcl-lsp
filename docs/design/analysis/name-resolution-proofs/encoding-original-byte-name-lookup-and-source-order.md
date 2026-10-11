# naming.encoding.original-byte-name-lookup-and-source-order

Kind: `native-observation`

## Problem statement

Encoding catalogue names are original String bytes, not Unicode identifiers. Source gets an explicit encoding String before opening the file but resolves its catalogue name after successful open. A Unicode validator, replacement-character rewrite, early lookup or ignored encoding operand changes the public result and source effects. Empty system reset is a stored platform default, distinct from the current system encoding.

## Question

For the exact counted String names, including literal and modified zero and invalid UTF-8 bytes, what encoding system and explicit source results, errorCode bytes, system changes and file effects occur on each actual initialized provider, and which source stage wins when the file is missing?

## Conclusion

All measured C providers use original byte encoding names with a literal-zero C-string extent: utf-8 followed by raw zero and a tail is accepted, uppercase UTF-8 is rejected, and raw FF/C0AF and modified-zero names produce original byte unknown-encoding diagnostics rather than a Unicode validation error. Empty system names restore the actual stored default: identity on the measured C8.4/C8.5 initialized processes and iso8859-1 on the measured C8.6/C9 processes; these are environment observations, not release-derived defaults. Binary is unknown for encoding system. C8.5/C8.6 explicit source accepts empty/binary channel names; C9 rejects them with its no-longer-supported diagnostic and NONE. Every explicit-source missing-file case on C8.5+ reports POSIX ENOENT before encoding lookup, including invalid and unsupported names. C8.4 rejects the explicit form by arity. Jim has no encoding command and accepts only the source filename grammar. Successful existing-file cases execute the exact script and return SOURCE_OK. Full result/errorCode/system/effect bytes remain in the original streams.

## Scope

Six separately compiled and executed probe processes, each process exit0, contain 41 fresh initialized interpreters: 13 exact public counted String constructor names across system setter, explicit-source existing and explicit-source missing workers, plus two plain-source controls. This is 246 interpreter blocks, not 246 subprocesses. C uses Tcl_CreateInterp/Tcl_Init and Jim uses its recorded initialized provider sequence; each block seeds errorCode and the source effect and observes the current system value separately. Each C block explicitly establishes utf-8 before the tested operation. Exact compiler/link commands, CLI versions, independent same-library versions, original probe ELF and library/header/source bytes and before/after pins are retained. Source excerpts support getter-before-open versus catalogue-after-open stages; public outputs establish file-failure precedence. No callback/updater/free chronology, Native reference-count or protective-hold schedule, complete encoding catalogue, non-ASCII file codec/profile/eof/BOM semantics, Rust build/assertion outcome, universal platform/default or BIG-IP result is established.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original compile/link command and compiler bytes, source/header/configuration/archive or shared-library bytes, invoked CLI and probe ELF bytes, CLI and independent same-library version captures, and before/after whole-byte pins are retained. A complete system toolchain and loaded dynamic-library closure is unrecorded.. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten.. Dialect: tcl8.4.

Original unknown encoding diagnostics retain raw bytes and set NONE. Literal-zero tails truncate; modified zero remains in the name. Empty system reset returns the measured identity default. Binary and uppercase are unknown. All 26 explicit-source calls fail by source fileName arity, with no script effect. Plain source succeeds on the existing file and reports ENOENT on the absent file.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original compile/link command and compiler bytes, source/header/configuration/archive or shared-library bytes, invoked CLI and probe ELF bytes, CLI and independent same-library version captures, and before/after whole-byte pins are retained. A complete system toolchain and loaded dynamic-library closure is unrecorded.. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten.. Dialect: tcl8.5.

System empty restores the measured identity default; binary/uppercase/raw invalid/modified-zero names are unknown with TCL LOOKUP ENCODING and exact original bytes. Existing explicit source accepts empty/binary and the supported names and executes the script; absent files always report POSIX ENOENT before lookup. Literal-zero names use the truncated prefix.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original compile/link command and compiler bytes, source/header/configuration/archive or shared-library bytes, invoked CLI and probe ELF bytes, CLI and independent same-library version captures, and before/after whole-byte pins are retained. A complete system toolchain and loaded dynamic-library closure is unrecorded.. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten.. Dialect: tcl8.6.

System empty restores the measured iso8859-1 default. Exact byte unknown names retain TCL LOOKUP ENCODING. Existing explicit source accepts empty/binary; absent files report POSIX ENOENT first. Literal-zero prefixes are selected and modified zero is not a terminator.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original compile/link command and compiler bytes, source/header/configuration/archive or shared-library bytes, invoked CLI and probe ELF bytes, CLI and independent same-library version captures, and before/after whole-byte pins are retained. A complete system toolchain and loaded dynamic-library closure is unrecorded.. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten.. Dialect: tcl9.0.

System empty restores the measured iso8859-1 default. Existing explicit source rejects empty/binary with the exact no-longer-supported diagnostic and NONE. Missing files report POSIX ENOENT before that configuration/lookup failure. Other original unknown names preserve byte diagnostics and TCL LOOKUP ENCODING.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original compile/link command and compiler bytes, source/header/configuration/archive or shared-library bytes, invoked CLI and probe ELF bytes, CLI and independent same-library version captures, and before/after whole-byte pins are retained. A complete system toolchain and loaded dynamic-library closure is unrecorded.. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten.. Dialect: tcl9.1.

System empty restores the measured iso8859-1 default. Existing explicit source rejects empty/binary with the exact no-longer-supported diagnostic and NONE. Missing files report POSIX ENOENT before configuration/lookup failure. Literal-zero prefix and original invalid-byte lookup results match the recorded C9.0 cases.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Original compile/link command and compiler bytes, source/header/configuration/archive or shared-library bytes, invoked CLI and probe ELF bytes, CLI and independent same-library version captures, and before/after whole-byte pins are retained. A complete system toolchain and loaded dynamic-library closure is unrecorded.. Channel: One original compiled public object-vector probe process with 41 fresh initialized interpreters. Names come from public counted String constructors over exact C arrays; original literal and modified zero bytes are not source-decoded or rewritten.. Dialect: jim.

Encoding is unavailable: its original command lookup error is retained for all13 system attempts and each separate before/after system query. All26 explicit source attempts fail the filename-only arity and preserve the seeded errorCode. Plain existing source runs the script and normalizes return to SOURCE_OK; plain missing source uses its measured capitalized No such file or directory and preserves the seeded errorCode.

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance process, build, library or executable was measured.. Channel: No original BIG-IP input was executed.. Dialect: bigip.

No original BIG-IP encoding or source worker observation is available.

## Exact evidence

- `original-counted-probe` (input): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/probe.c](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/probe.c). SHA-256 `7f7c2c4bc91f2305ff5ad12036a44a078fbcdf6acbf40968a559b5c0ae2e8f39`. Exact public counted String constructor inputs and object-vector schedule, with independent query and tested-command fields.
- `original-request` (input): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/request.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/request.json). SHA-256 `21cd13c419f677a78feacb7055c844ff68754f4d12da0d711b2bbc762f064704`. Exact original question and provider/case requirements; no golden answer is prescribed.
- `original-counted-names` (input): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/cases.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/cases.json). SHA-256 `9e43554c1e0f8eb516d9ab2360f69ee97dcb60c7fa30af859685526e7de11799`. All thirteen exact counted raw names, three workers and two plain-source controls; existing source script bytes are specified independently.
- `original-process-results` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/results.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/results.json). SHA-256 `fd84aff2ce623d746f71db1fe041882cdff327034042d77b0bacb407ea573e94`. Six closed original provider processes, each exit 0 and 41 actual interpreter blocks; original receipt hashes unchanged.
- `actual-block-projection` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/interpretation.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/interpretation.json). SHA-256 `15a2e5008192957642661f8e70ab8f18c827c53b5ecfb3b9436c9ddff1fb97b0`. All 246 exact raw blocks, preserving tested RETURN separately from availability/version/system queries.
- `exact-input-map` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/pinned-input-map.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/pinned-input-map.json). SHA-256 `a953e193261c1fc75f6caa5f089986834d013826662bddeaaedc451b18f513a3`. Every original input path joins its complete whole-byte archive member; no missing loader/toolchain closure is inferred.
- `exact-input-bytes` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/pinned-input-bytes.tar.gz](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/pinned-input-bytes.tar.gz). SHA-256 `6895a900e636f60d0bb604ec7f21766e82e49ee4e98faf4967fc31800825ae14`. Complete measured source/header/configuration/distribution library/compiler/CLI/input bytes, losslessly compressed and round-trip verified.
- `all-original-retention` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/retention.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/retention.json). SHA-256 `2781ce2e0da477979b3acc62902d0140a322a00900f8e06dc254f5811f61d544`. Every original receipt, raw stream and all twelve driver/version ELFs retained and independently round-trip verified.
- `original-existing-source` (input): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-existing.tcl](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-existing.tcl). SHA-256 `cdaaef7587b816c22bd7b78ff9c021d288afebf82978477fc698692e20e8b4a6`. Exact runner-owned ASCII file content; guaranteed absent sibling has separate before/after receipt evidence.
- `original-input-manifest` (input): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/input-manifest.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/input-manifest.json). SHA-256 `04d67eb4fef84e0c70967b22be08bd4aa23e0295a8471cf8f5a99f82c5afec97`. Exact original input request manifest bound by all original receipts.
- `original-compiler` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/compiler.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/compiler.json). SHA-256 `ce99a334ccb23cd96ed7779ea2313fc175e9f6037afc02df9c5f66eab07fe6d1`. Actual compiler executable fingerprint and version command/stream hashes.
- `same-library-results` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/library-version-results.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/library-version-results.json). SHA-256 `d0d29f047149b6502641d54cfe2776c5d446e3cfe9f3ff52480f46d06af283a2`. Six independent same-library version observations; not inserted into the original tested-command processes.
- `replay-path-adaptation` (input): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/REPLAY-PATH-ADAPTATION.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/REPLAY-PATH-ADAPTATION.json). SHA-256 `0b74954c5bf1cb988488d1265755c5ec7f64f14dacbd175a48469cafcb44b393`. Explicit path-only replay adaptation; unchanged original protocol scripts remain retained.
- `source-window-0` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/0/snippet`. Tcl_GetEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-1` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/1/snippet`. Tcl_SetSystemEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-2` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/2/snippet`. LoadEncodingFile: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-3` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/3/snippet`. Tcl_EncodingObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-4` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/4/snippet`. Tcl_SourceObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-5` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/5/snippet`. Tcl_GetEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-6` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/6/snippet`. Tcl_SetSystemEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-7` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/7/snippet`. LoadEncodingFile: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-8` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/8/snippet`. Tcl_EncodingObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-9` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/9/snippet`. Tcl_SourceObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-10` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/10/snippet`. Tcl_FSEvalFileEx: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-11` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/11/snippet`. Tcl_GetEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-12` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/12/snippet`. Tcl_SetSystemEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-13` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/13/snippet`. LoadEncodingFile: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-14` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/14/snippet`. EncodingSystemObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-15` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/15/snippet`. TclNRSourceObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-16` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/16/snippet`. Tcl_FSEvalFileEx: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-17` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/17/snippet`. Tcl_GetEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-18` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/18/snippet`. Tcl_SetSystemEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-19` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/19/snippet`. LoadEncodingFile: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-20` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/20/snippet`. EncodingSystemObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-21` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/21/snippet`. TclNRSourceObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-22` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/22/snippet`. Tcl_FSEvalFileEx: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-23` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/23/snippet`. Tcl_GetEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-24` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/24/snippet`. Tcl_SetSystemEncoding: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-25` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/25/snippet`. LoadEncodingFile: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-26` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/26/snippet`. EncodingSystemObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-27` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/27/snippet`. TclNRSourceObjCmd: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-28` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/28/snippet`. Tcl_FSEvalFileEx: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-29` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/29/snippet`. Jim_SourceCoreCommand: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-30` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/30/snippet`. TclInitEncodingSubsystem stored defaultEncoding initialization: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-31` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/31/snippet`. TclInitEncodingSubsystem stored defaultEncoding initialization: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-32` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/32/snippet`. TclInitEncodingSubsystem stored defaultEncoding initialization: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-33` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/33/snippet`. TclInitEncodingSubsystem stored defaultEncoding initialization: exact decoded snippet, physical-LF lines and whole source/archive join.
- `source-window-34` (source-anchor): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/source-windows.json). SHA-256 `732e1b290f1bdcc1fcc3f50e11dffe4b64c0c3ee65eb556ce7080a5af3c2311d`. JSON pointer `/anchors/34/snippet`. TclInitEncodingSubsystem stored defaultEncoding initialization: exact decoded snippet, physical-LF lines and whole source/archive join.
- `tcl8.4-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/receipt.json). SHA-256 `37cc822682c7bbeeadde3b9c886d5973aa54512ed5d2f819abc39ad3bec9410a`. Actual original compile/link/launch recipes, environment, input pins, source file absence checks and executable identity.
- `tcl8.4-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/execute.stdout). SHA-256 `3f8550728381dff0a2d1a23f4d176a978529d6389480146ec07f6d932801e24c`. Unchanged original raw pipe-delimited output for all41 fresh interpreters; tested RETURN is separate from observer queries.
- `tcl8.4-execute-stderr` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/execute.stderr](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original probe process stderr.
- `tcl8.4-observations-json` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/observations.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/observations.json). SHA-256 `bcdfa41e210d8e2265e083a0f257b306129948c862e08f3b340c8d5b4c0dafba`. Exact original parser projection of the retained raw output, byte-for-byte record unchanged.
- `tcl8.4-cli-version-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/cli-version.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/cli-version.stdout). SHA-256 `c4a268983bec029286482df6caa3aaae81571c33d467c0a13f258e335da5954f`. Actual invoked CLI patchlevel bytes, independently receipt-bound.
- `tcl8.4-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/same-library-version/receipt.json). SHA-256 `fcf59c5bfd494b1028df7bae3e6e115a67b997907bd18950129dc5e89f450a0b`. Independent version driver bound to the identical selected static library and original input pins.
- `tcl8.4-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.4.20/same-library-version/execute.stdout). SHA-256 `ef18d09813e9c05fbd397154e6f786ea0221f2ce8981597665e917a08a5c5d7f`. Actual linked-library patchlevel and C public version fields.
- `tcl8.5-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/receipt.json). SHA-256 `5cde5fe66bc2a4333d7a9d95f81f76bb6339e167a84bcbe3828a0569170752e0`. Actual original compile/link/launch recipes, environment, input pins, source file absence checks and executable identity.
- `tcl8.5-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/execute.stdout). SHA-256 `6d53603789c40062f4fcdfd52714452b929d48e4e2fc3a35aa1b5c75169f4456`. Unchanged original raw pipe-delimited output for all41 fresh interpreters; tested RETURN is separate from observer queries.
- `tcl8.5-execute-stderr` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/execute.stderr](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original probe process stderr.
- `tcl8.5-observations-json` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/observations.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/observations.json). SHA-256 `2383738009d598066c41b5153ee5e2bc8740e1cdeca7624a3328ca6409faae98`. Exact original parser projection of the retained raw output, byte-for-byte record unchanged.
- `tcl8.5-cli-version-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/cli-version.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/cli-version.stdout). SHA-256 `6b58793c92a3db92926bdacbe0a52a520d417fe876aaa7cf8c29fed54a1fea71`. Actual invoked CLI patchlevel bytes, independently receipt-bound.
- `tcl8.5-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/same-library-version/receipt.json). SHA-256 `51d00af4c334cc902ddfa7cf214cbeb5f3008ded767f7643db11587c652a0798`. Independent version driver bound to the identical selected static library and original input pins.
- `tcl8.5-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.5.19/same-library-version/execute.stdout). SHA-256 `1bd253489464d3ed255f286efdc8deb3768100a5b873e0a062f5026291fcf9bb`. Actual linked-library patchlevel and C public version fields.
- `tcl8.6-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/receipt.json). SHA-256 `9a70c06f16be9aa5d9ae7aeeb9c14719fccc467efcb7358105f6465d14052ce5`. Actual original compile/link/launch recipes, environment, input pins, source file absence checks and executable identity.
- `tcl8.6-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/execute.stdout). SHA-256 `870f84ddd82f83a76dca7cff9154f134b16cd69f882444314130594aa09a96ae`. Unchanged original raw pipe-delimited output for all41 fresh interpreters; tested RETURN is separate from observer queries.
- `tcl8.6-execute-stderr` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/execute.stderr](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original probe process stderr.
- `tcl8.6-observations-json` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/observations.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/observations.json). SHA-256 `431c1a6bb50f5482e4b7e4596c41b553e5c3a39cd5da8fe3e5f96c8c3be1fb6e`. Exact original parser projection of the retained raw output, byte-for-byte record unchanged.
- `tcl8.6-cli-version-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/cli-version.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/cli-version.stdout). SHA-256 `7523e113df6c321e18099a10ec22086db746e2fcc31a879e16d961e5d3878d01`. Actual invoked CLI patchlevel bytes, independently receipt-bound.
- `tcl8.6-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/same-library-version/receipt.json). SHA-256 `f782223aa90ce4bfcb02623e4174aa96c01a05dbd8a05cd33c8d473d9e1c2aa4`. Independent version driver bound to the identical selected static library and original input pins.
- `tcl8.6-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/8.6.18/same-library-version/execute.stdout). SHA-256 `dd12d5dc76bfc80ae3033ac87b1932541f275a888adf9e03dabbcb5878b0df8c`. Actual linked-library patchlevel and C public version fields.
- `tcl9.0-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/receipt.json). SHA-256 `a29e93c27e0202806a72291e454bb1312e1685178f4affec266ab8adccf79f06`. Actual original compile/link/launch recipes, environment, input pins, source file absence checks and executable identity.
- `tcl9.0-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/execute.stdout). SHA-256 `91b8a78fea58146fda99c491703692b8955b7e360175e83063ef878322a45480`. Unchanged original raw pipe-delimited output for all41 fresh interpreters; tested RETURN is separate from observer queries.
- `tcl9.0-execute-stderr` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/execute.stderr](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original probe process stderr.
- `tcl9.0-observations-json` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/observations.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/observations.json). SHA-256 `c9cde94e128f49210b8112b0ec9cdac97d8fc66dd65c5e44c2a47155ec12043c`. Exact original parser projection of the retained raw output, byte-for-byte record unchanged.
- `tcl9.0-cli-version-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/cli-version.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/cli-version.stdout). SHA-256 `b83920bac1b2d47c3840f588f95bee9dc69b85e26117f471541cdf85d7da52a2`. Actual invoked CLI patchlevel bytes, independently receipt-bound.
- `tcl9.0-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/same-library-version/receipt.json). SHA-256 `5b1d72369c29734f33e6fe8d64670a4cdcef61b0d4edc1ca3cbb885cb641e89c`. Independent version driver bound to the identical selected static library and original input pins.
- `tcl9.0-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.0.4/same-library-version/execute.stdout). SHA-256 `d016aed0da5636f83bfe6d76f7c26b06e30ce7951203b5d3543001780eb51a2c`. Actual linked-library patchlevel and C public version fields.
- `tcl9.1-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/receipt.json). SHA-256 `2e866e65c0e7048dd19395fa1873cc64340eb7703ef6345136fc125516275b24`. Actual original compile/link/launch recipes, environment, input pins, source file absence checks and executable identity.
- `tcl9.1-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/execute.stdout). SHA-256 `32d3f8733028139d981f27e438844e269d94a987491c42188a978428b846db68`. Unchanged original raw pipe-delimited output for all41 fresh interpreters; tested RETURN is separate from observer queries.
- `tcl9.1-execute-stderr` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/execute.stderr](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original probe process stderr.
- `tcl9.1-observations-json` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/observations.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/observations.json). SHA-256 `964f3413265b177745a959b931f108592cc40fd1c08e0009fbb0e55c9f691a2c`. Exact original parser projection of the retained raw output, byte-for-byte record unchanged.
- `tcl9.1-cli-version-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/cli-version.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/cli-version.stdout). SHA-256 `077ee5b3a3a7fc2622ceff6466fea0c28f3fb08b24653dcc7f4baab29b9aef36`. Actual invoked CLI patchlevel bytes, independently receipt-bound.
- `tcl9.1-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/same-library-version/receipt.json). SHA-256 `3a602fe131aa36d70fb5776d8151ddb68a27ba57b6c0097e93ae9947cb329778`. Independent version driver bound to the identical selected static library and original input pins.
- `tcl9.1-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/9.1.0/same-library-version/execute.stdout). SHA-256 `b225f15b75641002abfa4f29d4d1be66051123c85bc897b9a7234b22317ea071`. Actual linked-library patchlevel and C public version fields.
- `jim-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/receipt.json). SHA-256 `d723e7ef3c18f6b7af89110bbc17f49652950e296a03d3523439db1fe91abbb7`. Actual original compile/link/launch recipes, environment, input pins, source file absence checks and executable identity.
- `jim-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/execute.stdout). SHA-256 `a7c6258a0f7caf511cb26412856cbe91af21cd9eabff79e10edbe32002b84dae`. Unchanged original raw pipe-delimited output for all41 fresh interpreters; tested RETURN is separate from observer queries.
- `jim-execute-stderr` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/execute.stderr](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original probe process stderr.
- `jim-observations-json` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/observations.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/observations.json). SHA-256 `99ad167755299f486b4da0cf44d9908ba97b6799ebec3ff956640d189e4c2153`. Exact original parser projection of the retained raw output, byte-for-byte record unchanged.
- `jim-cli-version-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/cli-version.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/cli-version.stdout). SHA-256 `f9752899fa012cf994a528e0250402609067a6f67a144a1172644a9329386803`. Actual invoked CLI patchlevel bytes, independently receipt-bound.
- `jim-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/same-library-version/receipt.json). SHA-256 `654052a8eed20369ade32ea3b3fa851d6fd463f8e3152edf9f17be29326b786d`. Independent version driver bound to the identical selected static library and original input pins.
- `jim-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/Jim/same-library-version/execute.stdout). SHA-256 `d7ce62ac8c979a247ad4edf0a43c9133dc8b547cdfc8e5b540fccd9bbe848166`. Actual linked-library patchlevel and C public version fields.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 495–519. Line unit: physical LF only. Original byte range [16151, 16877). Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `9b604f69fa14f45ce33484f29de83022bdc7e517762a429cee35f2e096948adb`; retained evidence `source-window-0`.

```text
Tcl_GetEncoding(interp, name)
    Tcl_Interp *interp;		/* Interp for error reporting, if not NULL. */
    CONST char *name;		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *) Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);
    return LoadEncodingFile(interp, name);
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 765–792. Line unit: physical LF only. Original byte range [23082, 23817). Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `c78933322d4ad52d756b53adffd6a81befab2c610d3703fd48bd119a448db9f1`; retained evidence `source-window-1`.

```text
Tcl_SetSystemEncoding(interp, name)
    Tcl_Interp *interp;		/* Interp for error reporting, if not NULL. */
    CONST char *name;		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding;
    Encoding *encodingPtr;

    if (!name || !*name) {
	Tcl_MutexLock(&encodingMutex);
	encoding = defaultEncoding;
	encodingPtr = (Encoding *) encoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
    } else {
	encoding = Tcl_GetEncoding(interp, name);
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    Tcl_MutexLock(&encodingMutex);
    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);

    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 1291–1371. Line unit: physical LF only. Original byte range [39515, 41274). Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `7d9f330d72b641f9646fa7f978ba9bbb882a10543581cee4306a3e7cb04a2244`; retained evidence `source-window-2`.

```text
LoadEncodingFile(interp, name)
    Tcl_Interp *interp;		/* Interp for error reporting, if not NULL. */
    CONST char *name;		/* The name of the encoding file on disk
				 * and also the name for new encoding. */
{
    int objc, i, ch;
    Tcl_Obj **objv;
    Tcl_Obj *pathPtr;
    Tcl_Channel chan;
    Tcl_Encoding encoding;

    pathPtr = TclGetLibraryPath();
    if (pathPtr == NULL) {
	goto unknown;
    }
    objc = 0;
    Tcl_ListObjGetElements(NULL, pathPtr, &objc, &objv);

    chan = NULL;
    for (i = 0; i < objc; i++) {
	chan = OpenEncodingFile(Tcl_GetString(objv[i]), name);
	if (chan != NULL) {
	    break;
	}
    }

    if (chan == NULL) {
	goto unknown;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    encoding = NULL;
    switch (ch) {
	case 'S': {
	    encoding = LoadTableEncoding(interp, name, ENCODING_SINGLEBYTE,
		    chan);
	    break;
	}
	case 'D': {
	    encoding = LoadTableEncoding(interp, name, ENCODING_DOUBLEBYTE,
		    chan);
	    break;
	}
	case 'M': {
	    encoding = LoadTableEncoding(interp, name, ENCODING_MULTIBYTE,
		    chan);
	    break;
	}
	case 'E': {
	    encoding = LoadEscapeEncoding(name, chan);
	    break;
	}
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_AppendResult(interp, "invalid encoding file \"", name, "\"", NULL);
	if (ch == 'E') {
	    Tcl_AppendResult(interp, " or missing sub-encoding", NULL);
	}
    }
    Tcl_Close(NULL, chan);
    return encoding;

    unknown:
    if (interp != NULL) {
	Tcl_AppendResult(interp, "unknown encoding \"", name, "\"", NULL);
    }
    return NULL;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdAH.c`, function `Tcl_EncodingObjCmd`, lines 423–527. Line unit: physical LF only. Original byte range [10292, 12896). Full-source SHA-256 `4119b635cc3677728baa21df8ee8c683c3f37aa87bf9ca80fcab74eec948fd75`; snippet SHA-256 `2199d10f6fa5dbab66299360fc4b966c60dcce589e50384d535bcf3709670489`; retained evidence `source-window-3`.

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

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_SourceObjCmd`, lines 1056–1068. Line unit: physical LF only. Original byte range [28791, 29163). Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `80de1b70d48625832263c3f6e20b399261beb02b8599d9bcbb595bf6ae7ce8dd`; retained evidence `source-window-4`.

```text
Tcl_SourceObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "fileName");
	return TCL_ERROR;
    }

    return Tcl_FSEvalFile(interp, objv[1]);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 768–793. Line unit: physical LF only. Original byte range [23644, 24358). Full-source SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`; snippet SHA-256 `d4911812488dc4e170490c52250191cbad1bfcecfe0ee3bf8e11a1be8044fa6d`; retained evidence `source-window-5`.

```text
Tcl_GetEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *) Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);

    return LoadEncodingFile(interp, name);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 988–1015. Line unit: physical LF only. Original byte range [29334, 30056). Full-source SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`; snippet SHA-256 `896b4dbffba6ad0496f5f82a9a750c19374b956ca385d13bade59cae3f7edeaf`; retained evidence `source-window-6`.

```text
Tcl_SetSystemEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding;
    Encoding *encodingPtr;

    if (!name || !*name) {
	Tcl_MutexLock(&encodingMutex);
	encoding = defaultEncoding;
	encodingPtr = (Encoding *) encoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
    } else {
	encoding = Tcl_GetEncoding(interp, name);
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    Tcl_MutexLock(&encodingMutex);
    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 1584–1632. Line unit: physical LF only. Original byte range [47153, 48288). Full-source SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`; snippet SHA-256 `240dcc946f4585295ae84fd4b84df5e20fd7512935591fad9bc0cf0f78af6f16`; retained evidence `source-window-7`.

```text
LoadEncodingFile(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the encoding file on disk and
				 * also the name for new encoding. */
{
    Tcl_Channel chan = NULL;
    Tcl_Encoding encoding = NULL;
    int ch;

    chan = OpenEncodingFileChannel(interp, name);
    if (chan == NULL) {
	return NULL;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    switch (ch) {
    case 'S':
	encoding = LoadTableEncoding(name, ENCODING_SINGLEBYTE, chan);
	break;
    case 'D':
	encoding = LoadTableEncoding(name, ENCODING_DOUBLEBYTE, chan);
	break;
    case 'M':
	encoding = LoadTableEncoding(name, ENCODING_MULTIBYTE, chan);
	break;
    case 'E':
	encoding = LoadEscapeEncoding(name, chan);
	break;
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_AppendResult(interp, "invalid encoding file \"", name, "\"", NULL);
    }
    Tcl_Close(NULL, chan);

    return encoding;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdAH.c`, function `Tcl_EncodingObjCmd`, lines 426–528. Line unit: physical LF only. Original byte range [10204, 12822). Full-source SHA-256 `cdd328ee0a0d0c3ec7b67da50c90ae22b40591cc52e9492f19d7b257355fcf43`; snippet SHA-256 `400fc03cb450abdcd8d861af8f35e3b158847f0b4789698228dc46473a2dca6d`; retained evidence `source-window-8`.

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

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `Tcl_SourceObjCmd`, lines 936–966. Line unit: physical LF only. Original byte range [22991, 23739). Full-source SHA-256 `fe2c67a2097ece2e07d9345a5edf416b5fbc891276349e360f9c25da1cc8f348`; snippet SHA-256 `9043e538bce92348d2adf6272ef579201f852415b8be5381f75edf8d0bae8c1f`; retained evidence `source-window-9`.

```text
Tcl_SourceObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;

    if (objc != 2 && objc !=4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding name? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    }

    return Tcl_FSEvalFileEx(interp, fileName, encodingName);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1770–1892. Line unit: physical LF only. Original byte range [50144, 53829). Full-source SHA-256 `24e13e612b8207a7109d6cb6175a8972d4b5233ec1088b7fbfbf632bf47b9b73`; snippet SHA-256 `c72fbb10de3ee199a7d63037bc78d1f2419e0ecbdeb854503cbf70d1a8c3e738`; retained evidence `source-window-10`.

```text
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter in which to process file. */
    Tcl_Obj *pathPtr,		/* Path of file to process. Tilde-substitution
				 * will be performed on this name. */
    const char *encodingName)	/* If non-NULL, then use this encoding for the
				 * file. NULL means use the system encoding. */
{
    int length, result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == (Tcl_Channel) NULL) {
	Tcl_ResetResult(interp);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	return result;
    }

    /*
     * The eofchar is \32 (^Z). This is the usual on Windows, but we effect
     * this cross-platform to allow for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\32");

    /*
     * If the encoding is specified, set it for the channel. Else don't touch
     * it (and use the system encoding) Report error on unknown encoding.
     */

    if (encodingName != NULL) {
	if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
		!= TCL_OK) {
	    Tcl_Close(interp,chan);
	    return result;
	}
    }

    objPtr = Tcl_NewObj();
    Tcl_IncrRefCount(objPtr);
    /* Try to read first character of stream, so we can
     * check for utf-8 BOM to be handled especially.
     */
    if (Tcl_ReadChars(chan, objPtr, 1, 0) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }
    string = Tcl_GetString(objPtr);
    /*
     * If first character is not a BOM, append the remaining characters,
     * otherwise replace them [Bug 3466099].
     */
    if (Tcl_ReadChars(chan, objPtr, -1,
	    memcmp(string, "\xef\xbb\xbf", 3)) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }

    if (Tcl_Close(interp, chan) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = Tcl_GetStringFromObj(objPtr, &length);
    /* TIP #280 Force the evaluator to open a frame for a sourced
     * file. */
    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = Tcl_EvalEx(interp, string, length, 0);

    /*
     * Now we have to be careful; the script may have changed the
     * iPtr->scriptFile value, so we must reset it without assuming it still
     * points to 'pathPtr'.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information telling where the error occurred.
	 */

	const char *pathString = Tcl_GetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : length), pathString,
		(overflow ? "..." : ""), interp->errorLine));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 755–780. Line unit: physical LF only. Original byte range [22892, 23605). Full-source SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; snippet SHA-256 `87a2edabb834ba3feaa55988d2f9dad4438d39e287e8b68ad6918fd0f3f8210a`; retained evidence `source-window-11`.

```text
Tcl_GetEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *)Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);

    return LoadEncodingFile(interp, name);
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 978–1006. Line unit: physical LF only. Original byte range [28602, 29355). Full-source SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; snippet SHA-256 `41d196054cc62afa3fd8e255d4feddbea6647c7f668527ac7fcd303865fdc2b3`; retained evidence `source-window-12`.

```text
Tcl_SetSystemEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding;
    Encoding *encodingPtr;

    if (!name || !*name) {
	Tcl_MutexLock(&encodingMutex);
	encoding = defaultEncoding;
	encodingPtr = (Encoding *) encoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
    } else {
	encoding = Tcl_GetEncoding(interp, name);
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    Tcl_MutexLock(&encodingMutex);
    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);
    Tcl_FSMountsChanged(NULL);

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 1611–1661. Line unit: physical LF only. Original byte range [47338, 48542). Full-source SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; snippet SHA-256 `19b943118f80f7c8fd2c0fa8f5d514bb61deb522ceadd24d9d1f75bec9a1d65a`; retained evidence `source-window-13`.

```text
LoadEncodingFile(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of both the encoding file
				 * and the new encoding. */
{
    Tcl_Channel chan = NULL;
    Tcl_Encoding encoding = NULL;
    int ch;

    chan = OpenEncodingFileChannel(interp, name);
    if (chan == NULL) {
	return NULL;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    switch (ch) {
    case 'S':
	encoding = LoadTableEncoding(name, ENCODING_SINGLEBYTE, chan);
	break;
    case 'D':
	encoding = LoadTableEncoding(name, ENCODING_DOUBLEBYTE, chan);
	break;
    case 'M':
	encoding = LoadTableEncoding(name, ENCODING_MULTIBYTE, chan);
	break;
    case 'E':
	encoding = LoadEscapeEncoding(name, chan);
	break;
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"invalid encoding file \"%s\"", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENCODING", name, (char *)NULL);
    }
    Tcl_Close(NULL, chan);

    return encoding;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdAH.c`, function `EncodingSystemObjCmd`, lines 849–866. Line unit: physical LF only. Original byte range [22819, 23346). Full-source SHA-256 `752473fb5c05d166602ed00b2afff8e123dafd34f4c585628e941b8c5691ae88`; snippet SHA-256 `8b10aa1b064765bc4ffffd1bc000875d2d19b84dd0bb0240ef84502f80c97f33`; retained evidence `source-window-14`.

```text
EncodingSystemObjCmd(
    ClientData dummy,      /* Unused */
    Tcl_Interp* interp,     /* Tcl interpreter */
    int objc,		    /* Number of command line args */
    Tcl_Obj* const objv[])  /* Vector of command line args */
{
    if (objc > 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?encoding?");
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp,
			 Tcl_NewStringObj(Tcl_GetEncodingName(NULL), -1));
    } else {
	return Tcl_SetSystemEncoding(interp, TclGetString(objv[1]));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 985–1015. Line unit: physical LF only. Original byte range [24875, 25627). Full-source SHA-256 `c2fe2deb95449c94c2719dca830d4e57ec4a436741199da39122d772888e849e`; snippet SHA-256 `7e90d5fcad7d151b938fba44889b4e0812424f7a8dfb248bc63c2ba1a4658122`; retained evidence `source-window-15`.

```text
TclNRSourceObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;

    if (objc != 2 && objc !=4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding name? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    }

    return TclNREvalFile(interp, fileName, encodingName);
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1727–1860. Line unit: physical LF only. Original byte range [48406, 52138). Full-source SHA-256 `efc2579a7e21c6e4490116ddee386d3ea83819a3bcad76215127b480f5934f00`; snippet SHA-256 `99708366d088e82ca8ec9adf808d926816f676cbfc3a1944439346f19288601a`; retained evidence `source-window-16`.

```text
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter in which to process file. */
    Tcl_Obj *pathPtr,		/* Path of file to process. Tilde-substitution
				 * will be performed on this name. */
    const char *encodingName)	/* If non-NULL, then use this encoding for the
				 * file. NULL means use the system encoding. */
{
    int length, result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }

    /*
     * The eofchar is \32 (^Z). This is the usual on Windows, but we effect
     * this cross-platform to allow for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\32 {}");

    /*
     * If the encoding is specified, set it for the channel. Else don't touch
     * it (and use the system encoding) Report error on unknown encoding.
     */

    if (encodingName != NULL) {
	if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
		!= TCL_OK) {
	    Tcl_Close(interp,chan);
	    return result;
	}
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Try to read first character of stream, so we can check for utf-8 BOM to
     * be handled especially.
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) < 0) {
	Tcl_Close(interp, chan);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	goto end;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters,
     * otherwise replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, -1,
	    memcmp(string, "\xEF\xBB\xBF", 3)) < 0) {
	Tcl_Close(interp, chan);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	goto end;
    }

    if (Tcl_Close(interp, chan) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280 Force the evaluator to open a frame for a sourced file.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

    /*
     * Now we have to be careful; the script may have changed the
     * iPtr->scriptFile value, so we must reset it without assuming it still
     * points to 'pathPtr'.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information telling where the error occurred.
	 */

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 732–757. Line unit: physical LF only. Original byte range [22767, 23480). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `87a2edabb834ba3feaa55988d2f9dad4438d39e287e8b68ad6918fd0f3f8210a`; retained evidence `source-window-17`.

```text
Tcl_GetEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *)Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);

    return LoadEncodingFile(interp, name);
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 981–1023. Line unit: physical LF only. Original byte range [29089, 30148). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `ad5352fb15b1ed2323a95b6c27046504150b46c214e3329224b05b9c4b4b1a30`; retained evidence `source-window-18`.

```text
Tcl_SetSystemEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding = NULL;


    if (name && *name) {
	encoding = Tcl_GetEncoding(interp, name); /* this increases refCount */
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    /* Don't lock (and change anything, bump epoch) if it remains unchanged. */

    if ((encoding ? encoding : defaultEncoding) == systemEncoding) {
	if (encoding) {
	    Tcl_FreeEncoding(encoding); /* paired to Tcl_GetEncoding */


	}


	return TCL_OK;
    }

    /* Checks above ensure this is only called when system encoding changes */
    Tcl_MutexLock(&encodingMutex);
    if (!encoding) {
	encoding = defaultEncoding; /* need increase its refCount */
	((Encoding *)encoding)->refCount++;
    }

    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);

    Tcl_FSMountsChanged(NULL);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 1878–1928. Line unit: physical LF only. Original byte range [57123, 58332). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `7a86210ebc3cabc74903d59cffcc1e3754551237092cea0b44b9fe26f18fc4bd`; retained evidence `source-window-19`.

```text
LoadEncodingFile(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of both the encoding file
				 * and the new encoding. */
{
    Tcl_Channel chan = NULL;
    Tcl_Encoding encoding = NULL;
    int ch;

    chan = OpenEncodingFileChannel(interp, name);
    if (chan == NULL) {
	return NULL;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    switch (ch) {
    case 'S':
	encoding = LoadTableEncoding(name, ENCODING_SINGLEBYTE, chan);
	break;
    case 'D':
	encoding = LoadTableEncoding(name, ENCODING_DOUBLEBYTE, chan);
	break;
    case 'M':
	encoding = LoadTableEncoding(name, ENCODING_MULTIBYTE, chan);
	break;
    case 'E':
	encoding = LoadEscapeEncoding(name, chan);
	break;
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"invalid encoding file \"%s\"", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENCODING", name, (char *)NULL);
    }
    Tcl_CloseEx(NULL, chan, 0);

    return encoding;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdAH.c`, function `EncodingSystemObjCmd`, lines 820–837. Line unit: physical LF only. Original byte range [23019, 23528). Full-source SHA-256 `b1fd4aec514b23bbca0a43d99e25a613ec570752e50fadf1caf5c499b1a54ec1`; snippet SHA-256 `a3520d93f7c6c81c13f5a3395fe8b3915e31c747534de653019bca8a5fa27aaf`; retained evidence `source-window-20`.

```text
EncodingSystemObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp* interp,     /* Tcl interpreter */
    int objc,		    /* Number of command line args */
    Tcl_Obj* const objv[])  /* Vector of command line args */
{
    if (objc > 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?encoding?");
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp,
		Tcl_NewStringObj(Tcl_GetEncodingName(NULL), -1));
    } else {
	return Tcl_SetSystemEncoding(interp, TclGetString(objv[1]));
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 1104–1158. Line unit: physical LF only. Original byte range [28601, 30131). Full-source SHA-256 `080e201d1a09760aad4fedd9835e2091fd663d405d32d034f32dd0918d9006cc`; snippet SHA-256 `0a690aa7c36dc4393d6eae55827a952cf5f4197178d5c4679a6ec0b2f4f993f2`; retained evidence `source-window-21`.

```text
TclNRSourceObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;
    int result;
    void **pkgFiles = NULL;
    void *names = NULL;

    if (objc < 2 || objc > 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding encoding? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    } else if (objc == 3) {
	/* Handle undocumented -nopkg option. This should only be
	 * used by the internal ::tcl::Pkg::source utility function. */
	static const char *const nopkgoptions[] = {
	    "-nopkg", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], nopkgoptions,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	pkgFiles = (void **)Tcl_GetAssocData(interp, "tclPkgFiles", NULL);
	/* Make sure that during the following TclNREvalFile no filenames
	 * are recorded for inclusion in the "package files" command */
	names = *pkgFiles;
	*pkgFiles = NULL;
    }
    result = TclNREvalFile(interp, fileName, encodingName);
    if (pkgFiles) {
	/* restore "tclPkgFiles" assocdata to how it was. */
	*pkgFiles = names;
    }
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1691–1826. Line unit: physical LF only. Original byte range [44401, 48127). Full-source SHA-256 `a92cd056226140169ef4789870156962b7219d99cb72218a999ca5b8d9ce8770`; snippet SHA-256 `ad8317f27808f6fc0469edab8bcbed953f0982e409c93f4b70aa9d731778e93d`; retained evidence `source-window-22`.

```text
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter that evaluates the script. */
    Tcl_Obj *pathPtr,		/* Pathname of the file to process.
				 * Tilde-substitution is performed on this
				 * pathname. */
    const char *encodingName)	/* Either the name of an encoding or NULL to
				 * use the utf-8 encoding. */
{
    Tcl_Size length;
    int result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }

    /*
     * The eof character is \x1A (^Z). Tcl uses it on every platform to allow
     * for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\x1A");

    /*
     * If the encoding is specified, set the channel to that encoding.
     * Otherwise use utf-8.  If the encoding is unknown report an error.
     */

    if (encodingName == NULL) {
	encodingName = "utf-8";
    }
    if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
	    != TCL_OK) {
	Tcl_CloseEx(interp,chan,0);
	return result;
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Read first character of stream to check for utf-8 BOM
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	goto end;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters.
     * Otherwise, replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, TCL_INDEX_NONE,
	    memcmp(string, "\xEF\xBB\xBF", 3)) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	goto end;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

    /*
     * Restore the original iPtr->scriptFile value, but because the value may
     * have hanged during evaluation, don't assume it currently points to
     * pathPtr.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information about where the error occurred.
	 */

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : (int)length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Tcl_GetEncoding`, lines 740–765. Line unit: physical LF only. Original byte range [23064, 23777). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `87a2edabb834ba3feaa55988d2f9dad4438d39e287e8b68ad6918fd0f3f8210a`; retained evidence `source-window-23`.

```text
Tcl_GetEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding. */
{
    Tcl_HashEntry *hPtr;
    Encoding *encodingPtr;

    Tcl_MutexLock(&encodingMutex);
    if (name == NULL) {
	encodingPtr = (Encoding *) systemEncoding;
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return systemEncoding;
    }

    hPtr = Tcl_FindHashEntry(&encodingTable, name);
    if (hPtr != NULL) {
	encodingPtr = (Encoding *)Tcl_GetHashValue(hPtr);
	encodingPtr->refCount++;
	Tcl_MutexUnlock(&encodingMutex);
	return (Tcl_Encoding) encodingPtr;
    }
    Tcl_MutexUnlock(&encodingMutex);

    return LoadEncodingFile(interp, name);
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Tcl_SetSystemEncoding`, lines 989–1026. Line unit: physical LF only. Original byte range [29375, 30429). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `4defff683f856608ffe2088a108e9c43d67b79eb6231f6733366bb09c7a6d5df`; retained evidence `source-window-24`.

```text
Tcl_SetSystemEncoding(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of the desired encoding, or NULL/""
				 * to reset to default encoding. */
{
    Tcl_Encoding encoding = NULL;

    if (name && *name) {
	encoding = Tcl_GetEncoding(interp, name); /* this increases refCount */
	if (encoding == NULL) {
	    return TCL_ERROR;
	}
    }

    /* Don't lock (and change anything, bump epoch) if it remains unchanged. */

    if ((encoding ? encoding : defaultEncoding) == systemEncoding) {
	if (encoding) {
	    Tcl_FreeEncoding(encoding); /* paired to Tcl_GetEncoding */
	}
	return TCL_OK;
    }

    /* Checks above ensure this is only called when system encoding changes */
    Tcl_MutexLock(&encodingMutex);
    if (!encoding) {
	encoding = defaultEncoding; /* need increase its refCount */
	((Encoding *)encoding)->refCount++;
    }

    FreeEncoding(systemEncoding);
    systemEncoding = encoding;
    Tcl_MutexUnlock(&encodingMutex);

    Tcl_FSMountsChanged(NULL);

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `LoadEncodingFile`, lines 2420–2470. Line unit: physical LF only. Original byte range [76100, 77309). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `7a86210ebc3cabc74903d59cffcc1e3754551237092cea0b44b9fe26f18fc4bd`; retained evidence `source-window-25`.

```text
LoadEncodingFile(
    Tcl_Interp *interp,		/* Interp for error reporting, if not NULL. */
    const char *name)		/* The name of both the encoding file
				 * and the new encoding. */
{
    Tcl_Channel chan = NULL;
    Tcl_Encoding encoding = NULL;
    int ch;

    chan = OpenEncodingFileChannel(interp, name);
    if (chan == NULL) {
	return NULL;
    }

    Tcl_SetChannelOption(NULL, chan, "-encoding", "utf-8");

    while (1) {
	Tcl_DString ds;

	Tcl_DStringInit(&ds);
	Tcl_Gets(chan, &ds);
	ch = Tcl_DStringValue(&ds)[0];
	Tcl_DStringFree(&ds);
	if (ch != '#') {
	    break;
	}
    }

    switch (ch) {
    case 'S':
	encoding = LoadTableEncoding(name, ENCODING_SINGLEBYTE, chan);
	break;
    case 'D':
	encoding = LoadTableEncoding(name, ENCODING_DOUBLEBYTE, chan);
	break;
    case 'M':
	encoding = LoadTableEncoding(name, ENCODING_MULTIBYTE, chan);
	break;
    case 'E':
	encoding = LoadEscapeEncoding(name, chan);
	break;
    }
    if ((encoding == NULL) && (interp != NULL)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"invalid encoding file \"%s\"", name));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENCODING", name, (char *)NULL);
    }
    Tcl_CloseEx(NULL, chan, 0);

    return encoding;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdAH.c`, function `EncodingSystemObjCmd`, lines 841–858. Line unit: physical LF only. Original byte range [25580, 26085). Full-source SHA-256 `56db5fdd8c4bd688cb0beda3b91ad60111adb802fd63c28821055140b582fe04`; snippet SHA-256 `ed9e2cfa7a2e1767b8587a1793d30f883b584c2d9030038c0905382662ebbd1d`; retained evidence `source-window-26`.

```text
EncodingSystemObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Tcl interpreter */
    Tcl_Size objc,		/* Number of command line args */
    Tcl_Obj *const *objv)	/* Vector of command line args */
{
    if (objc > 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "?encoding?");
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp,
		Tcl_NewStringObj(Tcl_GetEncodingName(NULL), -1));
    } else {
	return Tcl_SetSystemEncoding(interp, TclGetString(objv[1]));
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `TclNRSourceObjCmd`, lines 1169–1223. Line unit: physical LF only. Original byte range [31813, 33346). Full-source SHA-256 `62be408d1faa29a35d3e30eacc4986fd9f1a973f7efcd474b3a2a9082ded512f`; snippet SHA-256 `f17dd80b646a218b4a9e91bd3a6909741b527ebd3096290e6f1de7ba2dd5bda4`; retained evidence `source-window-27`.

```text
TclNRSourceObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    const char *encodingName = NULL;
    Tcl_Obj *fileName;
    int result;
    void **pkgFiles = NULL;
    void *names = NULL;

    if (objc < 2 || objc > 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-encoding encoding? fileName");
	return TCL_ERROR;
    }

    fileName = objv[objc-1];

    if (objc == 4) {
	static const char *const options[] = {
	    "-encoding", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], options,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	encodingName = TclGetString(objv[2]);
    } else if (objc == 3) {
	/* Handle undocumented -nopkg option. This should only be
	 * used by the internal ::tcl::Pkg::source utility function. */
	static const char *const nopkgoptions[] = {
	    "-nopkg", NULL
	};
	int index;

	if (TCL_ERROR == Tcl_GetIndexFromObj(interp, objv[1], nopkgoptions,
		"option", TCL_EXACT, &index)) {
	    return TCL_ERROR;
	}
	pkgFiles = (void **)Tcl_GetAssocData(interp, "tclPkgFiles", NULL);
	/* Make sure that during the following TclNREvalFile no filenames
	 * are recorded for inclusion in the "package files" command */
	names = *pkgFiles;
	*pkgFiles = NULL;
    }
    result = TclNREvalFile(interp, fileName, encodingName);
    if (pkgFiles) {
	/* restore "tclPkgFiles" assocdata to how it was. */
	*pkgFiles = names;
    }
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1690–1825. Line unit: physical LF only. Original byte range [44472, 48198). Full-source SHA-256 `0e7e89671b5840ab90267b9ef5a4a9cac35b702c1a5b62a8a42de066dff514fd`; snippet SHA-256 `ad8317f27808f6fc0469edab8bcbed953f0982e409c93f4b70aa9d731778e93d`; retained evidence `source-window-28`.

```text
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter that evaluates the script. */
    Tcl_Obj *pathPtr,		/* Pathname of the file to process.
				 * Tilde-substitution is performed on this
				 * pathname. */
    const char *encodingName)	/* Either the name of an encoding or NULL to
				 * use the utf-8 encoding. */
{
    Tcl_Size length;
    int result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    const char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return result;
    }

    /*
     * The eof character is \x1A (^Z). Tcl uses it on every platform to allow
     * for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\x1A");

    /*
     * If the encoding is specified, set the channel to that encoding.
     * Otherwise use utf-8.  If the encoding is unknown report an error.
     */

    if (encodingName == NULL) {
	encodingName = "utf-8";
    }
    if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
	    != TCL_OK) {
	Tcl_CloseEx(interp,chan,0);
	return result;
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Read first character of stream to check for utf-8 BOM
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	goto end;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters.
     * Otherwise, replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, TCL_INDEX_NONE,
	    memcmp(string, "\xEF\xBB\xBF", 3)) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	goto end;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = TclGetStringFromObj(objPtr, &length);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = TclEvalEx(interp, string, length, 0, 1, NULL, string);

    /*
     * Restore the original iPtr->scriptFile value, but because the value may
     * have hanged during evaluation, don't assume it currently points to
     * pathPtr.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information about where the error occurred.
	 */

	const char *pathString = TclGetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : (int)length), pathString,
		(overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SourceCoreCommand`, lines 16711–16716. Line unit: physical LF only. Original byte range [529211, 529412). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `06f50ddd7d96c6cf52b7de221ead820052a94493441f3e1f3d016bb7708981b5`; retained evidence `source-window-29`.

```text
static int Jim_SourceCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retval = Jim_EvalFile(interp, Jim_String(argv[1]));

    return retval == JIM_RETURN ? JIM_OK : retval;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 347–386. Line unit: physical LF only. Original byte range [12281, 13438). Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `71280c0e7de662d0a14a7b87c521c6968392e317ad42cb59c9afac5b17fff141`; retained evidence `source-window-30`.

```text
TclInitEncodingSubsystem()
{
    Tcl_EncodingType type;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);
    
    /*
     * Create a few initial encodings.  Note that the UTF-8 to UTF-8 
     * translation is not a no-op, because it will turn a stream of
     * improperly formed UTF-8 into a properly formed stream.
     */

    type.encodingName	= "identity";
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;

    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfExtToUtfIntProc;
    type.fromUtfProc	= UtfIntToUtfExtProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    type.encodingName   = "unicode";
    type.toUtfProc	= UnicodeToUtfProc;
    type.fromUtfProc    = UtfToUnicodeProc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 544–634. Line unit: physical LF only. Original byte range [17524, 20200). Full-source SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`; snippet SHA-256 `9f0bc429e72d2c6b03610163742c90a86516004fd4437e6f7d0764354e26da4d`; retained evidence `source-window-31`.

```text
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;

    if (encodingsInitialized) {
	return;
    }

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings. Note that the UTF-8 to UTF-8
     * translation is not a no-op, because it will turn a stream of improperly
     * formed UTF-8 into a properly formed stream.
     */

    type.encodingName	= "identity";
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;

    defaultEncoding	= Tcl_CreateEncoding(&type);
    tclIdentityEncoding = Tcl_GetEncoding(NULL, type.encodingName);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfExtToUtfIntProc;
    type.fromUtfProc	= UtfIntToUtfExtProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    type.encodingName   = "unicode";
    type.toUtfProc	= UnicodeToUtfProc;
    type.fromUtfProc    = UtfToUnicodeProc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    {
	TableEncodingData *dataPtr = (TableEncodingData *)
		ckalloc(sizeof(TableEncodingData));
	unsigned size;
	unsigned short i;

	memset(dataPtr, 0, sizeof(TableEncodingData));
	dataPtr->fallback = '?';

	size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
	dataPtr->toUnicode = (unsigned short **) ckalloc(size);
	memset(dataPtr->toUnicode, 0, size);
	dataPtr->fromUnicode = (unsigned short **) ckalloc(size);
	memset(dataPtr->fromUnicode, 0, size);

	dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
	dataPtr->fromUnicode[0] = (unsigned short *)
		(dataPtr->fromUnicode + 256);
	for (i=1 ; i<256 ; i++) {
	    dataPtr->toUnicode[i] = emptyPage;
	    dataPtr->fromUnicode[i] = emptyPage;
	}

	for (i=0 ; i<256 ; i++) {
	    dataPtr->toUnicode[0][i] = i;
	    dataPtr->fromUnicode[0][i] = i;
	}

	type.encodingName	= "iso8859-1";
	type.toUtfProc		= Iso88591ToUtfProc;
	type.fromUtfProc	= Iso88591FromUtfProc;
	type.freeProc		= TableFreeProc;
	type.nullSize		= 1;
	type.clientData		= dataPtr;
	Tcl_CreateEncoding(&type);
    }

    encodingsInitialized = 1;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 533–622. Line unit: physical LF only. Original byte range [16889, 19612). Full-source SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; snippet SHA-256 `e1c1e1dbab49b645edc795fe9cec4d5be895d59c1382cb7aa99f207cbf48cec0`; retained evidence `source-window-32`.

```text
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;
    TableEncodingData *dataPtr;
    unsigned size;
    unsigned short i;
    union {
	char c;
	short s;
    } isLe;

    if (encodingsInitialized) {
	return;
    }

    isLe.s = 1;
    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings.  UTF-8 to UTF-8 translation is not a
     * no-op because it turns a stream of improperly formed UTF-8 into a
     * properly formed stream.
     */

    type.encodingName	= "identity";
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclIdentityEncoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfExtToUtfIntProc;
    type.fromUtfProc	= UtfIntToUtfExtProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    type.encodingName   = "unicode";
    type.toUtfProc	= UnicodeToUtfProc;
    type.fromUtfProc    = UtfToUnicodeProc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.clientData	= INT2PTR(isLe.c);
    Tcl_CreateEncoding(&type);

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    dataPtr = (TableEncodingData *)ckalloc(sizeof(TableEncodingData));
    memset(dataPtr, 0, sizeof(TableEncodingData));
    dataPtr->fallback = '?';

    size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
    dataPtr->toUnicode = (unsigned short **)ckalloc(size);
    memset(dataPtr->toUnicode, 0, size);
    dataPtr->fromUnicode = (unsigned short **)ckalloc(size);
    memset(dataPtr->fromUnicode, 0, size);

    dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
    dataPtr->fromUnicode[0] = (unsigned short *) (dataPtr->fromUnicode + 256);
    for (i=1 ; i<256 ; i++) {
	dataPtr->toUnicode[i] = emptyPage;
	dataPtr->fromUnicode[i] = emptyPage;
    }

    for (i=0 ; i<256 ; i++) {
	dataPtr->toUnicode[0][i] = i;
	dataPtr->fromUnicode[0][i] = i;
    }

    type.encodingName	= "iso8859-1";
    type.toUtfProc	= Iso88591ToUtfProc;
    type.fromUtfProc	= Iso88591FromUtfProc;
    type.freeProc	= TableFreeProc;
    type.nullSize	= 1;
    type.clientData	= dataPtr;
    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    encodingsInitialized = 1;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 524–659. Line unit: physical LF only. Original byte range [16679, 20842). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `33cb6bccddf1b69b6e04eb93f12d3c8a8bdbe733f368aba3558cf05a0ffbfe56`; retained evidence `source-window-33`.

```text
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;
    TableEncodingData *dataPtr;
    unsigned size;
    unsigned short i;
    union {
	char c;
	short s;
    } isLe;
    int leFlags;

    if (encodingsInitialized) {
	return;
    }

    /* Note: This DEPENDS on TCL_ENCODING_LE being defined in least sig byte */
    isLe.s = 1;
    leFlags = isLe.c ? TCL_ENCODING_LE : 0;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings.  UTF-8 to UTF-8 translation is not a
     * no-op because it turns a stream of improperly formed UTF-8 into a
     * properly formed stream.
     */

    type.encodingName	= NULL;
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclIdentityEncoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfToUtfProc;
    type.fromUtfProc	= UtfToUtfProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= INT2PTR(ENCODING_UTF);
    tclUtf8Encoding = Tcl_CreateEncoding(&type);
    type.clientData	= NULL;
    type.encodingName	= "cesu-8";
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc	= UtfToUcs2Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "ucs-2le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf32ToUtfProc;
    type.fromUtfProc	= UtfToUtf32Proc;
    type.freeProc	= NULL;
    type.nullSize	= 4;
    type.encodingName	= "utf-32le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc    = UtfToUtf16Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "utf-16le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

#ifndef TCL_NO_DEPRECATED
    type.encodingName	= "unicode";
    Tcl_CreateEncoding(&type);
#endif

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    dataPtr = (TableEncodingData *)Tcl_Alloc(sizeof(TableEncodingData));
    memset(dataPtr, 0, sizeof(TableEncodingData));
    dataPtr->fallback = '?';

    size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
    dataPtr->toUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->toUnicode, 0, size);
    dataPtr->fromUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->fromUnicode, 0, size);

    dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
    dataPtr->fromUnicode[0] = (unsigned short *) (dataPtr->fromUnicode + 256);
    for (i=1 ; i<256 ; i++) {
	dataPtr->toUnicode[i] = emptyPage;
	dataPtr->fromUnicode[i] = emptyPage;
    }

    for (i=0 ; i<256 ; i++) {
	dataPtr->toUnicode[0][i] = i;
	dataPtr->fromUnicode[0][i] = i;
    }

    type.encodingName	= "iso8859-1";
    type.toUtfProc	= Iso88591ToUtfProc;
    type.fromUtfProc	= Iso88591FromUtfProc;
    type.freeProc	= TableFreeProc;
    type.nullSize	= 1;
    type.clientData	= dataPtr;
    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    encodingsInitialized = 1;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `TclInitEncodingSubsystem stored defaultEncoding initialization`, lines 527–667. Line unit: physical LF only. Original byte range [16867, 21139). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `40343d9fe0e425117590396175455024e31f95d625b7ad0a3e1874d4b4f838c5`; retained evidence `source-window-34`.

```text
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;
    TableEncodingData *dataPtr;
    unsigned size;
    unsigned short i;
    union {
	char c;
	short s;
    } isLe;
    int leFlags;

    if (encodingsInitialized) {
	return;
    }

    /* Note: This DEPENDS on TCL_ENCODING_LE being defined in least sig byte */
    isLe.s = 1;
    leFlags = isLe.c ? TCL_ENCODING_LE : 0;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings.  UTF-8 to UTF-8 translation is not a
     * no-op because it turns a stream of improperly formed UTF-8 into a
     * properly formed stream.
     */

    type.encodingName	= NULL;
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclIdentityEncoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfToUtfProc;
    type.fromUtfProc	= UtfToUtfProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclUtf8Encoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "cesu-8";
    type.toUtfProc	= Cesu8ToUtfProc;
    type.fromUtfProc	= UtfToCesu8Proc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc	= UtfToUcs2Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "ucs-2le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf32ToUtfProc;
    type.fromUtfProc	= UtfToUtf32Proc;
    type.freeProc	= NULL;
    type.nullSize	= 4;
    type.encodingName	= "utf-32le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc    = UtfToUtf16Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "utf-16le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

#ifndef TCL_NO_DEPRECATED
    type.encodingName	= "unicode";
    Tcl_CreateEncoding(&type);
#endif

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    dataPtr = (TableEncodingData *)Tcl_Alloc(sizeof(TableEncodingData));
    memset(dataPtr, 0, sizeof(TableEncodingData));
    dataPtr->fallback = '?';

    size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
    dataPtr->toUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->toUnicode, 0, size);
    dataPtr->fromUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->fromUnicode, 0, size);

    dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
    dataPtr->fromUnicode[0] = (unsigned short *) (dataPtr->fromUnicode + 256);
    for (i=1 ; i<256 ; i++) {
	dataPtr->toUnicode[i] = emptyPage;
	dataPtr->fromUnicode[i] = emptyPage;
    }

    for (i=0 ; i<256 ; i++) {
	dataPtr->toUnicode[0][i] = i;
	dataPtr->fromUnicode[0][i] = i;
    }

    type.encodingName	= "iso8859-1";
    type.toUtfProc	= Iso88591ToUtfProc;
    type.fromUtfProc	= Iso88591FromUtfProc;
    type.freeProc	= TableFreeProc;
    type.nullSize	= 1;
    type.clientData	= dataPtr;
    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    encodingsInitialized = 1;
}

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "runtime/rust/tests/data/native_encoding_original_byte_name_source_order/643/run.py",
  "--output",
  "<new-absolute-directory>"
]
```

The replay adapter verifies all original pins and requires --output <fresh-absolute-directory>; it redirects only the retained original runner request/output constants. Unchanged original runners remain separately retained. Restore and verify the exact original source, header, archive, probe ELF and required distribution-library bytes and use a distinct fresh output directory. The preserved runner uses the original absolute provider paths and creates one probe process with 41 fresh interpreters per provider. Do not overwrite the original receipts or substitute a new executable while retaining their attribution. The full compiler/runtime dynamic load closure and any new Host callback or Rust chronology remain independent. The canonical adapter verifies all recorded original input paths first and redirects only request/output and the identical retained version-driver source. Original absolute provider paths must be restored and verified separately; this archive supplies whole pinned payloads by digest without automatically installing them. Any successful replay is a new experiment, never a replacement for these original streams.
