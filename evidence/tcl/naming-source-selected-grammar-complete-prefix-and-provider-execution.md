# Tcl execution evidence

## Question

> A selected source grammar can retain a complete command prefix while the same whole source reports incomplete or raises a terminal parse error. Treating info complete as successful parsing or as permission to enter every prefix conflates source analysis with the actual provider operation. These two original inputs expose release-specific braced-variable grammar and different reached prefix effects.

> Does the selected release consider the malformed braced-variable source incomplete, and which completed prefix/suffix effects occur when that same original source is evaluated? Does a terminal word-weld parse error remain complete while suppressing its suffix?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Two exact counted ASCII script values under the retained public info complete query, then public uplevel #0 evaluation and separate post-evaluation before/after variable reads. Six actual CLI processes and twelve ROWs are retained: five measured C releases and the measured Jim build. Every process exits0 with empty stderr; catch reports each script outcome independently. VERSION, extra VALUE output, counted source bytes, exact receipts, invoked ELF and captured Tcl library .tcl input pins remain distinct. Loaded shared-library identity, full SDK/build replay, internal parser strategy, compiled bytecode, original headers, lifetimes, callbacks, frames, Host refusal metadata, compiler-purpose admission, current Rust assertions and TMM are unmeasured. These two cases establish no general Jim command-prefix effect rule.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
set candidates {
  selected_braced_var 736574206265666f726520313b207075747320247b617b627d3b207365742061667465722032
  terminal_word 736574206265666f726520313b2073657420696e76616c6964207b617d623b207365742061667465722032
}
foreach {case hex} $candidates {
    catch {unset before}
    catch {unset after}
    catch {unset invalid}
    set [format %c%c%c 97 123 98] VALUE
    set script [binary format H* $hex]
    set completeCode [catch {info complete $script} complete]
    binary scan $complete H* completeHex
    set code [catch {uplevel #0 $script} message]
    binary scan $message H* messageHex
    set observedBefore MISSING
    set observedAfter MISSING
    if {[info exists before]} {set observedBefore $before}
    if {[info exists after]} {set observedAfter $after}
    puts [list ROW $case $completeCode $completeHex $code $messageHex $observedBefore $observedAfter]
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 8.4.20
VALUE
ROW selected_braced_var 0 31 0 32 1 2
ROW terminal_word 0 31 1 6578747261206368617261637465727320616674657220636c6f73652d6272616365 1 MISSING
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 8.5.19
VALUE
ROW selected_braced_var 0 31 0 32 1 2
ROW terminal_word 0 31 1 6578747261206368617261637465727320616674657220636c6f73652d6272616365 1 MISSING
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 8.6.18
VALUE
ROW selected_braced_var 0 31 0 32 1 2
ROW terminal_word 0 31 1 6578747261206368617261637465727320616674657220636c6f73652d6272616365 1 MISSING
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 9.0.4
ROW selected_braced_var 0 30 1 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d65 1 MISSING
ROW terminal_word 0 31 1 6578747261206368617261637465727320616674657220636c6f73652d6272616365 1 MISSING
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 9.1.0
ROW selected_braced_var 0 30 1 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d65 1 MISSING
ROW terminal_word 0 31 1 6578747261206368617261637465727320616674657220636c6f73652d6272616365 1 MISSING
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION 0.84-9-g5bac7c9
VALUE
ROW selected_braced_var 0 31 0 32 1 2
ROW terminal_word 0 31 1 6578747261206368617261637465727320616674657220636c6f73652d6272616365 MISSING MISSING
```

- Standard error: empty.
