# naming.source.original-crlf-evaluation-channels

Kind: `native-observation`

## Problem statement

A literal CRLF pair inside an original quoted source word can be confused with file-channel newline translation. Source syntax projection must retain the actual evaluation channel.

## Question

Does a literal CRLF pair in a quoted Tcl source word survive counted direct evaluation, and does evaluating the same bytes through source perform file newline translation?

## Conclusion

Twelve fresh initialized host processes exit0 with empty stderr, using the six unchanged executables and compile receipts from the dictionary319 capture; no new compiler command runs for this question. Direct counted source evaluation preserves CRLF in all five C releases and Jim, yielding /ROOT/one CR LF two$literal, length22 and codepoints13,10. Evaluating the same payload file through original source yields LF and length21 for C8.4–9.1; Jim retains CRLF and length22. Every original value is also retained as counted hex independently of the ASCII codepoint query. The fixed direct source assertion reflects these channel-specific observations; no software assertion pass is inferred.

## Scope

Exactly one literal ASCII source payload containing actual0d0a and a backslash-escaped literal dollar, evaluated directly and through source on each pinned provider. Whole counted original value, separate length/all-codepoint query, version, process stdout/stderr/receipt, unchanged absolute file path, query and payload bytes are retained. Six original319 compile receipts/ELFs are reused with their exact associations and pins; this question contributes twelve process commands and zero compiler commands. This finite file channel comparison establishes no general non-ASCII, filename identity, private object/header/cache, frame/body/compiler admission, arbitrary source replacement, editor equivalence or BIG-IP observation.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh original counted direct source; independent original source file invocation and separate query. Actual payload0d0a differs from a Tcl CRLF escape. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Direct counted source retains CRLF, length22 and codepoints13,10. Original source file loading translates CRLF to LF, length21 and codepoint10. Both original evaluations and separate length/all-ASCII-codepoint queries return0. Two fresh host processes exit0 with empty stderr.

### jim

Status: `observed`. Channel: Fresh original counted direct source; independent original source file invocation and separate query. Actual payload0d0a differs from a Tcl CRLF escape. Version: jim 0.84-9-g5bac7c9.

Direct and file controls both retain CRLF, length22 and codepoints13,10. Both original evaluations and separate length/all-ASCII-codepoint queries return0. Two fresh host processes exit0 with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [rust/tcl-compiler/src/auto_path_eval/source_expression.rs](../../../../rust/tcl-compiler/src/auto_path_eval/source_expression.rs), `capture_source_path_expression`: Retain exact original source path parts and their actual channel; the finite direct CRLF projection does not borrow C file-loading translation.
- [rust/tcl-compiler/src/auto_path_eval/source_expression.rs](../../../../rust/tcl-compiler/src/auto_path_eval/source_expression.rs), `auto_path_eval::source_expression::tests::original_source_path_parts_keep_selected_variable_roots_and_missing_premises` (linked): The fixed direct literal CRLF source projects /ROOT/one CR LF two$literal without translating it to LF; selected variable grammar and missing/stale/foreign owner refusals stay independent. Native330 separately measures direct/file channels; this authored source assertion grants no editor equivalence or successful Rust execution.

## Evidence

- [rust/tcl-registry/tests/data/native_original_source_crlf330/8.4.20](../../../../rust/tcl-registry/tests/data/native_original_source_crlf330/8.4.20)
- [rust/tcl-registry/tests/data/native_original_source_crlf330/8.5.19](../../../../rust/tcl-registry/tests/data/native_original_source_crlf330/8.5.19)
- [rust/tcl-registry/tests/data/native_original_source_crlf330/8.6.18](../../../../rust/tcl-registry/tests/data/native_original_source_crlf330/8.6.18)
- [rust/tcl-registry/tests/data/native_original_source_crlf330/9.0.4](../../../../rust/tcl-registry/tests/data/native_original_source_crlf330/9.0.4)
- [rust/tcl-registry/tests/data/native_original_source_crlf330/9.1.0](../../../../rust/tcl-registry/tests/data/native_original_source_crlf330/9.1.0)
- [rust/tcl-registry/tests](../../../../rust/tcl-registry/tests)
- [rust/tcl-registry/tests/data/native_original_source_crlf330/jim](../../../../rust/tcl-registry/tests/data/native_original_source_crlf330/jim)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
