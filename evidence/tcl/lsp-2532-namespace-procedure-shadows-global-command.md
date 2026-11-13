# A namespace procedure shadows a global command

## Question

When both `::MyDef` and `::app::MyDef` are procedures, which one does a bare
`MyDef` call reach from `::app`, from `::`, and from an unrelated namespace?

tcl-lsp lets a user spec pack describe the global `MyDef` while a workspace
procedure `::app::MyDef` is declared under its qualified name (#2532). The
pack must answer only where the interpreter would reach the global command.
The result covers command lookup for procedures in these namespaces only, not
`namespace path`, `namespace unknown` or imported commands.

## Assumptions

- Each interpreter ran the file as `tclsh script.tcl` / `jimsh script.tcl` in a
  fresh process, with `TCL_LIBRARY` set to that release's own `library`
  directory for C Tcl.
- The C Tcl interpreters are the unix builds under `tmp/` from
  `make ensure-tcl-deps`; Jim is the revision pinned in
  `rust/tcl-test-support/jim-reference.txt`, built by
  `scripts/dev/ensure-jim-oracle.sh`.
- No BIG-IP execution is recorded for this script.

## Script

```tcl-visible
proc MyDef {name params body} { return global }
namespace eval ::app { proc MyDef {name} { return app } }
puts "from-app [catch {namespace eval ::app { MyDef x }} result] $result"
puts "from-global [catch {MyDef x y z} result] $result"
puts "from-other [catch {namespace eval ::other { MyDef x y z }} result] $result"
```

## Results

### Tcl 8.4.20 (`info patchlevel` 8.4.20)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
from-app 0 app
from-global 0 global
from-other 0 global
```

- Standard error: empty.

### Tcl 8.5.19 (`info patchlevel` 8.5.19)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
from-app 0 app
from-global 0 global
from-other 0 global
```

- Standard error: empty.

### Tcl 8.6.18 (`info patchlevel` 8.6.18)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
from-app 0 app
from-global 0 global
from-other 0 global
```

- Standard error: empty.

### Tcl 9.0.4 (`info patchlevel` 9.0.4)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
from-app 0 app
from-global 0 global
from-other 0 global
```

- Standard error: empty.

### Tcl 9.1.0 (`info patchlevel` 9.1.0)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
from-app 0 app
from-global 0 global
from-other 0 global
```

- Standard error: empty.

### Jim (`info patchlevel` 0.84-10-g82fb4d0)

- Invocation: `jimsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
from-app 0 app
from-global 0 global
from-other 0 global
```

- Standard error: empty.
