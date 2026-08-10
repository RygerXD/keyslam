use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

#[cfg(not(windows))]
use std::process::{Child, Command, Stdio};

use crossbeam_channel::{RecvTimeoutError, Sender, TrySendError, bounded};

const MAX_CONCURRENT_SPEECH: usize = 8;
const MAX_PENDING_SPEECH: usize = 16;
const SPEECH_POLL_INTERVAL: Duration = Duration::from_millis(10);

enum SpeechCommand {
    Speak(String),
    Quit,
}

pub struct SpeechSystem {
    sender: Sender<SpeechCommand>,
    worker: Option<JoinHandle<()>>,
    status: Arc<Mutex<Option<String>>>,
}

impl SpeechSystem {
    pub fn new(locale: &str) -> Self {
        let (sender, receiver) = bounded(MAX_PENDING_SPEECH);
        let status = Arc::new(Mutex::new(None));
        let worker_status = Arc::clone(&status);
        let locale = locale.to_owned();
        let worker = thread::Builder::new()
            .name("babysmash-speech".to_owned())
            .spawn(move || speech_worker(receiver, worker_status, locale))
            .ok();
        if worker.is_none() {
            set_status(&status, "Could not start the speech worker".to_owned());
        }
        Self {
            sender,
            worker,
            status,
        }
    }

    pub fn speak(&self, text: impl Into<String>) {
        match self.sender.try_send(SpeechCommand::Speak(text.into())) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => {
                set_status(
                    &self.status,
                    "Speech queue is busy; a response was skipped".to_owned(),
                );
            }
        }
    }

    pub fn status(&self) -> Option<String> {
        self.status.lock().ok().and_then(|status| status.clone())
    }
}

impl Drop for SpeechSystem {
    fn drop(&mut self) {
        let _ = self.sender.send(SpeechCommand::Quit);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn set_status(status: &Arc<Mutex<Option<String>>>, message: String) {
    if let Ok(mut current) = status.lock() {
        *current = Some(message);
    }
}

#[cfg(windows)]
fn speech_worker(
    receiver: crossbeam_channel::Receiver<SpeechCommand>,
    status: Arc<Mutex<Option<String>>>,
    locale: String,
) {
    let Ok(first_voice) = configured_tts(&locale) else {
        set_status(
            &status,
            "Windows speech synthesis is unavailable".to_owned(),
        );
        return;
    };
    let mut voices = vec![first_voice];
    let mut pending = VecDeque::new();
    let mut voice_creation_failed = false;

    loop {
        while !pending.is_empty() {
            let mut available = voices
                .iter()
                .position(|voice| voice.is_speaking().is_ok_and(|speaking| !speaking));
            if available.is_none() && voices.len() < MAX_CONCURRENT_SPEECH && !voice_creation_failed
            {
                match configured_tts(&locale) {
                    Ok(voice) => {
                        voices.push(voice);
                        available = Some(voices.len() - 1);
                    }
                    Err(error) => {
                        voice_creation_failed = true;
                        set_status(
                            &status,
                            format!("Could not add another simultaneous voice: {error}"),
                        );
                    }
                }
            }
            let Some(index) = available else {
                break;
            };
            let Some(text) = pending.pop_front() else {
                break;
            };
            if let Err(error) = voices[index].speak(text, false) {
                set_status(&status, format!("Speech synthesis failed: {error}"));
            }
        }

        let command = if pending.is_empty() {
            match receiver.recv() {
                Ok(command) => Some(command),
                Err(_) => break,
            }
        } else {
            match receiver.recv_timeout(SPEECH_POLL_INTERVAL) {
                Ok(command) => Some(command),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        };
        let Some(command) = command else {
            continue;
        };
        match command {
            SpeechCommand::Speak(text) if pending.len() < MAX_PENDING_SPEECH => {
                pending.push_back(text);
            }
            SpeechCommand::Speak(_) => {
                set_status(&status, "Speech is busy; a response was skipped".to_owned())
            }
            SpeechCommand::Quit => break,
        }
    }

    for voice in &mut voices {
        let _ = voice.stop();
    }
}

#[cfg(windows)]
fn configured_tts(locale: &str) -> Result<tts::Tts, tts::Error> {
    let mut tts = tts::Tts::default()?;
    let locale = locale.to_ascii_lowercase();
    let language = locale.split('-').next().unwrap_or(&locale);
    if let Ok(voices) = tts.voices()
        && let Some(voice) = voices.iter().find(|voice| {
            let voice_locale = voice.language().to_string().to_ascii_lowercase();
            voice_locale == locale || voice_locale.split('-').next() == Some(language)
        })
    {
        let _ = tts.set_voice(voice);
    }
    Ok(tts)
}

#[cfg(not(windows))]
fn speech_worker(
    receiver: crossbeam_channel::Receiver<SpeechCommand>,
    status: Arc<Mutex<Option<String>>>,
    locale: String,
) {
    let Some(program) = speech_program() else {
        set_status(
            &status,
            "No speech synthesizer was found (install espeak-ng or espeak)".to_owned(),
        );
        return;
    };
    let mut children: Vec<Child> = Vec::new();
    let mut pending = VecDeque::new();

    loop {
        children.retain_mut(|child| match child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(error) => {
                set_status(&status, format!("Speech process failed: {error}"));
                false
            }
        });
        while children.len() < MAX_CONCURRENT_SPEECH {
            let Some(text) = pending.pop_front() else {
                break;
            };
            let mut command = Command::new(&program);
            command
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .args(["-v", &locale, &text]);
            match command.spawn() {
                Ok(spawned) => children.push(spawned),
                Err(error) => {
                    set_status(&status, format!("Speech synthesis failed: {error}"));
                }
            }
        }

        let command = if pending.is_empty() {
            match receiver.recv() {
                Ok(command) => Some(command),
                Err(_) => break,
            }
        } else {
            match receiver.recv_timeout(SPEECH_POLL_INTERVAL) {
                Ok(command) => Some(command),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        };
        let Some(command) = command else {
            continue;
        };
        match command {
            SpeechCommand::Speak(text) if pending.len() < MAX_PENDING_SPEECH => {
                pending.push_back(text);
            }
            SpeechCommand::Speak(_) => {
                set_status(&status, "Speech is busy; a response was skipped".to_owned())
            }
            SpeechCommand::Quit => break,
        }
    }
    stop_children(&mut children);
}

#[cfg(not(windows))]
fn speech_program() -> Option<String> {
    #[cfg(target_os = "macos")]
    return Some("say".to_owned());
    #[cfg(not(target_os = "macos"))]
    let candidates = ["espeak-ng", "espeak"];
    #[cfg(not(target_os = "macos"))]
    candidates
        .into_iter()
        .find(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        })
        .map(str::to_owned)
}

#[cfg(not(windows))]
fn stop_children(children: &mut Vec<Child>) {
    for mut running in children.drain(..) {
        let _ = running.kill();
        let _ = running.wait();
    }
}
