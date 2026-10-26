# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""WORK-only isolated baseline14, stack1 and separate actual other103 diagnostics."""

import argparse
import collections
import hashlib
import importlib.util
import json
import os
import re
import resource
import time
from pathlib import Path

STACK_FRAMES_SHA = "0700346a97d8a0397e386f766a045a96a54ab148993720927c5a40ad000607cf"
PROC_STACK_SELECTOR = 'interp::tests::unbounded_proc_recursion_errors_instead_of_crashing'
PROC_STACK_SOURCE_SHA = 'a7c850610cb71c008339248f62dd8361806cb8416874dadaadcd40b9c1f724f3'
ATTESTATION_CONTROLS = {'interp::tests::every_registered_builtin_with_an_intrinsic_is_attested_for_all_of_them': '60f2ca003533f1696b2ef64e85ebbacf5b1b5a096e9f50aa0dc393c79b8eeffa', 'interp::tests::engine_owned_ensemble_attestation_requires_original_workers_and_configuration': '31ef03f83cf2b18391057821042bab05ad6c578473ad37b692a38dd7f3059105', 'interp::tests::a_public_ensemble_copy_cannot_acquire_engine_intrinsic_attestation': '935ffb57107181a5dca53083aea2897e8b359e831bc9cc7f33baeeeb2af30672'}
EXECUTOR_SHA = "88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811"
ROSTER_SHA = "aa5b03f494b6eca91cf00b0b1d6d34e044dd231c76b16d7384e4e9399e6a493f"
OTHER_ROSTER_SHA = "cda2e5059ae902b92ce9881852358f2fc4188e64d9c14518501cf2b5a6d24821"
FAILURE_LOG_FILE_SHA = (
    "88dd41eae670e31bd0f08d857f3fc35f2ccf53164d0127096f5a30c08e675ab5"
)
FAILURE_LOG_SHA = "616b04e54b17eb137bb6f7942d3e07dcc691ee807a6233d0c47419f63a34967c"
PRUNE_CONTROL_AUTHORITY = "f82b1b05717e86fb250fda5cc65cad0ca987332c"
PRUNE_CONTROL_AUTHORITY_SOURCE_SHA = (
    "4c9481706b8f6f13847f48395517234e77483d0174be919a4e77bbfd017eeb8c"
)
PRUNE_CONTROLS = {
    "namespace::tests::publication_revision_keeps_fresh_births_but_rejects_slot_retirement": "164dfbfa82d103fb7cfe7f16cdbe74d9f2ba23d737005ba352758e5c302b594d",
    "namespace::tests::publication_revision_rejects_all_raw_withdrawal_paths": "ce5e6c635383b387bc15deb881ff481d8556578e7e5f76456a93516f037cd2a4",
    "namespace::tests::publication_revision_rejects_tentative_alias_slots_and_restore": "5cadbcab9355d6189f420494a6d8e8fb7dc6e06a097b61f11d740b9f2cef7a78",
    "namespace::tests::publication_revision_rejects_jim_local_shadow_and_restoration": "e2dc729fe689d522c0d35aad4844e07014f13651f880d887344ed3c811f4332c",
    "namespace::tests::publication_revision_refuses_foreign_dead_and_exhausted_worlds": "d9a97c709949fe10b47f7840127d67c543fa4dbd83cee870f3dd7f33110dfc0a",
    "namespace::tests::publication_revision_tracks_moves_and_callback_rebound_rename_sources": "ce0ed09d567fcdeea4c88c6c2e38b955f4ae3dd19b18e17dc0eabd24c456ecf4",
    "namespace::tests::publication_revision_tracks_occupied_jim_value_replacement": "fce936d54393d46b21f602e708e9c3852ede5b3ee10840f63dd37094afe7cb30",
}
TARGETED_PRUNE_CONTROL_AUTHORITY = "74526401072499b7f41f51256ef01257759e2140"
TARGETED_PRUNE_CONTROL_AUTHORITY_FILES = {
    "runtime/rust/src/namespace.rs": "90b3900bf27782c6d674bbf019ff7a0539bb821a0d313086c904366c86b77f19",
    "runtime/rust/src/interp/native_command_names.rs": "2f83a06abaa2b5c9fc5c9bcac6e53afb5a8aff9736133755a49da4bab20ddc87",
}
TARGETED_PRUNE_CONTROLS = {
    "namespace::tests::single_unlink_witness_requires_actual_c_and_exact_previous_revision": {
        "path": "runtime/rust/src/namespace.rs",
        "sha256": "80bf3fb36bee33c09e4874ea7369f987b1d74ead044bf697f331e73049acbed1",
    },
    "namespace::tests::single_unlink_witness_refuses_broad_foreign_dead_and_exhausted_history": {
        "path": "runtime/rust/src/namespace.rs",
        "sha256": "eea4c524fc8c01eca02ba2ae76a3a4b5dc943e8021a9fb46847cd70eb3f307d5",
    },
    "interp::native_command_names::repinned_registration_key_tests::original_byte_registration_single_unlink_matches_full_validation": {
        "path": "runtime/rust/src/interp/native_command_names.rs",
        "sha256": "cbf8c5d72636722e4b88f385226e2d81fc4c813442c0f29e495ee60bb25f1613",
    },
}
AOT5_SOURCE_PATH = 'runtime/rust/src/interp/native_body_artifact/native_i64_add/tests.rs'
AOT5_SOURCE_SHA = 'e7b3122d09b5463803f2231d8cc4fd61f8a37d4306fef71ddf9f92b4b058b957'
AOT5_SELECTORS = ['interp::native_body_artifact::native_i64_add::tests::guarded_future_body_preserves_real_publication_and_runs_primitive_add', 'interp::native_body_artifact::native_i64_add::tests::future_recipe_keeps_number_trace_and_replacement_fallbacks', 'interp::native_body_artifact::native_i64_add::tests::shared_original_recompile_cannot_copy_the_future_recipe', 'interp::native_body_artifact::native_i64_add::tests::reached_numeric_context_change_is_terminal_without_body_replay', 'interp::native_body_artifact::native_i64_add::tests::duplicate_original_formals_follow_original_first_slot_without_a_native_strategy']
NATIVE_GETTER11_CONTROLS = {'value_ops::tests::bytes_to_str_falls_back_on_invalid_utf8': {'path': 'runtime/rust/src/value_ops.rs', 'sha256': '9d81a03feb4597f41d8b46e9a71104f8ef0e3cb0202bfcb2d73fb73325ccf1e6', 'indent': '    '}, 'value_ops::tests::unicode_door_refuses_invalid_utf8_without_replacing_bytes': {'path': 'runtime/rust/src/value_ops.rs', 'sha256': 'a9e2e68d614d1e71bea2960912a81f20f16446b627e8748594fd28acb3979818', 'indent': '    '}, 'value_ops::tests::bytes_to_str_preserves_valid_utf8': {'path': 'runtime/rust/src/value_ops.rs', 'sha256': '35cbbb3a00e750471c87700f434f3700b71506757ac46dfe87d8dfbc0a44277c', 'indent': '    '}, 'value_ops::native_coercion_tests::native_unicode_getter_entry_tests::a_reached_native_unicode_view_preserves_the_original_invalid_byte_observation': {'path': 'runtime/rust/src/value_ops/native_unicode_getter_entry_tests.rs', 'sha256': '7d305226ae2895e0322b6db969307859cb57d1296efc61c14828cc142fb50905', 'indent': ''}, 'value_ops::native_coercion_tests::native_unicode_getter_entry_tests::a_foreign_or_unpaired_native_cache_cannot_borrow_a_generic_ascii_view': {'path': 'runtime/rust/src/value_ops/native_unicode_getter_entry_tests.rs', 'sha256': 'eceae5e9ebcb02c3ade5cd882e8f0b1f5da2965d823bb0330ca811f64b349fe6', 'indent': ''}, 'value_ops::native_coercion_tests::native_unicode_getter_entry_tests::well_formed_native_utf16_is_a_lossless_host_view': {'path': 'runtime/rust/src/value_ops/native_unicode_getter_entry_tests.rs', 'sha256': 'b9bd8d58bc40c056509bf712de996261cf3affa1402d1d3014e8bde523aaa8d3', 'indent': ''}, 'value_ops::native_coercion_tests::native_unicode_getter_entry_tests::a_stale_entered_operation_cannot_consume_an_existing_unicode_cache': {'path': 'runtime/rust/src/value_ops/native_unicode_getter_entry_tests.rs', 'sha256': '14699f70f53c7da99c8ab9517e40fb41a07b2470d047b0cfa822b61383d37313', 'indent': ''}, 'value_ops::native_coercion_tests::native_unicode_getter_entry_tests::generic_or_profile_only_construction_cannot_issue_a_native_getter': {'path': 'runtime/rust/src/value_ops/native_unicode_getter_entry_tests.rs', 'sha256': 'f41f99322f7e6463ce22ca51569e3d295afd7106ea8baa5db19076594a5c7cd5', 'indent': ''}, 'value_ops::native_coercion_tests::native_unicode_getter_entry_tests::getter_callbacks_cannot_refresh_the_original_constructor_entry': {'path': 'runtime/rust/src/value_ops/native_unicode_getter_entry_tests.rs', 'sha256': 'd569802620983e2393da2ff36676af33fc6c8bcf313cff3715a92f04cb39f639', 'indent': ''}, 'value_ops::native_coercion_tests::native_unicode_getter_entry_tests::a_retired_original_object_cannot_reenter_under_a_retained_allocation_lease': {'path': 'runtime/rust/src/value_ops/native_unicode_getter_entry_tests.rs', 'sha256': 'd0ea3c54522aa6a67b7b464752c24460c61378cc11b9e9bc0544322270a8421d', 'indent': ''}, 'value_ops::native_coercion_tests::native_unicode_getter_entry_tests::original_free_callback_cannot_publish_a_stale_unicode_view': {'path': 'runtime/rust/src/value_ops/native_unicode_getter_entry_tests.rs', 'sha256': '47ab02fc1538d6d2ddecf4a180a82ee3baee505d8cb3d0c53de50b9dacb13553', 'indent': ''}}
NATIVE_GETTER11_SELECTORS = list(NATIVE_GETTER11_CONTROLS)
JIM_LIBRARY3_CONTROLS = {'interp::tests::native_jim_core_keeps_only_constructor_empty_roles': {'path': 'runtime/rust/src/interp.rs', 'indent': '    ', 'sha256': '89ded4d4524d471334b5f23413830c0c1167cad32a6338408b92a9564ba60a99'}, 'builtins::native_jim_library_tests::namespace_source_selection_is_separate_from_core_and_preserves_a_replacement': {'path': 'runtime/rust/src/builtins/native_jim_library_tests.rs', 'indent': '', 'sha256': '5f53c1d8399a2abfb069c8c031b83d82af5b53f83050cd9305e68717f8003d82'}, 'builtins::native_jim_library_tests::selected_namespace_formal_conversion_retains_original_source_filename_and_line': {'path': 'runtime/rust/src/builtins/native_jim_library_tests.rs', 'indent': '', 'sha256': '72839a269d24c9b473a122b26c1f159073c63640e076df7aeb1f07ee9dcd012b'}}
JIM_LIBRARY3_SELECTORS = list(JIM_LIBRARY3_CONTROLS)
ALLOCATION2_CONTROLS = {'interp::native_error_headers::c9_error_root_generation_tests::allocation_lease_cannot_certify_a_retired_error_header': {'path': 'runtime/rust/src/interp/native_error_headers.rs', 'sha256': '1be29b6978e2e26504dfda142fc2bb239b0e9970ce720d909347f89a6b10dc7d', 'indent': '    '}, 'interp::native_error_headers::c9_error_root_generation_tests::c9_next_entry_publishes_original_error_headers_without_stringifying_code': {'path': 'runtime/rust/src/interp/native_error_headers.rs', 'sha256': 'c88f9e164039b22db42eaed37827d7351e4ad047120bfb78bbeac681d557e26b', 'indent': '    '}}
ALLOCATION2_SELECTORS = list(ALLOCATION2_CONTROLS)
HEADROOM2_CONTROLS = {'codegen_abi::tests::the_native_path_holds_one_activation_per_level_and_refuses_at_the_bound': {'path': 'runtime/rust/src/codegen_abi.rs', 'sha256': '476121576749ba5d7f043960e07e55b38890e9f10ad3149d91eeee9a6a24ab38', 'indent': '    '}, 'codegen_abi::original_frame_owner::tests::generated_entry_probe_uses_held_owner_and_keeps_refusal_terminal': {'path': 'runtime/rust/src/codegen_abi/original_frame_owner.rs', 'sha256': 'b97c66f78d1dd8ba73047eb9fc3d123d53d84dc6892677d58ce16d798665f058', 'indent': '    '}}
HEADROOM2_SELECTORS = list(HEADROOM2_CONTROLS)
ORDINARY_ENTRY3_CONTROLS = {'interp::native_body_artifact::native_compiler_pass::ordinary_entry_observer_tests::actual_self_removing_trace_cannot_capture_absence_while_firing': {'path': 'runtime/rust/src/interp/native_body_artifact/native_compiler_pass.rs', 'sha256': 'ec1d5758c770b9435a7f8adaaa24e2d6df9b4d748f36813b84d332de0cb4eeb0', 'indent': '    '}, 'interp::native_body_artifact::native_compiler_pass::ordinary_entry_observer_tests::actual_trace_registration_removal_and_state_retirement_stale_observation': {'path': 'runtime/rust/src/interp/native_body_artifact/native_compiler_pass.rs', 'sha256': '16f574b54a282c88fa7475c970853dcae97c2e14cbdbf34dc36704f78676d140', 'indent': '    '}, 'interp::native_body_artifact::native_compiler_pass::ordinary_entry_observer_tests::actual_limit_mutation_refuses_old_and_new_ordinary_entry_observations': {'path': 'runtime/rust/src/interp/native_body_artifact/native_compiler_pass.rs', 'sha256': 'fbf59d42138ae91e839b5d63749cae4c4a25a7114923c4259bf4df037e9f8787', 'indent': '    '}}
ORDINARY_ENTRY3_SELECTORS = list(ORDINARY_ENTRY3_CONTROLS)
CONSTRUCTOR3_CONTROLS = {'capi::collections::tests::generic_collection_preserves_first_refusal_before_original_updater': {'path': 'runtime/rust/src/capi/collections/tests.rs', 'sha256': '9e5e8a9e51bf614d23ae63fc41645ead6046647acc962f64a93e2d5a0d870d7f', 'indent': ''}, 'capi::collections::tests::generic_actual_c_profile_without_refusal_enters_collection_updater': {'path': 'runtime/rust/src/capi/collections/tests.rs', 'sha256': '7252f705670996d66c2dd22001072342b999a166c77a29f279b2313256ad9578', 'indent': ''}, 'interp::tests::native_math_function_commands_follow_constructor_surface': {'path': 'runtime/rust/src/interp.rs', 'sha256': '1fa06df1dca2dfec30633f0adafad1e732e0d1d49937d0abbb1e1e06be77d43f', 'indent': '    '}}
CONSTRUCTOR3_SELECTORS = list(CONSTRUCTOR3_CONTROLS)
GROUPS = (
    "baseline14",
    "stack_abort1",
    "other103",
    "prune_controls7",
    "targeted_prune_controls3",
)
CAP = 20 * 1024**2 - 65536
FLOOR = 2 * 1024**3
BASE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def inventories():
    baseline_path = BASE / "runtime-baseline-supplement-roster.json"
    require(sha(baseline_path) == ROSTER_SHA, "exact supplemental roster changed")
    roster = json.loads(baseline_path.read_text())
    other_path = BASE / "runtime-other-failures-roster.json"
    require(sha(other_path) == OTHER_ROSTER_SHA, "exact other103 roster changed")
    other = json.loads(other_path.read_text())
    log = BASE / "runtime-a26-failed-originals.log.json"
    require(sha(log) == FAILURE_LOG_FILE_SHA, "lossless failure log container changed")
    original_log = json.loads(log.read_text())
    original_bytes = original_log["text"].encode("utf-8")
    require(
        original_log["encoding"] == "utf-8"
        and original_log["raw_byte_count"] == len(original_bytes) == 326773
        and original_log["raw_sha256"] == FAILURE_LOG_SHA
        and hashlib.sha256(original_bytes).hexdigest() == FAILURE_LOG_SHA,
        "authenticated original failure log bytes changed",
    )
    require(
        other["observed_source"] == "a26ab0ffd2b1dc1681710dbf14e518025545ec33"
        and other["observed_run"] == 40039952540
        and other["observed_job"] == 120708509600
        and other["authenticated_decoded_log_sha256"] == FAILURE_LOG_SHA
        and other["baseline_roster_sha256"] == ROSTER_SHA,
        "original source/log/roster join differs",
    )
    failed = re.findall(
        r"\btest ([A-Za-z0-9_:]+) \.\.\. FAILED\b", original_log["text"]
    )
    require(
        len(failed) == len(set(failed)) == other["failed117_count"] == 117
        and set(roster["baseline14"]).issubset(failed),
        "exact authenticated117 failed identities required",
    )
    expected = [name for name in failed if name not in roster["baseline14"]]
    require(
        other["other103"] == expected and len(expected) == len(set(expected)) == 103,
        "other103 is not exact original117 minus baseline14",
    )
    roster["other103"] = expected
    roster["prune_controls7"] = list(PRUNE_CONTROLS)
    roster["targeted_prune_controls3"] = list(TARGETED_PRUNE_CONTROLS)
    return roster, other


