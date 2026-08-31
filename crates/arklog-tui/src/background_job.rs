use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

pub struct BackgroundJob<T> {
    receiver: Option<Receiver<Result<T, String>>>,
}

impl<T: Send + 'static> BackgroundJob<T> {
    pub fn new() -> Self {
        Self { receiver: None }
    }

    pub fn is_running(&self) -> bool {
        self.receiver.is_some()
    }

    pub fn start<F>(&mut self, task: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<T, String> + Send + 'static,
    {
        if self.is_running() {
            return Err("Background job is already running".to_string());
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let _ = sender.send(task());
        });
        self.receiver = Some(receiver);
        Ok(())
    }

    pub fn poll(&mut self) -> Option<Result<T, String>> {
        let receiver = self.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                Some(Err("Background job disconnected".to_string()))
            }
        }
    }
}
