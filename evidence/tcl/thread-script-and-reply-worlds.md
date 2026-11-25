# Thread script and result-variable execution coordinates

## Question

Does a sent Thread script execute in the target thread's global interpreter,
where are synchronous and asynchronous result variables written, and is the
`tsv` store shared across those interpreter worlds? These distinctions determine
registry script contexts and prevent shared-store effects being treated as
ordinary local Tcl variable cells.

## Assumptions

Thread 3.0.4 was built from the upstream `thread-3-0-4` tag against the source-built
C Tcl 9.0.4 tree. The script loads that build's shared library directly. The
package was not installed system-wide. The worker explicitly enters its event
loop. These observations cover this package, release and invocation channel;
they do not establish Thread behaviour in either Rust engine or another release.

## Script

```tcl-visible
load /tmp/thread-source-3.0.4/libtcl9thread3.0.4.so Thread
puts [package provide Thread]
set ::world parent
set tid [thread::create {set ::world child; thread::wait}]
proc synchronous {tid} {
    thread::send $tid {list $::world [info level] [namespace current]} result
    list $result [info exists result] [info exists ::result]
}
puts [synchronous $tid]
proc asynchronous {tid} {
    thread::send -async $tid {list $::world [info level] [namespace current]} ::reply
    vwait ::reply
    list $::reply [info exists reply]
}
puts [asynchronous $tid]
tsv::set shared key parent
puts [thread::send $tid {tsv::get shared key}]
thread::release $tid
```

## Results

### C Tcl 9.0.4 with Thread 3.0.4

Invocation: source-built `tmp/tcl9.0.4/unix/tclsh /tmp/thread-context-probe.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
3.0.4
{child 0 ::} 1 0
{child 0 ::} 0
parent
```

- Standard error: empty.
