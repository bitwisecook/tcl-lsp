foreach c {oo::configurable oo::abstract oo::singleton oo::define::classmethod oo::define::private} {
  puts "$c [llength [info commands $c]]"
}
