puts [list version [info patchlevel] initial_class [info commands class]]
set package_code [catch {package require oo} package_result]
puts [list oo_package $package_code $package_result class_command [info commands class]]
if {[llength [info commands class]] == 0} {
    puts [list jim_initialiser_controls not_applicable no_class_command]
} else {
    foreach member {new finalize method vars classvars classname methods defaultconstructor constructor destroy get eval} {
        set classname Init_$member
        set command "$classname $member"
        proc $command args {return STALE}
        set before [info body $command]
        set factory_code [catch {class $classname {}} factory_result]
        set body_code [catch {info body $command} after]
        puts [list initialiser $member $factory_code $body_code [expr {$body_code == 0 && $after eq $before}]]
    }
    proc {WithoutBase baseclass} args {return STALE}
    set before [info body {WithoutBase baseclass}]
    set code [catch {class WithoutBase {}} result]
    set body_code [catch {info body {WithoutBase baseclass}} after]
    puts [list baseclass_without_bases $code $body_code [expr {$body_code == 0 && $after eq $before}]]
    class ActualBase {}
    proc {WithBase baseclass} args {return STALE}
    set before [info body {WithBase baseclass}]
    set code [catch {class WithBase {ActualBase} {}} result]
    set body_code [catch {info body {WithBase baseclass}} after]
    puts [list baseclass_with_base $code $body_code [expr {$body_code == 0 && $after eq $before}]]
    set code [catch {{WithBase baseclass}} result]
    puts [list baseclass_current_target $code $result]
    class Later {}
    proc {Later get} args {return CUSTOM}
    set code [catch {set obj [Later new]; $obj get anything} result]
    puts [list custom_after_factory $code $result]
}
