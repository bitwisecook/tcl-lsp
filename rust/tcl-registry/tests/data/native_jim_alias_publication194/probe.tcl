puts [list version [info patchlevel] alias_available [info commands alias]]
if {[llength [info commands alias]] == 0} {
    puts [list alias_publication_scope not_applicable]
    exit 0
}
proc ::alias_slot {} {return ORIGINAL}
set original [alias ::alias_slot list ROOT_ALIAS]
set bare_code [catch {alias_slot} bare_result]
set rooted_code [catch {::alias_slot} rooted_result]
set body_code [catch {info body ::alias_slot} body_result]
puts [list rooted_replacement $original $bare_code $bare_result $rooted_code $rooted_result $body_code]
proc {::AliasProbe slot} {} {return ORIGINAL_MULTIWORD}
set original [alias {::AliasProbe slot} list MULTIWORD_ALIAS]
set bare_code [catch {{AliasProbe slot}} bare_result]
set rooted_code [catch {{::AliasProbe slot}} rooted_result]
set body_code [catch {info body {::AliasProbe slot}} body_result]
puts [list multiword_replacement $original $bare_code $bare_result $rooted_code $rooted_result $body_code]
set original [alias ::::colon_slot list COLON_ALIAS]
set bare_code [catch {colon_slot} bare_result]
set rooted_code [catch {::colon_slot} rooted_result]
set extra_root_code [catch {::::colon_slot} extra_root_result]
puts [list extra_root_extent $original $bare_code $bare_result $rooted_code $rooted_result $extra_root_code $extra_root_result]
proc competition_slot {} {return ORIGINAL_GLOBAL}
set original [namespace eval AliasContext {
    proc competition_slot {} {return LOCAL_PROC}
    alias ::competition_slot list GLOBAL_ALIAS
}]
set global_code [catch {competition_slot} global_result]
set local_code [catch {namespace eval AliasContext {competition_slot}} local_result]
set rooted_code [catch {namespace eval AliasContext {::competition_slot}} rooted_result]
puts [list namespace_competition $original $global_code $global_result $local_code $local_result $rooted_code $rooted_result]
