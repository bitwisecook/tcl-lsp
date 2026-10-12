# naming.error.original-reset-result-error-header-retirement

Kind: `native-observation`

## Problem statement

A result reset retires selected error-header references while callbacks may still see original private or global fields. Clearing fields before their decrement, eagerly formatting a lazy error header, or treating a freeIntRep hook as header death would change the reached stages and reference roles. Original C8.4 globals and later C private fields require distinct observations.

## Question

For five fresh original-header cases on each selected C release, what result/code/info fields and reference roles are visible during public Tcl_ResetResult, its pure freeIntRep and compatibility write callbacks, and which caller-held headers remain lazy until independently requested counted String getters and caller release?

## Conclusion

All five own original C producers complete16 closed0 processes and25 cases:100 ROW,78 EVENT and25 separate GETTER records. C8.4 retains original global code/info headers and clears its explicit-code flag without reset write traces. C8.5–C9.1 decrement code then info while each corresponding old private field is still selected, then clear it; held caller/global roles survive independently. The explicit COPY fixture reaches code then info global traces. Every result R reaches freeIntRep at refs1 while its same current header becomes empty/typeNone; this is not header death. No code/info String updater runs during reset; each later caller getter yields the same14 NUL/non-UTF8-containing bytes. Deliberately installed private info and COPY inputs are not public lazy-info/flag producer proofs. Jim/BIG-IP and matching Rust assertions are not tested; mutating reentry, Host/ABA, teardown callbacks and general header/VM equivalence remain unmeasured.

## Scope

Five fresh own C compilations select original actual SDK Makefile flags/cwd, strict actual header/link/runtime dependency admission and unchanged own executable identities. Five cases per provider select absent error fields, sole selected code/info holds, separate caller holds, explicit compatibility COPY flag with pure global traces, and one same original C header in both code/info fields with two selected holds plus one caller. Original result is independent and result-only held. Modern errorInfo uses an explicitly installed authentic private field/native +1 rather than a public lazy-info producer; COPY is an explicit input flag. C84 uses actual global storage and no modern private flag. Observer callbacks perform no variable worker, getter, shell creation or interpreter mutation; trace registration is its own setup worker. No original pointer is dereferenced after a freeIntRep callback, whose occurrence alone gives no general object-death claim. Caller String presentation and release follow reset; observers are disabled before interpreter teardown. There is no result/error header alias, mutating reentry, Host/ABA, physical frame/ABI, Jim/BIG-IP, Rust/VM, whole-suite or performance grant.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Fresh own GCC14 reset probe with exact selected original SDK Makefile flags/cwd/static library, admitted actual header/link/runtime maps and unchanged own executable identity; compile/dependencies/execute closed0. SDK configure/build replay is absent.. Channel: Public Tcl_ResetResult on five authentic prepared Interp/result/code/info header roles; pure original freeIntRep/compatibility variable trace observation followed by separate caller-held Tcl_GetStringFromObj and caller release.. Dialect: tcl8.4.

Five original cases complete with20 ROW/five separate counted GETTER/10 EVENT. Reset calls R freeIntRep at passedrefs1 while current result remains that same R header and becomes empty/typeNone. The original global code/info headers persist: sole global refs1 in case1, caller-plus-global refs2 in cases2/3, and shared code/info C refs3 in case4. The explicit-code flag goes1 to0; even with registered case3 write traces, reset reaches no global write trace. Later caller getters alone update the original headers and return14 bytes; caller release leaves the independently held globals. C84 has no selected modern private/COPY-flag route. All selected code/info headers stay lazy with updater0 through reset; the five later counted getters each return14 bytes containing NUL/non-UTF8. Modern errorInfo and COPY flag are explicit original-field input fixtures, not observed public producers. Pure observers perform no interpreter mutation. FreeIntRep alone does not prove header death; no teardown callback, Host/ABA, general reentry, safe pointer lifetime or Rust/VM parity follows.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Fresh own GCC14 reset probe with exact selected original SDK Makefile flags/cwd/static library, admitted actual header/link/runtime maps and unchanged own executable identity; compile/dependencies/execute closed0. SDK configure/build replay is absent.. Channel: Public Tcl_ResetResult on five authentic prepared Interp/result/code/info header roles; pure original freeIntRep/compatibility variable trace observation followed by separate caller-held Tcl_GetStringFromObj and caller release.. Dialect: tcl8.5.

