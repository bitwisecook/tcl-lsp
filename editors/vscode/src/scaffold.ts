export interface ScaffoldFile {
  relativePath: string;
  content: string;
}

export interface PackageScaffoldOptions {
  packageName: string;
  version: string;
  commandName: string;
  minimumTclVersion?: string;
}

export interface PackageScaffoldPlan {
  directoryName: string;
  mainFile: string;
  readmeFile: string;
  files: ScaffoldFile[];
}

const DEFAULT_TCL_VERSION = "8.6";

export function validatePackageName(value: string): string | undefined {
  const trimmed = value.trim();
  if (!trimmed) {
    return "Package name is required.";
  }
  if (!/^[A-Za-z_][A-Za-z0-9_:.-]*$/.test(trimmed)) {
    return "Use letters, digits, _, :, ., or - and start with a letter or _.";
  }
  return undefined;
}

export function validateCommandName(value: string): string | undefined {
  const trimmed = value.trim();
  if (!trimmed) {
    return "Exported command name is required.";
  }
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(trimmed)) {
    return "Use letters, digits, and _; first character must be a letter or _.";
  }
  return undefined;
}

export function validateVersion(value: string): string | undefined {
  const trimmed = value.trim();
  if (!trimmed) {
    return "Version is required.";
  }
  if (!/^\d+(?:\.\d+)*$/.test(trimmed)) {
    return "Use dotted numeric version format, e.g. 0.1 or 1.0.0.";
  }
  return undefined;
}

function sanitiseIdentifierPart(value: string): string {
  const cleaned = value.replace(/[^A-Za-z0-9_]/g, "_");
  if (!cleaned) {
    return "_";
  }
  if (!/^[A-Za-z_]/.test(cleaned)) {
    return `_${cleaned}`;
  }
  return cleaned;
}

export function namespaceFromPackageName(packageName: string): string {
  const parts = packageName
    .split("::")
    .filter((part) => part.length > 0)
    .map((part) => sanitiseIdentifierPart(part));
  if (parts.length === 0) {
    return "::pkg";
  }
  return `::${parts.join("::")}`;
}

export function packageDirectoryName(packageName: string): string {
  return packageName.replace(/[^A-Za-z0-9_-]/g, "_");
}

export function packageFileStem(packageName: string): string {
  return packageName.replace(/[^A-Za-z0-9_]/g, "_");
}

export function buildPackageScaffold(options: PackageScaffoldOptions): PackageScaffoldPlan {
  const packageName = options.packageName.trim();
  const version = options.version.trim();
  const commandName = options.commandName.trim();
  const minimumTclVersion = options.minimumTclVersion?.trim() || DEFAULT_TCL_VERSION;
  const namespaceName = namespaceFromPackageName(packageName);
  const dirName = packageDirectoryName(packageName);
  const stem = packageFileStem(packageName);
  const mainFile = `src/${stem}.tcl`;
  const readmeFile = "README.md";

  const files: ScaffoldFile[] = [
    {
      relativePath: "pkgIndex.tcl",
      content:
        `if {![package vsatisfies [package provide Tcl] ${minimumTclVersion}]} { return }\n` +
        `package ifneeded ${packageName} ${version} ` +
        `[list source [file join $dir src ${stem}.tcl]]\n`,
    },
    {
      relativePath: mainFile,
      content:
        `# ${packageName} -- generated Tcl package starter\n` +
        `package require Tcl ${minimumTclVersion}\n\n` +
        `namespace eval ${namespaceName} {\n` +
        `    namespace export ${commandName}\n` +
        `}\n\n` +
        `proc ${namespaceName}::${commandName} {name} {\n` +
        `    return "Hello, $name"\n` +
        `}\n\n` +
        `package provide ${packageName} ${version}\n`,
    },
    {
      relativePath: `tests/${stem}.test.tcl`,
      content:
        `package require tcltest 2\n` +
        `namespace import ::tcltest::*\n\n` +
        `source [file join [file dirname [info script]] .. src ${stem}.tcl]\n\n` +
        `test ${stem}-1.0 {${commandName} returns greeting} -body {\n` +
        `    ${namespaceName}::${commandName} Tcl\n` +
        `} -result {Hello, Tcl}\n\n` +
        `cleanupTests\n`,
    },
    {
      relativePath: "tests/run.tcl",
      content:
        `package require tcltest 2\n` +
        `namespace import ::tcltest::*\n\n` +
        `set script_dir [file dirname [info script]]\n` +
        `tcltest::configure -testdir $script_dir -verbose bpse\n` +
        `runAllTests\n`,
    },
    {
      relativePath: ".github/workflows/ci.yml",
      content:
        "name: Tcl CI\n\n" +
        "on:\n" +
        "  push:\n" +
        "  pull_request:\n\n" +
        "jobs:\n" +
        "  test:\n" +
        "    runs-on: ubuntu-latest\n" +
        "    steps:\n" +
        "      - uses: actions/checkout@v4\n" +
        "      - name: Install Tcl\n" +
        "        run: sudo apt-get update && sudo apt-get install -y tcl\n" +
        "      - name: Run package tests\n" +
        "        run: tclsh tests/run.tcl\n",
    },
    {
      relativePath: readmeFile,
      content:
        `# ${packageName}\n\n` +
        "Generated Tcl package scaffold.\n\n" +
        "## Layout\n\n" +
        `- \`${mainFile}\` -- package source and exported command(s)\n` +
        "- `pkgIndex.tcl` -- package loader entrypoint\n" +
        "- `tests/*.test.tcl` -- test files using `tcltest`\n" +
        "- `.github/workflows/ci.yml` -- GitHub Actions test workflow\n\n" +
        "## Quick Start\n\n" +
        "```tcl\n" +
        `source [file join [pwd] pkgIndex.tcl]\n` +
        `package require ${packageName} ${version}\n` +
        `puts [${namespaceName}::${commandName} Tcl]\n` +
        "```\n\n" +
        "## Run Tests\n\n" +
        "```bash\n" +
        "tclsh tests/run.tcl\n" +
        "```\n",
    },
  ];

  return {
    directoryName: dirName,
    mainFile,
    readmeFile,
    files,
  };
}
