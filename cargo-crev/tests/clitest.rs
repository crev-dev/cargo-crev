use std::ffi::*;
use std::io::Write;
use std::path::*;
use std::process::*;

#[test]
fn creates_new_id_implicitly() {
    let c = Cli::new();
    let empty_id = c.run(&["id", "query", "own"], "");
    assert!(!empty_id.status.success(), "{:?}", empty_id);
    let trust = c.run(&["id", "trust", "--level=medium", "FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE"], "");
    assert!(trust.status.success(), "{:?}", trust);
    assert!(c.run(&["id", "query", "own"], "").status.success());
}

struct Cli {
    home: tempfile::TempDir,
    exe: PathBuf,
}

impl Cli {
    pub fn new() -> Self {
        Self {
            exe: PathBuf::from(env!("CARGO_BIN_EXE_cargo-crev")),
            home: tempfile::tempdir().unwrap(),
        }
    }

    pub fn run(&self, args: &[impl AsRef<OsStr>], stdin_data: impl Into<String>) -> Output {
        let mut child = Command::new(&self.exe)
            .env("CARGO_CREV_ROOT_DIR_OVERRIDE", self.home.path())
            .env("EDITOR", "cat")
            .env("VISUAL", "cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg("crev")
            .args(args)
            .spawn()
            .unwrap_or_else(|_| panic!("Failed to run {}", self.exe.display()));

        let stdin_data = stdin_data.into();
        let mut stdin = child.stdin.take().unwrap();
        std::thread::spawn(move || {
            stdin.write_all(stdin_data.as_bytes()).unwrap();
        });
        child.wait_with_output().expect("child process lost")
    }
}
