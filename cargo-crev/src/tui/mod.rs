mod verify_screen;

pub use crate::deps::{DownloadCount, TrustCount, scan};
use crate::opts::Verify;
use crate::prelude::*;
use crate::shared::CommandExitStatus;
use crossterm::{AlternateScreen, KeyEvent, TerminalCursor};
use termimad::{Event, EventSource};
use verify_screen::VerifyScreen;

/// called in case of an --interactive execution
///
/// Right now the --interactive is only possible for "verify" subcommand
///  but this will hopefully change and the public function here would
///  be a run_command
pub fn verify_deps(args: Verify) -> Result<CommandExitStatus> {
    let computer = scan::Scanner::new(&args)?;

    let _alt_screen = AlternateScreen::to_alternate(true);
    let cursor = TerminalCursor::new();
    cursor.hide()?;

    let mut screen = VerifyScreen::new(computer.total_crate_count())?;

    screen.update();
    let crate_stats_rx = computer.run();
    let event_source = EventSource::new();
    let rx_user = event_source.receiver();
    let mut crate_count = 0;

    loop {
        screen.update();
        select! {
            recv(crate_stats_rx) -> crate_stats => {
                if let Ok(crate_stats) = crate_stats {
                    crate_count += 1;
                    screen.set_computation_status(crate_count);
                    if crate_stats.has_details() {
                        screen.add_dep(crate_stats);
                    }
                } else {
                    // This happens on computation end (channel closed).
                    // We don't break because we let the user read the result.
                }
            }
            recv(rx_user) -> user_event => {
                if let Ok(user_event) = user_event {
                    let quit = match user_event {
                        Event::Key(KeyEvent::Ctrl('q')) => true,
                        _ => {
                            screen.apply_event(&user_event);
                            false
                        }
                    };
                    event_source.unblock(quit); // this will lead to channel closing
                } else {
                    // The channel has been closed, which means the event source
                    // has properly released its resources, we may quit.
                    break;
                }
            }
        }
    }

    cursor.show()?; // if we don't do this, the poor terminal is cursorless
    Ok(CommandExitStatus::Successs)
}
