# naming.alias.jim-original-alias-diagnostic-name-extent

Kind: `native-observation`

## Problem statement

Counted command-name values and public alias diagnostic text can retain different byte extents. Missing-command lookup and a successful original procedure lookup followed by a not-an-alias error require separate observations, including names produced at evaluation with NUL, UTF-8 and invalid bytes.

## Question

What exact public name lengths and missing-command/not-an-alias diagnostic bytes do two original Jim info-alias sources return for middle-NUL, leading-NUL, UTF-8 and invalid-byte names?

## Conclusion

Jim completes both original sources with guest code0. Each source reports original string lengths17,13,7,7 for middle-NUL, leading-NUL, UTF-8 and invalid-byte names, respectively; each caught alias query reports code1. Missing-command diagnostics report invalid command name, while genuine original procedure declarations produce command is not an alias diagnostics. In both diagnostic forms, the middle-NUL rendered name stops at r2286_extent, the leading-NUL rendered name is empty, the UTF-8 name retains bytes72323238365fc3a9 and the invalid-byte name retains bytes72323238365fff. The full original result hex retains every label, length, catch code and message, including invalid UTF-8. These public outputs distinguish original counted values from diagnostic rendering without identifying a private command key, header, cache, allocation length or lifetime. All five C providers execute the explicit NOT_APPLICABLE capability branch; their info surface is not substituted with a C alias API. All twelve host processes exit0 with empty stderr. The separately retained Jim_SetResultFormatted source copy/excerpt describes a formatting implementation; it remains read-only source evidence distinct from the measured public bytes and does not expand this finite observation.

A marked Runtime definition links twelve exact original public alias-diagnostic windows under the explicit selected-core and Jim binary-adapter fixture.

A marked VM definition links twelve exact original alias-diagnostic windows under its explicit core/compiler and Jim byte-producer fixture.

## Scope

Two unchanged ASCII LF originals with four name cases each. binary format H* produces NUL, UTF-8 and FF bytes at evaluation; there is no literal NUL/non-ASCII source. Each original executes independently in a fresh fully initialised provider through the unchanged counted-source CAPI driver. Complete original source/request/compiler/ELF/process/stdout/stderr/code/result/version and required pins are retained. No private object/header/key/lookup identity, physical cache, arbitrary error producer, Rust assertion or BIG-IP result is observed.

The body does not replay external processes, inspect private slots/headers, infer a full Jim distribution or validate a source-character channel from binary-produced bytes. The exact original sources and C NOT_APPLICABLE/Jim result boundaries remain intact. No assertion outcome is attached; fixture selection cannot donate general worker, library or lookup authority.

This source definition supplies no process replay or passing software outcome. Binary-produced bytes do not establish physical source-character ingress, a full Jim distribution, header/cache identity or private lookup extent. C unavailable branches remain independent original results, without substituting another alias API. Existing raw provider evidence is unchanged.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: 8.4.20. Build: Exact unchanged counted-source CAPI driver, actual cc/ELF receipts and required archive/header/provider-source pins retained.. Channel: Two independent fresh fullInit original source evaluations; separate VERSION and ORIGINAL counted hex completion/result channels, no outer source envelope or text normalization.. Dialect: Pinned original provider API/source purpose.

Both original capability branches publicly complete guest0 with NOT_APPLICABLE. This provider does not answer the original Jim info-alias diagnostic question; no alternative C alias API is substituted. Two fresh host processes exit0 with empty stderr.

### tcl8.5

Status: `not-tested`. Version: 8.5.19. Build: Exact unchanged counted-source CAPI driver, actual cc/ELF receipts and required archive/header/provider-source pins retained.. Channel: Two independent fresh fullInit original source evaluations; separate VERSION and ORIGINAL counted hex completion/result channels, no outer source envelope or text normalization.. Dialect: Pinned original provider API/source purpose.

Both original capability branches publicly complete guest0 with NOT_APPLICABLE. This provider does not answer the original Jim info-alias diagnostic question; no alternative C alias API is substituted. Two fresh host processes exit0 with empty stderr.

### tcl8.6

Status: `not-tested`. Version: 8.6.18. Build: Exact unchanged counted-source CAPI driver, actual cc/ELF receipts and required archive/header/provider-source pins retained.. Channel: Two independent fresh fullInit original source evaluations; separate VERSION and ORIGINAL counted hex completion/result channels, no outer source envelope or text normalization.. Dialect: Pinned original provider API/source purpose.

Both original capability branches publicly complete guest0 with NOT_APPLICABLE. This provider does not answer the original Jim info-alias diagnostic question; no alternative C alias API is substituted. Two fresh host processes exit0 with empty stderr.

### tcl9.0

