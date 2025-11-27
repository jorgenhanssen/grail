use utils::Position;

/// Evaluation context with Position (for cached attack map) and phase
pub struct EvalContext<'a> {
    pub position: &'a Position<'a>,
    pub phase: f32,
    pub inv_phase: f32,
}

impl<'a> EvalContext<'a> {
    pub fn new(position: &'a Position<'a>, phase: f32) -> Self {
        Self {
            position,
            phase,
            inv_phase: 1.0 - phase,
        }
    }
}
