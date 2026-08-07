namespace eval ::U {}
::U::Meta create ::U::Widget { method go {} { return went } }
set u [::U::Widget new]
puts [$u go]
