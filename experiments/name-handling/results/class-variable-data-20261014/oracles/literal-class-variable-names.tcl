if {[catch {info patchlevel} version]} {catch {info version} version}
puts [list version $version]
set code [catch {
    oo::class create LiteralClassProbe {}
    oo::define LiteralClassProbe variable -set {} {$counter} {brace{name}} {-dash}
    set names [info class variables LiteralClassProbe]
    set found {}
    foreach name [list {} {$counter} {brace{name}} {-dash}] {
        lappend found [expr {[lsearch -exact $names $name] >= 0}]
    }
    list $names $found
} result]
puts [list class-variable-names $code $result]
