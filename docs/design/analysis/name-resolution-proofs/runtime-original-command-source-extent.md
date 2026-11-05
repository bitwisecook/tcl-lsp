# naming.runtime.original-command-source-extent

Kind: `native-observation`

## Problem statement

A command word span and a public command-source excerpt can end at different positions. Dropping original trailing spaces, tabs or delimiters changes error/frame reporting, while provider language rules can independently change an evaluated script value before that reporting.

## Question

Which exact original source bytes, particularly trailing spaces/tabs/continuations and closing delimiters, are retained in public errorInfo command excerpts and info frame cmd for parsed or compiled script/procedure commands?

## Conclusion

All five recorded C releases catch each of seven error specimens with code one and marker, and read ::errorInfo successfully. Its public command excerpt excludes leading indentation and the semicolon/newline separator but retains trailing spaces or literal tabs and complete quote/brace closers. EOF, semicolon, newline, quoted and braced examples retain three trailing spaces; the tab example retains tab-space-tab. The continuation script VALUE reported by SOURCE is error marker with a leading space and two trailing spaces on C, because the outer braced value has already formed that script; the error excerpt retains its two spaces. The source_error procedure likewise reports its error command with three trailing spaces. C8.5/8.6/9.0/9.1 catch info frame successfully and return source_frame_target plus three spaces in both measured caller-command fields; C8.4 catches unsupported info frame with code one and executes no frame branch. Jim 0.84-9-g5bac7c9 catches all seven errors with marker, but every explicit ::errorInfo read is caught with code one and its no-such-variable result. Jim info frame is actually available with code zero; both measured cmd fields are source_frame_target without trailing spaces. Its reported continuation script retains the literal backslash/newline and three following spaces, unlike the C-formed value. These public outcomes and their complete hex-encoded spellings are retained independently of raw source/request anticipations. Six whole processes exit zero with empty stderr; BIG-IP is not tested. No private frame/header/table identity, bytecode engine choice, Native compiler admission, list-object representation, NUL/Unicode, arbitrary script law or edit authority follows.

A marked default Runtime definition binds four concrete public errorInfo expectations to the shared command-slice/logging owner.

## Scope

One exact ASCII LF CLI file, including literal ASCII tabs and a literal braced backslash-newline, per fresh provider process. Seven eval specimens print their actual script VALUES as hex before caught evaluation, keeping value formation distinct from original file bytes. A procedure error and two info-frame caller commands are separate public controls. No compile/disassemble branch labels the actual engine; interpreted-versus-bytecode implementation cannot be inferred solely from these excerpts. The optional info-frame branch is governed by its actual caught availability result, not request expectations. C8.4 supplies no frame cmd observation; Jim supplies actual cmd text but no successful ::errorInfo read in this script. No general exception-option/errorStack surface or provider-wide absence is asserted.

This body uses the default leak_free interpreter and checks four software result strings. Its expectations do not establish every provider release, source byte offset, object/header identity, native opcode, compiled engine choice or a general errorStack protocol. Exact original CLI command-extent observations and their availability boundaries stay independent; no executed software result is attached.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fixed original ASCII LF source-file CLI; each specimen original script is also output as hex. Source file tabs are literal ASCII tabs. The continuation specimen is a literal backslash-newline inside a braced value; the language performs its specified brace continuation substitution. No Unicode/raw-NUL/list-object/private command table or compiler admission assertion is inferred. Version: tcl8.4 8.4.20.

Actual stock 8.4.20 retains trailing spaces/tabs and quote/brace closers in all seven successful ::errorInfo reads; each specimen catches marker with code one. The reported C continuation VALUE has two trailing spaces after outer braced formation. Procedure errorInfo retains error proc_marker plus three spaces. info frame is caught unsupported with code one and no frame cmd branch is executed. Original complete hex rows supply the exact public strings without a private engine/frame or arbitrary script claim. Whole process exits zero with empty stderr.

### tcl8.5

