puts [list version [info patchlevel]]
set ::basic__addr ROOT
set {:basic__addr} LITERAL
namespace eval ns {set basic__addr QUALIFIED}
set ::pool__members(0) ELEMENT
foreach {key script} {
    absolute {list $::basic__addr}
    repeated {list $:::basic__addr}
    loneColon {list ${:basic__addr}}
    qualified {list $::ns::basic__addr}
    bareElement {list $::pool__members(0)}
    bracedElement {list ${::pool__members(0)}}
    inertBraced {list {$::unseen__field}}
    escapedDollar {list \$::unseen__field}
    quotedRead {list "$::basic__addr"}
    expressionRead {expr {$::basic__addr eq "ROOT"}}
    nestedRead {list [list $::basic__addr]}
    commentOnly {# $::unseen__field
        list DONE}
    trailingRoot {list ${basic__addr::}}
} {
    set code [catch {eval $script} value]
    puts [list $key $code $value]
}
