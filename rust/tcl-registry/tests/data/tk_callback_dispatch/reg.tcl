set log {}
entry .one -validate all -validatecommand {lappend ::log ran; rename entry {}; return 1}
lappend log after-one=[llength [info commands entry]]
entry .two -validatecommand {eval %S}
lappend log two=[winfo exists .two]
set v x
entry .three -textvariable v -validate all -validatecommand {lappend ::log three-vcmd; return 1}
lappend log after-three
puts "tk [package require Tk] $log"
exit
