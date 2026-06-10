"""puts -- Write text to a channel."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl puts(1)"


@register
class PutsCommand(CommandDef):
    name = "puts"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="puts",
            byte_compiled=True,
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Write text to a channel (stdout by default).",
                synopsis=("puts ?-nonewline? ?channelId? string",),
                snippet="Use `-nonewline` to suppress the trailing newline.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="puts ?-nonewline? ?channelId? string",
                    options=(
                        OptionSpec(
                            name="-nonewline",
                            detail="Do not append trailing newline.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                # ``puts ?-nonewline? ?channelId? string`` accepts
                # 1, 2, or 3 raw args, but the static arity is
                # ``Arity(1, 2)`` *positional* — the analyser strips
                # the leading ``-nonewline`` flag (declared on
                # ``CommandSig.options``) before counting, so
                # ``puts -nonewline $chan msg`` (3 raw / 2 positional)
                # passes while ``puts a b c`` (3 positional) is
                # rejected as too many.  The Zig handler accepts the
                # 3-arg shape internally regardless of this static
                # bound; the runtime doesn't enforce ``arity_max``.
                arity=Arity(1, 2),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_puts",
                argc=1,
                nontrapping=True,
                params=("i32",),
                results=("i32",),
            ),
            taint_output_sink="T101",
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