def prune_control_source():
    path = Path("runtime/rust/src/namespace.rs")
    raw = path.read_text()
    matched = {}
    for name, expected in PRUNE_CONTROLS.items():
        simple = re.escape(name.rsplit("::", 1)[1])
        pattern = r"(?:^    #\[[^\n]*\]\n)+^    fn " + simple + r"\(\) \{\n.*?^    }\n"
        found = re.findall(pattern, raw, re.MULTILINE | re.DOTALL)
        require(
            len(found) == 1
            and hashlib.sha256(found[0].encode()).hexdigest() == expected,
            "unchanged authored prune control source required: " + name,
        )
        matched[name] = expected
    require(len(matched) == 7, "exact seven authored prune controls required")
    return {
        "authority_commit": PRUNE_CONTROL_AUTHORITY,
        "authority_source_sha256": PRUNE_CONTROL_AUTHORITY_SOURCE_SHA,
        "current_source_sha256": sha(path),
        "path": str(path),
        "function_sha256": matched,
        "matched_count": 7,
        "scope": "Authored Rust software controls only; no original C oracle or baseline identity grant.",
    }


def targeted_prune_control_source():
    current_files = {}
    for path in TARGETED_PRUNE_CONTROL_AUTHORITY_FILES:
        current_files[path] = sha(path)
    matched = {}
    for name, expected in TARGETED_PRUNE_CONTROLS.items():
        simple = re.escape(name.rsplit("::", 1)[1])
        pattern = r"(?:^    #\[[^\n]*\]\n)+^    fn " + simple + r"\(\) \{\n.*?^    }\n"
        found = re.findall(
            pattern, Path(expected["path"]).read_text(), re.MULTILINE | re.DOTALL
        )
        require(
            len(found) == 1
            and hashlib.sha256(found[0].encode()).hexdigest() == expected["sha256"],
            "unchanged targeted prune control source required: " + name,
        )
        matched[name] = expected
    require(len(matched) == 3, "exact three targeted prune controls required")
    return {
        "authority_commit": TARGETED_PRUNE_CONTROL_AUTHORITY,
        "authority_files_sha256": TARGETED_PRUNE_CONTROL_AUTHORITY_FILES,
        "current_files_sha256": current_files,
        "functions": matched,
        "matched_count": 3,
        "scope": "Separate authored software controls; no C oracle, baseline or performance grant.",
    }


def listed(suite, name):
    case = suite["testcases"].get(name)
    return case is not None and case["kind"] == "test" and not case["ignored"]


def selection(out):
    roster, other = inventories()
    require(
        len(roster["baseline14"]) == len(set(roster["baseline14"])) == 14,
        "baseline14 identities",
    )
    require(
        roster["stack_abort1"]
        == [
            "interp::tests::deeply_nested_foreach_reaches_the_explicit_host_evaluation_limit"
        ],
        "stack identity",
    )
    wanted = roster["baseline14"] + roster["stack_abort1"]
    require(len(wanted) == len(set(wanted)) == 15, "duplicate supplemental identities")
    require(
        len(set(wanted + roster["other103"])) == 118,
        "other103 intersects baseline14 or stack1",
    )
    require(
        len(roster["prune_controls7"]) == len(set(roster["prune_controls7"])) == 7
        and len(set(wanted + roster["other103"] + roster["prune_controls7"])) == 125,
        "exact separate authored prune7 identities required",
    )
    require(
        len(roster["targeted_prune_controls3"])
        == len(set(roster["targeted_prune_controls3"]))
        == 3
        and len(
            set(
                wanted
                + roster["other103"]
                + roster["prune_controls7"]
                + roster["targeted_prune_controls3"]
            )
        )
        == 128,
        "exact separate targeted prune3 identities required",
    )
    listing = json.loads((out / "selected.json").read_text())
    suites = [
        suite
        for suite in listing["rust-suites"].values()
        if suite["binary-id"] == "tcl-runtime"
    ]
    require(len(suites) == 1, "exact actual runtime ELF suite required")
    suite = suites[0]
    require(
        suite["kind"] == "lib" and suite["status"] == "listed",
        "actual lib suite required",
    )
    require(
        all(
            name in suite["testcases"]
            and suite["testcases"][name]["kind"] == "test"
            and not suite["testcases"][name]["ignored"]
            for name in wanted
        ),
        "supplement identity absent or ignored",
    )
    selected = {
        name
        for name, case in suite["testcases"].items()
        if case["filter-match"]["status"] == "matches"
    }
    require(
        len(selected) == 3
        and not selected.intersection(wanted)
        and selected.intersection(roster["other103"])
        == set(other["repeated_original3"])
        == {
            "builtins::tests::native_expression_and_increment_follow_selected_numeric_towers"
        },
        "original3 identities or explicitly repeated103 inventory differs",
    )
    require(
        all(listed(suite, name) for name in roster["prune_controls7"])
        and not selected.intersection(roster["prune_controls7"]),
        "authored prune7 absent, ignored or substituted for original3",
    )
    require(
        all(listed(suite, name) for name in roster["targeted_prune_controls3"])
        and not selected.intersection(roster["targeted_prune_controls3"]),
        "targeted prune3 absent, ignored or substituted for original3",
    )
    binary = Path(suite["binary-path"]).resolve()
    target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    require(
        binary.is_file() and binary.is_relative_to(target),
        "actual fresh-target ELF required",
    )
    return roster, suite, binary


