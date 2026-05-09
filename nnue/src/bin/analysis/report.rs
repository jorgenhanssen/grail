use candle_nn::Linear;
use cozy_chess::Square;
use nnue::encoding::NUM_FEATURES;
use nnue::network::model::OutputStack;
use nnue::network::{EMBEDDING_SIZE, HIDDEN_SIZE, Network, OUTPUT_BUCKETS};
use std::error::Error;
use std::fmt::{self, Write};

use crate::math::{col_norms, cosine, max_of, mean, min_of};
use crate::stats::{BucketStats, FEATURE_GROUPS, LayerStats, PIECE_TYPES, PIECES_PER_SQUARE};

/// Total width of the section rulers under each header.
const HEADER_WIDTH: usize = 68;

pub fn create(network: &Network) -> Result<String, Box<dyn Error>> {
    let mut out = String::new();

    write_embedding(&mut out, &network.embedding)?;
    write_piece_heatmaps(&mut out, &network.embedding)?;

    let bucket_stats = write_buckets(&mut out, &network.buckets)?;
    write_summary(&mut out, &bucket_stats)?;
    writeln!(&mut out)?;
    write_bucket_similarity(&mut out, &network.buckets)?;

    Ok(out)
}

fn write_header(out: &mut String, title: &str) -> fmt::Result {
    let bar = "=".repeat(HEADER_WIDTH.saturating_sub(title.len() + 1));
    writeln!(out, "{title} {bar}")
}

fn write_embedding(out: &mut String, embedding: &Linear) -> Result<(), Box<dyn Error>> {
    let stats = LayerStats::from_linear(embedding)?;
    let weights = embedding.weight().flatten_all()?.to_vec1::<f32>()?;
    let fan_in = embedding.weight().dim(1)?;
    let norms = col_norms(&weights, fan_in);

    write_header(
        out,
        &format!("embedding ({NUM_FEATURES} -> {EMBEDDING_SIZE})"),
    )?;
    write_layer(out, &stats, "  ")?;
    writeln!(out)?;

    // Feature groups, normalized to pieces mean so the scale reads as "x pieces".
    let pieces = &FEATURE_GROUPS[0];
    let pieces_mean = mean(&norms[pieces.start..pieces.end]);
    writeln!(out, "  feature groups (column norms, x pieces):")?;
    for g in FEATURE_GROUPS {
        let m = mean(&norms[g.start..g.end]);
        writeln!(
            out,
            "    {:8} ({:4} feats): norm {:.4} ({:.2}x)",
            g.name,
            g.len(),
            m,
            m / pieces_mean,
        )?;
    }
    writeln!(out)?;

    // Piece-type means combine us and them so the number reads as total weight
    // the net gives to e.g. all knights across the board.
    writeln!(out, "  piece types (column norms, x pawn):")?;
    let pawn_mean = piece_type_mean(&norms, PIECE_TYPES[0].1, PIECE_TYPES[0].2);
    for &(name, us_off, them_off) in PIECE_TYPES {
        let m = piece_type_mean(&norms, us_off, them_off);
        writeln!(
            out,
            "    {:6} ({:3} feats): norm {:.4} ({:.2}x)",
            name,
            Square::NUM * 2,
            m,
            m / pawn_mean,
        )?;
    }
    writeln!(out)?;
    Ok(())
}

fn piece_type_mean(col_norms: &[f32], us_off: usize, them_off: usize) -> f32 {
    let mut sum = 0.0;
    for sq in 0..Square::NUM {
        sum += col_norms[sq * PIECES_PER_SQUARE + us_off];
        sum += col_norms[sq * PIECES_PER_SQUARE + them_off];
    }
    sum / (Square::NUM * 2) as f32
}

