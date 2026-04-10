//! Side-effect metadata for structured effect analysis.

/// What kind of external state a command affects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideEffectTarget {
    /// Unknown or unclassified effect.
    Unknown,
    /// Variable mutation.
    Variable,
    /// File system I/O.
    FileIo,
    /// Network I/O.
    NetworkIo,
    /// Process management.
    Process,
    /// Channel I/O.
    ChannelIo,
    /// Interpreter state.
    InterpState,
}

/// Which connection side a command operates on (iRules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionSide {
    /// No connection side (not iRules or side-neutral).
    None,
    /// Client side.
    Client,
    /// Server side.
    Server,
}

/// Structured side-effect declaration for a command or subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SideEffect {
    /// What kind of state is affected.
    pub target: SideEffectTarget,
    /// Whether the command reads from the target.
    pub reads: bool,
    /// Whether the command writes to the target.
    pub writes: bool,
    /// Connection side (iRules).
    pub connection_side: ConnectionSide,
}

/// Inferred storage type for a command's target variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageType {
    /// Dictionary.
    Dict,
    /// List.
    List,
    /// Array.
    Array,
}