def prepare(out, source):
    require(
        not (out / "runtime-elf-before-original3.json").exists(),
        "exclusive before-original3 ELF seal required",
    )
    _roster, suite, binary = selection(out)
    save(
        out / "runtime-elf-before-original3.json",
        {
            "source": source,
            "binary": str(binary),
            "elf_sha256": sha(binary),
            "listing_sha256": sha(out / "selected.json"),
            "cwd": suite["cwd"],
            "original_selected_count": 3,
            "supplemental14_count": 14,
            "separate_stack_count": 1,
            "separate_other103_count": 103,
            "separate_prune_control_count": 7,
            "prune_control_source": prune_control_source(),
            "separate_targeted_prune_control_count": 3,
            "targeted_prune_control_source": targeted_prune_control_source(),
            "other103_listing_unavailable": [
                name for name in _roster["other103"] if not listed(suite, name)
            ],
            "other103_roster_sha256": OTHER_ROSTER_SHA,
            "original_failure_log_sha256": FAILURE_LOG_SHA,
        },
    )

    save(
        out / "supplement-plan.json",
        [
            {"group": group, "selector": name, "status": "UNRUN"}
            for group in GROUPS
            for name in _roster[group]
        ],
    )
    save(
        out / "prune-controls-plan.json",
        [
            {"group": "prune_controls7", "selector": name, "status": "UNRUN"}
            for name in PRUNE_CONTROLS
        ],
    )
    save(
        out / "targeted-prune-controls-plan.json",
        [
            {"group": "targeted_prune_controls3", "selector": name, "status": "UNRUN"}
            for name in TARGETED_PRUNE_CONTROLS
        ],
    )
    save(
        out / "original3-plan.json",
        {
            "source": source,
            "rows": [
                {"selector": name, "status": "UNRUN"}
                for name, case in suite["testcases"].items()
                if case["filter-match"]["status"] == "matches"
            ],
            "scope": "Planned inventory only; actual original3 terminal events remain authoritative.",
        },
    )


def provider_log_bytes(out):
    paths = [
        out / "c84-provider/bootstrap.stdout",
        out / "c84-provider/bootstrap.stderr",
    ]
    return sum(path.stat().st_size for path in paths if path.is_file())


def stack_streams(out):
    streams = {}
    for name in ["worker.stdout", "worker.stderr"]:
        path = out / "stack-frames" / name
        streams[name] = {
            "bytes": path.stat().st_size if path.exists() else 0,
            "sha256": sha(path) if path.exists() else None,
        }
    return streams


def frames(out, source):
    _roster, _suite, binary = selection(out)
    prepared = json.loads((out / "runtime-elf-before-original3.json").read_text())
    require(
        prepared["source"] == source
        and prepared["binary"] == str(binary)
        and prepared["elf_sha256"] == sha(binary)
        and prepared["listing_sha256"] == sha(out / "selected.json")
        and prepared["prune_control_source"] == prune_control_source()
        and prepared["targeted_prune_control_source"]
        == targeted_prune_control_source(),
        "same prepared original3 ELF and listing required before inspection",
    )
    helper = BASE / "runtime-stack-frames.py"
    require(sha(helper) == STACK_FRAMES_SHA, "reviewed stack metadata helper changed")
    spec = importlib.util.spec_from_file_location("runtime_stack_metadata", helper)
    if spec is None or spec.loader is None:
        raise RuntimeError("stack metadata helper loader unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    build_bytes = (out / "build.stderr").stat().st_size + provider_log_bytes(out)
    record = module.capture_stack_frames(
        out=out,
        binary=binary,
        source=source,
        expected_elf_sha256=prepared["elf_sha256"],
        environment=os.environ.copy(),
        absolute_deadline=float(os.environ["RUNTIME_DIAGNOSTIC_DEADLINE"]),
        remaining_log_bytes=max(0, 20 * 1024**2 - build_bytes),
    )
    streams = stack_streams(out)
    retained = sum(stream["bytes"] for stream in streams.values())
    process = record.get("process")
    closed = (
        record.get("status") == "UNRUN"
        if process is None
        else isinstance(process, dict)
        and process.get("group_empty_before_parent_reap") is True
        and process.get("consecutive_complete_empty_group_scans", 0) >= 2
        and process.get("owned_live_group_members") == []
    )
    unchanged = sha(binary) == prepared["elf_sha256"]
    complete = (
        record["source"] == source
        and record["binary"] == str(binary)
        and record["expected_elf_sha256"] == prepared["elf_sha256"]
        and record["status"] == "CAPTURED"
        and record["capture_incomplete"] is False
        and type(record["retained_bytes"]) is int
        and record["retained_bytes"] == retained
        and isinstance(process, dict)
        and closed
        and unchanged
    )
    save(
        out / "stack-frame-accounting.json",
        {
            "source": source,
            "elf_sha256": prepared["elf_sha256"],
            "helper_sha256": STACK_FRAMES_SHA,
            "summary_sha256": sha(out / "stack-frames/summary.json"),
            "streams": streams,
            "actual_retained_log_bytes": retained,
            "reported_retained_bytes": record["retained_bytes"],
            "owned_group_closed": closed,
            "elf_unchanged": unchanged,
            "complete": complete,
            "scope": "Static metadata only, not active stack/depth or a stack fix. Unknown/omitted data stays incomplete. Actual retained bytes join the existing combined budget; original3 stream caps stay unchanged and do not guarantee a retroactive aggregate cap.",
        },
    )
    if not closed or not unchanged:
        return 126
    return 0 if complete else 125


def original_outcome(stdout, receipt):
    code = receipt["exit_code"]
    if receipt["stop_reason"] == "selector_timeout":
        return "TIMEOUT"
    if not receipt["complete"]:
        return "INCOMPLETE"
    if code < 0:
        return "ABORT"
    terminals = re.findall(
        r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;",
        stdout,
        re.MULTILINE,
    )
    require(
        len(terminals) == 1 and "running 1 test" in stdout,
        "exact one-test terminal missing",
    )
    label, passed, failed, ignored, measured, _filtered = terminals[0]
    require(
        (label, int(passed), int(failed), int(ignored), int(measured), code)
        in [("ok", 1, 0, 0, 0, 0), ("FAILED", 0, 1, 0, 0, 101)],
        "exact terminal/exit disagreement",
    )
    return "PASS" if label == "ok" else "FAIL"


def proc_stack_source():
    path = Path("runtime/rust/src/interp.rs")
    pattern = r"(?:^    #\[[^\n]*\]\n)+^    fn unbounded_proc_recursion_errors_instead_of_crashing\(\) \{\n.*?^    }\n"
    found = re.findall(pattern, path.read_text(), re.MULTILINE | re.DOTALL)
    require(
        len(found) == 1
        and hashlib.sha256(found[0].encode()).hexdigest() == PROC_STACK_SOURCE_SHA,
        "unchanged original unbounded procedure recursion assertion required",
    )
    return {"path": str(path), "source_sha256": sha(path),
            "function_sha256": PROC_STACK_SOURCE_SHA}


def attestation_control_source():
    path = Path("runtime/rust/src/interp.rs")
    raw = path.read_text()
    for selector, expected in ATTESTATION_CONTROLS.items():
        simple = re.escape(selector.rsplit("::", 1)[1])
        pattern = r"(?:^    #\[[^\n]*\]\n)+^    fn " + simple + r"\(\) \{\n.*?^    }\n"
        found = re.findall(pattern, raw, re.MULTILINE | re.DOTALL)
        require(len(found) == 1 and hashlib.sha256(found[0].encode()).hexdigest() == expected,
                "unchanged reviewed attestation control source required: " + selector)
    return {"path": str(path), "source_sha256": sha(path), "functions": ATTESTATION_CONTROLS}


def run_aot5(owner, suite, binary, prepared, environment, supplemental, source,
             used, suite_deadline, absolute, prior_closed):
    rows = [{"group": "aot5", "selector": name, "status": "UNRUN"}
            for name in AOT5_SELECTORS]
    errors = []
    save(supplemental / "aot5-outcomes.json", rows)
    source_provenance = {"path": AOT5_SOURCE_PATH, "expected_sha256": AOT5_SOURCE_SHA}
    require(len(AOT5_SELECTORS) == len(set(AOT5_SELECTORS)) == 5, "exact distinct AOT5 selectors required")
    require(sha(AOT5_SOURCE_PATH) == AOT5_SOURCE_SHA, "reviewed actual AOT5 source changed")
    source_provenance["actual_sha256"] = sha(AOT5_SOURCE_PATH)
    if not prior_closed:
        errors.append("original135 process/ELF/completeness unsettled; AOT5 remains UNRUN")
    else:
        for index, row in enumerate(rows):
            if not listed(suite, row["selector"]):
                errors.append("reviewed AOT5 selector absent or ignored")
                break
            if suite["testcases"][row["selector"]]["filter-match"]["status"] == "matches":
                errors.append("AOT5 cannot substitute an original selected identity")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable for AOT5")
                break
            if used >= CAP:
                errors.append("remaining shared 20MiB log budget exhausted before AOT5")
                break
            if sha(binary) != prepared["elf_sha256"]:
                errors.append("same original3 ELF changed before AOT5")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
                break
            prefix = supplemental / f"aot5-{index}"
            receipt = owner.run(
                [str(binary), "--exact", row["selector"], "--nocapture", "--test-threads=1"],
                cwd=Path(suite["cwd"]), env=environment,
                stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
                receipt_path=str(prefix) + ".process.json", artifact_path=binary,
                deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
                disk_paths=(Path.cwd(), supplemental.parent, binary.parent, Path(os.environ["RUNNER_TEMP"])),
            )
            used += receipt["retained_bytes"]
            try:
                row["status"] = original_outcome(Path(str(prefix) + ".stdout").read_text(), receipt)
            except (RuntimeError, ValueError) as error:
                row["status"] = "INCOMPLETE"
                row["error"] = str(error)
            row["process"] = receipt
            row["stdout_sha256"] = sha(str(prefix) + ".stdout")
            row["stderr_sha256"] = sha(str(prefix) + ".stderr")
            save(supplemental / "aot5-outcomes.json", rows)
            print(row["group"], row["selector"], row["status"], flush=True)
            if (not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]
                or row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]):
                errors.append("AOT5 process/ELF/completeness guard failed")
                break
            if time.monotonic() > suite_deadline or time.time() > absolute:
                errors.append("shared suite or absolute deadline exceeded after AOT5")
                break
    for row in rows:
        if row["status"] == "UNRUN":
            row["unrun_reasons"] = errors
    save(supplemental / "aot5-outcomes.json", rows)
    save(supplemental / "aot5-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": source_provenance, "count": 5,
        "counts": dict(collections.Counter(row["status"] for row in rows)),
        "errors": errors, "complete": not errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate five actual AOT controls after unchanged original135; same fresh ELF/owner/env and remaining budgets, no build/reset/stack increase or original failure substitution.",
    })
    return rows, errors, used


