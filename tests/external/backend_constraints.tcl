# Backend constraint overlay for the upstream Tcl 9 tcltest suite.
#
# Source this AFTER `package require tcltest` and BEFORE sourcing a `.test`
# file. It never edits the upstream library or test files (the suite is a
# fixed contract); it only:
#
#   1. reads the backend-introspection keys our runtimes publish in
#      `tcl_platform` (`runtime`, `wasm`, `wasi`, `wasiVersion`, `ebpf` — see
#      `docs/design/runtime/backend-constraints.md`);
#   2. registers tcltest constraints describing the running backend and its
#      spec versions (`wasm`, `notWasm`, `wasi`, `ebpf`, `nativeRuntime`, ...);
#   3. feeds `tcltest::configure -skip` the test-id globs the current backend
#      cannot run, so those tests are *skipped* (matching how C tclsh skips
#      `win` / `unix` / `nonPortable` tests) rather than failing.
#
# A native build skips nothing — every fact is empty, the backend is
# `native`, and the suite runs in full. A wasm / WASI / eBPF build (or a
# native binary asked to evaluate one via the `TCL_*_SPEC` overrides) skips
# the impossible tests.

namespace eval ::tcltest::backend {
    namespace import ::tcltest::testConstraint

    # --- read the introspection facts (absent on C tclsh / older builds) ---
    proc Fact {key} {
        if {[info exists ::tcl_platform($key)]} {
            return $::tcl_platform($key)
        }
        return ""
    }

    variable runtime [Fact runtime]
    variable wasm    [Fact wasm]
    variable wasi    [Fact wasi]
    variable ebpf    [Fact ebpf]

    # --- classify the backend (most restrictive first) ---
    # eBPF is the most constrained target, then WASI, then bare wasm, then a
    # full native interpreter.
    variable backend
    if {$ebpf ne ""} {
        set backend ebpf
    } elseif {$wasi ne ""} {
        set backend wasi
    } elseif {$wasm ne ""} {
        set backend wasm
    } else {
        set backend native
    }

    # --- register the backend / capability constraints --------------------
    # A test guarded with one of these runs only on a matching backend; the
    # `not*` forms are the complement, for tests that must NOT run there.
    testConstraint native       [expr {$backend eq "native"}]
    testConstraint nativeRuntime [expr {$backend eq "native"}]
    testConstraint wasm         [expr {$wasm ne ""}]
    testConstraint notWasm      [expr {$wasm eq ""}]
    testConstraint wasi         [expr {$wasi ne ""}]
    testConstraint notWasi      [expr {$wasi eq ""}]
    testConstraint ebpf         [expr {$ebpf ne ""}]
    testConstraint notEbpf      [expr {$ebpf eq ""}]

    # Runtime-implementation constraints (which Rust interpreter is running).
    testConstraint bytecodeRuntime [expr {$runtime eq "bytecode"}]
    testConstraint treewalkRuntime [expr {$runtime eq "treewalk"}]

    # Spec-version constraints. `VersionAtLeast` does a dotted-decimal compare
    # so `wasi-0.2` is satisfied on a `0.2` build but not a `preview1` one.
    # `preview1` sorts below any numeric version by design.
    proc VersionAtLeast {have want} {
        if {$have eq ""} {return 0}
        # Map named WASI previews onto a comparable ordinal.
        set order {preview1 0.1 preview2 0.2}
        if {[dict exists $order $have]} {set have [dict get $order $have]}
        if {[dict exists $order $want]} {set want [dict get $order $want]}
        if {![string is double -strict $have] || ![string is double -strict $want]} {
            return [string equal $have $want]
        }
        return [expr {$have >= $want}]
    }

    testConstraint wasm2        [VersionAtLeast $wasm 2.0]
    testConstraint wasiPreview1 [expr {$wasi eq "preview1" || $wasi eq "0.1"}]
    testConstraint wasiPreview2 [VersionAtLeast $wasi 0.2]

    # --- exclusion table --------------------------------------------------
    # Each row is {globs {backends...} reason}. A test whose name matches any
    # glob is skipped when the current backend is in {backends...}. Keep this
    # ordered by capability area and cite why; it is meant to grow as backend
    # gaps are characterised. Globs match tcltest test names (e.g. `socket-3.2`).
    variable Exclusions {
        {
            {socket-* socket_*}
            {wasm wasi ebpf}
            {stream sockets are unavailable (WASI preview1 has none; eBPF has no I/O)}
        }
        {
            {exec-* exec_* pid-*}
            {wasm wasi ebpf}
            {no subprocess execution on any sandboxed backend}
        }
        {
            {fileevent-* event-* async-* timer-*}
            {wasm wasi ebpf}
            {no event loop / async fileevents}
        }
        {
            {io-* chan-* chanio-* iocmd-* iogt-* iortrans-*}
            {wasm wasi ebpf}
            {native channel / non-blocking / transform semantics unavailable}
        }
        {
            {thread-* tpool-*}
            {wasm wasi ebpf}
            {no OS threads}
        }
        {
            {load-* unload-*}
            {wasm wasi ebpf}
            {no dynamic library loading}
        }
        {
            {clock-*}
            {ebpf}
            {no wall-clock / timezone database under eBPF}
        }
        {
            {file-* glob-* cmdAH-* fCmd-* filesystem-* fileName-*}
            {ebpf}
            {no filesystem under eBPF}
        }
    }

    # --- apply the skip globs for the current backend ---------------------
    proc Apply {} {
        variable Exclusions
        variable backend
        if {$backend eq "native"} {
            return 0
        }
        set add {}
        foreach row $Exclusions {
            lassign $row globs backends _reason
            if {$backend in $backends} {
                foreach g $globs {lappend add $g}
            }
        }
        if {[llength $add]} {
            set cur [::tcltest::configure -skip]
            ::tcltest::configure -skip [concat $cur $add]
        }
        return [llength $add]
    }

    variable applied [Apply]
}
