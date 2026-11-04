# Tcl execution evidence

## Question

> The Jim package unknown-member error names forget, names, provide and require. A catalogue rebuilt only from that report could remove a supported hidden list operation, even though an error message is a presentation roster rather than the complete operation inventory.

> Does current Jim package list succeed with the same result as package names even though the exact unknown-member error omits list from its advertised operations?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One fixed ASCII script-file program in a fresh current Jim native process. Actual patchlevel, complete input/executable/stream digests, three caught outcomes and outer process status are recorded. Dynamic enumeration content belongs to this process, not a universal installed-package set.

## Script

```tcl-visible
puts "VERSION|[info patchlevel]"
foreach op {list names __r2286_absent__} {set code [catch {package $op} result]; puts [list RESULT $op $code $result]}
```

## Results

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Fixed ASCII script-file argument in a fresh native shell process
- Exit status: 0
- Standard output:

```text-visible
VERSION|0.84-9-g5bac7c9
RESULT list 0 {file glob clock eventloop signal tclcompat array tree tclprefix history ensemble binary initjimsh jsonencode zlib syslog aio nshelper stdlib json posix namespace oo interp regexp readdir exec pack}
RESULT names 0 {file glob clock eventloop signal tclcompat array tree tclprefix history ensemble binary initjimsh jsonencode zlib syslog aio nshelper stdlib json posix namespace oo interp regexp readdir exec pack}
RESULT __r2286_absent__ 1 {package, unknown command "__r2286_absent__": should be forget, names, provide, require}
```

- Standard error: empty.
