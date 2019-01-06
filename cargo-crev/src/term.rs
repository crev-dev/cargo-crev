use crev_lib::Colored;
use std::fmt::Arguments;
use std::io::{self, Write};
use term::color::Color;
use term::{self, StderrTerminal, StdoutTerminal};

pub struct Term {
    pub stdout_is_tty: bool,
    pub stderr_is_tty: bool,
    pub stdin_is_tty: bool,
    stdout: Option<Box<StdoutTerminal>>,
    stderr: Option<Box<StderrTerminal>>,
}

fn output_to<O>(
    args: std::fmt::Arguments,
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
            term.fg(color)?
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

    pub fn print<C>(&mut self, fmt: Arguments, color: C) -> io::Result<()>
    where
        C: Into<Option<Color>>,
    {
        let color = color.into();

        if let Some(ref mut term) = self.stdout {
            output_to(
                fmt,
                color,
                (&mut **term) as &mut term::Terminal<Output = _>,
                self.stdout_is_tty,
            )?;
        }
        Ok(())
    }

    pub fn stdout<T>(&mut self, fmt: Arguments, t: &T) -> io::Result<()>
    where
        T: Colored,
    {
        if let Some(ref mut term) = self.stdout {
            output_to(
                fmt,
                t.color(),
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
                t.color(),
                (&mut **term) as &mut term::Terminal<Output = _>,
                self.stderr_is_tty,
            )?;
        }
        Ok(())
    }
}
