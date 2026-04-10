# Tcl Code Importing Patterns — External References

Reference links for all known Tcl code importing/loading patterns.
Used to inform cross-file static analysis scope (issue #40).

## `package require` — Standard Package Loading

- [Tcl `package` manual (8.6)](https://www.tcl-lang.org/man/tcl8.6/TclCmd/package.htm)
- [Tcl `package` manual (9.0)](https://www.tcl-lang.org/man/tcl9.0/TclCmd/package.html) — adds `package files`
- [TIP 268: Version ranges](https://core.tcl-lang.org/tips/doc/trunk/tip/268.md) — `package require Tcl 8.5-`
- [Tcl Wiki: package require](https://wiki.tcl-lang.org/page/package+require) — includes dynamic/variable examples
- [tcllib ldap.tcl](https://github.com/tcltk/tcllib/blob/master/modules/ldap/ldap.tcl) — real-world version-pinned requires
- [tcllib rest.tcl](https://github.com/tcltk/tcllib/blob/master/modules/rest/rest.tcl) — multi-package require block
- [jimtcl tcltest.tcl](https://github.com/msteveb/jimtcl/blob/master/tcltest.tcl) — conditional `catch {package require $what}`

## `source` — Direct File Inclusion

- [Tcl `source` manual (8.6)](https://www.tcl-lang.org/man/tcl8.6/TclCmd/source.htm)
- [TIP 587: source encoding default](https://core.tcl-lang.org/tips/doc/trunk/tip/587.md) — Tcl 9 defaults to UTF-8
- [Tcl Wiki: source](https://wiki.tcl-lang.org/page/source) — idioms including glob-based bulk sourcing
- [tklib tablelist dirViewer.tcl](https://github.com/tcltk/tklib/blob/master/examples/tablelist/dirViewer.tcl) — `source [file join [file dirname [info script]] ...]`
- [tsp tsp.tcl](https://github.com/tpoindex/tsp/blob/master/tsp.tcl) — `set dir [file dirname [info script]]; source [file join $dir ...]`
- [tcllib irc_example.tcl](https://github.com/tcltk/tcllib/blob/master/examples/irc/irc_example.tcl) — `$::scriptDir` pattern

## `load` — Shared Library / DLL Loading

- [Tcl `load` manual (8.7)](https://www.tcl-lang.org/man/tcl8.7/TclCmd/load.html) — includes `-global`, `-lazy` flags
- [Tcl Wiki: pkgIndex.tcl](https://wiki.tcl-lang.org/page/pkgIndex.tcl) — `load {}` for statically linked packages
- [funtools pkgIndex.tcl](https://github.com/ericmandel/funtools/blob/master/pkgIndex.tcl) — `load [file join $dir libtclfun.so]`
- [CCP4 autoloading guide](https://legacy.ccp4.ac.uk/peter/programming/tcl_autoloading.html) — `info sharedlibextension` pattern

## `package ifneeded` / `pkgIndex.tcl`

- [Tcl `pkg_mkIndex` manual](https://www.tcl-lang.org/man/tcl8.7/TclCmd/pkgMkIndex.html) — auto-generates pkgIndex.tcl
- [Tcl Wiki: pkgIndex.tcl](https://wiki.tcl-lang.org/page/pkgIndex.tcl) — comprehensive patterns including `apply` wrapper, encoding, version guards
- [nat-418 Tcl package tutorial (gist)](https://gist.github.com/nat-418/0155a9f8093385de39e83d41e61606d5) — simple `package ifneeded` + `source`
- [TclTLS pkgIndex.tcl.in](https://github.com/eelcohn/TclTLS1.8.0/blob/master/pkgIndex.tcl.in) — conditional static vs dynamic load

## `auto_load` / `tclIndex`

- [Tcl `library` manual](https://www.tcl-lang.org/man/tcl8.7/TclCmd/library.html) — auto_load, auto_mkindex, auto_path
- [Tcl Wiki: auto_path](https://wiki.tcl-lang.org/page/auto_path)
- [Tcl Wiki: auto_mkindex](https://wiki.tcl-lang.org/page/auto_mkindex)
- [MIT Tcl library tclIndex](http://web.mit.edu/Tcl/lib/tcl8.1/tclIndex) — example tclIndex format

## `namespace import` / `namespace export` / `namespace ensemble`

- [Tcl `namespace` manual](https://www.tcl-lang.org/man/tcl8.6/TclCmd/namespace.htm)
- [tcllib ldap.tcl](https://github.com/tcltk/tcllib/blob/master/modules/ldap/ldap.tcl) — `namespace import ::asn::*` paired with export
- [tcllib websocket.tcl](https://github.com/tcltk/tcllib/blob/master/modules/websocket/websocket.tcl) — `namespace ensemble create`
- [tcllib json_write.tcl](https://github.com/tcltk/tcllib/blob/master/modules/json/json_write.tcl) — selective export + ensemble
- [tcl namespace.test](https://github.com/tcltk/tcl/blob/master/tests/namespace.test) — `namespace import -force`

## Tcl Modules (`.tm` files)

- [Tcl `tm` manual](https://www.tcl-lang.org/man/tcl/TclCmd/tm.htm) — `::tcl::tm::path`, naming convention
- [Tcl Wiki: tm](https://wiki.tcl-lang.org/page/tm)
- [Tcl Wiki: tcl::tm::path](https://wiki.tcl-lang.org/page/tcl::tm::path) — environment variables `TCL8_6_TM_PATH` etc.
- [nat-418 Tcl package tutorial (gist)](https://gist.github.com/nat-418/0155a9f8093385de39e83d41e61606d5) — `.tm` vs `pkgIndex.tcl` comparison

## `interp alias`

- [Tcl `interp` manual](https://www.tcl-lang.org/man/tcl8.6/TclCmd/interp.htm)
- [SpiceGenTcl](https://github.com/georgtree/SpiceGenTcl) — `interp alias {} dget {} dict get` shortcut aliases
- [tcllib ipMore.tcl](https://github.com/tcltk/tcllib/blob/master/modules/dns/ipMore.tcl) — compatibility aliases, C-accelerated fallbacks
- [thread ttrace.tcl](https://github.com/tcltk/thread/blob/master/lib/ttrace.tcl) — abstraction layer aliases (nsv_* backend)
- [TACC/Lmod modulecmd.tcl](https://github.com/TACC/Lmod/blob/main/tcl/modulecmd.tcl) — loop-based alias creation

## `rename` — Command Wrapping / Interception

- [thread ttrace.tcl](https://github.com/tcltk/thread/blob/master/lib/ttrace.tcl) — `rename ::unknown ::tcl::unknown` + replacement
- [tcllib comm.tcl](https://github.com/tcltk/tcllib/blob/master/modules/comm/comm.tcl) — vwait wrapping
- [tclreadline tclreadlineSetup.tcl.in](https://github.com/flightaware/tclreadline/blob/master/tclreadlineSetup.tcl.in) — `rename unknown _unknown` for completion

## `apply` — Anonymous Procedures

- [TIP 187: apply](https://www.tcl-lang.org/cgi-bin/tct/tip/187) — lambda-like anonymous procs
- [Tcl Wiki: Lambda in Tcl](https://wiki.tcl-lang.org/page/Lambda+in+Tcl)
- [tcllib lambda.tcl](https://github.com/tcltk/tcllib/blob/master/modules/lambda/lambda.tcl) — `lambda` helper proc

## Tcl 9.0 Import Changes

- [Tcl 9 migration guide](https://core.tcl-lang.org/tcl/wiki?name=Migrating+scripts+to+Tcl+9)
- [Tcl Wiki: Changes in Tcl/Tk 9.0](https://wiki.tcl-lang.org/page/Changes+in+Tcl/Tk+9.0)
- `source` defaults to UTF-8 encoding (TIP 587)
- Binary library naming: `tcl9` prefix (`libtcl9tk8.7.so`)
- New `package files` subcommand — lists files loaded during package init
- `load` init function is now case-sensitive
- `~` no longer interpreted as home directory in pathnames
- Namespace variable resolution no longer falls back to global scope
- [zipfs manual](https://www.tcl-lang.org/man/tcl8.7/TclCmd/zipfs.html) — virtual filesystem for bundled scripts
