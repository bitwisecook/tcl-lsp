# naming.variable.dictionary-rmw-command-versus-selected-local-cell

Kind: `native-observation`

## Problem statement

A read/write callback can mutate or relink a dictionary receiver between reading its object and publishing the new value. A generic command may revisit the current variable binding while an opcode retains an earlier local cell; treating these as one receiver would hide callback order and lifetime differences.

## Question

What completions and receiver/other/log tuples result from the sixty exact compiled-head versus generic-head dict set/unset/incr/append/lappend callback and ordinary controls?

## Conclusion

The 345 completed native processes establish only the exact command/local-cell publication and callback outcomes listed here. Fifteen C callback-relink processes terminated by signal and are excluded from successful Rust comparisons; their statuses remain evidence. C84 dict absence and Jim trace absence are measured guest errors, not missing rows or positive callback closure.

## Scope

Sixty original ASCII scripts evaluated by counted C/Jim object entry points in fresh interpreters/processes; C84-91 and the manifested Jim build. Result/options are captured before later reporter reads. Selected local-cell and generic command inputs are distinct. Signal controls do not establish semantic equivalence, normal completion, cell identity, or custom object release safety.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: binary_sha256=35c3c0a8727935bf8e58d39946b907ba0385d4003d36f9f5220cc34e157881bc; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| compiled-set-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-set-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-set-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-set-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-set-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-unset-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-unset-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-unset-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-unset-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-unset-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-incr-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-incr-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-incr-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-incr-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-incr-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-append-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-append-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-append-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-append-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-append-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-lappend-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-lappend-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-lappend-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-lappend-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-lappend-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| compiled-set-plain | 1 | "invalid command name \"dict\"" |
| compiled-unset-plain | 1 | "invalid command name \"dict\"" |
| compiled-incr-plain | 1 | "invalid command name \"dict\"" |
| compiled-append-plain | 1 | "invalid command name \"dict\"" |
| compiled-lappend-plain | 1 | "invalid command name \"dict\"" |
| generic-set-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-set-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-set-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-set-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-set-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-unset-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-unset-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-unset-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-unset-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-unset-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-incr-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-incr-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-incr-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-incr-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-incr-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-append-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-append-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-append-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-append-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-append-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-lappend-read-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-lappend-read-relink | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-lappend-read-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-lappend-write-change | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-lappend-write-error | 0 | "1 {invalid command name \"dict\"} {k 3} {k OTHER} {}" |
| generic-set-plain | 1 | "invalid command name \"dict\"" |
| generic-unset-plain | 1 | "invalid command name \"dict\"" |
| generic-incr-plain | 1 | "invalid command name \"dict\"" |
| generic-append-plain | 1 | "invalid command name \"dict\"" |
| generic-lappend-plain | 1 | "invalid command name \"dict\"" |

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: binary_sha256=1d497488b75341feafdb48c82ae648a0aa43db0be606ed9fe8c27b6ac098fceb; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| compiled-set-read-change | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| compiled-set-read-error | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| compiled-set-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-set-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k NEXT} {k OTHER} write" |
| compiled-unset-read-change | 0 | "0 {} {} {k OTHER} read" |
| compiled-unset-read-relink | 0 | "0 {} {} {} read" |
| compiled-unset-read-error | 0 | "0 {} {} {k OTHER} read" |
| compiled-unset-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-unset-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {} {k OTHER} write" |
| compiled-incr-read-change | 0 | "1 {expected integer but got \"OBS\"} {k OBS} {k OTHER} read" |
| compiled-incr-read-error | 0 | "0 {k 2} {k 2} {k OTHER} read" |
| compiled-incr-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-incr-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 5} {k OTHER} write" |
| compiled-append-read-change | 0 | "0 {k OBSYZ} {k OBSYZ} {k OTHER} read" |
| compiled-append-read-error | 0 | "0 {k YZ} {k YZ} {k OTHER} read" |
| compiled-append-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-append-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 3YZ} {k OTHER} write" |
| compiled-lappend-read-change | 0 | "0 {k {OBS Y Z}} {k {OBS Y Z}} {k OTHER} read" |
| compiled-lappend-read-relink | 0 | "0 {k {Y Z}} {k {Y Z}} {k {Y Z}} read" |
| compiled-lappend-read-error | 0 | "0 {k {Y Z}} {k {Y Z}} {k OTHER} read" |
| compiled-lappend-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-lappend-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k {3 Y Z}} {k OTHER} write" |
| compiled-set-plain | 0 | "k NEXT" |
| compiled-unset-plain | 0 | "" |
| compiled-incr-plain | 0 | "k 5" |
| compiled-append-plain | 0 | "k 3YZ" |
| compiled-lappend-plain | 0 | "k {3 Y Z}" |
| generic-set-read-change | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| generic-set-read-relink | 0 | "0 {k NEXT} {k NEXT} {k NEXT} read" |
| generic-set-read-error | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| generic-set-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-set-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k NEXT} {k OTHER} write" |
| generic-unset-read-change | 0 | "0 {} {} {k OTHER} read" |
| generic-unset-read-relink | 0 | "0 {} {} {} read" |
| generic-unset-read-error | 0 | "0 {} {} {k OTHER} read" |
| generic-unset-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-unset-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {} {k OTHER} write" |
| generic-incr-read-change | 0 | "1 {expected integer but got \"OBS\"} {k OBS} {k OTHER} read" |
| generic-incr-read-relink | 0 | "0 {k 2} {k 2} {k 2} read" |
| generic-incr-read-error | 0 | "0 {k 2} {k 2} {k OTHER} read" |
| generic-incr-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-incr-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 5} {k OTHER} write" |
| generic-append-read-change | 0 | "0 {k OBSYZ} {k OBSYZ} {k OTHER} read" |
| generic-append-read-relink | 0 | "0 {k YZ} {k YZ} {k YZ} read" |
| generic-append-read-error | 0 | "0 {k YZ} {k YZ} {k OTHER} read" |
| generic-append-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-append-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 3YZ} {k OTHER} write" |
| generic-lappend-read-change | 0 | "0 {k {OBS Y Z}} {k {OBS Y Z}} {k OTHER} read" |
| generic-lappend-read-relink | 0 | "0 {k {Y Z}} {k {Y Z}} {k {Y Z}} read" |
| generic-lappend-read-error | 0 | "0 {k {Y Z}} {k {Y Z}} {k OTHER} read" |
| generic-lappend-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-lappend-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k {3 Y Z}} {k OTHER} write" |
| generic-set-plain | 0 | "k NEXT" |
| generic-unset-plain | 0 | "" |
| generic-incr-plain | 0 | "k 5" |
| generic-append-plain | 0 | "k 3YZ" |
| generic-lappend-plain | 0 | "k {3 Y Z}" |

