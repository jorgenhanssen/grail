use std::fmt;

#[derive(Clone, Debug)]
pub struct Opening {
    pub name: &'static str,
    pub fen: &'static str,
}

impl fmt::Display for Opening {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

const OPENINGS: &[Opening] = &[
    Opening {
        name: "Standard",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    },
    // Sicilian Defense Variations
    Opening {
        name: "Sicilian",
        fen: "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    },
    Opening {
        name: "Sicilian (knights)",
        fen: "r1bqkbnr/pp1ppppp/2n5/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
    },
    Opening {
        name: "Closed Sicilian",
        fen: "r1bqkbnr/pp1ppppp/2n5/2p5/4P3/2N5/PPPP1PPP/R1BQKBNR w KQkq - 2 3",
    },
    Opening {
        name: "Sicilian Alapin's",
        fen: "rnbqkbnr/pp1ppppp/8/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR b KQkq - 0 2",
    },
    Opening {
        name: "Sicilian Grand Prix Attack",
        fen: "rnbqkbnr/pp1ppppp/8/2p5/4PP2/8/PPPP2PP/RNBQKBNR b KQkq f3 0 2",
    },
    Opening {
        name: "Sicilian, accelerated fianchetto, modern",
        fen: "r1bqk1nr/pp1pppbp/2n3p1/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 2 6",
    },
    Opening {
        name: "Sicilian Hyper Accelerated Dragon",
        fen: "rnbqkbnr/pp1ppp1p/6p1/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 3",
    },
    Opening {
        name: "Sicilian Najdorf",
        fen: "rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6",
    },
    // Caro-Kann Defense Variations
    Opening {
        name: "Caro-Kann",
        fen: "rnbqkbnr/pp1ppppp/2p5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    },
    Opening {
        name: "Caro-Kann (two knights)",
        fen: "rnbqkbnr/pp2pppp/2p5/3p4/4P3/2N2N2/PPPP1PPP/R1BQKB1R b KQkq - 1 3",
    },
    Opening {
        name: "Caro-Kann (exchange)",
        fen: "rnbqkbnr/pp1ppppp/2p5/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
    },
    Opening {
        name: "Caro-Kann (advanced)",
        fen: "rnbqkbnr/pp2pppp/2p5/3pP3/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3",
    },
    // Other e4 Defenses
    Opening {
        name: "Scandinavian",
        fen: "rnb1kbnr/ppp1pppp/8/q7/8/2N5/PPPP1PPP/R1BQKBNR w KQkq - 2 4",
    },
    Opening {
        name: "French Defense",
        fen: "rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    },
    Opening {
        name: "French Winawer",
        fen: "rnbqk1nr/ppp2ppp/4p3/3p4/1b1PP3/2N5/PPP2PPP/R1BQKBNR w KQkq - 2 4",
    },
    Opening {
        name: "Alekhine's Defense",
        fen: "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPPPPPP/RNBQKBNR w KQkq - 1 2",
    },
    Opening {
        name: "Pirc Defense",
        fen: "rnbqkbnr/ppp1pppp/3p4/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    },
    Opening {
        name: "Modern Defense",
        fen: "rnbqkbnr/pppppp1p/6p1/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    },
    // Philidor Defense
    Opening {
        name: "Philidor Defense",
        fen: "rnbqkbnr/ppp2ppp/3p4/4p3/4P3/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 3",
    },
    // Petrov'd
    Opening {
        name: "Petrov'd",
        fen: "rnbqkb1r/pppp1ppp/5n2/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
    },
    // Nimzowitsch Defense
    Opening {
        name: "Nimzowitsch Defense",
        fen: "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
    },
    // Open Games
    Opening {
        name: "Kings Pawn",
        fen: "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
    },
    Opening {
        name: "Vienna",
        fen: "rnbqkbnr/pppp1ppp/8/4p3/4P3/2N5/PPPP1PPP/R1BQKBNR b KQkq - 1 2",
    },
    Opening {
        name: "Ruy Lopez",
        fen: "r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
    },
    Opening {
        name: "Ruy Lopez Berlin Defense",
        fen: "r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
    },
    // Italian
    Opening {
        name: "Italian",
        fen: "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
    },
    // Scotch Opening
    Opening {
        name: "Scotch Opening",
        fen: "r1bqkbnr/pppp1ppp/2n5/4p3/3PP3/5N2/PPP2PPP/RNBQKB1R b KQkq d3 0 3",
    },
    Opening {
        name: "Four Knights Game",
        fen: "r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 4 4",
    },
    // King's Gambit and Related
    Opening {
        name: "King's Gambit",
        fen: "rnbqkbnr/pppp1ppp/8/4p3/4PP2/8/PPPP2PP/RNBQKBNR b KQkq f3 0 2",
    },
    Opening {
        name: "King's Gambit (accepted)",
        fen: "rnbqkbnr/pppp1ppp/8/8/4Pp2/8/PPPP2PP/RNBQKBNR w KQkq - 0 3",
    },
    Opening {
        name: "King's Gambit (declined), Falkbeer Countergambit",
        fen: "rnbqkbnr/ppp2ppp/8/3pp3/4PP2/8/PPPP2PP/RNBQKBNR w KQkq d6 0 3",
    },
    Opening {
        name: "Evans Gambit",
        fen: "r1bqk1nr/pppp1ppp/2n5/2b1p3/1PB1P3/5N2/P1PP1PPP/RNBQK2R b KQkq b3 0 4",
    },
    // d4 Openings and Indian Defenses
    Opening {
        name: "Queen's Gambit",
        fen: "rnbqkbnr/ppp1pppp/8/3p4/2PP4/8/PP2PPPP/RNBQKBNR b KQkq c3 0 2",
    },
    Opening {
        name: "Queen's Gambit (accepted)",
        fen: "rnbqkbnr/ppp1pppp/8/8/2pP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3",
    },
    Opening {
        name: "Queen's Gambit (declined), Slav",
        fen: "rnbqkbnr/pp2pppp/2p5/3p4/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3",
    },
    Opening {
        name: "Queen's Gambit Albin Countergambit",
        fen: "rnbqkbnr/ppp2ppp/8/3pp3/2PP4/8/PP2PPPP/RNBQKBNR w KQkq e6 0 3",
    },
    Opening {
        name: "Nimzo-Indian Defense",
        fen: "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 2 4",
    },
    Opening {
        name: "Catalan",
        fen: "rnbqkb1r/pppp1ppp/4pn2/8/2PP4/6P1/PP2PP1P/RNBQKBNR b KQkq - 0 3",
    },
    Opening {
        name: "Grunfeld Defense",
        fen: "rnbqkb1r/ppp1pp1p/5np1/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 0 4",
    },
    Opening {
        name: "King's Indian Defense",
        fen: "rnbqkb1r/pppppp1p/5np1/8/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3",
    },
    Opening {
        name: "Modern Benoni",
        fen: "rnbqkb1r/pp1p1ppp/4pn2/2pP4/2P5/8/PP2PPPP/RNBQKBNR w KQkq - 0 4",
    },
    Opening {
        name: "Queen's Indian Defense",
        fen: "rnbqkb1r/p1pp1ppp/1p2pn2/8/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 0 4",
    },
    Opening {
        name: "Dutch",
        fen: "rnbqkbnr/ppppp1pp/8/5p2/3P4/8/PPP1PPPP/RNBQKBNR w KQkq f6 0 2",
    },
    Opening {
        name: "London",
        fen: "rnbqkb1r/ppp1pppp/5n2/3p4/3P1B2/5N2/PPP1PPPP/RN1QKB1R b KQkq - 3 3",
    },
    Opening {
        name: "Trompowsky Attack",
        fen: "rnbqkb1r/pppppppp/5n2/6B1/3P4/8/PPP1PPPP/RN1QKBNR b KQkq - 2 2",
    },
    Opening {
        name: "Colle System",
        fen: "rnbqkb1r/ppp1pppp/5n2/3p4/3P4/4PN2/PPP2PPP/RNBQKB1R b KQkq - 0 3",
    },
    // Flank and Irregular Openings
    Opening {
        name: "English",
        fen: "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3 0 1",
    },
    Opening {
        name: "Reti",
        fen: "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1",
    },
    Opening {
        name: "Bird's Opening",
        fen: "rnbqkbnr/ppp1pppp/8/3p4/5P2/8/PPPPP1PP/RNBQKBNR w KQkq d6 0 2",
    },
    Opening {
        name: "Polish Opening",
        fen: "rnbqkbnr/pppppppp/8/8/1P6/8/P1PPPPPP/RNBQKBNR b KQkq b3 0 1",
    },
    Opening {
        name: "Budapest Gambit",
        fen: "rnbqkb1r/pppp1ppp/5n2/4p3/2PP4/8/PP2PPPP/RNBQKBNR w KQkq e6 0 3",
    },
];

#[inline]
pub fn get_all_openings() -> Vec<Opening> {
    OPENINGS.to_vec()
}
