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
            "ltm_profile_sip",
            module="ltm",
            object_types=("profile sip",),
        ),
        header_types=(("ltm", "profile sip"),),
        properties=(
            BigipPropertySpec(
                name="alg-enable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="community",
                value_type="reference",
                allow_none=True,
                references=("net_routing_community_list",),
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_sip",),
                default="sip",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dialog-aware",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="dialog-establishment-timeout",
                value_type="integer",
                default="10 seconds",
            ),
            BigipPropertySpec(
                name="enable-sip-firewall",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="insert-record-route-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="insert-via-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
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
            BigipPropertySpec(name="max-media-sessions", value_type="integer", default="6"),
            BigipPropertySpec(name="max-registrations", value_type="integer", default="100"),
            BigipPropertySpec(
                name="max-sessions-per-registration",
                value_type="integer",
                default="50",
            ),
            BigipPropertySpec(name="max-size", value_type="integer", default="65535 bytes"),
            BigipPropertySpec(
                name="registration-timeout",
                value_type="integer",
                default="3600 seconds",
            ),
            BigipPropertySpec(
                name="rtp-proxy-style",
                value_type="enum",
                enum_values=("any-location", "symmetric"),
                default="symmetric",
            ),
            BigipPropertySpec(
                name="secure-via-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="security",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="sip-session-timeout",
                value_type="integer",
                default="300 seconds",
            ),
            BigipPropertySpec(
                name="terminate-on-bye",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="user-via-header",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
        ),
    )
