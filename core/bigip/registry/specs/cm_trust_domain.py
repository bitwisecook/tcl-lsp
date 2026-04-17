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
            "cm_trust_domain",
            module="cm",
            object_types=("trust-domain",),
        ),
        header_types=(("cm", "trust-domain"),),
        properties=(
            BigipPropertySpec(name="add-device", value_type="string"),
            BigipPropertySpec(name="device-ip", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(
                name="device-port",
                value_type="integer",
                in_sections=("add-device",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="device-name", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                in_sections=("add-device",),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="password", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(
                name="sha1-fingerprint", value_type="string", in_sections=("add-device",)
            ),
            BigipPropertySpec(name="devices", value_type="string"),
            BigipPropertySpec(name="remove-device", value_type="string"),
            BigipPropertySpec(name="deprecated", value_type="string"),
            BigipPropertySpec(
                name="ca-devices",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="md5-fingerprint", value_type="string"),
            BigipPropertySpec(name="name", value_type="string"),
            BigipPropertySpec(
                name="non-ca-devices",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="serial", value_type="string"),
            BigipPropertySpec(name="sha1-fingerprint", value_type="string"),
            BigipPropertySpec(name="username", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="restart", value_type="string"),
            BigipPropertySpec(name="import-user-defined-cert", value_type="string"),
            BigipPropertySpec(name="import-user-defined-key", value_type="string"),
        ),
    )
