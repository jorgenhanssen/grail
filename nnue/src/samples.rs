use std::io::{self, Read, Write};

use crate::{encode_board, encoding::NUM_FEATURES};
use chess::Board;

#[derive(Clone, Debug)]
pub struct Sample {
    pub score: f32,
    pub features: [i8; NUM_FEATURES],
}

#[derive(Clone, Debug)]
pub struct Samples {
    pub samples: Vec<Sample>,
}

impl Samples {
    pub fn from_evaluations(evaluations: &Vec<(Board, f32)>) -> Self {
        let mut samples = Vec::with_capacity(evaluations.len());
        for (board, score) in evaluations {
            let encoded = encode_board(board);

            samples.push(Sample {
                score: *score,
                features: encoded,
            });
        }

        Self { samples }
    }

    pub fn write_to_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // 1) number of samples
        let num_samples = self.samples.len() as u64;
        writer.write_all(&num_samples.to_le_bytes())?;

        // 2) each sample
        for sample in &self.samples {
            // Score as f32 (4 bytes)
            writer.write_all(&sample.score.to_le_bytes())?;

            // Write all features at once
            writer.write_all(unsafe {
                std::slice::from_raw_parts(sample.features.as_ptr() as *const u8, NUM_FEATURES)
            })?;
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Read samples from an `io::Read` in the matching binary format.
    pub fn read_from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        // 1) read the u64 number of samples
        let mut size_buf = [0u8; 8];
        reader.read_exact(&mut size_buf)?;
        let num_samples = u64::from_le_bytes(size_buf);

        // 2) read each sample
        let mut samples = Vec::with_capacity(num_samples as usize);

        for _ in 0..num_samples {
            // read score (f32 => 4 bytes)
            let mut score_buf = [0u8; 4];
            reader.read_exact(&mut score_buf)?;
            let score = f32::from_le_bytes(score_buf);

            // read all features at once
            let mut features = [0i8; NUM_FEATURES];
            reader.read_exact(unsafe {
                std::slice::from_raw_parts_mut(features.as_mut_ptr() as *mut u8, NUM_FEATURES)
            })?;

            samples.push(Sample { score, features });
        }

        Ok(Self { samples })
    }
}
