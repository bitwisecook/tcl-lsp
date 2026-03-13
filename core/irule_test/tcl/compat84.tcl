# compat84.tcl -- Tcl 8.4 compatibility shim
#
# Provides implementations of Tcl 8.5+ features when running on 8.4.
# These are available to the framework internals (::state::, ::itest::,
# ::tmm::) but NOT to the iRule under test (which sees pure 8.4).
#
# When running on 8.5+, these are no-ops -- the native commands are
# used directly (and later hidden from the iRule by tmm_shim.tcl).
#
# Provides:
#   dict          - Full dict implementation for 8.4
#   lassign       - List assignment
#   lrepeat       - List repetition
#   lreverse      - List reversal
#   {*} expansion - NOT possible to polyfill; use [eval] instead
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

# Only install if we're actually on 8.4
if {[info tclversion] ne "8.4" && [package vcompare [info patchlevel] 8.5.0] >= 0} {
    # Running on 8.5+: nothing to do, native commands exist.
    # We'll save references for framework-internal use before
    # tmm_shim hides them.
    return
}

# ── dict command (Tcl 8.4 polyfill) ──────────────────────────────
#
# Implements dict using arrays internally.  This is the framework's
# internal dict, not exposed to the iRule.

if {[llength [info commands ::dict]] == 0} {

    proc ::dict {subcommand args} {
        switch -exact -- $subcommand {
            create {
                return $args
            }
            get {
                set d [lindex $args 0]
                set keys [lrange $args 1 end]
                foreach key $keys {
                    if {[llength $d] % 2 != 0} {
                        error "missing value to go with key"
                    }
                    set found 0
                    foreach {k v} $d {
                        if {$k eq $key} {
                            set d $v
                            set found 1
                            break
                        }
                    }
                    if {!$found} {
                        error "key \"$key\" not known in dictionary"
                    }
                }
                return $d
            }
            set {
                set dvar [lindex $args 0]
                upvar 1 $dvar d
                if {![info exists d]} { set d [list] }
                set key [lindex $args 1]
                set val [lindex $args 2]
                set new [list]
                set found 0
                foreach {k v} $d {
                    if {$k eq $key} {
                        lappend new $k $val
                        set found 1
                    } else {
                        lappend new $k $v
                    }
                }
                if {!$found} {
                    lappend new $key $val
                }
                set d $new
                return $new
            }
            unset {
                set dvar [lindex $args 0]
                upvar 1 $dvar d
                if {![info exists d]} { return }
                set key [lindex $args 1]
                set new [list]
                foreach {k v} $d {
                    if {$k ne $key} {
                        lappend new $k $v
                    }
                }
                set d $new
            }
            exists {
                set d [lindex $args 0]
                set keys [lrange $args 1 end]
                foreach key $keys {
                    if {[llength $d] % 2 != 0} { return 0 }
                    set found 0
                    foreach {k v} $d {
                        if {$k eq $key} {
                            set d $v
                            set found 1
                            break
                        }
                    }
                    if {!$found} { return 0 }
                }
                return 1
            }
            keys {
                set d [lindex $args 0]
                set pattern "*"
                if {[llength $args] > 1} {
                    set pattern [lindex $args 1]
                }
                set result [list]
                foreach {k v} $d {
                    if {[string match $pattern $k]} {
                        lappend result $k
                    }
                }
                return $result
            }
            values {
                set d [lindex $args 0]
                set pattern "*"
                if {[llength $args] > 1} {
                    set pattern [lindex $args 1]
                }
                set result [list]
                foreach {k v} $d {
                    if {[string match $pattern $v]} {
                        lappend result $v
                    }
                }
                return $result
            }
            size {
                set d [lindex $args 0]
                return [expr {[llength $d] / 2}]
            }
            for {
                set vars [lindex $args 0]
                set d [lindex $args 1]
                set body [lindex $args 2]
                set kvar [lindex $vars 0]
                set vvar [lindex $vars 1]
                upvar 1 $kvar k_up $vvar v_up
                foreach {k_up v_up} $d {
                    set code [catch {uplevel 1 $body} result]
                    switch -exact -- $code {
                        0 {}
                        1 { return -code error $result }
                        2 { return -code return $result }
                        3 { break }
                        4 { continue }
                    }
                }
            }
            append {
                set dvar [lindex $args 0]
                upvar 1 $dvar d
                if {![info exists d]} { set d [list] }
                set key [lindex $args 1]
                set str [lindex $args 2]
                set new [list]
                set found 0
                foreach {k v} $d {
                    if {$k eq $key} {
                        lappend new $k "${v}${str}"
                        set found 1
                    } else {
                        lappend new $k $v
                    }
                }
                if {!$found} {
                    lappend new $key $str
                }
                set d $new
            }
            incr {
                set dvar [lindex $args 0]
                upvar 1 $dvar d
                if {![info exists d]} { set d [list] }
                set key [lindex $args 1]
                set amount 1
                if {[llength $args] > 2} { set amount [lindex $args 2] }
                set new [list]
                set found 0
                foreach {k v} $d {
                    if {$k eq $key} {
                        set newval [expr {$v + $amount}]
                        lappend new $k $newval
                        set found 1
                    } else {
                        lappend new $k $v
                    }
                }
                if {!$found} {
                    lappend new $key $amount
                }
                set d $new
                return [dict get $d $key]
            }
            merge {
                set result [list]
                foreach d $args {
                    foreach {k v} $d {
                        # Later values override earlier ones
                        set new [list]
                        set found 0
                        foreach {ek ev} $result {
                            if {$ek eq $k} {
                                lappend new $ek $v
                                set found 1
                            } else {
                                lappend new $ek $ev
                            }
                        }
                        if {!$found} {
                            lappend new $k $v
                        }
                        set result $new
                    }
                }
                return $result
            }
            replace {
                set d [lindex $args 0]
                set pairs [lrange $args 1 end]
                foreach {k v} $pairs {
                    set new [list]
                    set found 0
                    foreach {ek ev} $d {
                        if {$ek eq $k} {
                            lappend new $ek $v
                            set found 1
                        } else {
                            lappend new $ek $ev
                        }
                    }
                    if {!$found} {
                        lappend new $k $v
                    }
                    set d $new
                }
                return $d
            }
            remove {
                set d [lindex $args 0]
                set keys [lrange $args 1 end]
                set result [list]
                foreach {k v} $d {
                    if {[lsearch -exact $keys $k] < 0} {
                        lappend result $k $v
                    }
                }
                return $result
            }
            info {
                set d [lindex $args 0]
                set subcmd2 [lindex $args 1]
                if {$subcmd2 eq "exists"} {
                    set key [lindex $args 2]
                    return [dict exists $d $key]
                }
            }
            default {
                error "unknown or ambiguous subcommand \"$subcommand\": must be append, create, exists, for, get, incr, info, keys, merge, remove, replace, set, size, unset, or values"
            }
        }
    }
}