Signal/nonzero controls excluded from completed rows: [{"case": "compiled-set-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-incr-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-append-read-relink", "exit": -11, "rows": 1}]. The manifest records process outcomes independently of dictionary guest completion.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: binary_sha256=3d5132fcfa0e09fb2418db5e0ff8a787be89485897e76ff783cbe04d49221464; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| compiled-set-read-change | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| compiled-set-read-error | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| compiled-set-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-set-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k NEXT} {k OTHER} write" |
| compiled-unset-read-change | 0 | "0 {} {} {k OTHER} read" |
| compiled-unset-read-error | 0 | "0 {} {} {k OTHER} read" |
| compiled-unset-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-unset-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {} {k OTHER} write" |
| compiled-incr-read-change | 0 | "1 {expected integer but got \"OBS\"} {k OBS} {k OTHER} read" |
| compiled-incr-read-error | 0 | "0 {k 2} {k 2} {k OTHER} read" |
| compiled-incr-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-incr-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 5} {k OTHER} write" |
| compiled-append-read-change | 0 | "0 {k OBSYZ} {k OBSYZ} {k OTHER} read" |
| compiled-append-read-error | 0 | "0 {k YZ} {k YZ} {k OTHER} read" |
| compiled-append-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-append-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 3YZ} {k OTHER} write" |
| compiled-lappend-read-change | 0 | "0 {k {OBS Y Z}} {k {OBS Y Z}} {k OTHER} read" |
| compiled-lappend-read-relink | 0 | "0 {k {Y Z}} {k {Y Z}} {k {Y Z}} read" |
| compiled-lappend-read-error | 0 | "0 {k {Y Z}} {k {Y Z}} {k OTHER} read" |
| compiled-lappend-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-lappend-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k {3 Y Z}} {k OTHER} write" |
| compiled-set-plain | 0 | "k NEXT" |
| compiled-unset-plain | 0 | "" |
| compiled-incr-plain | 0 | "k 5" |
| compiled-append-plain | 0 | "k 3YZ" |
| compiled-lappend-plain | 0 | "k {3 Y Z}" |
| generic-set-read-change | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| generic-set-read-relink | 0 | "0 {k NEXT} {k NEXT} {k NEXT} read" |
| generic-set-read-error | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| generic-set-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-set-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k NEXT} {k OTHER} write" |
| generic-unset-read-change | 0 | "0 {} {} {k OTHER} read" |
| generic-unset-read-relink | 0 | "0 {} {} {} read" |
| generic-unset-read-error | 0 | "0 {} {} {k OTHER} read" |
| generic-unset-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-unset-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {} {k OTHER} write" |
| generic-incr-read-change | 0 | "1 {expected integer but got \"OBS\"} {k OBS} {k OTHER} read" |
| generic-incr-read-relink | 0 | "0 {k 2} {k 2} {k 2} read" |
| generic-incr-read-error | 0 | "0 {k 2} {k 2} {k OTHER} read" |
| generic-incr-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-incr-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 5} {k OTHER} write" |
| generic-append-read-change | 0 | "0 {k OBSYZ} {k OBSYZ} {k OTHER} read" |
| generic-append-read-relink | 0 | "0 {k YZ} {k YZ} {k YZ} read" |
| generic-append-read-error | 0 | "0 {k YZ} {k YZ} {k OTHER} read" |
| generic-append-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-append-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 3YZ} {k OTHER} write" |
| generic-lappend-read-change | 0 | "0 {k {OBS Y Z}} {k {OBS Y Z}} {k OTHER} read" |
| generic-lappend-read-relink | 0 | "0 {k {Y Z}} {k {Y Z}} {k {Y Z}} read" |
| generic-lappend-read-error | 0 | "0 {k {Y Z}} {k {Y Z}} {k OTHER} read" |
| generic-lappend-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-lappend-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k {3 Y Z}} {k OTHER} write" |
| generic-set-plain | 0 | "k NEXT" |
| generic-unset-plain | 0 | "" |
| generic-incr-plain | 0 | "k 5" |
| generic-append-plain | 0 | "k 3YZ" |
| generic-lappend-plain | 0 | "k {3 Y Z}" |

