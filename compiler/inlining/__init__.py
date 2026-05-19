"""S4 inlining: catalogue and (forthcoming) IR-level splicer.

The package owns the inliner's policy and mechanism.  The policy
(``decision``) tags every ``IRProcedure`` with an
:class:`~core.compiler.ir.InlineDecision` after var-escape analysis
has computed ``pure_leaf``.  The mechanism (added in S4.2) walks the
IR module post-lowering and substitutes callee bodies into caller
``IRBlock`` s where the tag is ``ALWAYS`` or ``IF_SINGLE_CALL``.
"""

from .decision import (
    SMALL_BODY_THRESHOLD,
    apply_inline_catalogue,
    classify_proc,
    count_statements,
    count_static_calls,
)
from .inline_pass import inline_module

__all__ = [
    "SMALL_BODY_THRESHOLD",
    "apply_inline_catalogue",
    "classify_proc",
    "count_statements",
    "count_static_calls",
    "inline_module",
]
