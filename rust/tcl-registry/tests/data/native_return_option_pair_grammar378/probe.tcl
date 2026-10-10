proc hex {value} { binary scan $value H* result; return $result }
set has_options [expr {[catch {catch {expr 1} scratch scratch_options}] == 0}]
puts "META|[info patchlevel]|$has_options"
proc observe {label script} {
    global has_options
    catch {unset ::errorCode}
    catch {unset ::errorInfo}
    if {$has_options} {
        set own [catch $script result options]
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
observe empty {return}
observe result {return RESULT}
observe ordinary_pair {return foo bar}
observe ordinary_pair_result {return foo bar RESULT}
observe ordinary_pairs {return foo bar baz quux}
observe lone_code {return -code}
observe lone_level {return -level}
observe lone_options {return -options}
observe lone_dashdash {return --}
observe dashdash_pair {return -- VALUE}
observe code_ok {return -code ok RESULT}
observe code_error {return -code error RESULT}
observe code_return {return -code return RESULT}
observe code_break {return -code break RESULT}
observe code_continue {return -code continue RESULT}
observe code_signal {return -code signal RESULT}
observe code_exit {return -code exit RESULT}
observe code_eval {return -code eval RESULT}
observe code_prefix {return -code o RESULT}
observe code_010 {return -code 010 RESULT}
observe code_uintmax {return -code 4294967295 RESULT}
observe code_negative_uintmax {return -code -4294967295 RESULT}
observe code_over_uintmax {return -code 4294967296 RESULT}
observe code_intmin {return -code -2147483648 RESULT}
observe code_bad_then_ok {return -code BAD -code ok RESULT}
observe code_ok_then_bad {return -code ok -code BAD RESULT}
observe level_zero {return -level 0 RESULT}
observe level_two {return -level 2 RESULT}
observe level_negative {return -level -1 RESULT}
observe level_010 {return -level 010 RESULT}
observe level_intmax {return -level 2147483647 RESULT}
observe level_over_intmax {return -level 2147483648 RESULT}
observe level_uintmax {return -level 4294967295 RESULT}
observe level_bad_then_zero {return -level BAD -level 0 RESULT}
observe level_negative_then_zero {return -level -1 -level 0 RESULT}
observe errorcode_valid {return -code error -errorcode {A B} RESULT}
observe errorcode_bad {return -errorcode \{ RESULT}
observe errorcode_bad_then_good {return -errorcode \{ -errorcode {A B} RESULT}
observe errorinfo_any {return -errorinfo \{ RESULT}
observe errorstack_odd {return -errorstack {INNER} RESULT}
observe errorstack_even {return -errorstack {INNER command} RESULT}
observe errorstack_bad {return -errorstack \{ RESULT}
observe errorstack_bad_then_good {return -errorstack \{ -errorstack {INNER command} RESULT}
observe custom_dash {return -custom VALUE RESULT}
observe custom_bare {return custom VALUE RESULT}
observe custom_dynamic_value {set value V; return -custom $value RESULT}
observe options_empty {return -options {} RESULT}
observe options_code_ok {return -options {-code ok -level 0} RESULT}
observe options_odd {return -options {-code} RESULT}
observe options_bad_code {return -options {-code BAD} RESULT}
observe options_bad_overridden {return -options {-code BAD} -code ok RESULT}
observe options_bad_level_overridden {return -options {-level -1} -level 0 RESULT}
observe options_overrides_bad_code {return -code BAD -options {-code ok} RESULT}
observe options_nested {return -options {-options {-code ok -level 0}} RESULT}
observe options_code_key_bare {return -options {code error} RESULT}
observe code_return_level_zero {return -code return -level 0 RESULT}
observe code_return_level_two {return -code return -level 2 RESULT}
observe code_break_level_zero {return -code break -level 0 RESULT}
observe nul_key_code {set key [binary format H* 2d636f6465007461696c]; return $key error RESULT}
observe nul_code_ok {set value [binary format H* 6f6b007461696c]; return -code $value RESULT}
observe nul_custom_key {set key [binary format H* 637573746f6d007461696c]; return $key VALUE RESULT}
