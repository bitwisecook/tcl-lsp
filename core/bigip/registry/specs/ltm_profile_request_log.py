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
            "ltm_profile_request_log",
            module="ltm",
            object_types=("profile request-log",),
        ),
        header_types=(("ltm", "profile request-log"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_request_log",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="log-request-logging-errors",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-response-by-default",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-response-logging-error",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="proxy-close-on-error", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="proxy-respond-on-logging-error",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="proxy-response", value_type="string"),
            BigipPropertySpec(
                name="request-log-error-pool",
                value_type="reference",
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(
                name="request-log-error-protocol",
                value_type="enum",
                allow_none=True,
                enum_values=("tcp", "udp", "none"),
            ),
            BigipPropertySpec(name="request-log-error-template", value_type="string"),
            BigipPropertySpec(
                name="request-log-pool",
                value_type="reference",
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(
                name="request-log-protocol",
                value_type="enum",
                allow_none=True,
                enum_values=("tcp", "udp", "none"),
            ),
            BigipPropertySpec(name="request-log-template", value_type="string"),
            BigipPropertySpec(
                name="request-logging", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="response-log-error-pool",
                value_type="reference",
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(
                name="response-log-error-protocol",
                value_type="enum",
                allow_none=True,
                enum_values=("tcp", "udp", "none"),
            ),
            BigipPropertySpec(name="response-log-error-template", value_type="string"),
            BigipPropertySpec(
                name="response-log-pool",
                value_type="reference",
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(
                name="response-log-protocol",
                value_type="enum",
                allow_none=True,
                enum_values=("tcp", "udp", "none"),
            ),
            BigipPropertySpec(name="response-log-template", value_type="string"),
            BigipPropertySpec(
                name="response-logging", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
