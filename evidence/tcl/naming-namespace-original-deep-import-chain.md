# Tcl execution evidence

## Question

> Can original command/procedure/ensemble introspection follow 200 imported bindings, including depths 63,64,65 and200, without a fixed64 hop truncation? What do actual providers report after source rename, an unrelated replacement at the vacated spelling, middle import replacement, source deletion/recreation and ensemble configuration through a deep import? Genuine generation/token lifetimes must not be replaced by textual spelling heuristics.

> How do original namespace origin, procedure args/body/default/invocation and supported ensemble queries behave across depths 63,64,65 and 200 after source rename, vacated replacement, middle replacement and deletion/recreation?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> The immutable original ASCII LF source creates four independently named groups for rename/vacated replacement, middle replacement, deletion/recreation and ensemble queries. Depth samples are63,64,65 and 200. Every query records caught code and counted public result hex; mandatory setup rows gate each group, so unsupported C8.4/Jim ensemble setup cannot supply positive ensemble behavior. Version and executable queries are fresh public rows in each same original process; current Jim namespace origin/import helper bodies are independently returned by public info body. The listed 137 original CLI/header/static archive/configuration/source/distribution library inputs remain exact before and after. No new compile, private command token, object/header/cache, compiled opcode or Native frame is sampled. Twenty-three exact original source windows explain C original-command/procedure/namespace/ensemble routes and Jim scripted helper/alias behavior separately from observed public results. Recorded absolute paths, explicit TCL_LIBRARY overlays and original launch arguments are retained; compiler version, unlisted inherited environment and unrelated system libraries are unrecorded. Generic aliases, invocation identity, future lookup, handler entry, Normal beyond each caught query, editor equivalence and BIG-IP do not follow. Existing two-hop and other original records remain unchanged. Runtime traversal tests and current source contracts require independent software receipts; publication supplies no backend assertion PASS. Module registration establishes source reachability only, not an executable inventory, launched assertion or passing comparison. The original six processes,570 raw rows and all provider answers remain unchanged. The software cycle has no Native counterpart, and Jim scripted alias observations cannot become200 genuine C-style Imported generations or private token/header identity. No arbitrary-depth, frame, handler or edit authority follows.

## Script

