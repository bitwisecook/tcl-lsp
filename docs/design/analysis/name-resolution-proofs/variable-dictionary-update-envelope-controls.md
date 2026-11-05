# naming.variable.dictionary-update-envelope-controls

Kind: `native-observation`

## Problem statement

A dictionary update with a scalar first mapping and array-valued second destination has both an availability guard and an evaluation envelope. The exact original guarded eval form, a bare eval form and two catch-$script forms require independent public observations before attributing a result to either distinction.

## Question

Do the dict-availability guard and eval versus catch-$script envelope change the public array-second dict update tuple in the four exact recorded source-file controls?

## Conclusion

Twenty-four independent source-file processes, four per exact provider, exit zero with empty stderr. C8.5/C8.6/C9.0/C9.1 produce the same inner tuple in all four controls: 1 NEW 0 1 {can't read "blocked": variable is array}. The scalar first target advances to NEW, the update catches the array assignment failure, and the body remains unentered. Jim produces the same inner tuple in all four: 0 NEW 1 0 OTHER, with a normal update, entered body and scalar-readable blocked destination. The catch-$script forms print an additional outer zero and braced tuple; that envelope remains in the raw bytes. C8.4 has no dict command in this captured installation: guarded forms print UNAVAILABLE, while bare forms catch the missing command and retain BEFORE with an unentered body and array-read failure. No guarded dictionary branch executes there. These outcomes agree with the independently retained original C8.5+ column. The finite comparisons supply no observed C8.5+/Jim tuple difference attributable to guard or envelope, and no bytecode, input-object/cache/header or frame/storage mechanism is inferred. BIG-IP is not tested.

## Scope

Four fixed original ASCII LF CLI programs are evaluated in separate fresh processes per provider. The exact source files distinguish guarded versus bare body and eval versus catch of a script value. Raw whole stdout retains each outer/inner code and list rendering. The processes do not print their versions; each receipt explicitly retains its version-pin association to the independently captured source187 receipt, and the exact executable/environment/required artifact associations are checked against that receipt. This is an executable association rather than a new reported version observation. Source/library/header/build pins do not by themselves prove a compiled-source causal explanation. No raw/counting NUL, non-ASCII or continuation/control escape is exercised. Existing fixtures, original source187 observations, Runtime assertion outcomes and complete-suite claims remain independent. No native argv/object/header/cache identity, successful Compiler source-carrier admission, frame allocation, general storage equivalence or edit authority follows.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Four independent original ASCII LF CLI files: guarded/bare eval and guarded/bare catch-$script; one public output row per process. Version: tcl8.4 8.4.20.

This captured C8.4 installation lacks dict: guarded branches decline with UNAVAILABLE, while bare branches retain the caught missing-operation outcome and BEFORE. Exact rows: original-guarded-eval: UNAVAILABLE; bare-eval: 1 BEFORE 0 1 {can't read "blocked": variable is array}; guarded-variable-catch: 0 UNAVAILABLE; bare-variable-catch: 0 {1 BEFORE 0 1 {can't read "blocked": variable is array}}. The version 8.4.20 is independently associated through original source187 and the same executable SHA, not printed in these four processes. All four exits are zero and stderr empty. No evaluator-optimisation or native object/frame/storage cause is inferred.

### tcl8.5

Status: `observed`. Channel: Four independent original ASCII LF CLI files: guarded/bare eval and guarded/bare catch-$script; one public output row per process. Version: tcl8.5 8.5.19.

The exact four public controls retain the same inner tuple; catch-$script adds the printed outer-zero/list envelope. Exact rows: original-guarded-eval: 1 NEW 0 1 {can't read "blocked": variable is array}; bare-eval: 1 NEW 0 1 {can't read "blocked": variable is array}; guarded-variable-catch: 0 {1 NEW 0 1 {can't read "blocked": variable is array}}; bare-variable-catch: 0 {1 NEW 0 1 {can't read "blocked": variable is array}}. The version 8.5.19 is independently associated through original source187 and the same executable SHA, not printed in these four processes. All four exits are zero and stderr empty. No evaluator-optimisation or native object/frame/storage cause is inferred.

### tcl8.6

Status: `observed`. Channel: Four independent original ASCII LF CLI files: guarded/bare eval and guarded/bare catch-$script; one public output row per process. Version: tcl8.6 8.6.18.

