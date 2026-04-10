# lockfile.tcl — canonical JSON lockfile read/write.
#
# Uses tcllib json for parsing and a hand-rolled canonical writer for
# deterministic output (sorted keys, 2-space indent, LF endings).

namespace eval ::tclpkg::lockfile {

    # Serialise a lockfile dict to canonical JSON.
    proc serialise {lf} {
        set lines {}
        lappend lines "\{"
        # Top-level keys in sorted order.
        set keys [lsort [dict keys $lf]]
        set last_key [lindex $keys end]
        foreach key $keys {
            set val [dict get $lf $key]
            set comma ","
            if {$key eq $last_key} { set comma "" }
            if {$key eq "packages" || $key eq "replaces" || $key eq "excludes"} {
                lappend lines "  \"$key\": [_encode_list_of_dicts $val]$comma"
            } elseif {$key eq "version"} {
                lappend lines "  \"$key\": $val$comma"
            } else {
                lappend lines "  \"$key\": [_json_string $val]$comma"
            }
        }
        lappend lines "\}"
        return [join $lines "\n"]\n
    }

    proc _json_string {s} {
        if {$s eq "null"} { return null }
        set escaped [string map {\\ \\\\ \" \\\" \n \\n \r \\r \t \\t} $s]
        return "\"$escaped\""
    }

    proc _encode_list_of_dicts {lst} {
        if {[llength $lst] == 0} {
            return "\[\]"
        }
        set items {}
        foreach item $lst {
            lappend items [_encode_dict $item 4]
        }
        return "\[\n[join $items ",\n"]\n  \]"
    }

    proc _encode_dict {d indent_level} {
        set indent [string repeat " " $indent_level]
        set inner_indent [string repeat " " [expr {$indent_level + 2}]]
        set keys [lsort [dict keys $d]]
        set parts {}
        foreach key $keys {
            set val [dict get $d $key]
            if {[string is integer -strict $val]} {
                lappend parts "${inner_indent}\"$key\": $val"
            } elseif {$val eq "true" || $val eq "false" || $val eq "null"} {
                lappend parts "${inner_indent}\"$key\": $val"
            } elseif {[llength $val] > 1 || ($val ne "" && [lindex $val 0] ne $val)} {
                # Looks like a list or nested dict.
                if {[_is_dict $val]} {
                    lappend parts "${inner_indent}\"$key\": [_encode_dict $val [expr {$indent_level + 2}]]"
                } else {
                    lappend parts "${inner_indent}\"$key\": [_encode_string_list $val]"
                }
            } else {
                lappend parts "${inner_indent}\"$key\": [_json_string $val]"
            }
        }
        return "${indent}\{\n[join $parts ",\n"]\n${indent}\}"
    }

    proc _encode_string_list {lst} {
        if {[llength $lst] == 0} { return "\[\]" }
        set items {}
        foreach item $lst {
            lappend items [_json_string $item]
        }
        return "\[[join $items ", "]\]"
    }

    proc _is_dict {val} {
        if {[llength $val] < 2 || [llength $val] % 2 != 0} { return 0 }
        return [expr {![catch {dict size $val}]}]
    }

    # Deserialise a lockfile JSON string into a dict.
    proc deserialise {text} {
        if {[catch {package require json}]} {
            error "tcllib json package required for lockfile parsing"
        }
        set data [::json::json2dict $text]
        if {![dict exists $data version]} {
            dict set data version 1
        }
        set schema_ver [dict get $data version]
        if {$schema_ver > 1} {
            error "lockfile schema version $schema_ver is newer than supported (1); upgrade tclpkg"
        }
        return $data
    }

    # Write a lockfile to disk atomically.
    proc write {lf path} {
        set text [serialise $lf]
        set tmp "${path}.tmp"
        set fd [open $tmp w]
        fconfigure $fd -encoding utf-8 -translation lf
        puts -nonewline $fd $text
        close $fd
        file rename -force $tmp $path
    }

    # Read a lockfile from disk.
    proc read_ {path} {
        if {![file isfile $path]} {
            error "lockfile not found: $path (run 'tclpkg pkg install' to create one)"
        }
        set fd [open $path r]
        fconfigure $fd -encoding utf-8
        set text [read $fd]
        close $fd
        return [deserialise $text]
    }

    # Create an empty lockfile dict.
    proc new_lockfile {name {tcl_constraint ">=8.6"}} {
        return [dict create \
            version 1 \
            name $name \
            tcl $tcl_constraint \
            generated [clock format [clock seconds] -format "%Y-%m-%dT%H:%M:%SZ" -gmt 1] \
            generator "tclpkg 0.1.0" \
            root_hash "" \
            packages {} \
            replaces {} \
            excludes {}]
    }

    # Look up a package by name in a lockfile.
    proc lookup {lf name} {
        foreach pkg [dict get $lf packages] {
            if {[dict get $pkg name] eq $name} {
                return $pkg
            }
        }
        return ""
    }
}
