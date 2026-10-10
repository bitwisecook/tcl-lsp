# Original ASCII source-script lifecycle controls. No installed native object
# header, binary-produced operand or BIG-IP context is observed by this script.
puts [list META patchlevel [info patchlevel] tcloo_command [llength [info commands ::oo::class]]]
if {![llength [info commands ::oo::class]]} {
    puts [list UNAVAILABLE reason no_original_oo_class_command]
} else {
    foreach {label params body} {
        empty {a b} {}
        space {a b} { }
        comment {a b} {# body}
        empty_malformed {{a b c}} {}
        space_malformed {{a b c}} { }
    } {
        set name ::original_lifecycle_$label
        set code [catch [list ::oo::class create $name [list constructor $params $body]] result]
        puts [list DECL label $label code $code result $result]
        if {$code == 0} {
            set ownCode [catch [list info class constructor $name] ownResult]
            puts [list OWN label $label code $ownCode constructor $ownResult]
            foreach count {0 1 2 3} {
                set argv [list $name new]
                for {set i 0} {$i < $count} {incr i} {lappend argv V$i}
                set code [catch $argv result]
                set ec {}
                if {$code != 0 && [info exists ::errorCode]} {set ec $::errorCode}
                puts [list CALL label $label argc $count code $code result $result errorCode $ec]
                if {$code == 0} {$result destroy}
            }
            $name destroy
        }
    }
    ::oo::class create ::original_lifecycle_parent {constructor {a b} { }}
    ::oo::class create ::original_lifecycle_child {
        superclass ::original_lifecycle_parent
        constructor {{a b c}} {}
    }
    set ownCode [catch {info class constructor ::original_lifecycle_child} ownResult]
    puts [list OWN label inherited_after_empty_removal code $ownCode constructor $ownResult]
    foreach count {0 1 2 3} {
        set argv [list ::original_lifecycle_child new]
        for {set i 0} {$i < $count} {incr i} {lappend argv V$i}
        set code [catch $argv result]
        set ec {}
        if {$code != 0 && [info exists ::errorCode]} {set ec $::errorCode}
        puts [list CALL label inherited_after_empty_removal argc $count code $code result $result errorCode $ec]
        if {$code == 0} {$result destroy}
    }
    ::original_lifecycle_child destroy
    ::original_lifecycle_parent destroy
}
