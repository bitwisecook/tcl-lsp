# naming.corpus-pattern.list-prefix

Kind: `native-observation`

## Problem statement

A reviewed maintained-corpus naming idiom is reduced to the exact list-prefix program. The consumer-relevant distinction is the actual reached lookup result versus setup/capability failure and the independent static source, binding, frame and lifecycle premises. Pinned corpus source locations and counterexamples are retained in experiments/name-handling/patterns/sites.json and corpus-findings.md.

## Question

What lookup completion and result does the exact list-prefix reduction produce after successful setup on each recorded provider?

## Conclusion

8.4.20: Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. 8.5.19: Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. 8.6.18: Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. 9.0.4: Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. 9.1.0: Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. These are native observations of finite authored inputs, not full application execution, corpus prevalence, static recogniser coverage or Rust conformance. Jim 0.84-9-g5bac7c9: Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}". No guest errorCode is reported in the captured return options. Full setup/question return options and errorInfo remain in the exact raw capture. This observes only the reached original question, not a C-equivalence or Rust consumer result. F5 remains not tested for this exact question.

## Scope

One retained authored list-prefix reduction across real C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0; fresh process, real matching init, ASCII-only output capture. An error reached during lookup is a native observation. A failed setup is retained as a limitation and supplies no intended-question answer. No actual Tk/Snit/F5 host or extension semantics are inferred from a reduction. The same authored setup/final question was also reached in fresh upstream Jim 0.84-9-g5bac7c9 with its real tests/testing.tcl sourced before setup; full original Jim wrapper and separate raw fields are retained. Provider support/harness state is specific to this capture. No unchanged C tcltest execution, Jim-wide feature coverage or Rust conformance follows.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII authored probe encoded into retained catch wrapper, fresh matching real C interpreter; setup and lookup captured separately. Tcltest is loaded only by the Tcltest lifecycle reduction. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result.

### jim

Status: `observed`. Channel: Fresh upstream Jim process, LC_ALL=C, real tests/testing.tcl sourced before the unchanged authored setup and final-question catch wrappers. The original runner removes TCL_LIBRARY and TCLLIBPATH; other inherited environment is not retained. No C init.tcl or C expected value is imposed. Version: jim 0.84-9-g5bac7c9.

Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}". No guest errorCode is reported in the captured return options. Full setup/question return options and errorInfo remain in the exact raw capture. This observes only the reached original question, not a C-equivalence or Rust consumer result.

### bigip

Status: `not-tested`.

This exact authored corpus reduction was not executed on this provider. C results establish no Jim or appliance answer.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [experiments/name-handling/patterns/results](../../../../experiments/name-handling/patterns/results)
- [experiments/name-handling/patterns/probes](../../../../experiments/name-handling/patterns/probes)
- [rust/tcl-registry/tests/data/native_corpus_naming_patterns](../../../../rust/tcl-registry/tests/data/native_corpus_naming_patterns)
- [experiments/name-handling/patterns/jim-results-5bac7c9/corpus](../../../../experiments/name-handling/patterns/jim-results-5bac7c9/corpus)
- [experiments/name-handling/patterns](../../../../experiments/name-handling/patterns)
