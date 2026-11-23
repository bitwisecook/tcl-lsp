# Jim local: the command is deleted when the calling proc returns
proc outer {} {
    local proc helper {} {return local-helper}
    return [helper]
}
puts [outer]
puts "after: [llength [info commands helper]]"
