/// Modern engines use a gravity formula to update history entries.
/// https://www.chessprogramming.org/History_Heuristic
pub fn apply_gravity(entry: &mut i16, delta: i32, max_value: i32) {
    let h = *entry as i32;
    let b = delta.clamp(-max_value, max_value);
    let new = h + b - ((h * b.abs()) / max_value);
    *entry = new.clamp(-max_value, max_value) as i16;
}
