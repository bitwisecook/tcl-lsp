if {[catch {
oo::class create Factory {
    classmethod make {} {
        return [Factory new]
    }
}
puts "make -> [Factory make]"
} msg]} { puts "ERR: $msg" }
