use std::error::Error;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use chess::ChessMove;
use std::str::FromStr;

pub struct EngineProcess {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl EngineProcess {
    pub fn new(path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let mut child = Command::new(path)
            .arg("negamax")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = BufWriter::new(child.stdin.take().expect("Failed to open stdin"));
        let stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));

        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    #[inline]
    fn send_command(&mut self, command: &str) {
        self.stdin
            .write_all(command.as_bytes())
            .expect("Failed to write to stdin");
        self.stdin.flush().expect("Failed to flush stdin");
    }

    #[inline]
    fn read_line(&mut self) -> String {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("Failed to read line");

        line.trim().to_string()
    }

    pub fn best_move(&mut self, fen: &str, time: u64) -> ChessMove {
        self.send_command(&format!("position fen {}\n", fen));
        self.send_command(&format!("go movetime {}\n", time));

        let mut line = String::new();
        while !line.contains("bestmove") {
            line = self.read_line();
        }

        let txt = line.split("bestmove ").nth(1).unwrap().to_string();
        ChessMove::from_str(&txt).unwrap()
    }
}

impl Drop for EngineProcess {
    fn drop(&mut self) {
        // Send quit command to gracefully shutdown the engine
        if let Err(_) = self.stdin.write_all(b"quit\n") {
            // If we can't send quit, force kill the process
            let _ = self.child.kill();
        } else {
            let _ = self.stdin.flush();
            // Give the engine a moment to quit gracefully
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Check if the process has exited
            match self.child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited
                }
                Ok(None) => {
                    // Process is still running, force kill it
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                }
                Err(_) => {
                    // Error checking status, try to kill it
                    let _ = self.child.kill();
                }
            }
        }
    }
}