Status: `observed`. Channel: Fixed original ASCII LF source-file CLI; each specimen original script is also output as hex. Source file tabs are literal ASCII tabs. The continuation specimen is a literal backslash-newline inside a braced value; the language performs its specified brace continuation substitution. No Unicode/raw-NUL/list-object/private command table or compiler admission assertion is inferred. Version: tcl8.5 8.5.19.

Actual stock 8.5.19 retains trailing spaces/tabs and quote/brace closers in all seven successful ::errorInfo reads; each specimen catches marker with code one. The reported C continuation VALUE has two trailing spaces after outer braced formation. Procedure errorInfo retains error proc_marker plus three spaces. info frame is caught available with code zero/result2 and both measured cmd fields are source_frame_target plus three spaces. Original complete hex rows supply the exact public strings without a private engine/frame or arbitrary script claim. Whole process exits zero with empty stderr.

### tcl8.6

Status: `observed`. Channel: Fixed original ASCII LF source-file CLI; each specimen original script is also output as hex. Source file tabs are literal ASCII tabs. The continuation specimen is a literal backslash-newline inside a braced value; the language performs its specified brace continuation substitution. No Unicode/raw-NUL/list-object/private command table or compiler admission assertion is inferred. Version: tcl8.6 8.6.18.

Actual stock 8.6.18 retains trailing spaces/tabs and quote/brace closers in all seven successful ::errorInfo reads; each specimen catches marker with code one. The reported C continuation VALUE has two trailing spaces after outer braced formation. Procedure errorInfo retains error proc_marker plus three spaces. info frame is caught available with code zero/result2 and both measured cmd fields are source_frame_target plus three spaces. Original complete hex rows supply the exact public strings without a private engine/frame or arbitrary script claim. Whole process exits zero with empty stderr.

### tcl9.0

Status: `observed`. Channel: Fixed original ASCII LF source-file CLI; each specimen original script is also output as hex. Source file tabs are literal ASCII tabs. The continuation specimen is a literal backslash-newline inside a braced value; the language performs its specified brace continuation substitution. No Unicode/raw-NUL/list-object/private command table or compiler admission assertion is inferred. Version: tcl9.0 9.0.4.

Actual stock 9.0.4 retains trailing spaces/tabs and quote/brace closers in all seven successful ::errorInfo reads; each specimen catches marker with code one. The reported C continuation VALUE has two trailing spaces after outer braced formation. Procedure errorInfo retains error proc_marker plus three spaces. info frame is caught available with code zero/result2 and both measured cmd fields are source_frame_target plus three spaces. Original complete hex rows supply the exact public strings without a private engine/frame or arbitrary script claim. Whole process exits zero with empty stderr.

### tcl9.1

Status: `observed`. Channel: Fixed original ASCII LF source-file CLI; each specimen original script is also output as hex. Source file tabs are literal ASCII tabs. The continuation specimen is a literal backslash-newline inside a braced value; the language performs its specified brace continuation substitution. No Unicode/raw-NUL/list-object/private command table or compiler admission assertion is inferred. Version: tcl9.1 9.1.0.

Actual stock 9.1.0 retains trailing spaces/tabs and quote/brace closers in all seven successful ::errorInfo reads; each specimen catches marker with code one. The reported C continuation VALUE has two trailing spaces after outer braced formation. Procedure errorInfo retains error proc_marker plus three spaces. info frame is caught available with code zero/result2 and both measured cmd fields are source_frame_target plus three spaces. Original complete hex rows supply the exact public strings without a private engine/frame or arbitrary script claim. Whole process exits zero with empty stderr.

### jim

Status: `observed`. Channel: Fixed original ASCII LF source-file CLI; each specimen original script is also output as hex. Source file tabs are literal ASCII tabs. The continuation specimen is a literal backslash-newline inside a braced value; the language performs its specified brace continuation substitution. No Unicode/raw-NUL/list-object/private command table or compiler admission assertion is inferred. Version: jim 0.84-9-g5bac7c9.

