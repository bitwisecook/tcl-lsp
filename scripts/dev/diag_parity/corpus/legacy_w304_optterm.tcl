set v $userinput
exec $v
glob $pattern
switch $v -nocase {a {puts A}} default {puts B}
lsearch $list -foo
