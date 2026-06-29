//! Unit tests for Tcl-installation discovery and `libraryPaths` config
//! parsing.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut p = std::env::temp_dir();
        p.push(format!("tcl-lsp-install-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn write(&self, rel: &str, content: &str) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn discover_finds_a_library_subdir_and_its_auto_path() {
    let td = TempDir::new("disc");
    // A `lib`-style base with a `tcl8.6` library and a sibling package.
    td.write("lib/tcl8.6/init.tcl", "# tcl\n");
    td.write(
        "lib/mypkg1.0/pkgIndex.tcl",
        "package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
    );
    let base = td.0.join("lib");

    let found = discover(&[base.clone()]);
    assert_eq!(found.len(), 1, "{found:?}");
    let inst = &found[0];
    assert_eq!(inst.tcl_library, base.join("tcl8.6"));
    assert_eq!(inst.version, "8.6");
    // auto_path = [lib base (sibling packages live here), the library dir].
    assert_eq!(inst.auto_path, vec![base.clone(), base.join("tcl8.6")]);
}

#[test]
fn discover_treats_a_base_that_is_itself_a_library_dir() {
    let td = TempDir::new("baselib");
    td.write("tcl9.0/init.tcl", "# tcl\n");
    let lib = td.0.join("tcl9.0");
    let found = discover(&[lib.clone()]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].tcl_library, lib);
    assert_eq!(found[0].version, "9.0");
}

#[test]
fn discover_dedups_same_library() {
    let td = TempDir::new("dedup");
    td.write("lib/tcl8.6/init.tcl", "# tcl\n");
    let base = td.0.join("lib");
    let lib = base.join("tcl8.6");
    // Base scan + direct lib pointer both reach the same library.
    let found = discover(&[base, lib]);
    assert_eq!(
        found.len(),
        1,
        "duplicate library should appear once: {found:?}"
    );
}

#[test]
fn version_from_a_bundled_library_path() {
    assert_eq!(version_from_path(Path::new("/x/tcl8.6")), "8.6");
    assert_eq!(
        version_from_path(Path::new("/opt/tcl9.0.3/library")),
        "9.0.3"
    );
    assert_eq!(version_from_path(Path::new("/random/scripts")), "");
}

#[test]
fn library_paths_from_ini_multiline_and_comma_forms() {
    // Newline list with configparser indented continuation, [global] section.
    let global = "[global]\n\
                  libraryPaths =\n\
                  \x20   /opt/tcl-extras/lib\n\
                  \x20   /home/me/tcl-stubs\n\
                  \n\
                  [features]\n\
                  hover = true\n";
    assert_eq!(
        library_paths_from_ini(global, "global"),
        vec!["/opt/tcl-extras/lib", "/home/me/tcl-stubs"]
    );
    // The [project] section is not the [global] one.
    assert!(library_paths_from_ini(global, "project").is_empty());

    // One-line comma form.
    let project = "[project]\nlibraryPaths = /a/lib, /b/lib\n";
    assert_eq!(
        library_paths_from_ini(project, "project"),
        vec!["/a/lib", "/b/lib"]
    );

    // Inline single path.
    let one = "[global]\nlibraryPaths = /only/one\n";
    assert_eq!(library_paths_from_ini(one, "global"), vec!["/only/one"]);

    // Absent key.
    assert!(library_paths_from_ini("[global]\ndialect = tcl9.0\n", "global").is_empty());
}

#[test]
fn config_path_precedence_is_platform_native() {
    use std::ffi::OsStr;
    // XDG_CONFIG_HOME wins on every platform.
    assert_eq!(
        config_path_for(
            Some(OsStr::new("/x/xdg")),
            None,
            Some(OsStr::new("/home/me")),
            false,
            false
        ),
        Some(PathBuf::from("/x/xdg/tcl-lsp/config.ini"))
    );
    // Linux/BSD/WSL → ~/.config.
    assert_eq!(
        config_path_for(None, None, Some(OsStr::new("/home/me")), false, false),
        Some(PathBuf::from("/home/me/.config/tcl-lsp/config.ini"))
    );
    // macOS → ~/Library/Application Support.
    assert_eq!(
        config_path_for(None, None, Some(OsStr::new("/Users/me")), false, true),
        Some(PathBuf::from(
            "/Users/me/Library/Application Support/tcl-lsp/config.ini"
        ))
    );
    // Windows → %APPDATA% (build the expected via `join` so the path
    // separator matches the test host).
    let appdata = r"C:\Users\me\AppData\Roaming";
    assert_eq!(
        config_path_for(None, Some(OsStr::new(appdata)), None, true, false),
        Some(PathBuf::from(appdata).join("tcl-lsp").join("config.ini"))
    );
    // Empty XDG is ignored (falls through to HOME).
    assert_eq!(
        config_path_for(
            Some(OsStr::new("")),
            None,
            Some(OsStr::new("/home/me")),
            false,
            false
        ),
        Some(PathBuf::from("/home/me/.config/tcl-lsp/config.ini"))
    );
}

#[test]
fn project_config_path_is_dot_tcl_lsp_ini() {
    let root = Path::new("/ws/project");
    assert_eq!(
        project_config_path(root),
        PathBuf::from("/ws/project/.tcl-lsp.ini")
    );
}
