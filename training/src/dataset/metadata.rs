use std::ops::AddAssign;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub train: SplitMetadata,
    pub val: SplitMetadata,
    pub test: SplitMetadata,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct SplitMetadata {
    pub samples: usize,
    pub games: usize,
}

impl AddAssign for DatasetMetadata {
    fn add_assign(&mut self, other: Self) {
        self.train += other.train;
        self.val += other.val;
        self.test += other.test;
    }
}

impl AddAssign for SplitMetadata {
    fn add_assign(&mut self, other: Self) {
        self.samples += other.samples;
        self.games += other.games;
    }
}