Signal/nonzero controls excluded from completed rows: [{"case": "compiled-set-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-unset-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-incr-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-append-read-relink", "exit": -11, "rows": 1}]. The manifest records process outcomes independently of dictionary guest completion.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: binary_sha256=d2fa9e1db663eaa8d58d88ef1ded1b8383cc9a1ba1b935fae286ef82cd21245c; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| compiled-set-read-change | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| compiled-set-read-error | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| compiled-set-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-set-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k NEXT} {k OTHER} write" |
| compiled-unset-read-change | 0 | "0 {} {} {k OTHER} read" |
| compiled-unset-read-error | 0 | "0 {} {} {k OTHER} read" |
| compiled-unset-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-unset-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {} {k OTHER} write" |
| compiled-incr-read-change | 0 | "1 {expected integer but got \"OBS\"} {k OBS} {k OTHER} read" |
| compiled-incr-read-error | 0 | "0 {k 2} {k 2} {k OTHER} read" |
| compiled-incr-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-incr-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 5} {k OTHER} write" |
| compiled-append-read-change | 0 | "0 {k OBSYZ} {k OBSYZ} {k OTHER} read" |
| compiled-append-read-error | 0 | "0 {k YZ} {k YZ} {k OTHER} read" |
| compiled-append-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-append-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 3YZ} {k OTHER} write" |
| compiled-lappend-read-change | 0 | "0 {k {OBS Y Z}} {k {OBS Y Z}} {k OTHER} read" |
| compiled-lappend-read-relink | 0 | "0 {k {Y Z}} {k {Y Z}} {k {Y Z}} read" |
| compiled-lappend-read-error | 0 | "0 {k {Y Z}} {k {Y Z}} {k OTHER} read" |
| compiled-lappend-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-lappend-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k {3 Y Z}} {k OTHER} write" |
| compiled-set-plain | 0 | "k NEXT" |
| compiled-unset-plain | 0 | "" |
| compiled-incr-plain | 0 | "k 5" |
| compiled-append-plain | 0 | "k 3YZ" |
| compiled-lappend-plain | 0 | "k {3 Y Z}" |
| generic-set-read-change | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| generic-set-read-relink | 0 | "0 {k NEXT} {k NEXT} {k NEXT} read" |
| generic-set-read-error | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| generic-set-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-set-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k NEXT} {k OTHER} write" |
| generic-unset-read-change | 0 | "0 {} {} {k OTHER} read" |
| generic-unset-read-relink | 0 | "0 {} {} {} read" |
| generic-unset-read-error | 0 | "0 {} {} {k OTHER} read" |
| generic-unset-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-unset-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {} {k OTHER} write" |
| generic-incr-read-change | 0 | "1 {expected integer but got \"OBS\"} {k OBS} {k OTHER} read" |
| generic-incr-read-relink | 0 | "0 {k 2} {k 2} {k 2} read" |
| generic-incr-read-error | 0 | "0 {k 2} {k 2} {k OTHER} read" |
| generic-incr-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-incr-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 5} {k OTHER} write" |
| generic-append-read-change | 0 | "0 {k OBSYZ} {k OBSYZ} {k OTHER} read" |
| generic-append-read-relink | 0 | "0 {k YZ} {k YZ} {k YZ} read" |
| generic-append-read-error | 0 | "0 {k YZ} {k YZ} {k OTHER} read" |
| generic-append-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-append-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 3YZ} {k OTHER} write" |
| generic-lappend-read-change | 0 | "0 {k {OBS Y Z}} {k {OBS Y Z}} {k OTHER} read" |
| generic-lappend-read-relink | 0 | "0 {k {Y Z}} {k {Y Z}} {k {Y Z}} read" |
| generic-lappend-read-error | 0 | "0 {k {Y Z}} {k {Y Z}} {k OTHER} read" |
| generic-lappend-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-lappend-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k {3 Y Z}} {k OTHER} write" |
| generic-set-plain | 0 | "k NEXT" |
| generic-unset-plain | 0 | "" |
| generic-incr-plain | 0 | "k 5" |
| generic-append-plain | 0 | "k 3YZ" |
| generic-lappend-plain | 0 | "k {3 Y Z}" |

