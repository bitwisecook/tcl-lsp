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

## Historical references

- `reset-native-0` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/closure.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/closure.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-1` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-2` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-3` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/compiler-version.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-4` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/launch.py](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/launch.py). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-5` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/manifest.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/manifest.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-6` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/prepare.py](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/prepare.py). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-7` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/probe.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/probe.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-8` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.d). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-9` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-10` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-11` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-12` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/compile.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-13` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-14` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-15` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-16` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/dependencies.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-17` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-18` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-19` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-20` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/execute.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-21` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/link.map). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-22` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.4.20/probe.elf.gz). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-23` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.d). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-24` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-25` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-26` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-27` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/compile.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-28` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-29` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-30` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-31` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/dependencies.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-32` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-33` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-34` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-35` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/execute.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-36` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/link.map). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-37` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.5.19/probe.elf.gz). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-38` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.d). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-39` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-40` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-41` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-42` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/compile.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-43` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-44` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-45` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-46` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/dependencies.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-47` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-48` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-49` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-50` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/execute.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-51` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/link.map). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-52` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/8.6.18/probe.elf.gz). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-53` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.d). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-54` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-55` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-56` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-57` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/compile.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-58` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-59` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-60` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-61` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/dependencies.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-62` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-63` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-64` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-65` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/execute.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-66` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/link.map). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-67` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.0.4/probe.elf.gz). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-68` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.d](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.d). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-69` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-70` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-71` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-72` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/compile.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-73` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-74` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-75` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-76` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/dependencies.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-77` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.receipt.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.receipt.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-78` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.stderr](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.stderr). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-79` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.stdout](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.stdout). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-80` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.verification.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/execute.verification.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-81` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/link.map](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/link.map). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-82` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/probe.elf.gz](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/providers/9.1.0/probe.elf.gz). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-83` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/request.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/request.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-84` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/selected-makefile-builds-original614c.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/selected-makefile-builds-original614c.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-85` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.4.20/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.4.20/ResetObjResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-86` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.4.20/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.4.20/Tcl_ResetResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-87` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.5.19/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.5.19/ResetObjResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-88` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.5.19/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.5.19/Tcl_ResetResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-89` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.6.18/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.6.18/ResetObjResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-90` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.6.18/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/8.6.18/Tcl_ResetResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-91` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.0.4/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.0.4/ResetObjResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-92` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.0.4/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.0.4/Tcl_ResetResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-93` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.1.0/ResetObjResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.1.0/ResetObjResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-94` (input): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.1.0/Tcl_ResetResult.c](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows/9.1.0/Tcl_ResetResult.c). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-95` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/source-windows.json). Exact whole original request, preparation, raw stream, process/verification receipt, dependency map or fresh own output; original bytes preserved without retrospective outcome invention.
- `reset-native-96` (observation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/outer-launch.log](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/outer-launch.log). Exact saved outer launcher stream. Root reports its tool exit separately; no filesystem outer process receipt is fabricated.
- `reset-native-97` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/static-review/receiver.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/static-review/receiver.json). Exact independent static request/protocol review, separate from actual producer and API observations.
- `reset-native-98` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation/manifest.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation/manifest.json). Exact read-only16-process/25-case interpretation joins actual original bytes and separates pure field/free snapshots, caller getter/release and observer-disabled teardown. No new execution or Rust assertion.
- `reset-native-99` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation/interpretation.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/independent-interpretation/interpretation.json). Exact read-only16-process/25-case interpretation joins actual original bytes and separates pure field/free snapshots, caller getter/release and observer-disabled teardown. No new execution or Rust assertion.
- `canonical-sdk-input-map` (provider): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/recorded-path-map.json](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/recorded-path-map.json). Already retained exact original SDK/source/library/header/input map reused by full original/payload hashes. The reset request retains its own complete required input list and own producer gates; setter outputs and observations are not reset evidence.
- `canonical-sdk-issuer-request` (provider): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614c/request.json](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/614c/request.json). Exact prior admitted selected Makefile/SDK input issuer only. This request supplies no reset executable or reset API result; all reset own outputs and receipts are independently retained.
- `reset-recorded-path-map` (provider): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/recorded-path-map.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/recorded-path-map.json). Exact reset input/output/whole-SHA SDK reuse, compressed own image restoration and hash-only system prerequisite joins.
- `receiver618-manifest.json` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/manifest.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/manifest.json). Exact independent read-only actual618 receipt/source/own executable and25-case interpretation. Pure callback, input fixture, later caller getter/release and unobserved teardown limits remain explicit; this is no new execution or Rust assertion.
- `receiver618-interpretation.json` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/interpretation.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/interpretation.json). Exact independent read-only actual618 receipt/source/own executable and25-case interpretation. Pure callback, input fixture, later caller getter/release and unobserved teardown limits remain explicit; this is no new execution or Rust assertion.
- `receiver618-case-observations.json` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/case-observations.json](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/case-observations.json). Exact independent read-only actual618 receipt/source/own executable and25-case interpretation. Pure callback, input fixture, later caller getter/release and unobserved teardown limits remain explicit; this is no new execution or Rust assertion.
- `receiver618-verifier` (reconfirmation): [runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/verify.py](../../../../runtime/rust/tests/data/native_reset_result_error_header_retirement_original/618/receiver-crossreview/verify.py). Exact independent read-only verification source, retained without a new compile or native launch.
- `whole-reset-source-8.4.20` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4415-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4415-tclResult.c.gz). Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.
- `whole-reset-source-8.5.19` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4416-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4416-tclResult.c.gz). Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.
- `whole-reset-source-8.6.18` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4417-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4417-tclResult.c.gz). Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.
- `whole-reset-source-9.0.4` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4418-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4418-tclResult.c.gz). Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.
- `whole-reset-source-9.1.0` (source-anchor): [runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4419-tclResult.c.gz](../../../../runtime/rust/tests/data/native_explicit_error_code_header_setter_original/original-inputs/4419-tclResult.c.gz). Exact complete original Tcl result source gzip reused by verified whole original/payload hash. Static source inspection and selected library provenance stay separate; source-to-library configure/build replay is absent.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclResult.c`, function `Tcl_ResetResult`, lines 780–797. Retained reference `reset-native-86`.

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

tcl8.4 8.4.20, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclResult.c`, function `ResetObjResult`, lines 818–842. Retained reference `reset-native-85`.

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

tcl8.5 8.5.19, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclResult.c`, function `Tcl_ResetResult`, lines 889–930. Retained reference `reset-native-88`.

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

tcl8.5 8.5.19, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclResult.c`, function `ResetObjResult`, lines 951–973. Retained reference `reset-native-87`.

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

tcl8.6 8.6.18, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclResult.c`, function `Tcl_ResetResult`, lines 919–961. Retained reference `reset-native-90`.

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

tcl8.6 8.6.18, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclResult.c`, function `ResetObjResult`, lines 982–1003. Retained reference `reset-native-89`.

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

tcl9.0 9.0.4, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclResult.c`, function `Tcl_ResetResult`, lines 389–421. Retained reference `reset-native-92`.

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

tcl9.0 9.0.4, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclResult.c`, function `ResetObjResult`, lines 442–463. Retained reference `reset-native-91`.

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

tcl9.1 9.1.0, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclResult.c`, function `Tcl_ResetResult`, lines 390–422. Retained reference `reset-native-94`.

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

tcl9.1 9.1.0, revision `Exact retained whole source and physical LF function extent; independently joined source inspection, separate from finite original process observations`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclResult.c`, function `ResetObjResult`, lines 443–464. Retained reference `reset-native-93`.

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
