#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover exact scripts and observed results from the r2286l context matrix."""

import json
import re
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006l"

# Each description names the semantic distinction, not merely the measured output.
TOPICS = {
    "g_if_control": "a space-separated braced if condition and body parse as separate words",
    "g_if_chain": "adjacent braced if condition and body parse as separate words",
    "g_list_chain": "adjacent braced words become separate list arguments",
    "g_set_chain": "adjacent braced fragments become separate set arguments",
    "g_cmd_glued": "a command name can be immediately followed by braced words",
    "g_expansion": "the expansion prefix adjacent to a braced list expands its elements",
    "g_quote_chain": "a bare suffix after a quoted fragment forms a separate word",
    "g_dollarbrace": "a suffix following a braced variable substitution stays in the same word",
    "n_if_nextline": "a newline between an if condition and body separates commands",
    "n_while_nextline": "a newline between a while condition and body separates commands",
    "n_else_nextline": "else on a new line remains part of the preceding if",
    "n_list_absorb": "a braced word on a new line is absorbed by the preceding list command",
    "n_blank_breaks": "a blank line separates an if condition from a following braced body",
    "n_comment_breaks": "a comment line separates an if condition from a following braced body",
    "e_starts_with": "the starts_with expression operator accepts a prefix comparison",
    "e_contains": "the contains expression operator accepts a substring comparison",
    "e_matches": "the matches expression operator accepts equal string operands",
    "e_and_word": "the word and is accepted as a boolean expression operator",
    "e_adjacent_eq": "the eq expression operator is recognized without whitespace around it",
    "num_octal": "a leading-zero integer is interpreted as octal",
    "num_hex": "a hexadecimal integer literal is accepted and evaluated",
    "num_binary": "a binary-prefixed integer literal is accepted and evaluated",
    "num_exp": "an exponent-form numeric literal is accepted and evaluated",
    "s_dict": "the dict command is available and accepts this assignment",
    "s_lassign": "the lassign command is available and assigns list elements",
    "s_lmap": "the lmap command is available for list iteration",
    "s_try": "the try command is available for exception handling",
    "s_throw": "the throw command is available to raise a structured error",
    "s_apply": "the apply command is available to invoke a lambda",
    "s_package": "package names reports the available packages in the execution context",
    "s_interp": "interp aliases is available and reports aliases",
    "s_namespace": "namespace current reports the evaluation namespace",
    "s_exec": "exec can invoke the tested external executable in this context",
    "s_after": "after is available in this execution context",
    "c_if_comment_eof": "a hash beginning a braced if body comments through the end of that body",
    "c_if_comment_line": "a newline ends a comment inside a braced if body",
    "c_if_hash_arg": "a hash immediately following a braced word starts a comment or remains an argument",
    "c_semicolon_comment": "a hash after a semicolon starts a command comment until newline",
    "c_comment_brace_balanced": "balanced braces inside a comment affect subsequent command parsing",
    "w_brace_bare": "a bare suffix immediately after a braced word is accepted as a separate word",
    "w_brace_quote_bare": "adjacent braced, quoted and bare fragments form separate words",
    "w_repeat": "repeated adjacent braced fragments form separate words",
    "w_empty_repeat": "adjacent empty braced fragments form separate words",
    "w_var_gate": "a braced fragment immediately following an unbraced variable substitution is a new word",
    "w_braced_var_gate": "a braced fragment immediately following a braced variable substitution is a new word",
    "w_command_gate": "a braced fragment immediately following command substitution is a new word",
    "w_command_name": "an adjacent suffix can extend a braced command-name fragment",
    "n_if_crlf": "CRLF between an if condition and body separates commands",
    "n_if_lf": "LF between an if condition and body separates commands",
    "n_blank_bound": "a blank line breaks the association between an if condition and body",
    "n_comment_bound": "an intervening comment breaks the association between an if condition and body",
    "n_backslash_lf": "backslash-LF joins an if condition to a following body",
    "n_backslash_space_lf": "backslash-space-LF joins an if condition to a following body",
    "n_else_lf": "else on a following LF line belongs to the preceding if command",
    "n_elseif_lf": "elseif on a following LF line belongs to the preceding if command",
    "e_equals": "the equals expression operator accepts equal strings",
    "x_e_starts_with": "starts_with tests the indicated string prefix",
    "e_ends_with": "ends_with tests the indicated string suffix",
    "e_contains_true": "contains accepts the indicated present substring",
    "e_contains_false": "contains rejects the indicated absent substring",
    "e_matches_equal": "matches accepts identical strings",
    "e_matches_substring": "matches treats a proper substring as a match",
    "e_matches_glob": "matches_glob applies the tested glob pattern",
    "e_matches_regex": "matches_regex applies the tested regular expression",
    "e_or": "the word or is accepted as a boolean expression operator",
    "e_not": "the word not is accepted as a unary expression operator",
    "e_precedence_matches_or": "matches binds before or for the tested expression",
    "e_precedence_not_starts": "not binds before starts_with for the tested expression",
    "e_invalid_function": "an unknown expression function is rejected",
    "num_bad_octal": "a leading-zero integer containing 9 is rejected as invalid octal",
    "num_wide": "the tested wide integer is accepted without losing its value",
    "l_index_end_plus": "lindex accepts end+1 as an index",
    "l_index_integer_plus": "lindex accepts arithmetic notation in an integer index",
    "t_trace_old": "the legacy trace command installs and reports a variable trace",
    "t_trace_add": "trace add variable installs a read trace",
    "s_switch_glued": "a switch list with separated pattern and body is accepted",
    "s_switch_glued_pattern": "a switch list accepts a pattern immediately adjacent to its braced body",
    "p_patchlevel": "info patchlevel reports the version in this execution context",
    "p_tcl_patchlevel": "the tcl_patchLevel variable exists in this execution context",
    "p_tmsh_version": "tmsh::version is available through a computed command in this context",
    "p_platform_keys": "the tcl_platform array exposes platform keys in this context",
}
ALIASES = {
    "w_quote_bare": "g_quote_chain",
    "x_n_list_absorb": "n_list_absorb",
    "e_and": "e_and_word",
    "x_e_adjacent_eq": "e_adjacent_eq",
    "num_leading_zero": "num_octal",
    "x_num_hex": "num_hex",
    "x_num_binary": "num_binary",
    "p_packages": "s_package",
    "p_namespace_current": "s_namespace",
    "p_exec": "s_exec",
    "w_hash_after_brace": "c_if_hash_arg",
}


