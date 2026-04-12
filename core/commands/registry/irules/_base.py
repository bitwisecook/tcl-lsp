"""Per-package registry for iRules command definitions."""

from __future__ import annotations

from .._base import CommandDef, make_registry  # noqa: F401
from ..namespace_models import EventRequires

_IRULES_ONLY = frozenset({"f5-irules"})

# Header names whose values typically contain secrets — shared by HTTP::header
# and HTTP::cookie insert/replace subcommands.
_SENSITIVE_HTTP_HEADERS = frozenset(
    {
        "authorization",
        "proxy-authorization",
        "x-api-key",
        "x-auth-token",
        "x-secret",
    }
)

# Shared EventRequires for LSN:: commands — available in client-side events
# where the virtual server is associated with an LSN pool/profile.
_LSN_EVENT_REQUIRES = EventRequires(
    profiles=frozenset({"LSN"}),
    also_in=frozenset(
        {
            "AUTH_RESULT",
            "AUTH_WANTCREDENTIAL",
            "CACHE_REQUEST",
            "CACHE_UPDATE",
            "CLIENTSSL_CLIENTCERT",
            "CLIENTSSL_HANDSHAKE",
            "CLIENT_ACCEPTED",
            "CLIENT_DATA",
            "HTTP_CLASS_FAILED",
            "HTTP_CLASS_SELECTED",
            "HTTP_REQUEST",
            "HTTP_REQUEST_DATA",
            "LB_SELECTED",
            "MR_INGRESS",
            "RTSP_REQUEST",
            "RTSP_REQUEST_DATA",
            "SIP_REQUEST",
            "STREAM_MATCHED",
        }
    ),
)

_REGISTRY, register = make_registry()