Signal/nonzero controls excluded from completed rows: [{"case": "compiled-set-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-unset-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-incr-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-append-read-relink", "exit": -11, "rows": 1}]. The manifest records process outcomes independently of dictionary guest completion.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: binary_sha256=5dee747ef25ebfa154280a04905936fee0789fab12f798ebe34ae93ee4fdce32; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| compiled-set-read-change | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| compiled-set-read-error | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| compiled-set-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-set-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k NEXT} {k OTHER} write" |
| compiled-unset-read-change | 0 | "0 {} {} {k OTHER} read" |
| compiled-unset-read-error | 0 | "0 {} {} {k OTHER} read" |
| compiled-unset-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-unset-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {} {k OTHER} write" |
| compiled-incr-read-change | 0 | "1 {expected integer but got \"OBS\"} {k OBS} {k OTHER} read" |
| compiled-incr-read-error | 0 | "0 {k 2} {k 2} {k OTHER} read" |
| compiled-incr-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-incr-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 5} {k OTHER} write" |
| compiled-append-read-change | 0 | "0 {k OBSYZ} {k OBSYZ} {k OTHER} read" |
| compiled-append-read-error | 0 | "0 {k YZ} {k YZ} {k OTHER} read" |
| compiled-append-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-append-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 3YZ} {k OTHER} write" |
| compiled-lappend-read-change | 0 | "0 {k {OBS Y Z}} {k {OBS Y Z}} {k OTHER} read" |
| compiled-lappend-read-relink | 0 | "0 {k {Y Z}} {k {Y Z}} {k {Y Z}} read" |
| compiled-lappend-read-error | 0 | "0 {k {Y Z}} {k {Y Z}} {k OTHER} read" |
| compiled-lappend-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| compiled-lappend-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k {3 Y Z}} {k OTHER} write" |
| compiled-set-plain | 0 | "k NEXT" |
| compiled-unset-plain | 0 | "" |
| compiled-incr-plain | 0 | "k 5" |
| compiled-append-plain | 0 | "k 3YZ" |
| compiled-lappend-plain | 0 | "k {3 Y Z}" |
| generic-set-read-change | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| generic-set-read-relink | 0 | "0 {k NEXT} {k NEXT} {k NEXT} read" |
| generic-set-read-error | 0 | "0 {k NEXT} {k NEXT} {k OTHER} read" |
| generic-set-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-set-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k NEXT} {k OTHER} write" |
| generic-unset-read-change | 0 | "0 {} {} {k OTHER} read" |
| generic-unset-read-relink | 0 | "0 {} {} {} read" |
| generic-unset-read-error | 0 | "0 {} {} {k OTHER} read" |
| generic-unset-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-unset-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {} {k OTHER} write" |
| generic-incr-read-change | 0 | "1 {expected integer but got \"OBS\"} {k OBS} {k OTHER} read" |
| generic-incr-read-relink | 0 | "0 {k 2} {k 2} {k 2} read" |
| generic-incr-read-error | 0 | "0 {k 2} {k 2} {k OTHER} read" |
| generic-incr-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-incr-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 5} {k OTHER} write" |
| generic-append-read-change | 0 | "0 {k OBSYZ} {k OBSYZ} {k OTHER} read" |
| generic-append-read-relink | 0 | "0 {k YZ} {k YZ} {k YZ} read" |
| generic-append-read-error | 0 | "0 {k YZ} {k YZ} {k OTHER} read" |
| generic-append-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-append-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k 3YZ} {k OTHER} write" |
| generic-lappend-read-change | 0 | "0 {k {OBS Y Z}} {k {OBS Y Z}} {k OTHER} read" |
| generic-lappend-read-relink | 0 | "0 {k {Y Z}} {k {Y Z}} {k {Y Z}} read" |
| generic-lappend-read-error | 0 | "0 {k {Y Z}} {k {Y Z}} {k OTHER} read" |
| generic-lappend-write-change | 0 | "0 {k POST} {k POST} {k OTHER} write" |
| generic-lappend-write-error | 0 | "1 {can't set \"d\": WRITE_FAIL} {k {3 Y Z}} {k OTHER} write" |
| generic-set-plain | 0 | "k NEXT" |
| generic-unset-plain | 0 | "" |
| generic-incr-plain | 0 | "k 5" |
| generic-append-plain | 0 | "k 3YZ" |
| generic-lappend-plain | 0 | "k {3 Y Z}" |