Status: `not-tested`. Version: 9.0.4. Build: Exact unchanged counted-source CAPI driver, actual cc/ELF receipts and required archive/header/provider-source pins retained.. Channel: Two independent fresh fullInit original source evaluations; separate VERSION and ORIGINAL counted hex completion/result channels, no outer source envelope or text normalization.. Dialect: Pinned original provider API/source purpose.

Both original capability branches publicly complete guest0 with NOT_APPLICABLE. This provider does not answer the original Jim info-alias diagnostic question; no alternative C alias API is substituted. Two fresh host processes exit0 with empty stderr.

### tcl9.1

Status: `not-tested`. Version: 9.1.0. Build: Exact unchanged counted-source CAPI driver, actual cc/ELF receipts and required archive/header/provider-source pins retained.. Channel: Two independent fresh fullInit original source evaluations; separate VERSION and ORIGINAL counted hex completion/result channels, no outer source envelope or text normalization.. Dialect: Pinned original provider API/source purpose.

Both original capability branches publicly complete guest0 with NOT_APPLICABLE. This provider does not answer the original Jim info-alias diagnostic question; no alternative C alias API is substituted. Two fresh host processes exit0 with empty stderr.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Exact unchanged counted-source CAPI driver, actual cc/ELF receipts and required archive/header/provider-source pins retained.. Channel: Two independent fresh fullInit original source evaluations; separate VERSION and ORIGINAL counted hex completion/result channels, no outer source envelope or text normalization.. Dialect: Pinned original provider API/source purpose.

Both unchanged originals complete guest0 with all four exact original-name lengths17/13/7/7 and caught query code1. Missing-command and genuine-procedure/not-alias diagnostics retain distinct whole messages; NUL truncates the rendered name at its first NUL, while UTF-8 and FF bytes remain exact in full result hex. Original counted values and public rendering are distinct; private key/header/cache/allocation properties are unobserved. Two fresh host processes exit0 with empty stderr.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Historical references