def main():
    manifest = json.loads((DATA / "manifest.json").read_bytes())
    cases = {row["id"]: row for row in manifest["cases"]}
    by_source = defaultdict(list)
    for case, row in cases.items():
        source = bytes.fromhex(row["source_hex"])
        by_source[source].append(case)
        if ALIASES.get(case, case) not in TOPICS:
            raise ValueError(f"missing semantic question for {case}")
        for filename in (
            "host.tcl",
            "irule.conf",
            "irule-config.txt",
            "cli.conf",
            "cli-config.txt",
            "iapp.conf",
            "icall.conf",
        ):
            if row["source_hex"].encode() not in (DATA / filename).read_bytes():
                raise ValueError(f"source missing from {filename}: {case}")
    results = defaultdict(list)
    for filename, context, version in (
        ("cli-run.txt", "tmsh CLI script", "BIG-IP 21.1.0.1, Tcl 8.4.6"),
        ("host-tclsh8.4.txt", "appliance tclsh8.4", "BIG-IP 21.1.0.1, Tcl 8.4.13"),
        ("host-tclsh8.5.txt", "appliance tclsh8.5", "BIG-IP 21.1.0.1, Tcl 8.5.13"),
        ("scriptd-results.txt", None, "BIG-IP 21.1.0.1, Tcl 8.4.6"),
    ):
        for line in (DATA / filename).read_bytes().splitlines():
            if not line.startswith(b"R2286DEEP|") or b"|case=" not in line:
                continue
            fields = line.decode("ascii").split("|")
            values = dict(item.split("=", 1) for item in fields[3:] if "=" in item)
            case = values["case"]
            if case not in cases or values["source_hex"] != cases[case]["source_hex"]:
                raise ValueError(f"mismatched source in {filename}: {case}")
            label = (
                context
                or {
                    "IAppImplementation": "iApp implementation",
                    "ICallScript": "iCall script",
                }[fields[2]]
            )
            results[case].append(
                (label, version, values["rc"], bytes.fromhex(values["value_hex"]))
            )
    for line in (DATA / "ltm-raw.log").read_bytes().splitlines():
        if b"R2286DEEP|r2286l|TmmIRule|" not in line:
            continue
        unit_match = re.search(rb"\btmm(\d*)\[", line)
        if not unit_match:
            raise ValueError("missing TMM unit")
        tag = "tmm" + unit_match[1].decode()
        entries = line.split(b"R2286DEEP|r2286l|TmmIRule|", 1)[1].split(b"|", 1)[1]
        for entry in entries.split():
            match = re.fullmatch(rb"([A-Za-z0-9_]+),(\d+),([0-9a-f]*)", entry)
            if not match:
                raise ValueError("incomplete TMM result")
            case, code, value = match.groups()
            results[case.decode()].append(
                (
                    f"TMM log tag {tag}, RULE_INIT",
                    "BIG-IP 21.1.0.1, Tcl 8.4.6",
                    code.decode(),
                    bytes.fromhex(value.decode()),
                )
            )
    for case, entries in results.items():
        if len(entries) != 9 or len({entry[0] for entry in entries}) != 9:
            raise ValueError(f"incomplete or duplicate contexts for {case}")
    if set(results) != set(cases):
        raise ValueError("missing case observations")
    for source, identities in by_source.items():
        canonical = ALIASES.get(identities[0], identities[0])
        question = f"When evaluating this source, is it true that {TOPICS[canonical]}? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested."
        parts = [
            f"# BIG-IP and appliance Tcl context: {canonical.replace('_', ' ')}\n\n## Question\n\n{question}\n\n",
            (
                "## Assumptions\n\nEach wrapper dynamically decoded and evaluated the exact source under catch. "
                "Cases were run sequentially within the wrapper, so state left by earlier cases may matter; "
                "the contexts and versions are separate observations. The result is the Tcl catch value, "
                "not the wrapper process's exit status. Other assumptions were not separately recorded.\n\n## Script\n\n"
            ),
            markdown_block(source),
            "\n## Results\n",
        ]
        for case in identities:
            for label, version, code, result in sorted(results[case]):
                parts.extend(
                    [
                        f"\n### {label}, case {case}\n\n- Recorded version: {version}.\n",
                        f"- Recorded input channel: {label}.\n- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n- Recorded result:\n\n",
                        markdown_block(result, "text-visible"),
                        f"\nThe wrapper recorded Tcl completion code {code}.\n",
                    ]
                )
        document = "".join(parts).rstrip() + "\n"
        if errors := validate_document(document):
            raise ValueError(f"{canonical}: {errors}")
        path = ROOT / "evidence/tcl" / f"bigip-r2286l-{canonical.replace('_', '-')}.md"
        if path.exists() and path.read_text() != document:
            raise ValueError(f"refusing to overwrite {path}")
        path.write_text(document)
    print(
        f"Recovered {len(by_source)} distinct scripts from {len(cases)} context-matrix cases"
    )


if __name__ == "__main__":
    main()
