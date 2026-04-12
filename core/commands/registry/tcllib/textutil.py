"""textutil -- Text manipulation utilities (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib textutil package"
_PACKAGE = "textutil"


@register
class TextutilTrimCommand(CommandDef):
    name = "textutil::trim"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Remove leading whitespace from each line of text.",
                synopsis=("textutil::trim text ?regexp?",),
                source=_SOURCE,
                examples="set trimmed [textutil::trim $text]",
                return_value="The trimmed text.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="textutil::trim text ?regexp?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class TextutilSplitxCommand(CommandDef):
    name = "textutil::splitx"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Split a string using a regexp pattern as delimiter.",
                synopsis=("textutil::splitx string ?regexp?",),
                snippet=(
                    "Like the core split command but accepts a regular "
                    "expression as the delimiter pattern."
                ),
                source=_SOURCE,
                examples="set parts [textutil::splitx $text {\\s+}]",
                return_value="A list of substrings.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="textutil::splitx string ?regexp?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class TextutilAdjustCommand(CommandDef):
    name = "textutil::adjust"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Adjust a text block to a given line length.",
                synopsis=(
                    "textutil::adjust string ?-length num? "
                    "?-justify left|right|center|plain? ?-hyphenate bool? "
                    "?-full bool? ?-strictlength bool?",
                ),
                source=_SOURCE,
                examples="set wrapped [textutil::adjust $text -length 72]",
                return_value="The adjusted text.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="textutil::adjust string ?options?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class TextutilIndentCommand(CommandDef):
    name = "textutil::indent"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Indent each line of text by a given prefix.",
                synopsis=("textutil::indent text prefix ?skip?",),
                source=_SOURCE,
                examples='set indented [textutil::indent $text "    "]',
                return_value="The indented text.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="textutil::indent text prefix ?skip?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
        )


@register
class TextutilUndentCommand(CommandDef):
    name = "textutil::undent"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Remove common leading whitespace from all lines.",
                synopsis=("textutil::undent text",),
                source=_SOURCE,
                examples="set clean [textutil::undent $text]",
                return_value="The undented text.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="textutil::undent text"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


def _tu(name: str, summary: str, synopsis: str, arity: Arity) -> type:
    """Register a pure textutil command."""

    @register
    class _Cmd(CommandDef):
        pass

    _Cmd.name = f"textutil::{name}"
    _Cmd.spec = classmethod(  # type: ignore[assignment]
        lambda cls, _n=f"textutil::{name}", _s=summary, _syn=synopsis, _a=arity: CommandSpec(
            name=_n,
            tcllib_package=_PACKAGE,
            pure=True,
            hover=HoverSnippet(summary=_s, synopsis=(_syn,), source=_SOURCE),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=_syn),),
            validation=ValidationSpec(arity=_a),
        )
    )
    return _Cmd


_tu("tabify", "Convert spaces to tabs.", "textutil::tabify string ?num?", Arity(1, 2))
_tu("untabify", "Convert tabs to spaces.", "textutil::untabify string ?num?", Arity(1, 2))
_tu(
    "tabify2",
    "Convert spaces to tabs (position-aware).",
    "textutil::tabify2 string ?num?",
    Arity(1, 2),
)
_tu(
    "untabify2",
    "Convert tabs to spaces (position-aware).",
    "textutil::untabify2 string ?num?",
    Arity(1, 2),
)
_tu("strRepeat", "Repeat a string N times.", "textutil::strRepeat char num", Arity(2, 2))
_tu("blank", "Return a string of N spaces.", "textutil::blank n", Arity(1, 1))
_tu("cap", "Capitalise the first character.", "textutil::cap string", Arity(1, 1))
_tu("uncap", "Lowercase the first character.", "textutil::uncap string", Arity(1, 1))
_tu("chop", "Remove the last character.", "textutil::chop string", Arity(1, 1))
_tu("tail", "Remove the first character.", "textutil::tail string", Arity(1, 1))
_tu("capEachWord", "Capitalise each word.", "textutil::capEachWord sentence", Arity(1, 1))
_tu(
    "longestCommonPrefix",
    "Find the longest common prefix of strings.",
    "textutil::longestCommonPrefix ?string ...?",
    Arity(0),
)
_tu(
    "longestCommonPrefixList",
    "Find the longest common prefix of a list of strings.",
    "textutil::longestCommonPrefixList list",
    Arity(1, 1),
)
_tu(
    "trimleft",
    "Trim leading characters from each line.",
    "textutil::trimleft text ?regexp?",
    Arity(1, 2),
)
_tu(
    "trimright",
    "Trim trailing characters from each line.",
    "textutil::trimright text ?regexp?",
    Arity(1, 2),
)
_tu("trimPrefix", "Remove a prefix from a string.", "textutil::trimPrefix text prefix", Arity(2, 2))
_tu(
    "trimEmptyHeading",
    "Remove leading empty lines.",
    "textutil::trimEmptyHeading text",
    Arity(1, 1),
)
_tu("splitn", "Split a string into chunks of length N.", "textutil::splitn str ?len?", Arity(1, 2))
