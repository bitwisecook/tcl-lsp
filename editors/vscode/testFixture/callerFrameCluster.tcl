# Caller-frame injection cluster (issue #923 audit idx 7 / 22 / 57 / 58 / 98,
# issue #1019).  Every claim below is pinned on tclsh 9.0.4 and 8.6.16, which
# agree on all of it.
#
# idx 58 -- an out-parameter helper whose caller-frame variable happens to
# share its name with an unrelated TclOO method.  `chart new` prints
# `MY-SHARED-DATASET`; a `$`-led read can never denote a method.
proc gridlayoutHasDataSetObj {dts} {
    upvar 1 $dts dataset
    set dataset MY-SHARED-DATASET
}
oo::class create chart {
    constructor {} {
        gridlayoutHasDataSetObj dataset
        set _dataset $dataset
        puts $_dataset
    }
    method dataset {} { return 1 }
}

# idx 98 -- `upvar ::tk::FocusGrab($index) data` names one fixed global cell,
# so the sibling proc's own qualified accesses are the same variable.
proc SetFocusGrab {grab focus} {
    set index "$grab,$focus"
    upvar ::tk::FocusGrab($index) data
    lappend data one
}
proc RestoreFocusGrab {grab focus} {
    set index "$grab,$focus"
    if {[info exists ::tk::FocusGrab($index)]} {
        set data2 $::tk::FocusGrab($index)
        unset ::tk::FocusGrab($index)
    }
}

# idx 22 -- the callee is a *mixin* method reached by `my NameProcess ...`.
# `Widget new {-base 1}` prints `name=::oo::Obj24 params=-base 1`.
oo::class create Utility {
    method NameProcess {arguments object} {
        upvar name name
        set name $object
    }
}
oo::class create Widget {
    mixin Utility
    constructor {arguments} {
        my NameProcess $arguments [self object]
        set params $arguments
        puts "name=$name params=$params"
    }
}
