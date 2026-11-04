# Tcl execution evidence

## Question

> Trace prefixes installed in A and triggered in B can be misbound by an installer namespace or an unavailable future frame. Absolute and relative heads require separate observed lookup answers for variable, command and execution callbacks.

> Do absolute trace callback heads select Dest while relative heads select B for the exact variable write, command rename and execution enter/leave traces installed in A and triggered in B?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Original fixed ASCII LF source-file CLI on C8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9. Twenty original request/probe/receipt/stream files are retained exactly; independent current release-source excerpts explain only the selected evaluation entry. BIG-IP is not tested.

## Script

```tcl-visible
# Exact ASCII source-input lookup experiment. No internal native object or argv inspection.
puts [list version [info patchlevel]]
puts [list trace-command [info commands trace]]
set ::events {}
namespace eval ::Dest {
    proc cb {tag args} {
        lappend ::events [list $tag Dest [llength $args] [uplevel 1 {namespace current}]]
    }
}
namespace eval ::A {
    proc cb {tag args} {
        lappend ::events [list $tag A [llength $args] [uplevel 1 {namespace current}]]
    }
}
namespace eval ::B {
    proc cb {tag args} {
        lappend ::events [list $tag B [llength $args] [uplevel 1 {namespace current}]]
    }
    proc variable_trigger {} {set ::watched VALUE}
    proc command_trigger {} {rename ::target ::moved}
    proc execution_trigger {} {::target}
}
proc ::target {} {return VALUE}
foreach head {cb ::Dest::cb} {
    foreach kind {variable command execution} {
        set ::events {}
        if {$kind == "variable"} {
            set operations write
            set name ::watched
        } elseif {$kind == "command"} {
            set operations rename
            set name ::target
        } else {
            set operations {enter leave}
            set name ::target
        }
        set prefix [list $head $kind]
        set addCode [catch {namespace eval ::A [list trace add $kind $name $operations $prefix]} addResult]
        if {$addCode} {
            puts [list add $kind $head $addCode $addResult]
            continue
        }
        set trigger [list ::B::${kind}_trigger]
        set fireCode [catch $trigger fireResult]
        puts [list fire $kind $head $fireCode $fireResult $::events]
        if {$kind == "command"} {
            catch {trace remove command ::moved $operations $prefix}
            rename ::moved ::target
        } else {
            catch [list trace remove $kind $name $operations $prefix]
        }
    }
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20
trace-command trace
fire variable cb 0 VALUE {{variable B 3 ::B}}
fire command cb 0 {} {{command B 3 ::B}}
fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}}
fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}}
fire command ::Dest::cb 0 {} {{command Dest 3 ::B}}
fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19
trace-command trace
fire variable cb 0 VALUE {{variable B 3 ::B}}
fire command cb 0 {} {{command B 3 ::B}}
fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}}
fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}}
fire command ::Dest::cb 0 {} {{command Dest 3 ::B}}
fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18
trace-command trace
fire variable cb 0 VALUE {{variable B 3 ::B}}
fire command cb 0 {} {{command B 3 ::B}}
fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}}
fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}}
fire command ::Dest::cb 0 {} {{command Dest 3 ::B}}
fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4
trace-command trace
fire variable cb 0 VALUE {{variable B 3 ::B}}
fire command cb 0 {} {{command B 3 ::B}}
fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}}
fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}}
fire command ::Dest::cb 0 {} {{command Dest 3 ::B}}
fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0
trace-command trace
fire variable cb 0 VALUE {{variable B 3 ::B}}
fire command cb 0 {} {{command B 3 ::B}}
fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}}
fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}}
fire command ::Dest::cb 0 {} {{command Dest 3 ::B}}
fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9
trace-command {}
add variable cb 1 {invalid command name "trace"}
add command cb 1 {invalid command name "trace"}
add execution cb 1 {invalid command name "trace"}
add variable ::Dest::cb 1 {invalid command name "trace"}
add command ::Dest::cb 1 {invalid command name "trace"}
add execution ::Dest::cb 1 {invalid command name "trace"}
```

- Standard error: empty.
