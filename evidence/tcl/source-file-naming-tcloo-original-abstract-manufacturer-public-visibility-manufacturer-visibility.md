# Tcl source-file execution evidence

## Question

> An abstract metaclass can hide public manufacturer methods without replacing the inherited constructor machinery. Treating failed public Abstract new/create calls as a universal constructor-handler prohibition also mispredicts a concrete subclass or an explicit object-level export.

> For the exact original script, which manufacturer methods are publicly visible on an oo::abstract instance, do new/create/createWithNamespace public calls fail, can a concrete subclass create an instance, and does explicit object export new restore public Abstract new?

This is one source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

> These are independent source-file runs. Tcl 9 runs reach the visibility operations; older Tcl and Jim runs can instead report that the producer is unavailable. Private method, frame and object identities, BIG-IP/TMM behavior and Rust implementation equivalence were not measured. The full inherited environment and build replay were not recorded.

Other assumptions were not separately recorded.

## Script

```tcl-visible
oo::abstract create Abstract {}
puts [list abstract_public [lsort [info object methods Abstract -all]]]
puts [list abstract_private [lsort [info object methods Abstract -all -private]]]
foreach method {new create createWithNamespace} {
    if {$method eq "new"} {set command [list Abstract new]}
    if {$method eq "create"} {set command [list Abstract create x]}
    if {$method eq "createWithNamespace"} {set command [list Abstract createWithNamespace x ::x]}
    set code [catch $command result]
    puts [list attempt $method code $code result $result]
}
oo::class create Concrete {superclass Abstract}
set code [catch {Concrete create c} result]
puts [list concrete_create code $code class [info object class c]]
oo::objdefine Abstract export new
set code [catch {Abstract new} result]
if {$code == 0} {puts [list explicit_export_new code $code class [info object class $result]]} else {puts [list explicit_export_new code $code result $result]}
puts [list abstract_public_after [lsort [info object methods Abstract -all]]]
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
invalid command name "oo::abstract"
    while executing
"oo::abstract create Abstract {}"
    (file "/workspace/.proofs/abstract-manufacturer-native336/manufacturer-visibility.tcl" line 1)
```


### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
invalid command name "oo::abstract"
    while executing
"oo::abstract create Abstract {}"
    (file "/workspace/.proofs/abstract-manufacturer-native336/manufacturer-visibility.tcl" line 1)
```


### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
invalid command name "oo::abstract"
    while executing
"oo::abstract create Abstract {}"
    (file "/workspace/.proofs/abstract-manufacturer-native336/manufacturer-visibility.tcl" line 1)
```


### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
abstract_public destroy
abstract_private {<cloned> create createWithNamespace destroy eval new unknown variable varname}
attempt new code 1 result {unknown method "new": must be destroy}
attempt create code 1 result {unknown method "create": must be destroy}
attempt createWithNamespace code 1 result {unknown method "createWithNamespace": must be destroy}
concrete_create code 0 class ::Concrete
explicit_export_new code 0 class ::Abstract
abstract_public_after {destroy new}
```

- Standard error: empty.

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
abstract_public destroy
abstract_private {<cloned> create createWithNamespace destroy eval new unknown variable varname}
attempt new code 1 result {unknown method "new": must be destroy}
attempt create code 1 result {unknown method "create": must be destroy}
attempt createWithNamespace code 1 result {unknown method "createWithNamespace": must be destroy}
concrete_create code 0 class ::Concrete
explicit_export_new code 0 class ::Abstract
abstract_public_after {destroy new}
```

- Standard error: empty.

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
/workspace/.proofs/abstract-manufacturer-native336/manufacturer-visibility.tcl:1: Error: invalid command name "oo::abstract"
Traceback (most recent call last):
  File "/workspace/.proofs/abstract-manufacturer-native336/manufacturer-visibility.tcl", line 1
```
