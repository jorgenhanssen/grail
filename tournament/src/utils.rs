use chess::Color;

#[inline]
pub fn color_to_index(color: Color) -> usize {
    match color {
        Color::White => 0,
        Color::Black => 1,
    }
}

#[inline]
pub fn index_to_color(index: usize) -> Color {
    match index {
        0 => Color::White,
        1 => Color::Black,
        _ => unreachable!(),
    }
}
