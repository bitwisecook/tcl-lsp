from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_profile_rtsp",
            module="ltm",
            object_types=("profile rtsp",),
        ),
        header_types=(("ltm", "profile rtsp"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="check-source",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_profile_rtsp",),
                default="rtsp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="idle-timeout", value_type="integer", default="300 seconds"),
            BigipPropertySpec(
                name="log-profile",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="log-publisher",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="max-header-size", value_type="integer", default="4096 bytes"),
            BigipPropertySpec(name="max-queued-data", value_type="integer", default="32768 bytes"),
            BigipPropertySpec(
                name="multicast-redirect",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="proxy",
                value_type="enum",
                allow_none=True,
                enum_values=("external", "internal", "none"),
                default="none",
            ),
            BigipPropertySpec(
                name="proxy-header",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="real-http-persistence",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="rtcp-port", value_type="unknown", default="0 (zero)"),
            BigipPropertySpec(name="rtp-port", value_type="unknown", default="0 (zero)"),
            BigipPropertySpec(
                name="session-reconnect",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="unicast-redirect",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
