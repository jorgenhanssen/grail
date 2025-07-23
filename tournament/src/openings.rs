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
    Opening {
        name: "Sicilian Najdorf, English Attack",
        fen: "rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N1B3/PPP2PPP/R2QKB1R b KQkq - 1 6",
    },
    Opening {
        name: "Sicilian Sveshnikov",
        fen: "r1bqkb1r/pp1p1ppp/2n2n2/4p3/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq e6 0 6",
    },
    Opening {
        name: "Sicilian Richter-Rauzer Variation",
        fen: "r1bqkb1r/pp2pppp/2np1n2/6B1/3NP3/2N5/PPP2PPP/R2QKB1R b KQkq - 4 6",
    },
    Opening {
        name: "Sicilian Scheveningen Variation",
        fen: "rnbqkb1r/pp3ppp/3ppn2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6",
    },
    Opening {
        name: "Sicilian Dragon (main line)",
        fen: "r1bq1rk1/pp2ppbp/2np1np1/8/2BNP3/2N1BP2/PPPQ2PP/R3K2R b KQ - 4 9",
    },
    Opening {
        name: "Sicilian Accelerated Dragon Maroczy Bind",
        fen: "r1bqkbnr/pp1ppp1p/2n3p1/8/2PNP3/8/PP3PPP/RNBQKB1R b KQkq c3 0 5",
    },
    Opening {
        name: "Sicilian Smith-Morra Gambit",
        fen: "rnbqkbnr/pp1ppppp/8/2p5/3PP3/8/PPP2PPP/RNBQKBNR b KQkq d3 0 2",
    },
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
    Opening {
        name: "Caro-Kann: Fantasy Variation",
        fen: "rnbqkbnr/pp2pppp/2p5/3p4/3PP3/5P2/PPP3PP/RNBQKBNR b KQkq - 0 3",
    },
    Opening {
        name: "Caro-Kann: Bronstein-Larsen Variation",
        fen: "rnbqkb1r/pp2pp1p/2p2p2/8/3P4/8/PPP2PPP/R1BQKBNR w KQkq - 0 6",
    },
    Opening {
        name: "Scandinavian",
        fen: "rnb1kbnr/ppp1pppp/8/q7/8/2N5/PPPP1PPP/R1BQKBNR w KQkq - 2 4",
    },
    Opening {
        name: "French",
        fen: "rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    },
    Opening {
        name: "French Winawer",
        fen: "rnbqk1nr/ppp2ppp/4p3/3p4/1b1PP3/2N5/PPP2PPP/R1BQKBNR w KQkq - 2 4",
    },
    Opening {
        name: "French Advance Variation",
        fen: "rnbqkbnr/ppp2ppp/4p3/3pP3/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3",
    },
    Opening {
        name: "French Tarrasch Variation",
        fen: "rnbqkbnr/ppp2ppp/4p3/3p4/3PP3/8/PPPN1PPP/R1BQKBNR b KQkq - 1 3",
    },
    Opening {
        name: "French Classical Steinitz",
        fen: "rnbqkb1r/ppp2ppp/4pn2/3pP3/3P4/2N5/PPP2PPP/R1BQKBNR b KQkq - 0 4",
    },
    Opening {
        name: "French Exchange Variation",
        fen: "rnbqkbnr/ppp2ppp/4p3/3P4/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3",
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
        name: "Pirc Defense: Austrian Attack",
        fen: "rnbqkb1r/ppp1pp1p/3p1np1/8/3PPP2/2N5/PPP3PP/R1BQKBNR b KQkq f3 0 4",
    },
    Opening {
        name: "Tarrasch Defense",
        fen: "rnbqkbnr/pp3ppp/4p3/2pp4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq c6 0 4",
    },
    Opening {
        name: "Bogo-Indian",
        fen: "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 2 4",
    },
    Opening {
        name: "Modern Defense",
        fen: "rnbqkbnr/pppppp1p/6p1/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    },
    Opening {
        name: "Philidor Defense",
        fen: "rnbqkbnr/ppp2ppp/3p4/4p3/4P3/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 3",
    },
    Opening {
        name: "Petrov's Defense",
        fen: "rnbqkb1r/pppp1ppp/5n2/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
    },
    Opening {
        name: "Nimzowitsch Defense",
        fen: "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
    },
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
    Opening {
        name: "Italian",
        fen: "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
    },
    Opening {
        name: "Scotch Opening",
        fen: "r1bqkbnr/pppp1ppp/2n5/4p3/3PP3/5N2/PPP2PPP/RNBQKB1R b KQkq d3 0 3",
    },
    Opening {
        name: "Scotch Gambit",
        fen: "r1bqkbnr/pppp1ppp/2n5/8/2BpP3/5N2/PPP2PPP/RNBQK2R b KQkq - 1 4",
    },
    Opening {
        name: "Four Knights Game",
        fen: "r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 4 4",
    },
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
    Opening {
        name: "Queen's Gambit",
        fen: "rnbqkbnr/ppp1pppp/8/3p4/2PP4/8/PP2PPPP/RNBQKBNR b KQkq c3 0 2",
    },
    Opening {
        name: "Queen's Gambit Accepted",
        fen: "rnbqkbnr/ppp1pppp/8/8/2pP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3",
    },
    Opening {
        name: "Queen's Gambit Declined: Slav",
        fen: "rnbqkbnr/pp2pppp/2p5/3p4/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3",
    },
    Opening {
        name: "Queen's Gambit Declined: Albin Countergambit",
        fen: "rnbqkbnr/ppp2ppp/8/3pp3/2PP4/8/PP2PPPP/RNBQKBNR w KQkq e6 0 3",
    },
    Opening {
        name: "Queen's Gambit Declined: Orthodox Defense",
        fen: "r1bq1rk1/pppnbppp/4pn2/3p2B1/2PP4/2N1PN2/PP3PPP/R2QKB1R w KQ - 3 7",
    },
    Opening {
        name: "Queen's Gambit Declined: Chigorin Defense",
        fen: "r1bqkbnr/ppp1pppp/2n5/3p4/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 1 3",
    },
    Opening {
        name: "Semi-Slav Defense: Semi-Meran Variation",
        fen: "r1bqkb1r/pp1n1ppp/2p1pn2/8/2BP4/2N1PN2/PP3PPP/R1BQK2R b KQkq - 0 7",
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
    Opening {
        name: "Giuoco Pianissimo Variation",
        fen: "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/3P1N2/PPP2PPP/RNBQK2R b KQkq - 0 4",
    },
    Opening {
        name: "Italian Game: Fried Liver Attack",
        fen: "r1bqkb1r/ppp2Npp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R b KQkq - 0 6",
    },
    Opening {
        name: "Danish Gambit",
        fen: "rnbqkbnr/pppp1ppp/8/8/3pP3/2P5/PP3PPP/RNBQKBNR b KQkq - 0 3",
    },
    Opening {
        name: "King's Gambit Accepted: Muzio Gambit",
        fen: "rnbqkbnr/pppp1p1p/8/8/2B1Ppp1/5N2/PPPP2PP/RNBQ1RK1 b kq - 1 5",
    },
    Opening {
        name: "Petrov's Defense: Stafford Gambit",
        fen: "r1bqkb1r/pppp1ppp/2n2n2/4N3/4P3/8/PPPP1PPP/RNBQKB1R w KQkq - 1 4",
    },
    Opening {
        name: "Elephant Gambit",
        fen: "rnbqkbnr/ppp2ppp/8/3pp3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq d6 0 3",
    },
    Opening {
        name: "Benko Gambit",
        fen: "rnbqkb1r/p2ppppp/5n2/1ppP4/2P5/8/PP2PPPP/RNBQKBNR w KQkq b6 0 4",
    },
    Opening {
        name: "Benoni Fianchetto",
        fen: "rnbqkb1r/pp3p1p/3p1np1/2pP4/8/2N2NP1/PP2PP1P/R1BQKB1R b KQkq - 0 7",
    },
    Opening {
        name: "King’s Indian, Fianchetto Variation",
        fen: "rnbqk2r/ppppppbp/5np1/8/2PP4/6P1/PP2PPBP/RNBQK1NR b KQkq - 2 4",
    },
    Opening {
        name: "Old Indian Defense",
        fen: "rnbqkb1r/ppp1pppp/3p1n2/8/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3",
    },
    // New:
    Opening {
        name: "King’s Indian Attack",
        fen: "rnbqkbnr/ppp1pppp/8/3p4/8/5NP1/PPPPPP1P/RNBQKB1R b KQkq - 0 2",
    },
    Opening {
        name: "Torre Attack",
        fen: "rnbqkb1r/ppp1pppp/5n2/3p2B1/3P4/5N2/PPP1PPPP/RN1QKB1R b KQkq - 3 3",
    },
    Opening {
        name: "Hedgehog Defense",
        fen: "rn1qk2r/pb1pbppp/1p2pn2/2p5/2P5/2N2NP1/PP1PPPBP/R1BQ1RK1 w kq - 2 7",
    },
    Opening {
        name: "Symmetrical English",
        fen: "rnbqkbnr/pp1ppppp/8/2p5/2P5/8/PP1PPPPP/RNBQKBNR w KQkq c6 0 2",
    },
    Opening {
        name: "Botvinnik English",
        fen: "r1bqk1nr/ppp2pbp/2np2p1/4p3/2P1P3/2NP2P1/PP3PBP/R1BQK1NR b KQkq e3 0 6",
    },
    Opening {
        name: "Nimzowitsch-Larsen Attack",
        fen: "rnbqkbnr/pppppppp/8/8/8/1P6/P1PPPPPP/RNBQKBNR b KQkq - 0 1",
    },
    Opening {
        name: "Grob Opening",
        fen: "rnbqkbnr/pppppppp/8/8/6P1/8/PPPPPP1P/RNBQKBNR b KQkq g3 0 1",
    },
    Opening {
        name: "Owen’s Defense",
        fen: "rnbqkbnr/p1pppppp/1p6/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    },
    Opening {
        name: "Englund Gambit",
        fen: "rnbqkbnr/pppp1ppp/8/4p3/3P4/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 2",
    },
    Opening {
        name: "Hippopotamus Defense",
        fen: "rnbqkb1r/pppppppp/7n/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
    },
];

#[inline]
pub fn get_all_openings() -> Vec<Opening> {
    OPENINGS.to_vec()
}
