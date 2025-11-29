/// Gravity update for history entries used by modern engines.
///
/// Acts as a natural aging mechanism: recent information gradually outweighs old data.
/// Formula: new = old + bonus - (old * |bonus| / max)
/// - Bonus adds/subtracts from the score
/// - The decay term pulls values toward zero, proportional to their magnitude
/// - Prevents overflow and keeps scores bounded without explicit clamping logic
///
/// <https://www.chessprogramming.org/History_Heuristic>
pub fn apply_gravity(entry: &mut i16, delta: i32, max_value: i32) {
    let h = *entry as i32;
    let b = delta.clamp(-max_value, max_value);
    let new = h + b - ((h * b.abs()) / max_value);
    *entry = new.clamp(-max_value, max_value) as i16;
}
