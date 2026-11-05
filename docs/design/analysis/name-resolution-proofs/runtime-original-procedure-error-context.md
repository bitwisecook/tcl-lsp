# naming.runtime.original-procedure-error-context

Kind: `native-observation`

## Problem statement

Ordinary literal and dynamically named source errors, missing-command errors and instruction-labelled public error stacks are distinct producers. Source trace similarity cannot establish identical Native instruction or ABI execution.

## Question

What exact result/errorInfo/errorCode/errorstack fields do the original literal-error, dynamic-error and ordinary missing-command source controls report on each pinned provider?

## Conclusion

Every provider catches literal/dynamic errors with code1/result bad Q; complete public field bytes remain authoritative. C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 retain original source command text and procedure line3 in errorInfo. Jim does not emit errorInfo fields in this source. C8.6.18/C9.0.4/C9.1.0 literal-error stacks use returnImm; dynamically named error uses invokeStk1 on C8.6.18/C9.0.4 and invokeStk on C9.1.0. C8.4.20/C8.5.19/Jim info errorstack is unavailable with its original caught diagnostic. Ordinary definitely_missing_command has errorCode NONE on C8.4.20/C8.5.19/Jim and TCL LOOKUP COMMAND definitely_missing_command on C8.6.18/C9.0.4/C9.1.0. This ordinary source producer remains distinct from namespace-origin or manual ABI errors. All six external processes and six compiler builds exit 0; stderr is empty. Public instruction labels do not establish private bytecode identity, compiled-body admission, object/header identity, Native frame lookup or equivalence to a manual ABI source log. No Rust assertion/BIG-IP result follows.

## Scope

The exact authored ASCII LF source includes original catches/emit procedure and runs unchanged under fresh full provider initialisation via the native202 counted-source driver. The launcher adds no surrounding source envelope. Labelled guest hex fields and VERSION/ORIGINAL driver rows are parsed by tag, retaining original Jim stdout ordering.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Unchanged authored original counted ASCII LF source; full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: tcl8.4 8.4.20.

Exact labelled public fields: {"source_boom.code": "1", "source_boom.result": "bad Q", "source_boom.errorInfo": "bad Q\n    while executing\n\"error \"bad $a\"\"\n    (procedure \"source_boom\" line 3)\n    invoked from within\n\"$name Q\"", "source_boom.errorCode": "NONE", "source_boom.stackCode": "1", "source_boom.stack": "bad option \"errorstack\": must be args, body, cmdcount, commands, complete, default, exists, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars", "dynamic_boom.code": "1", "dynamic_boom.result": "bad Q", "dynamic_boom.errorInfo": "bad Q\n    while executing\n\"$command \"bad $a\"\"\n    (procedure \"dynamic_boom\" line 3)\n    invoked from within\n\"$name Q\"", "dynamic_boom.errorCode": "NONE", "dynamic_boom.stackCode": "1", "dynamic_boom.stack": "bad option \"errorstack\": must be args, body, cmdcount, commands, complete, default, exists, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars", "missing.code": "1", "missing.result": "invalid command name \"definitely_missing_command\"", "missing.errorCode": "NONE"}. Driver original completion code0/empty; process exit 0/empty stderr. Tag-based interpretation preserves every original stdout row and ordering, without private bytecode/frame/ABI identity inference.

### tcl8.5

Status: `observed`. Channel: Unchanged authored original counted ASCII LF source; full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: tcl8.5 8.5.19.

Exact labelled public fields: {"source_boom.code": "1", "source_boom.result": "bad Q", "source_boom.errorInfo": "bad Q\n    while executing\n\"error \"bad $a\"\"\n    (procedure \"source_boom\" line 3)\n    invoked from within\n\"$name Q\"", "source_boom.errorCode": "NONE", "source_boom.stackCode": "1", "source_boom.stack": "unknown or ambiguous subcommand \"errorstack\": must be args, body, cmdcount, commands, complete, default, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars", "dynamic_boom.code": "1", "dynamic_boom.result": "bad Q", "dynamic_boom.errorInfo": "bad Q\n    while executing\n\"$command \"bad $a\"\"\n    (procedure \"dynamic_boom\" line 3)\n    invoked from within\n\"$name Q\"", "dynamic_boom.errorCode": "NONE", "dynamic_boom.stackCode": "1", "dynamic_boom.stack": "unknown or ambiguous subcommand \"errorstack\": must be args, body, cmdcount, commands, complete, default, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars", "missing.code": "1", "missing.result": "invalid command name \"definitely_missing_command\"", "missing.errorCode": "NONE"}. Driver original completion code0/empty; process exit 0/empty stderr. Tag-based interpretation preserves every original stdout row and ordering, without private bytecode/frame/ABI identity inference.

### tcl8.6, tcl9.0

Status: `observed`. Channel: Unchanged authored original counted ASCII LF source; full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: tcl8.6 8.6.18; tcl9.0 9.0.4.

