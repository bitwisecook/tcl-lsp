# Enriched from F5 iRules reference documentation.
"""X509::subject -- Returns the subject of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__subject.html"


_av = make_av(_SOURCE)


@register
class X509SubjectCommand(CommandDef):
    name = "X509::subject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::subject",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the subject of an X509 certificate.",
                synopsis=("X509::subject CERTIFICATE (commonName)?",),
                snippet=(
                    "Returns the subject of the specified X509 certificate.\n"
                    "If commonName RDN is specified, returns the Subject CN in UTF8 format."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    "\n"
                    "  # Check if the client supplied one or more client certs\n"
                    "  if {[SSL::cert count] > 0}{\n"
                    "\n"
                    "    # Check the first client cert subject\n"
                    '    if { [X509::subject [SSL::cert 0]] equals "someSubject" } {\n'
                    '      log local0. "X509 Certificate Subject [X509::subject [SSL::cert 0]]"\n'
                    "      pool my_pool\n"
                    "    }\n"
                    "    # Check the first client cert subject commonName\n"
                    '    if { [X509::subject [SSL::cert 0] commonName] equals "someCommonName" } {'
                ),
                return_value="Returns the subject of an X509 certificate. If commonName RDN is specified, returns the Subject CN in UTF8 format.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::subject CERTIFICATE (commonName)?",
                    arg_values={
                        0: (
                            _av(
                                "commonName",
                                "X509::subject commonName",
                                "X509::subject CERTIFICATE (commonName)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
