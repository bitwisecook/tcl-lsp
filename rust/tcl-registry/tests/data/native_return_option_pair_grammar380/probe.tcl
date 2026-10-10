proc hex {value} { binary scan $value H* result; return $result }
set has_options [expr {[catch {catch {expr 1} scratch scratch_options}] == 0}]
puts "META|[info patchlevel]|$has_options"
proc observe {label script} {
    global has_options
    catch {unset ::errorCode}
    catch {unset ::errorInfo}
    if {$has_options} {
        set own [eval [list catch $script result options]]
        array set picked $options
        set optcode $picked(-code)
        set level $picked(-level)
        set errorcode ABSENT
        if {[info exists picked(-errorcode)]} { set errorcode [hex $picked(-errorcode)] }
        set optionshex [hex $options]
    } else {
        set own [catch $script result]
        set optcode ABSENT
        set level ABSENT
        set errorcode ABSENT
        set optionshex ABSENT
        if {[info exists ::errorCode]} {set errorcode [hex $::errorCode]}
    }
    puts "ROW|$label|$own|[hex $result]|$optcode|$level|$errorcode|$optionshex"
}
observe level_negative_uintmax {return -level -4294967295 RESULT}
observe level_negative_over_intmin {return -level -2147483649 RESULT}
observe level_negative_over_uintmax {return -level -4294967296 RESULT}
observe level_over_uintmax {return -level 4294967296 RESULT}
observe level_named_ok {return -level ok RESULT}
observe options_nested_before_outer_code {return -options {-options {-code error} -code ok} RESULT}
observe options_nested_after_outer_code {return -options {-code ok -options {-code error}} RESULT}
observe options_repeated_nested_bad_then_good {return -options {-options {-code BAD} -options {-code ok}} RESULT}
observe options_repeated_nested_invalid_then_good {return -options {-options \{ -options {-code ok}} RESULT}
observe options_repeated_code_bad_then_good {return -options {-code BAD -code ok} RESULT}
observe options_odd_then_empty {return -options {-code} -options {} RESULT}
observe options_bad_list_then_empty {return -options \{ -options {} RESULT}
observe options_errorcode_origin {return -options {-code error -level 0 -errorcode {MERGED CODE}} RESULT}
observe nul_numeric_code {set value [binary format H* 31007461696c]; return -code $value RESULT}
observe nul_numeric_level {set value [binary format H* 31007461696c]; return -level $value RESULT}
