# Tcl source-file execution evidence

## Question

> A property setter or getter can change the property roster while configure remains active. Reusing an old selected name as though it were a current member, or rebuilding a configure-all read inventory after each getter, changes the selected property and returned snapshot. Public result equality must remain separate from the native table/header storage recipe.

> For the two exact original configurable-class scripts, which setter executes after the first setter inserts b, and which properties and events does configure-all return when its first getter inserts b and an unrelated method?

This is one source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

> These source-file observations measure script-visible completion, result, event and property-roster values. They do not observe private pointer identity, reference-count chronology or internal error-formatting state. Two separately recorded setup refusals are distinct from the configurable-class observations. BIG-IP and Rust implementation equivalence were not tested. The inherited full environment and build replay were not recorded.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set events {}
oo::configurable create C {
    property a -get {return A} -set {
        lappend ::events a
        oo::define C {property b -get {return B} -set {lappend ::events b}}
    }
    property z -get {return Z} -set {lappend ::events z}
}
C create o
set code [catch {o configure -a ONE -z TWO} result options]
puts [list code $code result $result events $events properties [info object properties o -all -writable]]
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
invalid command name "oo::configurable"
    while executing
"oo::configurable create C {
    property a -get {return A} -set {
        lappend ::events a
        oo::define C {property b -get {return B} -set {la..."
    (file "/workspace/.proofs/property-report-native335/multi-write-configurable.tcl" line 2)
```


### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
invalid command name "oo::configurable"
    while executing
"oo::configurable create C {
    property a -get {return A} -set {
        lappend ::events a
        oo::define C {property b -get {return B} -set {la..."
    (file "/workspace/.proofs/property-report-native335/multi-write-configurable.tcl" line 2)
```


### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
invalid command name "oo::configurable"
    while executing
"oo::configurable create C {
    property a -get {return A} -set {
        lappend ::events a
        oo::define C {property b -get {return B} -set {la..."
    (file "/workspace/.proofs/property-report-native335/multi-write-configurable.tcl" line 2)
```


### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
code 0 result {} events {a b} properties {-a -b -z}
```

- Standard error: empty.

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
code 0 result {} events {a b} properties {-a -b -z}
```

- Standard error: empty.

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
/workspace/.proofs/property-report-native335/multi-write-configurable.tcl:2: Error: invalid command name "oo::configurable"
Traceback (most recent call last):
  File "/workspace/.proofs/property-report-native335/multi-write-configurable.tcl", line 2
```
