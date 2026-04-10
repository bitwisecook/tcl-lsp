# version.tcl — semver parsing and ordering with Tcl-friendly extensions.
#
# Bit-compatible with tclpkg/version.py: same parse rules, same ordering.
# Supports: v-prefix, missing patch (1.2 → 1.2.0), Tcl-style a1/b2/rc1.

namespace eval ::tclpkg::version {
    # Parse a version string into a dict {major minor patch pre pre_raw build original}.
    # Raises error on invalid input.
    proc parse {raw} {
        set stripped [string trim $raw]
        if {$stripped eq ""} {
            error "empty version string"
        }
        # Strip optional v prefix.
        if {[string index $stripped 0] eq "v"} {
            set stripped [string range $stripped 1 end]
        }
        # Try full semver: MAJOR.MINOR.PATCH[-pre][+build]
        set re {^(0|[1-9][0-9]*)(?:\.([0-9]+))?(?:\.([0-9]+))?((?:-[0-9A-Za-z.-]+)|(?:[ab][0-9]+)|(?:rc[0-9]+))?(?:\+([0-9A-Za-z.-]+))?$}
        if {![regexp $re $stripped -> major minor patch pre build]} {
            error "invalid version: '$raw'"
        }
        if {$minor eq ""} { set minor 0 }
        if {$patch eq ""} { set patch 0 }
        set pre_raw ""
        set pre_key {}
        if {$pre ne ""} {
            set pre_raw $pre
            set pre_key [_parse_prerelease $pre]
        }
        return [dict create \
            major $major minor $minor patch $patch \
            pre $pre_key pre_raw $pre_raw build $build \
            original [string trim $raw]]
    }

    # Internal: parse a prerelease tag into a comparable list.
    proc _parse_prerelease {raw} {
        # Tcl-style short forms: a1 → {0 0 0 1}, b2 → {0 1 0 2}, rc1 → {0 2 0 1}
        if {[regexp {^(a)([0-9]+)$} $raw -> label num]} {
            return [list 0 0 0 $num]
        }
        if {[regexp {^(b)([0-9]+)$} $raw -> label num]} {
            return [list 0 1 0 $num]
        }
        if {[regexp {^(rc)([0-9]+)$} $raw -> label num]} {
            return [list 0 2 0 $num]
        }
        # Semver-style: -alpha.1, -rc.4, etc.
        set tag [string trimleft $raw "-"]
        set pieces [split $tag "."]
        set result {}
        foreach piece $pieces {
            if {[string is integer -strict $piece]} {
                lappend result 0 $piece
            } else {
                # Map known labels to numeric order.
                switch -- $piece {
                    alpha   { lappend result 0 0 }
                    beta    { lappend result 0 1 }
                    rc - pre { lappend result 0 2 }
                    default { lappend result 1 $piece }
                }
            }
        }
        return $result
    }

    # Canonical form: MAJOR.MINOR.PATCH[-pre][+build]
    proc canonical {ver} {
        set s "[dict get $ver major].[dict get $ver minor].[dict get $ver patch]"
        set pre_raw [dict get $ver pre_raw]
        if {$pre_raw ne ""} {
            # Normalise Tcl short forms to semver style.
            if {[regexp {^(a)([0-9]+)$} $pre_raw -> _ num]} {
                append s "-alpha.$num"
            } elseif {[regexp {^(b)([0-9]+)$} $pre_raw -> _ num]} {
                append s "-beta.$num"
            } elseif {[regexp {^(rc)([0-9]+)$} $pre_raw -> _ num]} {
                append s "-rc.$num"
            } else {
                append s $pre_raw
            }
        }
        set build [dict get $ver build]
        if {$build ne ""} {
            append s "+$build"
        }
        return $s
    }

    # Compare two parsed versions. Returns -1, 0, or 1.
    proc compare {a b} {
        foreach field {major minor patch} {
            set va [dict get $a $field]
            set vb [dict get $b $field]
            if {$va < $vb} { return -1 }
            if {$va > $vb} { return 1 }
        }
        # Prerelease: empty (no pre) > any prerelease.
        set pa [dict get $a pre]
        set pb [dict get $b pre]
        if {$pa eq "" && $pb eq ""} { return 0 }
        if {$pa eq ""} { return 1 }
        if {$pb eq ""} { return -1 }
        # Compare prerelease lists element-wise.
        set len [expr {max([llength $pa], [llength $pb])}]
        for {set i 0} {$i < $len} {incr i} {
            set ea [lindex $pa $i]
            set eb [lindex $pb $i]
            if {$ea eq "" && $eb ne ""} { return -1 }
            if {$ea ne "" && $eb eq ""} { return 1 }
            if {[string is integer -strict $ea] && [string is integer -strict $eb]} {
                if {$ea < $eb} { return -1 }
                if {$ea > $eb} { return 1 }
            } else {
                set cmp [string compare $ea $eb]
                if {$cmp != 0} { return $cmp }
            }
        }
        return 0
    }

    # Convenience: parse and return canonical string.
    proc normalise {raw} {
        return [canonical [parse $raw]]
    }

    # Compare two raw version strings. Returns -1, 0, or 1.
    proc vcompare {a b} {
        return [compare [parse $a] [parse $b]]
    }

    # Return the maximum version from a list of raw strings.
    proc vmax {versions} {
        if {[llength $versions] == 0} {
            error "vmax requires a non-empty list"
        }
        set best [lindex $versions 0]
        foreach v [lrange $versions 1 end] {
            if {[vcompare $v $best] > 0} {
                set best $v
            }
        }
        return $best
    }
}
