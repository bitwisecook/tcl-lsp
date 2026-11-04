# Tcl execution evidence

## Question

> The first dictionary mapping assignment, an array-valued second destination, an explicit upvar alias, Jim reference-formal syntax and a direct multiword worker can produce different public results. The exact captured foreach/catch variable-source input channel must remain explicit alongside independently retained specimens.

> How do native dictionary update, array-root links and Jim reference formals reach the first mapped assignment and body?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Original fixed ASCII LF CLI file reports PATCHLEVEL, then iterates five exact script values through catch $script. All controls share one fresh provider process but use distinct authored dictionary and alias names. The outer catch code and inner caught code/result/value tuple are separately retained; process exit zero does not imply every guest operation succeeded. Stock C8.4 dictionary absence applies only to this installation, with the alias and reference-formal controls still executed. No raw/counting NUL, non-ASCII, continuation or control escape is exercised. Exact input source, command, executable and required library/header/build/source pins are checked independently. The public variable controls supply no class-carrier authority. Other provider, input envelope and independent fixture results remain independent. No Rust assertion pass, source-carrier authority, representation identity, bytecode causal explanation, actual frame/storage allocation, broad parity or edit authority follows.

## Script

```tcl-visible
if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts [list PATCHLEVEL $probeVersion]
foreach {label script} {
    update_scalar {set d {first NEW};set ok BEFORE;set entered 0;set c [catch {dict update d first ok {set entered 1}} r];list $c $r $ok $entered $d}
    update_array_second {array set blocked {k OLD};set d {first NEW second OTHER};set ok BEFORE;set entered 0;set c [catch {dict update d first ok second blocked {set entered 1}} r];list $c $r $ok $entered [catch {set blocked} v] $v $d}
    alias_array_replacement {array set aliasSource {k OLD};proc probeAliasWrite {} {upvar 1 aliasSource original;set original OTHER};set c [catch {probeAliasWrite} r];list $c $r [array exists aliasSource] [catch {set aliasSource} v] $v}
    reference_formal {set referenceSource {first NEW};proc probeReference {&original} {list $original};set c [catch {probeReference referenceSource} r];list $c $r $referenceSource}
    update_direct_multiword {set directSource {first DIRECT};set directTarget BEFORE;set entered 0;set c [catch {{dict update} directSource first directTarget {set entered 1}} r];list $c $r $directTarget $entered $directSource}
} {
    set probeCode [catch $script probeResult]
    puts [list RESULT $label $probeCode $probeResult]
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.4.20
RESULT update_scalar 0 {1 {invalid command name "dict"} BEFORE 0 {first NEW}}
RESULT update_array_second 0 {1 {invalid command name "dict"} BEFORE 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}
RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}
RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}
RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.5.19
RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}
RESULT update_array_second 0 {1 {can't set "blocked": variable is array} NEW 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}
RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}
RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}
RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.6.18
RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}
RESULT update_array_second 0 {1 {can't set "blocked": variable is array} NEW 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}
RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}
RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}
RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.0.4
RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}
RESULT update_array_second 0 {1 {can't set "blocked": variable is array} NEW 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}
RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}
RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}
RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.1.0
RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}
RESULT update_array_second 0 {1 {can't set "blocked": variable is array} NEW 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}
RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}
RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}
RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 0.84-9-g5bac7c9
RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}
RESULT update_array_second 0 {0 1 NEW 1 0 OTHER {first NEW second OTHER}}
RESULT alias_array_replacement 0 {0 OTHER 0 0 OTHER}
RESULT reference_formal 0 {0 {{first NEW}} {first NEW}}
RESULT update_direct_multiword 0 {0 1 DIRECT 1 {first DIRECT}}
```

- Standard error: empty.
