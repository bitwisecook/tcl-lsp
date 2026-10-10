puts [list version [info patchlevel] initial_class [info commands class]]
set package_code [catch {package require oo} package_result]
puts [list oo_package $package_code $package_result class_command [info commands class]]
if {[llength [info commands class]] == 0} {
    puts [list class_initialiser_scope not_applicable]
    exit 0
}
proc {::RootConstructor constructor} {args} {lappend ::seen_root_constructor $args}
set original_constructor_body [info body {::RootConstructor constructor}]
set create_code [catch {class ::RootConstructor {}} create_result]
set body_code [catch {info body {::RootConstructor constructor}} current_constructor_body]
puts [list rooted_early_constructor $create_code $body_code [expr {$original_constructor_body eq $current_constructor_body}]]
set seen_root_constructor {}
set object_code [catch {::RootConstructor new FIRST} object_result]
puts [list rooted_early_constructor_call $object_code $seen_root_constructor]
proc {::RootGet get} {} {return ORIGINAL_GET}
set original_get_body [info body {::RootGet get}]
set create_code [catch {class ::RootGet {}} create_result]
set body_code [catch {info body {::RootGet get}} current_get_body]
puts [list rooted_early_get $create_code $body_code [expr {$original_get_body eq $current_get_body}]]
proc {::RootConstructor constructor} {args} {lappend ::seen_root_constructor [linsert $args 0 REPLACEMENT]}
set seen_root_constructor {}
set object_code [catch {::RootConstructor new SECOND} object_result]
puts [list rooted_later_constructor_call $object_code $seen_root_constructor]
