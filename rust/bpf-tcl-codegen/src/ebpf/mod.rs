//! Direct eBPF bytecode backend: BPF-IR → eBPF instructions, encoded by our own
//! 64-bit instruction model (no assembler, no LLVM in the compile path).

pub mod disasm;
pub mod emit;
pub mod insn;

pub use disasm::disasm;
pub use emit::{EbpfObject, emit_program};
pub use insn::Insn;
