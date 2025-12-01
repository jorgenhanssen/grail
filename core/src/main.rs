mod engine;
mod grail;
mod nnue;
mod worker;

use std::error::Error;

use grail::Grail;

fn main() -> Result<(), Box<dyn Error>> {
    Grail::new().run()
}
