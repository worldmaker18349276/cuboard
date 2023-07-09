use std::error::Error;

use train::{cuboard_input_printer, cuboard_input_trainer};

mod algorithm;
mod bluetooth;
mod console;
mod cube;
mod cuboard;
mod view;
mod train;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let command = std::env::args().nth(1);
    let text_filename = std::env::args().nth(2);

    match command {
        Some(command) if command == "console" => {
            console::run().await?;
        }
        Some(command) if command == "cube" => {
            view::window::run().await?;
        }
        Some(command) if command == "train" => match text_filename {
            Some(filename) => {
                cuboard_input_trainer(filename).await?;
            }
            None => {
                cuboard_input_printer().await?;
            }
        },
        _ => {
            println!("unknown command");
        }
    }

    Ok(())
}
