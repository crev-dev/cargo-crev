use crev_lib::Colored;
use std::fmt::Arguments;
use std::io::{self, Write};
use term::{self, StderrTerminal, StdoutTerminal};

pub struct Term {
    pub stdout_is_tty: bool,
    pub stderr_is_tty: bool,
    pub stdin_is_tty: bool,
    stdout: Option<Box<StdoutTerminal>>,
    stderr: Option<Box<StderrTerminal>>,
}

fn output_to<T, O>(
    args: std::fmt::Arguments,
    t: &T,
    term: &mut dyn term::Terminal<Output = O>,
    is_tty: bool,
) -> io::Result<()>
where
    T: Colored,
    O: Write,
{
    let use_color = is_tty && term.supports_color();
    if use_color {
        if let Some(color) = t.color() {
            term.fg(color)?
        }
    }
    term.get_mut().write_fmt(args)?;

    if use_color {
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

    pub fn stdout<T>(&mut self, fmt: Arguments, t: &T) -> io::Result<()>
    where
        T: Colored,
    {
        if let Some(ref mut term) = self.stdout {
            output_to(
                fmt,
                t,
                (&mut **term) as &mut term::Terminal<Output = _>,
                self.stdout_is_tty,
            )?;
        }
        Ok(())
    }

    #[allow(unused)]
    pub fn stderr<T>(&mut self, fmt: Arguments, t: &T) -> io::Result<()>
    where
        T: Colored,
    {
        if let Some(ref mut term) = self.stderr {
            output_to(
                fmt,
                t,
                (&mut **term) as &mut term::Terminal<Output = _>,
                self.stderr_is_tty,
            )?;
        }
        Ok(())
    }

    /*
        fn set_term_color(&self, t: &mut Box<term::StdoutTerminal>) -> Result<()> {
            if !t.supports_color() {
                return Ok(());
            }

            match *self {
                VerificationStatus::Verified => {
                    t.fg(term::color::GREEN)?;
                },
                VerificationStatus::Flagged => {
                    t.fg(term::color::RED)?;
                },
                _ => {}
            }
            Ok(())
        }

        pub fn write_colored_to_stdout(&self) -> Result<()> {
            match term::stdout() {
                Some(ref mut t) => {
                    self.set_term_color(t)?;
                    write!(t, "{:8}", *self)?;
                    t.reset()?;
                }
                None => {
                    print!("{:8}", *self);
                }
            }
            Ok(())
        }
    */
}
