puts [list interpreter [info patchlevel] library [info library] executable [info nameofexecutable]]
set modules /tmp/name-issues-refresh-20261014/tcllib-tcllib-2-0/modules
foreach package {struct::graph struct::tree} {
  foreach arguments {{} {owned} {{}} {{$literal}}} {
    set child [interp create]
    interp eval $child [list lappend auto_path [file join $modules struct] [file join $modules cmdline]]
    set version [interp eval $child [list package require $package]]
    puts [list provider $package implementation [interp eval $child [list set ::${package}::loaded]] loaded [interp eval $child {info loaded}]]
    set code [catch {interp eval $child [linsert $arguments 0 ::$package]} result options]
    set empty [interp eval $child {namespace which -command ::}]
    set requested [expr {[llength $arguments] ? [lindex $arguments 0] : {}}]
    set reached [expr {$code == 0 ? [interp eval $child [list namespace which -command $result]] : {}}]
    puts [list constructor $package version $version argc [llength $arguments] requested_hex [binary encode hex $requested] code $code result_hex [binary encode hex $result] empty_command_hex [binary encode hex $empty] reached_hex [binary encode hex $reached]]
    interp delete $child
  }
}
puts [list loaded [info loaded]]
