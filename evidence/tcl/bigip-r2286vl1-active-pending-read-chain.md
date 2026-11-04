# BIG-IP variable lifecycle: active-pending-read-chain

## Question

When a read trace destroys and recreates its variable, do later pending read traces still run? This constrains trace dispatch and variable-cell lifetime in the compiler and VM.

## Assumptions

The payload was evaluated under catch in an HTTP_REQUEST event, sequentially with 60 other cases in the same event frame. Earlier cases can affect later cases, so the results do not assume a fresh interpreter or unset variables. The script's own nested catch and hex encoding, where present, remain part of the result. The observations are limited to the recorded context and inputs. The immediately preceding case was [active-element](bigip-r2286vl1-active-element.md).

## Script

```tcl-visible
set events {}; set v OLD
proc later {n k op} {upvar 1 events events; lappend events later-$op}
proc kill {n k op} {upvar 1 events events; lappend events kill-$op; if {$op eq "read"} {uplevel 1 {unset v; set v NEW}}}
trace add variable v read later
trace add variable v {read unset} kill
set h lappend
set answer [$h v EXTRA]
set summary [list $answer [set v] $events [trace info variable v]]
set summary
```

## Results

### TMM 0:0, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {kill-read kill-unset} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42103 included this case in the sequential control matrix.

### TMM 0:1, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {kill-read kill-unset} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42102 included this case in the sequential control matrix.

### TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {kill-read kill-unset} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42101 included this case in the sequential control matrix.

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {kill-read kill-unset} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42100 included this case in the sequential control matrix.
