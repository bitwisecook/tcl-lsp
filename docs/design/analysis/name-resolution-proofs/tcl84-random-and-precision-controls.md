# naming.tcl84-random-and-precision-controls

Kind: `native-observation`

## Problem statement

Random stream and precision belong to an interpreter and to the time a Double first receives a string. Sharing another interpreter stream or recomputing a retained string under later precision can change the value.

## Question

How do seeded random calls and lazy precision stringification behave in the exact Tcl8.4 source?

## Conclusion

The retained format-random source/output supports the measured Tcl8.4 seeded sequence and precision timing only. Authored simulation defaults remain explicit model choices; no BIG-IP or host interpreter stream is inferred.

## Scope

Single C8.4 provider; exact format-random.tcl and format-random84.txt, ordinary guest operations and captured executable identity.

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

- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random-manifest.json](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random-manifest.json)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl)
