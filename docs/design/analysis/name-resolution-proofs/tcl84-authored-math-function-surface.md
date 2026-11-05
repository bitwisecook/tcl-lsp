# naming.tcl84-authored-math-function-surface

Kind: `native-observation`

## Problem statement

An authored Tcl8.4 math simulation must distinguish fixed function validation from runtime operands and wide/numeric conversion. Modern math-command dispatch or a parser capability does not establish the old fixed-function surface.

## Question

What fixed function, arity, numeric-width and operand-effect results do the sixteen original Tcl8.4 math expressions produce?

## Conclusion

The retained Tcl8.4 source/output control defines the tested function and arity/numeric cases only. It supplies a conditional authored provider comparison, not an actual Native function table or BIG-IP appliance seed, state or object header.

## Scope

C8.4-only source.tcl and native84.txt with exact executable/header/implementation digests in the manifest.

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

- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/manifest.json](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/manifest.json)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt)
- [rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl)
