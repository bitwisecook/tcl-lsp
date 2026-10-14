set input [lindex $argv 0]
set channel [open $input w]
puts $channel FIRST
puts $channel SECOND
close $channel
proc inspect_line {command input} {
    foreach name {{a b} {$literal} {[literal]} {} {a($key)}} {
        set seen {}
        set code [catch {
            $command $name $input {lappend seen [set $name]}
        } value]
        puts [list receiver $command $name $code $seen $value]
        catch {unset $name}
    }
}
set core [catch {foreachLine probe $input {}} value]
puts [list core-availability $core $value]
if {!$core} {inspect_line foreachLine $input}
lappend auto_path {/tmp/name-issues-refresh-20261014/tcllib-tcllib-2-0/modules}
set loaded [catch {package require fileutil} value]
puts [list fileutil-availability $loaded $value]
if {!$loaded} {inspect_line ::fileutil::foreachLine $input}
file delete $input
