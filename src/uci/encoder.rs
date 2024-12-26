use super::commands::UciOutput;

pub struct Encoder {}

impl Encoder {
    pub fn encode(&self, response: &UciOutput) -> String {
        let output = match response {
            UciOutput::IdName(name) => format!("id name {}", name),
            UciOutput::IdAuthor(author) => format!("id author {}", author),

            UciOutput::UciOk => "uciok".to_string(),
            UciOutput::ReadyOk => "readyok".to_string(),

            UciOutput::BestMove { bestmove, ponder } => {
                format!(
                    "bestmove {}{}",
                    bestmove,
                    ponder
                        .as_ref()
                        .map_or(String::new(), |m| format!(" ponder {}", m))
                )
            }
            UciOutput::Info(info) => {
                format!(
                    "info depth {} multipv 1 score cp {} nodes {} nps {} time {} pc {}",
                    info.depth,
                    info.score,
                    info.nodes,
                    info.nodes_per_second,
                    info.time,
                    info.line
                        .iter()
                        .map(|m| m.to_string())
                        .collect::<Vec<String>>()
                        .join(" ")
                )
            }
        };

        output
    }
}
