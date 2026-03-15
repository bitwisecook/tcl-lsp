"""when -- Declare an iRules event handler block."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..namespace_data import (
    EVENT_PROPS,
    event_side_label,
    get_event_description,
    get_event_detail,
)
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/when.html"


def _when_event_values() -> tuple[ArgumentValueSpec, ...]:
    values: list[ArgumentValueSpec] = []
    for event_name in EVENT_PROPS:
        desc = get_event_description(event_name)
        summary = desc or f"Run this handler when `{event_name}` fires."

        # Build a richer snippet with side info from EventProps.
        props = EVENT_PROPS.get(event_name)
        snippet = ""
        if props is not None:
            side = event_side_label(props)
            parts: list[str] = [f"**Context**: {side}"]
            if props.transport:
                t = props.transport
                tlabel = "/".join(x.upper() for x in t) if isinstance(t, tuple) else t.upper()
                parts.append(f"**Transport**: {tlabel}")
            if props.implied_profiles:
                parts.append(f"**Profile**: {', '.join(sorted(props.implied_profiles))}")
            snippet = "  \n".join(parts)

        values.append(
            ArgumentValueSpec(
                value=event_name,
                detail=get_event_detail(event_name),
                hover=HoverSnippet(
                    summary=summary,
                    synopsis=(f"when {event_name} {{ body }}",),
                    snippet=snippet,
                    source=_SOURCE,
                ),
            )
        )
    return tuple(values)


def _when_keyword_values() -> tuple[ArgumentValueSpec, ...]:
    return (
        ArgumentValueSpec(
            value="priority",
            detail="Declare handler priority.",
            hover=HoverSnippet(
                summary="Set explicit execution priority for this event handler.",
                synopsis=("when EVENT priority N { body }",),
                source=_SOURCE,
            ),
        ),
        ArgumentValueSpec(
            value="timing",
            detail="Enable/disable timing metrics for this handler.",
            hover=HoverSnippet(
                summary="Control timing statistics for the declared event handler.",
                synopsis=("when EVENT timing enable { body }",),
                source=_SOURCE,
            ),
        ),
    )


def _when_timing_values() -> tuple[ArgumentValueSpec, ...]:
    return (
        ArgumentValueSpec(
            value="enable",
            detail="Enable timing metrics for this handler.",
            hover=HoverSnippet(
                summary="Enable timing statistics for the declared event handler.",
                synopsis=("when EVENT timing enable { body }",),
                source=_SOURCE,
            ),
        ),
        ArgumentValueSpec(
            value="disable",
            detail="Disable timing metrics for this handler.",
            hover=HoverSnippet(
                summary="Disable timing statistics for the declared event handler.",
                synopsis=("when EVENT timing disable { body }",),
                source=_SOURCE,
            ),
        ),
    )


@register
class WhenCommand(CommandDef):
    name = "when"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="when",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Declare an iRules event handler block.",
                synopsis=(
                    "when EVENT { body }",
                    "when EVENT priority N { body }",
                    "when EVENT timing enable|disable { body }",
                ),
                snippet="`body` runs whenever the specified BIG-IP event fires.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="when EVENT { body }",
                    arg_values={
                        0: _when_event_values(),
                        1: _when_keyword_values(),
                        2: _when_timing_values(),
                        3: _when_keyword_values(),
                        4: _when_timing_values(),
                    },
                ),
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="when EVENT priority N { body }",
                    arg_values={
                        0: _when_event_values(),
                        1: _when_keyword_values(),
                        2: _when_timing_values(),
                        3: _when_keyword_values(),
                        4: _when_timing_values(),
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 6),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