Exact labelled public fields: {"source_boom.code": "1", "source_boom.result": "bad Q", "source_boom.errorInfo": "bad Q\n    while executing\n\"error \"bad $a\"\"\n    (procedure \"source_boom\" line 3)\n    invoked from within\n\"$name Q\"", "source_boom.errorCode": "NONE", "source_boom.stackCode": "0", "source_boom.stack": "INNER {returnImm {bad Q} {}} CALL {source_boom Q}", "dynamic_boom.code": "1", "dynamic_boom.result": "bad Q", "dynamic_boom.errorInfo": "bad Q\n    while executing\n\"$command \"bad $a\"\"\n    (procedure \"dynamic_boom\" line 3)\n    invoked from within\n\"$name Q\"", "dynamic_boom.errorCode": "NONE", "dynamic_boom.stackCode": "0", "dynamic_boom.stack": "INNER {invokeStk1 error {bad Q}} CALL {dynamic_boom Q}", "missing.code": "1", "missing.result": "invalid command name \"definitely_missing_command\"", "missing.errorCode": "TCL LOOKUP COMMAND definitely_missing_command"}. Driver original completion code0/empty; process exit 0/empty stderr. Tag-based interpretation preserves every original stdout row and ordering, without private bytecode/frame/ABI identity inference.

### tcl9.1

Status: `observed`. Channel: Unchanged authored original counted ASCII LF source; full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: tcl9.1 9.1.0.

Exact labelled public fields: {"source_boom.code": "1", "source_boom.result": "bad Q", "source_boom.errorInfo": "bad Q\n    while executing\n\"error \"bad $a\"\"\n    (procedure \"source_boom\" line 3)\n    invoked from within\n\"$name Q\"", "source_boom.errorCode": "NONE", "source_boom.stackCode": "0", "source_boom.stack": "INNER {returnImm {bad Q} {}} CALL {source_boom Q}", "dynamic_boom.code": "1", "dynamic_boom.result": "bad Q", "dynamic_boom.errorInfo": "bad Q\n    while executing\n\"$command \"bad $a\"\"\n    (procedure \"dynamic_boom\" line 3)\n    invoked from within\n\"$name Q\"", "dynamic_boom.errorCode": "NONE", "dynamic_boom.stackCode": "0", "dynamic_boom.stack": "INNER {invokeStk error {bad Q}} CALL {dynamic_boom Q}", "missing.code": "1", "missing.result": "invalid command name \"definitely_missing_command\"", "missing.errorCode": "TCL LOOKUP COMMAND definitely_missing_command"}. Driver original completion code0/empty; process exit 0/empty stderr. Tag-based interpretation preserves every original stdout row and ordering, without private bytecode/frame/ABI identity inference.

### jim

Status: `observed`. Channel: Unchanged authored original counted ASCII LF source; full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: jim 0.84-9-g5bac7c9.

Exact labelled public fields: {"source_boom.code": "1", "source_boom.result": "bad Q", "source_boom.errorCode": "NONE", "source_boom.stackCode": "1", "source_boom.stack": "info, unknown command \"errorstack\": should be alias, aliases, args, body, channels, commands, complete, exists, frame, globals, help, hostname, level, locals, nameofexecutable, patchlevel, procs, references, returncodes, script, source, stacktrace, statics, tainted, usage, vars, version", "dynamic_boom.code": "1", "dynamic_boom.result": "bad Q", "dynamic_boom.errorCode": "NONE", "dynamic_boom.stackCode": "1", "dynamic_boom.stack": "info, unknown command \"errorstack\": should be alias, aliases, args, body, channels, commands, complete, exists, frame, globals, help, hostname, level, locals, nameofexecutable, patchlevel, procs, references, returncodes, script, source, stacktrace, statics, tainted, usage, vars, version", "missing.code": "1", "missing.result": "invalid command name \"definitely_missing_command\"", "missing.errorCode": "NONE"}. Driver original completion code0/empty; process exit 0/empty stderr. Tag-based interpretation preserves every original stdout row and ordering, without private bytecode/frame/ABI identity inference.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/codegen_abi.rs](../../../../runtime/rust/src/codegen_abi.rs), `codegen_abi::tests::invoke_argv_reports_unknown_error_with_options` (linked): Actual default C9.0 ABI missing-command control preserves ordinary-miss guest result/options/errorCode separately from namespace-origin errors. Native source295 observes the ordinary public producer only, not the manual ABI argument objects or physical identity.

## Evidence

- [rust/tcl-registry/tests/data/native_source_error_context295](../../../../rust/tcl-registry/tests/data/native_source_error_context295)
- [rust/tcl-registry/tests/data/native_source_error_context295/8.4.20/source-error-context](../../../../rust/tcl-registry/tests/data/native_source_error_context295/8.4.20/source-error-context)
- [rust/tcl-registry/tests/data/native_source_error_context295/8.5.19/source-error-context](../../../../rust/tcl-registry/tests/data/native_source_error_context295/8.5.19/source-error-context)
- [rust/tcl-registry/tests/data/native_source_error_context295/8.6.18/source-error-context](../../../../rust/tcl-registry/tests/data/native_source_error_context295/8.6.18/source-error-context)
- [rust/tcl-registry/tests/data/native_source_error_context295/9.0.4/source-error-context](../../../../rust/tcl-registry/tests/data/native_source_error_context295/9.0.4/source-error-context)
- [rust/tcl-registry/tests/data/native_source_error_context295/9.1.0/source-error-context](../../../../rust/tcl-registry/tests/data/native_source_error_context295/9.1.0/source-error-context)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_source_error_context295/jim/source-error-context](../../../../rust/tcl-registry/tests/data/native_source_error_context295/jim/source-error-context)