Five original cases complete with20 ROW/five separate counted GETTER/17 EVENT. Reset calls R freeIntRep at passedrefs1 while current result remains that same R header and becomes empty/typeNone. With sole private holds, free-C sees codeC/infoI at passedrefs0, then free-I sees codeNONE/infoI before its own field is cleared. Caller-held case2 removes only each private hold, refs2 to1; the later caller releases trigger their frees. Explicit COPY-flag case3 reaches code then info global write traces while their respective private headers are still selected; caller-plus-global refs2 remain after private clear and become1 after caller release. Shared code/info C in case4 loses two private holds, refs3 to1, then a separate getter and caller release occur. All selected code/info headers stay lazy with updater0 through reset; the five later counted getters each return14 bytes containing NUL/non-UTF8. Modern errorInfo and COPY flag are explicit original-field input fixtures, not observed public producers. Pure observers perform no interpreter mutation. FreeIntRep alone does not prove header death; no teardown callback, Host/ABA, general reentry, safe pointer lifetime or Rust/VM parity follows.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Fresh own GCC14 reset probe with exact selected original SDK Makefile flags/cwd/static library, admitted actual header/link/runtime maps and unchanged own executable identity; compile/dependencies/execute closed0. SDK configure/build replay is absent.. Channel: Public Tcl_ResetResult on five authentic prepared Interp/result/code/info header roles; pure original freeIntRep/compatibility variable trace observation followed by separate caller-held Tcl_GetStringFromObj and caller release.. Dialect: tcl8.6.

Five original cases complete with20 ROW/five separate counted GETTER/17 EVENT. Reset calls R freeIntRep at passedrefs1 while current result remains that same R header and becomes empty/typeNone. With sole private holds, free-C sees codeC/infoI at passedrefs0, then free-I sees codeNONE/infoI before its own field is cleared. Caller-held case2 removes only each private hold, refs2 to1; the later caller releases trigger their frees. Explicit COPY-flag case3 reaches code then info global write traces while their respective private headers are still selected; caller-plus-global refs2 remain after private clear and become1 after caller release. Shared code/info C in case4 loses two private holds, refs3 to1, then a separate getter and caller release occur. All selected code/info headers stay lazy with updater0 through reset; the five later counted getters each return14 bytes containing NUL/non-UTF8. Modern errorInfo and COPY flag are explicit original-field input fixtures, not observed public producers. Pure observers perform no interpreter mutation. FreeIntRep alone does not prove header death; no teardown callback, Host/ABA, general reentry, safe pointer lifetime or Rust/VM parity follows.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Fresh own GCC14 reset probe with exact selected original SDK Makefile flags/cwd/static library, admitted actual header/link/runtime maps and unchanged own executable identity; compile/dependencies/execute closed0. SDK configure/build replay is absent.. Channel: Public Tcl_ResetResult on five authentic prepared Interp/result/code/info header roles; pure original freeIntRep/compatibility variable trace observation followed by separate caller-held Tcl_GetStringFromObj and caller release.. Dialect: tcl9.0.

Five original cases complete with20 ROW/five separate counted GETTER/17 EVENT. Reset calls R freeIntRep at passedrefs1 while current result remains that same R header and becomes empty/typeNone. With sole private holds, free-C sees codeC/infoI at passedrefs0, then free-I sees codeNONE/infoI before its own field is cleared. Caller-held case2 removes only each private hold, refs2 to1; the later caller releases trigger their frees. Explicit COPY-flag case3 reaches code then info global write traces while their respective private headers are still selected; caller-plus-global refs2 remain after private clear and become1 after caller release. Shared code/info C in case4 loses two private holds, refs3 to1, then a separate getter and caller release occur. All selected code/info headers stay lazy with updater0 through reset; the five later counted getters each return14 bytes containing NUL/non-UTF8. Modern errorInfo and COPY flag are explicit original-field input fixtures, not observed public producers. Pure observers perform no interpreter mutation. FreeIntRep alone does not prove header death; no teardown callback, Host/ABA, general reentry, safe pointer lifetime or Rust/VM parity follows.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Fresh own GCC14 reset probe with exact selected original SDK Makefile flags/cwd/static library, admitted actual header/link/runtime maps and unchanged own executable identity; compile/dependencies/execute closed0. SDK configure/build replay is absent.. Channel: Public Tcl_ResetResult on five authentic prepared Interp/result/code/info header roles; pure original freeIntRep/compatibility variable trace observation followed by separate caller-held Tcl_GetStringFromObj and caller release.. Dialect: tcl9.1.

