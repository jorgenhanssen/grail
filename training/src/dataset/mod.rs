mod binary;
mod collection;
mod metadata;
mod reader;
mod samples;
mod writer;

pub use collection::DatasetCollection;
pub use metadata::{DatasetMetadata, SplitMetadata};
pub use reader::{DatasetReader, GameReader};
pub use samples::{GameData, GameOutcome, MoveData, Sample, TrainingSample};
pub use writer::DatasetWriter;

const TRAIN_FILE: &str = "train.bin";
const VAL_FILE: &str = "val.bin";
const TEST_FILE: &str = "test.bin";
const METADATA_FILE: &str = "meta.json";
