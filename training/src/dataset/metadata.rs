use serde::Serialize;

#[derive(Serialize)]
pub struct DatasetMetadata {
    pub train: SplitMetadata,
    pub val: SplitMetadata,
    pub test: SplitMetadata,
}

#[derive(Clone, Copy, Default, Serialize)]
pub struct SplitMetadata {
    pub samples: usize,
    pub games: usize,
}
