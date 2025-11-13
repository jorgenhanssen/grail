use nnue::samples::Samples;
use std::{
    collections::HashSet,
    error::Error,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

pub fn load_samples() -> Result<Samples, Box<dyn Error>> {
    let data_dir = PathBuf::from("nnue/data");

    if !data_dir.exists() {
        return Err("Data directory not found. Please run the generator first.".into());
    }

    let mut csv_files: Vec<PathBuf> = fs::read_dir(&data_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension()? == "csv" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if csv_files.is_empty() {
        return Err("No CSV files found in data directory".into());
    }

    csv_files.sort();

    let mut all_samples = Samples::new();
    let mut max_game_id = 0;

    for path in &csv_files {
        let file = File::open(path)?;
        let mut file_samples = Samples::read(BufReader::new(file))?;

        log::info!(
            "Loaded {} samples from {}",
            file_samples.len(),
            path.display()
        );

        if max_game_id > 0 {
            file_samples.offset_game_ids(max_game_id + 1);
        }

        if let Some(file_max) = file_samples.max_game_id() {
            max_game_id = file_max;
        }

        all_samples.extend(file_samples);
    }

    display_statistics(&all_samples);

    Ok(all_samples)
}

fn display_statistics(samples: &Samples) {
    let total_positions = samples.len();
    log::info!("Total positions: {}", total_positions);

    // Count unique positions for uniqueness metric
    let unique_positions: HashSet<&str> = samples.fens.iter().map(|s| s.as_ref()).collect();
    log::info!(
        "Unique positions: {:.2}%",
        (unique_positions.len() as f64 / total_positions as f64) * 100.0
    );

    // Count unique games
    let total_games: HashSet<_> = samples.game_ids.iter().copied().collect();
    log::info!("Total games: {}", total_games.len());

    // Check contiguity
    let mut current_game_id = None;
    let mut seen_games = HashSet::new();
    let mut is_continuous = true;

    for &game_id in &samples.game_ids {
        if Some(game_id) != current_game_id {
            if seen_games.contains(&game_id) {
                is_continuous = false;
                break;
            }
            seen_games.insert(game_id);
            current_game_id = Some(game_id);
        }
    }

    if is_continuous {
        log::info!("All game IDs appear in contiguous sequences");
    } else {
        log::warn!("Warning: Some game IDs appear in non-contiguous sequences!");
    }
}
