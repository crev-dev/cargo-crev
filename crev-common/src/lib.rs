//! Bunch of code that is auxiliary and common for all `crev`

pub mod blake2b256;
pub mod convert;
pub mod fs;
pub mod rand;
pub mod serde;

pub use crate::blake2b256::Blake2b256;

use failure::bail;

use blake2::{digest::FixedOutput, Digest};
use failure::format_err;
use std::{
    collections::HashSet,
    env,
    ffi::OsString,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
    process,
};

/// Now with a fixed offset of the current system timezone
pub fn now() -> chrono::DateTime<chrono::offset::FixedOffset> {
    let date = chrono::offset::Local::now();
    date.with_timezone(&date.offset())
}

pub fn blake2b256sum(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Blake2b256::new();
    hasher.input(bytes);
    hasher.fixed_result().to_vec()
}

pub fn blake2b256sum_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut hasher = Blake2b256::new();
    read_file_to_digest_input(path, &mut hasher)?;
    Ok(hasher.fixed_result().to_vec())
}

pub fn base64_decode<T: ?Sized + AsRef<[u8]>>(input: &T) -> Result<Vec<u8>, base64::DecodeError> {
    base64::decode_config(input, base64::URL_SAFE_NO_PAD)
}

pub fn base64_encode<T: ?Sized + AsRef<[u8]>>(input: &T) -> String {
    base64::encode_config(input, base64::URL_SAFE_NO_PAD)
}

/// Takes a name and converts it to something safe for use in paths etc.
///
/// # Examples
///
/// ```
/// # use std::path::Path;
/// # use crev_common::sanitize_name_for_fs;
/// // Pass through when able
/// assert_eq!(sanitize_name_for_fs("lazy_static"), Path::new("lazy_static-Bda78Hdy9hiPaGTczi9ADA"));
///
/// // Hash reserved windows filenames (or any other 3 letter name)
/// assert_eq!(sanitize_name_for_fs("CON"), Path::new("CON--NhvzH8hSGvoA4DSfBFbpg"));
///
/// // Hash on escaped chars to avoid collisions
/// assert_eq!(sanitize_name_for_fs("://baluga.?io"), Path::new("___baluga__io-7zPdDFu-AyMMKrFrpmY7BQ"));
///
/// // Limit absurdly long names.  Combining a bunch of these can still run into filesystem limits however.
/// let a16   = std::iter::repeat("a").take(  16).collect::<String>();
/// let a2048 = std::iter::repeat("a").take(2048).collect::<String>();
/// let a2049 = std::iter::repeat("a").take(2049).collect::<String>();
/// assert_eq!(sanitize_name_for_fs(a2048.as_str()).to_str().unwrap(), format!("{}-4iupJgrBwxluPQ8DRmrnXg", a16));
/// assert_eq!(sanitize_name_for_fs(a2049.as_str()).to_str().unwrap(), format!("{}-VMRqy6kfWHPoPp1iKIGt1A", a16));
/// ```
pub fn sanitize_name_for_fs(s: &str) -> PathBuf {
    let mut buffer = String::new();
    for ch in s.chars().take(16) {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => buffer.push(ch),
            _ => {
                // Intentionally 'escaped' here:
                //  '.' (path navigation attacks, and windows doesn't like leading/trailing '.'s)
                //  ':' (windows reserves this for drive letters)
                //  '/', '\\' (path navigation attacks)
                // Unicode, Punctuation (out of an abundance of cross platform paranoia)
                buffer.push('_');
            }
        }
    }
    buffer.push('-');
    buffer.push_str(&base64_encode(&blake2b256sum(s.as_bytes())[..16]));
    PathBuf::from(buffer)
}