Five original cases complete with20 ROW/five separate counted GETTER/17 EVENT. Reset calls R freeIntRep at passedrefs1 while current result remains that same R header and becomes empty/typeNone. With sole private holds, free-C sees codeC/infoI at passedrefs0, then free-I sees codeNONE/infoI before its own field is cleared. Caller-held case2 removes only each private hold, refs2 to1; the later caller releases trigger their frees. Explicit COPY-flag case3 reaches code then info global write traces while their respective private headers are still selected; caller-plus-global refs2 remain after private clear and become1 after caller release. Shared code/info C in case4 loses two private holds, refs3 to1, then a separate getter and caller release occur. All selected code/info headers stay lazy with updater0 through reset; the five later counted getters each return14 bytes containing NUL/non-UTF8. Modern errorInfo and COPY flag are explicit original-field input fixtures, not observed public producers. Pure observers perform no interpreter mutation. FreeIntRep alone does not prove header death; no teardown callback, Host/ABA, general reentry, safe pointer lifetime or Rust/VM parity follows.

### jim

Status: `not-tested`. Version: No jim process selected. Build: No original jim process is compiled or executed for this reset C API question. Channel: No original jim input reaches this selected Tcl_ResetResult C API question. Dialect: jim.

The selected worker is the original Tcl_ResetResult C API, and no Jim Tcl API process is selected or executed. This is not a measured Jim unavailable-worker response or an observation of Jim result APIs.

### bigip

Status: `not-tested`. Version: No bigip process selected. Build: No original bigip process is compiled or executed for this reset C API question. Channel: No original bigip input reaches this selected Tcl_ResetResult C API question. Dialect: bigip.

No BIG-IP process executes this reset-header question. C source inspection and finite C observations do not establish BIG-IP or iRules reset semantics.

## Exact evidence

