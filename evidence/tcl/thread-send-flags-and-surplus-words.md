# Thread send flags and trailing result words

## Question

Which argument holds the target thread and script when `thread::send` combines
its flags, does `-command` consume an extra option operand, and does the
implementation reject surplus words after a synchronous result-variable name?
The registry must preserve the actual operand coordinates and source arity
rather than infer a stricter grammar from the manual synopsis.

## Assumptions

The directly loaded shared library was built from upstream Thread 3.0.4's
`thread-3-0-4` tag against the source-built C Tcl 9.0.4 tree. The worker uses
Thread's default event loop. These are observations of that source-built
package, not either Rust engine or another Thread release. The ignored-word
behaviour is an observed implementation detail; it is not a recommended API.

## Script

```tcl-visible
load /tmp/thread-source-3.0.4/libtcl9thread3.0.4.so Thread
set tid [thread::create]
puts [list excess [thread::send $tid {expr 42} result ignored alsoIgnored] $result]
set got absent
proc callback {value} {set ::got $value}
puts [list queued [thread::send -async -head $tid {expr 43} resultAsync]]
vwait resultAsync
puts [list asynchronous $resultAsync]
puts [list callbackQueued [thread::send -command -head $tid {expr 44} callback]]
vwait got
puts [list callbackResult $got]
puts [list flagsTooMany [catch {thread::send -async -head -command $tid {expr 45} callback}]]
thread::release $tid
```

## Results

### C Tcl 9.0.4 with Thread 3.0.4

Invocation: source-built `tmp/tcl9.0.4/unix/tclsh /tmp/thread-send-layout-probe.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
excess 0 42
queued {}
asynchronous 43
callbackQueued {}
callbackResult 44
flagsTooMany 1
```

- Standard error: empty.
