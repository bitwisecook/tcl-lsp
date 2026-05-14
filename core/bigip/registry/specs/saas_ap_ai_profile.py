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
            "saas_ap_ai_profile",
            module="saas",
            object_types=("ap-ai profile",),
        ),
        header_types=(("saas", "ap-ai profile"),),
        properties=(
            BigipPropertySpec(
                name="account-protection",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="add-connecting-ip",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="ai-header-name", value_type="string"),
            BigipPropertySpec(name="ap-header-name", value_type="string"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="authentication-intelligence",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="block-response-body", value_type="string", allow_none=True),
            BigipPropertySpec(name="block-response-code", value_type="integer", allow_none=True),
            BigipPropertySpec(name="block-response-content-type", value_type="string"),
            BigipPropertySpec(name="connecting-ip-header", value_type="string"),
            BigipPropertySpec(name="customer-id", value_type="string"),
            BigipPropertySpec(
                name="decrypt-cookie",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("saas_ap_ai_profile",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="domain-pool", value_type="reference", allow_none=True),
            BigipPropertySpec(name="encryption-key", value_type="string"),
            BigipPropertySpec(name="hostname", value_type="string"),
            BigipPropertySpec(
                name="irules",
                value_type="list",
                allow_none=True,
                references=("apm_policy_agent_irule_event", "pem_irule"),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="ivs-ssl", value_type="string"),
            BigipPropertySpec(
                name="js-inject-exclude-paths",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="js-inject-exclude-paths-enable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="js-inject-include-paths",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="js-inject-include-paths-enable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="js-inject-location",
                value_type="enum",
                enum_values=("body", "head"),
            ),
            BigipPropertySpec(
                name="js-inject-script-attribute",
                value_type="enum",
                enum_values=("async", "async-defer", "defer", "sync"),
            ),
            BigipPropertySpec(name="js-path", value_type="string"),
            BigipPropertySpec(
                name="protected-endpoints",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="ai-endpoint",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="ap-endpoint",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("protected-endpoints",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="enforcement-mode",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("mitigate", "monitor"),
            ),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("protected-endpoints",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="max-cookie-age",
                value_type="integer",
                in_sections=("protected-endpoints",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="mitigate-malformed-cookie",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="mitigate-max-cookie-age",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="mitigate-missing-cookie",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="mitigation-action",
                value_type="enum",
                in_sections=("protected-endpoints",),
                enum_values=("block", "redirect"),
            ),
            BigipPropertySpec(
                name="path",
                value_type="string",
                in_sections=("protected-endpoints",),
                allow_none=True,
            ),
            BigipPropertySpec(name="proxy-destination", value_type="string", allow_none=True),
            BigipPropertySpec(name="proxy-password", value_type="string", allow_none=True),
            BigipPropertySpec(name="proxy-pool", value_type="reference", allow_none=True),
            BigipPropertySpec(name="proxy-username", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="recommendation-cookie-name",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="redirect-path", value_type="string", allow_none=True),
            BigipPropertySpec(name="redirect-response-code", value_type="integer", allow_none=True),
            BigipPropertySpec(name="telemetry-path", value_type="string"),
            BigipPropertySpec(
                name="use-proxy-server",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="use-sni",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