// Side-by-side 8x8 heatmaps per piece type from the perspective the embedding
// sees: "us" on the left, "them" on the right. Each side is normalized against
// its own range so any us/them asymmetry is obvious. Squares are oriented from
// the perspective viewer's side, so rank 1 is the back rank for us.
fn write_piece_heatmaps(out: &mut String, embedding: &Linear) -> Result<(), Box<dyn Error>> {
    let weights = embedding.weight().flatten_all()?.to_vec1::<f32>()?;
    let fan_in = embedding.weight().dim(1)?;
    let norms = col_norms(&weights, fan_in);

    write_header(out, "piece heatmaps (col norm per square, us | them)")?;
    for &(name, us_off, them_off) in PIECE_TYPES {
        let us = per_square_norms(&norms, us_off);
        let them = per_square_norms(&norms, them_off);
        let (umin, umax) = (min_of(&us), max_of(&us));
        let (tmin, tmax) = (min_of(&them), max_of(&them));

        writeln!(
            out,
            "  {name:6} us ({umin:.3} - {umax:.3})        them ({tmin:.3} - {tmax:.3})",
        )?;
        writeln!(
            out,
            "      a  b  c  d  e  f  g  h       a  b  c  d  e  f  g  h"
        )?;
        for rank in (0..8).rev() {
            write_rank(out, "    ", rank, &us, umin, umax)?;
            write!(out, "    ")?;
            write_rank(out, "", rank, &them, tmin, tmax)?;
            writeln!(out)?;
        }
        writeln!(out)?;
    }
    Ok(())
}

fn per_square_norms(col_norms: &[f32], offset: usize) -> Vec<f32> {
    (0..Square::NUM)
        .map(|sq| col_norms[sq * PIECES_PER_SQUARE + offset])
        .collect()
}

fn write_rank(
    out: &mut String,
    indent: &str,
    rank: usize,
    per_square: &[f32],
    min: f32,
    max: f32,
) -> fmt::Result {
    write!(out, "{indent}{}", rank + 1)?;
    let range = (max - min).max(f32::EPSILON);
    for file in 0..8 {
        let sq = rank * 8 + file;
        let shade = match ((per_square[sq] - min) / range * 5.0) as usize {
            0 => "  ",
            1 => "░░",
            2 => "▒▒",
            3 => "▓▓",
            _ => "██",
        };
        write!(out, " {shade}")?;
    }
    Ok(())
}

fn write_buckets(
    out: &mut String,
    buckets: &[OutputStack; OUTPUT_BUCKETS],
) -> Result<Vec<BucketStats>, Box<dyn Error>> {
    let mut all = Vec::with_capacity(OUTPUT_BUCKETS);
    for (i, stack) in buckets.iter().enumerate() {
        let stats = BucketStats::from_stack(stack)?;

        write_header(out, &format!("bucket {i}"))?;
        writeln!(out, "  hidden1 ({} -> {HIDDEN_SIZE}):", 2 * EMBEDDING_SIZE)?;
        write_layer(out, &stats.h1, "    ")?;
        writeln!(out, "  hidden2 ({HIDDEN_SIZE} -> {HIDDEN_SIZE}):")?;
        write_layer(out, &stats.h2, "    ")?;
        writeln!(out, "  output ({HIDDEN_SIZE} -> 1):")?;
        write_layer(out, &stats.output, "    ")?;
        writeln!(out)?;

        all.push(stats);
    }
    Ok(all)
}

fn write_layer(out: &mut String, s: &LayerStats, indent: &str) -> fmt::Result {
    let rel = match s.rel_to_ref {
        Some(r) => format!(", rel {r:.2}x"),
        None => String::new(),
    };
    writeln!(
        out,
        "{indent}weight:  mean {:.4}, median {:.4}, std {:.4}, max {:.4}, scale {:.2}x, CoV {:.2}{rel}, dead {:.1}%",
        s.weight_mean_abs,
        s.weight_median_abs,
        s.weight_std,
        s.weight_max_abs,
        s.scale,
        s.cov,
        s.dead_fraction * 100.0,
    )?;
    writeln!(
        out,
        "{indent}neurons: {}/{} active, row norm min {:.3} / mean {:.3} / max {:.3}",
        s.active_neurons, s.total_neurons, s.row_norm_min, s.row_norm_mean, s.row_norm_max,
    )?;
    writeln!(
        out,
        "{indent}bias:    mean {:+.4} (|b| mean {:.4}, max {:.4})",
        s.bias_mean_signed, s.bias_mean_abs, s.bias_max_abs,
    )
}

