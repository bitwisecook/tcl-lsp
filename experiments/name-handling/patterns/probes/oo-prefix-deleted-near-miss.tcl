oo::class create ::nhclass {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}
set object [::nhclass new]
set prefix [$object prefix]
$object destroy
catch {eval $prefix} result
string match {invalid command name *} $result

