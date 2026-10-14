puts [list interpreter [info patchlevel]]
foreach name [list {} {$dollar} {${braced}} {arr(key)}] {
    catch {unset $name}
    set code [catch {return PAYLOAD} $name]
    set exists [info exists $name]
    set value MISSING
    if {$exists} { set value [set $name] }
    puts [list catch $name code $code exists $exists value $value]
}
foreach {result options} [list {} {$options} {$dollar2} {${opts}} {arr2(key)} {opts(key)}] {
    catch {unset $result}
    catch {unset $options}
    set outer [catch {catch {return PAYLOAD2} $result $options} message]
    puts [list catch_options $result $options outer $outer result_exists [info exists $result] options_exists [info exists $options] message $message]
    if {[info exists $result]} { puts [list result $result [set $result]] }
}
if {[llength [info commands try]]} {
    foreach {result options} [list {} {$try_options} {$try_dollar} {${try_opts}} {try_arr(key)} {try_opts(key)}] {
        catch {unset $result}
        catch {unset $options}
        set outer [catch {try {error BOOM} on error [list $result $options] {
            puts [list try $result $options result_exists [info exists $result] value [set $result] options_exists [info exists $options]]
        }} message]
        puts [list try_status $result $options $outer $message]
    }
} else { puts [list try unavailable] }
