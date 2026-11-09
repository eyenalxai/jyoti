use std::sync::mpsc::{self, Sender};
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

#[derive(Clone, Debug)]
pub enum WorkerEvent {
    State {
        id: String,
        state: BulbState,
    },
    Sent {
        id: String,
    },
    Error {
        id: String,
        error: MiioError,
    },
    Restored {
        id: String,
        generation: u64,
        ok: bool,
    },
    Reachable {
        id: String,
        reachable: bool,
    },
}

pub fn spawn(
    bulb: Bulb,
    keyring: KeyringStore,
    config: AppConfig,
    events: Sender<WorkerEvent>,
    waker: Waker,
) -> Sender<Command> {
    let (commands, inbox) = mpsc::channel::<Command>();
    let id = bulb.id.clone();
    let error_id = id.clone();
    let error_events = events.clone();

    let spawn_result = thread::Builder::new()
        .name(format!("jyoti-bulb-{}", bulb.id))
        .spawn(move || {
            let token = match keyring.get(&bulb.id) {
                Ok(Some(token)) if is_token(&token) => token,
                Ok(_) => {
                    let _ = events.send(WorkerEvent::Error {
                        id: id.clone(),
                        error: MiioError::Token {
                            address: bulb.address.clone(),
                        },
                    });
                    waker();
                    return;
                }
                Err(error) => {
                    let _ = events.send(WorkerEvent::Error {
                        id: id.clone(),
                        error: MiioError::Network {
                            address: bulb.address.clone(),
                            message: error.to_string(),
                        },
                    });
                    waker();
                    return;
                }
            };
            let mut session = match MiioSession::open(
                &bulb.address,
                &token,
                config.miio_port,
                config.miio_timeout,
            ) {
                Ok(session) => session,
                Err(error) => {
                    let _ = events.send(WorkerEvent::Error {
                        id: id.clone(),
                        error,
                    });
                    waker();
                    return;
                }
            };

            match session.get_state() {
                Ok(state) => {
                    let _ = events.send(WorkerEvent::State {
                        id: id.clone(),
                        state,
                    });
                }
                Err(error) => {
                    let _ = events.send(WorkerEvent::Error {
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
                        Ok(()) => sent(&events, &id),
                        Err(error) => fail(&mut session, &events, &id, error),
                    },
                    Command::SetBrightness(value) => match session.set_brightness(value) {
                        Ok(()) => sent(&events, &id),
                        Err(error) => fail(&mut session, &events, &id, error),
                    },
                    Command::SetTemperature(value) => match session.set_temperature(value) {
                        Ok(()) => sent(&events, &id),
                        Err(error) => fail(&mut session, &events, &id, error),
                    },
                    Command::SetColor(color) => match session.set_color(color) {
                        Ok(()) => sent(&events, &id),
                        Err(error) => fail(&mut session, &events, &id, error),
                    },
                    Command::Restore {
                        generation,
                        color,
                        temperature,
                        brightness,
                        deadline,
                    } => {
                        let empty = Vec::new();
                        let params =
                            restore_params(color, temperature, brightness).unwrap_or(empty);
                        let ok = params.is_empty()
                            || session.confirm_properties_until(params, deadline).is_ok();
                        let _ = events.send(WorkerEvent::Restored {
                            id: id.clone(),
                            generation,
                            ok,
                        });
                    }
                    Command::Probe => {
                        let ok = session.hello().is_ok();
                        let _ = events.send(WorkerEvent::Reachable {
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
        });

    if spawn_result.is_err() {
        let _ = error_events.send(WorkerEvent::Error {
            id: error_id,
            error: MiioError::Network {
                address: "this bulb".to_owned(),
                message: "Could not start the bulb worker.".to_owned(),
            },
        });
    }

    commands
}

fn sent(events: &Sender<WorkerEvent>, id: &str) {
    let _ = events.send(WorkerEvent::Sent { id: id.to_owned() });
}

fn fail(session: &mut MiioSession, events: &Sender<WorkerEvent>, id: &str, error: MiioError) {
    let _ = events.send(WorkerEvent::Error {
        id: id.to_owned(),
        error,
    });
    if let Ok(state) = session.get_state() {
        let _ = events.send(WorkerEvent::State {
            id: id.to_owned(),
            state,
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
