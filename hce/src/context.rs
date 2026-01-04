use utils::Node;

/// Evaluation context with Node (for cached metrics) and phase
pub struct EvalContext<'a> {
    pub node: &'a Node,
    pub phase: f32,
    pub inv_phase: f32,
}

impl<'a> EvalContext<'a> {
    pub fn new(node: &'a Node, phase: f32) -> Self {
        Self {
            node,
            phase,
            inv_phase: 1.0 - phase,
        }
    }
}