def jim_library3_control_source():
    for selector, control in JIM_LIBRARY3_CONTROLS.items():
        raw = Path(control["path"]).read_text()
        indent = re.escape(control["indent"])
        simple = re.escape(selector.rsplit("::", 1)[1])
        pattern = (r"(?:^" + indent + r"#\[[^\n]*\]\n)+^" + indent
                   + r"fn " + simple + r"\(\) \{\n.*?^" + indent + r"}\n")
        found = re.findall(pattern, raw, re.MULTILINE | re.DOTALL)
        require(len(found) == 1
                and hashlib.sha256(found[0].encode()).hexdigest() == control["sha256"],
                "reviewed Jim core/library control required: " + selector)
    return JIM_LIBRARY3_CONTROLS

def run_jim_library3(owner, suite, binary, prepared, environment, supplemental, source,
             used, suite_deadline, absolute, prior_closed):
    rows = [{"group": "jim_library3", "selector": name, "status": "UNRUN"}
            for name in JIM_LIBRARY3_SELECTORS]
    errors = []
    save(supplemental / "jim_library3-outcomes.json", rows)
    source_provenance = jim_library3_control_source()
    require(len(JIM_LIBRARY3_SELECTORS) == len(set(JIM_LIBRARY3_SELECTORS)) == 3,
            "exact distinct jim_library3 selectors required")
    if not prior_closed:
        errors.append("original161 process/ELF/completeness unsettled; jim_library3 remains UNRUN")
    else:
        for index, row in enumerate(rows):
            if not listed(suite, row["selector"]):
                errors.append("reviewed jim_library3 selector absent or ignored")
                break
            if suite["testcases"][row["selector"]]["filter-match"]["status"] == "matches":
                errors.append("jim_library3 cannot substitute an original selected identity")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable for jim_library3")
                break
            if used >= CAP:
                errors.append("remaining shared 20MiB log budget exhausted before jim_library3")
                break
            if sha(binary) != prepared["elf_sha256"]:
                errors.append("same original3 ELF changed before jim_library3")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
                break
            prefix = supplemental / f"jim_library3-{index}"
            receipt = owner.run(
                [str(binary), "--exact", row["selector"], "--nocapture", "--test-threads=1"],
                cwd=Path(suite["cwd"]), env=environment,
                stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
                receipt_path=str(prefix) + ".process.json", artifact_path=binary,
                deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
                disk_paths=(Path.cwd(), supplemental.parent, binary.parent, Path(os.environ["RUNNER_TEMP"])),
            )
            used += receipt["retained_bytes"]
            try:
                row["status"] = original_outcome(Path(str(prefix) + ".stdout").read_text(), receipt)
            except (RuntimeError, ValueError) as error:
                row["status"] = "INCOMPLETE"
                row["error"] = str(error)
            row["process"] = receipt
            row["stdout_sha256"] = sha(str(prefix) + ".stdout")
            row["stderr_sha256"] = sha(str(prefix) + ".stderr")
            save(supplemental / "jim_library3-outcomes.json", rows)
            print(row["group"], row["selector"], row["status"], flush=True)
            if (not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]
                or row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]):
                errors.append("jim_library3 process/ELF/completeness guard failed")
                break
            if time.monotonic() > suite_deadline or time.time() > absolute:
                errors.append("shared suite or absolute deadline exceeded after jim_library3")
                break
    for row in rows:
        if row["status"] == "UNRUN":
            row["unrun_reasons"] = errors
    save(supplemental / "jim_library3-outcomes.json", rows)
    save(supplemental / "jim_library3-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": source_provenance, "count": 3,
        "counts": dict(collections.Counter(row["status"] for row in rows)),
        "errors": errors, "complete": not errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate original constructor and two genuine Jim source-library controls after unchanged original161; same fresh ELF/owner/env and remaining budgets, no build/reset/stack increase or original failure substitution.",
    })
    return rows, errors, used



def native_getter11_control_source():
    for selector, control in NATIVE_GETTER11_CONTROLS.items():
        raw = Path(control["path"]).read_text()
        indent = re.escape(control["indent"])
        simple = re.escape(selector.rsplit("::", 1)[1])
        pattern = (r"(?:^" + indent + r"#\[[^\n]*\]\n)+^" + indent
                   + r"fn " + simple + r"\(\) \{\n.*?^" + indent + r"}\n")
        found = re.findall(pattern, raw, re.MULTILINE | re.DOTALL)
        require(len(found) == 1
                and hashlib.sha256(found[0].encode()).hexdigest() == control["sha256"],
                "reviewed allocation control required: " + selector)
    return NATIVE_GETTER11_CONTROLS

def run_native_getter11(owner, suite, binary, prepared, environment, supplemental, source,
             used, suite_deadline, absolute, prior_closed):
    rows = [{"group": "native_getter11", "selector": name, "status": "UNRUN"}
            for name in NATIVE_GETTER11_SELECTORS]
    errors = []
    save(supplemental / "native_getter11-outcomes.json", rows)
    source_provenance = native_getter11_control_source()
    require(len(NATIVE_GETTER11_SELECTORS) == len(set(NATIVE_GETTER11_SELECTORS)) == 11,
            "exact distinct native_getter11 selectors required")
    if not prior_closed:
        errors.append("original150 process/ELF/completeness unsettled; native_getter11 remains UNRUN")
    else:
        for index, row in enumerate(rows):
            if not listed(suite, row["selector"]):
                errors.append("reviewed native_getter11 selector absent or ignored")
                break
            if suite["testcases"][row["selector"]]["filter-match"]["status"] == "matches":
                errors.append("native_getter11 cannot substitute an original selected identity")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable for native_getter11")
                break
            if used >= CAP:
                errors.append("remaining shared 20MiB log budget exhausted before native_getter11")
                break
            if sha(binary) != prepared["elf_sha256"]:
                errors.append("same original3 ELF changed before native_getter11")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
                break
            prefix = supplemental / f"native_getter11-{index}"
            receipt = owner.run(
                [str(binary), "--exact", row["selector"], "--nocapture", "--test-threads=1"],
                cwd=Path(suite["cwd"]), env=environment,
                stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
                receipt_path=str(prefix) + ".process.json", artifact_path=binary,
                deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
                disk_paths=(Path.cwd(), supplemental.parent, binary.parent, Path(os.environ["RUNNER_TEMP"])),
            )
            used += receipt["retained_bytes"]
            try:
                row["status"] = original_outcome(Path(str(prefix) + ".stdout").read_text(), receipt)
            except (RuntimeError, ValueError) as error:
                row["status"] = "INCOMPLETE"
                row["error"] = str(error)
            row["process"] = receipt
            row["stdout_sha256"] = sha(str(prefix) + ".stdout")
            row["stderr_sha256"] = sha(str(prefix) + ".stderr")
            save(supplemental / "native_getter11-outcomes.json", rows)
            print(row["group"], row["selector"], row["status"], flush=True)
            if (not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]
                or row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]):
                errors.append("native_getter11 process/ELF/completeness guard failed")
                break
            if time.monotonic() > suite_deadline or time.time() > absolute:
                errors.append("shared suite or absolute deadline exceeded after native_getter11")
                break
    for row in rows:
        if row["status"] == "UNRUN":
            row["unrun_reasons"] = errors
    save(supplemental / "native_getter11-outcomes.json", rows)
    save(supplemental / "native_getter11-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": source_provenance, "count": 11,
        "counts": dict(collections.Counter(row["status"] for row in rows)),
        "errors": errors, "complete": not errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate eleven approved character getter and preserved UTF8 controls after unchanged original150; same fresh ELF/owner/env and remaining budgets, no build/reset/stack increase or original failure substitution.",
    })
    return rows, errors, used


def allocation2_control_source():
    for selector, control in ALLOCATION2_CONTROLS.items():
        raw = Path(control["path"]).read_text()
        indent = re.escape(control["indent"])
        simple = re.escape(selector.rsplit("::", 1)[1])
        pattern = (r"(?:^" + indent + r"#\[[^\n]*\]\n)+^" + indent
                   + r"fn " + simple + r"\(\) \{\n.*?^" + indent + r"}\n")
        found = re.findall(pattern, raw, re.MULTILINE | re.DOTALL)
        require(len(found) == 1
                and hashlib.sha256(found[0].encode()).hexdigest() == control["sha256"],
                "reviewed allocation control required: " + selector)
    return ALLOCATION2_CONTROLS

