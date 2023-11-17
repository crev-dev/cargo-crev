use crevette::Crevette;
use std::error::Error as _;
use crevette::Error;
use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        let mut source = e.source();
        while let Some(e) = source {
            eprintln!("  {e}");
            source = e.source();
        }
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run() -> Result<(), Error> {

    match std::env::args().nth(1).as_deref() {
        Some("--help") => {
            eprintln!("https://lib.rs/crevette {}
Run without args to update your crev repo.
Run with --debcargo to make a vet file from Debian package list.", env!("CARGO_PKG_VERSION"));
            return Ok(())
        },
        Some("--debcargo") => {
            if !cfg!(feature = "debcargo") {
                eprintln!("Reinstall with debcargo enabled:\ncargo install crevette --features=debcargo");
                return Err(Error::UnsupportedVersion(0));
            }
            #[cfg(feature = "debcargo")]
            {
                let dirs = directories_next::BaseDirs::new().unwrap();
                let cache_dir = dirs.cache_dir().join("crevette");
                println!("{}", Crevette::from_debcargo_repo(&cache_dir)?);
                return Ok(())
            }
        },
        Some(other) => {
            eprintln!("unknown argument: {other}");
        },
        None => {},
    }
    let res = Crevette::new().and_then(|c| c.convert_into_repo())?;
        println!(
            "Wrote '{}'\nRun `cargo crev publish` to upload the file to {}\nThen run `cargo vet import yourname {}`\n",
            res.local_path.display(),
            res.repo_git_url.as_deref().unwrap_or("your git repo (not configured yet?)"),
            res.repo_https_url.as_deref().unwrap_or("https://<your repo URL>/audits.toml"),
        );
    Ok(())
}
