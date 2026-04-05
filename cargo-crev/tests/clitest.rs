use std::{ffi::*, io::Write, path::*, process::*};

/// Fixture: a package-review proof body (without the `----- BEGIN CREV PROOF -----`
/// envelope or signature), as it would be produced by
/// `cargo crev review --no-store --print-unsigned > file`. Modeled after a
/// real review from an existing proof repository.
const UNSIGNED_REVIEW_FIXTURE: &str = r#"kind: package review
version: -1
date: "2021-08-15T12:34:56.000000000+00:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: log
  version: 0.4.6
  digest: BhDmOOjfESqs8i3z9qsQANH8A39eKklgQKuVtrwN-Tw
review:
  thoroughness: low
  understanding: medium
  rating: positive
comment: "test comment for import fixture"
"#;

#[test]
fn import_unsigned_review_from_file() {
    let c = Cli::new();
    let fixture_path = c.home.path().join("unsigned-review.yaml");
    std::fs::write(&fixture_path, UNSIGNED_REVIEW_FIXTURE).unwrap();

    let out = c.run(
        &[
            "review".as_ref(),
            "--import-unsigned-from".as_ref(),
            fixture_path.as_os_str(),
            "--no-store".as_ref(),
            "--no-edit".as_ref(),
            "--print-unsigned".as_ref(),
        ],
        "",
    );
    assert!(
        out.status.success(),
        "command failed: stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );
    let stdout = String::from_utf8(out.stdout).unwrap();

    // Values from the fixture should round-trip into the printed unsigned body.
    assert!(stdout.contains("name: log"), "stdout: {stdout}");
    assert!(stdout.contains("version: 0.4.6"), "stdout: {stdout}");
    assert!(
        stdout.contains("digest: BhDmOOjfESqs8i3z9qsQANH8A39eKklgQKuVtrwN-Tw"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("thoroughness: low"), "stdout: {stdout}");
    assert!(stdout.contains("understanding: medium"), "stdout: {stdout}");
    assert!(stdout.contains("rating: positive"), "stdout: {stdout}");
    assert!(
        stdout.contains("test comment for import fixture"),
        "stdout: {stdout}"
    );
    // The `from` should have been replaced with the auto-created id, so the
    // original id must not appear in the output.
    assert!(
        !stdout.contains("FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE"),
        "the `from` id should have been replaced; stdout: {stdout}"
    );
    // The `kind` must be backfilled and present so the proof would be valid.
    assert!(stdout.contains("kind: package review"), "stdout: {stdout}");
}

#[test]
#[ignore]
// TODO: rewrite to be a standalone binary
fn creates_new_id_implicitly() {
    let c = Cli::new();
    let empty_id = c.run(&["id", "query", "own"], "");
    assert!(!empty_id.status.success(), "{empty_id:?}");
    let trust = c.run(
        &[
            "id",
            "trust",
            "--level=medium",
            "FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE",
        ],
        "",
    );
    assert!(trust.status.success(), "{trust:?}");
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

    #[track_caller]
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
