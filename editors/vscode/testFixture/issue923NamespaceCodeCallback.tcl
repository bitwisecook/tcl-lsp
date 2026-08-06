# Issue #923 idx 92 — Tk's fontchooser.tcl idiom: a trace whose handler is
# wrapped in `[namespace code [list Tracer]]`.  Verified against tclsh 8.6.16
# and 9.0.4 (see issue923Callbacks.test.ts for the recorded oracle output).
namespace eval idx92 {
    variable S
    proc Idx92Tracer {args} {
        return $args
    }
    proc Idx92Setup {} {
        variable S
        set S(size) 12
        trace add variable S(size) write [namespace code [list Idx92Tracer]]
    }
    proc Idx92Done {} {
        variable S
        trace remove variable S(size) write [namespace code [list Idx92Tracer]]
    }
}
