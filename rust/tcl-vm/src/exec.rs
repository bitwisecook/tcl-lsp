//! The non-recursive (NRE / trampoline) execution engine.
//!
//! The VM owns an explicit stack of activation records and trampolines over it,
//! mirroring C Tcl's `TEBCresume`. M1 only ever holds one activation (builtins
//! run synchronously and never push frames), but the structure is the M2 slot:
//! a proc `INVOKE` will push a new [`Frame`] instead of recursing, so
//! coroutines / `tailcall` / deep recursion fall out without a host-stack
//! rewrite. See the steering doc §8b.

use std::collections::HashMap;
use std::rc::Rc;

use tcl_bytecode::{FunctionAsm, Instruction, ModuleAsm, Op, Operand};
use tcl_runtime_api::{Completion, FrameId};
use tcl_syntax::expr::{BinOp, UnaryOp};

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
    #[allow(dead_code)] // wired through; used by the M2 frame/var model.
    id: FrameId,
}

/// What one instruction step does to the trampoline.
enum Tick {
    /// Stay in the current frame.
    Continue,
    /// The current frame finished — unwind with this completion.
    Return(Completion<Value>),
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

/// Resolve a jump operand (`Operand::Label`) to a target instruction index.
fn jump_target(
    asm: &FunctionAsm,
    off2idx: &HashMap<i32, usize>,
    instr: &Instruction,
) -> Option<usize> {
    let label = label0(instr)?;
    let off = *asm.labels.get(label)?;
    let off_i32 = i32::try_from(off).ok()?;
    Some(
        off2idx
            .get(&off_i32)
            .copied()
            .unwrap_or(asm.instructions.len()),
    )
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

impl Vm {
    /// Run a module's top-level script. (Procedures are M2.)
    pub fn run_module(&mut self, module: &ModuleAsm) -> Completion<Value> {
        self.run_function(&module.top_level)
    }

    /// Run one bytecode function to completion via the NRE trampoline.
    pub fn run_function(&mut self, asm: &FunctionAsm) -> Completion<Value> {
        let asm = Rc::new(asm.clone());
        let off2idx = Rc::new(build_off2idx(&asm));
        let mut frames = vec![Frame {
            asm,
            off2idx,
            pc: 0,
            stack: Vec::new(),
            last_result: Value::empty(),
            id: FrameId(0),
        }];

        loop {
            let tick = {
                let top = frames.last_mut().expect("frame stack is non-empty");
                self.tick(top)
            };
            match tick {
                Tick::Continue => {}
                Tick::Return(c) => {
                    frames.pop();
                    match frames.last_mut() {
                        // M1: single frame, so this is unreachable; the structure
                        // is the M2 slot where a proc result returns to its caller.
                        Some(parent) => {
                            if !c.code.is_ok() {
                                return c;
                            }
                            parent.stack.push(c.result);
                        }
                        None => return c,
                    }
                }
            }
        }
    }

    /// Execute a single instruction of the top frame.
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
            Op::NOP | Op::START_CMD => {}

            // -- variables (stack form, top level) --
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
                let name = lvt
                    .get(usize::try_from(imm0(instr)).unwrap_or(0))
                    .cloned()
                    .unwrap_or_default();
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
                let name = lvt
                    .get(usize::try_from(imm0(instr)).unwrap_or(0))
                    .cloned()
                    .unwrap_or_default();
                let value = pop(f);
                self.set_var(&name, value.clone());
                f.stack.push(value);
            }
            Op::INCR_SCALAR1 => {
                let name = lvt
                    .get(usize::try_from(imm0(instr)).unwrap_or(0))
                    .cloned()
                    .unwrap_or_default();
                let amount = pop(f);
                match amount.as_int() {
                    Ok(a) => try_op!(self.incr_var(f, &name, a)),
                    Err(e) => return Tick::Return(err(e.message)),
                }
            }
            Op::INCR_SCALAR1_IMM => {
                let name = lvt
                    .get(usize::try_from(imm0(instr)).unwrap_or(0))
                    .cloned()
                    .unwrap_or_default();
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

            // -- command dispatch / expr --
            Op::INVOKE_STK1 | Op::INVOKE_STK4 => {
                let argc = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < argc || argc == 0 {
                    return Tick::Return(err("invoke: stack underflow"));
                }
                let words = f.stack.split_off(f.stack.len() - argc);
                let name = words[0].to_str();
                let res = self.invoke(&name, &words[1..]);
                if !res.code.is_ok() {
                    return Tick::Return(res);
                }
                f.stack.push(res.result);
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
                    "opcode {} not implemented in tcl-vm M1",
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
