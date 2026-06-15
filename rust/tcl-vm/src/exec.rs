//! The non-recursive (NRE / trampoline) execution engine.
//!
//! The VM owns an explicit stack of **activation records** and trampolines over
//! it, mirroring C Tcl's `TEBCresume`. A proc call pushes a new [`Frame`] (and a
//! [`crate::interp::Vm`] call-frame) instead of recursing, so deep recursion /
//! `tailcall` / coroutines need no host-stack rewrite (steering §8b). Completion
//! codes unwind the activation stack: `Return` is absorbed at a proc boundary
//! (→ `Ok`), `Error`/`Break`/`Continue` propagate to the top of this `run`
//! (where `catch`, which invoked us via `eval_source`, observes them).

use std::collections::HashMap;
use std::rc::Rc;

use tcl_bytecode::{FunctionAsm, Instruction, ModuleAsm, Op, Operand};
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::expr::{BinOp, UnaryOp};

use crate::command::{Command, ProcDef};
use crate::expr;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// One activation record: a bytecode function in mid-execution.
struct Frame {
    asm: Rc<FunctionAsm>,
    off2idx: Rc<HashMap<i32, usize>>,
    pc: usize,
    stack: Vec<Value>,
    /// Last value dropped by `POP` — the `DONE` result when the stack is empty.
    last_result: Value,
    /// Whether this activation owns a `Vm` call-frame (a proc body) that must be
    /// popped, and whose boundary absorbs `Return`.
    is_proc: bool,
}

impl Frame {
    fn new(asm: Rc<FunctionAsm>, is_proc: bool) -> Self {
        let off2idx = Rc::new(build_off2idx(&asm));
        Self {
            asm,
            off2idx,
            pc: 0,
            stack: Vec::new(),
            last_result: Value::empty(),
            is_proc,
        }
    }
}

/// What one instruction step does to the trampoline.
enum Tick {
    /// Stay in the current frame.
    Continue,
    /// The current frame finished — unwind with this completion.
    Return(Completion<Value>),
    /// Call a proc — push a new activation + call-frame.
    Call { proc: Rc<ProcDef>, argv: Vec<Value> },
}

fn build_off2idx(asm: &FunctionAsm) -> HashMap<i32, usize> {
    asm.instructions
        .iter()
        .enumerate()
        .map(|(i, instr)| (instr.offset, i))
        .collect()
}

fn imm0(instr: &Instruction) -> i32 {
    match instr.operands.first() {
        Some(Operand::Imm(n)) => *n,
        _ => 0,
    }
}

fn imm_at(instr: &Instruction, idx: usize) -> i32 {
    match instr.operands.get(idx) {
        Some(Operand::Imm(n)) => *n,
        _ => 0,
    }
}