/// Takes an url and converts it to something safe for use in paths etc.
///
/// # Examples
///
/// ```
/// # use std::path::Path;
/// # use crev_common::sanitize_url_for_fs;
/// // Hash on escaped chars to avoid collisions
/// assert_eq!(sanitize_url_for_fs("https://crates.io"), Path::new("crates_io-yTEHLALL07ZuqIYj8EHFkg"));
///
/// // Limit absurdly long names.  Combining a bunch of these can still run into filesystem limits however.
/// let a48   = std::iter::repeat("a").take(  48).collect::<String>();
/// let a2048 = std::iter::repeat("a").take(2048).collect::<String>();
/// let a2049 = std::iter::repeat("a").take(2049).collect::<String>();
/// assert_eq!(sanitize_url_for_fs(a2048.as_str()).to_str().unwrap(), format!("{}-4iupJgrBwxluPQ8DRmrnXg", a48));
/// assert_eq!(sanitize_url_for_fs(a2049.as_str()).to_str().unwrap(), format!("{}-VMRqy6kfWHPoPp1iKIGt1A", a48));
/// ```
pub fn sanitize_url_for_fs(url: &str) -> PathBuf {
    let mut buffer = String::new();

    let trimmed = url.trim();

    let stripped = if trimmed.starts_with("http://") {
        &trimmed["http://".len()..]
    } else if trimmed.starts_with("https://") {
        &trimmed["https://".len()..]
    } else {
        trimmed
    };

    for ch in stripped.chars().take(48) {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => buffer.push(ch),
            _ => {
                // Intentionally 'escaped' here:
                //  '.' (path navigation attacks, and windows doesn't like leading/trailing '.'s)
                //  ':' (windows reserves this for drive letters)
                //  '/', '\\' (path navigation attacks)
                // Unicode, Punctuation (out of an abundance of cross platform paranoia)
                buffer.push('_');
            }
        }
    }
    buffer.push('-');
    buffer.push_str(&base64_encode(&blake2b256sum(trimmed.as_bytes())[..16]));
    PathBuf::from(buffer)
}

pub fn is_equal_default<T: Default + PartialEq>(t: &T) -> bool {
    *t == T::default()
}

pub fn is_vec_empty<T>(t: &[T]) -> bool {
    t.is_empty()
}

pub fn is_set_empty<T>(t: &HashSet<T>) -> bool {
    t.is_empty()
}

pub fn read_file_to_digest_input(
    path: &Path,
    input: &mut impl blake2::digest::Input,
) -> io::Result<()> {
    let file = std::fs::File::open(path)?;

    let mut reader = io::BufReader::new(file);

    loop {
        let length = {
            let buffer = reader.fill_buf()?;
            input.input(buffer);
            buffer.len()
        };
        if length == 0 {
            break;
        }
        reader.consume(length);
    }

    Ok(())
}

pub fn try_again_or_cancel() -> common_failures::Result<()> {
    if !yes_or_no_was_y("Try again (y/n) ")? {
        bail!("Canceled by the user");
    }

    Ok(())
}

pub fn yes_or_no_was_y(msg: &str) -> io::Result<bool> {
    loop {
        let reply = rprompt::prompt_reply_stderr(msg)?;

        match reply.as_str() {
            "y" | "Y" => return Ok(true),
            "n" | "N" => return Ok(false),
            _ => {}
        }
    }
}

pub fn run_with_shell_cmd(
    cmd: OsString,
    arg: Option<&Path>,
) -> io::Result<std::process::ExitStatus> {
    Ok(run_with_shell_cmd_custom(cmd, arg, false)?.status)
}

pub fn run_with_shell_cmd_capture_stdout(cmd: OsString, arg: Option<&Path>) -> io::Result<Vec<u8>> {
    let output = run_with_shell_cmd_custom(cmd, arg, true)?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            io::ErrorKind::Other,
            "command failed with non-zero status",
        ));
    }
    Ok(output.stdout)
}

