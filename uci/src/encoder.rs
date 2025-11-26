use super::commands::{Score, UciOutput};

pub struct Encoder {}

impl Encoder {
    pub fn encode(&self, response: &UciOutput) -> String {
        match response {
            UciOutput::IdName(name) => format!("id name {}", name),
            UciOutput::IdAuthor(author) => format!("id author {}", author),

            UciOutput::UciOk => "uciok".to_string(),
            UciOutput::ReadyOk => "readyok".to_string(),

            UciOutput::BestMove(best_move) => format!("bestmove {}", best_move),
            UciOutput::Info(info) => {
                format!(
                    "info depth {} seldepth {} multipv 1 score {} nodes {} nps {} time {} pv {}",
                    info.depth,
                    info.sel_depth,
                    match info.score {
                        Score::Centipawns(cp) => format!("cp {}", cp),
                        Score::Mate(moves) => format!("mate {}", moves),
                    },
                    info.nodes,
                    info.nodes_per_second,
                    info.time,
                    info.pv.join(" ")
                )
            }
            UciOutput::Option(option_str) => option_str.clone(),
            UciOutput::Raw(message) => message.clone(),
        }
    }
}
