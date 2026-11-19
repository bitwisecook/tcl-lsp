foreach opt [list {-dir} {-dog} {-path} {-tails} {-tails a} {-dir a -d} {-dir a -d b xx} {-dir a -path p xx} {-path a -d b xx} {-path a -path b xx} {-types {}} {} {-d a -d b -n *}] {
 set code [catch [linsert $opt 0 glob] result]; if {$code} {set ec $::errorCode} else {set ec {}}
 puts [list $opt $code $result $ec]
}
