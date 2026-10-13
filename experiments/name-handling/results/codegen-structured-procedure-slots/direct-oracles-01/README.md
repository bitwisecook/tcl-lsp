# Procedure slots, display collisions and sequential replacement

C Tcl 8.6.18, 9.0.4 and 9.1.0 keep namespace `a:` / leaf `p` and namespace `a` / leaf `:p` distinct. Both `namespace which -command` results render as `::a:::p`; their namespace contexts and return values distinguish them. Replacing the first command changes LEFT to REPLACED and preserves the second command's RIGHT result.

C Tcl 8.4.20 and 8.5.19 reject the second definition: `can't create procedure ":p" in non-global namespace with name starting with ":"`. Those two scripts exit1 and their original stderr is retained. The collision is not admitted uniformly across the five C versions.

The captured Jim binary selects its flat-name policy: both calls return RIGHT after the second definition and REPLACED after replacement. The identical display is an actual shared command identity there. Compiler identity and lookup must retain the chosen naming policy and publication grammar.

All six interpreters return FIRST before and SECOND after sequential replacement of `audit`. Ten of the twelve runs exit0 with empty stderr; the two older-C collision runs retain the original refusal. Inputs, exact argv, raw outputs and before/after executable hashes are retained in answers.json. No compiler map, native table, Tcltest or F5 acceptance is implied. Jim upstream freshness is independently established elsewhere and is not reverified here.

The BIG-IP chat can execute both retained programs unchanged and report exact version, completion status, stdout/stderr, namespace contexts, displayed names and before/after results in this branch. The question is whether F5 admits both declarations, keeps the contextual commands separate and preserves one while replacing the other; no provider result is assumed for F5.
