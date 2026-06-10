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
            "ltm_message_routing_sip_profile_session",
            module="ltm",
            object_types=("message-routing sip profile session",),
        ),
        header_types=(("ltm", "message-routing sip profile session"),),
        properties=(
            BigipPropertySpec(
                name="allow-unknown-methods",
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
            BigipPropertySpec(name="custom-via", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_message_routing_sip_profile_session",),
                default="session",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="do-not-connect-back",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="enable-sip-firewall",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="generate-response-on-failure",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="honor-route-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="honor-route-mode",
                value_type="enum",
                enum_values=("loose", "strict"),
            ),
            BigipPropertySpec(
                name="honor-via",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="insert-record-route-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="insert-via-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="loop-detection",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="loop-detection-mode",
                value_type="enum",
                enum_values=("Loose", "Strict"),
            ),
            BigipPropertySpec(
                name="maintenance-mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="max-forwards-check",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="max-msg-header-count", value_type="integer"),
            BigipPropertySpec(name="max-msg-header-size", value_type="integer"),
            BigipPropertySpec(name="max-msg-size", value_type="integer"),
            BigipPropertySpec(
                name="passthru-mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="persistence",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="persist-key",
                        value_type="enum",
                        in_sections=("persistence",),
                        enum_values=("Call-ID", "Custom", "Src-Addr"),
                    ),
                    BigipPropertySpec(
                        name="persist-timeout",
                        value_type="integer",
                        in_sections=("persistence",),
                    ),
                    BigipPropertySpec(
                        name="persist-type",
                        value_type="enum",
                        in_sections=("persistence",),
                        allow_none=True,
                        enum_values=("none", "session"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="persist-key",
                value_type="enum",
                in_sections=("persistence",),
                enum_values=("Call-ID", "Custom", "Src-Addr"),
            ),
            BigipPropertySpec(
                name="persist-timeout",
                value_type="integer",
                in_sections=("persistence",),
            ),
            BigipPropertySpec(
                name="persist-type",
                value_type="enum",
                in_sections=("persistence",),
                allow_none=True,
                enum_values=("none", "session"),
            ),
            BigipPropertySpec(
                name="record-route-mode",
                value_type="enum",
                enum_values=("double", "single"),
            ),
            BigipPropertySpec(name="service-port", value_type="integer"),
        ),
    )
