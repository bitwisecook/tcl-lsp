namespace eval N {}
proc alias_writer {source_name local_name} {
    upvar 1 $source_name $local_name
    set $local_name WRITTEN
}
proc witness {name} {
    alias_writer $name { local alias }
    return [list [set $name] [expr {[lsearch -exact [info locals] $name] >= 0}]]
}
foreach name [list {} { with space } {$literal} {[literal]} {literal(open} 1abc café N::qualified array(key)] {
    set code [catch {witness $name} result]
    puts [list $name $code $result]
}
