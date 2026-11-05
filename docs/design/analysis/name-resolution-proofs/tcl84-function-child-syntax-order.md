# naming.tcl84-function-child-syntax-order

Kind: `native-observation`

## Problem statement

An invalid nested command in a function argument may be rejected before the function or argument is entered. Inventing child effects from recovered syntax would alter the function evaluation order.

## Question

How does Tcl8.4 reject the original nested child syntax around fixed math function validation?

## Conclusion

The exact child-syntax.tcl / child-syntax84.txt capture retains Tcl8.4 delimiter/function outcomes and operand marker state. It does not authenticate private child compiler entry or another release.

## Scope

Single explicitly pinned C8.4 provider; original ASCII child-syntax source and complete captured diagnostic/value output.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original ASCII script passed to the recorded native shell. Version: tcl8.4 8.4 (retained corpus.

The complete original source/output is retained with process exit 0. Its exact function/diagnostic/result rows answer the scoped question; no other release is inferred.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax-manifest.json](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax-manifest.json)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl)
