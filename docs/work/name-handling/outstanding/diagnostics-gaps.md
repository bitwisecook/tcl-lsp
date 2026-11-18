# Diagnostics gaps

- A call proved to raise through its callee's summary has no call-site diagnostic: `proc f {d} {dict update d k v {puts $v}}; proc g {} {f {}}; g` raises `can't read "v"` (tclsh 8.6.18/9.0.4), and by the ruling of 18 November nothing is reported in `f`'s body (`fp_depth::bug_dict_update_value_var::dict_update_harvests_keys_through_the_sccp_version`).
