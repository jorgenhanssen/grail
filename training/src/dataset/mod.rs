mod binary;
mod collection;
mod reader;
mod samples;
mod stats;
mod writer;

pub use collection::DatasetCollection;
pub use reader::{DatasetReader, GameReader};
pub use samples::{GameData, GameOutcome, MoveData, Sample, TrainingSample};
pub use stats::DatasetCollectionStats;
pub use writer::DatasetWriter;

const TRAIN_FILE: &str = "train.bin";
const VAL_FILE: &str = "val.bin";
const TEST_FILE: &str = "test.bin";