```tcl-visible
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
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries
- Exit status: Not recorded
- Standard output:

```text-visible
CASE version.patchlevel code 0 result_hex 382e342e3230
CASE version.executable code 0 result_hex 2f776f726b73706163652f74636c2d6c73702f746d702f74636c382e342e32302f756e69782f74636c7368
CASE helper.namespace_origin code 1 result_hex 226e616d657370616365206f726967696e222069736e277420612070726f636564757265
CASE helper.namespace_import code 1 result_hex 226e616d65737061636520696d706f7274222069736e277420612070726f636564757265
CASE setup.P code 0 result_hex {}
CASE P.before63.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before63.args code 0 result_hex 78
CASE P.before63.body code 0 result_hex 72657475726e202478
CASE P.before63.default code 0 result_hex 31204f524947494e414c
CASE P.before63.invoke code 0 result_hex 4f524947494e414c
CASE P.before64.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before64.args code 0 result_hex 78
CASE P.before64.body code 0 result_hex 72657475726e202478
CASE P.before64.default code 0 result_hex 31204f524947494e414c
CASE P.before64.invoke code 0 result_hex 4f524947494e414c
CASE P.before65.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before65.args code 0 result_hex 78
CASE P.before65.body code 0 result_hex 72657475726e202478
CASE P.before65.default code 0 result_hex 31204f524947494e414c
CASE P.before65.invoke code 0 result_hex 4f524947494e414c
CASE P.before200.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before200.args code 0 result_hex 78
CASE P.before200.body code 0 result_hex 72657475726e202478
CASE P.before200.default code 0 result_hex 31204f524947494e414c
CASE P.before200.invoke code 0 result_hex 4f524947494e414c
CASE P.rename code 0 result_hex {}
CASE P.vacated_replacement code 0 result_hex {}
CASE P.after_rename.origin code 0 result_hex 3a3a503a3a6e303a3a6d6f766564
CASE P.after_rename.args code 0 result_hex 78
CASE P.after_rename.body code 0 result_hex 72657475726e202478
CASE P.after_rename.default code 0 result_hex 31204f524947494e414c
CASE P.after_rename.invoke code 0 result_hex 4f524947494e414c
CASE P.source_call code 0 result_hex 4f524947494e414c
CASE P.vacated_call code 0 result_hex 56414341544544
CASE setup.R code 0 result_hex {}
CASE R.before.origin code 0 result_hex 3a3a523a3a6e303a3a70
CASE R.before.args code 0 result_hex 78
CASE R.before.body code 0 result_hex 72657475726e202478
CASE R.before.default code 0 result_hex 31204f524947494e414c
CASE R.before.invoke code 0 result_hex 4f524947494e414c
CASE R.middle_replacement code 0 result_hex {}
CASE R.after_middle_replacement.origin code 0 result_hex 3a3a523a3a6e3130303a3a70
CASE R.after_middle_replacement.args code 0 result_hex 78
CASE R.after_middle_replacement.body code 0 result_hex 72657475726e202478
CASE R.after_middle_replacement.default code 0 result_hex 31205245504c41434544
CASE R.after_middle_replacement.invoke code 0 result_hex 5245504c41434544
CASE setup.D code 0 result_hex {}
CASE D.before.origin code 0 result_hex 3a3a443a3a6e303a3a70
CASE D.before.args code 0 result_hex 78
CASE D.before.body code 0 result_hex 72657475726e202478
CASE D.before.default code 0 result_hex 31204f524947494e414c
CASE D.before.invoke code 0 result_hex 4f524947494e414c
CASE D.delete_source code 0 result_hex {}
CASE D.after_delete.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_delete.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.recreate_source code 0 result_hex {}
CASE D.after_recreate.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_recreate.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE setup.E code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before63.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36333a3a656e7322
CASE E.before63.exists code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before63.map code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before63.args code 1 result_hex 223a3a453a3a6e36333a3a656e73222069736e277420612070726f636564757265
CASE E.before63.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36333a3a656e7322
CASE E.before64.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36343a3a656e7322
CASE E.before64.exists code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before64.map code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before64.args code 1 result_hex 223a3a453a3a6e36343a3a656e73222069736e277420612070726f636564757265
CASE E.before64.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36343a3a656e7322
CASE E.before65.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36353a3a656e7322
CASE E.before65.exists code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before65.map code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before65.args code 1 result_hex 223a3a453a3a6e36353a3a656e73222069736e277420612070726f636564757265
CASE E.before65.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36353a3a656e7322
CASE E.before200.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.before200.exists code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before200.map code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.before200.args code 1 result_hex 223a3a453a3a6e3230303a3a656e73222069736e277420612070726f636564757265
CASE E.before200.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.configure_import code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.source_map code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.import_map code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.rename_source code 1 result_hex 63616e27742072656e616d6520223a3a453a3a6e303a3a656e73223a20636f6d6d616e6420646f65736e2774206578697374
CASE E.vacated_replacement code 0 result_hex {}
CASE E.after_rename.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.after_rename.exists code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.after_rename.map code 1 result_hex 626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
CASE E.after_rename.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.vacated_call code 0 result_hex 564143415445445f454e53454d424c45
CLOSED_DEEP_IMPORT_CHAIN_200
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries
- Exit status: Not recorded
- Standard output:

```text-visible
CASE version.patchlevel code 0 result_hex 382e352e3139
CASE version.executable code 0 result_hex 2f776f726b73706163652f74636c2d6c73702f746d702f74636c382e352e31392f756e69782f74636c7368
CASE helper.namespace_origin code 1 result_hex 226e616d657370616365206f726967696e222069736e277420612070726f636564757265
CASE helper.namespace_import code 1 result_hex 226e616d65737061636520696d706f7274222069736e277420612070726f636564757265
CASE setup.P code 0 result_hex {}
CASE P.before63.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before63.args code 0 result_hex 78
CASE P.before63.body code 0 result_hex 72657475726e202478
CASE P.before63.default code 0 result_hex 31204f524947494e414c
CASE P.before63.invoke code 0 result_hex 4f524947494e414c
CASE P.before64.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before64.args code 0 result_hex 78
CASE P.before64.body code 0 result_hex 72657475726e202478
CASE P.before64.default code 0 result_hex 31204f524947494e414c
CASE P.before64.invoke code 0 result_hex 4f524947494e414c
CASE P.before65.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before65.args code 0 result_hex 78
CASE P.before65.body code 0 result_hex 72657475726e202478
CASE P.before65.default code 0 result_hex 31204f524947494e414c
CASE P.before65.invoke code 0 result_hex 4f524947494e414c
CASE P.before200.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before200.args code 0 result_hex 78
CASE P.before200.body code 0 result_hex 72657475726e202478
CASE P.before200.default code 0 result_hex 31204f524947494e414c
CASE P.before200.invoke code 0 result_hex 4f524947494e414c
CASE P.rename code 0 result_hex {}
CASE P.vacated_replacement code 0 result_hex {}
CASE P.after_rename.origin code 0 result_hex 3a3a503a3a6e303a3a6d6f766564
CASE P.after_rename.args code 0 result_hex 78
CASE P.after_rename.body code 0 result_hex 72657475726e202478
CASE P.after_rename.default code 0 result_hex 31204f524947494e414c
CASE P.after_rename.invoke code 0 result_hex 4f524947494e414c
CASE P.source_call code 0 result_hex 4f524947494e414c
CASE P.vacated_call code 0 result_hex 56414341544544
CASE setup.R code 0 result_hex {}
CASE R.before.origin code 0 result_hex 3a3a523a3a6e303a3a70
CASE R.before.args code 0 result_hex 78
CASE R.before.body code 0 result_hex 72657475726e202478
CASE R.before.default code 0 result_hex 31204f524947494e414c
CASE R.before.invoke code 0 result_hex 4f524947494e414c
CASE R.middle_replacement code 0 result_hex {}
CASE R.after_middle_replacement.origin code 0 result_hex 3a3a523a3a6e3130303a3a70
CASE R.after_middle_replacement.args code 0 result_hex 78
CASE R.after_middle_replacement.body code 0 result_hex 72657475726e202478
CASE R.after_middle_replacement.default code 0 result_hex 31205245504c41434544
CASE R.after_middle_replacement.invoke code 0 result_hex 5245504c41434544
CASE setup.D code 0 result_hex {}
CASE D.before.origin code 0 result_hex 3a3a443a3a6e303a3a70
CASE D.before.args code 0 result_hex 78
CASE D.before.body code 0 result_hex 72657475726e202478
CASE D.before.default code 0 result_hex 31204f524947494e414c
CASE D.before.invoke code 0 result_hex 4f524947494e414c
CASE D.delete_source code 0 result_hex {}
CASE D.after_delete.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_delete.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.recreate_source code 0 result_hex {}
CASE D.after_recreate.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_recreate.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE setup.E code 0 result_hex {}
CASE E.before63.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before63.exists code 0 result_hex 31
CASE E.before63.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before63.args code 1 result_hex 223a3a453a3a6e36333a3a656e73222069736e277420612070726f636564757265
CASE E.before63.invoke code 0 result_hex 454e53454d424c45
CASE E.before64.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before64.exists code 0 result_hex 31
CASE E.before64.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before64.args code 1 result_hex 223a3a453a3a6e36343a3a656e73222069736e277420612070726f636564757265
CASE E.before64.invoke code 0 result_hex 454e53454d424c45
CASE E.before65.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before65.exists code 0 result_hex 31
CASE E.before65.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before65.args code 1 result_hex 223a3a453a3a6e36353a3a656e73222069736e277420612070726f636564757265
CASE E.before65.invoke code 0 result_hex 454e53454d424c45
CASE E.before200.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before200.exists code 0 result_hex 31
CASE E.before200.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before200.args code 1 result_hex 223a3a453a3a6e3230303a3a656e73222069736e277420612070726f636564757265
CASE E.before200.invoke code 0 result_hex 454e53454d424c45
CASE E.configure_import code 0 result_hex {}
CASE E.source_map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.import_map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.rename_source code 0 result_hex {}
CASE E.vacated_replacement code 0 result_hex {}
CASE E.after_rename.origin code 0 result_hex 3a3a453a3a6e303a3a6d6f766564
CASE E.after_rename.exists code 0 result_hex 31
CASE E.after_rename.map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.after_rename.invoke code 0 result_hex 454e53454d424c45
CASE E.vacated_call code 0 result_hex 564143415445445f454e53454d424c45
CLOSED_DEEP_IMPORT_CHAIN_200
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries
- Exit status: Not recorded
- Standard output:

```text-visible
CASE version.patchlevel code 0 result_hex 382e362e3138
CASE version.executable code 0 result_hex 2f776f726b73706163652f74636c2d6c73702f746d702f74636c382e362e31382f756e69782f74636c7368
CASE helper.namespace_origin code 1 result_hex 226e616d657370616365206f726967696e222069736e277420612070726f636564757265
CASE helper.namespace_import code 1 result_hex 226e616d65737061636520696d706f7274222069736e277420612070726f636564757265
CASE setup.P code 0 result_hex {}
CASE P.before63.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before63.args code 0 result_hex 78
CASE P.before63.body code 0 result_hex 72657475726e202478
CASE P.before63.default code 0 result_hex 31204f524947494e414c
CASE P.before63.invoke code 0 result_hex 4f524947494e414c
CASE P.before64.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before64.args code 0 result_hex 78
CASE P.before64.body code 0 result_hex 72657475726e202478
CASE P.before64.default code 0 result_hex 31204f524947494e414c
CASE P.before64.invoke code 0 result_hex 4f524947494e414c
CASE P.before65.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before65.args code 0 result_hex 78
CASE P.before65.body code 0 result_hex 72657475726e202478
CASE P.before65.default code 0 result_hex 31204f524947494e414c
CASE P.before65.invoke code 0 result_hex 4f524947494e414c
CASE P.before200.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before200.args code 0 result_hex 78
CASE P.before200.body code 0 result_hex 72657475726e202478
CASE P.before200.default code 0 result_hex 31204f524947494e414c
CASE P.before200.invoke code 0 result_hex 4f524947494e414c
CASE P.rename code 0 result_hex {}
CASE P.vacated_replacement code 0 result_hex {}
CASE P.after_rename.origin code 0 result_hex 3a3a503a3a6e303a3a6d6f766564
CASE P.after_rename.args code 0 result_hex 78
CASE P.after_rename.body code 0 result_hex 72657475726e202478
CASE P.after_rename.default code 0 result_hex 31204f524947494e414c
CASE P.after_rename.invoke code 0 result_hex 4f524947494e414c
CASE P.source_call code 0 result_hex 4f524947494e414c
CASE P.vacated_call code 0 result_hex 56414341544544
CASE setup.R code 0 result_hex {}
CASE R.before.origin code 0 result_hex 3a3a523a3a6e303a3a70
CASE R.before.args code 0 result_hex 78
CASE R.before.body code 0 result_hex 72657475726e202478
CASE R.before.default code 0 result_hex 31204f524947494e414c
CASE R.before.invoke code 0 result_hex 4f524947494e414c
CASE R.middle_replacement code 0 result_hex {}
CASE R.after_middle_replacement.origin code 0 result_hex 3a3a523a3a6e3130303a3a70
CASE R.after_middle_replacement.args code 0 result_hex 78
CASE R.after_middle_replacement.body code 0 result_hex 72657475726e202478
CASE R.after_middle_replacement.default code 0 result_hex 31205245504c41434544
CASE R.after_middle_replacement.invoke code 0 result_hex 5245504c41434544
CASE setup.D code 0 result_hex {}
CASE D.before.origin code 0 result_hex 3a3a443a3a6e303a3a70
CASE D.before.args code 0 result_hex 78
CASE D.before.body code 0 result_hex 72657475726e202478
CASE D.before.default code 0 result_hex 31204f524947494e414c
CASE D.before.invoke code 0 result_hex 4f524947494e414c
CASE D.delete_source code 0 result_hex {}
CASE D.after_delete.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_delete.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.recreate_source code 0 result_hex {}
CASE D.after_recreate.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_recreate.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE setup.E code 0 result_hex {}
CASE E.before63.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before63.exists code 0 result_hex 31
CASE E.before63.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before63.args code 1 result_hex 223a3a453a3a6e36333a3a656e73222069736e277420612070726f636564757265
CASE E.before63.invoke code 0 result_hex 454e53454d424c45
CASE E.before64.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before64.exists code 0 result_hex 31
CASE E.before64.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before64.args code 1 result_hex 223a3a453a3a6e36343a3a656e73222069736e277420612070726f636564757265
CASE E.before64.invoke code 0 result_hex 454e53454d424c45
CASE E.before65.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before65.exists code 0 result_hex 31
CASE E.before65.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before65.args code 1 result_hex 223a3a453a3a6e36353a3a656e73222069736e277420612070726f636564757265
CASE E.before65.invoke code 0 result_hex 454e53454d424c45
CASE E.before200.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before200.exists code 0 result_hex 31
CASE E.before200.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before200.args code 1 result_hex 223a3a453a3a6e3230303a3a656e73222069736e277420612070726f636564757265
CASE E.before200.invoke code 0 result_hex 454e53454d424c45
CASE E.configure_import code 0 result_hex {}
CASE E.source_map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.import_map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.rename_source code 0 result_hex {}
CASE E.vacated_replacement code 0 result_hex {}
CASE E.after_rename.origin code 0 result_hex 3a3a453a3a6e303a3a6d6f766564
CASE E.after_rename.exists code 0 result_hex 31
CASE E.after_rename.map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.after_rename.invoke code 0 result_hex 454e53454d424c45
CASE E.vacated_call code 0 result_hex 564143415445445f454e53454d424c45
CLOSED_DEEP_IMPORT_CHAIN_200
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries
- Exit status: Not recorded
- Standard output:

```text-visible
CASE version.patchlevel code 0 result_hex 392e302e34
CASE version.executable code 0 result_hex 2f776f726b73706163652f74636c2d6c73702f746d702f74636c392e302e342f756e69782f74636c7368
CASE helper.namespace_origin code 1 result_hex 226e616d657370616365206f726967696e222069736e277420612070726f636564757265
CASE helper.namespace_import code 1 result_hex 226e616d65737061636520696d706f7274222069736e277420612070726f636564757265
CASE setup.P code 0 result_hex {}
CASE P.before63.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before63.args code 0 result_hex 78
CASE P.before63.body code 0 result_hex 72657475726e202478
CASE P.before63.default code 0 result_hex 31204f524947494e414c
CASE P.before63.invoke code 0 result_hex 4f524947494e414c
CASE P.before64.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before64.args code 0 result_hex 78
CASE P.before64.body code 0 result_hex 72657475726e202478
CASE P.before64.default code 0 result_hex 31204f524947494e414c
CASE P.before64.invoke code 0 result_hex 4f524947494e414c
CASE P.before65.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before65.args code 0 result_hex 78
CASE P.before65.body code 0 result_hex 72657475726e202478
CASE P.before65.default code 0 result_hex 31204f524947494e414c
CASE P.before65.invoke code 0 result_hex 4f524947494e414c
CASE P.before200.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before200.args code 0 result_hex 78
CASE P.before200.body code 0 result_hex 72657475726e202478
CASE P.before200.default code 0 result_hex 31204f524947494e414c
CASE P.before200.invoke code 0 result_hex 4f524947494e414c
CASE P.rename code 0 result_hex {}
CASE P.vacated_replacement code 0 result_hex {}
CASE P.after_rename.origin code 0 result_hex 3a3a503a3a6e303a3a6d6f766564
CASE P.after_rename.args code 0 result_hex 78
CASE P.after_rename.body code 0 result_hex 72657475726e202478
CASE P.after_rename.default code 0 result_hex 31204f524947494e414c
CASE P.after_rename.invoke code 0 result_hex 4f524947494e414c
CASE P.source_call code 0 result_hex 4f524947494e414c
CASE P.vacated_call code 0 result_hex 56414341544544
CASE setup.R code 0 result_hex {}
CASE R.before.origin code 0 result_hex 3a3a523a3a6e303a3a70
CASE R.before.args code 0 result_hex 78
CASE R.before.body code 0 result_hex 72657475726e202478
CASE R.before.default code 0 result_hex 31204f524947494e414c
CASE R.before.invoke code 0 result_hex 4f524947494e414c
CASE R.middle_replacement code 0 result_hex {}
CASE R.after_middle_replacement.origin code 0 result_hex 3a3a523a3a6e3130303a3a70
CASE R.after_middle_replacement.args code 0 result_hex 78
CASE R.after_middle_replacement.body code 0 result_hex 72657475726e202478
CASE R.after_middle_replacement.default code 0 result_hex 31205245504c41434544
CASE R.after_middle_replacement.invoke code 0 result_hex 5245504c41434544
CASE setup.D code 0 result_hex {}
CASE D.before.origin code 0 result_hex 3a3a443a3a6e303a3a70
CASE D.before.args code 0 result_hex 78
CASE D.before.body code 0 result_hex 72657475726e202478
CASE D.before.default code 0 result_hex 31204f524947494e414c
CASE D.before.invoke code 0 result_hex 4f524947494e414c
CASE D.delete_source code 0 result_hex {}
CASE D.after_delete.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_delete.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.recreate_source code 0 result_hex {}
CASE D.after_recreate.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_recreate.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE setup.E code 0 result_hex {}
CASE E.before63.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before63.exists code 0 result_hex 31
CASE E.before63.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before63.args code 1 result_hex 223a3a453a3a6e36333a3a656e73222069736e277420612070726f636564757265
CASE E.before63.invoke code 0 result_hex 454e53454d424c45
CASE E.before64.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before64.exists code 0 result_hex 31
CASE E.before64.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before64.args code 1 result_hex 223a3a453a3a6e36343a3a656e73222069736e277420612070726f636564757265
CASE E.before64.invoke code 0 result_hex 454e53454d424c45
CASE E.before65.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before65.exists code 0 result_hex 31
CASE E.before65.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before65.args code 1 result_hex 223a3a453a3a6e36353a3a656e73222069736e277420612070726f636564757265
CASE E.before65.invoke code 0 result_hex 454e53454d424c45
CASE E.before200.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before200.exists code 0 result_hex 31
CASE E.before200.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before200.args code 1 result_hex 223a3a453a3a6e3230303a3a656e73222069736e277420612070726f636564757265
CASE E.before200.invoke code 0 result_hex 454e53454d424c45
CASE E.configure_import code 0 result_hex {}
CASE E.source_map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.import_map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.rename_source code 0 result_hex {}
CASE E.vacated_replacement code 0 result_hex {}
CASE E.after_rename.origin code 0 result_hex 3a3a453a3a6e303a3a6d6f766564
CASE E.after_rename.exists code 0 result_hex 31
CASE E.after_rename.map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.after_rename.invoke code 0 result_hex 454e53454d424c45
CASE E.vacated_call code 0 result_hex 564143415445445f454e53454d424c45
CLOSED_DEEP_IMPORT_CHAIN_200
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries
- Exit status: Not recorded
- Standard output:

```text-visible
CASE version.patchlevel code 0 result_hex 392e312e30
CASE version.executable code 0 result_hex 2f776f726b73706163652f74636c2d6c73702f746d702f74636c392e312e302f756e69782f74636c7368
CASE helper.namespace_origin code 1 result_hex 226e616d657370616365206f726967696e222069736e277420612070726f636564757265
CASE helper.namespace_import code 1 result_hex 226e616d65737061636520696d706f7274222069736e277420612070726f636564757265
CASE setup.P code 0 result_hex {}
CASE P.before63.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before63.args code 0 result_hex 78
CASE P.before63.body code 0 result_hex 72657475726e202478
CASE P.before63.default code 0 result_hex 31204f524947494e414c
CASE P.before63.invoke code 0 result_hex 4f524947494e414c
CASE P.before64.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before64.args code 0 result_hex 78
CASE P.before64.body code 0 result_hex 72657475726e202478
CASE P.before64.default code 0 result_hex 31204f524947494e414c
CASE P.before64.invoke code 0 result_hex 4f524947494e414c
CASE P.before65.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before65.args code 0 result_hex 78
CASE P.before65.body code 0 result_hex 72657475726e202478
CASE P.before65.default code 0 result_hex 31204f524947494e414c
CASE P.before65.invoke code 0 result_hex 4f524947494e414c
CASE P.before200.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before200.args code 0 result_hex 78
CASE P.before200.body code 0 result_hex 72657475726e202478
CASE P.before200.default code 0 result_hex 31204f524947494e414c
CASE P.before200.invoke code 0 result_hex 4f524947494e414c
CASE P.rename code 0 result_hex {}
CASE P.vacated_replacement code 0 result_hex {}
CASE P.after_rename.origin code 0 result_hex 3a3a503a3a6e303a3a6d6f766564
CASE P.after_rename.args code 0 result_hex 78
CASE P.after_rename.body code 0 result_hex 72657475726e202478
CASE P.after_rename.default code 0 result_hex 31204f524947494e414c
CASE P.after_rename.invoke code 0 result_hex 4f524947494e414c
CASE P.source_call code 0 result_hex 4f524947494e414c
CASE P.vacated_call code 0 result_hex 56414341544544
CASE setup.R code 0 result_hex {}
CASE R.before.origin code 0 result_hex 3a3a523a3a6e303a3a70
CASE R.before.args code 0 result_hex 78
CASE R.before.body code 0 result_hex 72657475726e202478
CASE R.before.default code 0 result_hex 31204f524947494e414c
CASE R.before.invoke code 0 result_hex 4f524947494e414c
CASE R.middle_replacement code 0 result_hex {}
CASE R.after_middle_replacement.origin code 0 result_hex 3a3a523a3a6e3130303a3a70
CASE R.after_middle_replacement.args code 0 result_hex 78
CASE R.after_middle_replacement.body code 0 result_hex 72657475726e202478
CASE R.after_middle_replacement.default code 0 result_hex 31205245504c41434544
CASE R.after_middle_replacement.invoke code 0 result_hex 5245504c41434544
CASE setup.D code 0 result_hex {}
CASE D.before.origin code 0 result_hex 3a3a443a3a6e303a3a70
CASE D.before.args code 0 result_hex 78
CASE D.before.body code 0 result_hex 72657475726e202478
CASE D.before.default code 0 result_hex 31204f524947494e414c
CASE D.before.invoke code 0 result_hex 4f524947494e414c
CASE D.delete_source code 0 result_hex {}
CASE D.after_delete.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_delete.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_delete.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.recreate_source code 0 result_hex {}
CASE D.after_recreate.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE D.after_recreate.args code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.body code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.default code 1 result_hex 223a3a443a3a6e3230303a3a70222069736e277420612070726f636564757265
CASE D.after_recreate.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a443a3a6e3230303a3a7022
CASE setup.E code 0 result_hex {}
CASE E.before63.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before63.exists code 0 result_hex 31
CASE E.before63.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before63.args code 1 result_hex 223a3a453a3a6e36333a3a656e73222069736e277420612070726f636564757265
CASE E.before63.invoke code 0 result_hex 454e53454d424c45
CASE E.before64.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before64.exists code 0 result_hex 31
CASE E.before64.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before64.args code 1 result_hex 223a3a453a3a6e36343a3a656e73222069736e277420612070726f636564757265
CASE E.before64.invoke code 0 result_hex 454e53454d424c45
CASE E.before65.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before65.exists code 0 result_hex 31
CASE E.before65.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before65.args code 1 result_hex 223a3a453a3a6e36353a3a656e73222069736e277420612070726f636564757265
CASE E.before65.invoke code 0 result_hex 454e53454d424c45
CASE E.before200.origin code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.before200.exists code 0 result_hex 31
CASE E.before200.map code 0 result_hex 676f203a3a453a3a6e303a3a746172676574
CASE E.before200.args code 1 result_hex 223a3a453a3a6e3230303a3a656e73222069736e277420612070726f636564757265
CASE E.before200.invoke code 0 result_hex 454e53454d424c45
CASE E.configure_import code 0 result_hex {}
CASE E.source_map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.import_map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.rename_source code 0 result_hex {}
CASE E.vacated_replacement code 0 result_hex {}
CASE E.after_rename.origin code 0 result_hex 3a3a453a3a6e303a3a6d6f766564
CASE E.after_rename.exists code 0 result_hex 31
CASE E.after_rename.map code 0 result_hex 6368616e676564203a3a453a3a6e303a3a746172676574
CASE E.after_rename.invoke code 0 result_hex 454e53454d424c45
CASE E.vacated_call code 0 result_hex 564143415445445f454e53454d424c45
CLOSED_DEEP_IMPORT_CHAIN_200
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries
- Exit status: Not recorded
- Standard output:

```text-visible
CASE version.patchlevel code 0 result_hex 302e38342d392d6735626163376339
CASE version.executable code 0 result_hex 2f776f726b73706163652f2e70726f6f66732f6e61746976652d70726f7669646572732f6a696d74636c2f6a696d7368
CASE helper.namespace_origin code 0 result_hex 0a09736574206e7363616e6f6e205b75706c6576656c2031205b6c697374203a3a6e616d6573706163652063616e6f6e20246e616d655d5d0a096966207b5b657869737473202d616c69617320246e7363616e6f6e5d7d207b0a09097461696c63616c6c207b6e616d657370616365206f726967696e7d205b696e666f20616c69617320246e7363616e6f6e5d0a097d0a096966207b5b657869737473202d636f6d6d616e6420246e7363616e6f6e5d7d207b0a090972657475726e203a3a246e7363616e6f6e0a097d0a096966207b5b657869737473202d636f6d6d616e6420246e616d655d7d207b0a090972657475726e203a3a246e616d650a097d0a0a0972657475726e202d636f6465206572726f722022696e76616c696420636f6d6d616e64206e616d65205c22246e616d655c22220a
CASE helper.namespace_import code 0 result_hex 0a097365742063757272656e74205b75706c6576656c2031203a3a6e616d6573706163652063616e6f6e5d0a0a09666f7265616368207061747465726e202461726773207b0a0909666f726561636820636d64205b696e666f20636f6d6d616e6473205b6e616d6573706163652063616e6f6e202463757272656e7420247061747465726e5d5d207b0a0909096966207b5b6e616d657370616365207175616c6966696572732024636d645d206571202463757272656e747d207b0a0909090972657475726e202d636f6465206572726f722022696d706f7274207061747465726e205c22247061747465726e5c2220747269657320746f20696d706f72742066726f6d206e616d657370616365205c222463757272656e745c2220696e746f20697473656c66220a0909097d0a0a0a090909736574206e6577636d6420247b63757272656e747d3a3a5b6e616d657370616365207461696c2024636d645d0a0a09090973657420616c6961732024636d640a0909097768696c65207b5b657869737473202d616c6961732024616c6961735d7d207b0a0909090973657420616c696173205b696e666f20616c6961732024616c6961735d0a090909096966207b24616c69617320657120246e6577636d647d207b0a090909090972657475726e202d636f6465206572726f722022696d706f7274207061747465726e205c22247061747465726e5c2220776f756c64206372656174652061206c6f6f70220a090909097d0a0909097d0a0a090909616c69617320246e6577636d642024636d640a09097d0a097d0a
CASE setup.P code 0 result_hex {}
CASE P.before63.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before63.args code 1 result_hex 636f6d6d616e6420223a3a503a3a6e36333a3a7022206973206e6f7420612070726f636564757265
CASE P.before63.body code 1 result_hex 636f6d6d616e6420223a3a503a3a6e36333a3a7022206973206e6f7420612070726f636564757265
CASE P.before63.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE P.before63.invoke code 0 result_hex 4f524947494e414c
CASE P.before64.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before64.args code 1 result_hex 636f6d6d616e6420223a3a503a3a6e36343a3a7022206973206e6f7420612070726f636564757265
CASE P.before64.body code 1 result_hex 636f6d6d616e6420223a3a503a3a6e36343a3a7022206973206e6f7420612070726f636564757265
CASE P.before64.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE P.before64.invoke code 0 result_hex 4f524947494e414c
CASE P.before65.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before65.args code 1 result_hex 636f6d6d616e6420223a3a503a3a6e36353a3a7022206973206e6f7420612070726f636564757265
CASE P.before65.body code 1 result_hex 636f6d6d616e6420223a3a503a3a6e36353a3a7022206973206e6f7420612070726f636564757265
CASE P.before65.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE P.before65.invoke code 0 result_hex 4f524947494e414c
CASE P.before200.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.before200.args code 1 result_hex 636f6d6d616e6420223a3a503a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE P.before200.body code 1 result_hex 636f6d6d616e6420223a3a503a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE P.before200.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE P.before200.invoke code 0 result_hex 4f524947494e414c
CASE P.rename code 0 result_hex {}
CASE P.vacated_replacement code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.after_rename.origin code 0 result_hex 3a3a503a3a6e303a3a70
CASE P.after_rename.args code 1 result_hex 636f6d6d616e6420223a3a503a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE P.after_rename.body code 1 result_hex 636f6d6d616e6420223a3a503a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE P.after_rename.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE P.after_rename.invoke code 0 result_hex 56414341544544
CASE P.source_call code 0 result_hex 4f524947494e414c
CASE P.vacated_call code 0 result_hex 56414341544544
CASE setup.R code 0 result_hex {}
CASE R.before.origin code 0 result_hex 3a3a523a3a6e303a3a70
CASE R.before.args code 1 result_hex 636f6d6d616e6420223a3a523a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE R.before.body code 1 result_hex 636f6d6d616e6420223a3a523a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE R.before.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE R.before.invoke code 0 result_hex 4f524947494e414c
CASE R.middle_replacement code 0 result_hex 3a3a523a3a6e3130303a3a70
CASE R.after_middle_replacement.origin code 0 result_hex 3a3a523a3a6e3130303a3a70
CASE R.after_middle_replacement.args code 1 result_hex 636f6d6d616e6420223a3a523a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE R.after_middle_replacement.body code 1 result_hex 636f6d6d616e6420223a3a523a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE R.after_middle_replacement.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE R.after_middle_replacement.invoke code 0 result_hex 5245504c41434544
CASE setup.D code 0 result_hex {}
CASE D.before.origin code 0 result_hex 3a3a443a3a6e303a3a70
CASE D.before.args code 1 result_hex 636f6d6d616e6420223a3a443a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE D.before.body code 1 result_hex 636f6d6d616e6420223a3a443a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE D.before.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE D.before.invoke code 0 result_hex 4f524947494e414c
CASE D.delete_source code 0 result_hex {}
CASE D.after_delete.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d652022443a3a6e303a3a7022
CASE D.after_delete.args code 1 result_hex 636f6d6d616e6420223a3a443a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE D.after_delete.body code 1 result_hex 636f6d6d616e6420223a3a443a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE D.after_delete.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE D.after_delete.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d652022443a3a6e303a3a7022
CASE D.recreate_source code 0 result_hex 3a3a443a3a6e303a3a70
CASE D.after_recreate.origin code 0 result_hex 3a3a443a3a6e303a3a70
CASE D.after_recreate.args code 1 result_hex 636f6d6d616e6420223a3a443a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE D.after_recreate.body code 1 result_hex 636f6d6d616e6420223a3a443a3a6e3230303a3a7022206973206e6f7420612070726f636564757265
CASE D.after_recreate.default code 1 result_hex 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
CASE D.after_recreate.invoke code 0 result_hex 524543524541544544
CASE setup.E code 1 result_hex 77726f6e67202320617267733a2073686f756c642062652022656e73656d626c6520636f6d6d616e64203f2d6175746f6d6170207072656669783f22
CASE E.before63.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36333a3a656e7322
CASE E.before63.exists code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.before63.map code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.before63.args code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36333a3a656e7322
CASE E.before63.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36333a3a656e7322
CASE E.before64.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36343a3a656e7322
CASE E.before64.exists code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.before64.map code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.before64.args code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36343a3a656e7322
CASE E.before64.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36343a3a656e7322
CASE E.before65.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36353a3a656e7322
CASE E.before65.exists code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.before65.map code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.before65.args code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36353a3a656e7322
CASE E.before65.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e36353a3a656e7322
CASE E.before200.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.before200.exists code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.before200.map code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.before200.args code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.before200.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.configure_import code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.source_map code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.import_map code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.rename_source code 1 result_hex 63616e27742072656e616d6520223a3a453a3a6e303a3a656e73223a20636f6d6d616e6420646f65736e2774206578697374
CASE E.vacated_replacement code 0 result_hex 3a3a453a3a6e303a3a656e73
CASE E.after_rename.origin code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.after_rename.exists code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.after_rename.map code 1 result_hex 6f6e6c79205b6e616d65737061636520656e73656d626c65206372656174655d20697320737570706f72746564
CASE E.after_rename.invoke code 1 result_hex 696e76616c696420636f6d6d616e64206e616d6520223a3a453a3a6e3230303a3a656e7322
CASE E.vacated_call code 0 result_hex 564143415445445f454e53454d424c45
CLOSED_DEEP_IMPORT_CHAIN_200
```

- Standard error: empty.