def run_allocation2(owner, suite, binary, prepared, environment, supplemental, source,
             used, suite_deadline, absolute, prior_closed):
    rows = [{"group": "allocation2", "selector": name, "status": "UNRUN"}
            for name in ALLOCATION2_SELECTORS]
    errors = []
    save(supplemental / "allocation2-outcomes.json", rows)
    source_provenance = allocation2_control_source()
    require(len(ALLOCATION2_SELECTORS) == len(set(ALLOCATION2_SELECTORS)) == 2,
            "exact distinct allocation2 selectors required")
    if not prior_closed:
        errors.append("original148 process/ELF/completeness unsettled; allocation2 remains UNRUN")
    else:
        for index, row in enumerate(rows):
            if not listed(suite, row["selector"]):
                errors.append("reviewed allocation2 selector absent or ignored")
                break
            if suite["testcases"][row["selector"]]["filter-match"]["status"] == "matches":
                errors.append("allocation2 cannot substitute an original selected identity")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable for allocation2")
                break
            if used >= CAP:
                errors.append("remaining shared 20MiB log budget exhausted before allocation2")
                break
            if sha(binary) != prepared["elf_sha256"]:
                errors.append("same original3 ELF changed before allocation2")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
                break
            prefix = supplemental / f"allocation2-{index}"
            receipt = owner.run(
                [str(binary), "--exact", row["selector"], "--nocapture", "--test-threads=1"],
                cwd=Path(suite["cwd"]), env=environment,
                stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
                receipt_path=str(prefix) + ".process.json", artifact_path=binary,
                deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
                disk_paths=(Path.cwd(), supplemental.parent, binary.parent, Path(os.environ["RUNNER_TEMP"])),
            )
            used += receipt["retained_bytes"]
            try:
                row["status"] = original_outcome(Path(str(prefix) + ".stdout").read_text(), receipt)
            except (RuntimeError, ValueError) as error:
                row["status"] = "INCOMPLETE"
                row["error"] = str(error)
            row["process"] = receipt
            row["stdout_sha256"] = sha(str(prefix) + ".stdout")
            row["stderr_sha256"] = sha(str(prefix) + ".stderr")
            save(supplemental / "allocation2-outcomes.json", rows)
            print(row["group"], row["selector"], row["status"], flush=True)
            if (not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]
                or row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]):
                errors.append("allocation2 process/ELF/completeness guard failed")
                break
            if time.monotonic() > suite_deadline or time.time() > absolute:
                errors.append("shared suite or absolute deadline exceeded after allocation2")
                break
    for row in rows:
        if row["status"] == "UNRUN":
            row["unrun_reasons"] = errors
    save(supplemental / "allocation2-outcomes.json", rows)
    save(supplemental / "allocation2-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": source_provenance, "count": 2,
        "counts": dict(collections.Counter(row["status"] for row in rows)),
        "errors": errors, "complete": not errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate two owned teardown and original-header controls after unchanged original148; same fresh ELF/owner/env and remaining budgets, no build/reset/stack increase or original failure substitution.",
    })
    return rows, errors, used


def headroom2_control_source():
    for selector, control in HEADROOM2_CONTROLS.items():
        raw = Path(control["path"]).read_text()
        indent = re.escape(control["indent"])
        simple = re.escape(selector.rsplit("::", 1)[1])
        pattern = (r"(?:^" + indent + r"#\[[^\n]*\]\n)+^" + indent
                   + r"fn " + simple + r"\(\) \{\n.*?^" + indent + r"}\n")
        found = re.findall(pattern, raw, re.MULTILINE | re.DOTALL)
        require(len(found) == 1
                and hashlib.sha256(found[0].encode()).hexdigest() == control["sha256"],
                "unchanged reviewed constructor control required: " + selector)
    return HEADROOM2_CONTROLS

def run_headroom2(owner, suite, binary, prepared, environment, supplemental, source,
             used, suite_deadline, absolute, prior_closed):
    rows = [{"group": "headroom2", "selector": name, "status": "UNRUN"}
            for name in HEADROOM2_SELECTORS]
    errors = []
    save(supplemental / "headroom2-outcomes.json", rows)
    source_provenance = headroom2_control_source()
    require(len(HEADROOM2_SELECTORS) == len(set(HEADROOM2_SELECTORS)) == 2,
            "exact distinct headroom2 selectors required")
    if not prior_closed:
        errors.append("original146 process/ELF/completeness unsettled; headroom2 remains UNRUN")
    else:
        for index, row in enumerate(rows):
            if not listed(suite, row["selector"]):
                errors.append("reviewed headroom2 selector absent or ignored")
                break
            if suite["testcases"][row["selector"]]["filter-match"]["status"] == "matches":
                errors.append("headroom2 cannot substitute an original selected identity")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable for headroom2")
                break
            if used >= CAP:
                errors.append("remaining shared 20MiB log budget exhausted before headroom2")
                break
            if sha(binary) != prepared["elf_sha256"]:
                errors.append("same original3 ELF changed before headroom2")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
                break
            prefix = supplemental / f"headroom2-{index}"
            receipt = owner.run(
                [str(binary), "--exact", row["selector"], "--nocapture", "--test-threads=1"],
                cwd=Path(suite["cwd"]), env=environment,
                stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
                receipt_path=str(prefix) + ".process.json", artifact_path=binary,
                deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
                disk_paths=(Path.cwd(), supplemental.parent, binary.parent, Path(os.environ["RUNNER_TEMP"])),
            )
            used += receipt["retained_bytes"]
            try:
                row["status"] = original_outcome(Path(str(prefix) + ".stdout").read_text(), receipt)
            except (RuntimeError, ValueError) as error:
                row["status"] = "INCOMPLETE"
                row["error"] = str(error)
            row["process"] = receipt
            row["stdout_sha256"] = sha(str(prefix) + ".stdout")
            row["stderr_sha256"] = sha(str(prefix) + ".stderr")
            save(supplemental / "headroom2-outcomes.json", rows)
            print(row["group"], row["selector"], row["status"], flush=True)
            if (not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]
                or row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]):
                errors.append("headroom2 process/ELF/completeness guard failed")
                break
            if time.monotonic() > suite_deadline or time.time() > absolute:
                errors.append("shared suite or absolute deadline exceeded after headroom2")
                break
    for row in rows:
        if row["status"] == "UNRUN":
            row["unrun_reasons"] = errors
    save(supplemental / "headroom2-outcomes.json", rows)
    save(supplemental / "headroom2-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": source_provenance, "count": 2,
        "counts": dict(collections.Counter(row["status"] for row in rows)),
        "errors": errors, "complete": not errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate two generated entry headroom controls after unchanged original146; same fresh ELF/owner/env and remaining budgets, no build/reset/stack increase or original failure substitution.",
    })
    return rows, errors, used


def ordinary_entry3_control_source():
    for selector, control in ORDINARY_ENTRY3_CONTROLS.items():
        raw = Path(control["path"]).read_text()
        indent = re.escape(control["indent"])
        simple = re.escape(selector.rsplit("::", 1)[1])
        pattern = (r"(?:^" + indent + r"#\[[^\n]*\]\n)+^" + indent
                   + r"fn " + simple + r"\(\) \{\n.*?^" + indent + r"}\n")
        found = re.findall(pattern, raw, re.MULTILINE | re.DOTALL)
        require(len(found) == 1
                and hashlib.sha256(found[0].encode()).hexdigest() == control["sha256"],
                "unchanged reviewed constructor control required: " + selector)
    return ORDINARY_ENTRY3_CONTROLS

def run_ordinary_entry3(owner, suite, binary, prepared, environment, supplemental, source,
             used, suite_deadline, absolute, prior_closed):
    rows = [{"group": "ordinary_entry3", "selector": name, "status": "UNRUN"}
            for name in ORDINARY_ENTRY3_SELECTORS]
    errors = []
    save(supplemental / "ordinary_entry3-outcomes.json", rows)
    source_provenance = ordinary_entry3_control_source()
    require(len(ORDINARY_ENTRY3_SELECTORS) == len(set(ORDINARY_ENTRY3_SELECTORS)) == 3,
            "exact distinct ordinary_entry3 selectors required")
    if not prior_closed:
        errors.append("original143 process/ELF/completeness unsettled; ordinary_entry3 remains UNRUN")
    else:
        for index, row in enumerate(rows):
            if not listed(suite, row["selector"]):
                errors.append("reviewed ordinary_entry3 selector absent or ignored")
                break
            if suite["testcases"][row["selector"]]["filter-match"]["status"] == "matches":
                errors.append("ordinary_entry3 cannot substitute an original selected identity")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable for ordinary_entry3")
                break
            if used >= CAP:
                errors.append("remaining shared 20MiB log budget exhausted before ordinary_entry3")
                break
            if sha(binary) != prepared["elf_sha256"]:
                errors.append("same original3 ELF changed before ordinary_entry3")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
                break
            prefix = supplemental / f"ordinary_entry3-{index}"
            receipt = owner.run(
                [str(binary), "--exact", row["selector"], "--nocapture", "--test-threads=1"],
                cwd=Path(suite["cwd"]), env=environment,
                stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
                receipt_path=str(prefix) + ".process.json", artifact_path=binary,
                deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
                disk_paths=(Path.cwd(), supplemental.parent, binary.parent, Path(os.environ["RUNNER_TEMP"])),
            )
            used += receipt["retained_bytes"]
            try:
                row["status"] = original_outcome(Path(str(prefix) + ".stdout").read_text(), receipt)
            except (RuntimeError, ValueError) as error:
                row["status"] = "INCOMPLETE"
                row["error"] = str(error)
            row["process"] = receipt
            row["stdout_sha256"] = sha(str(prefix) + ".stdout")
            row["stderr_sha256"] = sha(str(prefix) + ".stderr")
            save(supplemental / "ordinary_entry3-outcomes.json", rows)
            print(row["group"], row["selector"], row["status"], flush=True)
            if (not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]
                or row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]):
                errors.append("ordinary_entry3 process/ELF/completeness guard failed")
                break
            if time.monotonic() > suite_deadline or time.time() > absolute:
                errors.append("shared suite or absolute deadline exceeded after ordinary_entry3")
                break
    for row in rows:
        if row["status"] == "UNRUN":
            row["unrun_reasons"] = errors
    save(supplemental / "ordinary_entry3-outcomes.json", rows)
    save(supplemental / "ordinary_entry3-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": source_provenance, "count": 3,
        "counts": dict(collections.Counter(row["status"] for row in rows)),
        "errors": errors, "complete": not errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate three actual observer issuer controls after unchanged original143; same fresh ELF/owner/env and remaining budgets, no build/reset/stack increase or original failure substitution.",
    })
    return rows, errors, used


def constructor3_control_source():
    for selector, control in CONSTRUCTOR3_CONTROLS.items():
        raw = Path(control["path"]).read_text()
        indent = re.escape(control["indent"])
        simple = re.escape(selector.rsplit("::", 1)[1])
        pattern = (r"(?:^" + indent + r"#\[[^\n]*\]\n)+^" + indent
                   + r"fn " + simple + r"\(\) \{\n.*?^" + indent + r"}\n")
        found = re.findall(pattern, raw, re.MULTILINE | re.DOTALL)
        require(len(found) == 1
                and hashlib.sha256(found[0].encode()).hexdigest() == control["sha256"],
                "unchanged reviewed constructor control required: " + selector)
    return CONSTRUCTOR3_CONTROLS


