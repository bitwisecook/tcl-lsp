# naming.wiki.oo-3

Kind: `native-observation`

## Problem statement

The Tcl Wiki idiom and its adversarial reduction exercise forward target observes command replacement. A static consumer must distinguish the measured runtime result from the independent proof of source, namespace/frame, binding and lifecycle needed for navigation, diagnostics, edits and optimisation. The exact original input and per-case Tcltest receipts are retained.

## Question

Does this retained program establish that forward target observes command replacement?

## Conclusion

8.4.20: ++++ wiki-oo-3 SKIPPED: tcloo. This case did not execute and supplies no oracle answer. 8.5.19: ++++ wiki-oo-3 SKIPPED: tcloo. This case did not execute and supplies no oracle answer. 8.6.18: wiki-oo-3 PASSED: actual result matched retained -result {replacement fixed}. 9.0.4: wiki-oo-3 PASSED: actual result matched retained -result {replacement fixed}. 9.1.0: wiki-oo-3 PASSED: actual result matched retained -result {replacement fixed}. These are finite native script observations. They do not issue compiler source, current binding, object/cell, frame, edit or optimisation authority; Jim, BIG-IP and Rust implementation execution are separate obligations.

## Scope

Only retained authored test cases wiki-oo-3 in stock C Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 on 2026-10-12. Each test has a fresh child with normal startup; real matching upstream Tcltest drives assertions. Wiki pages provide pattern provenance, not provider authority. A skipped capability/version constraint supplies no native answer. Historical supplied VM/runtime binaries are outside these native-provider answers.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.2.11 test harness; binary/startup hashes retained. Version: tcl8.4 8.4.20.

++++ wiki-oo-3 SKIPPED: tcloo. This case did not execute and supplies no oracle answer.

### tcl8.5

Status: `not-tested`. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.3.8 test harness; binary/startup hashes retained. Version: tcl8.5 8.5.19.

++++ wiki-oo-3 SKIPPED: tcloo. This case did not execute and supplies no oracle answer.

### tcl8.6, tcl9.0

Status: `observed`. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.5.11 test harness; binary/startup hashes retained. Version: tcl8.6 8.6.18; tcl9.0 9.0.4.

wiki-oo-3 PASSED: actual result matched retained -result {replacement fixed}.

### tcl9.1

Status: `observed`. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.6.0 test harness; binary/startup hashes retained. Version: tcl9.1 9.1.0.

wiki-oo-3 PASSED: actual result matched retained -result {replacement fixed}.

### jim, bigip

Status: `not-tested`.

This authored Wiki reduction was not executed on this provider. No equivalent result is inferred from C.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [experiments/name-handling/patterns/wiki-probes.tcl](../../../../experiments/name-handling/patterns/wiki-probes.tcl)
- [rust/tcl-registry/tests/data/native_wiki_naming_patterns/capture-runner-original.txt](../../../../rust/tcl-registry/tests/data/native_wiki_naming_patterns/capture-runner-original.txt)
- [rust/tcl-registry/tests/data/native_wiki_naming_patterns/observations.json](../../../../rust/tcl-registry/tests/data/native_wiki_naming_patterns/observations.json)
- [experiments/name-handling/patterns/wiki-results](../../../../experiments/name-handling/patterns/wiki-results)
