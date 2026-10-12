puts "VERSION|[info patchlevel]"
foreach {id argv} [list \
 baseline_optionlike_tail [list string equal -nocase -nocase -nocase] \
 shortened_selected_prefix [list string e -n -nocase -nocase] \
 corrupted_tail [list string e -n -n -nocase] \
 two_literal_tails [list string equal -nocase -nocase] \
 corrupted_first_literal [list string equal -n -nocase] \
 selected_length_before_tail [list string equal -nocase -length 3 -nocase -nocase]] {
 set code [catch {uplevel #0 $argv} result]
 binary scan $result H* hex
 puts "$id|$code|$hex"
}
