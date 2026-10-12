# Exact ASCII/LF public source. The chain length is 200 imported bindings.
# Names contain ordinary ASCII separators; no NUL or Unicode name claim is made.
proc emit {case script} {
    set code [catch {uplevel #0 $script} result]
    binary scan $result H* hex
    puts [list CASE $case code $code result_hex $hex]
    return $code
}
proc seed_proc {root} {
    namespace eval ${root}::n0 {namespace export p; proc p {{x ORIGINAL}} {return $x}}
    for {set depth 1} {$depth <= 200} {incr depth} {
        set prior [expr {$depth - 1}]
        namespace eval ${root}::n${depth} [list namespace import ${root}::n${prior}::p]
        namespace eval ${root}::n${depth} {namespace export p}
    }
}
proc query_proc {prefix name} {
    emit ${prefix}.origin [list namespace origin $name]
    emit ${prefix}.args [list info args $name]
    emit ${prefix}.body [list info body $name]
    emit ${prefix}.default [list apply_default_query $name]
    emit ${prefix}.invoke [list $name]
}
proc apply_default_query {name} {
    set present [info default $name x default]
    if {$present} {return [list $present $default]}
    return [list $present]
}
proc seed_ensemble {root} {
    namespace eval ${root}::n0 {
        proc target {} {return ENSEMBLE}
        namespace export ens
        namespace ensemble create -command ens -map [list go [list ::E::n0::target]]
    }
    for {set depth 1} {$depth <= 200} {incr depth} {
        set prior [expr {$depth - 1}]
        namespace eval ${root}::n${depth} [list namespace import ${root}::n${prior}::ens]
        namespace eval ${root}::n${depth} {namespace export ens}
    }
}
proc query_ensemble {prefix name} {
    emit ${prefix}.origin [list namespace origin $name]
    emit ${prefix}.exists [list namespace ensemble exists $name]
    emit ${prefix}.map [list namespace ensemble configure $name -map]
    emit ${prefix}.args [list info args $name]
    emit ${prefix}.invoke [list $name go]
}
emit version.patchlevel {info patchlevel}
emit version.executable {info nameofexecutable}
emit helper.namespace_origin {info body {namespace origin}}
emit helper.namespace_import {info body {namespace import}}
emit setup.P {seed_proc ::P}
foreach depth {63 64 65 200} {
    query_proc P.before${depth} ::P::n${depth}::p
}
emit P.rename {rename ::P::n0::p ::P::n0::moved}
emit P.vacated_replacement {proc ::P::n0::p {{x VACATED}} {return $x}}
query_proc P.after_rename ::P::n200::p
emit P.source_call {::P::n0::moved}
emit P.vacated_call {::P::n0::p}
emit setup.R {seed_proc ::R}
query_proc R.before ::R::n200::p
emit R.middle_replacement {proc ::R::n100::p {{x REPLACED}} {return $x}}
query_proc R.after_middle_replacement ::R::n200::p
emit setup.D {seed_proc ::D}
query_proc D.before ::D::n200::p
emit D.delete_source {rename ::D::n0::p {}}
query_proc D.after_delete ::D::n200::p
emit D.recreate_source {proc ::D::n0::p {{x RECREATED}} {return $x}}
query_proc D.after_recreate ::D::n200::p
emit setup.E {seed_ensemble ::E}
foreach depth {63 64 65 200} {
    query_ensemble E.before${depth} ::E::n${depth}::ens
}
emit E.configure_import {namespace ensemble configure ::E::n200::ens -map {changed ::E::n0::target}}
emit E.source_map {namespace ensemble configure ::E::n0::ens -map}
emit E.import_map {namespace ensemble configure ::E::n200::ens -map}
emit E.rename_source {rename ::E::n0::ens ::E::n0::moved}
emit E.vacated_replacement {proc ::E::n0::ens {} {return VACATED_ENSEMBLE}}
emit E.after_rename.origin {namespace origin ::E::n200::ens}
emit E.after_rename.exists {namespace ensemble exists ::E::n200::ens}
emit E.after_rename.map {namespace ensemble configure ::E::n200::ens -map}
emit E.after_rename.invoke {::E::n200::ens changed}
emit E.vacated_call {::E::n0::ens}
puts CLOSED_DEEP_IMPORT_CHAIN_200
