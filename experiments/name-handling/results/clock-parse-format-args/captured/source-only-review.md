# C8.6 ParseFormatArgs support worker

The reported matching `all.tcl` run stops before `var.test` when real clock
format calls missing `ParseFormatArgs`; that failure is separate from the
1182 passing naming assertions and successful real init at5597256ba.
Original owner: `tmp/tcl8.6.18/generic/tclClock.c:187–189,219,242–305` registers
`::tcl::clock::ParseFormatArgs` through `Tcl_CreateObjCommand` for non-safe interps, outside the ensemble
map. `ClockParseformatargsObjCmd(ClientData,Tcl_Interp*,int,Tcl_Obj*const[])`
at1895–1990 owns the parser. Existing [constructor capture](../../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/c-fresh.stdout.txt)
measures this command present/non-proc/NULL-hook, independent of effects.
The real `library/clock.tcl:664–672` procedure invokes
`lassign [ParseFormatArgs {*}$args] format locale timezone` before timezone
setup or rendering. Keeping that ordinary namespace lookup is required.
Arity: command + clockval + zero or more option/value pairs; `objc>=2` and even.
Wrong arity uses the fixed `clock format clockval ...` usage and errorCode
`CLOCK wrongNumArgs`. The result is **three** Tcl list elements, not the stale
C comment's four: format, locale, timezone. Defaults (`literals:72–81`) are
`%a %b %d %H:%M:%S %Z %Y`, `C`, empty; true `-gmt` selects `:GMT`.
`Tcl_GetIndexFromObj(...,0)` admits unique prefixes of -format/-gmt/-locale/
-timezone. Last repeated value wins; every -gmt value is boolean-converted.
Options are parsed first; then `TclGetWideIntFromObj` validates clockval.
Seeing both -gmt and -timezone errors even for false -gmt, after time validation:
`cannot use -gmt and -timezone in same call`, `CLOCK gmtWithTimezone`.
Bad/ambiguous option overrides errorCode to `CLOCK badOption <original option>`;
boolean/time conversion errors retain their original shared conversion errors.
Object protocol: results reuse supplied option objects or refcounted
`ClockClientData` literal-pool objects; `Tcl_NewListObj(3,results)` retains them.
Registration increments pool refcount; deletion at2114–2128 releases literals
only after the last worker. NULL hook does not establish purity/no side effects.
C8.5's table at223 also registers this worker; behaviour not probed here.
C8.4 has monolithic `Tcl_ClockObjCmd`; C9.0/C9.1 tables at134/142 omit this
worker and register C format handlers. Pinned Jim5bac7c99 clockInit at213–218
registers its clock subcommands, with no ParseFormatArgs source definition.
These source observations grant no cross-release handler or F5 result claim.

Next bounded repair: registry-owned C8.6-only support-worker admission/identity,
arity/options and measured NULL-hook registration; one shared `tcl_cmd_core`
parser consumed by `cmd_clock` with existing original-object conversion/list
owners and retained literals. Do not forward to `clock format` or its current
core formatter (which validates time first, ignores locale/timezone, renders).
Test returned object lifetime after worker retirement, errors/order and native
relative epochs. Keep the other six support workers and map order separate.

Next discrete oracle input, unexecuted here; run unchanged in fresh original
C8.4–9.1 and latest pinned Jim processes with matching real startup, retain
input/source/binary/library hashes and exact exits/streams in results/clock-parse-format-args/:
```tcl
set cases {{} {0} {0 -g 1} {0 -format A -format B} {0 -gmt 0 -timezone :GMT} {bad -gmt bad} {bad -gmt 1 -timezone :GMT} {0 -z X} {0 - X} {0 -format} {9223372036854775808}}
foreach args $cases {set ::errorCode UNSET; set code [catch {eval [linsert $args 0 ::tcl::clock::ParseFormatArgs]} value]; puts [list $args $code $value $::errorCode]}
```
