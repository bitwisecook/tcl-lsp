puts [list version [info patchlevel] interp_available [info commands interp]]
if {[llength [info commands interp]] == 0} {
    puts [list current_holder_scope not_applicable]
    exit 0
}
namespace eval N {namespace eval q {}}
set code [catch {namespace eval N {interp alias {} q::a {} list VALUE}} result]
puts [list relative_qualified $code $result [info commands ::N::q::a] [info commands ::q::a]]
set local_code [catch {::N::q::a} local_result]
set root_code [catch {::q::a} root_result]
puts [list relative_calls $local_code $local_result $root_code $root_result]
set code [catch {namespace eval N {interp alias {} plain {} list PLAIN}} result]
puts [list unqualified $code $result [info commands ::N::plain] [info commands ::plain]]
set code [catch {namespace eval N {interp alias {} ::N::q::rooted {} list ROOTED}} result]
set call_code [catch {::N::q::rooted} call_result]
puts [list explicitly_rooted $code $result [info commands ::N::q::rooted] $call_code $call_result]
