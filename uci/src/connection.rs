use super::commands::UciOutput;
use super::encoder::Encoder;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Handles UCI output communication.
///
/// Spawns a dedicated thread for printing UCI responses to stdout.
/// The main thread and engine worker can send output via the channel.
pub struct UciConnection {
    output_tx: Sender<UciOutput>,
}

impl Default for UciConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl UciConnection {
    pub fn new() -> Self {
        let (output_tx, output_rx) = channel();
        Self::spawn_output_handler(output_rx);
        Self { output_tx }
    }

    /// Returns a sender for UCI output messages.
    /// Can be cloned and shared with the engine worker thread.
    pub fn output_sender(&self) -> Sender<UciOutput> {
        self.output_tx.clone()
    }

    fn spawn_output_handler(output_rx: Receiver<UciOutput>) {
        thread::spawn(move || {
            let encoder = Encoder {};

            while let Ok(output) = output_rx.recv() {
                println!("{}", encoder.encode(&output));
            }
        });
    }
}
