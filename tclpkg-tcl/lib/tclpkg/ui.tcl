# ui.tcl -- ANSI colour helpers, progress, and JSON output.

namespace eval ::tclpkg::ui {
    variable use_colour 1

    proc init {} {
        variable use_colour
        if {[info exists ::env(NO_COLOR)] && $::env(NO_COLOR) ne ""} {
            set use_colour 0
        } elseif {[catch {chan configure stdout -translation}]} {
            set use_colour 0
        }
    }

    proc _c {code text} {
        variable use_colour
        if {!$use_colour} { return $text }
        return "\033\[${code}m${text}\033\[0m"
    }

    proc ok {text}   { return [_c 32 "  \u2713 $text"] }
    proc fail {text}  { return [_c 31 "  \u2717 $text"] }
    proc warn {text}  { return [_c 33 "  \u26a0 $text"] }
    proc info_ {text} { return [_c 36 $text] }
    proc dim {text}   { return [_c 2 $text] }
    proc bold {text}  { return [_c 1 $text] }

    proc progress {msg} {
        puts -nonewline stderr "\r  $msg\033\[K"
        flush stderr
    }

    proc progress_done {} {
        puts -nonewline stderr "\r\033\[K"
        flush stderr
    }

    # Minimal JSON encoder for simple dicts and lists.
    proc json_encode {value} {
        if {[string is integer -strict $value]} {
            return $value
        }
        if {$value eq "true" || $value eq "false" || $value eq "null"} {
            return $value
        }
        # Check if it looks like a dict (even number of elements, at least 2).
        if {[llength $value] >= 2 && [llength $value] % 2 == 0} {
            # Heuristic: if every other element looks like a key, treat as dict.
            set is_dict 1
            set idx 0
            foreach {k v} $value {
                if {![string is wordchar -strict $k] && ![string match "*_*" $k]} {
                    set is_dict 0
                    break
                }
            }
            if {$is_dict} {
                return [json_encode_dict $value]
            }
        }
        if {[llength $value] > 1 || ([llength $value] == 1 && [lindex $value 0] ne $value)} {
            return [json_encode_list $value]
        }
        # String.
        set escaped [string map {
            \\ \\\\ \" \\\" \n \\n \r \\r \t \\t
        } $value]
        return "\"$escaped\""
    }

    proc json_encode_dict {d} {
        set parts {}
        foreach key [lsort [dict keys $d]] {
            set v [dict get $d $key]
            lappend parts "\"$key\": [json_encode $v]"
        }
        return "\{[join $parts ", "]\}"
    }

    proc json_encode_list {lst} {
        set parts {}
        foreach item $lst {
            lappend parts [json_encode $item]
        }
        return "\[[join $parts ", "]\]"
    }

    proc json_output {data} {
        # For real JSON output we use tcllib json::write if available,
        # otherwise fall back to our minimal encoder.
        if {![catch {package require json::write} _]} {
            puts [::json::write object {*}[_dict_to_json_write $data]]
        } else {
            puts [json_encode $data]
        }
    }

    proc _dict_to_json_write {d} {
        set result {}
        foreach {k v} $d {
            if {[string is integer -strict $v]} {
                lappend result $k $v
            } elseif {$v eq "true" || $v eq "false"} {
                lappend result $k $v
            } elseif {[llength $v] > 1} {
                lappend result $k [::json::write array {*}[lmap item $v {::json::write string $item}]]
            } else {
                lappend result $k [::json::write string $v]
            }
        }
        return $result
    }

    init
}
