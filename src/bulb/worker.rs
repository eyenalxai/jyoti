use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use rune::Waker;
use serde_json::json;

use crate::bulb::keyring::KeyringStore;
use crate::bulb::model::{
    Bulb, BulbState, PIID_BRIGHTNESS, PIID_COLOR, PIID_TEMPERATURE, Rgb, is_token,
};
use crate::config::AppConfig;
use crate::error::MiioError;
use crate::miio::properties::{PropertyParam, properties_with_value};
use crate::miio::session::MiioSession;

static NEXT_WORKER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub enum Command {
    SetPower(bool),
    SetBrightness(u8),
    SetTemperature(u16),
    SetColor(Rgb),
    Restore {
        generation: u64,
        color: Option<Rgb>,
        temperature: Option<u16>,
        brightness: Option<u8>,
        deadline: Instant,
    },
    Probe,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerExit {
    Stopped,
    Failed,
}

#[derive(Clone, Debug)]
pub enum WorkerEvent {
    State {
        worker: u64,
        id: String,
        state: BulbState,
        observed_at: Instant,
    },
    Sent {
        worker: u64,
        id: String,
    },
    Error {
        worker: u64,
        id: String,
        error: MiioError,
    },
    Restored {
        worker: u64,
        id: String,
        generation: u64,
        ok: bool,
    },
    Reachable {
        worker: u64,
        id: String,
        reachable: bool,
    },
    Exited {
        worker: u64,
        id: String,
        exit: WorkerExit,
    },
}

impl WorkerEvent {
    pub fn id(&self) -> &str {
        match self {
            WorkerEvent::State { id, .. }
            | WorkerEvent::Sent { id, .. }
            | WorkerEvent::Error { id, .. }
            | WorkerEvent::Restored { id, .. }
            | WorkerEvent::Reachable { id, .. }
            | WorkerEvent::Exited { id, .. } => id,
        }
    }

    pub fn worker(&self) -> u64 {
        match self {
            WorkerEvent::State { worker, .. }
            | WorkerEvent::Sent { worker, .. }
            | WorkerEvent::Error { worker, .. }
            | WorkerEvent::Restored { worker, .. }
            | WorkerEvent::Reachable { worker, .. }
            | WorkerEvent::Exited { worker, .. } => *worker,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerClosed;

impl std::fmt::Display for WorkerClosed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("The bulb worker has stopped.")
    }
}

#[derive(Clone, Debug)]
pub struct WorkerHandle {
    worker: u64,
    commands: Sender<Command>,
}

impl WorkerHandle {
    pub fn send(&self, command: Command) -> Result<(), WorkerClosed> {
        self.commands.send(command).map_err(|_| WorkerClosed)
    }

    pub fn stop(&self) {
        let _ = self.commands.send(Command::Stop);
    }

    /// The capture thread sends directly to the worker; the app keeps the typed
    /// handle for every path that reports a failure.
    pub fn capture_sender(&self) -> Sender<Command> {
        self.commands.clone()
    }

    pub fn is_current(&self, worker: u64) -> bool {
        self.worker == worker
    }
}

#[cfg(test)]
pub(crate) fn test_handle(commands: Sender<Command>) -> WorkerHandle {
    WorkerHandle {
        worker: 0,
        commands,
    }
}

pub fn spawn(
    bulb: Bulb,
    keyring: KeyringStore,
    config: AppConfig,
    events: Sender<WorkerEvent>,
    waker: Waker,
) -> WorkerHandle {
    let (commands, inbox) = mpsc::channel::<Command>();
    let id = bulb.id.clone();
    let worker = NEXT_WORKER.fetch_add(1, Ordering::Relaxed);
    let handle = WorkerHandle { worker, commands };
    let error_id = id.clone();
    let error_events = events.clone();
    let exit_events = events.clone();
    let exit_id = id.clone();
    let exit_waker = Arc::clone(&waker);

    let spawn_result = thread::Builder::new()
        .name(format!("jyoti-bulb-{id}"))
        .spawn(move || {
            let mut notice = ExitNotice {
                events: exit_events,
                id: exit_id,
                worker,
                waker: exit_waker,
                sent: false,
            };
            let exit = run_worker(bulb, &keyring, &config, &events, waker, worker, inbox);
            notice.send(exit);
        });

    if spawn_result.is_err() {
        let _ = error_events.send(WorkerEvent::Error {
            worker,
            id: error_id,
            error: MiioError::Network {
                address: "this bulb".to_owned(),
                message: "Could not start the bulb worker.".to_owned(),
            },
        });
    }

    handle
}

fn run_worker(
    bulb: Bulb,
    keyring: &KeyringStore,
    config: &AppConfig,
    events: &Sender<WorkerEvent>,
    waker: Waker,
    worker: u64,
    inbox: Receiver<Command>,
) -> WorkerExit {
    let id = bulb.id;
    let token = match keyring.get(&id) {
        Ok(Some(token)) if is_token(&token) => token,
        Ok(_) => {
            let _ = events.send(WorkerEvent::Error {
                worker,
                id: id.clone(),
                error: MiioError::Token {
                    address: bulb.address.clone(),
                },
            });
            waker();
            return WorkerExit::Failed;
        }
        Err(error) => {
            let _ = events.send(WorkerEvent::Error {
                worker,
                id: id.clone(),
                error: MiioError::Network {
                    address: bulb.address.clone(),
                    message: error.to_string(),
                },
            });
            waker();
            return WorkerExit::Failed;
        }
    };
    let mut session =
        match MiioSession::open(&bulb.address, &token, config.miio_port, config.miio_timeout) {
            Ok(session) => session,
            Err(error) => {
                let _ = events.send(WorkerEvent::Error {
                    worker,
                    id: id.clone(),
                    error,
                });
                waker();
                return WorkerExit::Failed;
            }
        };

    let observed_at = Instant::now();
    match session.get_state() {
        Ok(state) => {
            let _ = events.send(WorkerEvent::State {
                worker,
                id: id.clone(),
                state,
                observed_at,
            });
        }
        Err(error) => {
            let _ = events.send(WorkerEvent::Error {
                worker,
                id: id.clone(),
                error,
            });
        }
    }
    waker();

    while let Ok(command) = inbox.recv() {
        let mut stop = false;
        match command {
            Command::SetPower(on) => match session.set_power(on) {
                Ok(()) => sent(events, worker, &id),
                Err(error) => fail(&mut session, events, worker, &id, error),
            },
            Command::SetBrightness(value) => match session.set_brightness(value) {
                Ok(()) => sent(events, worker, &id),
                Err(error) => fail(&mut session, events, worker, &id, error),
            },
            Command::SetTemperature(value) => match session.set_temperature(value) {
                Ok(()) => sent(events, worker, &id),
                Err(error) => fail(&mut session, events, worker, &id, error),
            },
            Command::SetColor(color) => match session.set_color(color) {
                Ok(()) => sent(events, worker, &id),
                Err(error) => fail(&mut session, events, worker, &id, error),
            },
            Command::Restore {
                generation,
                color,
                temperature,
                brightness,
                deadline,
            } => {
                let empty = Vec::new();
                let params = restore_params(color, temperature, brightness).unwrap_or(empty);
                let ok =
                    params.is_empty() || session.confirm_properties_until(params, deadline).is_ok();
                let _ = events.send(WorkerEvent::Restored {
                    worker,
                    id: id.clone(),
                    generation,
                    ok,
                });
            }
            Command::Probe => {
                let ok = session.hello().is_ok();
                let _ = events.send(WorkerEvent::Reachable {
                    worker,
                    id: id.clone(),
                    reachable: ok,
                });
            }
            Command::Stop => stop = true,
        }
        waker();
        if stop {
            break;
        }
    }
    WorkerExit::Stopped
}

struct ExitNotice {
    events: Sender<WorkerEvent>,
    id: String,
    worker: u64,
    waker: Waker,
    sent: bool,
}

impl ExitNotice {
    fn send(&mut self, exit: WorkerExit) {
        if self.sent {
            return;
        }
        self.sent = true;
        let _ = self.events.send(WorkerEvent::Exited {
            id: self.id.clone(),
            worker: self.worker,
            exit,
        });
        (self.waker)();
    }
}

impl Drop for ExitNotice {
    fn drop(&mut self) {
        self.send(WorkerExit::Failed);
    }
}

fn sent(events: &Sender<WorkerEvent>, worker: u64, id: &str) {
    let _ = events.send(WorkerEvent::Sent {
        worker,
        id: id.to_owned(),
    });
}

fn fail(
    session: &mut MiioSession,
    events: &Sender<WorkerEvent>,
    worker: u64,
    id: &str,
    error: MiioError,
) {
    let _ = events.send(WorkerEvent::Error {
        worker,
        id: id.to_owned(),
        error,
    });
    let observed_at = Instant::now();
    if let Ok(state) = session.get_state() {
        let _ = events.send(WorkerEvent::State {
            worker,
            id: id.to_owned(),
            state,
            observed_at,
        });
    }
}

fn restore_params(
    color: Option<Rgb>,
    temperature: Option<u16>,
    brightness: Option<u8>,
) -> Option<Vec<PropertyParam>> {
    let mut params = Vec::new();
    if let Some(color) = color {
        params.extend(properties_with_value(PIID_COLOR, json!(color.pack())));
    } else if let Some(temperature) = temperature {
        params.extend(properties_with_value(PIID_TEMPERATURE, json!(temperature)));
    }
    if let Some(brightness) = brightness {
        params.extend(properties_with_value(PIID_BRIGHTNESS, json!(brightness)));
    }
    (!params.is_empty()).then_some(params)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::mpsc;

    use super::{Command, ExitNotice, WorkerClosed, WorkerEvent, WorkerExit, WorkerHandle};

    #[test]
    fn a_send_after_the_worker_stops_returns_an_error() {
        let (commands, inbox) = mpsc::channel();
        let handle = WorkerHandle {
            worker: 1,
            commands,
        };
        drop(inbox);

        assert_eq!(handle.send(Command::Probe), Err(WorkerClosed));
    }

    #[test]
    fn a_replaced_worker_is_not_the_current_one() {
        let (commands, _inbox) = mpsc::channel();
        let handle = WorkerHandle {
            worker: 7,
            commands,
        };

        assert!(handle.is_current(7));
        assert!(!handle.is_current(8));
    }

    #[test]
    fn a_worker_exit_is_reported_once() {
        let (events, inbox) = mpsc::channel();
        let mut notice = ExitNotice {
            events,
            id: "desk".to_owned(),
            worker: 3,
            waker: Arc::new(|| {}),
            sent: false,
        };

        notice.send(WorkerExit::Failed);
        notice.send(WorkerExit::Failed);
        drop(notice);

        let reported: Vec<WorkerEvent> = inbox.try_iter().collect();
        assert_eq!(reported.len(), 1);
        match reported.as_slice() {
            [WorkerEvent::Exited { id, worker, exit }] => {
                assert_eq!(id, "desk");
                assert_eq!(*worker, 3);
                assert_eq!(*exit, WorkerExit::Failed);
            }
            other => assert!(other.is_empty(), "expected one exit, got {other:?}"),
        }
    }
}
