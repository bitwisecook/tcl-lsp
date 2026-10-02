set ::out {}

{*}{set statement 2}
lappend ::out statement $statement

lappend ::out head-ordinary-expanded \
    [{*}{list head} ordinary {*}{tail1 tail2}]
lappend ::out empty [{*}{}]
lappend ::out multi [{*}{list a b}]

set prefix l
set suffix {ist seed}
lappend ::out substitutions [{*}$prefix$suffix]

set value [{*}{list value} position]
lappend ::out value $value

set bad "list {"
set code [catch {{*}$bad} message options]
lappend ::out malformed $code $message [dict get $options -errorcode]

set ::out
