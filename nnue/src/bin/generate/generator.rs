use candle_core::Device;
use candle_nn::VarMap;
use chess::{Board, ChessMove, Game, MoveGen};
use evaluation::{hce, NNUE};
use indicatif::{ProgressBar, ProgressStyle};
use nnue::version::VersionManager;
use rand::Rng;
use search::{Engine, EngineConfig, NegamaxEngine};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use uci::commands::GoParams;

pub struct Generator {
    threads: usize,
    nnue_path: Option<PathBuf>,
    version: u32,
    opening_book: (PathBuf, usize), // (path, line_count)
}

impl Generator {
    pub fn new(
        threads: usize,
        manager: &VersionManager,
        opening_book_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let version = manager.get_latest_version()?;

        let line_count = count_opening_book_lines(&opening_book_path)?;
        log::info!(
            "Loaded opening book with {} positions from {}",
            line_count,
            opening_book_path
        );
        let opening_book = (PathBuf::from(opening_book_path), line_count);

        let generator = match version {
            Some(version) => Self {
                threads,
                nnue_path: Some(manager.file_path(version, "model.safetensors")),
                version,
                opening_book,
            },
            _ => Self {
                threads,
                nnue_path: None,
                version: 0,
                opening_book,
            },
        };

        Ok(generator)
    }

    pub fn run(&self, duration: u64, depth: u8) -> Vec<(String, i16, f32, usize)> {
        let eval_name = match &self.nnue_path {
            Some(path) => path.display().to_string(),
            None => "traditional evaluator".to_string(),
        };

        log::info!(
            "Generating samples using {} threads ({})",
            self.threads,
            eval_name,
        );

        let pb = ProgressBar::new(duration);
        pb.set_style(
            ProgressStyle::with_template(
                " {spinner:.cyan} {wide_bar:.cyan/blue} {eta_precise} | {msg}",
            )
            .unwrap(),
        );
        let pb = Arc::new(pb);

        let sample_counter = Arc::new(AtomicUsize::new(0));
        let game_id_counter = Arc::new(AtomicUsize::new(0));
        let opening_book = Arc::new(self.opening_book.clone());

        let handles: Vec<_> = (0..self.threads)
            .map(|tid| {
                let nnue_path = self.nnue_path.clone();
                let version = self.version;
                let sample_counter = Arc::clone(&sample_counter);
                let game_id_counter = Arc::clone(&game_id_counter);
                let opening_book = Arc::clone(&opening_book);
                let pb = Arc::clone(&pb);

                std::thread::spawn(move || {
                    let nnue: Option<Box<dyn NNUE>> = if let Some(path) = nnue_path {
                        let mut varmap = VarMap::new();
                        let mut nnue = nnue::Evaluator::new(&varmap, &Device::Cpu, version);

                        varmap.load(path).unwrap();
                        nnue.enable_nnue();

                        Some(Box::new(nnue))
                    } else {
                        None
                    };

                    let mut worker = SelfPlayWorker::new(
                        tid,
                        sample_counter,
                        game_id_counter,
                        depth,
                        nnue,
                        &opening_book,
                    );
                    worker.play_games(duration, &pb)
                })
            })
            .collect();

        let evaluations: Vec<_> = handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect();

        pb.finish_with_message(format!("Generated {} samples", evaluations.len()));

        evaluations
    }
}

/// Count lines in opening book EPD file
fn count_opening_book_lines(path: &str) -> Result<usize, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut count = 0;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        count += 1;
    }

    Ok(count)
}

/// Read a random position from the opening book
fn read_random_opening_position(
    path: &PathBuf,
    line_count: usize,
) -> Result<String, Box<dyn Error>> {
    let mut rng = rand::thread_rng();
    let target_line = rng.gen_range(0..line_count);

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut current_line = 0;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if current_line == target_line {
            // EPD format is FEN fields (6 fields) followed by optional operations
            // We extract just the FEN part
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                // Construct FEN from first 4 fields (board, side, castling, ep)
                // For EPD we set halfmove and fullmove to 0 and 1
                let fen = format!("{} {} {} {} 0 1", parts[0], parts[1], parts[2], parts[3]);
                return Ok(fen);
            }
        }
        current_line += 1;
    }

    Err("Failed to read opening position".into())
}

