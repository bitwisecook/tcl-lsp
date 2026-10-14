proc run {label body} {
 set code [catch [list uplevel 1 $body] result]
 puts [list $label $code $result]
}
run empty_incr {set {} 4; incr {}; set {}}
run empty_foreach {foreach {{}} {VALUE} {set saved ${}}; set saved}
run dollar_dict_with {set {$name} {key VALUE}; dict with {$name} {set saved $key}; set saved}
run empty_dict_with {set {} {key VALUE}; dict with {} {set saved $key}; set saved}
run literal_dollar_foreach {foreach {{$name}} {VALUE} {set saved ${$name}}; set saved}
