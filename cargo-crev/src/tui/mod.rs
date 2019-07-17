mod user_events;

use std::thread;
use crossbeam_channel::{Sender, Receiver, unbounded, RecvError};
use crossterm::{
    AlternateScreen,
    TerminalCursor,
    TerminalInput,
    KeyEvent,
    Color::*
};
use rayon::prelude::*;

use crate::prelude::*;
use crate::opts::{
    Verify,
};
use crate::shared::CommandExitStatus;
use crate::tui::user_events::*;
use crate::repo::Repo;
pub use crate::dep::{
    CrateCounts, TrustCount, Progress,
    RowComputationStatus, TableComputationStatus,
    DepTable, DepRow,
};



///
fn is_quit_event(event: &Event) -> bool {
    match event {
        Event::Key(KeyEvent::Ctrl('q')) => true,
        _ => false,
    }
}


/// called in case of an --interactive execution
///
/// Right now the --interactive is only possible for "verify" subcommand
///  but this will hopefully change and the public function here would
///  be a run_command
pub fn verify_deps(args: Verify) -> Result<CommandExitStatus> {
    let _alt_screen = AlternateScreen::to_alternate(true);
    let cursor = TerminalCursor::new();
    cursor.hide()?;


    let repo = Repo::auto_open_cwd()?;
    let package_set = repo.non_local_dep_crates()?;
    let mut table = DepTable::new(&package_set)?;

    let mut progress = Progress {
        done: 0,
        total: table.rows.len(),
    };
    table.computation_status = TableComputationStatus::ComputingGeiger {
        progress,
    };

    let (tx_geiger, rx_geiger) = unbounded();

    thread::spawn(move || {
        println!("processing...");
        let cursor = TerminalCursor::new();
        loop {
            let b = rx_geiger.recv().unwrap();
            progress.done += 1;
            cursor.goto(2, 2).unwrap();
            println!("progres: {:?}", &progress);
            if progress.is_complete() {
                break;
            }
        }
    });

    table.rows
        .par_iter_mut()
        .for_each(|row| {
            row.download_if_needed().unwrap();
            row.count_geiger();
            tx_geiger.send(true).unwrap();
        });

    println!("Hit ctrl-Q to quit");

    let event_source = EventSource::new();
    let mut quit = false;
    loop {
        let event = match event_source.recv() {
            Ok(event) => event,
            Err(_) => {
                // this is how we quit the application,
                // when the input thread is properly closed
                break;
            }
        };

        quit = is_quit_event(&event);
        // handle event & display here
        event_source.unblock(quit);
    }

    cursor.show()?;
    Ok(CommandExitStatus::Successs)
}