struct SelfPlayWorker {
    tid: usize,
    sample_counter: Arc<AtomicUsize>,
    game_id_counter: Arc<AtomicUsize>,
    engine: NegamaxEngine,
    depth: u8,
    opening_book: (PathBuf, usize), // (path, line_count)

    // Game-specific state
    game: Game,
    game_id: usize,
    position_counts: std::collections::HashMap<u64, usize>,
    current_game_positions: Vec<(String, i16)>,
}

impl SelfPlayWorker {
    pub fn new(
        tid: usize,
        sample_counter: Arc<AtomicUsize>,
        game_id_counter: Arc<AtomicUsize>,
        depth: u8,
        nnue: Option<Box<dyn NNUE>>,
        opening_book: &(PathBuf, usize),
    ) -> Self {
        let mut config = EngineConfig::default();
        // Reduce hash size for data generation (384 MB instead of 1024 MB)
        // With 32 threads, this reduces RAM from 32GB to ~12GB
        config.hash_size.value = 384;

        let hce = Box::new(hce::Evaluator::new(
            config.get_piece_values(),
            config.get_hce_config(),
        ));

        Self {
            tid,
            sample_counter,
            game_id_counter,
            game: Game::new(),
            game_id: 0,
            depth,
            engine: NegamaxEngine::new(&config, hce, nnue),
            position_counts: std::collections::HashMap::new(),
            current_game_positions: Vec::new(),
            opening_book: opening_book.clone(),
        }
    }

    pub fn play_games(
        &mut self,
        duration: u64,
        pb: &ProgressBar,
    ) -> Vec<(String, i16, f32, usize)> {
        let start_time = Instant::now();
        let mut evaluations = Vec::new();

        self.reset_game();

        loop {
            let current_elapsed = start_time.elapsed().as_secs();
            if current_elapsed >= duration {
                break;
            }

            if self.tid == 0 {
                let sample_count = self.sample_counter.load(Ordering::Relaxed);
                pb.set_message(format!("{} samples", sample_count));
                pb.set_position(current_elapsed);
            }

            let terminal = self.play_single_move();

            if terminal {
                // Game ended - assign WDL to all positions and flush
                self.flush_game_to_evaluations(&mut evaluations);
                self.reset_game();
            }
        }

        pb.finish_with_message("waiting...");

        evaluations
    }

    fn play_single_move(&mut self) -> bool {
        // Check for game end via chess rules
        if let Some(_result) = self.game.result() {
            return true;
        }

        let board = self.game.current_position();
        let board_hash = board.get_hash();

        // Track position repetitions for three-fold repetition
        *self.position_counts.entry(board_hash).or_insert(0) += 1;
        if self.position_counts[&board_hash] >= 3 {
            // Three-fold repetition - end game
            return true;
        }

        // Select move and get evaluation
        let (chosen_move, score) = self.select_move(board);

        // Check if game should be aborted (stable drawish position)
        if self.should_abort_game(&score) {
            return true;
        }

        self.game.make_move(chosen_move);

        false
    }

    fn select_move(&mut self, board: chess::Board) -> (ChessMove, i16) {
        let num_moves = self.current_game_positions.len();

        // Do single deep search to get best move + evaluation
        let (best_move, engine_score) = self.get_engine_move(&board);

        // Store position with evaluation (from white's perspective)
        let white_score = if board.side_to_move() == chess::Color::White {
            engine_score
        } else {
            -engine_score
        };
        self.current_game_positions
            .push((board.to_string(), white_score));

        // Apply temperature-based move selection
        // Use full turns (ply pairs) so both sides get equal temperature
        let full_turns = num_moves / 2;
        let chosen_move = self.select_move_with_temperature(&board, best_move, full_turns);

        (chosen_move, engine_score)
    }

