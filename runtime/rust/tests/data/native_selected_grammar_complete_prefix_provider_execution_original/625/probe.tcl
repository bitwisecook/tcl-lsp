puts [list VERSION [info patchlevel]]
set candidates {
  selected_braced_var 736574206265666f726520313b207075747320247b617b627d3b207365742061667465722032
  terminal_word 736574206265666f726520313b2073657420696e76616c6964207b617d623b207365742061667465722032
}
foreach {case hex} $candidates {
    catch {unset before}
    catch {unset after}
    catch {unset invalid}
    set [format %c%c%c 97 123 98] VALUE
    set script [binary format H* $hex]
    set completeCode [catch {info complete $script} complete]
    binary scan $complete H* completeHex
    set code [catch {uplevel #0 $script} message]
    binary scan $message H* messageHex
    set observedBefore MISSING
    set observedAfter MISSING
    if {[info exists before]} {set observedBefore $before}
    if {[info exists after]} {set observedAfter $after}
    puts [list ROW $case $completeCode $completeHex $code $messageHex $observedBefore $observedAfter]
}
