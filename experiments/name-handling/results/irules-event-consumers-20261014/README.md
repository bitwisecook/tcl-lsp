# Retained iRules event consumers

Unused-procedure library detection, persistent-worker-state warnings,
global-name fixes and cross-event static warnings use the original shared event
body owner. Procedure report names do not select event semantics. Top level
carries no event; unavailable event ownership cannot issue event-only suppression
or an automatic unused-procedure rewrite.

`event_body_for_function` validates the exact actual Module-issued body ID,
original source/configuration and Registry correspondence, the current retained
Module/function metadata, and the same original event Arc. The existing
procedure-event owner supplies the declaration identity; this adapter supplies
no entered event, executing worker, Native frame or successful rewrite proof.

The new controls use real Registry/context/CU construction and then alter only
report labels or independently invalidate the body owner. They cover unchanged
HTTP_REQUEST identity under a forged RULE_INIT label, ordinary event-looking
procedure names, mismatched IDs, stale source, foreign Modules, changed event
Arc and wrong Registry. Existing real RULE_INIT/request event controls remain
unchanged. The unused-procedure fixture now explicitly passes its already
available actual compilation Registry to the pass rather than fabricating a
default Registry or changing any expected result.

Validation: scratch Rustfmt and Main patch applicability pass. Cargo controls
are unexecuted. No F5/BIG-IP experiment or Native authority is claimed. Root's
F5-XC translator/test source is untouched.