    fn select_move_with_temperature(
        &mut self,
        board: &Board,
        best_move: ChessMove,
        full_turns: usize,
    ) -> ChessMove {
        let mut rng = rand::thread_rng();

        // Turn-based temperature decay (both White and Black get same temp per turn)
        // Formula: temp = 3.0 * exp(-full_turns / 7.5)
        // At turn 0: temp ≈ 3.0 (high randomness)
        // At turn 7-8: temp ≈ 1.1 (moderate randomness)
        // At turn 15: temp ≈ 0.40 (low randomness)
        // At turn 25: temp ≈ 0.10 (nearly optimal)
        // At turn 30+: temp < 0.05 (essentially optimal)
        let temperature = 3.0 * (-(full_turns as f32) / 7.5).exp();

        // With very low temperature, just play the best move
        if temperature < 0.05 {
            return best_move;
        }

        // Generate all legal moves
        let legal_moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
        if legal_moves.len() == 1 {
            return legal_moves[0];
        }

        // Use random move probability: play random move with probability = temperature / 3.0
        // This ensures both sides have equal exploration
        let random_prob = (temperature / 3.0).min(1.0);

        if rng.gen::<f32>() < random_prob {
            // Pick a truly random legal move
            let index = rng.gen_range(0..legal_moves.len());
            legal_moves[index]
        } else {
            // Play the best move
            best_move
        }
    }

    #[inline]
    fn get_engine_move(&mut self, board: &Board) -> (ChessMove, i16) {
        self.engine.set_position(*board);

        let params = GoParams {
            depth: Some(self.depth),
            ..Default::default()
        };

        self.engine.search(&params, None).unwrap()
    }

    fn should_abort_game(&self, _score: &i16) -> bool {
        let num_moves: usize = self.current_game_positions.len();

        // Check if position has been stable (drawish) for a long time
        if num_moves >= 40 {
            let start_idx = num_moves - 40;
            let last_40_positions = &self.current_game_positions[start_idx..];

            let all_drawish = last_40_positions.iter().all(|(_, eval)| eval.abs() < 20);

            if all_drawish {
                // Game has been balanced for 40+ moves - it's a draw
                return true;
            }
        }

        false
    }

    fn flush_game_to_evaluations(&mut self, evaluations: &mut Vec<(String, i16, f32, usize)>) {
        // Determine game outcome (WDL from white's perspective)
        let (wdl, is_decisive) = if let Some(result) = self.game.result() {
            use chess::GameResult;
            match result {
                GameResult::WhiteCheckmates | GameResult::BlackResigns => (1.0, true),
                GameResult::BlackCheckmates | GameResult::WhiteResigns => (0.0, true),
                GameResult::Stalemate | GameResult::DrawAccepted | GameResult::DrawDeclared => {
                    (0.5, false)
                }
            }
        } else {
            // Game aborted due to stable drawish eval (40+ moves with |eval| < 20)
            (0.5, false)
        };

        // For decisive games: include all positions
        // For drawn games: only include balanced positions (|eval| < 1000)
        // This prevents labeling clearly winning positions as draws
        let game_id = self.game_id;
        let num_positions = if is_decisive {
            let count = self.current_game_positions.len();
            for (fen, score) in self.current_game_positions.drain(..) {
                evaluations.push((fen, score, wdl, game_id));
            }
            count
        } else {
            let mut count = 0;
            for (fen, score) in self.current_game_positions.drain(..) {
                // Only include balanced positions in drawn games
                if score.abs() < 1000 {
                    evaluations.push((fen, score, wdl, game_id));
                    count += 1;
                }
            }
            count
        };

        // Update sample counter
        self.sample_counter
            .fetch_add(num_positions, Ordering::Relaxed);
    }

    #[inline]
    fn reset_game(&mut self) {
        // Get unique game ID
        self.game_id = self.game_id_counter.fetch_add(1, Ordering::Relaxed);

        // Read a random position from opening book
        let (ref path, line_count) = self.opening_book;
        if let Ok(fen) = read_random_opening_position(path, line_count) {
            if let Ok(board) = Board::from_str(&fen) {
                self.game = Game::new_with_board(board);
            } else {
                self.game = Game::new();
            }
        } else {
            self.game = Game::new();
        }

        self.position_counts.clear();
        self.current_game_positions.clear();
        self.engine.new_game();
    }
}