The exact four public controls retain the same inner tuple; catch-$script adds the printed outer-zero/list envelope. Exact rows: original-guarded-eval: 1 NEW 0 1 {can't read "blocked": variable is array}; bare-eval: 1 NEW 0 1 {can't read "blocked": variable is array}; guarded-variable-catch: 0 {1 NEW 0 1 {can't read "blocked": variable is array}}; bare-variable-catch: 0 {1 NEW 0 1 {can't read "blocked": variable is array}}. The version 8.6.18 is independently associated through original source187 and the same executable SHA, not printed in these four processes. All four exits are zero and stderr empty. No evaluator-optimisation or native object/frame/storage cause is inferred.

### tcl9.0

Status: `observed`. Channel: Four independent original ASCII LF CLI files: guarded/bare eval and guarded/bare catch-$script; one public output row per process. Version: tcl9.0 9.0.4.

The exact four public controls retain the same inner tuple; catch-$script adds the printed outer-zero/list envelope. Exact rows: original-guarded-eval: 1 NEW 0 1 {can't read "blocked": variable is array}; bare-eval: 1 NEW 0 1 {can't read "blocked": variable is array}; guarded-variable-catch: 0 {1 NEW 0 1 {can't read "blocked": variable is array}}; bare-variable-catch: 0 {1 NEW 0 1 {can't read "blocked": variable is array}}. The version 9.0.4 is independently associated through original source187 and the same executable SHA, not printed in these four processes. All four exits are zero and stderr empty. No evaluator-optimisation or native object/frame/storage cause is inferred.

### tcl9.1

Status: `observed`. Channel: Four independent original ASCII LF CLI files: guarded/bare eval and guarded/bare catch-$script; one public output row per process. Version: tcl9.1 9.1.0.

The exact four public controls retain the same inner tuple; catch-$script adds the printed outer-zero/list envelope. Exact rows: original-guarded-eval: 1 NEW 0 1 {can't read "blocked": variable is array}; bare-eval: 1 NEW 0 1 {can't read "blocked": variable is array}; guarded-variable-catch: 0 {1 NEW 0 1 {can't read "blocked": variable is array}}; bare-variable-catch: 0 {1 NEW 0 1 {can't read "blocked": variable is array}}. The version 9.1.0 is independently associated through original source187 and the same executable SHA, not printed in these four processes. All four exits are zero and stderr empty. No evaluator-optimisation or native object/frame/storage cause is inferred.

### jim

Status: `observed`. Channel: Four independent original ASCII LF CLI files: guarded/bare eval and guarded/bare catch-$script; one public output row per process. Version: jim 0.84-9-g5bac7c9.

The exact four public controls retain the same inner tuple; catch-$script adds the printed outer-zero/list envelope. Exact rows: original-guarded-eval: 0 NEW 1 0 OTHER; bare-eval: 0 NEW 1 0 OTHER; guarded-variable-catch: 0 {0 NEW 1 0 OTHER}; bare-variable-catch: 0 {0 NEW 1 0 OTHER}. The version 0.84-9-g5bac7c9 is independently associated through original source187 and the same executable SHA, not printed in these four processes. All four exits are zero and stderr empty. No evaluator-optimisation or native object/frame/storage cause is inferred.

### bigip

Status: `not-tested`.

No appliance execution answers this exact dictionary-envelope question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_variable_container_envelope188/8.4.20](../../../../rust/tcl-registry/tests/data/native_variable_container_envelope188/8.4.20)
- [rust/tcl-registry/tests/data/native_variable_container_envelope188/8.5.19](../../../../rust/tcl-registry/tests/data/native_variable_container_envelope188/8.5.19)
- [rust/tcl-registry/tests/data/native_variable_container_envelope188/8.6.18](../../../../rust/tcl-registry/tests/data/native_variable_container_envelope188/8.6.18)
- [rust/tcl-registry/tests/data/native_variable_container_envelope188/9.0.4](../../../../rust/tcl-registry/tests/data/native_variable_container_envelope188/9.0.4)
- [rust/tcl-registry/tests/data/native_variable_container_envelope188/9.1.0](../../../../rust/tcl-registry/tests/data/native_variable_container_envelope188/9.1.0)
- [rust/tcl-registry/tests/data/native_variable_container_envelope188/jim](../../../../rust/tcl-registry/tests/data/native_variable_container_envelope188/jim)
