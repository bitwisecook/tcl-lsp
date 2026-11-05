# naming.editor.original-property-self-selector-candidates

Kind: `implementation-contract`

## Problem statement

A readonly property reference report needs original selector and declaring-property ownership, while incomplete source coverage cannot establish that no other occurrence exists.

## Question

How do readonly self-property selector candidates retain original source ownership, and when can their report claim a complete source occurrence census?

## Conclusion

`original_source_self_property_references` retains positive selector candidates only under the current source image, resolved input and source-transition tape, matching an original instance property through its exact source-name policy and bytes and member-body ownership. Core reports a complete source occurrence census only when its independent checked source walk and typed method/property correspondence establish coverage; otherwise it retains partial or unavailable coverage. The server limits a source census to one retained document and maps complete coverage to `complete-source-census`. A nonempty complete or partial report can supply a clickable reference command. This source projection does not establish property installation, receiver allocation, body entry, runtime lookup, observer state, Native execution or edit permission.

## Scope

Readonly source selectors and source occurrence reporting for original instance properties. Complete coverage requires a nonempty ASCII original property name and the checked potential-source walk to cover selected self-dispatch commands and each command span to join exactly once to the typed method or property catalogue. Dynamic, ambiguous, unselected and otherwise unrepresented commands stay partial or unavailable. A positive catalogue alone does not establish complete occurrence coverage or runtime absence. No provider execution or software test result is attached.

The original class member source association is documented in `docs/design/contracts/shared-utility-contracts-rust.md` and checked by the owner-resolution gate.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No provider execution for this source contract.

### bigip

Status: `not-tested`.

No appliance execution for this source contract.

## Owner-resolution map

| Purpose | Existing owner | Limitation |
| --- | --- | --- |
| Selector source ownership | `original_source_self_property_references` | Positive candidates alone do not close an occurrence census. |
| Core coverage | `readonly_reference_report` | Exact coverage requires its independent source checks. |
| Server presentation | `single_document_coverage` and `coverage_tag` | Multiple retained documents keep coverage partial; no runtime or edit authority follows. |
