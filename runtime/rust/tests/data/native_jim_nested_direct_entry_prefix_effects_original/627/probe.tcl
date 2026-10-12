puts [list VERSION [info patchlevel]]
set candidates {
  terminal_child 736574206265666f726520313b207365742073696e6b205b6c697374207b617d625d3b207365742061667465722032
  terminal_child_prefix 736574206265666f726520313b207365742073696e6b205b73657420696e6e657220313b206c697374207b617d625d3b207365742061667465722032
  terminal_quoted_child 736574206265666f726520313b207365742073696e6b20225b6c697374207b617d625d223b207365742061667465722032
  terminal_index_child 736574206265666f726520313b207365742073696e6b2024617272285b6c697374207b617d625d293b207365742061667465722032
  missing_child_brace 736574206265666f726520313b207365742073696e6b205b6c697374207b615d3b207365742061667465722032
  missing_child_variable_brace 736574206265666f726520313b207365742073696e6b205b6c69737420247b615d3b207365742061667465722032
  bare_child_open_brace 736574206265666f726520313b207365742073696e6b205b6c69737420787b5d3b207365742061667465722032
}
foreach {case hex} $candidates {
    foreach name {before inner sink after arr} {catch {unset $name}}
    set arr(known) VALUE
    set script [binary format H* $hex]
    set completeCode [catch {info complete $script} complete]
    binary scan $complete H* completeHex
    set code [catch {uplevel #0 $script} message]
    binary scan $message H* messageHex
    set effects {}
    foreach name {before inner sink after} {
        if {[info exists $name]} {binary scan [set $name] H* value; lappend effects $name PRESENT $value} else {lappend effects $name MISSING}
    }
    puts [list ROW $case COMPLETE_CODE $completeCode COMPLETE_HEX $completeHex CODE $code RESULT_HEX $messageHex EFFECTS $effects]
}