fn write_summary(out: &mut String, buckets: &[BucketStats]) -> fmt::Result {
    write_header(out, &format!("summary (across {} buckets)", buckets.len()))?;

    let h1_scale: Vec<f32> = buckets.iter().map(|b| b.h1.scale).collect();
    let h2_scale: Vec<f32> = buckets.iter().map(|b| b.h2.scale).collect();
    let out_scale: Vec<f32> = buckets.iter().map(|b| b.output.scale).collect();
    writeln!(out, "  scale (x init):")?;
    summary_row(out, "hidden1", &h1_scale, "x", 2)?;
    summary_row(out, "hidden2", &h2_scale, "x", 2)?;
    summary_row(out, "output", &out_scale, "x", 2)?;
    writeln!(out)?;

    // Pre-scale to percent so the helper stays simple.
    let h1_dead: Vec<f32> = buckets.iter().map(|b| b.h1.dead_fraction * 100.0).collect();
    let h2_dead: Vec<f32> = buckets.iter().map(|b| b.h2.dead_fraction * 100.0).collect();
    let out_dead: Vec<f32> = buckets
        .iter()
        .map(|b| b.output.dead_fraction * 100.0)
        .collect();
    writeln!(out, "  dead weight fraction:")?;
    summary_row(out, "hidden1", &h1_dead, "%", 1)?;
    summary_row(out, "hidden2", &h2_dead, "%", 1)?;
    summary_row(out, "output", &out_dead, "%", 1)?;
    writeln!(out)?;

    let h1_active: Vec<f32> = buckets.iter().map(|b| b.h1.active_neurons as f32).collect();
    let h2_active: Vec<f32> = buckets.iter().map(|b| b.h2.active_neurons as f32).collect();
    writeln!(out, "  active neurons (of {HIDDEN_SIZE}):")?;
    summary_row(out, "hidden1", &h1_active, "", 1)?;
    summary_row(out, "hidden2", &h2_active, "", 1)?;
    writeln!(out)?;

    let h1_cov: Vec<f32> = buckets.iter().map(|b| b.h1.cov).collect();
    let h2_cov: Vec<f32> = buckets.iter().map(|b| b.h2.cov).collect();
    let out_cov: Vec<f32> = buckets.iter().map(|b| b.output.cov).collect();
    writeln!(out, "  CoV (std / mean |W|):")?;
    summary_row(out, "hidden1", &h1_cov, "", 2)?;
    summary_row(out, "hidden2", &h2_cov, "", 2)?;
    summary_row(out, "output", &out_cov, "", 2)?;
    writeln!(out)?;

    let h1_rel: Vec<f32> = buckets
        .iter()
        .map(|b| b.h1.rel_to_ref.unwrap_or(0.0))
        .collect();
    let h2_rel: Vec<f32> = buckets
        .iter()
        .map(|b| b.h2.rel_to_ref.unwrap_or(0.0))
        .collect();
    writeln!(out, "  rel (x bucket output mean |W|):")?;
    summary_row(out, "hidden1", &h1_rel, "x", 2)?;
    summary_row(out, "hidden2", &h2_rel, "x", 2)?;

    Ok(())
}

fn summary_row(
    out: &mut String,
    label: &str,
    values: &[f32],
    suffix: &str,
    decimals: usize,
) -> fmt::Result {
    writeln!(
        out,
        "    {label:7}: avg {:.*}{suffix} (range {:.*}{suffix} - {:.*}{suffix})",
        decimals,
        mean(values),
        decimals,
        min_of(values),
        decimals,
        max_of(values),
    )
}

// Upper-triangle cosine similarity between each bucket's output weight vector.
// Diagonal is always 1.00 and the matrix is symmetric, so we skip both.
fn write_bucket_similarity(
    out: &mut String,
    buckets: &[OutputStack; OUTPUT_BUCKETS],
) -> Result<(), Box<dyn Error>> {
    let mut weights: Vec<Vec<f32>> = Vec::with_capacity(OUTPUT_BUCKETS);
    for b in buckets {
        weights.push(b.output.weight().flatten_all()?.to_vec1::<f32>()?);
    }

    write_header(out, "bucket output similarity (cosine)")?;
    write!(out, "     ")?;
    for j in 1..OUTPUT_BUCKETS {
        write!(out, "{:>6}", format!("b{j}"))?;
    }
    writeln!(out)?;

    for i in 0..OUTPUT_BUCKETS - 1 {
        write!(out, "  b{i} ")?;
        for j in 1..OUTPUT_BUCKETS {
            if j <= i {
                write!(out, "      ")?;
            } else {
                write!(out, "{:>6.2}", cosine(&weights[i], &weights[j]))?;
            }
        }
        writeln!(out)?;
    }
    Ok(())
}
