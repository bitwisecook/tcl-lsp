# Tcl variable inlining execution

## Question

How do original variable parsing, reaching definitions, callee upvar mutation and read observers affect literal single-use variable inlining in the LSP? The recorded comparisons ask:

> Removing a store based on a printed Unicode name can change a source that the C evaluator parses as a shorter variable reference.

> How does bare $café differ from braced ${café} in the retained pure source controls?

This file records one side of the comparison; its result alone does not establish the result of the other script.

## Assumptions

> Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

The captures use fresh native processes. Handwritten replacement scripts are controls, not output from the Rust refactor. BIG-IP was not tested. Other assumptions were not separately recorded.

## Script

```tcl-visible
set café 7
puts $café
```

## Results

### c84 8.4.20, script file

- Recorded version: 8.4.20
- Recorded input channel: script file
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
    while executing
"puts $café"
    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c84-pure_unicode_bare_original.tcl" line 2)
```


### c85 8.5.19, script file

- Recorded version: 8.5.19
- Recorded input channel: script file
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
    while executing
"puts $café"
    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c85-pure_unicode_bare_original.tcl" line 2)
```


### c86 8.6.18, script file

- Recorded version: 8.6.18
- Recorded input channel: script file
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
    while executing
"puts $café"
    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c86-pure_unicode_bare_original.tcl" line 2)
```


### c90 9.0.4, script file

- Recorded version: 9.0.4
- Recorded input channel: script file
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
    while executing
"puts $café"
    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c90-pure_unicode_bare_original.tcl" line 2)
```


### c91 9.1.0, script file

- Recorded version: 9.1.0
- Recorded input channel: script file
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
    while executing
"puts $café"
    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c91-pure_unicode_bare_original.tcl" line 2)
```


### jim 0.84-9-g5bac7c9, script file

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
7
```

- Standard error: empty.

### c84 8.4.20, standard input

- Recorded version: 8.4.20
- Recorded input channel: standard input
- Exit status: 0
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
```


### c85 8.5.19, standard input

- Recorded version: 8.5.19
- Recorded input channel: standard input
- Exit status: 0
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
```


### c86 8.6.18, standard input

- Recorded version: 8.6.18
- Recorded input channel: standard input
- Exit status: 0
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
```


### c90 9.0.4, standard input

- Recorded version: 9.0.4
- Recorded input channel: standard input
- Exit status: 0
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
```


### c91 9.1.0, standard input

- Recorded version: 9.1.0
- Recorded input channel: standard input
- Exit status: 0
- Standard output: empty.
- Standard error:

```text-visible
can't read "caf": no such variable
```


### jim 0.84-9-g5bac7c9, standard input

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: standard input
- Exit status: 0
- Standard output:

```text-visible
7
```

- Standard error: empty.
