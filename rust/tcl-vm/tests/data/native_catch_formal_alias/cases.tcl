puts "VERSION|[info patchlevel]"
proc report {label script} {
    set code [catch {uplevel #0 $script} result]
    binary scan $result H* resultHex
    puts "$label|$code|$resultHex"
}
report LINKED {
    set ::destination OLD
    proc mutate {} {
        uplevel 1 {unset name; upvar #0 ::destination name}
        error BOOM
    }
    proc probe {name} {
        catch {mutate} name
        list $name $::destination
    }
    probe FORMAL
}
report PLAIN {
    proc plain {name} {
        catch {error BOOM} name
        list $name
    }
    plain FORMAL
}
report SEPARATE_RESULT {
    set ::destination OLD
    proc direct {name} {
        catch {mutate} result
        list $name $result $::destination
    }
    direct FORMAL
}
