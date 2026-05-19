"""fconfigure -- Set and get options on a channel."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
    WasmRuntimeImport,
)
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl fconfigure(1)"


@register
class FconfigureCommand(CommandDef):
    name = "fconfigure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fconfigure",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Set and get options on a channel.",
                synopsis=("fconfigure channelId ?optionName? ?value ...?",),
                snippet="",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fconfigure channelId ?optionName? ?value ...?",
                    options=(
                        OptionSpec(name="-blocking", takes_value=True, value_hint="boolean"),
                        OptionSpec(name="-buffering", takes_value=True, value_hint="mode"),
                        OptionSpec(name="-buffersize", takes_value=True, value_hint="size"),
                        OptionSpec(name="-encoding", takes_value=True, value_hint="encoding"),
                        OptionSpec(name="-eofchar", takes_value=True, value_hint="chars"),
                        OptionSpec(name="-translation", takes_value=True, value_hint="mode"),
                        # Encoding error profile (-profile: TIP 656, Tcl 9.0).
                        OptionSpec(
                            name="-profile",
                            takes_value=True,
                            value_hint="profile",
                            detail="Encoding error profile: strict | tcl8 | replace. (TIP 656, Tcl 9.0+)",
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        # Socket channel options (-nodelay, -keepalive: TIP 528, Tcl 9.0).
                        OptionSpec(
                            name="-keepalive",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Query or set TCP keepalive on a socket channel. (TIP 528, Tcl 9.0+)",
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-nodelay",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Query or set TCP_NODELAY on a socket channel. (TIP 528, Tcl 9.0+)",
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        # Console input mode (TIP 160, Tcl 9.0).
                        OptionSpec(
                            name="-inputmode",
                            takes_value=True,
                            value_hint="mode",
                            detail="Console input mode: normal | password | raw | reset. (TIP 160, Tcl 9.0+)",
                            dialects=frozenset({"tcl9.0"}),
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_roles={0: frozenset({ArgRole.CHANNEL})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            configures_channel=True,
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_fconfigure",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