# ── lassign (Tcl 8.4 polyfill) ───────────────────────────────────

if {[llength [info commands ::lassign]] == 0} {
    proc ::lassign {list args} {
        set i 0
        foreach var $args {
            upvar 1 $var v
            set v [lindex $list $i]
            incr i
        }
        return [lrange $list $i end]
    }
}

# ── lrepeat (Tcl 8.4 polyfill) ───────────────────────────────────

if {[llength [info commands ::lrepeat]] == 0} {
    proc ::lrepeat {count args} {
        set result [list]
        for {set i 0} {$i < $count} {incr i} {
            foreach val $args {
                lappend result $val
            }
        }
        return $result
    }
}

# ── lreverse (Tcl 8.4 polyfill) ──────────────────────────────────

if {[llength [info commands ::lreverse]] == 0} {
    proc ::lreverse {list} {
        set result [list]
        set len [llength $list]
        for {set i [expr {$len - 1}]} {$i >= 0} {incr i -1} {
            lappend result [lindex $list $i]
        }
        return $result
    }
}

# ── ni / in operators ─────────────────────────────────────────────
#
# 8.4 lacks "in" and "ni" operators in [expr].
# These are provided as math functions for framework use.
# (The TMM expression operators in expr_ops.tcl are separate.)

if {[catch {expr {"a" in {a b c}}}]} {
    # Running on 8.4 where "in" doesn't exist in expr
    # These are for framework internal use only
    namespace eval ::tcl::mathfunc {
        proc in {val list} {
            return [expr {[lsearch -exact $list $val] >= 0}]
        }
        proc ni {val list} {
            return [expr {[lsearch -exact $list $val] < 0}]
        }
    }
}