- `reset-native-0` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/closure.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/closure.json). SHA-256 `e0e0dbacb79e1812d97fd7c35103d8b5a7220fe7c855c22dee61fd2a04c571b8`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-1` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.receipt.json). SHA-256 `4018693cb72cf5325a1250daf8c076af441dbf996b844e7396724808df8196ff`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-2` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-3` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.stdout). SHA-256 `2ae7618a3dfad6644f70dbf191f205882a96d3aac6350978839c4c19389f95a5`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-4` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/launch.py](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/launch.py). SHA-256 `6fd2374ef054b2d0c4615dbf59a1a9f8a189ac4f0565c1da65c09f1cc2ab32d5`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-5` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/manifest.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/manifest.json). SHA-256 `d87cc25b1fb6ad6795266b191160b08998e9be0a55dad525f5fcaf0038d6324f`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-6` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/prepare.py](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/prepare.py). SHA-256 `62a0683ac4d1f8eb7ad4805acd179f5ed05ffc37c562e6f359ec9ec9d64e34f8`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-7` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/probe.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/probe.c). SHA-256 `b1117a8046f729e83a8d15df8c5292378724e35aaf9dfe3c9c1ecd1694ae4588`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-8` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.d). SHA-256 `b48eed9ef11497a2a6329f0fcde4e6ed3d071ae2fc23b5a8e68a2988a9dafef1`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-9` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.receipt.json). SHA-256 `7f7c062424b1905b6824814ab18ee6c061b9d96f39939be1d0a2937fd4808ea6`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-10` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-11` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-12` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.verification.json). SHA-256 `fec57c152e4067d9e52846ed5a5071f45ca90d2175d77c587a124fe2233e9560`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-13` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.receipt.json). SHA-256 `1a3502fce0dede081abb021356fe3b82521eb14c1c60db3d8d143bdb8ef71c01`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-14` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-15` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.stdout). SHA-256 `33cd504cd3e7a02d25d2fa32fea1083946d9b3aa3c550017b64b5f642262cfea`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-16` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.verification.json). SHA-256 `d47e1289ee4c964527970d47bc0b8f1bd684e718bdfad2e9a55407d12e80d6d9`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-17` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.receipt.json). SHA-256 `7d3d14899bd6de3f53f1f5ed100dfb39e1ee1d04b7684c78601a356adae807a5`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-18` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-19` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.stdout). SHA-256 `f4a4254098a5e27d1f514b639a38dc5dba02f602d792aa895bcc22508a257725`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-20` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.verification.json). SHA-256 `c78a76074484bd012dd3a01403c16b52db1db46c841a72e36957b4d05fb7805d`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-21` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/link.map). SHA-256 `5068f5663a61e985fae1c719e1f94ed35ebf6992f847361eaf9b6fe5c0ed6335`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-22` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/probe.elf.gz). SHA-256 `de2fd1506f18cfb1cd988b85d5b2ef008cddc130f6872c417d201a2226f04c5b`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-23` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.d). SHA-256 `ea5176afbb4ab38c3294a950804a787c36dba55a1500d60bc406986d33661d4a`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-24` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.receipt.json). SHA-256 `208dbee5d99071ab1480252ab7d83a0933163461ffcb177314c9b5f21feec866`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-25` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-26` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-27` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.verification.json). SHA-256 `7d2b56ceb45512ebc4b6cb53970ee78c84ba60b35961d768aacbc96d120a7955`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-28` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.receipt.json). SHA-256 `7da420be4d58101d3bf3ec426b4f587e2b27b8206b706e771477094a998a85fc`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-29` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-30` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.stdout). SHA-256 `f96f8124f7a8bef9f6b9ff59d84e6a388da7ad67f867652635d5598eda715b09`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-31` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.verification.json). SHA-256 `911f0cf9f603c1d4dc9d148b7612b33e4b94c7c95b2af4694dee0e8b3801ba3f`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-32` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.receipt.json). SHA-256 `68ad156d4bbb3cfeef781cb68d3fd1a823118878d2da0900cb488f1965b91f9d`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-33` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-34` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.stdout). SHA-256 `7dd86d36f04c9885588b65742dce12e3a890d0712b9bef07aca115a5d7652215`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-35` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.verification.json). SHA-256 `997db719b9ed68bb5209969aed343a2f7fbadfc0f49078fb4ea49a74cf54bad2`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-36` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/link.map). SHA-256 `064e7286c07fc12a4097813a8a773c023a0e296ae4b0942b296fab5247d9e18d`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-37` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/probe.elf.gz). SHA-256 `06413828c6bc4026266ecbf848abf12a183aa3fe636a723137db482bf7b19ba7`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-38` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.d). SHA-256 `aecf294cd8f14f782dbd8caa36e07646fd78e763856d7d4185fd8e734d7ffee9`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-39` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.receipt.json). SHA-256 `c1b9724b3da879851ede70dd84b2ce292e78dc44e753a2817efd82c85efae6e1`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-40` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-41` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-42` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.verification.json). SHA-256 `f90ef830d18c5a4c5cea6d04212eac7e9aec9fc170305aa0c4e0ef52948ceffb`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-43` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.receipt.json). SHA-256 `a4694980d66cc6f8110d0acea34ee25da415061d2bbd40d45948d679d15c206b`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-44` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-45` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.stdout). SHA-256 `8ccba5b0e86a449dc63cece3a779587aaff9a176cb6e50a79bc8ffd8884a8a7e`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-46` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.verification.json). SHA-256 `55d986e8f66ebd6e45c2c919daf63389f448a4406d89bd403565e92753b772a8`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-47` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.receipt.json). SHA-256 `1bea664b6cd54a169b74a34ce5b01be031d44f84f807f0a995082556279dc10f`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-48` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-49` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.stdout). SHA-256 `66179b8d1a93fcab8532f7df63031c4b307ff5428004eac8b87425112e103d7a`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-50` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.verification.json). SHA-256 `236ea724a834baca5f133ebf052e2463fca7e52f8d047552ea109a6b90d0d32e`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-51` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/link.map). SHA-256 `a56c68760f8354b74f275bd341927215690fe21b263d47e660d5224548e43c10`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-52` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/probe.elf.gz). SHA-256 `6af450b1da6128abd33b76f6777f089b70c26a6740ba9fc735a09242b28fc328`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-53` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.d). SHA-256 `fe6cd5c082a8f5e8905a5e284bd17822f6e46d719df8ce5a9eaeaa81def56dd2`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-54` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.receipt.json). SHA-256 `f4ce69d66269b8449a714289da63841901868d5407cf12d58be6d27104d4dee0`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-55` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.stderr). SHA-256 `2403c201bc65df1cf927ce1240018198e05b36111d3c771e91f3e103e7e3de28`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-56` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-57` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.verification.json). SHA-256 `39cfd78a34a6e32f1fa63c3ee9de08c9f2dbd442e9cda5d9cc6eb58c9401ef92`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-58` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.receipt.json). SHA-256 `a49a6516d39769cfb9303781101ad657566418650d530f46995dbe83be34d371`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-59` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-60` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.stdout). SHA-256 `770492ccc9862804e8fab9d9b3d605825ae30cac65da92da566a3e3293d19877`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-61` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.verification.json). SHA-256 `7a2c57d8046694cfb934b04369cc68d7f9cbf4e67e826e862ad36b4a84664879`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-62` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.receipt.json). SHA-256 `e9db9a9a8e20a603bf6989a858527920a76a5cfb1c01c2e4100e77de304dd6a5`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-63` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-64` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.stdout). SHA-256 `45aa20658e38d94adf3f72f6136b108a07f19f632ebc18eb45ac6d4efbb449a4`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-65` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.verification.json). SHA-256 `516941f1ba84b7f858c7b66d1404d31671c6b3512b9287094f7f7acd5bdbefc6`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-66` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/link.map). SHA-256 `6e269298af893cfc2a5dcb4ae0de205070378e8d99610a4799c0322816dcfc9a`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-67` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/probe.elf.gz). SHA-256 `345db2034a29d4eb04e1b5bb99c327e2b703be8c5aa77be554e643c77b964132`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-68` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.d). SHA-256 `1215bd77533e58ff0a1d4552a0812abb673ab493b862e0976212a4d7ef627d64`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-69` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.receipt.json). SHA-256 `c9b07b70260597ddefcb7708025c50ce1ca65952f511f97e3c0932588e179a92`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-70` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.stderr). SHA-256 `1f1e78c47d5a27cc411568d96312bd01771f22426f8ca86e68f6d4292624a834`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-71` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-72` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.verification.json). SHA-256 `2441948668456b7f3fb885c1402d1414aef8c4477f73aa1f352958aaed8e6d91`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-73` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.receipt.json). SHA-256 `7daba5e70a4ac50122226724448e6b3b8c764860cb3eb58c6e78904fb1f8cb9b`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-74` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-75` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.stdout). SHA-256 `3b3c9dcc7da4e87947b3d5babe84163e112c2f46ae9982cc467075fb29a37544`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-76` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.verification.json). SHA-256 `d96a3dee295b63b1a6dcdfafa5ca7106836f024a3ef7642b67bdc283a1cb7747`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-77` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.receipt.json). SHA-256 `528863a71a49a11fc5acb906f26345b23a50e3327c72b3ba43cdbfc1b47c2331`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-78` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-79` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.stdout). SHA-256 `e966266941ed99aafc9c903e83732b0d23280201dda97119744f769a43cf2373`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-80` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.verification.json). SHA-256 `d8c6d8460fbd8d44c86eda7044299e3ee4b571ab3aab4bcb7ed67d62b7eb869c`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-81` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/link.map). SHA-256 `72b2ba0660b7efd50a2c343e8837cc688302831a999d947ad692d81a25978af9`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-82` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/probe.elf.gz). SHA-256 `9fef702813aa418289e3056cddec65d5eb8b4cfaa2f50222c2818b369c776161`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-83` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/request.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/request.json). SHA-256 `82e4ccd978c28f3d3af6385aebd481aad5e88065e4fff59dba1bf00e37ed7536`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-84` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/selected-makefile-builds-original614c.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/selected-makefile-builds-original614c.json). SHA-256 `383384caf469fb6f7bc8566fe66ad3331fb9feada85d8a5ec1f3c05302d31295`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-85` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.4.20/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.4.20/ResetObjResult.c). SHA-256 `e542ec39d7a9a480ce555ae6ec80fb8c21012432f72bdd833c20843e9bc8426f`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-86` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.4.20/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.4.20/Tcl_ResetResult.c). SHA-256 `8199b4f1feb3f7cbf6c1ddde58cf74c163eba23e3b58bb06b84495abd0149a6a`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-87` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.5.19/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.5.19/ResetObjResult.c). SHA-256 `feed8d109d1e069fa84773d574d1385f58adcdf7d96b27114471d206c0e15c3e`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-88` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.5.19/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.5.19/Tcl_ResetResult.c). SHA-256 `df05c1e9f61e9150fb3a6010117555156b7114c9e38530a60d614b9bd3819218`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-89` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.6.18/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.6.18/ResetObjResult.c). SHA-256 `a06d6032f455f2281260e2c1d1a27e0d8a3debd5fd5ea0d01fb2b3fd1eec1891`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-90` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.6.18/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.6.18/Tcl_ResetResult.c). SHA-256 `440321d5a507e5679703d0e05a05f5a6afcd980a1adaa4f7d48e3badb57568c1`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-91` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.0.4/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.0.4/ResetObjResult.c). SHA-256 `b7358ed71467cd9ee344373d3b33fcbfba56d5b47a2684bc11e5d05bbbdd0bd0`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-92` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.0.4/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.0.4/Tcl_ResetResult.c). SHA-256 `4097cc4f75a9bb24fc5ae9f3b376118a3204fd30a9b0e28595bc73ffea4e6306`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-93` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.1.0/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.1.0/ResetObjResult.c). SHA-256 `b7358ed71467cd9ee344373d3b33fcbfba56d5b47a2684bc11e5d05bbbdd0bd0`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-94` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.1.0/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.1.0/Tcl_ResetResult.c). SHA-256 `4097cc4f75a9bb24fc5ae9f3b376118a3204fd30a9b0e28595bc73ffea4e6306`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-95` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows.json). SHA-256 `47abf47504805131060db5b11a8a9754e8d5e4ba1e3f04b0a1f46e76eda0be4d`. Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-96` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/outer-launch.log](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/outer-launch.log). SHA-256 `66baaee6d7175a5af98436f48f263ddc8cd3422e186427be690711ef0373651b`. Exact saved outer launcher stream. Root reports its tool exit separately; no filesystem outer process receipt is fabricated.
- `reset-native-97` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/static-review/receiver.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/static-review/receiver.json). SHA-256 `e9e90ab8b26f1a66ca8431bdc12275dfb97d4bd960735a85f7c781729e178b58`. Exact independent static request/protocol review, separate from actual producer and API observations.
- `reset-native-98` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation/manifest.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation/manifest.json). SHA-256 `25da551b00347c359dfb126c92ade34ad441df84ab257d8e9331b2d0f0e2e9dc`. Exact read-only16-process/25-case interpretation joins actual original bytes and separates pure field/free snapshots, caller getter/release and observer-disabled teardown. No new execution or Rust assertion.
- `reset-native-99` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation/interpretation.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation/interpretation.json). SHA-256 `a753f58b3127f9aa66fd8e099fc71f0dcbb4ebef1ef11338fd64b2be6e76c99f`. Exact read-only16-process/25-case interpretation joins actual original bytes and separates pure field/free snapshots, caller getter/release and observer-disabled teardown. No new execution or Rust assertion.
- `canonical-sdk-input-map` (provider): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/recorded-path-map.json](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/recorded-path-map.json). SHA-256 `97b17a210b253e90c159defae1127fab76de26b4a996b92347f513a86ac7ed14`. Already retained exact original SDK/source/library/header/input map reused by full original/payload hashes. The reset request retains its own complete required input list and own producer gates; setter outputs and observations are not reset evidence.
- `canonical-sdk-issuer-request` (provider): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614c/request.json](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614c/request.json). SHA-256 `f12d6c65ddf19995cba44ef55dd6f0768a935a95fe773879e1faecbe5db3fcd3`. Exact prior admitted selected Makefile/SDK input issuer only. This request supplies no reset executable or reset API result; all reset own outputs and receipts are independently retained.
- `reset-recorded-path-map` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/recorded-path-map.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/recorded-path-map.json). SHA-256 `83a79ce06d0c0e0e170f99b28156c4a9f06930018837fe5759738092f1878c5d`. Exact reset input/output/whole-SHA SDK reuse, compressed own image restoration and hash-only system prerequisite joins.
- `receiver618-manifest.json` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/manifest.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/manifest.json). SHA-256 `ab558f00bebfd75508424832247c525e9159c7126edbe26804978d72c0062d71`. Exact independent read-only actual618 receipt/source/own executable and25-case interpretation. Pure callback, input fixture, later caller getter/release and unobserved teardown limits remain explicit; this is no new execution or Rust assertion.
- `receiver618-interpretation.json` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/interpretation.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/interpretation.json). SHA-256 `4c883635ae1aebc2be79eb069a22cf65ab7493b812cfcedf7cba886bf93ce786`. Exact independent read-only actual618 receipt/source/own executable and25-case interpretation. Pure callback, input fixture, later caller getter/release and unobserved teardown limits remain explicit; this is no new execution or Rust assertion.
- `receiver618-case-observations.json` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/case-observations.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/case-observations.json). SHA-256 `7a86467f85784e5894a2fd061ce4e09097b185870099d4fa4f7eed8d2e6f6192`. Exact independent read-only actual618 receipt/source/own executable and25-case interpretation. Pure callback, input fixture, later caller getter/release and unobserved teardown limits remain explicit; this is no new execution or Rust assertion.
- `receiver618-verifier` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/verify.py](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/verify.py). SHA-256 `da615653cf595d4d4a3a1c0c6f2a8184726b63e07bb7a85872f761bfb67dcc02`. Exact independent read-only verification source, retained without a new compile or native launch.
- `whole-reset-source-8.4.20` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4415-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4415-tclResult.c.gz). SHA-256 `3afbfb3aa961d72bfdc4a5acc0b0023841c87d102ac06001d3411ab54e0de494`. Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.
- `whole-reset-source-8.5.19` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4416-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4416-tclResult.c.gz). SHA-256 `6f0ff5ffc5396df15fabc395ec4bf1d0ed5642bc1837c2e54a88a631ebe2b253`. Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.
- `whole-reset-source-8.6.18` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4417-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4417-tclResult.c.gz). SHA-256 `138c65b1f18d3451c13bcb5189e4718081cf913620889a86756ff869304760ae`. Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.
- `whole-reset-source-9.0.4` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4418-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4418-tclResult.c.gz). SHA-256 `7a46a883eb840f74548a4feab9806c9d51a8761b440cc9aea84cce60dada098a`. Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.
- `whole-reset-source-9.1.0` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4419-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4419-tclResult.c.gz). SHA-256 `dd93897b14cbf4a64999de0ba29db43c41aaebadd22ed125324574274ddc7d84`. Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclResult.c`, function `Tcl_ResetResult`, lines 780–797. Full-source SHA-256 `710dcd51ab81bafaee88e9390d261858745f637f7bfd2d1d7ceedc502604dddf`; snippet SHA-256 `8199b4f1feb3f7cbf6c1ddde58cf74c163eba23e3b58bb06b84495abd0149a6a`; retained evidence `reset-native-86`.

```text
Tcl_ResetResult(interp)
    register Tcl_Interp *interp; /* Interpreter for which to clear result. */
{
    register Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->freeProc != NULL) {
	if (iPtr->freeProc == TCL_DYNAMIC) {
	    ckfree(iPtr->result);
	} else {
	    (*iPtr->freeProc)(iPtr->result);
	}
	iPtr->freeProc = 0;
    }
    iPtr->result = iPtr->resultSpace;
    iPtr->resultSpace[0] = 0;
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_IN_PROGRESS | ERROR_CODE_SET);
}

