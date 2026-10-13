Logical constant values
=======================

`LogicalVariableScopeAdvice` selects declarations from an independently retained
Logical source image and its exact lexer and availability inputs. Qualified
variables consult actual Logical namespace declaration frames; they do not
reuse command-resolution namespaces. Multiple matching declaration tables
supply no unique declaration.

The analyser's private value selector joins that exact declaration to a
recorded dominating value, complete source image, `ResolvedAnalysisInput`,
declaration span and value span. It also checks the current public value map
still holds the recorded value. Snapshot and restore retain the private record
alongside the value map. Current-frame and explicit-root uplevel source advice
remain distinct from unavailable caller frames.

Dynamic lambda inspection, Logical declared-symbol substitutions and DictWith
value inspection use this selector. Native and hosted source cannot obtain a
Logical value grant. Links, array elements and unavailable method namespace
coordinates require independent value provenance. A braced scalar name keeps
its scalar form even when it contains an opening parenthesis.

This value record supplies readonly source metadata. It is not an entered cell,
runtime alias, current receiver, Native activation or permission to edit a
computed declaration.
