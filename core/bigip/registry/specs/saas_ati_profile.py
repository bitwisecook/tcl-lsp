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
            "saas_ati_profile",
            module="saas",
            object_types=("ati profile",),
        ),
        header_types=(("saas", "ati profile"),),
        properties=(
            BigipPropertySpec(
                name="api-svc-add-connecting-ip",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="api-svc-connecting-ip-header", value_type="string"),
            BigipPropertySpec(name="api-svc-domain-pool", value_type="reference", allow_none=True),
            BigipPropertySpec(name="api-svc-hostname", value_type="string"),
            BigipPropertySpec(name="api-svc-ivs-ssl", value_type="reference", allow_none=True),
            BigipPropertySpec(name="api-svc-js-path", value_type="string"),
            BigipPropertySpec(name="api-svc-telemetry-path", value_type="string"),
            BigipPropertySpec(
                name="api-svc-use-sni",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="bas-domain-pool", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="bas-enable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="bas-hostname", value_type="string"),
            BigipPropertySpec(name="bas-ivs-ssl", value_type="reference", allow_none=True),
            BigipPropertySpec(name="bas-proxy-destination", value_type="string"),
            BigipPropertySpec(name="bas-telemetry-path", value_type="string"),
            BigipPropertySpec(name="customer-id", value_type="string"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("saas_ati_profile",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="irules",
                value_type="list",
                allow_none=True,
                references=("apm_policy_agent_irule_event", "pem_irule"),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
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
            BigipPropertySpec(name="proxy-destination", value_type="string"),
            BigipPropertySpec(name="proxy-password", value_type="string", allow_none=True),
            BigipPropertySpec(name="proxy-pool", value_type="reference", allow_none=True),
            BigipPropertySpec(name="proxy-username", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="use-proxy-server",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
