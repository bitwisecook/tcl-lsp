# Independent native dictionary compiler protocol; no compiler output sets wants.
puts [list dictionary-available [expr {[llength [info commands dict]] != 0}]]
if {[llength [info commands dict]] == 0} {return}
proc rebind {} {
    rename dict savedDict
    proc dict args {return CUSTOM}
    return VALUE
}
proc observe {name body} {
    proc P {} $body
    set code [catch P result]
    puts [list $name $code $result]
    if {[llength [info commands savedDict]]} {rename dict {}; rename savedDict dict}
}
observe create-before-argv {dict create k [rebind]}
observe get-before-argv {dict get {k OLD} [rebind]}
observe set-local-before-argv {set d {}; dict set d k [rebind]; list $d}
observe set-global-before-argv {set ::d {}; dict set ::d k [rebind]; list $::d}
observe for-captured-break {set seen {}; dict for {k v} {a A b B} {lappend seen $k; break}; list $seen}
observe for-captured-continue {set seen {}; dict for {k v} {a A b B} {if {$k eq "a"} {continue}; lappend seen $k}; list $seen}
observe update-return-writeback {set d {a A}; set code [catch {dict update d a value {set value NEW; return BODY}} result]; list $code $result $d}
observe with-return-writeback {set d {a A}; set code [catch {dict with d {set a NEW; return BODY}} result]; list $code $result $d}
observe for-body-replaces-helper {set seen {}; dict for {k v} {a A b B} {lappend seen $k; if {$k eq "a"} {rename ::tcl::dict::for ::tcl::dict::savedFor}}; list $seen}
if {[llength [info commands ::tcl::dict::savedFor]]} {rename ::tcl::dict::savedFor ::tcl::dict::for}

observe with-malformed-writeback {set d {k OLD};set c [catch {dict with d {set d BROKEN}} r];list $c $r $d}
observe with-dictionary-key-collision {set d {d INNER k OLD};set c [catch {dict with d {set k NEW}} r];list $c $r $d}