```

tcl8.4 8.4.20, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclResult.c`, function `ResetObjResult`, lines 818–842. Full-source SHA-256 `710dcd51ab81bafaee88e9390d261858745f637f7bfd2d1d7ceedc502604dddf`; snippet SHA-256 `e542ec39d7a9a480ce555ae6ec80fb8c21012432f72bdd833c20843e9bc8426f`; retained evidence `reset-native-85`.

```text
ResetObjResult(iPtr)
    register Interp *iPtr;	/* Points to the interpreter whose result
				 * object should be reset. */
{
    register Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if ((objResultPtr->bytes != NULL)
	        && (objResultPtr->bytes != tclEmptyStringRep)) {
	    ckfree((char *) objResultPtr->bytes);
	}
	objResultPtr->bytes  = tclEmptyStringRep;
	objResultPtr->length = 0;
	if ((objResultPtr->typePtr != NULL)
	        && (objResultPtr->typePtr->freeIntRepProc != NULL)) {
	    objResultPtr->typePtr->freeIntRepProc(objResultPtr);
	}
	objResultPtr->typePtr = (Tcl_ObjType *) NULL;
    }
}

```

tcl8.5 8.5.19, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclResult.c`, function `Tcl_ResetResult`, lines 889–930. Full-source SHA-256 `dda0e0604a19da0b8998432c3dd831a705de70f96de28abfc6a4f47c6ecc7a37`; snippet SHA-256 `df05c1e9f61e9150fb3a6010117555156b7114c9e38530a60d614b9bd3819218`; retained evidence `reset-native-88`.

```text
Tcl_ResetResult(
    register Tcl_Interp *interp)/* Interpreter for which to clear result. */
{
    register Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->freeProc != NULL) {
	if (iPtr->freeProc == TCL_DYNAMIC) {
	    ckfree(iPtr->result);
	} else {
	    (*iPtr->freeProc)(iPtr->result);
	}
	iPtr->freeProc = 0;
    }
    iPtr->result = iPtr->resultSpace;
    iPtr->resultSpace[0] = 0;
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}

