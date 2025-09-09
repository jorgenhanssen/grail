use chess::ChessMove;
use std::{
    error::Error,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    str::FromStr,
    thread,
    time::Duration,
};

pub struct EngineProcess {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl EngineProcess {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().ok_or(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "Failed to open stdin",
        ))?;
        let stdout = child.stdout.take().ok_or(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "Failed to open stdout",
        ))?;

        Ok(Self {
            child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
        })
    }

    fn send_command(&mut self, command: &str) -> io::Result<()> {
        self.stdin.write_all(command.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }

    fn read_line(&mut self) -> io::Result<String> {
        let mut line = String::new();
        self.stdout.read_line(&mut line)?;
        Ok(line.trim().to_string())
    }

    pub fn best_move_infinite(
        &mut self,
        fen: &str,
        move_time: u64,
    ) -> Result<ChessMove, Box<dyn Error>> {
        self.send_command(&format!("position fen {}\n", fen))?;
        self.send_command(&format!("go movetime {}\n", move_time))?;

        self.wait_for_bestmove()
    }

    pub fn best_move_timed(
        &mut self,
        fen: &str,
        wtime: u64,
        btime: u64,
        increment: u64,
    ) -> Result<ChessMove, Box<dyn Error>> {
        self.send_command(&format!("position fen {}\n", fen))?;
        self.send_command(&format!(
            "go wtime {} btime {} winc {} binc {}\n",
            wtime, btime, increment, increment
        ))?;

        self.wait_for_bestmove()
    }

    pub fn new_game(&mut self) -> io::Result<()> {
        self.send_command("ucinewgame\n")?;
        Ok(())
    }

    fn wait_for_bestmove(&mut self) -> Result<ChessMove, Box<dyn Error>> {
        loop {
            let line = self.read_line()?;
            if line.starts_with("bestmove") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    match ChessMove::from_str(parts[1]) {
                        Ok(mv) => return Ok(mv),
                        Err(_) => return Err("Unable to parse move".into()),
                    }
                } else {
                    return Err("Invalid bestmove response".into());
                }
            }
        }
    }
}

impl Drop for EngineProcess {
    fn drop(&mut self) {
        // Attempt to send quit command
        if self.send_command("quit\n").is_ok() {
            // Give the engine time to shut down
            thread::sleep(Duration::from_millis(100));

            // Check if process has exited
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
        }

        // If quit failed or process still running, kill it
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
