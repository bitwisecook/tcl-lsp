# naming.jim.scripted-dictionary-worker-body-and-dispatch

Kind: `native-observation`

## Problem statement

Jim installs real multiword dictionary procedures with reference formals and body-level upvar/uplevel behavior. Treating them as a C dict opcode would lose both original body ownership and argument routing.

## Question

What installed body/formal bytes and original dispatch/body completions do the seven Jim scripted dictionary workers expose?

## Conclusion

Fourteen native body/formal results and thirteen executable programs remain distinct. The original stdlib source owns these workers and their reference/upvar semantics. Selector abbreviation and wrong-arity/body outcomes retain their exact bytes; they grant no C dictionary opcode or physical header claim.

A marked VM definition binds seven genuine scripted dictionary body/formal pairs and an independently constructed backend parameter-header identity control.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

Exact captured public body/formal bytes do not expose the original Jim parameter object/header pointer. The additional identity equality belongs only to the selected backend declaration owner. No new original process or passing assertion, private dictionary worker/frame state, full distribution or generic Native handler admission is attached. Existing original public scripted-dictionary evidence remains unchanged.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: jim Jim 0.84 pinned native build.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-vm/src/native_fixture.rs](../../../../rust/tcl-vm/src/native_fixture.rs), `interpreter_with_dictionary_library`: Construct the actual selected software core/compiler and explicitly choose only Dictionary for distribution specimens; library choice is independent of core/metadata availability.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::install_scripted_library`: Retain the explicitly selected shared scripted-library source and genuine procedure declarations under the actual backend bootstrap, rather than creating public wrappers from catalogue labels.
- [rust/tcl-vm/src/cmd_dict/scripted_tests.rs](../../../../rust/tcl-vm/src/cmd_dict/scripted_tests.rs), `cmd_dict::scripted_tests::original_jim_dictionary_library_bodies_and_formals_are_real_procedures` (linked): The selected Jim software core/compiler explicitly loads Dictionary, then compares fourteen public body/formal byte windows for update/replace/lappend/append/incr/remove/for. Its info args result also equals this backend declaration's retained parameter header; that separate software identity assertion is not a native public pointer observation.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-vm/tests/data/native_jim_dictionary](../../../../rust/tcl-vm/tests/data/native_jim_dictionary)
