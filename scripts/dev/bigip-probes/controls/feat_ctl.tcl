set ::tests {
  expand_op      {list {*}{a b}}
  dict           {dict set d a 1}
  dict_for       {dict for {k v} {a 1} {}}
  lassign        {lassign {1 2} aa bb}
  apply          {apply {{} {return 1}}}
  lreverse       {lreverse {1 2 3}}
  lrepeat        {lrepeat 2 x}
  string_reverse {string reverse abc}
  pow_operator   {expr {2**3}}
  ne_operator    {expr {"a" ne "b"}}
  in_operator    {expr {"a" in {a b}}}
  mathfunc_ns    {namespace eval ::tcl::mathfunc {}}
  mathop_ns      {::tcl::mathop::+ 1 2}
  chan_cmd       {chan names}
  switch_matchvar {switch -matchvar mv -regexp -- abc {a.c {}}}
  string_is_entier {string is entier 1}
  string_is_wide {string is wideinteger 1}
  info_frame     {info frame}
  try_cmd        {try {set x 1}}
  dict_getdef    {dict getwithdefault {a 1} b 0}
  lmap_cmd       {lmap i {1 2} {set i}}
  unset_nocomp   {unset -nocomplain zz_absent}
  clock_format   {clock format 0 -format %Y -gmt 1}
  binary_encode  {binary encode hex abc}
  namespace_ens  {namespace ensemble exists ::probe_absent_ens}
}
foreach {name script} $::tests {
  if {[catch {uplevel #0 $script} err]} { set r "FEAT|$name|FAIL|$err" } else { set r "FEAT|$name|OK|" }
  puts $r
}
