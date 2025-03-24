use crev_lib::VerificationStatus;
use std::{
    env,
    fmt::Arguments,
    io::{self, Write},
};
use term::{
    self,
    color::{self, Color},
    StderrTerminal, StdoutTerminal,
};

use crate::creds;

pub fn verification_status_color(s: VerificationStatus) -> Option<color::Color> {
    use VerificationStatus::*;
    match s {
        Verified | Local => Some(term::color::GREEN),
        Insufficient => None,
        Negative => Some(term::color::YELLOW),
    }
}

pub fn known_owners_count_color(count: u64) -> Option<color::Color> {
    if count > 0 {
        Some(color::GREEN)
    } else {
        None
    }
}

/// Helper to control (possibly) colored output
pub struct Term {
    pub stdout_is_tty: bool,
    stderr_is_tty: bool,
    stdin_is_tty: bool,
    stdout: Option<Box<StdoutTerminal>>,
    #[allow(unused)]
    stderr: Option<Box<StderrTerminal>>,
}

fn output_to<O>(
    args: std::fmt::Arguments<'_>,
    color: Option<Color>,
    term: &mut dyn term::Terminal<Output = O>,
    is_tty: bool,
) -> io::Result<()>
where
    O: Write,
{
    let use_color = is_tty && term.supports_color();
    if use_color {
        if let Some(color) = color {
            term.fg(color)?;
        }
    }
    term.get_mut().write_fmt(args)?;

    if use_color && color.is_some() {
        term.reset()?;
    }

    Ok(())
}

impl Term {
    pub fn new() -> Term {
        Term {
            stdout: term::stdout(),
            stderr: term::stderr(),
            stdin_is_tty: atty::is(atty::Stream::Stdin),
            stdout_is_tty: atty::is(atty::Stream::Stdout),
            stderr_is_tty: atty::is(atty::Stream::Stderr),
        }
    }

    pub fn print<C>(&mut self, fmt: Arguments<'_>, color: C) -> io::Result<()>
    where
        C: Into<Option<Color>>,
    {
        let color = color.into();

        if let Some(ref mut term) = self.stdout {
            output_to(
                fmt,
                color,
                (&mut **term) as &mut dyn term::Terminal<Output = _>,
                self.stdout_is_tty,
            )?;
        }
        Ok(())
    }

    pub fn eprint<C>(&mut self, fmt: Arguments<'_>, color: C) -> io::Result<()>
    where
        C: Into<Option<Color>>,
    {
        let color = color.into();

        if let Some(ref mut term) = self.stderr {
            output_to(
                fmt,
                color,
                (&mut **term) as &mut dyn term::Terminal<Output = _>,
                self.stdout_is_tty,
            )?;
        }
        Ok(())
    }

    pub fn eprintln<C>(&mut self, fmt: Arguments<'_>, color: C) -> io::Result<()>
    where
        C: Into<Option<Color>>,
    {
        let color = color.into();
        self.print(fmt, color)?;
        self.print(format_args!("\n"), color)?;

        Ok(())
    }

    pub(crate) fn is_interactive(&self) -> bool {
        self.stderr_is_tty && self.stdout_is_tty
    }

    pub(crate) fn is_input_interactive(&self) -> bool {
        self.stdin_is_tty
    }
}

pub fn read_passphrase() -> io::Result<String> {
    #[cfg(target_os = "macos")]
    let by_keychain = creds::retrieve_existing_passphrase(creds::NO_ID).ok();
    #[cfg(not(target_os = "macos"))]
    let by_keychain = None;

    if let Some(pass) = by_keychain {
        eprintln!("Using passphrase retrieved from KeyChain");
        Ok(pass)
    } else if let Ok(pass) = env::var("CREV_PASSPHRASE") {
        eprintln!("Using passphrase set in CREV_PASSPHRASE");
        Ok(pass)
    } else if let Some(cmd) = env::var_os("CREV_PASSPHRASE_CMD") {
        Ok(
            String::from_utf8_lossy(&crev_common::run_with_shell_cmd_capture_stdout(&cmd, None)?)
                .trim()
                .to_owned(),
        )
    } else {
        eprint!("Enter passphrase to unlock: ");
        rpassword::read_password()
    }
}

pub fn read_new_passphrase() -> io::Result<String> {
    let password = if let Ok(pass) = env::var("CREV_PASSPHRASE") {
        eprintln!("Using passphrase set in CREV_PASSPHRASE");
        pass
    } else {
        'term: loop {
            eprint!("Enter new passphrase: ");
            let p1 = rpassword::read_password()?;
            eprint!("Enter new passphrase again: ");
            let p2 = rpassword::read_password()?;
            if p1 == p2 {
                break 'term p1;
            }
            eprintln!("\nPassphrases don't match, try again.");
        }
    };

    #[cfg(target_os = "macos")]
    creds::save_new_passphrase(creds::NO_ID, &password).ok();

    Ok(password)
}
