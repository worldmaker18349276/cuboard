use std::error::Error;

use cli::{cuboard_input_printer, cuboard_input_trainer};

mod bluetooth;
mod cli;
mod cube;
mod cuboard;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let text_filename = std::env::args().nth(1);

    match text_filename {
        Some(filename) => cuboard_input_trainer(filename).await?,
        None => cuboard_input_printer().await?,
    }

    Ok(())
}
