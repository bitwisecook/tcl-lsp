// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `try` compiles as C's `TclCompileTryCmd` does (#2544). Tcl 9.1 dispatches
//! trapless handlers through one `jumpTableNum`, ends a handler's error path
//! with `dictPut; swap`, and emits no `returnCodeBranch`.
//!
//! Each row is `tcl::unsupported::disassemble proc t` as `pc:mnemonic` on
//! tclsh 9.0.4 (8.6.18 prints the same) and 9.1.0. The 9.1 rows are matched
//! exactly. The 9.0 rows are matched under 8.6 and 9.0 opcode by opcode
//! without width suffixes: our 8.6 and 9.0 `try` keep shorter jumps than C's,
//! which is the corpus work of #2545.

use super::*;

const CASES: &[(&str, &str, &str, &str)] = &[
    (
        "one",
        "try {foo $x} on error {m} {return 1}",
        "0:beginCatch4 5:push1 7:loadScalar1 9:invokeStk1 11:endCatch 12:jump4 17:pushReturnCode 18:pushResult 19:pushReturnOpts 20:endCatch 21:storeScalar1 23:pop 24:storeScalar1 26:pop 27:dup 28:push1 30:eq 31:jumpFalse4 36:pop 37:loadScalar1 39:storeScalar1 41:pop 42:beginCatch4 47:startCommand 56:push1 58:push1 60:returnImm 69:endCatch 70:jump4 75:pushResult 76:pushReturnOpts 77:pushReturnCode 78:endCatch 79:push1 81:eq 82:jumpFalse1 84:loadScalar1 86:reverse 91:storeScalar1 93:pop 94:push1 96:reverse 101:dictSet 110:reverse 115:returnStk 116:jump4 121:pop 122:loadScalar1 124:loadScalar1 126:returnStk 127:done",
        "0:beginCatch 5:push 10:loadScalar 15:invokeStk 20:endCatch 21:jump 26:pushReturnCode 27:pushResult 28:pushReturnOpts 29:endCatch 30:storeScalar 35:pop 36:storeScalar 41:pop 42:jumpTableNum 47:jump 52:loadScalar 57:storeScalar 62:pop 63:beginCatch 68:startCommand 77:push 82:push 87:returnImm 96:endCatch 97:jump 102:pushResult 103:pushReturnOpts 104:pushReturnCode 105:endCatch 106:push 111:eq 112:jumpFalse 117:push 122:loadScalar 127:dictPut 128:swap 129:returnStk 130:jump 135:loadScalar 140:loadScalar 145:returnStk 146:done",
    ),
    (
        "several",
        "try {foo $x} on error {m} {return 1} on break {} {return 2} on continue {} {return 3}",
        "0:beginCatch4 5:push1 7:loadScalar1 9:invokeStk1 11:endCatch 12:jump4 17:pushReturnCode 18:pushResult 19:pushReturnOpts 20:endCatch 21:storeScalar1 23:pop 24:storeScalar1 26:pop 27:dup 28:push1 30:eq 31:jumpFalse4 36:pop 37:loadScalar1 39:storeScalar1 41:pop 42:beginCatch4 47:startCommand 56:push1 58:push1 60:returnImm 69:endCatch 70:jump4 75:pushResult 76:pushReturnOpts 77:pushReturnCode 78:endCatch 79:push1 81:eq 82:jumpFalse1 84:loadScalar1 86:reverse 91:storeScalar1 93:pop 94:push1 96:reverse 101:dictSet 110:reverse 115:returnStk 116:jump4 121:dup 122:push1 124:eq 125:jumpFalse4 130:pop 131:beginCatch4 136:startCommand 145:push1 147:push1 149:returnImm 158:endCatch 159:jump4 164:pushResult 165:pushReturnOpts 166:pushReturnCode 167:endCatch 168:push1 170:eq 171:jumpFalse1 173:loadScalar1 175:reverse 180:storeScalar1 182:pop 183:push1 185:reverse 190:dictSet 199:reverse 204:returnStk 205:jump4 210:dup 211:push1 213:eq 214:jumpFalse4 219:pop 220:beginCatch4 225:startCommand 234:push1 236:push1 238:returnImm 247:endCatch 248:jump4 253:pushResult 254:pushReturnOpts 255:pushReturnCode 256:endCatch 257:push1 259:eq 260:jumpFalse1 262:loadScalar1 264:reverse 269:storeScalar1 271:pop 272:push1 274:reverse 279:dictSet 288:reverse 293:returnStk 294:jump4 299:pop 300:loadScalar1 302:loadScalar1 304:returnStk 305:done",
        "0:beginCatch 5:push 10:loadScalar 15:invokeStk 20:endCatch 21:jump 26:pushReturnCode 27:pushResult 28:pushReturnOpts 29:endCatch 30:storeScalar 35:pop 36:storeScalar 41:pop 42:jumpTableNum 47:jump 52:loadScalar 57:storeScalar 62:pop 63:beginCatch 68:startCommand 77:push 82:push 87:returnImm 96:endCatch 97:jump 102:pushResult 103:pushReturnOpts 104:pushReturnCode 105:endCatch 106:push 111:eq 112:jumpFalse 117:push 122:loadScalar 127:dictPut 128:swap 129:returnStk 130:jump 135:beginCatch 140:startCommand 149:push 154:push 159:returnImm 168:endCatch 169:jump 174:pushResult 175:pushReturnOpts 176:pushReturnCode 177:endCatch 178:push 183:eq 184:jumpFalse 189:push 194:loadScalar 199:dictPut 200:swap 201:returnStk 202:jump 207:beginCatch 212:startCommand 221:push 226:push 231:returnImm 240:endCatch 241:jump 246:pushResult 247:pushReturnOpts 248:pushReturnCode 249:endCatch 250:push 255:eq 256:jumpFalse 261:push 266:loadScalar 271:dictPut 272:swap 273:returnStk 274:jump 279:loadScalar 284:loadScalar 289:returnStk 290:done",
    ),
    (
        "fallthrough",
        "try {foo $x} on error - on break {} {return 2}",
        "0:push1 2:push1 4:push1 6:push1 8:push1 10:push1 12:push1 14:push1 16:push1 18:invokeStk1 20:done",
        "0:push 5:push 10:push 15:push 20:push 25:push 30:push 35:push 40:push 45:invokeStk 50:done",
    ),
    (
        "on_ok",
        "try {foo $x} on ok {r} {return $r} on error {m} {return 1}",
        "0:beginCatch4 5:push1 7:loadScalar1 9:invokeStk1 11:push1 13:reverse 18:jump1 20:pushReturnCode 21:pushResult 22:pushReturnOpts 23:endCatch 24:storeScalar1 26:pop 27:storeScalar1 29:pop 30:dup 31:push1 33:eq 34:jumpFalse4 39:pop 40:loadScalar1 42:storeScalar1 44:pop 45:beginCatch4 50:startCommand 59:loadScalar1 61:push1 63:returnImm 72:endCatch 73:jump4 78:pushResult 79:pushReturnOpts 80:pushReturnCode 81:endCatch 82:push1 84:eq 85:jumpFalse1 87:loadScalar1 89:reverse 94:storeScalar1 96:pop 97:push1 99:reverse 104:dictSet 113:reverse 118:returnStk 119:jump4 124:dup 125:push1 127:eq 128:jumpFalse4 133:pop 134:loadScalar1 136:storeScalar1 138:pop 139:beginCatch4 144:startCommand 153:push1 155:push1 157:returnImm 166:endCatch 167:jump4 172:pushResult 173:pushReturnOpts 174:pushReturnCode 175:endCatch 176:push1 178:eq 179:jumpFalse1 181:loadScalar1 183:reverse 188:storeScalar1 190:pop 191:push1 193:reverse 198:dictSet 207:reverse 212:returnStk 213:jump4 218:pop 219:loadScalar1 221:loadScalar1 223:returnStk 224:done",
        "0:beginCatch 5:push 10:loadScalar 15:invokeStk 20:push 25:swap 26:jump 31:pushReturnCode 32:pushResult 33:pushReturnOpts 34:endCatch 35:storeScalar 40:pop 41:storeScalar 46:pop 47:jumpTableNum 52:jump 57:loadScalar 62:storeScalar 67:pop 68:beginCatch 73:startCommand 82:loadScalar 87:push 92:returnImm 101:endCatch 102:jump 107:pushResult 108:pushReturnOpts 109:pushReturnCode 110:endCatch 111:push 116:eq 117:jumpFalse 122:push 127:loadScalar 132:dictPut 133:swap 134:returnStk 135:jump 140:loadScalar 145:storeScalar 150:pop 151:beginCatch 156:startCommand 165:push 170:push 175:returnImm 184:endCatch 185:jump 190:pushResult 191:pushReturnOpts 192:pushReturnCode 193:endCatch 194:push 199:eq 200:jumpFalse 205:push 210:loadScalar 215:dictPut 216:swap 217:returnStk 218:jump 223:loadScalar 228:loadScalar 233:returnStk 234:done",
    ),
    (
        "finally",
        "try {foo $x} on error {m} {return 1} finally {bar}",
        "0:beginCatch4 5:push1 7:loadScalar1 9:invokeStk1 11:endCatch 12:storeScalar1 14:pop 15:push1 17:storeScalar1 19:pop 20:jump4 25:pushReturnCode 26:pushResult 27:pushReturnOpts 28:endCatch 29:storeScalar1 31:pop 32:storeScalar1 34:pop 35:dup 36:push1 38:eq 39:jumpFalse4 44:pop 45:beginCatch4 50:loadScalar1 52:storeScalar1 54:pop 55:startCommand 64:push1 66:push1 68:returnImm 77:push1 79:pushReturnOpts 80:reverse 85:jump1 87:pushReturnOpts 88:pushReturnCode 89:pushResult 90:endCatch 91:storeScalar1 93:pop 94:push1 96:eq 97:jumpFalse1 99:loadScalar1 101:push1 103:reverse 108:storeScalar1 110:pop 111:dictSet 120:jump1 122:storeScalar1 124:pop 125:beginCatch4 130:push1 132:invokeStk1 134:endCatch 135:pop 136:jump1 138:pushResult 139:pushReturnOpts 140:pushReturnCode 141:endCatch 142:push1 144:eq 145:jumpFalse1 147:loadScalar1 149:push1 151:reverse 156:storeScalar1 158:pop 159:dictSet 168:pop 169:jump1 171:storeScalar1 173:pop 174:storeScalar1 176:pop 177:loadScalar1 179:loadScalar1 181:returnStk 182:done",
        "0:beginCatch 5:push 10:loadScalar 15:invokeStk 20:endCatch 21:storeScalar 26:pop 27:push 32:storeScalar 37:pop 38:jump 43:pushReturnCode 44:pushResult 45:pushReturnOpts 46:endCatch 47:storeScalar 52:pop 53:storeScalar 58:pop 59:jumpTableNum 64:jump 69:beginCatch 74:loadScalar 79:storeScalar 84:pop 85:startCommand 94:push 99:push 104:returnImm 113:pushReturnOpts 114:endCatch 115:jump 120:pushResult 121:pushReturnOpts 122:pushReturnCode 123:endCatch 124:push 129:eq 130:jumpFalse 135:push 140:loadScalar 145:dictPut 146:storeScalar 151:pop 152:storeScalar 157:pop 158:beginCatch 163:push 168:invokeStk 173:endCatch 174:pop 175:jump 180:pushResult 181:pushReturnOpts 182:pushReturnCode 183:endCatch 184:push 189:eq 190:jumpFalse 195:push 200:loadScalar 205:dictPut 206:storeScalar 211:pop 212:storeScalar 217:pop 218:loadScalar 223:loadScalar 228:returnStk 229:done",
    ),
    (
        "trap",
        "try {foo $x} trap {POSIX ENOENT} {m} {return 1} on error {m} {return 2}",
        "0:beginCatch4 5:push1 7:loadScalar1 9:invokeStk1 11:endCatch 12:jump4 17:pushReturnCode 18:pushResult 19:pushReturnOpts 20:endCatch 21:storeScalar1 23:pop 24:storeScalar1 26:pop 27:dup 28:push1 30:eq 31:jumpFalse4 36:loadScalar1 38:push1 40:dictGet 45:listRangeImm 54:push1 56:streq 57:jumpFalse4 62:pop 63:loadScalar1 65:storeScalar1 67:pop 68:beginCatch4 73:startCommand 82:push1 84:push1 86:returnImm 95:endCatch 96:jump4 101:pushResult 102:pushReturnOpts 103:pushReturnCode 104:endCatch 105:push1 107:eq 108:jumpFalse1 110:loadScalar1 112:reverse 117:storeScalar1 119:pop 120:push1 122:reverse 127:dictSet 136:reverse 141:returnStk 142:jump4 147:dup 148:push1 150:eq 151:jumpFalse4 156:pop 157:loadScalar1 159:storeScalar1 161:pop 162:beginCatch4 167:startCommand 176:push1 178:push1 180:returnImm 189:endCatch 190:jump4 195:pushResult 196:pushReturnOpts 197:pushReturnCode 198:endCatch 199:push1 201:eq 202:jumpFalse1 204:loadScalar1 206:reverse 211:storeScalar1 213:pop 214:push1 216:reverse 221:dictSet 230:reverse 235:returnStk 236:jump4 241:pop 242:loadScalar1 244:loadScalar1 246:returnStk 247:done",
        "0:beginCatch 5:push 10:loadScalar 15:invokeStk 20:endCatch 21:jump 26:pushReturnCode 27:pushResult 28:pushReturnOpts 29:endCatch 30:storeScalar 35:pop 36:storeScalar 41:pop 42:dup 43:push 48:eq 49:jumpFalse 54:loadScalar 59:push 64:dictGet 69:push 74:errorPrefixEq 79:jumpFalse 84:pop 85:loadScalar 90:storeScalar 95:pop 96:beginCatch 101:startCommand 110:push 115:push 120:returnImm 129:endCatch 130:jump 135:pushResult 136:pushReturnOpts 137:pushReturnCode 138:endCatch 139:push 144:eq 145:jumpFalse 150:push 155:loadScalar 160:dictPut 161:swap 162:returnStk 163:jump 168:dup 169:push 174:eq 175:jumpFalse 180:pop 181:loadScalar 186:storeScalar 191:pop 192:beginCatch 197:startCommand 206:push 211:push 216:returnImm 225:endCatch 226:jump 231:pushResult 232:pushReturnOpts 233:pushReturnCode 234:endCatch 235:push 240:eq 241:jumpFalse 246:push 251:loadScalar 256:dictPut 257:swap 258:returnStk 259:jump 264:pop 265:loadScalar 270:loadScalar 275:returnStk 276:done",
    ),
    (
        "trap_finally",
        "try {foo $x} trap {A} {m} {return 1} finally {bar}",
        "0:beginCatch4 5:push1 7:loadScalar1 9:invokeStk1 11:endCatch 12:storeScalar1 14:pop 15:push1 17:storeScalar1 19:pop 20:jump4 25:pushReturnCode 26:pushResult 27:pushReturnOpts 28:endCatch 29:storeScalar1 31:pop 32:storeScalar1 34:pop 35:dup 36:push1 38:eq 39:jumpFalse4 44:loadScalar1 46:push1 48:dictGet 53:listRangeImm 62:push1 64:streq 65:jumpFalse4 70:pop 71:beginCatch4 76:loadScalar1 78:storeScalar1 80:pop 81:startCommand 90:push1 92:push1 94:returnImm 103:push1 105:pushReturnOpts 106:reverse 111:jump1 113:pushReturnOpts 114:pushReturnCode 115:pushResult 116:endCatch 117:storeScalar1 119:pop 120:push1 122:eq 123:jumpFalse1 125:loadScalar1 127:push1 129:reverse 134:storeScalar1 136:pop 137:dictSet 146:jump1 148:storeScalar1 150:pop 151:beginCatch4 156:push1 158:invokeStk1 160:endCatch 161:pop 162:jump1 164:pushResult 165:pushReturnOpts 166:pushReturnCode 167:endCatch 168:push1 170:eq 171:jumpFalse1 173:loadScalar1 175:push1 177:reverse 182:storeScalar1 184:pop 185:dictSet 194:pop 195:jump1 197:storeScalar1 199:pop 200:storeScalar1 202:pop 203:loadScalar1 205:loadScalar1 207:returnStk 208:done",
        "0:beginCatch 5:push 10:loadScalar 15:invokeStk 20:endCatch 21:storeScalar 26:pop 27:push 32:storeScalar 37:pop 38:jump 43:pushReturnCode 44:pushResult 45:pushReturnOpts 46:endCatch 47:storeScalar 52:pop 53:storeScalar 58:pop 59:dup 60:push 65:eq 66:jumpFalse 71:loadScalar 76:push 81:dictGet 86:push 91:errorPrefixEq 96:jumpFalse 101:pop 102:beginCatch 107:loadScalar 112:storeScalar 117:pop 118:startCommand 127:push 132:push 137:returnImm 146:push 151:pushReturnOpts 152:reverse 157:jump 162:pushReturnOpts 163:pushReturnCode 164:pushResult 165:endCatch 166:storeScalar 171:pop 172:push 177:eq 178:jumpFalse 183:push 188:loadScalar 193:dictPut 194:storeScalar 199:pop 200:beginCatch 205:push 210:invokeStk 215:endCatch 216:pop 217:jump 222:pushResult 223:pushReturnOpts 224:pushReturnCode 225:endCatch 226:push 231:eq 232:jumpFalse 237:push 242:loadScalar 247:dictPut 248:storeScalar 253:pop 254:storeScalar 259:pop 260:loadScalar 265:loadScalar 270:returnStk 271:done",
    ),
    (
        "finally_only",
        "try {foo $x} finally {bar}",
        "0:beginCatch4 5:push1 7:loadScalar1 9:invokeStk1 11:jump1 13:pushResult 14:pushReturnOpts 15:endCatch 16:beginCatch4 21:push1 23:invokeStk1 25:endCatch 26:pop 27:jump1 29:pushResult 30:pushReturnOpts 31:pushReturnCode 32:endCatch 33:push1 35:eq 36:jumpFalse1 38:push1 40:over 45:list 50:listConcat 51:reverse 56:pop 57:pop 58:jump1 60:reverse 65:returnStk 66:done",
        "0:beginCatch 5:push 10:loadScalar 15:invokeStk 20:jump 25:pushResult 26:pushReturnOpts 27:endCatch 28:beginCatch 33:push 38:invokeStk 43:endCatch 44:pop 45:swap 46:jump 51:pushResult 52:pushReturnOpts 53:pushReturnCode 54:endCatch 55:push 60:eq 61:jumpFalse 66:push 71:over 76:dictPut 77:reverse 82:pop 83:pop 84:returnStk 85:done",
    ),
];

