use std::io::{self, BufRead, Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cozy_chess::{Board, Move, Piece, Square};
use nnue::network::EVAL_OUTPUT_BOUND;

use super::{GameData, GameOutcome, MoveData, Sample};

pub fn write_game(writer: &mut impl Write, game: &GameData) -> io::Result<()> {
    let position = format!("{:#}", game.start_position);

    writer.write_u16::<LittleEndian>(position.len() as u16)?;
    writer.write_all(position.as_bytes())?;

    writer.write_u32::<LittleEndian>(game.moves.len() as u32)?;
    for data in &game.moves {
        write_move(writer, data)?;
    }

    Ok(())
}
fn write_move(writer: &mut impl Write, data: &MoveData) -> io::Result<()> {
    let from = data.mv.from as u8;
    let to = data.mv.to as u8;
    let promotion = data.mv.promotion.map(|piece| piece as u8).unwrap_or(0);
    let has_sample = u8::from(data.sample.is_some());

    writer.write_u8(from)?;
    writer.write_u8(to)?;
    writer.write_u8(promotion)?;
    writer.write_u8(has_sample)?;

    if let Some(sample) = &data.sample {
        let score = sample.score.clamp(-EVAL_OUTPUT_BOUND, EVAL_OUTPUT_BOUND);
        let outcome = match sample.outcome {
            GameOutcome::White => 0,
            GameOutcome::Draw => 1,
            GameOutcome::Black => 2,
        };

        writer.write_i16::<LittleEndian>(score)?;
        writer.write_u8(outcome)?;
    }

    Ok(())
}

pub fn read_game(reader: &mut impl BufRead) -> io::Result<Option<GameData>> {
    if reader.fill_buf()?.is_empty() {
        return Ok(None);
    }

    let position_len = reader.read_u16::<LittleEndian>()? as usize;
    let mut position = vec![0; position_len];
    reader.read_exact(&mut position)?;

    let start_position = String::from_utf8(position)
        .map_err(io::Error::other)?
        .parse::<Board>()
        .map_err(|error| io::Error::other(error.to_string()))?;

    let move_count = reader.read_u32::<LittleEndian>()? as usize;
    let mut moves = Vec::with_capacity(move_count);
    for _ in 0..move_count {
        moves.push(read_move(reader)?);
    }

    Ok(Some(GameData {
        start_position,
        moves,
    }))
}
fn read_move(reader: &mut impl Read) -> io::Result<MoveData> {
    let from = reader.read_u8()?;
    let to = reader.read_u8()?;
    let promotion = reader.read_u8()?;
    let has_sample = reader.read_u8()?;

    let mv = Move {
        from: Square::index(from as usize),
        to: Square::index(to as usize),
        promotion: if promotion == 0 {
            None
        } else {
            Some(Piece::index(promotion as usize))
        },
    };

    let sample = if has_sample != 0 {
        let score = reader.read_i16::<LittleEndian>()?;
        let outcome = match reader.read_u8()? {
            0 => GameOutcome::White,
            1 => GameOutcome::Draw,
            2 => GameOutcome::Black,
            _ => return Err(io::Error::other("Unknown game outcome")),
        };

        Some(Sample { score, outcome })
    } else {
        None
    };

    Ok(MoveData { mv, sample })
}
