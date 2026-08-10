use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

#[cfg(not(windows))]
use std::process::{Child, Command, Stdio};

use crossbeam_channel::{Sender, TrySendError, bounded};

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
        let (sender, receiver) = bounded(16);
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
    let Ok(mut tts) = tts::Tts::default() else {
        set_status(
            &status,
            "Windows speech synthesis is unavailable".to_owned(),
        );
        return;
    };
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
    while let Ok(command) = receiver.recv() {
        match command {
            SpeechCommand::Speak(text) => {
                if let Err(error) = tts.speak(text, true) {
                    set_status(&status, format!("Speech synthesis failed: {error}"));
                }
            }
            SpeechCommand::Quit => break,
        }
    }
    let _ = tts.stop();
}

#[cfg(not(windows))]
fn speech_worker(
    receiver: crossbeam_channel::Receiver<SpeechCommand>,
    status: Arc<Mutex<Option<String>>>,
    locale: String,
) {
    let program = speech_program();
    if program.is_none() {
        set_status(
            &status,
            "No speech synthesizer was found (install espeak-ng or espeak)".to_owned(),
        );
    }
    let mut child: Option<Child> = None;
    while let Ok(command) = receiver.recv() {
        match command {
            SpeechCommand::Speak(text) => {
                stop_child(&mut child);
                let Some(program) = program.as_deref() else {
                    continue;
                };
                let mut command = Command::new(program);
                command.stdout(Stdio::null()).stderr(Stdio::null());
                #[cfg(target_os = "macos")]
                command.args(["-v", &locale, &text]);
                #[cfg(not(target_os = "macos"))]
                command.args(["-v", &locale, &text]);
                match command.spawn() {
                    Ok(spawned) => child = Some(spawned),
                    Err(error) => set_status(&status, format!("Speech synthesis failed: {error}")),
                }
            }
            SpeechCommand::Quit => break,
        }
    }
    stop_child(&mut child);
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
fn stop_child(child: &mut Option<Child>) {
    if let Some(mut running) = child.take() {
        let _ = running.kill();
        let _ = running.wait();
    }
}
