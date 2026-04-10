# resolver.tcl -- Go-style Minimum Version Selection (MVS).
#
# Bit-compatible with tclpkg/resolver.py: same BFS walk, same
# max-of-minimums, same replace/exclude semantics.

namespace eval ::tclpkg::resolver {

    # Resolve dependencies using MVS.
    #
    # direct      -- list of dicts {name minimum}
    # dev_direct  -- list of dicts {name minimum} (dev-only)
    # replaces    -- list of dicts {name version}
    # excludes    -- list of dicts {name version}
    # provider    -- command prefix: {name version} -> list of {name minimum}
    # include_dev -- boolean (default 1)
    #
    # Returns a list of dicts {name version dev requires}.
    proc resolve {direct {dev_direct {}} args} {
        set replaces {}
        set excludes {}
        set provider ""
        set include_dev 1

        foreach {k v} $args {
            switch -- $k {
                -replaces    { set replaces $v }
                -excludes    { set excludes $v }
                -provider    { set provider $v }
                -include_dev { set include_dev $v }
            }
        }

        # Build lookup tables.
        array set replace_map {}
        foreach r $replaces {
            set replace_map([dict get $r name]) $r
        }
        set exclude_set {}
        foreach e $excludes {
            lappend exclude_set [list [dict get $e name] [dict get $e version]]
        }

        set dev_names {}
        foreach r $dev_direct {
            lappend dev_names [dict get $r name]
        }

        # Phase 1: BFS to collect max-of-minimums.
        set pending {}
        array set minimums {}
        array set requires_map {}

        foreach ref $direct {
            lappend pending $ref
        }
        if {$include_dev} {
            foreach ref $dev_direct {
                lappend pending $ref
            }
        }

        set iterations 0
        while {[llength $pending] > 0} {
            incr iterations
            if {$iterations > 10000} {
                error "resolver exceeded maximum iterations (possible cycle)"
            }
            set ref [lindex $pending 0]
            set pending [lrange $pending 1 end]
            set name [dict get $ref name]
            set version [dict get $ref minimum]

            # Apply replace.
            if {[info exists replace_map($name)]} {
                set version [dict get $replace_map($name) version]
            }

            # Check exclude.
            foreach exc $exclude_set {
                lassign $exc ename ever
                if {$ename eq $name && $ever eq $version} {
                    error "package $name $version is excluded by the root manifest"
                }
            }

            # Update minimum if this ref bumps it.
            if {[info exists minimums($name)]} {
                if {[::tclpkg::version::vcompare $version $minimums($name)] <= 0} {
                    continue
                }
            }
            set minimums($name) $version

            # Fetch transitive deps via provider.
            set child_refs {}
            if {$provider ne ""} {
                set child_refs [{*}$provider $name $version]
            }
            set requires_map($name) $child_refs

            foreach child $child_refs {
                lappend pending $child
            }
        }

        # Phase 2: build result list sorted by name.
        set result {}
        foreach name [lsort [array names minimums]] {
            set ver $minimums($name)
            set reqs {}
            if {[info exists requires_map($name)]} {
                set reqs $requires_map($name)
            }
            set is_dev [expr {$name in $dev_names}]
            lappend result [dict create \
                name $name version $ver dev $is_dev requires $reqs]
        }
        return $result
    }
}
