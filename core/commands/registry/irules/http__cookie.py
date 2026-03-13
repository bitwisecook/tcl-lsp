# Enriched from F5 iRules reference documentation.
"""HTTP::cookie -- F5 iRules command `HTTP::cookie`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, _SENSITIVE_HTTP_HEADERS, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__cookie.html"


_av = make_av(_SOURCE)


@register
class HttpCookieCommand(CommandDef):
    name = "HTTP::cookie"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::cookie",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Queries for or manipulates cookies in HTTP requests and responses.",
                synopsis=("HTTP::cookie <subcommand> ?arg ...?",),
                snippet="Queries for or manipulates cookies in HTTP requests and responses. This command replaces the BIG-IP 4.X variable http_cookie.",
                source=_SOURCE,
                examples=(
                    "# Or just for one statically defined cookie:\n"
                    "when HTTP_RESPONSE {\n"
                    "   HTTP::cookie version myCookie 1\n"
                    "   HTTP::cookie httponly myCookie enable\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::cookie <subcommand> ?arg ...?",
                    arg_values={
                        0: (
                            _av("names", "Return list of cookie names.", "HTTP::cookie names"),
                            _av("count", "Return the number of cookies.", "HTTP::cookie count"),
                            _av(
                                "value",
                                "Return value of named cookie.",
                                "HTTP::cookie value <name>",
                            ),
                            _av(
                                "version",
                                "Get/set cookie version.",
                                "HTTP::cookie version <name> ?value?",
                            ),
                            _av("path", "Get/set cookie path.", "HTTP::cookie path <name> ?value?"),
                            _av(
                                "domain",
                                "Get/set cookie domain.",
                                "HTTP::cookie domain <name> ?value?",
                            ),
                            _av(
                                "ports",
                                "Get/set cookie ports.",
                                "HTTP::cookie ports <name> ?value?",
                            ),
                            _av(
                                "insert",
                                "Insert a new cookie.",
                                "HTTP::cookie insert name <name> value <value>",
                            ),
                            _av("remove", "Remove a cookie by name.", "HTTP::cookie remove <name>"),
                            _av(
                                "sanitize",
                                "Remove all cookies except named ones.",
                                "HTTP::cookie sanitize <name-list>",
                            ),
                            _av(
                                "exists", "Check if a cookie exists.", "HTTP::cookie exists <name>"
                            ),
                            _av(
                                "maxage",
                                "Get/set cookie max-age.",
                                "HTTP::cookie maxage <name> ?value?",
                            ),
                            _av(
                                "expires",
                                "Get/set cookie expires.",
                                "HTTP::cookie expires <name> ?value?",
                            ),
                            _av(
                                "comment",
                                "Get/set cookie comment.",
                                "HTTP::cookie comment <name> ?value?",
                            ),
                            _av(
                                "secure",
                                "Get/set cookie secure flag.",
                                "HTTP::cookie secure <name> ?enable|disable?",
                            ),
                            _av(
                                "commenturl",
                                "Get/set cookie comment URL.",
                                "HTTP::cookie commenturl <name> ?value?",
                            ),
                            _av(
                                "encrypt",
                                "Encrypt a cookie value.",
                                "HTTP::cookie encrypt <name> <passphrase>",
                            ),
                            _av(
                                "decrypt",
                                "Decrypt a cookie value.",
                                "HTTP::cookie decrypt <name> <passphrase>",
                            ),
                            _av(
                                "httponly",
                                "Get/set cookie httponly flag.",
                                "HTTP::cookie httponly <name> ?enable|disable?",
                            ),
                            _av(
                                "attribute",
                                "Get/set arbitrary cookie attribute.",
                                "HTTP::cookie attribute <name> ?attribute? ?value?",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "names": SubCommand(
                    name="names",
                    arity=Arity(0, 0),
                    detail="Return list of cookie names.",
                    synopsis="HTTP::cookie names",
                ),
                "count": SubCommand(
                    name="count",
                    arity=Arity(0, 0),
                    detail="Return the number of cookies.",
                    synopsis="HTTP::cookie count",
                ),
                "value": SubCommand(
                    name="value",
                    arity=Arity(1, 1),
                    detail="Return value of named cookie.",
                    synopsis="HTTP::cookie value <name>",
                ),
                "version": SubCommand(
                    name="version",
                    arity=Arity(1, 2),
                    detail="Get/set cookie version.",
                    synopsis="HTTP::cookie version <name> ?value?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie version <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie version <name> <value>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "path": SubCommand(
                    name="path",
                    arity=Arity(1, 2),
                    detail="Get/set cookie path.",
                    synopsis="HTTP::cookie path <name> ?value?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie path <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie path <name> <value>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "domain": SubCommand(
                    name="domain",
                    arity=Arity(1, 2),
                    detail="Get/set cookie domain.",
                    synopsis="HTTP::cookie domain <name> ?value?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie domain <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie domain <name> <value>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "ports": SubCommand(
                    name="ports",
                    arity=Arity(1, 2),
                    detail="Get/set cookie ports.",
                    synopsis="HTTP::cookie ports <name> ?value?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie ports <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie ports <name> <value>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(2),
                    detail="Insert a new cookie.",
                    synopsis="HTTP::cookie insert name <name> value <value>",
                    credential_arg=2,
                    sensitive_headers=_SENSITIVE_HTTP_HEADERS,
                    mutator=True,
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(1, 1),
                    detail="Remove a cookie by name.",
                    synopsis="HTTP::cookie remove <name>",
                    mutator=True,
                ),
                "sanitize": SubCommand(
                    name="sanitize",
                    arity=Arity(1),
                    detail="Remove all cookies except named ones.",
                    synopsis="HTTP::cookie sanitize <name-list>",
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Check if a cookie exists.",
                    synopsis="HTTP::cookie exists <name>",
                ),
                "maxage": SubCommand(
                    name="maxage",
                    arity=Arity(1, 2),
                    detail="Get/set cookie max-age.",
                    synopsis="HTTP::cookie maxage <name> ?value?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie maxage <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie maxage <name> <value>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "expires": SubCommand(
                    name="expires",
                    arity=Arity(1, 2),
                    detail="Get/set cookie expires.",
                    synopsis="HTTP::cookie expires <name> ?value?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie expires <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie expires <name> <value>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "comment": SubCommand(
                    name="comment",
                    arity=Arity(1, 2),
                    detail="Get/set cookie comment.",
                    synopsis="HTTP::cookie comment <name> ?value?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie comment <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie comment <name> <value>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "secure": SubCommand(
                    name="secure",
                    arity=Arity(1, 2),
                    detail="Get/set cookie secure flag.",
                    synopsis="HTTP::cookie secure <name> ?enable|disable?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie secure <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie secure <name> <enable|disable>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "commenturl": SubCommand(
                    name="commenturl",
                    arity=Arity(1, 2),
                    detail="Get/set cookie comment URL.",
                    synopsis="HTTP::cookie commenturl <name> ?value?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie commenturl <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie commenturl <name> <value>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "encrypt": SubCommand(
                    name="encrypt",
                    arity=Arity(2, 2),
                    detail="Encrypt a cookie value.",
                    synopsis="HTTP::cookie encrypt <name> <passphrase>",
                ),
                "decrypt": SubCommand(
                    name="decrypt",
                    arity=Arity(2, 2),
                    detail="Decrypt a cookie value.",
                    synopsis="HTTP::cookie decrypt <name> <passphrase>",
                ),
                "httponly": SubCommand(
                    name="httponly",
                    arity=Arity(1, 2),
                    detail="Get/set cookie httponly flag.",
                    synopsis="HTTP::cookie httponly <name> ?enable|disable?",
                    forms=(
                        FormSpec(
                            kind=FormKind.GETTER,
                            synopsis="HTTP::cookie httponly <name>",
                            arity=Arity(1, 1),
                            pure=True,
                        ),
                        FormSpec(
                            kind=FormKind.SETTER,
                            synopsis="HTTP::cookie httponly <name> <enable|disable>",
                            arity=Arity(2, 2),
                            mutator=True,
                        ),
                    ),
                ),
                "attribute": SubCommand(
                    name="attribute",
                    arity=Arity(1, 3),
                    detail="Get/set arbitrary cookie attribute.",
                    synopsis="HTTP::cookie attribute <name> ?attribute? ?value?",
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(2),
                    detail="Replace cookie value.",
                    synopsis="HTTP::cookie replace <name> <value>",
                    credential_arg=2,
                    sensitive_headers=_SENSITIVE_HTTP_HEADERS,
                    mutator=True,
                ),
            },
            validation=ValidationSpec(arity=Arity()),
            taint_output_sink="IRULE3002",
            taint_output_sink_subcommands=frozenset({"insert", "replace"}),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            cse_candidate=True,
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_COOKIE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
