# Tcl execution evidence

## Question

> A readonly callback source producer can retain prefix words, but alias target-name relookup, nested capture order, moves and later replacement require independent public native controls. Error labels also differ by release and cannot identify a source declaration.

> What exact public comparator arguments, alias moves, target-name relookup and replacement-arity results do the retained lsort callback alias controls produce?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Original fixed ASCII LF CLI file on five pinned C releases and current Jim0.84-9-g5bac7c9. Twenty original input/receipt/stream files retain exact release-specific outcomes and independently checked executable/library/header/Makefile/source pins. The two-item lsort vectors retain the exact recorded comparison order. BIG-IP is not tested.

## Script

```tcl-visible
puts [list version [info patchlevel] alias_command [info commands alias]]
proc create_alias {name target captured} {
    if {[llength [info commands alias]] != 0} {
        eval [concat [list alias $name $target] $captured]
    } else {
        eval [concat [list interp alias {} $name {} $target] $captured]
    }
}
proc target args {
    lappend ::observations $args
    return 0
}
set ::observations {}
set code [catch {create_alias cb target {FIXED}; lsort -command cb {2 1}} result]
puts [list single $code $result $::observations]
set ::observations {}
set code [catch {lsort -command {cb BAKED} {2 1}} result]
puts [list baked $code $result $::observations]
set ::observations {}
set code [catch {create_alias inner target {INNER}; create_alias outer inner {OUTER}; lsort -command {outer BAKED} {2 1}} result]
puts [list nested $code $result $::observations]
set ::observations {}
set code [catch {rename cb moved_cb; lsort -command moved_cb {2 1}} result]
puts [list moved_alias $code $result $::observations]
set ::observations {}
set code [catch {rename target moved_target; lsort -command moved_cb {2 1}} result]
puts [list moved_target $code $result $::observations]
proc target {a b c d} {return 0}
set code [catch {lsort -command moved_cb {2 1}} result]
puts [list replacement_four $code $result]
set code [catch {lsort -command {moved_cb BAKED} {2 1}} result]
puts [list replacement_baked_four $code $result]
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim.
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20 alias_command {}
single 0 {2 1} {{FIXED 2 1}}
baked 0 {2 1} {{FIXED BAKED 2 1}}
nested 0 {2 1} {{INNER OUTER BAKED 2 1}}
moved_alias 0 {2 1} {{FIXED 2 1}}
moved_target 1 {invalid command name "target"} {}
replacement_four 1 {wrong # args: should be "target a b c d"}
replacement_baked_four 0 {2 1}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim.
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19 alias_command {}
single 0 {2 1} {{FIXED 2 1}}
baked 0 {2 1} {{FIXED BAKED 2 1}}
nested 0 {2 1} {{INNER OUTER BAKED 2 1}}
moved_alias 0 {2 1} {{FIXED 2 1}}
moved_target 1 {invalid command name "target"} {}
replacement_four 1 {wrong # args: should be "moved_cb b c d"}
replacement_baked_four 0 {2 1}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim.
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18 alias_command {}
single 0 {2 1} {{FIXED 2 1}}
baked 0 {2 1} {{FIXED BAKED 2 1}}
nested 0 {2 1} {{INNER OUTER BAKED 2 1}}
moved_alias 0 {2 1} {{FIXED 2 1}}
moved_target 1 {invalid command name "target"} {}
replacement_four 1 {wrong # args: should be "moved_cb b c d"}
replacement_baked_four 0 {2 1}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim.
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4 alias_command {}
single 0 {2 1} {{FIXED 2 1}}
baked 0 {2 1} {{FIXED BAKED 2 1}}
nested 0 {2 1} {{INNER OUTER BAKED 2 1}}
moved_alias 0 {2 1} {{FIXED 2 1}}
moved_target 1 {invalid command name "target"} {}
replacement_four 1 {wrong # args: should be "moved_cb b c d"}
replacement_baked_four 0 {2 1}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim.
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0 alias_command {}
single 0 {2 1} {{FIXED 2 1}}
baked 0 {2 1} {{FIXED BAKED 2 1}}
nested 0 {2 1} {{INNER OUTER BAKED 2 1}}
moved_alias 0 {2 1} {{FIXED 2 1}}
moved_target 1 {invalid command name "target"} {}
replacement_four 1 {wrong # args: should be "moved_cb b c d"}
replacement_baked_four 0 {2 1}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim.
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9 alias_command alias
single 0 {2 1} {{FIXED 2 1}}
baked 0 {2 1} {{FIXED BAKED 2 1}}
nested 0 {2 1} {{INNER OUTER BAKED 2 1}}
moved_alias 0 {2 1} {{FIXED 2 1}}
moved_target 1 {invalid command name "target"} {}
replacement_four 1 {wrong # args: should be "target a b c d"}
replacement_baked_four 0 {2 1}
```

- Standard error: empty.
