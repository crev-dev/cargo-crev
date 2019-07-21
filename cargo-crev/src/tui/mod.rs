mod user_events;
mod verify_screen;
mod table_view;

use crossterm::{
    AlternateScreen,
    TerminalCursor,
    KeyEvent,
};
use crate::prelude::*;
use crate::opts::{
    Verify,
};
use crate::shared::CommandExitStatus;
use crate::tui::user_events::*;
pub use crate::dep::{
    Progress, TableComputationStatus, DepComputationStatus, CrateCounts, TrustCount,
    Dep, ComputedDep, DepTable,
    DepComputer,
};
use verify_screen::{
    VerifyScreen,
};

/// called in case of an --interactive execution
///
/// Right now the --interactive is only possible for "verify" subcommand
///  but this will hopefully change and the public function here would
///  be a run_command
pub fn verify_deps(args: Verify) -> Result<CommandExitStatus> {
    let _alt_screen = AlternateScreen::to_alternate(true);
    let cursor = TerminalCursor::new();
    cursor.hide()?;

    let mut screen = VerifyScreen::new()?;

    let mut table = DepTable::new();
    screen.update_for(&table);

    let computer = DepComputer::new(&args)?;
    let rx_comp = computer.run_computation(); // computation starts here on other threads
    let event_source = EventSource::new();
    let rx_user = event_source.receiver();

    loop {
        select! {
            recv(rx_comp) -> comp_event => {
                if let Ok(comp_event) = comp_event {
                    table.update(comp_event);
                    screen.update_for(&table);
                } else {
                    // This happens on computation end (channel closed).
                    // We don't break because we let the user read the result.
                }
            }
            recv(rx_user) -> user_event => {
                if let Ok(user_event) = user_event {
                    let mut quit = false;
                    match user_event {
                        Event::Key(KeyEvent::Ctrl('q')) => {
                            quit = true;
                        }
                        Event::Key(KeyEvent::PageUp) => {
                            screen.try_scroll_pages(-1);
                        }
                        Event::Key(KeyEvent::PageDown) => {
                            screen.try_scroll_pages(1);
                        }
                        _ => {}
                    }
                    event_source.unblock(quit); // this will lead to channel closing
                    if !quit {
                        screen.update_for(&table);
                    }
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