```

tcl8.5 8.5.19, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclResult.c`, function `ResetObjResult`, lines 951–973. Full-source SHA-256 `dda0e0604a19da0b8998432c3dd831a705de70f96de28abfc6a4f47c6ecc7a37`; snippet SHA-256 `feed8d109d1e069fa84773d574d1385f58adcdf7d96b27114471d206c0e15c3e`; retained evidence `reset-native-87`.

```text
ResetObjResult(
    register Interp *iPtr)	/* Points to the interpreter whose result
				 * object should be reset. */
{
    register Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != tclEmptyStringRep) {
	    if (objResultPtr->bytes) {
		ckfree((char *) objResultPtr->bytes);
	    }
	    objResultPtr->bytes = tclEmptyStringRep;
	    objResultPtr->length = 0;
	}
	TclFreeIntRep(objResultPtr);
	objResultPtr->typePtr = NULL;
    }
}

```

tcl8.6 8.6.18, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclResult.c`, function `Tcl_ResetResult`, lines 919–961. Full-source SHA-256 `f48609a974267129f32aee9924208175be75e1cb288c2f1b9fc9dad232f4b93b`; snippet SHA-256 `440321d5a507e5679703d0e05a05f5a6afcd980a1adaa4f7d48e3badb57568c1`; retained evidence `reset-native-90`.

```text
Tcl_ResetResult(
    Tcl_Interp *interp)		/* Interpreter for which to clear result. */
{
    Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->freeProc != NULL) {
	if (iPtr->freeProc == TCL_DYNAMIC) {
	    ckfree(iPtr->result);
	} else {
	    iPtr->freeProc(iPtr->result);
	}
	iPtr->freeProc = 0;
    }
    iPtr->result = iPtr->resultSpace;
    iPtr->resultSpace[0] = 0;
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->resetErrorStack = 1;
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}

