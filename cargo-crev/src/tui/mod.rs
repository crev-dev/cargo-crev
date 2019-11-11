mod verify_screen;

pub use crate::deps::{scan, CountWithTotal};
use crate::{
    deps,
    opts::{CrateSelector, CrateVerify},
    prelude::*,
    shared::CommandExitStatus,
};
use crossbeam::channel::select;
use crossterm::{AlternateScreen, KeyEvent, TerminalCursor};
use termimad::{Event, EventSource};
use verify_screen::VerifyScreen;

/// called in case of an --interactive execution
///
/// Right now the --interactive is only possible for "verify" subcommand
///  but this will hopefully change and the public function here would
///  be a run_command
pub fn verify_deps(crate_: CrateSelector, opts: CrateVerify) -> Result<CommandExitStatus> {
    let computer = scan::Scanner::new(crate_, &opts)?;

    let _alt_screen = AlternateScreen::to_alternate(true);
    let cursor = TerminalCursor::new();
    cursor.hide()?;

    let mut screen = VerifyScreen::new(computer.selected_crate_count(), opts.common.cargo_opts)?;

    screen.update();
    let crate_stats_rx = computer.run();
    let event_source = EventSource::new();
    let user_event_rx = event_source.receiver();
    let mut crate_count = 0;

    fn handle_crate_stats(
        screen: &mut VerifyScreen,
        crate_stats: deps::CrateStats,
        crate_count: &mut usize,
    ) {
        *crate_count += 1;
        screen.set_computation_status(*crate_count);
        screen.add_dep(crate_stats);
    }

    fn handle_user_action(
        screen: &mut VerifyScreen,
        user_event: termimad::Event,
        event_source: &EventSource,
    ) {
        let quit = match user_event {
            Event::Key(KeyEvent::Ctrl('q')) => true,
            _ => {
                screen.apply_event(&user_event);
                false
            }
        };
        event_source.unblock(quit); // this will lead to channel closing
    }

    loop {
        screen.update();
        select! {
            recv(crate_stats_rx) -> crate_stats => {
                if let Ok(crate_stats) = crate_stats {
                    handle_crate_stats(&mut screen, crate_stats, &mut crate_count);
                    // drain the channel completely, so do don't redraw screen (slow)
                    // over and over again causing irritating blinking and cpu waste
                    while let Ok(crate_stats) = crate_stats_rx.recv_timeout(std::time::Duration::from_micros(0)) {
                        handle_crate_stats(&mut screen, crate_stats, &mut crate_count);
                    }
                } else {
                    break;
                }
            }
            recv(user_event_rx) -> user_event => {
                if let  Ok(user_event) = user_event {
                    handle_user_action(&mut screen, user_event, &event_source);
                } else {
                    break;
                }
            }
        }
    }

    screen.update();
    for user_event in user_event_rx.into_iter() {
        handle_user_action(&mut screen, user_event, &event_source);
        screen.update();
    }

    cursor.show()?; // if we don't do this, the poor terminal is cursorless
    Ok(CommandExitStatus::Success)
}
