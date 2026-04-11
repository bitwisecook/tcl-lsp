//! `file` — manipulate file names and attributes.

use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "atime",
        arity: Arity::new(1, 2),
        detail: "Returns a decimal string giving the time at which file name was last accessed.",
        synopsis: "file atime name ?time?",
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "attributes",
        arity: Arity::at_least(1),
        detail: "Query or set file attributes.",
        synopsis: "file attributes name ?option? ?value ...?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "channels",
        arity: Arity::new(0, 1),
        detail: "Returns a list of names of all registered open channels in this interpreter.",
        synopsis: "file channels ?pattern?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "copy",
        arity: Arity::at_least(2),
        detail: "Copy files or directories.",
        synopsis: "file copy ?-force? ?--? source target",
        return_type: Some(TclType::String),
        mutator: true,
        options: &[
            OptionSpec {
                name: "-force",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(1),
        detail: "Removes the file or directory specified by each pathname argument.",
        synopsis: "file delete ?-force? ?--? ?pathname ...?",
        return_type: Some(TclType::String),
        mutator: true,
        destructive: true,
        options: &[
            OptionSpec {
                name: "-force",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dirname",
        arity: Arity::exact(1),
        detail: "Returns all of the path components in name excluding the last element.",
        synopsis: "file dirname name",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "executable",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is executable by the current user, 0 otherwise.",
        synopsis: "file executable name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name exists and the current user has search privileges for the directories leading to it, 0 otherwise.",
        synopsis: "file exists name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        arity: Arity::exact(1),
        detail: "Returns all of the characters in name after and including the last dot.",
        synopsis: "file extension name",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "home",
        arity: Arity::new(0, 1),
        detail: "Returns the home directory of the current user.",
        synopsis: "file home ?username?",
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isdirectory",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is a directory, 0 otherwise.",
        synopsis: "file isdirectory name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isfile",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is a regular file, 0 otherwise.",
        synopsis: "file isfile name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "join",
        arity: Arity::at_least(1),
        detail: "Combines one or more file names using the correct path separator for the current platform.",
        synopsis: "file join name ?name ...?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "link",
        arity: Arity::new(1, 2),
        detail: "Returns the value of the link given by linkName, or creates a link.",
        synopsis: "file link ?-linktype? linkName ?target?",
        return_type: Some(TclType::String),
        options: &[
            OptionSpec {
                name: "-symbolic",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-hard",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lstat",
        arity: Arity::new(1, 2),
        detail: "Same as stat except uses the lstat kernel call instead of stat.",
        synopsis: "file lstat name ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mkdir",
        arity: Arity::at_least(1),
        detail: "Creates each directory specified.",
        synopsis: "file mkdir ?dir ...?",
        return_type: Some(TclType::String),
        mutator: true,
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mtime",
        arity: Arity::new(1, 2),
        detail: "Returns a decimal string giving the time at which file name was last modified.",
        synopsis: "file mtime name ?time?",
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nativename",
        arity: Arity::exact(1),
        detail: "Returns the platform-specific name of the file.",
        synopsis: "file nativename name",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "normalize",
        arity: Arity::exact(1),
        detail: "Returns a unique normalized path representation for the file-system object.",
        synopsis: "file normalize name",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "owned",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is owned by the current user, 0 otherwise.",
        synopsis: "file owned name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pathtype",
        arity: Arity::exact(1),
        detail: "Returns one of absolute, relative, volumerelative.",
        synopsis: "file pathtype name",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readable",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is readable by the current user, 0 otherwise.",
        synopsis: "file readable name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readlink",
        arity: Arity::exact(1),
        detail: "Returns the value of the symbolic link given by name.",
        synopsis: "file readlink name",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rename",
        arity: Arity::at_least(2),
        detail: "Rename or move files/directories.",
        synopsis: "file rename ?-force? ?--? source target",
        return_type: Some(TclType::String),
        mutator: true,
        destructive: true,
        options: &[
            OptionSpec {
                name: "-force",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rootname",
        arity: Arity::exact(1),
        detail: "Returns all characters in name up to but not including the last dot in the last component.",
        synopsis: "file rootname name",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "separator",
        arity: Arity::new(0, 1),
        detail: "Returns the character used to separate path segments for native files on this platform.",
        synopsis: "file separator ?name?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(1),
        detail: "Returns a decimal string giving the size of file name in bytes.",
        synopsis: "file size name",
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "split",
        arity: Arity::exact(1),
        detail: "Returns a list whose elements are the path components in name.",
        synopsis: "file split name",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "stat",
        arity: Arity::new(1, 2),
        detail: "Invokes the stat kernel call on name and returns the information.",
        synopsis: "file stat name ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "system",
        arity: Arity::exact(1),
        detail: "Returns a list describing the filesystem type for the given file.",
        synopsis: "file system name",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tail",
        arity: Arity::exact(1),
        detail: "Returns all of the characters in the last filesystem component of name.",
        synopsis: "file tail name",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tempdir",
        arity: Arity::new(0, 1),
        detail: "Creates a temporary directory and returns its name.",
        synopsis: "file tempdir ?template?",
        return_type: Some(TclType::String),
        mutator: true,
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tempfile",
        arity: Arity::new(0, 2),
        detail: "Creates a temporary file and returns a read-write channel opened on that file.",
        synopsis: "file tempfile ?nameVar? ?template?",
        return_type: Some(TclType::Channel),
        mutator: true,
        arg_roles: &[(0, ArgRole::VarWrite)],
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tildeexpand",
        arity: Arity::exact(1),
        detail: "Returns the result of performing tilde substitution on name.",
        synopsis: "file tildeexpand name",
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Returns a string giving the type of file name.",
        synopsis: "file type name",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "volumes",
        arity: Arity::exact(0),
        detail: "Returns the absolute paths to the volumes mounted on the system.",
        synopsis: "file volumes",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "writable",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is writable by the current user, 0 otherwise.",
        synopsis: "file writable name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `file`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "file",
        traits: Traits::HAS_DESTRUCTIVE_OPS | Traits::RETURNS_PATH,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Manipulate file names and attributes.",
            &["file option name ?arg arg ...?"],
            "Tcl file(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
