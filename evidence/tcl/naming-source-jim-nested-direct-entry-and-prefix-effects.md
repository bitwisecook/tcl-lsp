# Tcl execution evidence

## Question

> Recursive validation of nested script syntax can accidentally suppress an outer command prefix that the selected provider really executes. Conversely, admitting a complete-looking prefix can execute commands when the provider rejects the whole script at entry. Command substitutions, quoted substitutions and variable-index substitutions require the actual selected script-entry purpose rather than a single universal parse cut.

> For these seven counted nested-script sources, which outer and child prefix effects occur under public Direct evaluation on C Tcl 8.4–9.1 and the measured Jim build, and how do they relate to the separate info complete query?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Seven exact counted ASCII source values reconstructed with binary format H* inside the unchanged public probe. Each uses info complete followed by uplevel #0 and independent reads of before, inner, sink and after. Six original subprocesses exit0 with empty stderr and yield 42 case rows. Physical source LF, binary source bytes, completeness results, script catch results and reached effects are independent facts. No bytecode, compiler binding, original Native object/header, lifetime, callback, frame, Host error metadata, F5/TMM or arbitrary nested grammar equivalence is measured.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
set candidates {
  terminal_child 736574206265666f726520313b207365742073696e6b205b6c697374207b617d625d3b207365742061667465722032
  terminal_child_prefix 736574206265666f726520313b207365742073696e6b205b73657420696e6e657220313b206c697374207b617d625d3b207365742061667465722032
  terminal_quoted_child 736574206265666f726520313b207365742073696e6b20225b6c697374207b617d625d223b207365742061667465722032
  terminal_index_child 736574206265666f726520313b207365742073696e6b2024617272285b6c697374207b617d625d293b207365742061667465722032
  missing_child_brace 736574206265666f726520313b207365742073696e6b205b6c697374207b615d3b207365742061667465722032
  missing_child_variable_brace 736574206265666f726520313b207365742073696e6b205b6c69737420247b615d3b207365742061667465722032
  bare_child_open_brace 736574206265666f726520313b207365742073696e6b205b6c69737420787b5d3b207365742061667465722032
}
foreach {case hex} $candidates {
    foreach name {before inner sink after arr} {catch {unset $name}}
    set arr(known) VALUE
    set script [binary format H* $hex]
    set completeCode [catch {info complete $script} complete]
    binary scan $complete H* completeHex
    set code [catch {uplevel #0 $script} message]
    binary scan $message H* messageHex
    set effects {}
    foreach name {before inner sink after} {
        if {[info exists $name]} {binary scan [set $name] H* value; lappend effects $name PRESENT $value} else {lappend effects $name MISSING}
    }
    puts [list ROW $case COMPLETE_CODE $completeCode COMPLETE_HEX $completeHex CODE $code RESULT_HEX $messageHex EFFECTS $effects]
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 8.4.20
ROW terminal_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_child_prefix COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_quoted_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_index_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_variable_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d65 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW bare_child_open_brace COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 0 RESULT_HEX 32 EFFECTS {before PRESENT 31 inner MISSING sink PRESENT 785c7b after PRESENT 32}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 8.5.19
ROW terminal_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_child_prefix COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_quoted_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_index_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_variable_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d65 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW bare_child_open_brace COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 0 RESULT_HEX 32 EFFECTS {before PRESENT 31 inner MISSING sink PRESENT 785c7b after PRESENT 32}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 8.6.18
ROW terminal_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_child_prefix COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_quoted_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_index_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_variable_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d65 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW bare_child_open_brace COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 0 RESULT_HEX 32 EFFECTS {before PRESENT 31 inner MISSING sink PRESENT 785c7b after PRESENT 32}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 9.0.4
ROW terminal_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_child_prefix COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_quoted_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_index_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_variable_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d65 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW bare_child_open_brace COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 0 RESULT_HEX 32 EFFECTS {before PRESENT 31 inner MISSING sink PRESENT 785c7b after PRESENT 32}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 9.1.0
ROW terminal_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_child_prefix COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_quoted_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_index_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_variable_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d65 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW bare_child_open_brace COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 0 RESULT_HEX 32 EFFECTS {before PRESENT 31 inner MISSING sink PRESENT 785c7b after PRESENT 32}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 0.84-9-g5bac7c9
ROW terminal_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_child_prefix COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_quoted_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW terminal_index_child COMPLETE_CODE 0 COMPLETE_HEX 31 CODE 1 RESULT_HEX 6578747261206368617261637465727320616674657220636c6f73652d6272616365 EFFECTS {before PRESENT 31 inner MISSING sink MISSING after MISSING}
ROW missing_child_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 756e6d61746368656420225b22 EFFECTS {before MISSING inner MISSING sink MISSING after MISSING}
ROW missing_child_variable_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 756e6d61746368656420225b22 EFFECTS {before MISSING inner MISSING sink MISSING after MISSING}
ROW bare_child_open_brace COMPLETE_CODE 0 COMPLETE_HEX 30 CODE 1 RESULT_HEX 756e6d61746368656420225b22 EFFECTS {before MISSING inner MISSING sink MISSING after MISSING}
```

- Standard error: empty.