fn label0(instr: &Instruction) -> Option<&str> {
    match instr.operands.first() {
        Some(Operand::Label(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn pop(f: &mut Frame) -> Value {
    f.stack.pop().unwrap_or_else(Value::empty)
}

fn code_from_int(n: i32) -> Code {
    match n {
        1 => Code::Error,
        2 => Code::Return,
        3 => Code::Break,
        4 => Code::Continue,
        _ => Code::Ok,
    }
}

fn label_to_idx(asm: &FunctionAsm, off2idx: &HashMap<i32, usize>, label: &str) -> Option<usize> {
    let off = *asm.labels.get(label)?;
    let off_i32 = i32::try_from(off).ok()?;
    Some(
        off2idx
            .get(&off_i32)
            .copied()
            .unwrap_or(asm.instructions.len()),
    )
}

/// Resolve a jump operand (`Operand::Label`) to a target instruction index.
fn jump_target(
    asm: &FunctionAsm,
    off2idx: &HashMap<i32, usize>,
    instr: &Instruction,
) -> Option<usize> {
    label_to_idx(asm, off2idx, label0(instr)?)
}

fn bin(f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    match expr::arith(op, &a, &b) {
        Ok(v) => {
            f.stack.push(v);
            Ok(())
        }
        Err(e) => Err(err(e.message)),
    }
}

fn cmp(f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    match expr::compare(op, &a, &b) {
        Ok(t) => {
            f.stack.push(Value::bool(t));
            Ok(())
        }
        Err(e) => Err(err(e.message)),
    }
}

fn un(f: &mut Frame, op: UnaryOp) -> Result<(), Completion<Value>> {
    let v = pop(f);
    match expr::unary(op, &v) {
        Ok(r) => {
            f.stack.push(r);
            Ok(())
        }
        Err(e) => Err(err(e.message)),
    }
}

/// The Tcl `wrong # args` usage message for a proc.
fn proc_usage(proc: &ProcDef) -> String {
    let simple = proc.name.rsplit("::").next().unwrap_or(&proc.name);
    let mut s = simple.to_owned();
    for p in &proc.params {
        s.push(' ');
        if p.name == "args" {
            s.push_str("?arg ...?");
        } else if p.default.is_some() {
            s.push('?');
            s.push_str(&p.name);
            s.push('?');
        } else {
            s.push_str(&p.name);
        }
    }
    format!("wrong # args: should be \"{s}\"")
}

impl Vm {
    /// Run a module: register its compiled procs, then run the top-level script.
    pub fn run_module(&mut self, module: &ModuleAsm) -> Completion<Value> {
        self.merge_procs(&module.procedures);
        self.run_function(&module.top_level)
    }

    /// Run one bytecode function to completion via the NRE trampoline.
    pub fn run_function(&mut self, asm: &FunctionAsm) -> Completion<Value> {
        let mut acts: Vec<Frame> = vec![Frame::new(Rc::new(asm.clone()), false)];
        loop {
            let tick = {
                let top = acts.last_mut().expect("activation stack is non-empty");
                self.tick(top)
            };
            match tick {
                Tick::Continue => {}
                Tick::Call { proc, argv } => match self.enter_proc(&proc, &argv) {
                    Ok(()) => acts.push(Frame::new(Rc::clone(&proc.body), true)),
                    Err(c) => {
                        if let Some(done) = self.unwind(&mut acts, c) {
                            return done;
                        }
                    }
                },
                Tick::Return(c) => {
                    if let Some(done) = self.unwind(&mut acts, c) {
                        return done;
                    }
                }
            }
        }
    }

    /// Unwind one or more activations with completion `c`. Returns `Some` when
    /// the whole `run` is finished, `None` when a parent activation resumed.
    fn unwind(
        &mut self,
        acts: &mut Vec<Frame>,
        mut c: Completion<Value>,
    ) -> Option<Completion<Value>> {
        loop {
            let act = acts.pop().expect("unwinding a non-empty stack");
            if act.is_proc {
                self.pop_call_frame();
                if c.code == Code::Return {
                    c = ok(c.result);
                }
            }
            match acts.last_mut() {
                None => return Some(c),
                Some(parent) => {
                    if c.code.is_ok() {
                        parent.stack.push(c.result);
                        return None;
                    }
                    // Error / Break / Continue / unabsorbed Return keep unwinding.
                }
            }
        }
    }

    /// Push a call-frame and bind `argv` to the proc's parameters.
    fn enter_proc(&mut self, proc: &ProcDef, argv: &[Value]) -> Result<(), Completion<Value>> {
        let simple = proc.name.rsplit("::").next().unwrap_or(&proc.name);
        let mut call_argv = Vec::with_capacity(argv.len() + 1);
        call_argv.push(Value::string(simple));
        call_argv.extend(argv.iter().cloned());
        self.push_call_frame(Some(proc.name.clone()), call_argv);

        let mut i = 0;
        let n = proc.params.len();
        for (idx, p) in proc.params.iter().enumerate() {
            if proc.has_args && idx == n - 1 {
                let rest: Vec<Value> = argv.get(i..).unwrap_or(&[]).to_vec();
                self.set_local("args", Value::list(rest));
                i = argv.len();
            } else if i < argv.len() {
                self.set_local(&p.name, argv[i].clone());
                i += 1;
            } else if let Some(d) = &p.default {
                self.set_local(&p.name, d.clone());
            } else {
                self.pop_call_frame();
                return Err(err(proc_usage(proc)));
            }
        }
        if i < argv.len() {
            self.pop_call_frame();
            return Err(err(proc_usage(proc)));
        }
        Ok(())
    }

    /// Execute a single instruction of the top activation.
    #[allow(clippy::too_many_lines)]
    fn tick(&mut self, f: &mut Frame) -> Tick {
        let asm = Rc::clone(&f.asm);
        if f.pc >= asm.instructions.len() {
            return Tick::Return(ok(f
                .stack
                .last()
                .cloned()
                .unwrap_or_else(|| f.last_result.clone())));
        }
        let instr = &asm.instructions[f.pc];
        f.pc += 1;
        let lits = asm.literals.entries();
        let lvt = asm.lvt.entries();

        macro_rules! try_op {
            ($e:expr) => {
                if let Err(c) = $e {
                    return Tick::Return(c);
                }
            };
        }

        let lvt_name = |slot: i32| -> String {
            lvt.get(usize::try_from(slot).unwrap_or(usize::MAX))
                .cloned()
                .unwrap_or_default()
        };

        match instr.op {
            // -- stack --
            Op::PUSH1 | Op::PUSH4 => {
                let raw = usize::try_from(imm0(instr))
                    .ok()
                    .and_then(|idx| lits.get(idx))
                    .cloned()
                    .unwrap_or_default();
                match crate::subst::subst_word(&raw, self) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }
            Op::POP => {
                if let Some(v) = f.stack.pop() {
                    f.last_result = v;
                }
            }
            Op::DUP => {
                if let Some(v) = f.stack.last().cloned() {
                    f.stack.push(v);
                }
            }
            Op::OVER => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if let Some(i) = f.stack.len().checked_sub(n + 1) {
                    let v = f.stack[i].clone();
                    f.stack.push(v);
                }
            }
            Op::STR_CONCAT1 => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < n {
                    return Tick::Return(err("strcat: stack underflow"));
                }
                let parts = f.stack.split_off(f.stack.len() - n);
                let mut s = String::new();
                for p in &parts {
                    s.push_str(&p.to_str());
                }
                f.stack.push(Value::string(s));
            }
            Op::NOP | Op::START_CMD => {}

            // -- variables (stack form, by name) --
            Op::LOAD_STK => {
                let name = pop(f).to_str();
                match self.get_var(&name) {
                    Some(v) => f.stack.push(v),
                    None => {
                        return Tick::Return(err(format!(
                            "can't read \"{name}\": no such variable"
                        )));
                    }
                }
            }
            Op::STORE_STK => {
                let value = pop(f);
                let name = pop(f).to_str();
                self.set_var(&name, value.clone());
                f.stack.push(value);
            }
            Op::INCR_STK_IMM => {
                let name = pop(f).to_str();
                try_op!(self.incr_var(f, &name, i64::from(imm0(instr))));
            }
            Op::INCR_STK => {
                let amount = pop(f);
                let name = pop(f).to_str();
                match amount.as_int() {
                    Ok(a) => try_op!(self.incr_var(f, &name, a)),
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }

            // -- variables (LVT form, proc bodies) --
            Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4 => {
                let name = lvt_name(imm0(instr));
                match self.get_var(&name) {
                    Some(v) => f.stack.push(v),
                    None => {
                        return Tick::Return(err(format!(
                            "can't read \"{name}\": no such variable"
                        )));
                    }
                }
            }
            Op::STORE_SCALAR1 | Op::STORE_SCALAR4 => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                self.set_var(&name, value.clone());
                f.stack.push(value);
            }
            Op::INCR_SCALAR1 => {
                let name = lvt_name(imm0(instr));
                let amount = pop(f);
                match amount.as_int() {
                    Ok(a) => try_op!(self.incr_var(f, &name, a)),
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }
            Op::INCR_SCALAR1_IMM => {
                let name = lvt_name(imm0(instr));
                try_op!(self.incr_var(f, &name, i64::from(imm_at(instr, 1))));
            }

            // -- arithmetic / bitwise / shift --
            Op::ADD => try_op!(bin(f, BinOp::Add)),
            Op::SUB => try_op!(bin(f, BinOp::Sub)),
            Op::MULT => try_op!(bin(f, BinOp::Mul)),
            Op::DIV => try_op!(bin(f, BinOp::Div)),
            Op::MOD => try_op!(bin(f, BinOp::Mod)),
            Op::EXPON => try_op!(bin(f, BinOp::Pow)),
            Op::LSHIFT => try_op!(bin(f, BinOp::LShift)),
            Op::RSHIFT => try_op!(bin(f, BinOp::RShift)),
            Op::BITAND => try_op!(bin(f, BinOp::BitAnd)),
            Op::BITOR => try_op!(bin(f, BinOp::BitOr)),
            Op::BITXOR => try_op!(bin(f, BinOp::BitXor)),

            // -- comparisons --
            Op::EQ => try_op!(cmp(f, BinOp::Eq)),
            Op::NEQ => try_op!(cmp(f, BinOp::Ne)),
            Op::LT => try_op!(cmp(f, BinOp::Lt)),
            Op::GT => try_op!(cmp(f, BinOp::Gt)),
            Op::LE => try_op!(cmp(f, BinOp::Le)),
            Op::GE => try_op!(cmp(f, BinOp::Ge)),
            Op::STR_EQ => try_op!(cmp(f, BinOp::StrEq)),
            Op::STR_NEQ => try_op!(cmp(f, BinOp::StrNe)),

            // -- unary --
            Op::UMINUS => try_op!(un(f, UnaryOp::Neg)),
            Op::UPLUS => try_op!(un(f, UnaryOp::Pos)),
            Op::BITNOT => try_op!(un(f, UnaryOp::BitNot)),
            Op::NOT | Op::LNOT => try_op!(un(f, UnaryOp::Not)),

            // -- control flow --
            Op::JUMP1 | Op::JUMP4 => {
                if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                    f.pc = idx;
                }
            }
            Op::JUMP_TRUE1 | Op::JUMP_TRUE4 => {
                let c = pop(f);
                match c.as_bool() {
                    Ok(true) => {
                        if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                            f.pc = idx;
                        }
                    }
                    Ok(false) => {}
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }
            Op::JUMP_FALSE1 | Op::JUMP_FALSE4 => {
                let c = pop(f);
                match c.as_bool() {
                    Ok(false) => {
                        if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                            f.pc = idx;
                        }
                    }
                    Ok(true) => {}
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }
            Op::JUMP_TABLE => {
                let key = pop(f).to_str();
                if let Some(jt) = &instr.jump_table
                    && let Some(label) = jt.get(&*key)
                    && let Some(idx) = label_to_idx(&asm, &f.off2idx, label)
                {
                    f.pc = idx;
                }
            }

            // -- break / continue --
            Op::BREAK => {
                return Tick::Return(Completion::new(Code::Break, Value::empty(), Value::empty()));
            }
            Op::CONTINUE => {
                return Tick::Return(Completion::new(
                    Code::Continue,
                    Value::empty(),
                    Value::empty(),
                ));
            }

            // -- return --
            Op::RETURN_IMM | Op::RETURN_STK => {
                let (result, options) = if f.stack.len() >= 2 {
                    let opts = pop(f);
                    let res = pop(f);
                    (res, opts)
                } else {
                    (pop(f), Value::empty())
                };
                let code = if instr.op == Op::RETURN_IMM {
                    code_from_int(imm0(instr))
                } else {
                    Code::Ok
                };
                let final_code = if code == Code::Ok { Code::Return } else { code };
                return Tick::Return(Completion::new(final_code, result, options));
            }

            // -- command dispatch / expr --
            Op::INVOKE_STK1 | Op::INVOKE_STK4 => {
                let argc = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < argc || argc == 0 {
                    return Tick::Return(err("invoke: stack underflow"));
                }
                let words = f.stack.split_off(f.stack.len() - argc);
                let name = words[0].to_str();
                match self.lookup_command(&name) {
                    Some(Command::Proc(p)) => {
                        return Tick::Call {
                            proc: p,
                            argv: words[1..].to_vec(),
                        };
                    }
                    Some(Command::Builtin(bf)) => {
                        let res = bf(self, &words[1..]);
                        if res.code.is_ok() {
                            f.stack.push(res.result);
                        } else {
                            return Tick::Return(res);
                        }
                    }
                    None => {
                        return Tick::Return(err(format!("invalid command name \"{name}\"")));
                    }
                }
            }
            Op::EXPR_STK => {
                let s = pop(f).to_str();
                match self.eval_expr(&s) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }

            // -- termination --
            Op::DONE => {
                return Tick::Return(ok(f
                    .stack
                    .last()
                    .cloned()
                    .unwrap_or_else(|| f.last_result.clone())));
            }

            other => {
                return Tick::Return(err(format!(
                    "opcode {} not implemented in tcl-vm",
                    other.mnemonic()
                )));
            }
        }

        Tick::Continue
    }

    /// `incr` helper shared by the scalar/stk increment opcodes.
    fn incr_var(
        &mut self,
        f: &mut Frame,
        name: &str,
        amount: i64,
    ) -> Result<(), Completion<Value>> {
        let old = match self.get_var(name) {
            Some(v) => v.as_int().map_err(|e| err(e.message))?,
            None => 0,
        };
        let next = Value::int(old.wrapping_add(amount));
        self.set_var(name, next.clone());
        f.stack.push(next);
        Ok(())
    }
}
