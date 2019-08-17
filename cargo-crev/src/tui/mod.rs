mod verify_screen;

use crossterm::{
    AlternateScreen,
    KeyEvent,
    TerminalCursor,
};
use crate::prelude::*;
use crate::opts::{
    Verify,
};
use crate::shared::CommandExitStatus;
pub use crate::dep::{
    Progress, TableComputationStatus, DepComputationStatus, CrateCounts, TrustCount,
    Dep, ComputedDep,
    DepComputer,
};
use termimad::{
    Event,
    EventSource,
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

    screen.update();
    let computer = DepComputer::new(&args)?;
    let rx_comp = computer.run_computation(); // computation starts here on other threads
    let event_source = EventSource::new();
    let rx_user = event_source.receiver();

    loop {
        screen.update();
        select! {
            recv(rx_comp) -> comp_event => {
                if let Ok(comp_event) = comp_event {
                    screen.set_computation_status(comp_event.computation_status);
                    if let Some(dep) = comp_event.finished_dep {
                        screen.add_dep(dep);
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