Signal/nonzero controls excluded from completed rows: [{"case": "compiled-set-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-unset-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-incr-read-relink", "exit": -11, "rows": 1}, {"case": "compiled-append-read-relink", "exit": -11, "rows": 1}]. The manifest records process outcomes independently of dictionary guest completion.

### jim

Status: `observed`. Version: not recorded (manifest label Jim). Build: binary_sha256=f1af81b53069339ab5e92ee7130804c40a21950bc6a53911479c838907d216b1; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Jim Tcl.

Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| compiled-set-read-change | 1 | "invalid command name \"trace\"" |
| compiled-set-read-relink | 1 | "invalid command name \"trace\"" |
| compiled-set-read-error | 1 | "invalid command name \"trace\"" |
| compiled-set-write-change | 1 | "invalid command name \"trace\"" |
| compiled-set-write-error | 1 | "invalid command name \"trace\"" |
| compiled-unset-read-change | 1 | "invalid command name \"trace\"" |
| compiled-unset-read-relink | 1 | "invalid command name \"trace\"" |
| compiled-unset-read-error | 1 | "invalid command name \"trace\"" |
| compiled-unset-write-change | 1 | "invalid command name \"trace\"" |
| compiled-unset-write-error | 1 | "invalid command name \"trace\"" |
| compiled-incr-read-change | 1 | "invalid command name \"trace\"" |
| compiled-incr-read-relink | 1 | "invalid command name \"trace\"" |
| compiled-incr-read-error | 1 | "invalid command name \"trace\"" |
| compiled-incr-write-change | 1 | "invalid command name \"trace\"" |
| compiled-incr-write-error | 1 | "invalid command name \"trace\"" |
| compiled-append-read-change | 1 | "invalid command name \"trace\"" |
| compiled-append-read-relink | 1 | "invalid command name \"trace\"" |
| compiled-append-read-error | 1 | "invalid command name \"trace\"" |
| compiled-append-write-change | 1 | "invalid command name \"trace\"" |
| compiled-append-write-error | 1 | "invalid command name \"trace\"" |
| compiled-lappend-read-change | 1 | "invalid command name \"trace\"" |
| compiled-lappend-read-relink | 1 | "invalid command name \"trace\"" |
| compiled-lappend-read-error | 1 | "invalid command name \"trace\"" |
| compiled-lappend-write-change | 1 | "invalid command name \"trace\"" |
| compiled-lappend-write-error | 1 | "invalid command name \"trace\"" |
| compiled-set-plain | 0 | "k NEXT" |
| compiled-unset-plain | 0 | "" |
| compiled-incr-plain | 0 | "k 5" |
| compiled-append-plain | 0 | "k 3YZ" |
| compiled-lappend-plain | 0 | "k {3 Y Z}" |
| generic-set-read-change | 1 | "invalid command name \"trace\"" |
| generic-set-read-relink | 1 | "invalid command name \"trace\"" |
| generic-set-read-error | 1 | "invalid command name \"trace\"" |
| generic-set-write-change | 1 | "invalid command name \"trace\"" |
| generic-set-write-error | 1 | "invalid command name \"trace\"" |
| generic-unset-read-change | 1 | "invalid command name \"trace\"" |
| generic-unset-read-relink | 1 | "invalid command name \"trace\"" |
| generic-unset-read-error | 1 | "invalid command name \"trace\"" |
| generic-unset-write-change | 1 | "invalid command name \"trace\"" |
| generic-unset-write-error | 1 | "invalid command name \"trace\"" |
| generic-incr-read-change | 1 | "invalid command name \"trace\"" |
| generic-incr-read-relink | 1 | "invalid command name \"trace\"" |
| generic-incr-read-error | 1 | "invalid command name \"trace\"" |
| generic-incr-write-change | 1 | "invalid command name \"trace\"" |
| generic-incr-write-error | 1 | "invalid command name \"trace\"" |
| generic-append-read-change | 1 | "invalid command name \"trace\"" |
| generic-append-read-relink | 1 | "invalid command name \"trace\"" |
| generic-append-read-error | 1 | "invalid command name \"trace\"" |
| generic-append-write-change | 1 | "invalid command name \"trace\"" |
| generic-append-write-error | 1 | "invalid command name \"trace\"" |
| generic-lappend-read-change | 1 | "invalid command name \"trace\"" |
| generic-lappend-read-relink | 1 | "invalid command name \"trace\"" |
| generic-lappend-read-error | 1 | "invalid command name \"trace\"" |
| generic-lappend-write-change | 1 | "invalid command name \"trace\"" |
| generic-lappend-write-error | 1 | "invalid command name \"trace\"" |
| generic-set-plain | 0 | "k NEXT" |
| generic-unset-plain | 0 | "" |
| generic-incr-plain | 0 | "k 5" |
| generic-append-plain | 0 | "k 3YZ" |
| generic-lappend-plain | 0 | "k {3 Y Z}" |

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-vm/tests/data/native_dictionary_rmw/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/manifest.json). SHA-256 `38d788e8499c90ba4ceab069d790e402e360ea0b80532fd9b15fb76218526b16`. All sixty original inputs, compilation/build hashes and per-case process exit/status/row counts for six builds; 345 completed rows and 15 signal-terminated controls.
- `e1` (input): [rust/tcl-vm/tests/data/native_dictionary_rmw/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/probe.c). SHA-256 `c3a2673dbb3eccbcf9c97335b754ac6092c841c22829d305f31efde93d4bb07e`. Retained exact probe.c; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e2` (input): [rust/tcl-vm/tests/data/native_dictionary_rmw/cases.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/cases.tsv). SHA-256 `87b69f764d9c96b8434caa2f6a39211436c234f3ba80da363c5c69a3afb1ce0b`. Retained exact cases.tsv; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e3` (input): [rust/tcl-vm/tests/data/native_dictionary_rmw/README.md](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/README.md). SHA-256 `0ab33c5df76884c86dbf032377ea04c720b6b83c778b4d2e18954b9f7cb8f574`. Retained exact README.md; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e4` (observation): [rust/tcl-vm/tests/data/native_dictionary_rmw/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/8.4.20.tsv). SHA-256 `4838285423c959168d5f7fe23cbfe043b62e3ba833c8620555ab62df16441e18`. Complete retained semantic TSV for 8.4.20; Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.
- `e5` (observation): [rust/tcl-vm/tests/data/native_dictionary_rmw/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/8.5.19.tsv). SHA-256 `eb63a655f636afe5c802b20cd25116cd373f508adb46c3a6168dc36dcc5f11fe`. Complete retained semantic TSV for 8.5.19; Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.
- `e6` (observation): [rust/tcl-vm/tests/data/native_dictionary_rmw/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/8.6.18.tsv). SHA-256 `a2eaaef2907c0c6bf25ad33214ba098671563e0f4c6fd0d2e524563d90d6858c`. Complete retained semantic TSV for 8.6.18; Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.
- `e7` (observation): [rust/tcl-vm/tests/data/native_dictionary_rmw/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/9.0.4.tsv). SHA-256 `a2eaaef2907c0c6bf25ad33214ba098671563e0f4c6fd0d2e524563d90d6858c`. Complete retained semantic TSV for 9.0.4; Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.
- `e8` (observation): [rust/tcl-vm/tests/data/native_dictionary_rmw/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/9.1.0.tsv). SHA-256 `a2eaaef2907c0c6bf25ad33214ba098671563e0f4c6fd0d2e524563d90d6858c`. Complete retained semantic TSV for 9.1.0; Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.
- `e9` (observation): [rust/tcl-vm/tests/data/native_dictionary_rmw/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_rmw/Jim.tsv). SHA-256 `641508086915f256d9b808aa1a2670380db8f4719c6a0c56e3af906c5bbc7e0d`. Complete retained semantic TSV for Jim; Completed rows record outer code and the inner code/result/d/other/log tuple; reported errors are preserved.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `dictionary_command_and_local_cell_publication_match_345_complete_native_processes`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::dictionary_command_and_local_cell_publication_match_345_complete_native_processes` (linked): Compares exactly the 345 complete native result tuples; the 15 signal processes are not counted as passing controls.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/to/recorded/generic",
  "-I/path/to/recorded/unix",
  "rust/tcl-vm/tests/data/native_dictionary_rmw/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/native-variable-proof"
]
```

Build the exact retained probe with each manifest command, replacing only authentic source/header/archive output paths. Jim independently requires USE_JIM and the manifested libjim build/link flags. Invoke /tmp/native-variable-proof N separately for each original zero-based case ordinal N=0..59 in a fresh process. Compare all completed TSV rows and guest result bytes; independently retain and compare the manifest per-case process statuses, including the fifteen signal terminations. Require empty compilation stderr and preserve original reporter timing. Original raw diagnostic logs are referenced by hashes but not retained as complete files here; the semantic TSVs and per-case statuses delimit this record. No Rust pass is inferred.
