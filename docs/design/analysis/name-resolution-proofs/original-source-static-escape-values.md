# naming.source.original-static-escape-values

Kind: `native-observation`

## Problem statement

Static source escapes are release-dependent. Assuming a uniform two-digit hex boundary can turn a legacy x25b source word into percent-b even when the provider produces a single bracket character; braces preserve a different literal value.

## Question

What are the caught status and length/hex results of the thirteen exact bare, quoted and braced source escape forms under the six retained providers, separately through CLI binary-scan projection and counted SDK result getters?

## Conclusion

C8.4.20 and C8.5.19 produce hex 5b and length 1 for bare/quoted x25b and x25B; C8.6.18, C9.0.4, C9.1.0 and current Jim produce 2562 or 2542 and length 2. Explicit x25 followed by u0062 produces 2562 and length 2 under all six. Braced backslashes remain literal. The independently recorded x4142, x25z, x0 and cu controls retain their exact per-release results. The independent SDK companion agrees on the twelve nonzero-byte controls. Its counted result getter returns c080 for x0 under every C release and 00 under Jim, while CLI binary scan projects 00 under all six.

## Scope

Six original fresh provider processes with six version rows and 78 caught source observations. Fixed ASCII LF source-file CLI; thirteen scripts run sequentially within each provider process. The original request ID naming.word.original-source-escape-version-boundaries remains immutable while this canonical question indexes those same finite source forms. Hex is the actual binary scan H* projection and length is string length; neither measures native string getter extent or original object identity. The x0 projection records 00 but grants no raw-NUL/C080 native input recipe. No format dispatch, source rewrite, arbitrary escape equivalence, compiler admission, physical frame/cell/cache/header, Native Normal transfer or BIG-IP observation follows. Independent SDK companion: six compile/process exits 0, six counted version rows and 78 source-result rows; each source case has its own initialized interpreter. Inputs remain ASCII NUL-terminated source, not original object vectors. The companion uses Tcl_Eval/Jim_Eval and counted result getters. Every printed pointer bit is 0 with no sampled argument, so no address or original-object identity follows. Raw SDK question/scope fields contain unrelated forward boilerplate and queued launch metadata; exact source/cases, provider receipts and outputs retain this question's actual channel. Only these reached getter bytes are measured; input NUL representation, arbitrary strings, cache/header ownership and source/compiler/Normal permissions remain separate. Linked Rust source controls cover only their named literal/geometry or source version-advice assertions; they do not replay counted getters or supply an independently executed Rust result.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs thirteen caught fixed source scripts sequentially after its actual version row. Results use string length and binary scan H*, independently of native getters. Separately, C SDK evaluates each of the thirteen exact ASCII source strings in its own freshly initialized interpreter through Tcl_Eval or Jim_Eval, then samples the result through counted Tcl_GetStringFromObj or Jim_GetString. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Bare/quoted x25b and x25B yield binary hex5b length1; x4142 yields42 length1. Explicit x25 followed by u0062 yields2562 length2. Braced forms preserve literal backslashes. Each x0 result projects00 with string length1; this projection does not identify native string-getter bytes, original objects or headers. The independent direct native result getter agrees on the other twelve hex values; x0 returnsc080 under each C release and00 under Jim, unlike the CLI binary projection00. No input-object or pointer identity is measured.

### tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs thirteen caught fixed source scripts sequentially after its actual version row. Results use string length and binary scan H*, independently of native getters. Separately, C SDK evaluates each of the thirteen exact ASCII source strings in its own freshly initialized interpreter through Tcl_Eval or Jim_Eval, then samples the result through counted Tcl_GetStringFromObj or Jim_GetString. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Bare/quoted x25b yield binary hex2562 length2; x25B yields2542 length2 and x4142 yields413432 length3. Explicit x25 followed by u0062 yields2562 length2. Braced forms preserve literal backslashes. Each x0 result projects00 with string length1; this projection does not identify native string-getter bytes, original objects or headers. The independent direct native result getter agrees on the other twelve hex values; x0 returnsc080 under each C release and00 under Jim, unlike the CLI binary projection00. No input-object or pointer identity is measured.

### bigip

Status: `not-tested`.

No appliance observation of these exact source literals.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs), `analyser::diagnostics::version_gate::tests::original_format_gates_read_literal_values_without_promoting_dynamic_words` (linked): Explicit hex/Unicode escape boundaries supply selected literal format bytes and version advice; greedy C8.4/C8.5 x25b contains no percent conversion. Braced/dynamic/captured/expanded, shadowed, moved or unavailable templates do not borrow current values or fabricated operand ordinals.
- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `registry_invocation::source_structure::tests::original_source_static_values_keep_escapes_separate_from_frozen_runtime_values` (linked): Six authoring contexts compare genuine bare, quoted and braced original words. Greedy C8.4/C8.5 x25b forms produce bracket; C8.6+ and Jim form percent-b. Explicit x25 followed by u0062 forms percent-b under all six, braces remain literal, and a written dynamic substitution cannot borrow a prior cell value. The exact original whole-word extent remains independent of these formed values.

## Evidence

- [rust/tcl-registry/tests/data/native_source_escapes_original](../../../../rust/tcl-registry/tests/data/native_source_escapes_original)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_source_escape_getter_original](../../../../rust/tcl-registry/tests/data/native_source_escape_getter_original)
