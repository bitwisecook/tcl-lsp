# A loaded procedure replaces a catalogued command

## Question

Which commands does a fresh interpreter already hold, and when a script
defines a procedure under a name that is a package's command, a built-in, or
a name imported into `::`, is that procedure the command a bare call reaches?

tcl-lsp declares each workspace document's procedures to the other documents
(#2532). Whether such a declaration may stand in for a catalogued command —
shipped core, a package's (`http`, tcllib's `json` and `md5`), Tk's — depends
on whether the interpreter would run the procedure in its place. The result
covers these names and these interpreters only; it does not show which
workspace files a real program loads, or in what order.

## Assumptions

- Each interpreter ran the file as `tclsh script.tcl` / `jimsh script.tcl` in a
  fresh process, with `TCL_LIBRARY` set to that release's own `library`
  directory for C Tcl. No package was required before the probes.
- The C Tcl interpreters are the unix builds under `tmp/` from
  `make ensure-tcl-deps`; Jim is the revision pinned in
  `rust/tcl-test-support/jim-reference.txt`, built by
  `scripts/dev/ensure-jim-oracle.sh`.
- `info commands NAME` is taken as whether the fresh interpreter binds NAME.
  An auto-loadable command (`parray` on C Tcl) is reported unbound until it is
  first called.
- No BIG-IP execution is recorded for this script.

## Script

```tcl-visible
foreach name {MyDef lpop lassign dict ::http::geturl ::json::json2dict ::md5::md5 button parray} {
    puts "fresh $name [expr {[llength [info commands $name]] ? {bound} : {unbound}}]"
}
puts "package-name [catch {
    namespace eval ::http { proc geturl {url args} { return user } }
    ::http::geturl x
} result] $result"
puts "redefine [catch {
    proc lassign {list args} { return user }
    lassign {a b} x
} result] $result"
puts "import [catch {
    namespace eval ::impl { namespace export MyDef; proc MyDef {name args body} { return user } }
    namespace eval :: { namespace import ::impl::MyDef }
    list [MyDef foo {a} {}] [namespace origin MyDef]
} result] $result"
```

## Results

### Tcl 8.4.20 (`info patchlevel` 8.4.20)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
fresh MyDef unbound
fresh lpop unbound
fresh lassign unbound
fresh dict unbound
fresh ::http::geturl unbound
fresh ::json::json2dict unbound
fresh ::md5::md5 unbound
fresh button unbound
fresh parray unbound
package-name 0 user
redefine 0 user
import 0 user ::impl::MyDef
```

- Standard error: empty.

### Tcl 8.5.19 (`info patchlevel` 8.5.19)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
fresh MyDef unbound
fresh lpop unbound
fresh lassign bound
fresh dict bound
fresh ::http::geturl unbound
fresh ::json::json2dict unbound
fresh ::md5::md5 unbound
fresh button unbound
fresh parray unbound
package-name 0 user
redefine 0 user
import 0 user ::impl::MyDef
```

- Standard error: empty.

### Tcl 8.6.18 (`info patchlevel` 8.6.18)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
fresh MyDef unbound
fresh lpop unbound
fresh lassign bound
fresh dict bound
fresh ::http::geturl unbound
fresh ::json::json2dict unbound
fresh ::md5::md5 unbound
fresh button unbound
fresh parray unbound
package-name 0 user
redefine 0 user
import 0 user ::impl::MyDef
```

- Standard error: empty.

### Tcl 9.0.4 (`info patchlevel` 9.0.4)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
fresh MyDef unbound
fresh lpop bound
fresh lassign bound
fresh dict bound
fresh ::http::geturl unbound
fresh ::json::json2dict unbound
fresh ::md5::md5 unbound
fresh button unbound
fresh parray unbound
package-name 0 user
redefine 0 user
import 0 user ::impl::MyDef
```

- Standard error: empty.

### Tcl 9.1.0 (`info patchlevel` 9.1.0)

- Invocation: `tclsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
fresh MyDef unbound
fresh lpop bound
fresh lassign bound
fresh dict bound
fresh ::http::geturl unbound
fresh ::json::json2dict unbound
fresh ::md5::md5 unbound
fresh button unbound
fresh parray unbound
package-name 0 user
redefine 0 user
import 0 user ::impl::MyDef
```

- Standard error: empty.

### Jim (`info patchlevel` 0.84-10-g82fb4d0)

- Invocation: `jimsh script.tcl`
- Exit status: `0`
- Standard output:

```text-visible
fresh MyDef unbound
fresh lpop unbound
fresh lassign bound
fresh dict bound
fresh ::http::geturl unbound
fresh ::json::json2dict unbound
fresh ::md5::md5 unbound
fresh button unbound
fresh parray bound
package-name 0 user
redefine 0 user
import 0 user ::impl::MyDef
```

- Standard error: empty.