fn disassembly(profile: &str, body: &str) -> Vec<(i32, &'static str)> {
    let mut vm = crate::native_fixture::interpreter(
        tcl_dialect::DialectProfile::find(profile).expect("release profile"),
    );
    let words = ["t", "x", body].map(|word| Value::new_native_string_bytes(word.as_bytes()));
    let head = Value::new_native_string_bytes(b"proc".as_slice());
    assert_eq!(
        vm.invoke_host_original_object_vector(&head, &words).code,
        Code::Ok
    );
    let Some(Command::Proc(command)) = vm.lookup_command("t") else {
        panic!("{profile}: procedure");
    };
    let prepared = vm
        .ensure_proc_traced(command)
        .unwrap_or_else(|error| panic!("{profile}: {error:?}"));
    let wide = vm.runtime_version().emits_wide_instructions_only();
    prepared
        .body
        .asm
        .instructions
        .iter()
        .map(|instruction| {
            let name = if wide {
                instruction.op.wide_mnemonic()
            } else {
                instruction.op.mnemonic()
            };
            (instruction.offset, name)
        })
        .collect()
}

fn without_width(name: &str) -> &str {
    name.strip_suffix('1')
        .or_else(|| name.strip_suffix('4'))
        .unwrap_or(name)
}

#[test]
fn try_compiles_as_tclsh_9_0_and_9_1() {
    let mut mismatches = Vec::new();
    for (name, body, tcl90, tcl91) in CASES {
        let ours = disassembly("tcl9.1", body)
            .into_iter()
            .map(|(pc, op)| format!("{pc}:{op}"))
            .collect::<Vec<_>>()
            .join(" ");
        if ours != *tcl91 {
            mismatches.push(format!("tcl9.1 {name}:\n  {ours}\n  {tcl91}"));
        }
        let expected: Vec<_> = tcl90
            .split(' ')
            .map(|row| without_width(row.split_once(':').expect("pc:op").1).to_owned())
            .collect();
        for profile in ["tcl8.6", "tcl9.0"] {
            let ours: Vec<_> = disassembly(profile, body)
                .into_iter()
                .map(|(_, op)| without_width(op).to_owned())
                .collect();
            if ours != expected {
                mismatches.push(format!("{profile} {name}:\n  {ours:?}\n  {expected:?}"));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// tclsh 8.6.18, 9.0.4 and 9.1.0 give [`ANSWERS`] for this script: each
/// procedure's completion code, result, `-code` and `finally` log.
const RUN: &str = r#"proc thrower {x} {
    switch -- $x {
        e {error boom {} {POSIX ENOENT x}}
        b {return -code break}
        c {return -code continue}
        r {return -code return R}
        o {return ok}
        n {error other {} {OTHER}}
    }
}
proc one {x} {try {thrower $x} on error {m} {return E:$m}}
proc several {x} {try {thrower $x} on error {m} {return E:$m} on break {} {return B} on continue {} {return C}}
proc fall {x} {try {thrower $x} on error - on break {} {return EB}}
proc onok {x} {try {thrower $x} on ok {r} {return OK:$r} on error {m} {return E:$m}}
proc fin {x} {try {thrower $x} on error {m} {lappend ::log h; return E:$m} finally {lappend ::log f}}
proc finraise {x} {try {thrower $x} on error {m} {return E:$m} finally {error FIN}}
proc trapf {x} {try {thrower $x} trap {POSIX ENOENT} {m} {return T:$m} on error {m} {return E:$m} finally {set ::z 1}}
proc handlerraise {x} {try {thrower $x} on error {m} {error H2} finally {set ::z 2}}
set out {}
foreach p {one several fall onok fin finraise trapf handlerraise} {
    foreach x {e b c r o n} {
        set ::log {}
        set code [catch {$p $x} res opts]
        lappend out "$p/$x=$code:$res:[dict get $opts -code]:$::log"
    }
}
join $out \n"#;

const ANSWERS: &str = r#"one/e=0:E:boom:0:
one/b=1:invoked "break" outside of a loop:1:
one/c=1:invoked "continue" outside of a loop:1:
one/r=0:R:0:
one/o=0:ok:0:
one/n=0:E:other:0:
several/e=0:E:boom:0:
several/b=0:B:0:
several/c=0:C:0:
several/r=0:R:0:
several/o=0:ok:0:
several/n=0:E:other:0:
fall/e=1:bad handler type "break": must be finally, on, or trap:1:
fall/b=1:bad handler type "break": must be finally, on, or trap:1:
fall/c=1:bad handler type "break": must be finally, on, or trap:1:
fall/r=1:bad handler type "break": must be finally, on, or trap:1:
fall/o=1:bad handler type "break": must be finally, on, or trap:1:
fall/n=1:bad handler type "break": must be finally, on, or trap:1:
onok/e=0:E:boom:0:
onok/b=1:invoked "break" outside of a loop:1:
onok/c=1:invoked "continue" outside of a loop:1:
onok/r=0:R:0:
onok/o=0:OK:ok:0:
onok/n=0:E:other:0:
fin/e=0:E:boom:0:h f
fin/b=1:invoked "break" outside of a loop:1:f
fin/c=1:invoked "continue" outside of a loop:1:f
fin/r=0:R:0:f
fin/o=0:ok:0:f
fin/n=0:E:other:0:h f
finraise/e=1:FIN:1:
finraise/b=1:FIN:1:
finraise/c=1:FIN:1:
finraise/r=1:FIN:1:
finraise/o=1:FIN:1:
finraise/n=1:FIN:1:
trapf/e=0:T:boom:0:
trapf/b=1:invoked "break" outside of a loop:1:
trapf/c=1:invoked "continue" outside of a loop:1:
trapf/r=0:R:0:
trapf/o=0:ok:0:
trapf/n=0:E:other:0:
handlerraise/e=1:H2:1:
handlerraise/b=1:invoked "break" outside of a loop:1:
handlerraise/c=1:invoked "continue" outside of a loop:1:
handlerraise/r=0:R:0:
handlerraise/o=0:ok:0:
handlerraise/n=1:H2:1:"#;

#[test]
fn try_dispatch_answers_as_tclsh_does() {
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::interpreter(
            tcl_dialect::DialectProfile::find(profile).expect("release profile"),
        );
        let completion = vm
            .try_eval_source(RUN)
            .unwrap_or_else(|error| panic!("{profile}: {error:?}"));
        assert_eq!(
            String::from_utf8_lossy(completion.result.string_bytes().as_ref()),
            ANSWERS,
            "{profile}"
        );
    }
}

/// A compiled `subst` of a command substitution, as tclsh 9.1.0 disassembles
/// it: the return code goes through `jumpTableNum`, not `returnCodeBranch`,
/// behind one jump to the shared break target.
const SUBST_CASES: &[(&str, &str)] = &[
    (
        "subst {a[foo $x]b}",
        "0:push 5:jump 10:jump 15:beginCatch 20:push 25:loadScalar 30:invokeStk 35:endCatch 36:jump 41:pushReturnCode 42:jumpTableNum 47:jump 52:pushReturnOpts 53:pushResult 54:endCatch 55:returnStk 56:endCatch 57:jump 62:endCatch 63:jump 68:pushResult 69:endCatch 70:strcat 72:push 77:strcat 79:done",
    ),
    (
        "subst {[foo $x][bar]}",
        "0:push 5:jump 10:jump 15:beginCatch 20:push 25:loadScalar 30:invokeStk 35:endCatch 36:jump 41:pushReturnCode 42:jumpTableNum 47:jump 52:pushReturnOpts 53:pushResult 54:endCatch 55:returnStk 56:endCatch 57:jump 62:endCatch 63:jump 68:pushResult 69:endCatch 70:strcat 72:beginCatch 77:push 82:invokeStk 87:endCatch 88:jump 93:pushReturnCode 94:jumpTableNum 99:jump 104:pushReturnOpts 105:pushResult 106:endCatch 107:returnStk 108:endCatch 109:jump 114:endCatch 115:jump 120:pushResult 121:endCatch 122:strcat 124:done",
    ),
];

#[test]
fn subst_compiles_as_tclsh_9_1() {
    for (body, tcl91) in SUBST_CASES {
        let ours = disassembly("tcl9.1", body)
            .into_iter()
            .map(|(pc, op)| format!("{pc}:{op}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(ours, *tcl91, "{body}");
    }
}

/// tclsh 8.6.18, 9.0.4 and 9.1.0 give [`SUBST_ANSWERS`] for this script.
const SUBST_RUN: &str = r#"proc ok {} {subst {a[string length xyz]b}}
proc ret {} {subst {a[return R]b}}
proc brk {} {subst {a[set ::n 1][break][set ::n 2]c}}
proc cont {} {subst {a[continue]b[string length xy]c}}
proc err {} {subst {a[error boom]b}}
proc other {} {subst {a[return -code 5 Q]b}}
proc two {} {subst {[string length ab][continue]x[return Z]}}
set out {}
foreach p {ok ret brk cont err other two} {
    set ::n 0
    set code [catch {$p} res]
    lappend out "$p=$code:$res:$::n"
}
join $out \n"#;

const SUBST_ANSWERS: &str = r"ok=0:a3b:0
ret=0:aRb:0
brk=0:a1:1
cont=0:ab2c:0
err=1:boom:0
other=0:aQb:0
two=0:2xZ:0";

#[test]
fn subst_dispatch_answers_as_tclsh_does() {
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::interpreter(
            tcl_dialect::DialectProfile::find(profile).expect("release profile"),
        );
        let completion = vm
            .try_eval_source(SUBST_RUN)
            .unwrap_or_else(|error| panic!("{profile}: {error:?}"));
        assert_eq!(
            String::from_utf8_lossy(completion.result.string_bytes().as_ref()),
            SUBST_ANSWERS,
            "{profile}"
        );
    }
}
