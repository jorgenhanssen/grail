use crate::utils::{MAX_PIECE_PRIORITY, MIN_PRIORITY};

// clamp to 0-7, most empirical chess research says this is reasonable
const fn clamp(rd: usize, mut r: usize) -> u8 {
    let limit = if rd > 0 { rd - 1 } else { 0 };
    if r > limit {
        r = limit;
    }
    if r > 7 {
        r = 7;
    }
    r as u8
}

const BUCKET_0: usize = 0;
const BUCKET_1: usize = 1;
const BUCKET_2: usize = 2;
const BUCKET_3: usize = 3;

// high-priority   base = rd / 4
const LMR_LESS: [[u8; 100]; 4] = {
    let mut table = [[0u8; 100]; 4];
    let mut rd = 0; // remaining depth
    while rd < 100 {
        let base = rd / 4;

        table[BUCKET_0][rd] = clamp(rd, base);
        table[BUCKET_1][rd] = clamp(rd, base + 1);
        table[BUCKET_2][rd] = clamp(rd, base + 2);
        table[BUCKET_3][rd] = clamp(rd, base + 3);

        rd += 1;
    }
    table
};

// low-priority    base = rd / 3
const LMR_MORE: [[u8; 100]; 4] = {
    let mut table = [[0u8; 100]; 4];
    let mut rd = 0; // remaining depth
    while rd < 100 {
        let base = rd / 3;

        table[BUCKET_0][rd] = clamp(rd, base);
        table[BUCKET_1][rd] = clamp(rd, base + 1);
        table[BUCKET_2][rd] = clamp(rd, base + 2);
        table[BUCKET_3][rd] = clamp(rd, base + 3);

        rd += 1;
    }
    table
};

// Moves later in the list get a penalty (often worse)
// Assuming no more than 100 moves in the list
#[inline(always)]
const fn move_index_bucket(idx: usize) -> usize {
    match idx {
        0..=5 => BUCKET_0,
        6..=9 => BUCKET_1,
        10..=17 => BUCKET_2,
        _ => BUCKET_3,
    }
}

#[inline(always)]
pub fn lmr(
    remaining_depth: u8,
    static_score: i16,
    tactical: bool,  // in check OR giving check
    move_idx: usize, // 0-based index in move list
) -> u8 {
    // No reduction for:
    // Tactical nodes or near the horizon
    if tactical || remaining_depth < 3 {
        return 0;
    }
    // Any captures/promotions
    if static_score > MAX_PIECE_PRIORITY {
        return 0;
    }
    // First three moves (might raise alpha)
    if move_idx < 3 {
        return 0;
    }

    let bucket = move_index_bucket(move_idx);

    if static_score > MIN_PRIORITY {
        LMR_LESS[bucket][remaining_depth as usize]
    } else {
        LMR_MORE[bucket][remaining_depth as usize]
    }
}
