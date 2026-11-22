::tcl::tm::path add [list <fixtures>/tm]
puts [list [package require acme::widget] [::acme::widget::v] [lsort [package versions acme::widget]]]
