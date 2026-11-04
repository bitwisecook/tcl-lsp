# BIG-IP APL callback: import-forget-global-origin

## Question

Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

This callback addresses only the `import-forget-global-origin` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: NH16 adapted names; fresh interpreter only, global export state changes; clean ::__tcl_lsp_probe_nh20261012aplbroad1_import_forget_global_origin_source. Child-interpreter adaptation.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

```tcl-visible
set nh_apl_rows {}
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 736574206e685f6672657368205b696e74657270206372656174655d]
set nh_operation [binary format H* 696e74657270206576616c20246e685f6672657368205b62696e61727920666f726d617420482a203665363136643635373337303631363336353230363537363631366332303361336135663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332363137303663363237323666363136343331356636393664373036663732373435663636366637323637363537343566363736633666363236313663356636663732363936373639366532303762376430613730373236663633323033613361356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323631373036633632373236663631363433313566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665356637333666373537323633363532303762376432303762373236353734373537323665323034373463346634323431346335663439346435303466353235343764306136653631366436353733373036313633363532303635373636313663323033613361323037623665363136643635373337303631363336353230363537383730366637323734323035663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332363137303663363237323666363136343331356636393664373036663732373435663636366637323637363537343566363736633666363236313663356636663732363936373639366535663733366637353732363336353764306136653631366436353733373036313633363532303635373636313663323033613361356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323631373036633632373236663631363433313566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665323037623665363136643635373337303631363336353230363936643730366637323734323033613361356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323631373036633632373236663631363433313566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665356637333666373537323633363533623230373336353734323036323635363636663732363532303562356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323631373036633632373236663631363433313566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665356637333666373537323633363535643764306136653631366436353733373036313633363532303635373636313663323033613361356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323631373036633632373236663631363433313566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665323037623665363136643635373337303631363336353230363636663732363736353734323033613361356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323631373036633632373236663631363433313566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665356637333666373537323633363533623230366336393733373432303234363236353636366637323635323035623665363136643635373337303631363336353230373736383639363336383230356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323631373036633632373236663631363433313566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665356637333666373537323633363535643230356236393665363636663230363336663664366436313665363437333230336133613566356637343633366335663663373337303566373037323666363236353566366536383332333033323336333133303331333236313730366336323732366636313634333135663639366437303666373237343566363636663732363736353734356636373663366636323631366335663666373236393637363936653361336135663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332363137303663363237323666363136343331356636393664373036663732373435663636366637323637363537343566363736633666363236313663356636663732363936373639366535663733366637353732363336353564376430615d]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field == "returnoptions" && $nh_options_available == "unavailable"} {continue}
    if {$nh_field != "result" && $nh_field != "returnoptions" && $nh_setup_code != 1} {continue}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {[catch {$nh_encoder convertto utf-8 $nh_value} nh_encoded]} {
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {([string length $nh_hex] + 399) / 400}]
    if {$nh_chunks == 0} {set nh_chunks 1}
    for {set nh_i 0} {$nh_i < $nh_chunks} {incr nh_i} {
        set nh_part [string range $nh_hex [expr {$nh_i * 400}] [expr {$nh_i * 400 + 399}]]
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|import-forget-global-origin|iapp-apl|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
if {$nh_setup_code == 0} {
set nh_code [catch {eval $nh_operation} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field == "returnoptions" && $nh_options_available == "unavailable"} {continue}
    if {$nh_field != "result" && $nh_field != "returnoptions" && $nh_code != 1} {continue}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {[catch {$nh_encoder convertto utf-8 $nh_value} nh_encoded]} {
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {([string length $nh_hex] + 399) / 400}]
    if {$nh_chunks == 0} {set nh_chunks 1}
    for {set nh_i 0} {$nh_i < $nh_chunks} {incr nh_i} {
        set nh_part [string range $nh_hex [expr {$nh_i * 400}] [expr {$nh_i * 400 + 399}]]
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|import-forget-global-origin|iapp-apl|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 696e746572702064656c65746520246e685f6672657368]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field == "returnoptions" && $nh_options_available == "unavailable"} {continue}
    if {$nh_field != "result" && $nh_field != "returnoptions" && $nh_code != 1} {continue}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {[catch {$nh_encoder convertto utf-8 $nh_value} nh_encoded]} {
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {([string length $nh_hex] + 399) / 400}]
    if {$nh_chunks == 0} {set nh_chunks 1}
    for {set nh_i 0} {$nh_i < $nh_chunks} {incr nh_i} {
        set nh_part [string range $nh_hex [expr {$nh_i * 400}] [expr {$nh_i * 400 + 399}]]
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|import-forget-global-origin|iapp-apl|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}

return [join $nh_apl_rows "\n"]
```

## Results

### BIG-IP iApp APL

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.4.6.
- Recorded input channel: browser Application Services template choice callback.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012aplbroad1|import-forget-global-origin|iapp-apl|setup|0|none|once|result|utf-8|0|1|696e7465727030
NHO|nh20261012aplbroad1|import-forget-global-origin|iapp-apl|lookup|0|none|once|result|utf-8|0|1|474c4f42414c5f494d504f5254203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f75726365207b7d
NHO|nh20261012aplbroad1|import-forget-global-origin|iapp-apl|runtime-cleanup|0|none|once|result|utf-8|0|1|
```

These are the retained DOM option values projected one row per value, not process stdout. The fields below decode their hexadecimal payloads; completion codes are the callback's caught Tcl codes.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
interp0⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
GLOBAL_IMPORT ::__tcl_lsp_probe_nh20261012aplbroad1_import_forget_global_origin_source {}⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
