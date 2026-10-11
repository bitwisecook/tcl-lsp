puts "VERSION|[info patchlevel]"
namespace eval scope {proc within {} {return [info level]}; puts "NESTED|[within]"}
puts "DIRECT|[scope::within]"
