# self and my commands
oo::class create Dog {
    variable name
    constructor {n} { set name $n }
    method get_name {} { return $name }
    method who {} { return [self] }
    method speak {} { return [my get_name] }
    method my_class {} { return [self class] }
}
set obj [Dog create rex Rex]
set result {}
lappend result [rex who]
lappend result [rex speak]
lappend result [rex my_class]
set result
