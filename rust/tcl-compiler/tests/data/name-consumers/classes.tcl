oo::class create C
oo::class create ::ns::C
namespace eval ns {
    puts [info object class [C new]]
    puts [info object class [::C new]]
}
puts [info object class [::ns:::C new]]
oo::class create ::ns::
puts [info object class [::ns::: new]]
