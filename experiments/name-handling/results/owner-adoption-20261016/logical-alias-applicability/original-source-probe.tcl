set root_value ROOT
namespace eval N {
    variable root_value NAMESPACE
    set original_name root_value
    global $original_name
    puts "namespace:[set root_value]"
}
proc read_global {original_name} {
    global $original_name
    puts "procedure:[set root_value]"
}
read_global root_value