```

tcl8.6 8.6.18, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclResult.c`, function `ResetObjResult`, lines 982–1003. Full-source SHA-256 `f48609a974267129f32aee9924208175be75e1cb288c2f1b9fc9dad232f4b93b`; snippet SHA-256 `a06d6032f455f2281260e2c1d1a27e0d8a3debd5fd5ea0d01fb2b3fd1eec1891`; retained evidence `reset-native-89`.

```text
ResetObjResult(
    Interp *iPtr)		/* Points to the interpreter whose result
				 * object should be reset. */
{
    Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != tclEmptyStringRep) {
	    if (objResultPtr->bytes) {
		ckfree(objResultPtr->bytes);
	    }
	    objResultPtr->bytes = tclEmptyStringRep;
	    objResultPtr->length = 0;
	}
	TclFreeIntRep(objResultPtr);
    }
}

```

tcl9.0 9.0.4, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclResult.c`, function `Tcl_ResetResult`, lines 389–421. Full-source SHA-256 `29f06b862e3448ecf827971f5cf1519c2cb6b8159ff99713fc0527b3308233a1`; snippet SHA-256 `4097cc4f75a9bb24fc5ae9f3b376118a3204fd30a9b0e28595bc73ffea4e6306`; retained evidence `reset-native-92`.

```text
Tcl_ResetResult(
    Tcl_Interp *interp)		/* Interpreter for which to clear result. */
{
    Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->resetErrorStack = 1;
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}

