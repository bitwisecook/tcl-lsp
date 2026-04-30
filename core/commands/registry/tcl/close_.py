"""close -- Close a channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
    WasmRuntimeImport,
)
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl close(1)"


@register
class CloseCommand(CommandDef):
    name = "close"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="close",
            hover=HoverSnippet(
                summary="Close a channel.",
                synopsis=("close channelId ?r(ead)|w(rite)?",),
                snippet="For bidirectional pipelines you may close one direction (`read`/`write`) selectively.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="close channelId ?r(ead)|w(rite)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            arg_roles={0: frozenset({ArgRole.CHANNEL})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_close",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
