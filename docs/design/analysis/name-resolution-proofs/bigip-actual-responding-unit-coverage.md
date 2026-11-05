# naming.bigip.actual-responding-unit-coverage

Kind: `native-observation`

## Problem statement

A requested CMP setting or expected roster is not evidence that every TMM actually responded. Traffic runners can report a subset, an empty unit field, a transport failure or a target-only mutation request. Treating each as complete four-TMM execution would overstate the applicability of a naming result.

## Question

Which actual response-unit counts, expected-roster comparisons and runner outcomes are retained for the recorded traffic invocations?

## Conclusion

The retained runner/coverage artifacts separately record actual response-unit counts, expected rosters, missing units, return codes and streams. Counts are scoped to each invocation; a target-only write, unit-only label, empty identity field or failed transport cannot supply unobserved group/unit coverage. Semantic result bytes remain in separate response/log evidence.

## Scope

Retained canonical and expanded procedure/Unicode traffic invocations on BIG-IP21.1.0.1 build0.0.26; no native name-table or cross-blade conclusion.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl/Jim execution of this appliance traffic or chunk-transport question is claimed.

### bigip

Status: `observed`. Channel: Retained canonical and expanded procedure/Unicode traffic invocations on BIG-IP21.1.0.1 build0.0.26; no native name-table or cross-blade conclusion. Version: bigip 21.1.0.1.

The retained runner/coverage artifacts separately record actual response-unit counts, expected rosters, missing units, return codes and streams. Counts are scoped to each invocation; a target-only write, unit-only label, empty identity field or failed transport cannot supply unobserved group/unit coverage. Semantic result bytes remain in separate response/log evidence.

## Consumer bindings

No implementation binding is claimed by this observation record.
