pub struct MetricsTracker {
    best_val_loss: f32,
    epochs_no_improve: u64,
    patience: u64,
}

impl MetricsTracker {
    pub fn new(patience: u64) -> Self {
        Self {
            best_val_loss: f32::MAX,
            epochs_no_improve: 0,
            patience,
        }
    }

    // Returns if model improved (lowest val loss)
    pub fn update(&mut self, val_loss: f32) -> bool {
        if val_loss < self.best_val_loss {
            self.best_val_loss = val_loss;
            self.epochs_no_improve = 0;
            true
        } else {
            self.epochs_no_improve += 1;
            false
        }
    }

    pub fn should_stop(&self) -> bool {
        self.epochs_no_improve >= self.patience
    }
}
