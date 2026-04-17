# Proc argument trait examples
#
# These demonstrate the six types of proc argument traits that
# tcl-lsp can infer automatically from proc body analysis.

# EVAL trait: argument is eval'd as a script
# The proc takes a script and evaluates it via eval/uplevel.
proc run_script {script} {
    eval $script
}

proc run_in_caller {level script} {
    uplevel $level $script
}

# BODY trait: argument is used as a loop/control body
# The proc takes a body argument that is used as a foreach/while body.
proc iterate_and_run {items body} {
    foreach item $items {
        uplevel 1 $body
    }
}

proc repeat_n {n body} {
    for {set i 0} {$i < $n} {incr i} {
        uplevel 1 $body
    }
}

# VAR_WRITE trait: argument names a variable the proc writes
# The proc takes a variable name and uses upvar to modify it.
proc safe_incr {varName {amount 1}} {
    upvar 1 $varName var
    if {![info exists var]} {
        set var 0
    }
    incr var $amount
}

proc set_default {varName defaultValue} {
    upvar 1 $varName var
    if {![info exists var]} {
        set var $defaultValue
    }
}

# VAR_READ trait: argument names a variable the proc reads via upvar
# but never writes through the alias — only reads.
proc is_positive {varName} {
    upvar 1 $varName var
    return [expr {$var > 0}]
}

# LOOP_LIST trait: argument is used as the list in foreach
# The proc takes a list argument used in a foreach iteration.
proc sum_list {numbers} {
    set total 0
    foreach n $numbers {
        incr total $n
    }
    return $total
}

proc map_apply {items transform} {
    set result [list]
    foreach item $items {
        lappend result [uplevel 1 $transform]
    }
    return $result
}

# EXPR trait: argument is evaluated as an expression
proc check_condition {condition} {
    if {$condition} {
        return 1
    }
    return 0
}

# Combined traits: one proc with multiple trait types
proc foreach_in_collection_impl {varName collection body} {
    # varName -> VAR_WRITE (used as upvar target)
    # collection -> LOOP_LIST (iterated over)
    # body -> BODY (used as a loop body)
    upvar 1 $varName var
    foreach var $collection {
        uplevel 1 $body
    }
}
