"""lseq -- Generate a sequence of numbers as a list (Tcl 9.0)."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page lseq.n (Tcl 9.0)"


@register
class LseqCommand(CommandDef):
    name = "lseq"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lseq",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Generate a list of numeric values in a range.",
                synopsis=(
                    "lseq n",
                    "lseq start end ?step?",
                    "lseq start to end",
                    "lseq start 'count' count",
                    "lseq start 'by' step 'count' count",
                ),
                snippet=(
                    "Returns a list of numbers from start through end "
                    "(inclusive) with optional step.  One-arg form yields "
                    "0..n-1.  Float and double values are supported."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    # C Tcl 9.0 ``Tcl_LseqObjCmd`` rejects objc > 6 and
                    # falls to the syntax-error path when the key
                    # decoder can't make sense of fewer than 2 objc's
                    # worth.  Args after the command name: 1..5.
                    synopsis="lseq ?start? ?op? end ?by step? ?count n?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 5),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.LIST,
            side_effect_hints=(),
        )
