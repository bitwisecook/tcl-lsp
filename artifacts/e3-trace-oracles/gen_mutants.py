#!/usr/bin/env python3
"""Generate the mutant corpus: one (from, to) pair per guard introduced by
lane E3, plus the manifest the runner walks."""
import os

D = os.path.dirname(os.path.abspath(__file__))
M = os.path.join(D, "mutants")
os.makedirs(M, exist_ok=True)

CC = "rust/tcl-cmd-core/src/trace.rs"
VMI = "rust/tcl-vm/src/interp.rs"
VMC = "rust/tcl-vm/src/cmd_trace.rs"
RTI = "runtime/rust/src/interp.rs"
RTC = "runtime/rust/src/cmd_trace.rs"

BOTH = "-p tcl-cmd-core -p tcl-vm"
VM = "-p tcl-vm"
RT = "runtime"

MUTANTS = [
    # --- shared owner: canonical op order + legacy rendering ---
    ("M02-info-order-command", CC, BOTH,
     '            TraceKind::Command => &["rename", "delete"],\n',
     '            TraceKind::Command => &["delete", "rename"],\n'),

    ("M03-legacy-not-canonicalised", CC, BOTH,
     "    Ok(canonical_set(&words, TraceKind::Variable))\n",
     "    Ok(words)\n"),

    ("M04-letters-order", CC, BOTH,
     "const LEGACY_LETTERS: [(u8, &str); 4] = [\n"
     '    (b\'r\', "read"),\n'
     '    (b\'w\', "write"),\n'
     '    (b\'u\', "unset"),\n'
     '    (b\'a\', "array"),\n'
     "];\n",
     "const LEGACY_LETTERS: [(u8, &str); 4] = [\n"
     '    (b\'a\', "array"),\n'
     '    (b\'u\', "unset"),\n'
     '    (b\'w\', "write"),\n'
     '    (b\'r\', "read"),\n'
     "];\n"),

    ("M05-callback-op-word-ignores-old-style", CC, BOTH,
     "    if !old_style {\n        return op;\n    }\n",
     "    if true {\n        return op;\n    }\n"),

    ("M12-option-word-exact-only", CC, BOTH,
     '    let table = OptionTable::abbreviating("option", visible);\n',
     '    let table = OptionTable::exact_only("option", visible);\n'),

    # --- #1438: the write is committed before the traces run ---
    ("M06-set-var-rolls-back", VMI, VM,
     "        self.write_scalar_raw(name, value);\n"
     "        if self.var_traces.is_empty() {\n"
     "            return Ok(());\n"
     "        }\n"
     '        self.fire_var_traces(name, "write")\n',
     "        let old = self.get_var(name);\n"
     "        self.write_scalar_raw(name, value);\n"
     "        if self.var_traces.is_empty() {\n"
     "            return Ok(());\n"
     "        }\n"
     '        if let Err(e) = self.fire_var_traces(name, "write") {\n'
     "            if let Some(o) = old {\n"
     "                self.write_scalar_raw(name, o);\n"
     "            } else {\n"
     "                let (lvl, nm) = self.locate(name);\n"
     "                if let Some(f) = self.frames.get_mut(lvl) {\n"
     "                    f.locals.remove(&nm);\n"
     "                }\n"
     "            }\n"
     "            return Err(e);\n"
     "        }\n"
     "        Ok(())\n"),

    ("M07-set-array-elem-rolls-back", VMI, VM,
     "        self.write_array_raw(name, key, value)?;\n"
     "        if self.var_traces.is_empty() {\n"
     "            return Ok(());\n"
     "        }\n"
     '        self.fire_var_traces(&format!("{name}({key})"), "write")\n',
     "        let old = self.get_array_elem(name, key);\n"
     "        self.write_array_raw(name, key, value)?;\n"
     "        if self.var_traces.is_empty() {\n"
     "            return Ok(());\n"
     "        }\n"
     '        if let Err(e) = self.fire_var_traces(&format!("{name}({key})"), "write") {\n'
     "            match old {\n"
     "                Some(o) => {\n"
     "                    let _ = self.write_array_raw(name, key, o);\n"
     "                }\n"
     "                None => self.array_unset_elem(name, key),\n"
     "            }\n"
     "            return Err(e);\n"
     "        }\n"
     "        Ok(())\n"),

    # --- #1440: firing order ---
    ("M08-element-before-array", VMI, VM,
     "        keys.push(self.trace_key(name));\n",
     "        keys.insert(0, self.trace_key(name));\n"),

    ("M09-var-traces-oldest-first", VMI, VM,
     "            for tr in traces.iter().rev() {\n",
     "            for tr in traces.iter() {\n"),

    ("M10-cmd-traces-oldest-first", VMI, VM,
     "        for entry in entries.into_iter().rev() {\n",
     "        for entry in entries.into_iter() {\n"),

    ("M11-vm-remove-var-oldest", VMI, VM,
     "                .rposition(|t| t.ops == ops && t.command == command)\n",
     "                .position(|t| t.ops == ops && t.command == command)\n"),

    ("M13-vm-remove-cmd-oldest", VMI, VM,
     "                .rposition(|t| t.ops == ops && t.callback == callback)\n",
     "                .position(|t| t.ops == ops && t.callback == callback)\n"),

    # --- #1444: the registry owns the release boundary ---
    ("M14-vm-option-gate-open", VMC, VM,
     "        .filter(|sub| {\n"
     "            sub.dialects\n"
     "                .or(spec.dialects)\n"
     "                .is_none_or(|gate| gate.intersects(profile.availability_mask))\n"
     "        })\n",
     "        .filter(|_sub| true)\n"),

    # --- runtime engine ---
    ("M15-rt-var-traces-oldest-first", RTI, RT,
     "                .traces\n                .iter()\n                .rev()\n",
     "                .traces\n                .iter()\n"),

    ("M16-rt-element-before-array", RTI, RT,
     "        let cmds: Vec<(crate::cmd_trace::VarTraceScope, Vec<u8>, bool)> = selected(true)\n"
     "            .chain(selected(false))\n",
     "        let cmds: Vec<(crate::cmd_trace::VarTraceScope, Vec<u8>, bool)> = selected(false)\n"
     "            .chain(selected(true))\n"),

    ("M17-rt-callback-op-word-ignores-old-style", RTI, RT,
     "            let op_word = tcl_cmd_core::trace::callback_op_word(&op_name, old_style);\n",
     "            let op_word = tcl_cmd_core::trace::callback_op_word(&op_name, false);\n"),

    ("M18-rt-cmd-traces-oldest-first", RTI, RT,
     "            .cmd_traces\n"
     "            .iter()\n"
     "            .rev()\n"
     "            .filter(|t| t.name == old_fqn && (t.ops & op_bit) != 0)\n",
     "            .cmd_traces\n"
     "            .iter()\n"
     "            .filter(|t| t.name == old_fqn && (t.ops & op_bit) != 0)\n"),

    ("M21-rt-ns-unset-not-reversed", RTI, RT,
     "        // C fires each variable's unset traces newest-first, like every other\n"
     "        // trace list; `retain` visits our Vec oldest-first. The order *across*\n"
     "        // the namespace's variables is its hash-table walk in C, so it is not\n"
     "        // pinned either way — only the within-variable order is. Issue #1440.\n"
     "        victims.reverse();\n",
     ""),

    ("M22-rt-ns-cmd-not-reversed", RTI, RT,
     "        // Newest-first per command, as `take_ns_unset_traces` explains; the\n"
     "        // order across the namespace's commands is C's hash walk. Issue #1440.\n"
     "        victims.reverse();\n",
     ""),

    ("M19-rt-remove-var-oldest", RTC, RT,
     "            .rposition(|t| t.name == name && t.ops == ops && t.command == command);\n",
     "            .position(|t| t.name == name && t.ops == ops && t.command == command);\n"),

    ("M20-rt-remove-cmd-oldest", RTC, RT,
     "            .rposition(|t| t.name == fqn && t.ops == flags && t.command == command);\n",
     "            .position(|t| t.name == fqn && t.ops == flags && t.command == command);\n"),

    ("M23-rt-option-gate-open", RTC, RT,
     "        .filter(|sub| {\n"
     "            sub.dialects\n"
     "                .or(spec.dialects)\n"
     "                .is_none_or(|gate| gate.intersects(profile.availability_mask))\n"
     "        })\n",
     "        .filter(|_sub| true)\n"),
]

with open(os.path.join(M, "manifest.tsv"), "w") as mf:
    for mid, path, scope, frm, to in MUTANTS:
        with open(os.path.join(M, mid + ".from"), "w") as f:
            f.write(frm)
        with open(os.path.join(M, mid + ".to"), "w") as f:
            f.write(to)
        mf.write(f"{mid}\t{path}\t{scope}\n")
print(f"{len(MUTANTS)} mutants written to {M}")