```

tcl9.0 9.0.4, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclResult.c`, function `ResetObjResult`, lines 442–463. Full-source SHA-256 `29f06b862e3448ecf827971f5cf1519c2cb6b8159ff99713fc0527b3308233a1`; snippet SHA-256 `b7358ed71467cd9ee344373d3b33fcbfba56d5b47a2684bc11e5d05bbbdd0bd0`; retained evidence `reset-native-91`.

```text
ResetObjResult(
    Interp *iPtr)		/* Points to the interpreter whose result
				 * object should be reset. */
{
    Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != &tclEmptyString) {
	    if (objResultPtr->bytes) {
		Tcl_Free(objResultPtr->bytes);
	    }
	    objResultPtr->bytes = &tclEmptyString;
	    objResultPtr->length = 0;
	}
	TclFreeInternalRep(objResultPtr);
    }
}

```

tcl9.1 9.1.0, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclResult.c`, function `Tcl_ResetResult`, lines 390–422. Full-source SHA-256 `0dc7c5d3c2bd48c90723507d3d1001f7b8aab1abcebbfaf5c1084adf16f84c3b`; snippet SHA-256 `4097cc4f75a9bb24fc5ae9f3b376118a3204fd30a9b0e28595bc73ffea4e6306`; retained evidence `reset-native-94`.

```text
Tcl_ResetResult(
    Tcl_Interp *interp)		/* Interpreter for which to clear result. */
{
    Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->resetErrorStack = 1;
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}

```

tcl9.1 9.1.0, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclResult.c`, function `ResetObjResult`, lines 443–464. Full-source SHA-256 `0dc7c5d3c2bd48c90723507d3d1001f7b8aab1abcebbfaf5c1084adf16f84c3b`; snippet SHA-256 `b7358ed71467cd9ee344373d3b33fcbfba56d5b47a2684bc11e5d05bbbdd0bd0`; retained evidence `reset-native-93`.

```text
ResetObjResult(
    Interp *iPtr)		/* Points to the interpreter whose result
				 * object should be reset. */
{
    Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != &tclEmptyString) {
	    if (objResultPtr->bytes) {
		Tcl_Free(objResultPtr->bytes);
	    }
	    objResultPtr->bytes = &tclEmptyString;
	    objResultPtr->length = 0;
	}
	TclFreeInternalRep(objResultPtr);
    }
}

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The whole original request/probe/selected Make provenance,16 process receipts, raw streams, strict verification/maps and fresh own gzip images are retained. Restore each original gzip payload to exact SHA/size/mode/path, join inherited SDK payloads through the already canonical full-hash map and independently supply matching hash-only system inputs; replay needs fresh output paths. SDK configure/build replay and source-to-library identity are unavailable. Each original link map names an unretained compiler ephemeral object with null digest. Pure observers and explicitly prepared info/COPY inputs bound this experiment; no observed mutating reentry, teardown callbacks, Jim worker or Rust assertion exists.
