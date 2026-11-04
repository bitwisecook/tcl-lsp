# Tcl execution evidence

## Question

> Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

This record addresses only the case described below; it does not answer every part of the broader question. The distinction informs name resolution and diagnostics in the LSP, compiler and VM.

> Related question F5-N07; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

> What completion and result did retained variant capability-runtime-control-v1 of case import-forget-global-origin produce in stock-8.4, run nh20261012broad1, unit None, invocation 'once'?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Only retained case import-forget-global-origin, variant capability-runtime-control-v1, run nh20261012broad1, context stock-8.4; event None, unit None, invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: NH16 adapted names; fresh interpreter only, global export state changes; clean ::__tcl_lsp_probe_nh20261012broad10_import_forget_global_origin_source. Child-interpreter adaptation. Authored/generated source descriptor is the retained case metadata, not an unchanged portable Tcl program. This corpus receipt does not print Tcl patchlevel; see separate cap2 binary-path/version control. . Interpreter/action facts retained in the measurement: {"interpreter_path": "/usr/bin/tclsh8.4", "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt.

## Script

```tcl-visible
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 736574206e685f6672657368205b696e74657270206372656174655d]
set nh_operation [binary format H* 696e74657270206576616c20246e685f6672657368205b62696e61727920666f726d617420482a20366536313664363537333730363136333635323036353736363136633230336133613566356637343633366335663663373337303566373037323666363236353566366536383332333033323336333133303331333236323732366636313634333133303566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665323037623764306137303732366636333230336133613566356637343633366335663663373337303566373037323666363236353566366536383332333033323336333133303331333236323732366636313634333133303566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665356637333666373537323633363532303762376432303762373236353734373537323665323034373463346634323431346335663439346435303466353235343764306136653631366436353733373036313633363532303635373636313663323033613361323037623665363136643635373337303631363336353230363537383730366637323734323035663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332363237323666363136343331333035663639366437303666373237343566363636663732363736353734356636373663366636323631366335663666373236393637363936653566373336663735373236333635376430613665363136643635373337303631363336353230363537363631366332303361336135663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332363237323666363136343331333035663639366437303666373237343566363636663732363736353734356636373663366636323631366335663666373236393637363936653230376236653631366436353733373036313633363532303639366437303666373237343230336133613566356637343633366335663663373337303566373037323666363236353566366536383332333033323336333133303331333236323732366636313634333133303566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665356637333666373537323633363533623230373336353734323036323635363636663732363532303562356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323632373236663631363433313330356636393664373036663732373435663636366637323637363537343566363736633666363236313663356636663732363936373639366535663733366637353732363336353564376430613665363136643635373337303631363336353230363537363631366332303361336135663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332363237323666363136343331333035663639366437303666373237343566363636663732363736353734356636373663366636323631366335663666373236393637363936653230376236653631366436353733373036313633363532303636366637323637363537343230336133613566356637343633366335663663373337303566373037323666363236353566366536383332333033323336333133303331333236323732366636313634333133303566363936643730366637323734356636363666373236373635373435663637366336663632363136633566366637323639363736393665356637333666373537323633363533623230366336393733373432303234363236353636366637323635323035623665363136643635373337303631363336353230373736383639363336383230356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323632373236663631363433313330356636393664373036663732373435663636366637323637363537343566363736633666363236313663356636663732363936373639366535663733366637353732363336353564323035623639366536363666323036333666366436643631366536343733323033613361356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323632373236663631363433313330356636393664373036663732373435663636366637323637363537343566363736633666363236313663356636663732363936373639366533613361356635663734363336633566366337333730356637303732366636323635356636653638333233303332333633313330333133323632373236663631363433313330356636393664373036663732373435663636366637323637363537343566363736633666363236313663356636663732363936373639366535663733366637353732363336353564376430615d]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_setup_code != 1} {continue}
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
        puts "NHO|nh20261012broad1|import-forget-global-origin|stock-8.4|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        puts "NHO|nh20261012broad1|import-forget-global-origin|stock-8.4|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        puts "NHO|nh20261012broad1|import-forget-global-origin|stock-8.4|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### bigip

- Recorded version: 21.1.0.1
- Recorded input channel: interpreter path: /usr/bin/tclsh8.4
- Exit status: Not recorded
- Standard output:

```text-visible
NHO|nh20261012broad1|import-forget-global-origin|stock-8.4|setup|0|none|once|result|utf-8|0|1|696e7465727030
NHO|nh20261012broad1|import-forget-global-origin|stock-8.4|lookup|0|none|once|result|utf-8|0|1|474c4f42414c5f494d504f5254203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f75726365207b7d
NHO|nh20261012broad1|import-forget-global-origin|stock-8.4|runtime-cleanup|0|none|once|result|utf-8|0|1|
```

- Standard error:

```text-visible
** WARNING: connection is not using a post-quantum key exchange algorithm.⟦CRLF⟧
** This session may be vulnerable to "store now, decrypt later" attacks.⟦CRLF⟧
** The server may need to be upgraded. See https://openssh.com/pq.html⟦CRLF⟧
```
