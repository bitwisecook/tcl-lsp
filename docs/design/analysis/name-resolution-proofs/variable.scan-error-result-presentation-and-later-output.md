# naming.variable.scan-error-result-presentation-and-later-output

Kind: `native-observation`

## Problem statement

A scan destination can fail because its namespace is absent or its write callback errors. The command result object, its string/character getters and later output assignment can differ by release and stage; a canonical error label cannot stand in for the actual object or control path.

## Question

Scan failure result presentation and later output assignment What exact stages and results occur in the retained native probe?

## Conclusion

The retained fifteen C case transcripts distinguish one missing destination, two missing destinations and a failing legacy write trace, with exact completion, result-object stages, result bytes and later output state. Tcl 9 legacy trace syntax rejects before the scan; that row is not a positive Tcl 9 write-callback result. The separate modern C9 control answers that path.

## Scope

Three fixed scripts/provider, C84-91 public evaluation plus private result-header/character cache reporting; no Jim/BIG-IP. Header fields and semantic results remain independent observations.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl8.4 8.4.20.

Exact captured transcript:

"VERSION|8.4.20\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a666972737422\nAFTER-STRING|string|1|1|-1\nCHARS|39\nAFTER-LENGTH|string|1|1|39\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a666972737422636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a7365636f6e6422\nAFTER-STRING|string|1|1|-1\nCHARS|79\nAFTER-LENGTH|string|1|1|79\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c652022666972737422\nAFTER-STRING|string|1|1|-1\nCHARS|29\nAFTER-LENGTH|string|1|1|29\nOUTPUT|2\n"

### tcl8.5

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl8.5 8.5.19.

Exact captured transcript:

"VERSION|8.5.19\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a666972737422\nAFTER-STRING|string|1|1|-1\nCHARS|39\nAFTER-LENGTH|string|1|1|39\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a666972737422636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a7365636f6e6422\nAFTER-STRING|string|1|1|-1\nCHARS|79\nAFTER-LENGTH|string|1|1|79\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c652022666972737422\nAFTER-STRING|string|1|1|-1\nCHARS|29\nAFTER-LENGTH|string|1|1|29\nOUTPUT|2\n"

### tcl8.6

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl8.6 8.6.18.

Exact captured transcript:

"VERSION|8.6.18\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420226669727374223a20424f4f4d\nAFTER-STRING|string|1|1|-1\nCHARS|23\nAFTER-LENGTH|string|1|1|23\nOUTPUT|2\n"

### tcl9.0

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl9.0 9.0.4.

Exact captured transcript:

"VERSION|9.0.4\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665\nAFTER-STRING|string|1|1|-1\nCHARS|51\nAFTER-LENGTH|string|1|1|51\nOUTPUT|2\n"

### tcl9.1

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl9.1 9.1.0.

Exact captured transcript:

"VERSION|9.1.0\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665\nAFTER-STRING|string|1|1|-1\nCHARS|51\nAFTER-LENGTH|string|1|1|51\nOUTPUT|2\n"

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/manifest.json)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/probe.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/probe.c)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.4.20-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.4.20-run.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.5.19-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.5.19-run.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.6.18-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.6.18-run.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/9.0.4-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/9.0.4-run.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/9.1.0-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/9.1.0-run.stdout)
