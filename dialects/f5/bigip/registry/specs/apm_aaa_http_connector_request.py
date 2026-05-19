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
            "apm_aaa_http_connector_request",
            module="apm",
            object_types=("aaa http-connector-request",),
        ),
        header_types=(("apm", "aaa http-connector-request"),),
        properties=(
            BigipPropertySpec(
                name="auth",
                value_type="enum",
                allow_none=True,
                enum_values=("basic", "bearer", "custom", "none"),
                default="none",
            ),
            BigipPropertySpec(name="method", value_type="string", required=True),
            BigipPropertySpec(name="password", value_type="string", allow_none=True),
            BigipPropertySpec(name="request-body", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="request-headers",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="response-action",
                value_type="enum",
                enum_values=("ignore", "parse", "save"),
            ),
            BigipPropertySpec(name="response-headers", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="secure-variables",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="token", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="transport",
                value_type="reference",
                required=True,
                references=(
                    "apm_aaa_http_connector_transport",
                    "ltm_message_routing_diameter_transport_config",
                    "ltm_message_routing_generic_transport_config",
                    "ltm_message_routing_mqtt_transport_config",
                    "ltm_message_routing_sip_transport_config",
                ),
            ),
            BigipPropertySpec(name="url", value_type="string", required=True),
            BigipPropertySpec(name="username", value_type="string", allow_none=True),
        ),
    )
