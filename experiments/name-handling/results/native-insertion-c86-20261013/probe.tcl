foreach index {0 end 1 end-1 -8 2147483647} {
    proc p {name} "linsert \$name $index X Y"
    puts [list INDEX $index RESULT [p {{A} B C}]]
    puts [tcl::unsupported::disassemble proc p]
}
proc helper {args name} {linsert $args 0 interp alias {} $name {}}
puts [tcl::unsupported::disassemble proc helper]
puts [list HELPER [helper {one two} original]]
