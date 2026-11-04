# Tcl execution evidence

## Question

> An after idle script is scheduled by a procedure in ::A with competing global and ::A callback names. Reusing the installer namespace would choose a different delayed implementation; timing alone says when it runs, not where its head resolves.

> When the exact ::A::queue schedules after idle delayed and update reaches it, which namespace resolves delayed?

> A comparator named cmp exists in both the root namespace and ::A. Resolving the receiving lsort head or copying its prefix does not establish where the comparator itself will resolve; choosing the root table would navigate or analyze the wrong implementation.

> In the exact selected lsort -command call inside ::A::sort, which namespace resolves an unqualified cmp callback?

> A trace prefix is installed in ::A while the operation that fires it runs in ::B, and matching prefix heads also exist at root. Resolving a prefix at installation can bind the wrong callback even when prefix bytes remain intact. Variable, command and execution trace purposes each need an actual triggering-frame observation.

> Which namespace resolves unqualified prefix heads for the exact variable write, command rename and execution enter traces installed in ::A and triggered by ::B procedures?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

Recorded scope and limits:

> Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

Recorded scope and limits:

> Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

## Script

```tcl-visible
puts [list provider [info patchlevel]]
set ::seen {}
proc cmp {a b} {lappend ::seen ROOT; return 0}
namespace eval ::A {
    proc cmp {a b} {lappend ::seen A [uplevel 1 {info level}] [uplevel 1 {namespace current}]; return 0}
    proc sort {} {lsort -command cmp {b a}}
}
set code [catch {::A::sort} result]
puts [list lsort $code $result $::seen]
set ::seen {}
proc delayed {} {lappend ::seen ROOT [info level] [namespace current]}
namespace eval ::A {
    proc delayed {} {lappend ::seen A}
    proc queue {} {after idle delayed}
}
set code [catch {::A::queue; update} result]
puts [list after $code $result $::seen]
set ::seen {}
proc watch {n1 n2 op} {lappend ::seen ROOT}
namespace eval ::A {proc watch {n1 n2 op} {lappend ::seen A}}
namespace eval ::B {
    proc watch {n1 n2 op} {lappend ::seen B [uplevel 1 {info level}] [uplevel 1 {namespace current}]}
    proc trigger {} {set ::observed VALUE}
}
set code [catch {namespace eval ::A {trace add variable ::observed write watch}; ::B::trigger} result]
puts [list trace $code $result $::seen]
set ::seen {}
proc observedCommand {} {return VALUE}
proc watchCommand {old new op} {lappend ::seen ROOT}
namespace eval ::A {proc watchCommand {old new op} {lappend ::seen A}}
namespace eval ::B {
    proc watchCommand {old new op} {lappend ::seen B [uplevel 1 {info level}] [uplevel 1 {namespace current}]}
    proc triggerCommand {} {rename ::observedCommand ::renamedCommand}
}
set code [catch {namespace eval ::A {trace add command ::observedCommand rename watchCommand}; ::B::triggerCommand} result]
puts [list command-trace $code $result $::seen]
set ::seen {}
proc observedExecution {value} {return $value}
proc watchExecution {cmd op} {lappend ::seen ROOT}
namespace eval ::A {proc watchExecution {cmd op} {lappend ::seen A}}
namespace eval ::B {
    proc watchExecution {cmd op} {lappend ::seen B [uplevel 1 {info level}] [uplevel 1 {namespace current}]}
    proc triggerExecution {} {::observedExecution VALUE}
}
set code [catch {namespace eval ::A {trace add execution ::observedExecution enter watchExecution}; ::B::triggerExecution} result]
puts [list execution-trace $code $result $::seen]
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: ASCII source file through selected native CLI
- Exit status: Not recorded
- Standard output:

```text-visible
provider 8.4.20
lsort 0 {b a} {A 1 ::A}
after 0 {} {ROOT 1 ::}
trace 0 VALUE {B 1 ::B}
command-trace 0 {} {B 1 ::B}
execution-trace 0 VALUE {B 1 ::B}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: ASCII source file through selected native CLI
- Exit status: Not recorded
- Standard output:

```text-visible
provider 8.5.19
lsort 0 {b a} {A 1 ::A}
after 0 {} {ROOT 1 ::}
trace 0 VALUE {B 1 ::B}
command-trace 0 {} {B 1 ::B}
execution-trace 0 VALUE {B 1 ::B}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: ASCII source file through selected native CLI
- Exit status: Not recorded
- Standard output:

```text-visible
provider 8.6.18
lsort 0 {b a} {A 1 ::A}
after 0 {} {ROOT 1 ::}
trace 0 VALUE {B 1 ::B}
command-trace 0 {} {B 1 ::B}
execution-trace 0 VALUE {B 1 ::B}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: ASCII source file through selected native CLI
- Exit status: Not recorded
- Standard output:

```text-visible
provider 9.0.4
lsort 0 {b a} {A 1 ::A}
after 0 {} {ROOT 1 ::}
trace 0 VALUE {B 1 ::B}
command-trace 0 {} {B 1 ::B}
execution-trace 0 VALUE {B 1 ::B}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: ASCII source file through selected native CLI
- Exit status: Not recorded
- Standard output:

```text-visible
provider 9.1.0
lsort 0 {b a} {A 1 ::A}
after 0 {} {ROOT 1 ::}
trace 0 VALUE {B 1 ::B}
command-trace 0 {} {B 1 ::B}
execution-trace 0 VALUE {B 1 ::B}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: ASCII source file through selected native CLI
- Exit status: Not recorded
- Standard output:

```text-visible
provider 0.84-9-g5bac7c9
lsort 0 {b a} {A 1 ::A}
after 0 {ROOT 1 ::} {ROOT 1 ::}
trace 1 {invalid command name "trace"} {}
command-trace 1 {invalid command name "trace"} {}
execution-trace 1 {invalid command name "trace"} {}
```

- Standard error: empty.