- `native_jim_alias_error_extent279-8.4.20-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-original-missing-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-missing-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-missing-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-original-missing-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-missing-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-missing-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-original-missing-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-missing-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-missing-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-original-nonalias-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-nonalias-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-nonalias-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-original-nonalias-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-nonalias-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-nonalias-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-original-nonalias-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-nonalias-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-nonalias-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.4.20-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-original-missing-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-missing-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-missing-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-original-missing-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-missing-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-missing-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-original-missing-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-missing-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-missing-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-original-nonalias-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-nonalias-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-nonalias-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-original-nonalias-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-nonalias-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-nonalias-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-original-nonalias-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-nonalias-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-nonalias-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.5.19-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-original-missing-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-missing-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-missing-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-original-missing-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-missing-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-missing-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-original-missing-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-missing-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-missing-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-original-nonalias-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-nonalias-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-nonalias-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-original-nonalias-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-nonalias-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-nonalias-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-original-nonalias-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-nonalias-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-nonalias-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-8.6.18-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-original-missing-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-missing-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-missing-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-original-missing-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-missing-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-missing-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-original-missing-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-missing-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-missing-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-original-nonalias-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-nonalias-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-nonalias-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-original-nonalias-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-nonalias-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-nonalias-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-original-nonalias-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-nonalias-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-nonalias-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.0.4-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-original-missing-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-missing-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-missing-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-original-missing-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-missing-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-missing-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-original-missing-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-missing-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-missing-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-original-nonalias-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-nonalias-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-nonalias-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-original-nonalias-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-nonalias-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-nonalias-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-original-nonalias-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-nonalias-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-nonalias-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-9.1.0-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-capture.py` (input): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/capture.py). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-original-missing-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-missing-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-missing-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-original-missing-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-missing-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-missing-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-original-missing-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-missing-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-missing-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-original-nonalias-alias-diagnostic-extent-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-nonalias-alias-diagnostic-extent/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-nonalias-alias-diagnostic-extent/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-original-nonalias-alias-diagnostic-extent-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-nonalias-alias-diagnostic-extent/stderr](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-nonalias-alias-diagnostic-extent/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-original-nonalias-alias-diagnostic-extent-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-nonalias-alias-diagnostic-extent/stdout](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-nonalias-alias-diagnostic-extent/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-original-missing-alias-diagnostic-extent.tcl` (input): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/original-missing-alias-diagnostic-extent.tcl](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/original-missing-alias-diagnostic-extent.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-original-nonalias-alias-diagnostic-extent.tcl` (input): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/original-nonalias-alias-diagnostic-extent.tcl](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/original-nonalias-alias-diagnostic-extent.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-probe.c` (input): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/probe.c](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/probe.c). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-request.json` (input): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-request-Jim_SetResultFormatted.c.txt` (source-anchor): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/Jim_SetResultFormatted.c.txt](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/Jim_SetResultFormatted.c.txt). Exact separately hashed read-only formatting excerpt; source interpretation does not donate private state to public observations.
- `native_jim_alias_error_extent279-request-jim.c` (source-anchor): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-request-original-missing-alias-diagnostic-extent.tcl` (input): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/original-missing-alias-diagnostic-extent.tcl](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/original-missing-alias-diagnostic-extent.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-request-original-nonalias-alias-diagnostic-extent.tcl` (input): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/original-nonalias-alias-diagnostic-extent.tcl](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/original-nonalias-alias-diagnostic-extent.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_jim_alias_error_extent279-request-request.json` (input): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/request.json](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/request.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

## Source inspection

jim 0.84-9-g5bac7c9 independently associated by the retained provider receipt; source content hash separately pinned., revision `Source-control revision unrecorded for this artifact.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetResultFormatted`, lines 17112–17178. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
 * Very simple printf-like formatting, designed for error messages.
 *
 * The format may contain up to 5 '%s' or '%#s', corresponding to variable arguments.
 * The resulting string is created and set as the result.
 *
 * Each '%s' should correspond to a regular string parameter.
 * Each '%#s' should correspond to a (Jim_Obj *) parameter.
 * Any other printf specifier is not allowed (but %% is allowed for the % character).
 *
 * e.g. Jim_SetResultFormatted(interp, "Bad option \"%#s\" in proc \"%#s\"", optionObjPtr, procNamePtr);
 *
 * Note: We take advantage of the fact that printf has the same behaviour for both %s and %#s
 *
 * Note that any Jim_Obj parameters with zero ref count will be freed as a result of this call.
 */
void Jim_SetResultFormatted(Jim_Interp *interp, const char *format, ...)
{
    /* Initial space needed */
    int len = strlen(format);
    int extra = 0;
    int n = 0;
    const char *params[5];
    int nobjparam = 0;
    Jim_Obj *objparam[5];
    char *buf;
    va_list args;
    int i;

    va_start(args, format);

    for (i = 0; i < len && n < 5; i++) {
        int l;

        if (strncmp(format + i, "%s", 2) == 0) {
            params[n] = va_arg(args, char *);

            l = strlen(params[n]);
        }
        else if (strncmp(format + i, "%#s", 3) == 0) {
            Jim_Obj *objPtr = va_arg(args, Jim_Obj *);

            params[n] = Jim_GetString(objPtr, &l);
            objparam[nobjparam++] = objPtr;
            Jim_IncrRefCount(objPtr);
        }
        else {
            if (format[i] == '%') {
                i++;
            }
            continue;
        }
        n++;
        extra += l;
    }

    len += extra;
    buf = Jim_Alloc(len + 1);
    len = snprintf(buf, len + 1, format, params[0], params[1], params[2], params[3], params[4]);

    va_end(args);

    Jim_SetResult(interp, Jim_NewStringObjNoAlloc(interp, buf, len));

    for (i = 0; i < nobjparam; i++) {
        Jim_DecrRefCount(interp, objparam[i]);
    }
}

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

- [runtime/rust/src/cmd_info/native_jim_inventory_tests.rs](../../../../runtime/rust/src/cmd_info/native_jim_inventory_tests.rs), `original_result`: Decode the retained original ORIGINAL code/result-hex fields for exact public comparisons; decoding supplies no header/cache/lookup identity or provider roster.
- [runtime/rust/src/cmd_info/native_jim_inventory_tests.rs](../../../../runtime/rust/src/cmd_info/native_jim_inventory_tests.rs), `cmd_info::native_jim_inventory_tests::original_alias_diagnostic_extents_match_all_12_native_public_windows` (linked): Two unchanged ASCII fixture programs across six selected software cores compare all12 original public completion/result windows and require no pending Host cause. Jim explicitly installs only the byte-producing binary adapter; original C NOT_APPLICABLE branches remain unchanged. NUL, UTF8 e-acute and FF are produced at evaluation, not literal source characters.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

- [rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs), `original_result`: Retain the original public alias diagnostic completion/result window without reconstructing a native name slot or header from diagnostic bytes.
- [rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs), `cmd_info::native_jim_inventory_tests::original_alias_diagnostic_extents_match_all_12_native_public_windows` (linked): Two unchanged ASCII originals across six selected software cores compare all12 public code/result windows and require no Host cause. Jim registers only the binary byte producer; original C NOT_APPLICABLE branches remain. NUL, UTF8 e-acute and FF are evaluated binary-produced bytes, not source-character inputs.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