def run_constructor3(owner, suite, binary, prepared, environment, supplemental, source,
             used, suite_deadline, absolute, prior_closed):
    rows = [{"group": "constructor3", "selector": name, "status": "UNRUN"}
            for name in CONSTRUCTOR3_SELECTORS]
    errors = []
    save(supplemental / "constructor3-outcomes.json", rows)
    source_provenance = constructor3_control_source()
    require(len(CONSTRUCTOR3_SELECTORS) == len(set(CONSTRUCTOR3_SELECTORS)) == 3,
            "exact distinct constructor3 selectors required")
    if not prior_closed:
        errors.append("original140 process/ELF/completeness unsettled; constructor3 remains UNRUN")
    else:
        for index, row in enumerate(rows):
            if not listed(suite, row["selector"]):
                errors.append("reviewed constructor3 selector absent or ignored")
                break
            if suite["testcases"][row["selector"]]["filter-match"]["status"] == "matches":
                errors.append("constructor3 cannot substitute an original selected identity")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable for constructor3")
                break
            if used >= CAP:
                errors.append("remaining shared 20MiB log budget exhausted before constructor3")
                break
            if sha(binary) != prepared["elf_sha256"]:
                errors.append("same original3 ELF changed before constructor3")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
                break
            prefix = supplemental / f"constructor3-{index}"
            receipt = owner.run(
                [str(binary), "--exact", row["selector"], "--nocapture", "--test-threads=1"],
                cwd=Path(suite["cwd"]), env=environment,
                stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
                receipt_path=str(prefix) + ".process.json", artifact_path=binary,
                deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
                disk_paths=(Path.cwd(), supplemental.parent, binary.parent, Path(os.environ["RUNNER_TEMP"])),
            )
            used += receipt["retained_bytes"]
            try:
                row["status"] = original_outcome(Path(str(prefix) + ".stdout").read_text(), receipt)
            except (RuntimeError, ValueError) as error:
                row["status"] = "INCOMPLETE"
                row["error"] = str(error)
            row["process"] = receipt
            row["stdout_sha256"] = sha(str(prefix) + ".stdout")
            row["stderr_sha256"] = sha(str(prefix) + ".stderr")
            save(supplemental / "constructor3-outcomes.json", rows)
            print(row["group"], row["selector"], row["status"], flush=True)
            if (not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]
                or row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]):
                errors.append("constructor3 process/ELF/completeness guard failed")
                break
            if time.monotonic() > suite_deadline or time.time() > absolute:
                errors.append("shared suite or absolute deadline exceeded after constructor3")
                break
    for row in rows:
        if row["status"] == "UNRUN":
            row["unrun_reasons"] = errors
    save(supplemental / "constructor3-outcomes.json", rows)
    save(supplemental / "constructor3-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": source_provenance, "count": 3,
        "counts": dict(collections.Counter(row["status"] for row in rows)),
        "errors": errors, "complete": not errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate three genuine constructor controls after unchanged original140; same fresh ELF/owner/env and remaining budgets, no build/reset/stack increase or original failure substitution.",
    })
    return rows, errors, used


