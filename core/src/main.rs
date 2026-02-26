mod display;
mod engine;
mod grail;
mod nnue;
mod search_metadata;
mod worker;

use std::error::Error;

use grail::Grail;

fn main() -> Result<(), Box<dyn Error>> {
    Grail::new().run()
}
