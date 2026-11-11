/// Move an element to another index, keeping the other elements in order.
/// Not sure why Rust doesn't have this already but here we are.
pub fn move_vec_item<T>(items: &mut [T], from: usize, to: usize) {
    if from > to {
        items[to..=from].rotate_right(1);
    } else {
        items[from..=to].rotate_left(1);
    }
}