pub fn run_with_shell_cmd_custom(
    cmd: OsString,
    arg: Option<&Path>,
    capture_stdout: bool,
) -> io::Result<std::process::Output> {
    Ok(if cfg!(windows) {
        // cmd.exe /c "..." or cmd.exe /k "..." avoid unescaping "...", which makes .arg()'s built-in escaping problematic:
        // https://github.com/rust-lang/rust/blob/379c380a60e7b3adb6c6f595222cbfa2d9160a20/src/libstd/sys/windows/process.rs#L488
        // We can bypass this by (ab)using env vars.  Bonus points:  invalid unicode still works.
        let mut proc = process::Command::new("cmd.exe");
        if let Some(arg) = arg {
            proc.arg("/c").arg("%CREV_CMD% %CREV_ARG%");
            proc.env("CREV_CMD", &cmd);
            proc.env("CREV_ARG", arg);
        } else {
            proc.arg("/c").arg("%CREV_CMD%");
            proc.env("CREV_CMD", &cmd);
        }
        proc
    } else if cfg!(unix) {
        let mut proc = process::Command::new("/bin/sh");
        if let Some(arg) = arg {
            proc.arg("-c").arg(format!(
                "{} {}",
                cmd.clone().into_string().map_err(|_| std::io::Error::new(
                    io::ErrorKind::InvalidData,
                    "not a valid unicode"
                ))?,
                shell_escape::escape(arg.display().to_string().into())
            ));
        } else {
            proc.arg("-c").arg(format!(
                "{}",
                cmd.clone().into_string().map_err(|_| std::io::Error::new(
                    io::ErrorKind::InvalidData,
                    "not a valid unicode"
                ))?,
            ));
        }
        proc
    } else {
        panic!("What platform are you running this on? Please submit a PR!");
    }
    .stdin(process::Stdio::inherit())
    .stderr(process::Stdio::inherit())
    .stdout(if capture_stdout {
        process::Stdio::piped()
    } else {
        process::Stdio::inherit()
    })
    .output()?)
}

pub fn read_passphrase() -> io::Result<String> {
    if let Ok(pass) = env::var("CREV_PASSPHRASE") {
        eprint!("Using passphrase set in CREV_PASSPHRASE\n");
        return Ok(pass);
    } else if let Some(cmd) = env::var_os("CREV_PASSPHRASE_CMD") {
        return Ok(
            String::from_utf8_lossy(&run_with_shell_cmd_capture_stdout(cmd, None)?)
                .trim()
                .to_owned(),
        );
    }
    eprint!("Enter passphrase to unlock: ");
    rpassword::read_password()
}

pub fn read_new_passphrase() -> io::Result<String> {
    if let Ok(pass) = env::var("CREV_PASSPHRASE") {
        eprint!("Using passphrase set in CREV_PASSPHRASE\n");
        return Ok(pass);
    }
    loop {
        eprint!("Enter new passphrase: ");
        let p1 = rpassword::read_password()?;
        eprint!("Enter new passphrase again: ");
        let p2 = rpassword::read_password()?;
        if p1 == p2 {
            return Ok(p1);
        }
        eprintln!("\nPassphrases don't match, try again.");
    }
}

pub fn read_file_to_string(path: &Path) -> io::Result<String> {
    let mut file = std::fs::File::open(&path)?;
    let mut res = String::new();
    file.read_to_string(&mut res)?;

    Ok(res)
}

pub fn save_to_yaml_file<T>(path: &Path, t: &T) -> common_failures::Result<()>
where
    T: ::serde::Serialize,
{
    std::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| format_err!("Can't save to root path"))?,
    )?;
    let text = serde_yaml::to_string(t)?;
    store_str_to_file(&path, &text)?;
    Ok(())
}

pub fn read_from_yaml_file<T>(path: &Path) -> common_failures::Result<T>
where
    T: ::serde::de::DeserializeOwned,
{
    let text = read_file_to_string(path)?;

    Ok(serde_yaml::from_str(&text)?)
}

pub fn store_str_to_file(path: &Path, s: &str) -> io::Result<()> {
    std::fs::create_dir_all(path.parent().expect("Not a root path"))?;
    let tmp_path = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp_path)?;
    file.write_all(&s.as_bytes())?;
    file.flush()?;
    drop(file);
    std::fs::rename(tmp_path, path)?;
    Ok(())
}

pub fn store_to_file_with<E, F>(path: &Path, f: F) -> io::Result<Result<(), E>>
where
    F: Fn(&mut dyn io::Write) -> Result<(), E>,
{
    std::fs::create_dir_all(path.parent().expect("Not a root path"))?;
    let tmp_path = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp_path)?;
    if let Err(e) = f(&mut file) {
        return Ok(Err(e));
    }
    file.flush()?;
    file.sync_data()?;
    drop(file);
    std::fs::rename(tmp_path, path)?;
    Ok(Ok(()))
}