def run(out, source):
    supplemental = out / "supplement"
    supplemental.mkdir()
    roster, _other = inventories()
    rows = [
        {"group": group, "selector": name, "status": "UNRUN"}
        for group in GROUPS
        for name in roster[group]
    ]
    save(supplemental / "outcomes.json", rows)
    proc_row = {"group": "proc_stack1", "selector": PROC_STACK_SELECTOR, "status": "UNRUN"}
    save(supplemental / "proc-stack-outcome.json", proc_row)
    attestation_rows = [
        {"group": "attestation3", "selector": name, "status": "UNRUN"}
        for name in ATTESTATION_CONTROLS
    ]
    save(supplemental / "attestation3-outcomes.json", attestation_rows)
    save(
        supplemental / "prune-controls-outcomes.json",
        [row for row in rows if row["group"] == "prune_controls7"],
    )
    save(
        supplemental / "targeted-prune-controls-outcomes.json",
        [row for row in rows if row["group"] == "targeted_prune_controls3"],
    )
    roster, suite, binary = selection(out)
    errors = []
    started = time.monotonic()
    suite_deadline = started + 2400
    absolute = float(os.environ["RUNTIME_DIAGNOSTIC_DEADLINE"])
    require(
        time.time() < absolute <= time.time() + 3600,
        "actual bounded workflow deadline required",
    )
    prepared = json.loads((out / "runtime-elf-before-original3.json").read_text())
    require(
        prepared["source"] == source
        and prepared["binary"] == str(binary)
        and prepared["elf_sha256"] == sha(binary),
        "same original3 ELF required",
    )
    require(
        prepared["listing_sha256"] == sha(out / "selected.json")
        and prepared["prune_control_source"] == prune_control_source()
        and prepared["targeted_prune_control_source"]
        == targeted_prune_control_source(),
        "original full listing changed",
    )
    original = json.loads((out / "diagnostic-completeness.json").read_text())
    require(not original["incomplete"], "original3 capture incomplete")
    executor = BASE / "runtime-diagnostics-exec.py"
    require(sha(executor) == EXECUTOR_SHA, "reviewed owned-process executor changed")
    spec = importlib.util.spec_from_file_location("runtime_supplement_owner", executor)
    if spec is None or spec.loader is None:
        raise RuntimeError("process owner loader unavailable")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    used = sum(
        path.stat().st_size
        for path in [
            out / "build.stderr",
            out / "terminal/nextest-stderr.txt",
            out / "terminal/reporter.raw.jsonl",
        ]
    )
    used += provider_log_bytes(out)
    frame_streams = stack_streams(out)
    used += sum(stream["bytes"] for stream in frame_streams.values())
    accounting_path = out / "stack-frame-accounting.json"
    if not accounting_path.exists():
        errors.append("stack metadata accounting missing")
    else:
        accounting = json.loads(accounting_path.read_text())
        if (
            accounting["source"] != source
            or accounting["elf_sha256"] != prepared["elf_sha256"]
            or accounting["helper_sha256"] != STACK_FRAMES_SHA
            or accounting["streams"] != frame_streams
            or accounting["summary_sha256"] != sha(out / "stack-frames/summary.json")
            or accounting["complete"] is not True
        ):
            errors.append("stack metadata unavailable, changed or incomplete")
    environment = os.environ.copy()
    environment["TCL_LSP_RUNTIME_BASELINE_DIAGNOSTIC"] = "1"
    for index, row in enumerate(rows):
        if not listed(suite, row["selector"]):
            row["unrun_reason"] = (
                "original selector absent or ignored in fresh ELF listing"
            )
            errors.append("fresh listing does not authenticate " + row["selector"])
            save(supplemental / "outcomes.json", rows)
            save(
                supplemental / "prune-controls-outcomes.json",
                [row for row in rows if row["group"] == "prune_controls7"],
            )
            save(
                supplemental / "targeted-prune-controls-outcomes.json",
                [row for row in rows if row["group"] == "targeted_prune_controls3"],
            )
            continue
        if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
            errors.append(
                "insufficient remaining time for unchanged 180s case plus 20s cleanup and 5s preflight reserve"
            )
            break
        if used >= CAP:
            errors.append("aggregate 20MiB log budget exhausted")
            break
        if sha(binary) != prepared["elf_sha256"]:
            errors.append("actual original3 ELF changed")
            break
        if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
            errors.append("full case/cleanup reserve unavailable after ELF hash")
            break
        prefix = supplemental / f"case-{index:02}"
        receipt = owner.run(
            [
                str(binary),
                "--exact",
                row["selector"],
                "--nocapture",
                "--test-threads=1",
            ],
            cwd=Path(suite["cwd"]),
            env=environment,
            stdout_path=str(prefix) + ".stdout",
            stderr_path=str(prefix) + ".stderr",
            receipt_path=str(prefix) + ".process.json",
            artifact_path=binary,
            deadline_seconds=180,
            cap_bytes=CAP - used,
            disk_floor_bytes=FLOOR,
            disk_paths=(
                Path.cwd(),
                out,
                binary.parent,
                Path(os.environ["RUNNER_TEMP"]),
            ),
        )
        used += receipt["retained_bytes"]
        try:
            row["status"] = original_outcome(
                Path(str(prefix) + ".stdout").read_text(), receipt
            )
        except (RuntimeError, ValueError) as error:
            row["status"] = "INCOMPLETE"
            row["error"] = str(error)
        row["process"] = receipt
        row["stdout_sha256"] = sha(str(prefix) + ".stdout")
        row["stderr_sha256"] = sha(str(prefix) + ".stderr")
        save(supplemental / "outcomes.json", rows)
        save(
            supplemental / "prune-controls-outcomes.json",
            [row for row in rows if row["group"] == "prune_controls7"],
        )
        save(
            supplemental / "targeted-prune-controls-outcomes.json",
            [row for row in rows if row["group"] == "targeted_prune_controls3"],
        )
        print(row["group"], row["selector"], row["status"], flush=True)
        if time.monotonic() > suite_deadline or time.time() > absolute:
            errors.append("suite or absolute diagnostic deadline exceeded after case")
            break
        if (
            not receipt["group_empty_before_parent_reap"]
            or not receipt["artifact_unchanged"]
            or row["status"] == "INCOMPLETE"
        ):
            errors.append("owned process/artifact/completeness guard failed")
            break
    # Separate original procedure control after original3 plus all old128.
    # Reuse their exact ELF, owner, environment and remaining aggregate budgets.
    proc_errors = []
    proc_source = proc_stack_source()
    if not all(row["status"] in ["PASS", "FAIL", "ABORT"] for row in rows):
        proc_errors.append("old131 execution incomplete; separate proc control remains UNRUN")
    elif any(not row["process"]["group_empty_before_parent_reap"]
             or not row["process"]["artifact_unchanged"] for row in rows):
        proc_errors.append("old131 process or ELF guard failed")
    elif not listed(suite, PROC_STACK_SELECTOR):
        proc_errors.append("unchanged original proc selector absent or ignored")
    elif suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
        proc_errors.append("remaining shared case/cleanup reserve unavailable for proc control")
    elif used >= CAP:
        proc_errors.append("remaining shared 20MiB log budget exhausted before proc control")
    elif sha(binary) != prepared["elf_sha256"]:
        proc_errors.append("same original3 ELF changed before proc control")
    elif suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
        proc_errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
    else:
        prefix = supplemental / "proc-stack"
        receipt = owner.run(
            [str(binary), "--exact", PROC_STACK_SELECTOR, "--nocapture", "--test-threads=1"],
            cwd=Path(suite["cwd"]), env=environment,
            stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
            receipt_path=str(prefix) + ".process.json", artifact_path=binary,
            deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
            disk_paths=(Path.cwd(), out, binary.parent, Path(os.environ["RUNNER_TEMP"])),
        )
        used += receipt["retained_bytes"]
        try:
            proc_row["status"] = original_outcome(
                Path(str(prefix) + ".stdout").read_text(), receipt)
        except (RuntimeError, ValueError) as error:
            proc_row["status"] = "INCOMPLETE"
            proc_row["error"] = str(error)
        proc_row["process"] = receipt
        proc_row["stdout_sha256"] = sha(str(prefix) + ".stdout")
        proc_row["stderr_sha256"] = sha(str(prefix) + ".stderr")
        if not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]:
            proc_errors.append("separate proc process or ELF guard failed")
        if proc_row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]:
            proc_errors.append("separate proc execution incomplete")
        if time.monotonic() > suite_deadline or time.time() > absolute:
            proc_errors.append("shared suite or absolute deadline exceeded after proc control")
        print(proc_row["group"], proc_row["selector"], proc_row["status"], flush=True)
    proc_row["unrun_reasons"] = proc_errors if proc_row["status"] == "UNRUN" else []
    save(supplemental / "proc-stack-outcome.json", proc_row)
    save(supplemental / "proc-stack-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": proc_source, "count": 1, "status": proc_row["status"],
        "errors": proc_errors, "complete": not proc_errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate unchanged original proc test after old131, same fresh ELF and remaining budgets; no new build, reset, stack increase or gate substitution.",
    })
    errors.extend(proc_errors)
    # Separate original baseline attestation and two owned-currentness controls.
    # The original132 and their budgets are never restarted or replaced.
    attestation_errors = []
    attestation_source = attestation_control_source()
    prior_closed = (
        all(row["status"] in ["PASS", "FAIL", "ABORT"]
            and row["process"]["group_empty_before_parent_reap"]
            and row["process"]["artifact_unchanged"] for row in rows)
        and proc_row["status"] in ["PASS", "FAIL", "ABORT"]
        and proc_row["process"]["group_empty_before_parent_reap"]
        and proc_row["process"]["artifact_unchanged"]
    )
    if not prior_closed:
        attestation_errors.append("original132 execution unsettled; attestation3 remains UNRUN")
    else:
        for index, row in enumerate(attestation_rows):
            if not listed(suite, row["selector"]):
                attestation_errors.append("reviewed attestation selector absent or ignored")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                attestation_errors.append("remaining shared case/cleanup reserve unavailable")
                break
            if used >= CAP:
                attestation_errors.append("remaining shared 20MiB log budget exhausted")
                break
            if sha(binary) != prepared["elf_sha256"]:
                attestation_errors.append("same original3 ELF changed before attestation control")
                break
            if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
                attestation_errors.append("remaining shared case/cleanup reserve unavailable after ELF hash")
                break
            prefix = supplemental / f"attestation3-{index}"
            receipt = owner.run(
                [str(binary), "--exact", row["selector"], "--nocapture", "--test-threads=1"],
                cwd=Path(suite["cwd"]), env=environment,
                stdout_path=str(prefix) + ".stdout", stderr_path=str(prefix) + ".stderr",
                receipt_path=str(prefix) + ".process.json", artifact_path=binary,
                deadline_seconds=180, cap_bytes=CAP - used, disk_floor_bytes=FLOOR,
                disk_paths=(Path.cwd(), out, binary.parent, Path(os.environ["RUNNER_TEMP"])),
            )
            used += receipt["retained_bytes"]
            try:
                row["status"] = original_outcome(Path(str(prefix) + ".stdout").read_text(), receipt)
            except (RuntimeError, ValueError) as error:
                row["status"] = "INCOMPLETE"
                row["error"] = str(error)
            row["process"] = receipt
            row["stdout_sha256"] = sha(str(prefix) + ".stdout")
            row["stderr_sha256"] = sha(str(prefix) + ".stderr")
            save(supplemental / "attestation3-outcomes.json", attestation_rows)
            print(row["group"], row["selector"], row["status"], flush=True)
            if (not receipt["group_empty_before_parent_reap"] or not receipt["artifact_unchanged"]
                or row["status"] in ["UNRUN", "INCOMPLETE", "TIMEOUT"]):
                attestation_errors.append("attestation process, ELF or completeness guard failed")
                break
            if time.monotonic() > suite_deadline or time.time() > absolute:
                attestation_errors.append("shared suite or absolute deadline exceeded")
                break
    for row in attestation_rows:
        if row["status"] == "UNRUN":
            row["unrun_reasons"] = attestation_errors
    save(supplemental / "attestation3-outcomes.json", attestation_rows)
    save(supplemental / "attestation3-summary.json", {
        "source": source, "binary": str(binary), "elf_sha256": prepared["elf_sha256"],
        "source_provenance": attestation_source, "count": 3,
        "counts": dict(collections.Counter(row["status"] for row in attestation_rows)),
        "errors": attestation_errors, "complete": not attestation_errors,
        "case_budget_seconds": 180, "suite_budget_seconds": 2400,
        "workflow_bound_seconds": 3600, "aggregate_retained_log_bytes": used,
        "aggregate_log_cap_bytes": CAP + 65536,
        "scope": "Separate baselinePF plus2 genuine owner controls after unchanged original132; same fresh ELF/owner/env and remaining budgets, no build/reset/stack increase.",
    })
    errors.extend(attestation_errors)
    aot_prior_closed = prior_closed and all(
        row["status"] in ["PASS", "FAIL", "ABORT"]
        and row["process"]["group_empty_before_parent_reap"]
        and row["process"]["artifact_unchanged"] for row in attestation_rows)
    aot_rows, aot_errors, used = run_aot5(
        owner, suite, binary, prepared, environment, supplemental, source,
        used, suite_deadline, absolute, aot_prior_closed)
    errors.extend(aot_errors)
    constructor_prior_closed = aot_prior_closed and len(aot_rows) == 5 and all(
        row["status"] in ["PASS", "FAIL", "ABORT"]
        and row["process"]["group_empty_before_parent_reap"]
        and row["process"]["artifact_unchanged"] for row in aot_rows)
    constructor_rows, constructor_errors, used = run_constructor3(
        owner, suite, binary, prepared, environment, supplemental, source,
        used, suite_deadline, absolute, constructor_prior_closed)
    errors.extend(constructor_errors)
    ordinary_entry_prior_closed = constructor_prior_closed and len(constructor_rows) == 3 and all(
        row["status"] in ["PASS", "FAIL", "ABORT"]
        and row["process"]["group_empty_before_parent_reap"]
        and row["process"]["artifact_unchanged"] for row in constructor_rows)
    ordinary_entry_rows, ordinary_entry_errors, used = run_ordinary_entry3(
        owner, suite, binary, prepared, environment, supplemental, source,
        used, suite_deadline, absolute, ordinary_entry_prior_closed)
    errors.extend(ordinary_entry_errors)
    headroom_prior_closed = ordinary_entry_prior_closed and len(ordinary_entry_rows) == 3 and all(
        row["status"] in ["PASS", "FAIL", "ABORT"]
        and row["process"]["group_empty_before_parent_reap"]
        and row["process"]["artifact_unchanged"] for row in ordinary_entry_rows)
    headroom_rows, headroom_errors, used = run_headroom2(
        owner, suite, binary, prepared, environment, supplemental, source,
        used, suite_deadline, absolute, headroom_prior_closed)
    errors.extend(headroom_errors)
    allocation_prior_closed = headroom_prior_closed and len(headroom_rows) == 2 and all(
        row["status"] in ["PASS", "FAIL", "ABORT"]
        and row["process"]["group_empty_before_parent_reap"]
        and row["process"]["artifact_unchanged"] for row in headroom_rows)
    allocation_rows, allocation_errors, used = run_allocation2(
        owner, suite, binary, prepared, environment, supplemental, source,
        used, suite_deadline, absolute, allocation_prior_closed)
    errors.extend(allocation_errors)
    native_getter_prior_closed = allocation_prior_closed and len(allocation_rows) == 2 and all(
        row["status"] in ["PASS", "FAIL", "ABORT"]
        and row["process"]["group_empty_before_parent_reap"]
        and row["process"]["artifact_unchanged"] for row in allocation_rows)
    native_getter_rows, native_getter_errors, used = run_native_getter11(
        owner, suite, binary, prepared, environment, supplemental, source,
        used, suite_deadline, absolute, native_getter_prior_closed)
    errors.extend(native_getter_errors)
    jim_library_prior_closed = native_getter_prior_closed and len(native_getter_rows) == 11 and all(
        row["status"] in ["PASS", "FAIL", "ABORT"]
        and row["process"]["group_empty_before_parent_reap"]
        and row["process"]["artifact_unchanged"] for row in native_getter_rows)
    jim_library_rows, jim_library_errors, used = run_jim_library3(
        owner, suite, binary, prepared, environment, supplemental, source,
        used, suite_deadline, absolute, jim_library_prior_closed)
    errors.extend(jim_library_errors)
    unchanged = sha(binary) == prepared["elf_sha256"]
    if not unchanged:
        errors.append("ELF changed after supplemental cases")
    counts = {
        group: dict(
            collections.Counter(row["status"] for row in rows if row["group"] == group)
        )
        for group in GROUPS
    }
    complete = not errors and all(
        row["status"] not in ["UNRUN", "INCOMPLETE", "TIMEOUT"] for row in rows
    )
    save(
        supplemental / "summary.json",
        {
            "source": source,
            "binary": str(binary),
            "elf_sha256": prepared["elf_sha256"],
            "elf_unchanged": unchanged,
            "counts": counts,
            "errors": errors,
            "complete": complete,
            "separate_jim_library3_outcomes": "jim_library3-outcomes.json",
            "separate_native_getter11_outcomes": "native_getter11-outcomes.json",
            "separate_allocation2_outcomes": "allocation2-outcomes.json",
            "separate_headroom2_outcomes": "headroom2-outcomes.json",
            "separate_ordinary_entry3_outcomes": "ordinary_entry3-outcomes.json",
            "separate_constructor3_outcomes": "constructor3-outcomes.json",
            "separate_aot5_outcomes": "aot5-outcomes.json",
            "separate_attestation3_outcomes": "attestation3-outcomes.json",
            "separate_proc_stack_outcome": "proc-stack-outcome.json",
            "original3_receipts": "../terminal/summary.json and ../process-outcomes.json remain separate",
            "suite_budget_seconds": 2400,
            "case_budget_seconds": 180,
            "workflow_bound_seconds": 3600,
            "aggregate_retained_log_bytes": used,
            "aggregate_log_cap_bytes": CAP + 65536,
            "inventory_complete": {
                group: all(
                    row["status"] not in ["UNRUN", "INCOMPLETE", "TIMEOUT"]
                    for row in rows
                    if row["group"] == group
                )
                for group in GROUPS
            },
            "other103_roster_sha256": OTHER_ROSTER_SHA,
            "original_failure_log_sha256": FAILURE_LOG_SHA,
            "stack_frame_retained_log_bytes": sum(
                stream["bytes"] for stream in frame_streams.values()
            ),
            "stack_frame_accounting": "../stack-frame-accounting.json",
            "scope": "Original3 remain separate; unchangedbaseline14 and stack1 precede distinctactualother103. One authenticated numeric selector repeats original3 without identity/count merging. No baseline substitution, fullsharedprocessgate or performance acceptance. Unstarted cases stayUNRUN; samebudgets.",
        },
    )
    save(
        supplemental / "prune-controls-summary.json",
        {
            "source": source,
            "count": 7,
            "counts": counts["prune_controls7"],
            "complete": all(
                row["status"] not in ["UNRUN", "INCOMPLETE", "TIMEOUT"]
                for row in rows
                if row["group"] == "prune_controls7"
            ),
            "source_provenance": prepared["prune_control_source"],
            "case_budget_seconds": 180,
            "scope": "Separate authored controls after unchanged original3/baseline14/stack1/other103; no substitution or semantic gate acceptance.",
        },
    )
    save(
        supplemental / "targeted-prune-controls-summary.json",
        {
            "source": source,
            "count": 3,
            "counts": counts["targeted_prune_controls3"],
            "complete": all(
                row["status"] not in ["UNRUN", "INCOMPLETE", "TIMEOUT"]
                for row in rows
                if row["group"] == "targeted_prune_controls3"
            ),
            "source_provenance": prepared["targeted_prune_control_source"],
            "case_budget_seconds": 180,
            "scope": "Separate authored controls after unchanged original3/baseline14/stack1/other103/prune7; no substitution or semantic gate acceptance.",
        },
    )
    return (
        125
        if not complete
        else 100
        if any(row["status"] != "PASS" for row in jim_library_rows)
        or any(row["status"] != "PASS" for row in native_getter_rows)
        or any(row["status"] != "PASS" for row in allocation_rows)
        or any(row["status"] != "PASS" for row in headroom_rows)
        or any(row["status"] != "PASS" for row in ordinary_entry_rows)
        or any(row["status"] != "PASS" for row in constructor_rows)
        or any(row["status"] != "PASS" for row in aot_rows)
        or any(row["status"] != "PASS" for row in attestation_rows)
        or proc_row["status"] != "PASS" or any(row["status"] != "PASS" for row in rows)
        else 0
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["prepare", "frames", "run"])
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--sha", required=True)
    args = parser.parse_args()
    require(re.fullmatch("[0-9a-f]{40}", args.sha), "full exact source SHA required")
    require(
        os.environ.get("GITHUB_SHA") == args.sha
        and os.environ.get("GITHUB_REF") == "refs/heads/work",
        "actual WORK source required",
    )
    out = args.out.resolve()
    require(
        out.is_dir() and not out.is_relative_to(Path.cwd()),
        "existing external diagnostic evidence required",
    )
    try:
        if args.phase == "prepare":
            prepare(out, args.sha)
            return 0
        if args.phase == "frames":
            return frames(out, args.sha)
        return run(out, args.sha)
    except (
        RuntimeError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        ImportError,
        AttributeError,
    ) as error:
        save(
            out / ("supplement-" + args.phase + "-error.json"),
            {"error": repr(error), "complete": False},
        )
        if args.phase == "run":
            directory = out / "supplement"
            directory.mkdir(exist_ok=True)
            constructor_path = directory / "jim_library3-outcomes.json"
            constructor_rows = json.loads(constructor_path.read_text()) if constructor_path.exists() else [
                {"group": "jim_library3", "selector": name, "status": "UNRUN"}
                for name in JIM_LIBRARY3_SELECTORS]
            for row in constructor_rows:
                if row["status"] == "UNRUN":
                    row["unrun_reasons"] = [repr(error)]
            save(constructor_path, constructor_rows)
            save(directory / "jim_library3-summary.json", {
                "source": args.sha, "count": 3, "complete": False, "errors": [repr(error)],
                "counts": dict(collections.Counter(row["status"] for row in constructor_rows)),
                "scope": "JimLibrary3 unadmitted/incomplete; original161 remain separately reported.",
            })
            constructor_path = directory / "native_getter11-outcomes.json"
            constructor_rows = json.loads(constructor_path.read_text()) if constructor_path.exists() else [
                {"group": "native_getter11", "selector": name, "status": "UNRUN"}
                for name in NATIVE_GETTER11_SELECTORS]
            for row in constructor_rows:
                if row["status"] == "UNRUN":
                    row["unrun_reasons"] = [repr(error)]
            save(constructor_path, constructor_rows)
            save(directory / "native_getter11-summary.json", {
                "source": args.sha, "count": 11, "complete": False, "errors": [repr(error)],
                "counts": dict(collections.Counter(row["status"] for row in constructor_rows)),
                "scope": "NativeGetter11 unadmitted/incomplete; original150 remain separately reported.",
            })
            constructor_path = directory / "allocation2-outcomes.json"
            constructor_rows = json.loads(constructor_path.read_text()) if constructor_path.exists() else [
                {"group": "allocation2", "selector": name, "status": "UNRUN"}
                for name in ALLOCATION2_SELECTORS]
            for row in constructor_rows:
                if row["status"] == "UNRUN":
                    row["unrun_reasons"] = [repr(error)]
            save(constructor_path, constructor_rows)
            save(directory / "allocation2-summary.json", {
                "source": args.sha, "count": 2, "complete": False, "errors": [repr(error)],
                "counts": dict(collections.Counter(row["status"] for row in constructor_rows)),
                "scope": "Allocation2 unadmitted/incomplete; original148 remain separately reported.",
            })
            constructor_path = directory / "headroom2-outcomes.json"
            constructor_rows = json.loads(constructor_path.read_text()) if constructor_path.exists() else [
                {"group": "headroom2", "selector": name, "status": "UNRUN"}
                for name in HEADROOM2_SELECTORS]
            for row in constructor_rows:
                if row["status"] == "UNRUN":
                    row["unrun_reasons"] = [repr(error)]
            save(constructor_path, constructor_rows)
            save(directory / "headroom2-summary.json", {
                "source": args.sha, "count": 2, "complete": False, "errors": [repr(error)],
                "counts": dict(collections.Counter(row["status"] for row in constructor_rows)),
                "scope": "Headroom2 unadmitted/incomplete; original146 remain separately reported.",
            })
            constructor_path = directory / "ordinary_entry3-outcomes.json"
            constructor_rows = json.loads(constructor_path.read_text()) if constructor_path.exists() else [
                {"group": "ordinary_entry3", "selector": name, "status": "UNRUN"}
                for name in ORDINARY_ENTRY3_SELECTORS]
            for row in constructor_rows:
                if row["status"] == "UNRUN":
                    row["unrun_reasons"] = [repr(error)]
            save(constructor_path, constructor_rows)
            save(directory / "ordinary_entry3-summary.json", {
                "source": args.sha, "count": 3, "complete": False, "errors": [repr(error)],
                "counts": dict(collections.Counter(row["status"] for row in constructor_rows)),
                "scope": "OrdinaryEntry3 unadmitted/incomplete; original143 remain separately reported.",
            })
            constructor_path = directory / "constructor3-outcomes.json"
            constructor_rows = json.loads(constructor_path.read_text()) if constructor_path.exists() else [
                {"group": "constructor3", "selector": name, "status": "UNRUN"}
                for name in CONSTRUCTOR3_SELECTORS]
            for row in constructor_rows:
                if row["status"] == "UNRUN":
                    row["unrun_reasons"] = [repr(error)]
            save(constructor_path, constructor_rows)
            save(directory / "constructor3-summary.json", {
                "source": args.sha, "count": 3, "complete": False, "errors": [repr(error)],
                "counts": dict(collections.Counter(row["status"] for row in constructor_rows)),
                "scope": "Constructor3 unadmitted/incomplete; original140 remain separately reported.",
            })
            aot_path = directory / "aot5-outcomes.json"
            aot_rows = json.loads(aot_path.read_text()) if aot_path.exists() else [
                {"group": "aot5", "selector": name, "status": "UNRUN"} for name in AOT5_SELECTORS]
            for row in aot_rows:
                if row["status"] == "UNRUN":
                    row["unrun_reasons"] = [repr(error)]
            save(aot_path, aot_rows)
            save(directory / "aot5-summary.json", {
                "source": args.sha, "count": 5, "complete": False, "errors": [repr(error)],
                "counts": dict(collections.Counter(row["status"] for row in aot_rows)),
                "scope": "AOT5 unadmitted/incomplete; original135 remain separately reported.",
            })
            outcomes = directory / "outcomes.json"
            if outcomes.exists():
                rows = json.loads(outcomes.read_text())
            else:
                roster = json.loads(
                    (BASE / "runtime-baseline-supplement-roster.json").read_text()
                )
                roster["other103"] = json.loads(
                    (BASE / "runtime-other-failures-roster.json").read_text()
                )["other103"]
                roster["prune_controls7"] = list(PRUNE_CONTROLS)
                roster["targeted_prune_controls3"] = list(TARGETED_PRUNE_CONTROLS)
                rows = [
                    {"group": group, "selector": name, "status": "UNRUN"}
                    for group in GROUPS
                    for name in roster[group]
                ]
                save(outcomes, rows)
            save(
                directory / "prune-controls-outcomes.json",
                [row for row in rows if row["group"] == "prune_controls7"],
            )
            save(
                directory / "targeted-prune-controls-outcomes.json",
                [row for row in rows if row["group"] == "targeted_prune_controls3"],
            )
            save(
                directory / "prune-controls-summary.json",
                {
                    "source": args.sha,
                    "complete": False,
                    "errors": [repr(error)],
                    "counts": dict(
                        collections.Counter(
                            row["status"]
                            for row in rows
                            if row["group"] == "prune_controls7"
                        )
                    ),
                    "scope": "Authored prune controls unadmitted; original inventories remain separately reported.",
                },
            )
            save(
                directory / "targeted-prune-controls-summary.json",
                {
                    "source": args.sha,
                    "complete": False,
                    "errors": [repr(error)],
                    "counts": dict(
                        collections.Counter(
                            row["status"]
                            for row in rows
                            if row["group"] == "targeted_prune_controls3"
                        )
                    ),
                    "scope": "Targeted prune controls unadmitted; original inventories remain separately reported.",
                },
            )
            save(
                directory / "summary.json",
                {
                    "source": args.sha,
                    "complete": False,
                    "errors": [repr(error)],
                    "counts": {
                        group: dict(
                            collections.Counter(
                                row["status"] for row in rows if row["group"] == group
                            )
                        )
                        for group in GROUPS
                    },
                    "scope": "Original supplemental assertions not admitted; completed process observations remain in outcomes, all unstarted cases stay UNRUN.",
                },
            )
        return 125


if __name__ == "__main__":
    raise SystemExit(main())
