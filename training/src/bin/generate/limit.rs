use std::fmt;
use uci::commands::GoParams;

/// Per-move search limit used during self-play.
#[derive(Clone, Copy, Debug)]
pub enum SearchLimit {
    /// Search to a fixed depth.
    Depth(u8),
    /// Search until a node budget is spen (finishes the ongoing iteration).
    SoftNodes(u64),
}

impl SearchLimit {
    pub fn go_params(&self) -> GoParams {
        match self {
            Self::Depth(depth) => GoParams {
                depth: Some(*depth),
                ..Default::default()
            },
            Self::SoftNodes(nodes) => GoParams {
                soft_nodes: Some(*nodes),
                ..Default::default()
            },
        }
    }
}

impl fmt::Display for SearchLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Depth(depth) => write!(f, "depth={}", depth),
            Self::SoftNodes(nodes) => write!(f, "soft_nodes={}", nodes),
        }
    }
}
