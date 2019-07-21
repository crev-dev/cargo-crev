use std::thread;
use std::time::{Instant, Duration};
use crossbeam::channel::{Sender, Receiver, unbounded};
use crossterm::{InputEvent, KeyEvent, MouseEvent, TerminalInput};

const DOUBLE_CLICK_MAX_DURATION: Duration = Duration::from_millis(700);

/// a valid user event
#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Click(u16, u16),
    DoubleClick(u16, u16),
}

impl Event {
    pub fn from_crossterm_event(crossterm_event: Option<InputEvent>) -> Option<Event> {
        match crossterm_event {
            Some(InputEvent::Keyboard(key)) => Some(Event::Key(key)),
            Some(InputEvent::Mouse(MouseEvent::Release(x, y))) => Some(Event::Click(x, y)),
            _ => None,
        }
    }
}

/// an event with time of occuring
struct TimedEvent {
    time: Instant,
    event: Event,
}
impl From<Event> for TimedEvent {
    fn from(event: Event) -> Self {
        TimedEvent {
            time: Instant::now(),
            event,
        }
    }
}

/// a thread backed event listener.
pub struct EventSource {
    rx_events: Receiver<Event>,
    tx_quit: Sender<bool>,
}

impl EventSource {
    /// create a new source
    pub fn new() -> EventSource {
        let (tx_events, rx_events) = unbounded();
        let (tx_quit, rx_quit) = unbounded();
        thread::spawn(move || {
            let input = TerminalInput::new();
            let mut last_event: Option<TimedEvent> = None;
            if let Err(e) = input.enable_mouse_mode() {
                eprintln!("WARN Error while enabling mouse. {:?}", e);
            }
            let mut crossterm_events = input.read_sync();
            loop {
                let crossterm_event = crossterm_events.next();
                if let Some(mut event) = Event::from_crossterm_event(crossterm_event) {
                    // save the event, and maybe change it
                    // (may change a click into a double-click)
                    if let Event::Click(x, y) = event {
                        if let Some(TimedEvent{time, event:Event::Click(_, last_y)}) = last_event {
                            if last_y == y && time.elapsed() < DOUBLE_CLICK_MAX_DURATION {
                                event = Event::DoubleClick(x, y);
                            }
                        }
                    }
                    last_event = Some(TimedEvent::from(event.clone()));
                    // we send the even to the receiver in the main event loop
                    tx_events.send(event).unwrap();
                    let quit = rx_quit.recv().unwrap();
                    if quit {
                        // Cleanly quitting this thread is necessary
                        //  to ensure stdin is properly closed when
                        //  we launch an external application in the same
                        //  terminal
                        // Disabling mouse mode is also necessary to let the
                        //  terminal in a proper state.
                        input.disable_mouse_mode().unwrap();
                        return;
                    }
                }
            }
        });
        EventSource {
            rx_events,
            tx_quit,
        }
    }

    /// either start listening again, or quit, depending on the passed bool.
    /// It's mandatory to call this with quit=true at end for a proper ending
    /// of the thread (and its resources)
    pub fn unblock(&self, quit: bool) {
        self.tx_quit.send(quit).unwrap();
    }

    pub fn receiver(&self) -> Receiver<Event> {
        self.rx_events.clone()
    }
}

