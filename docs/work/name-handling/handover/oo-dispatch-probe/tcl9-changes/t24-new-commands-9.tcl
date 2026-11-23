foreach c {lseq ledit lpop lremove const coroinject coroprobe readFile writeFile foreachLine fpclassify zipfs
           tcl::process tcl::idna::encode tcl::build-info tcl::unsupported::inject case lfilter unicode timer tcl::registry} {
  puts "$c [llength [info commands $c]]"
}
