use super::commands::{UciInput, UciOutput};
use super::decoder::Decoder;
use super::encoder::Encoder;
use log::debug;
use std::error::Error;
use std::io::{self, BufRead};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub struct UciConnection {
    output_tx: Sender<UciOutput>,
}

impl UciConnection {
    pub fn new() -> Self {
        // Create a channel for UCI outputs
        let (output_tx, output_rx) = channel();

        // Spawn a thread to handle output printing
        Self::spawn_output_handler(output_rx);

        Self { output_tx }
    }

    // Takes a callback function that handles commands and returns a sender for responses
    pub fn listen<F>(&mut self, mut callback: F) -> io::Result<()>
    where
        F: FnMut(&UciInput, Sender<UciOutput>) -> Result<(), Box<dyn Error>>,
    {
        let decoder = Decoder {};
        let stdin = io::stdin();
        let mut reader = stdin.lock();

        loop {
            let mut in_line = String::new();
            reader.read_line(&mut in_line)?;

            let in_line = in_line.trim();
            debug!("Input: {:?}", in_line);

            let input = decoder.decode(&in_line);

            // Handle potential errors from callback
            if let Err(e) = callback(&input, self.output_tx.clone()) {
                debug!("Callback error: {:?}", e);
            }

            if matches!(input, UciInput::Quit) {
                break;
            }
        }

        Ok(())
    }

    fn spawn_output_handler(output_rx: Receiver<UciOutput>) {
        thread::spawn(move || {
            let encoder = Encoder {};

            while let Ok(output) = output_rx.recv() {
                let out_line = encoder.encode(&output);
                debug!("Output: {:?}", out_line);
                println!("{}", out_line);
            }
        });
    }
}
