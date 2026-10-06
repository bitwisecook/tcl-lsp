#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Generate isolated iRule procedure, folder, partition, and Unicode controls."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", required=True)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]{0,15}", args.run):
        parser.error("run must match [A-Za-z][A-Za-z0-9_]{0,15}")
    args.out.mkdir(parents=True, exist_ok=False)

    prefix = f"__tcl_lsp_probe_2286_{args.run}"
    partition = f"R2286_{args.run}"
    common_a = f"/Common/{prefix}_fa"
    common_b = f"/Common/{prefix}_fb"
    part_root = f"/{partition}"
    part_a = f"{part_root}/{prefix}_fa"
    part_b = f"{part_root}/{prefix}_fb"
    part_nested = f"{part_a}/{prefix}_nested"
    fixtures = []
    rows = []

    objects = {
        "partition": partition,
        "folders_create_order": [common_a, common_b, part_a, part_b, part_nested],
        "folders_delete_order": [part_nested, part_b, part_a, common_b, common_a],
        "providers": {
            "common_root": f"/Common/{prefix}_rootlib",
            "common_a": f"{common_a}/{prefix}_lib",
            "common_b": f"{common_b}/{prefix}_lib",
            "partition_root": f"{part_root}/{prefix}_rootlib",
            "partition_a": f"{part_a}/{prefix}_lib",
            "partition_b": f"{part_b}/{prefix}_lib",
            "partition_nested": f"{part_nested}/{prefix}_lib",
        },
        "callers": {
            "common_root": f"/Common/{prefix}_caller",
            "common_a": f"{common_a}/{prefix}_caller",
            "partition_root": f"{part_root}/{prefix}_caller",
            "partition_a": f"{part_a}/{prefix}_caller",
            "partition_nested": f"{part_nested}/{prefix}_caller",
        },
        "literal_unicode": {},
        "dynamic_unicode": f"/Common/{prefix}_unicode_dynamic",
    }

    def emit(case, object_name, body, purpose):
        data = f"ltm rule {object_name} {{\n{body}\n}}\n".encode("utf-8")
        filename = f"{case}.conf"
        (args.out / filename).write_bytes(data)
        (args.out / f"{filename}.hex").write_text(data.hex() + "\n", encoding="ascii")
        fixtures.append(
            {
                "case": case,
                "object": object_name,
                "file": filename,
                "sha256": hashlib.sha256(data).hexdigest(),
                "size": len(data),
                "purpose": purpose,
            }
        )
        rows.append(f"{filename}\t{object_name}")

    provider_body = '''proc identify {expected} {
    set command [binary format H* 6e616d6573706163652063757272656e74]
    set current [eval $command]
    return [list $expected $current [info level 0]]
}'''
    provider_cases = {
        "provider_common_root": "common_root",
        "provider_common_a": "common_a",
        "provider_common_b": "common_b",
        "provider_partition_root": "partition_root",
        "provider_partition_a": "partition_a",
        "provider_partition_b": "partition_b",
        "provider_partition_nested": "partition_nested",
    }
    for case, key in provider_cases.items():
        emit(
            case,
            objects["providers"][key],
            provider_body,
            "unattached F5 procedure provider returning namespace current and info level 0",
        )

    def caller_body(case, calls):
        rows_tcl = " ".join("{%s %s}" % pair for pair in calls)
        return f'''proc local_identify {{expected}} {{
    set command [binary format H* 6e616d6573706163652063757272656e74]
    set current [eval $command]
    return [list $expected $current [info level 0]]
}}
when HTTP_REQUEST {{
    set unit [TMM::cmp_unit]
    set request [HTTP::header value X-R2286-Request]
    foreach item [list {rows_tcl}] {{
        set label [lindex $item 0]
        set target [lindex $item 1]
        set rc [catch {{call $target $label}} value]
        set which_script [list [binary format H* 6e616d657370616365] which -command $target]
        set which_rc [catch {{eval $which_script}} which]
        binary scan $value H* value_hex
        binary scan $which H* which_hex
        log local0. "R2286|{args.run}|{case}|unit=$unit|request=$request|label=$label|target=$target|rc=$rc|value_hex=$value_hex|which_rc=$which_rc|which_hex=$which_hex"
    }}
    HTTP::header insert X-R2286-Scope "{case}:$unit"
}}
when HTTP_RESPONSE {{
    HTTP::header insert X-R2286-TMM [TMM::cmp_unit]
}}'''

    p = prefix
    caller_specs = {
        "caller_common_root": (
            objects["callers"]["common_root"],
            [
                ("local", "local_identify"),
                ("common_root_relative", f"{p}_rootlib::identify"),
                ("common_root_absolute", f"/Common/{p}_rootlib::identify"),
                ("common_a_absolute", f"{common_a}/{p}_lib::identify"),
                ("common_b_absolute", f"{common_b}/{p}_lib::identify"),
                ("partition_root_absolute", f"{part_root}/{p}_rootlib::identify"),
                ("partition_a_absolute", f"{part_a}/{p}_lib::identify"),
                ("wrong_folder", f"{common_a}/{p}_missing::identify"),
                ("tcl_absolute_namespace", f"::{p}_rootlib::identify"),
            ],
        ),
        "caller_common_a": (
            objects["callers"]["common_a"],
            [
                ("local", "local_identify"),
                ("same_folder_relative", f"{p}_lib::identify"),
                ("same_folder_absolute", f"{common_a}/{p}_lib::identify"),
                ("common_root_relative", f"{p}_rootlib::identify"),
                ("common_root_absolute", f"/Common/{p}_rootlib::identify"),
                ("sibling_absolute", f"{common_b}/{p}_lib::identify"),
                ("partition_a_absolute", f"{part_a}/{p}_lib::identify"),
            ],
        ),
        "caller_partition_root": (
            objects["callers"]["partition_root"],
            [
                ("local", "local_identify"),
                ("partition_root_relative", f"{p}_rootlib::identify"),
                ("partition_root_absolute", f"{part_root}/{p}_rootlib::identify"),
                ("partition_a_absolute", f"{part_a}/{p}_lib::identify"),
                ("common_root_relative", f"{p}_rootlib::identify"),
                ("common_root_absolute", f"/Common/{p}_rootlib::identify"),
                ("common_a_absolute", f"{common_a}/{p}_lib::identify"),
            ],
        ),
        "caller_partition_a": (
            objects["callers"]["partition_a"],
            [
                ("local", "local_identify"),
                ("same_folder_relative", f"{p}_lib::identify"),
                ("same_folder_absolute", f"{part_a}/{p}_lib::identify"),
                ("partition_root_relative", f"{p}_rootlib::identify"),
                ("partition_root_absolute", f"{part_root}/{p}_rootlib::identify"),
                ("sibling_absolute", f"{part_b}/{p}_lib::identify"),
                ("nested_absolute", f"{part_nested}/{p}_lib::identify"),
                ("common_root_relative", f"{p}_rootlib::identify"),
                ("common_root_absolute", f"/Common/{p}_rootlib::identify"),
            ],
        ),
        "caller_partition_nested": (
            objects["callers"]["partition_nested"],
            [
                ("local", "local_identify"),
                ("same_folder_relative", f"{p}_lib::identify"),
                ("same_folder_absolute", f"{part_nested}/{p}_lib::identify"),
                ("parent_absolute", f"{part_a}/{p}_lib::identify"),
                ("partition_root_relative", f"{p}_rootlib::identify"),
                ("partition_root_absolute", f"{part_root}/{p}_rootlib::identify"),
                ("common_root_absolute", f"/Common/{p}_rootlib::identify"),
            ],
        ),
    }
    for case, (object_name, calls) in caller_specs.items():
        emit(
            case,
            object_name,
            caller_body(case, calls),
            "F5 call routing and Tcl namespace introspection with backend forwarding",
        )

    unicode_values = {
        "precomposed": "é",
        "decomposed": "e\u0301",
        "bmp_copyright": "©",
        "bmp_snowman": "☃",
        "bmp_heart": "❤",
        "emoji_grinning": "😀",
        "emoji_text_vs": "❤\ufe0f",
        "emoji_skin_tone": "👍🏽",
        "emoji_zwj": "👩🏽\u200d💻",
        "emoji_family": "👨\u200d👩\u200d👧\u200d👦",
        "emoji_flag": "🇳🇿",
        "emoji_rainbow_flag": "🏳️\u200d🌈",
        "emoji_keycap": "1️⃣",
    }
    for label, value in unicode_values.items():
        object_name = f"/Common/{prefix}_unicode_literal_{label}"
        objects["literal_unicode"][label] = object_name
        emit(
            f"unicode_literal_{label}",
            object_name,
            f'''when HTTP_REQUEST {{
    set unit [TMM::cmp_unit]
    set request [HTTP::header value X-R2286-Request]
    set value "{value}"
    binary scan $value H* value_hex
    log local0. "R2286|{args.run}|unicode_literal_{label}|unit=$unit|request=$request|value=$value|value_hex=$value_hex|chars=[string length $value]"
    HTTP::header insert X-R2286-Scope "unicode_literal_{label}:$unit"
}}
when HTTP_RESPONSE {{
    HTTP::header insert X-R2286-TMM [TMM::cmp_unit]
}}''',
            f"literal UTF-8 log payload {label}",
        )

    dynamic_rows = []
    for label, value in unicode_values.items():
        dynamic_rows.append(f"{{{label} {value.encode('utf-8').hex()}}}")
    emit(
        "unicode_dynamic",
        objects["dynamic_unicode"],
        f'''when HTTP_REQUEST {{
    set unit [TMM::cmp_unit]
    set request [HTTP::header value X-R2286-Request]
    foreach item [list {' '.join(dynamic_rows)}] {{
        set label [lindex $item 0]
        set source_hex [lindex $item 1]
        set value [binary format H* $source_hex]
        binary scan $value H* value_hex
        log local0. "R2286|{args.run}|unicode_dynamic|unit=$unit|request=$request|label=$label|source_hex=$source_hex|value=$value|value_hex=$value_hex|chars=[string length $value]"
    }}
    HTTP::header insert X-R2286-Scope "unicode_dynamic:$unit"
}}
when HTTP_RESPONSE {{
    HTTP::header insert X-R2286-TMM [TMM::cmp_unit]
}}''',
        "runtime-materialized UTF-8 log payloads with exact byte and character evidence",
    )

    (args.out / "rules.tsv").write_text("\n".join(rows) + "\n", encoding="utf-8")
    (args.out / "objects.json").write_text(
        json.dumps(objects, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    manifest = {
        "run": args.run,
        "kind": "additional_procedure_scope_and_unicode_controls",
        "encoding": "UTF-8",
        "expected_appliance_results": "UNMEASURED",
        "objects": objects,
        "unicode_values": {
            label: {
                "text": value,
                "utf8_hex": value.encode("utf-8").hex(),
                "codepoints": [f"U+{ord(char):04X}" for char in value],
            }
            for label, value in unicode_values.items()
        },
        "fixtures": fixtures,
    }
    (args.out / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Generated {len(fixtures)} controls in {args.out}")


if __name__ == "__main__":
    main()
