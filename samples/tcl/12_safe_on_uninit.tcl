# 12: Commands that safely initialise uninitialised variables
#
# These commands create the variable if it does not exist, so using
# them before an explicit ``set`` is idiomatic Tcl — not a bug.
# The LSP should NOT emit W210 for these patterns.

# --- lappend: creates empty list, then appends ---
proc collect_names {people} {
    foreach person $people {
        lappend names [dict get $person name]
    }
    return $names
}

# --- append: creates empty string, then appends ---
proc build_csv {rows} {
    foreach row $rows {
        append result [join $row ,]\n
    }
    return $result
}

# --- incr: treats uninitialised as 0 (Tcl 8.5+) ---
proc count_words {text} {
    foreach word [split $text] {
        incr counts($word)
    }
    return [array get counts]
}

# --- dict set: creates empty dict, then sets key ---
proc index_by_name {items} {
    foreach item $items {
        dict set index [dict get $item name] $item
    }
    return $index
}

# --- dict lappend: creates empty dict, then lappends ---
proc group_by_type {items} {
    foreach item $items {
        dict lappend groups [dict get $item type] $item
    }
    return $groups
}

# --- dict incr: creates empty dict, then increments ---
proc frequency {words} {
    foreach w $words {
        dict incr freq $w
    }
    return $freq
}

# --- dict append: creates empty dict, then appends ---
proc concat_by_key {pairs} {
    foreach {key val} $pairs {
        dict append merged $key $val
    }
    return $merged
}
