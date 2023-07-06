use std::error::Error;

use cli::{cuboard_input_printer, cuboard_input_trainer};

mod bluetooth;
mod cli;
mod cube;
mod cuboard;
mod console;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let command = std::env::args().nth(1);
    let text_filename = std::env::args().nth(2);

    match command {
        Some(command) if command == "console" => {
            console::run().await?;
        }
        Some(command) if command == "train" => {
            match text_filename {
                Some(filename) => {
                    cuboard_input_trainer(filename).await?;
                }
                None => {
                    cuboard_input_printer().await?;
                }
            }
        }
        _ => {
            println!("unknown command");
        }
    }

    Ok(())
}
