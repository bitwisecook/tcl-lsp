# naming.variable.scan-modern-c9-write-callback-result-presentation

Kind: `native-observation`

## Problem statement

Tcl 9 rejects legacy trace syntax, so the earlier scan-presenter row cannot answer whether a reached scan write callback preserves a later destination/result. A separate modern trace script is necessary for that concrete path.

## Question

Modern Tcl 9 scan write-callback failure and result getters What exact stages and results occur in the retained native probe?

## Conclusion

The two independent C9 modern-trace captures retain a reached failing write callback, scan completion/result getter stages and later output. They answer only that modern syntax/script and cannot repair or replace the legacy rejection rows, establish arbitrary object hooks, or certify compiler completion.

## Scope

Tcl9.0.4 and9.1.0 only; modern trace add variable write, public counted source evaluation and private result header/character cache stages; C84-86/Jim/BIG-IP not tested by this probe.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `not-tested`.

No observation for this question.

### tcl9.0

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl9.0 9.0.4.

Exact captured transcript:

"VERSION|9.0.4\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420226669727374223a20424f4f4d\nAFTER-STRING|string|1|1|-1\nCHARS|23\nAFTER-LENGTH|string|1|1|23\nOUTPUT|2\n"

### tcl9.1

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl9.1 9.1.0.

Exact captured transcript:

"VERSION|9.1.0\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420226669727374223a20424f4f4d\nAFTER-STRING|string|1|1|-1\nCHARS|23\nAFTER-LENGTH|string|1|1|23\nOUTPUT|2\n"

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/manifest.json)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/probe.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/probe.c)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/9.0.4-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/9.0.4-run.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/9.1.0-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/9.1.0-run.stdout)
