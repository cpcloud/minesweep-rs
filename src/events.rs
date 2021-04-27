use std::{io, sync::mpsc, thread, time::Duration};
use termion::{event::Key, input::TermRead};

pub(crate) enum Event<I> {
    Input(I),
    Tick,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub(crate) struct Events {
    rx: mpsc::Receiver<Event<Key>>,
    _input_handle: thread::JoinHandle<()>,
    _tick_handle: thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Config {
    pub(crate) exit_key: Key,
    pub(crate) tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Char('q'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Events {
    pub(crate) fn new() -> Self {
        Self::with_config(Config::default())
    }

    pub(crate) fn with_config(config: Config) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            rx,
            _input_handle: {
                let tx = tx.clone();
                thread::spawn(move || {
                    let stdin = io::stdin();
                    for key in stdin.keys().flatten() {
                        if let Err(err) = tx.send(Event::Input(key)) {
                            eprintln!("{}", err);
                            return;
                        }
                    }
                })
            },
            _tick_handle: {
                thread::spawn(move || loop {
                    if tx.send(Event::Tick).is_err() {
                        break;
                    }
                    thread::sleep(config.tick_rate);
                })
            },
        }
    }

    pub(crate) fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}
