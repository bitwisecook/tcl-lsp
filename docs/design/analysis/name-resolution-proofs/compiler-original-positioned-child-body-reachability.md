# naming.compiler.original-positioned-child-body-reachability

Kind: `implementation-contract`

## Problem statement

Recursively splitting a decoded Body argument can assign nested command roles using a nominal head even after a represented replacement, move or alias. Possible call edges need each genuine child invocation and its retained source owner.

## Question

How does positioned interprocedural reachability retain possible child-body targets through original replacements, aliases and moves without rebuilding nested roles from decoded argument text?

## Conclusion

scan_positioned_reachability consumes the genuine parent SourceInvocationBinding and its invocation site, then queries the actual retained Realm::possible_entered_body_invocations inventory. Each child supplies its own source position, selected source/lookup context and possible execution target. The interprocedural inventory can add a possible local procedure edge from that child while independently withholding effects/purity and successful runtime entry. Decoded Body argument values cannot rebuild a nested command vocabulary or bypass known replacement/delete/refusal. The fixed original time/catch controls retain the stock child, authentic alias and represented rename, and refuse edges from original procedures that replace catch or a nominal held head. Module.source_metadata_input supplies the complete actual context for interprocedural transfer/May/operand metadata; a scalar profile supplies no missing child or handler proof.

## Scope

One fixed whole-source interprocedural control compares possible leaf reachability under stock, original procedure replacement, captured alias, original move and nominal procedure-head inputs. It verifies source ownership/target selection independently of runtime callback, entered frame, handler effects, Normal completion or caller argument values. All seven native providers remain not tested for this source/API question.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-compiler/src/interprocedural.rs](../../../../rust/tcl-compiler/src/interprocedural.rs), `scan_positioned_reachability`: Consume genuine positioned parent and each retained possible child invocation independently of purity/effects.
- [rust/tcl-compiler/src/realm.rs](../../../../rust/tcl-compiler/src/realm.rs), `CommandBindingRealm::possible_entered_body_invocations`: Project the retained original child dispatch inventory with source lookup context and ownership intact.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `possible_entered_body_invocations`: Retain each genuine body child and selected source target rather than recapture decoded argument roles.
- [rust/tcl-compiler/src/interprocedural.rs](../../../../rust/tcl-compiler/src/interprocedural.rs), `interprocedural::tests::original_positioned_child_bodies_preserve_replacements_and_aliases` (linked): Inside authentic time source, stock catch, its captured alias and original rename preserve a possible leaf edge; genuine catch replacement and nominal held procedure heads do not supply body vocabulary. Source-only reachability does not close runtime effects/purity/frame entry.

## Evidence

- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs)
- [rust/tcl-compiler/src/interprocedural.rs](../../../../rust/tcl-compiler/src/interprocedural.rs)
- [rust/tcl-compiler/src/realm.rs](../../../../rust/tcl-compiler/src/realm.rs)