Actual Jim 0.84-9-g5bac7c9 records seven caught marker errors and a caught proc_marker procedure error. Each explicit ::errorInfo read is caught with code one and no-such-variable text. info frame is actually available with code zero/result1; both cmd fields are exactly source_frame_target without trailing spaces. The reported continuation script VALUE retains literal backslash/newline plus three spaces. Complete original hex rows remain authoritative; the request anticipation does not replace this actual availability. Whole process exits zero with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact command-source reporting question; stock C/Jim caller projections supply no hosted result.

## Consumer bindings

- [rust/tcl-lexer/src/script.rs](../../../../rust/tcl-lexer/src/script.rs), `CommandSpan::source_extent`: Own original terminator-delimited physical command reporting extent separately from structural word spans.
- [rust/tcl-lexer/src/native_script_words.rs](../../../../rust/tcl-lexer/src/native_script_words.rs), `NativeScriptCommandWords`: Retain optional authentic whole-image command source extent; vector-only replay grants no containing-source extent.
- `rust/tcl-compiler/src/codegen/statements.rs`, `CodegenCtx::native_command_source_span`: Use known physical extent only under actual C source-log recipe; keep Jim and represented vector spans independent.
- [rust/tcl-lexer/src/script.rs](../../../../rust/tcl-lexer/src/script.rs), `script::tests::original_command_source_extent_keeps_terminator_trivia_separate` (linked): Physical lexer extent keeps closers/trailing trivia, omits terminators/leading indent and refuses invalid token/source boundaries independently of native execution.
- [rust/tcl-lexer/src/native_script_words.rs](../../../../rust/tcl-lexer/src/native_script_words.rs), `native_script_words::tests::original_command_extent_rebases_inside_its_genuine_region` (linked): Genuine bounded original region retains rebased physical source extent separately from word envelope and whole-image NativeWord ownership.
- `rust/tcl-compiler/src/codegen/statements.rs`, `codegen::statements::tests::original_c_source_reporting_retains_parser_extent_without_promoting_vectors_or_jim` (linked): C recipe emitter uses parser-issued physical reporting extent while vector-only/Jim/unknown recipe retains represented span; opaque byte coverage is implementation-only, not an additional native observed input.

- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::log_command_info`: Pass the actual parsed command slice and source line to the shared reached error-log owner; an already logged nested command suppresses enclosing duplicate logging.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::log_command_bytes`: Retain reached command bytes, selected error-log policy and frame-relative line through shared errorInfo/errorStack publication.
- [runtime/rust/src/cmd_error.rs](../../../../runtime/rust/src/cmd_error.rs), `cmd_error::tests::error_info_stack_traces` (linked): Four default Interp public errorInfo strings assert trailing space in a simple proc command, body-relative multiline line3, suppression of the enclosing nested-substitution command, and the apply lambda frame. These are software expectations, not new six-provider excerpt or source-offset observations.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_command_source_extent286](../../../../rust/tcl-registry/tests/data/native_command_source_extent286)
- [rust/tcl-registry/tests/data/native_command_source_extent286/8.4.20](../../../../rust/tcl-registry/tests/data/native_command_source_extent286/8.4.20)
- [rust/tcl-registry/tests/data/native_command_source_extent286/8.5.19](../../../../rust/tcl-registry/tests/data/native_command_source_extent286/8.5.19)
- [rust/tcl-registry/tests/data/native_command_source_extent286/8.6.18](../../../../rust/tcl-registry/tests/data/native_command_source_extent286/8.6.18)
- [rust/tcl-registry/tests/data/native_command_source_extent286/9.0.4](../../../../rust/tcl-registry/tests/data/native_command_source_extent286/9.0.4)
- [rust/tcl-registry/tests/data/native_command_source_extent286/9.1.0](../../../../rust/tcl-registry/tests/data/native_command_source_extent286/9.1.0)
- [rust/tcl-registry/tests/data/native_command_source_extent286/jim](../../../../rust/tcl-registry/tests/data/native_command_source_extent286/jim)
