use indicatif::{ProgressBar, ProgressStyle};

pub struct TrainingProgressBar {
    bar: ProgressBar,
}

impl TrainingProgressBar {
    pub fn new(num_batches: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let bar = ProgressBar::new(num_batches as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.cyan} {pos}/{len} [{wide_bar:.cyan/blue}] {eta_precise} | {msg}",
                )
                .unwrap(),
        );
        Ok(Self { bar })
    }

    pub fn update(&self, loss: f32) {
        self.bar.set_message(format!("loss: {:.5}", loss));
        self.bar.inc(1);
    }

    pub fn finish(&self, val_loss: f32, train_loss: f32) {
        self.bar
            .set_message(format!("val: {:.5}, loss: {:.5}", val_loss, train_loss));

        self.bar.finish();
    }
}
