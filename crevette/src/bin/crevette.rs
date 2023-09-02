use crevette::Crevette;
use std::error::Error as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    match Crevette::new().and_then(|c| c.convert_into_repo()) {
        Ok(res) => {
            println!(
                "Wrote '{}'\nRun `cargo crev publish` to upload the file to {}\nThen run `cargo vet import yourname {}`\n",
                res.local_path.display(),
                res.repo_git_url.as_deref().unwrap_or("your git repo (not configured yet?)"),
                res.repo_https_url.as_deref().unwrap_or("https://<your repo URL>/audits.toml"),
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            let mut source = e.source();
            while let Some(e) = source {
                eprintln!("  {e}");
                source = e.source();
            }
            ExitCode::FAILURE
        }
    }
}
