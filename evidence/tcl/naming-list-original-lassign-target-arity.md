# Tcl execution evidence

## Question

> A target-free lassign invocation can be a valid guest operation in one release and an arity error in another. Ordinary handler grammar, procedure-script results and native compiler admission are separate purposes; Jim also reports a different usage synopsis.

> For the seven exact original ASCII lassign source forms, which captured providers accept a target-free list, what error precedes malformed-list inspection, and what exact usage or unavailable-command result does each provider return?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Seven finite forms in one fixed ASCII LF source-file CLI probe per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim process: no operands, literal/dynamic-head/procedure target-free, malformed target-free, one target and empty target-free. A successful procedure result does not establish a native compiler hook or target-free ListAssignment admission. No original object/header/cache identity, opaque/zero input, callback behavior, Tcllib shim or BIG-IP result is measured. The original receipts contain a copied input_channel sentence about trace registration; the exact probe, argument vectors and boundary fields identify these lassign-only source controls.

## Script

```tcl-visible
if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts "PATCHLEVEL $probeVersion"
foreach {label script} {
    no_operands {lassign}
    literal_no_target {lassign {A B}}
    dynamic_no_target {set probeHead lassign; $probeHead {A B}}
    compiled_no_target {proc probeCompiled {} {lassign {A B}}; probeCompiled}
    malformed_no_target {set probeBad \{; set probeHead lassign; $probeHead $probeBad}
    literal_one_target {lassign {A B} probeFirst}
    empty_no_target {lassign {}}
} {
    set probeCode [catch $script probeResult]
    puts [list RESULT $label $probeCode $probeResult]
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.4.20
RESULT no_operands 1 {invalid command name "lassign"}
RESULT literal_no_target 1 {invalid command name "lassign"}
RESULT dynamic_no_target 1 {invalid command name "lassign"}
RESULT compiled_no_target 1 {invalid command name "lassign"}
RESULT malformed_no_target 1 {invalid command name "lassign"}
RESULT literal_one_target 1 {invalid command name "lassign"}
RESULT empty_no_target 1 {invalid command name "lassign"}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.5.19
RESULT no_operands 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT literal_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT dynamic_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT compiled_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT malformed_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT literal_one_target 0 B
RESULT empty_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.6.18
RESULT no_operands 1 {wrong # args: should be "lassign list ?varName ...?"}
RESULT literal_no_target 0 {A B}
RESULT dynamic_no_target 0 {A B}
RESULT compiled_no_target 0 {A B}
RESULT malformed_no_target 1 {unmatched open brace in list}
RESULT literal_one_target 0 B
RESULT empty_no_target 0 {}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.0.4
RESULT no_operands 1 {wrong # args: should be "lassign list ?varName ...?"}
RESULT literal_no_target 0 {A B}
RESULT dynamic_no_target 0 {A B}
RESULT compiled_no_target 0 {A B}
RESULT malformed_no_target 1 {unmatched open brace in list}
RESULT literal_one_target 0 B
RESULT empty_no_target 0 {}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.1.0
RESULT no_operands 1 {wrong # args: should be "lassign list ?varName ...?"}
RESULT literal_no_target 0 {A B}
RESULT dynamic_no_target 0 {A B}
RESULT compiled_no_target 0 {A B}
RESULT malformed_no_target 1 {unmatched open brace in list}
RESULT literal_one_target 0 B
RESULT empty_no_target 0 {}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 0.84-9-g5bac7c9
RESULT no_operands 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT literal_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT dynamic_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT compiled_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT malformed_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT literal_one_target 0 B
RESULT empty_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
```

- Standard error: empty.
