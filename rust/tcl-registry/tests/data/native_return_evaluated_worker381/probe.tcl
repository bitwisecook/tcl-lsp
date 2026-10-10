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
if {[info exists ::jim_version]} { alias original_return_worker return } else { interp alias {} original_return_worker {} return }
observe empty {original_return_worker}
observe result {original_return_worker RESULT}
observe ordinary_pair {original_return_worker foo bar}
observe ordinary_pair_result {original_return_worker foo bar RESULT}
observe ordinary_pairs {original_return_worker foo bar baz quux}
observe lone_code {original_return_worker -code}
observe lone_level {original_return_worker -level}
observe lone_options {original_return_worker -options}
observe lone_dashdash {original_return_worker --}
observe dashdash_pair {original_return_worker -- VALUE}
observe code_ok {original_return_worker -code ok RESULT}
observe code_error {original_return_worker -code error RESULT}
observe code_return {original_return_worker -code original_return_worker RESULT}
observe code_break {original_return_worker -code break RESULT}
observe code_continue {original_return_worker -code continue RESULT}
observe code_signal {original_return_worker -code signal RESULT}
observe code_exit {original_return_worker -code exit RESULT}
observe code_eval {original_return_worker -code eval RESULT}
observe code_prefix {original_return_worker -code o RESULT}
observe code_010 {original_return_worker -code 010 RESULT}
observe code_uintmax {original_return_worker -code 4294967295 RESULT}
observe code_negative_uintmax {original_return_worker -code -4294967295 RESULT}
observe code_over_uintmax {original_return_worker -code 4294967296 RESULT}
observe code_intmin {original_return_worker -code -2147483648 RESULT}
observe code_bad_then_ok {original_return_worker -code BAD -code ok RESULT}
observe code_ok_then_bad {original_return_worker -code ok -code BAD RESULT}
observe level_zero {original_return_worker -level 0 RESULT}
observe level_two {original_return_worker -level 2 RESULT}
observe level_negative {original_return_worker -level -1 RESULT}
observe level_010 {original_return_worker -level 010 RESULT}
observe level_intmax {original_return_worker -level 2147483647 RESULT}
observe level_over_intmax {original_return_worker -level 2147483648 RESULT}
observe level_uintmax {original_return_worker -level 4294967295 RESULT}
observe level_bad_then_zero {original_return_worker -level BAD -level 0 RESULT}
observe level_negative_then_zero {original_return_worker -level -1 -level 0 RESULT}
observe errorcode_valid {original_return_worker -code error -errorcode {A B} RESULT}
observe errorcode_bad {original_return_worker -errorcode \{ RESULT}
observe errorcode_bad_then_good {original_return_worker -errorcode \{ -errorcode {A B} RESULT}
observe errorinfo_any {original_return_worker -errorinfo \{ RESULT}
observe errorstack_odd {original_return_worker -errorstack {INNER} RESULT}
observe errorstack_even {original_return_worker -errorstack {INNER command} RESULT}
observe errorstack_bad {original_return_worker -errorstack \{ RESULT}
observe errorstack_bad_then_good {original_return_worker -errorstack \{ -errorstack {INNER command} RESULT}
observe custom_dash {original_return_worker -custom VALUE RESULT}
observe custom_bare {original_return_worker custom VALUE RESULT}
observe custom_dynamic_value {set value V; original_return_worker -custom $value RESULT}
observe options_empty {original_return_worker -options {} RESULT}
observe options_code_ok {original_return_worker -options {-code ok -level 0} RESULT}
observe options_odd {original_return_worker -options {-code} RESULT}
observe options_bad_code {original_return_worker -options {-code BAD} RESULT}
observe options_bad_overridden {original_return_worker -options {-code BAD} -code ok RESULT}
observe options_bad_level_overridden {original_return_worker -options {-level -1} -level 0 RESULT}
observe options_overrides_bad_code {original_return_worker -code BAD -options {-code ok} RESULT}
observe options_nested {original_return_worker -options {-options {-code ok -level 0}} RESULT}
observe options_code_key_bare {original_return_worker -options {code error} RESULT}
observe code_return_level_zero {original_return_worker -code original_return_worker -level 0 RESULT}
observe code_return_level_two {original_return_worker -code original_return_worker -level 2 RESULT}
observe code_break_level_zero {original_return_worker -code break -level 0 RESULT}
observe nul_key_code {set key [binary format H* 2d636f6465007461696c]; original_return_worker $key error RESULT}
observe nul_code_ok {set value [binary format H* 6f6b007461696c]; original_return_worker -code $value RESULT}
observe nul_custom_key {set key [binary format H* 637573746f6d007461696c]; original_return_worker $key VALUE RESULT}
observe level_negative_uintmax {original_return_worker -level -4294967295 RESULT}
observe level_negative_over_intmin {original_return_worker -level -2147483649 RESULT}
observe level_negative_over_uintmax {original_return_worker -level -4294967296 RESULT}
observe level_over_uintmax {original_return_worker -level 4294967296 RESULT}
observe level_named_ok {original_return_worker -level ok RESULT}
observe options_nested_before_outer_code {original_return_worker -options {-options {-code error} -code ok} RESULT}
observe options_nested_after_outer_code {original_return_worker -options {-code ok -options {-code error}} RESULT}
observe options_repeated_nested_bad_then_good {original_return_worker -options {-options {-code BAD} -options {-code ok}} RESULT}
observe options_repeated_nested_invalid_then_good {original_return_worker -options {-options \{ -options {-code ok}} RESULT}
observe options_repeated_code_bad_then_good {original_return_worker -options {-code BAD -code ok} RESULT}
observe options_odd_then_empty {original_return_worker -options {-code} -options {} RESULT}
observe options_bad_list_then_empty {original_return_worker -options \{ -options {} RESULT}
observe options_errorcode_origin {original_return_worker -options {-code error -level 0 -errorcode {MERGED CODE}} RESULT}
observe nul_numeric_code {set value [binary format H* 31007461696c]; original_return_worker -code $value RESULT}
observe nul_numeric_level {set value [binary format H* 31007461696c]; original_return_worker -level $value RESULT}
