catch {fconfigure stdout -buffering none}
set version_code [catch {info patchlevel} version]
puts [list version $version_code $version]
set open [format %c 123]
set close [format %c 125]
set sigil [format %c 36]
set namespace_name "ns$open"
set old_name "${namespace_name}::x$close"
set first_name "${namespace_name}::x"
set new_name "${namespace_name}::new"
set old "$sigil$open$old_name$close"
set proposed "$sigil$open$new_name$close"
set selected [lindex $argv 0]
if {[string equal $selected complete]} {
    foreach label {old proposed} {
        set reference [set $label]
        set code [catch {info complete $reference} value]
        puts [list $label $reference complete $code $value]
    }
} elseif {[string equal $selected missing-read]} {
    foreach label {old proposed} {
        set reference [set $label]
        set code [catch {subst -nocommands -nobackslashes $reference} value]
        puts [list $label $reference read $code $value]
    }
} elseif {[string equal $selected populated-read]} {
    puts [list namespace-attempt $namespace_name]
    set code [catch {namespace eval $namespace_name {}} value]
    puts [list namespace-result $code $value]
    if {$code == 0} {
        foreach name [list $old_name $first_name $new_name] label {OLD_BALANCED OLD_FIRST NEW} {
            set code [catch {set $name $label} value]
            puts [list set-result $name $code $value]
        }
        foreach label {old proposed} {
            set reference [set $label]
            set code [catch {subst -nocommands -nobackslashes $reference} value]
            puts [list $label $reference read $code $value]
        }
    }
}
