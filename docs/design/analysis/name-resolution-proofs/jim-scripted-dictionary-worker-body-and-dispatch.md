# naming.jim.scripted-dictionary-worker-body-and-dispatch

Kind: `native-observation`

## Problem statement

Jim installs real multiword dictionary procedures with reference formals and body-level upvar/uplevel behavior. Treating them as a C dict opcode would lose both original body ownership and argument routing.

## Question

What installed body/formal bytes and original dispatch/body completions do the seven Jim scripted dictionary workers expose?

## Conclusion

Fourteen native body/formal results and thirteen executable programs remain distinct. The original stdlib source owns these workers and their reference/upvar semantics. Selector abbreviation and wrong-arity/body outcomes retain their exact bytes; they grant no C dictionary opcode or physical header claim.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.5.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.6.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl9.0.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl9.1.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### jim

Status: `observed`. Version: Jim 0.84 pinned native build. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-13-row-14`):

```json
{
  "engine": "Jim 0.84 pinned native build",
  "observations": [
    {
      "id": "update-body",
      "result_hex": "0a09736574206b657973207b7d0a09666f7265616368207b6e20767d202461726773207b0a09097570766172202476207661725f24760a09096966207b5b646963742065786973747320247661724e616d6520246e5d7d207b0a090909736574207661725f2476205b646963742067657420247661724e616d6520246e5d0a09097d0a097d0a096361746368207b75706c6576656c203120247363726970747d206d7367206f7074730a096966207b5b696e666f20657869737473207661724e616d655d7d207b0a0909666f7265616368207b6e20767d202461726773207b0a0909096966207b5b696e666f20657869737473207661725f24765d7d207b0a090909096469637420736574207661724e616d6520246e205b736574207661725f24765d0a0909097d20656c7365207b0a090909096469637420756e736574207661724e616d6520246e0a0909097d0a09097d0a097d0a0972657475726e207b2a7d246f70747320246d73670a"
    },
    {
      "id": "update-args",
      "result_hex": "267661724e616d65206172677320736372697074"
    },
    {
      "id": "replace-body",
      "result_hex": "0a096966207b5b6c6c656e67746820247b6b65792076616c75657d5d202520327d207b0a09097461696c63616c6c207b64696374207265706c6163657d0a097d0a097461696c63616c6c2064696374206d65726765202464696374696f6e61727920247b6b65792076616c75657d0a"
    },
    {
      "id": "replace-args",
      "result_hex": "64696374696f6e617279207b61726773207b6b65792076616c75657d7d"
    },
    {
      "id": "lappend-body",
      "result_hex": "0a09757076617220247661724e616d6520646963740a096966207b5b65786973747320646963745d202626205b646963742065786973747320246469637420246b65795d7d207b0a0909736574206c697374205b646963742067657420246469637420246b65795d0a097d0a096c617070656e64206c697374207b2a7d2476616c75650a096469637420736574206469637420246b657920246c6973740a"
    },
    {
      "id": "lappend-args",
      "result_hex": "7661724e616d65206b6579207b617267732076616c75657d"
    },
    {
      "id": "append-body",
      "result_hex": "0a09757076617220247661724e616d6520646963740a096966207b5b65786973747320646963745d202626205b646963742065786973747320246469637420246b65795d7d207b0a090973657420737472205b646963742067657420246469637420246b65795d0a097d0a09617070656e6420737472207b2a7d2476616c75650a096469637420736574206469637420246b657920247374720a"
    },
    {
      "id": "append-args",
      "result_hex": "7661724e616d65206b6579207b617267732076616c75657d"
    },
    {
      "id": "incr-body",
      "result_hex": "0a09757076617220247661724e616d6520646963740a096966207b5b65786973747320646963745d202626205b646963742065786973747320246469637420246b65795d7d207b0a09097365742076616c7565205b646963742067657420246469637420246b65795d0a097d0a09696e63722076616c75652024696e6372656d656e740a096469637420736574206469637420246b6579202476616c75650a"
    },
    {
      "id": "incr-args",
      "result_hex": "7661724e616d65206b6579207b696e6372656d656e7420317d"
    },
    {
      "id": "remove-body",
      "result_hex": "0a09666f7265616368206b20246b6579207b0a09096469637420756e7365742064696374696f6e61727920246b0a097d0a0972657475726e202464696374696f6e6172790a"
    },
    {
      "id": "remove-args",
      "result_hex": "64696374696f6e617279207b61726773206b65797d"
    },
    {
      "id": "for-body",
      "result_hex": "0a096966207b5b6c6c656e6774682024766172735d20213d20327d207b0a090972657475726e202d636f6465206572726f7220226d75737420686176652065786163746c792074776f207661726961626c65206e616d6573220a097d0a09646963742073697a65202464696374696f6e6172790a097461696c63616c6c20666f7265616368202476617273202464696374696f6e61727920247363726970740a"
    },
    {
      "id": "for-args",
      "result_hex": "766172732064696374696f6e61727920736372697074"
    },
    {
      "id": "append",
      "result_hex": "6b204142"
    },
    {
      "id": "lappend",
      "result_hex": "6b207b4120427d"
    },
    {
      "id": "incr",
      "result_hex": "6b2032"
    },
    {
      "id": "remove",
      "result_hex": "622032"
    },
    {
      "id": "replace",
      "result_hex": "61203220622033"
    },
    {
      "id": "for",
      "result_hex": "61203120622032"
    },
    {
      "id": "update-missing",
      "result_hex": "7b616273656e74204b4545507d204b454550"
    },
    {
      "id": "update-return",
      "result_hex": "454e44"
    },
    {
      "id": "update-abbreviation",
      "result_hex": "696e76616c696420636f6d6d616e64206e616d6520226469637420757022"
    },
    {
      "id": "update-deletion",
      "result_hex": "696e76616c696420636f6d6d616e64206e616d652022646963742075706461746522"
    },
    {
      "id": "update-twoargs",
      "result_hex": "30"
    },
    {
      "id": "update-threeargs",
      "result_hex": "30"
    },
    {
      "id": "update-fourargs",
      "result_hex": "34"
    }
  ]
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/LICENSE.jim](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/LICENSE.jim). SHA-256 `419f1146334e57fb258d6c957d1f94d43455ec0d24b1b54c4447e5f61a353950`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/append-args.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/append-args.bin). SHA-256 `6ac0c490500306a76cf9826c9fc425d75d1270c7ab288cfffc4e5a7b269e5846`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/append-body.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/append-body.bin). SHA-256 `27739f349bd15179cafa8a870966b1b7523d8de36b0e652b5d191139e1d9268a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/append.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/append.tcl). SHA-256 `b19b4f450338618904606028e13b2fed8d33d0f163d2eba16edcf246290c998f`. Exact retained input/program bytes; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/for-args.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/for-args.bin). SHA-256 `1f0eee7cc6229b1166c4aee46757dce72bcf4f0121ed55eebd61fa929b6e5975`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/for-body.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/for-body.bin). SHA-256 `e28a9fadc1fa97c4b4f0aaab06f1b68e29b32c5565280292d28635954f9bbe8d`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/for.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/for.tcl). SHA-256 `b77914a45214d1e3be75d15864a20d0f4e42e613b4b86a62894f892f3f9705c8`. Exact retained input/program bytes; purpose is limited to this question.
- `file-7` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/incr-args.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/incr-args.bin). SHA-256 `514b43f32f38e5cc2ea0cb16e3847731535af458040136d33c5e8cd1629d309c`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-8` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/incr-body.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/incr-body.bin). SHA-256 `8e10df32ff76122cdeb30b3d43c2eacd14739bc12c26d8ed7bc40bd6d7f15395`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-9` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/incr.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/incr.tcl). SHA-256 `abdef0dde8c1e61b44d0d74488004e96314cf0f42e21fd6cd52643c09deac86c`. Exact retained input/program bytes; purpose is limited to this question.
- `file-10` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/lappend-args.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/lappend-args.bin). SHA-256 `6ac0c490500306a76cf9826c9fc425d75d1270c7ab288cfffc4e5a7b269e5846`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-11` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/lappend-body.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/lappend-body.bin). SHA-256 `3d8614803a7d15f464c7c3da933bf691c6c23c674d19ad4a65ec886fb41b08e5`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-12` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/lappend.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/lappend.tcl). SHA-256 `7b652e02e0395b070b6aa8e7283de831bd7b1f8b32df020d8fbb0cefeafd9bae`. Exact retained input/program bytes; purpose is limited to this question.
- `file-13` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/manifest.json](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/manifest.json). SHA-256 `f9aba2365d673040c2f615e40bc9940a448bb16e26815e7c4ad1be1b633e1796`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-13-row-14` (provider): [rust/tcl-vm/tests/data/native_jim_dictionary/manifest.json](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/manifest.json). SHA-256 `f9aba2365d673040c2f615e40bc9940a448bb16e26815e7c4ad1be1b633e1796`. JSON pointer ``. Original provider/capture association at its exact selected row.
- `file-15` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/observations.tsv](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/observations.tsv). SHA-256 `07e0fdfe33691f4789ccd35e6f2f6c884a875d12be122368a63c59c8e3247db1`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-16` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/remove-args.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/remove-args.bin). SHA-256 `476d6a1a167c2f43dc9d94e748e3775ee63d85d2fb831c32ed3ffeba46390f47`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-17` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/remove-body.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/remove-body.bin). SHA-256 `3d6affc5f2c5ccf06b5e88cc38632bfe696bca19b62dd74898fded335df245b5`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-18` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/remove.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/remove.tcl). SHA-256 `97d241e9c8e506a06bc6ba060d2d56f357802c4fd093142c77bc0d49cf673852`. Exact retained input/program bytes; purpose is limited to this question.
- `file-19` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/replace-args.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/replace-args.bin). SHA-256 `3e215d161f96b0c2a9ac1b1bcca348e1a89a3051ff2a2184763d98a52730dffb`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-20` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/replace-body.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/replace-body.bin). SHA-256 `41bacb5909f3164993e05a68cb610c2adec3c390fe923090829f50718dda7998`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-21` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/replace.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/replace.tcl). SHA-256 `976cfb01f484e60a9ac62a31fa0f9288ff9e097464f74e34c5c9bbeab4f37723`. Exact retained input/program bytes; purpose is limited to this question.
- `file-22` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/stdlib.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/stdlib.tcl). SHA-256 `3597b76b785f697242a5354816792ad5fa538dd0a41b3973bdcbcb5edbb8893a`. Exact retained input/program bytes; purpose is limited to this question.
- `file-23` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/update-abbreviation.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-abbreviation.tcl). SHA-256 `ce6ac977a1117cf061043c95ddc9102e38b652e343c08324e5b6fd3b6c4a3810`. Exact retained input/program bytes; purpose is limited to this question.
- `file-24` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/update-args.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-args.bin). SHA-256 `1a4594c397bb0ca01e8f9d4aac7d796869bd100a8b5cb51479589602c979c97d`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-25` (observation): [rust/tcl-vm/tests/data/native_jim_dictionary/update-body.bin](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-body.bin). SHA-256 `aa88e73e4edb4127d857fd7bebba954e2c02bdb8b683ab118b4ec1ab1255ecfa`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-26` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/update-deletion.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-deletion.tcl). SHA-256 `034ef2a63caacc1f3127a99430239bb2eb942d31a69173ae4ec5b4c752204ddc`. Exact retained input/program bytes; purpose is limited to this question.
- `file-27` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/update-fourargs.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-fourargs.tcl). SHA-256 `c7bfc9f6b71fbff1b13d6bb41a7cb99c1bff3f91cd6656ce493d1c7cddb0eca5`. Exact retained input/program bytes; purpose is limited to this question.
- `file-28` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/update-missing.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-missing.tcl). SHA-256 `f68159468a8d21e442d244e341c3fd6e5ee40c84283de9661ccd82203ddf8473`. Exact retained input/program bytes; purpose is limited to this question.
- `file-29` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/update-return.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-return.tcl). SHA-256 `ed93eee9f258e6b2813aa7d056aee216fca0baad10710b8826fc300573ac40cc`. Exact retained input/program bytes; purpose is limited to this question.
- `file-30` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/update-threeargs.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-threeargs.tcl). SHA-256 `ca4c84ffe62e9a55d1e814bddd3975b642e58f52ca3b5eaf286991db2f11adf6`. Exact retained input/program bytes; purpose is limited to this question.
- `file-31` (input): [rust/tcl-vm/tests/data/native_jim_dictionary/update-twoargs.tcl](../../../../rust/tcl-vm/tests/data/native_jim_dictionary/update-twoargs.tcl). SHA-256 `8d2fbfb0afe844cd41d19546bfb63938f45ee02ccac231006fe3642bd0b6e164`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
