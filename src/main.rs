use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use cube::format_moves;
use std::error::Error;

use bluetooth::gancubev2::{GanCubeV2Builder, ResponseMessage};

use crate::cuboard::Cuboard;

mod bluetooth;
mod cube;
mod cuboard;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // get the first bluetooth adapter
    let manager = platform::Manager::new().await.unwrap();
    let adapters = manager.adapters().await?;
    let adapter = adapters.into_iter().next().unwrap();

    // start scanning for devices
    adapter.start_scan(ScanFilter::default()).await?;
    println!("press enter if your GANCube is ready to connect.");

    let mut buf = String::new();
    let builder = 'a: loop {
        std::io::stdin().read_line(&mut buf)?;

        println!("scan devices...");
        let found = GanCubeV2Builder::find_gancube_device(&adapter).await?;
        if let Some(builder) = found.into_iter().next() {
            break 'a builder;
        }
        println!("no GANCube is found, please try again.");
    };

    println!("connect to GANCube...");
    let gancube = builder.connect().await?;
    println!("connected! have fun~");
    println!();

    println!("{}", DEFAULT_CHEATSHEET);
    println!();

    let mut input_handler = CuboardInputPrinter::new();
    let handle = gancube
        .register_handler(Box::new(move |msg| input_handler.handle_message(msg)))
        .await?;

    gancube.subscribe_response().await?;
    gancube.request_cube_state().await?;

    handle.await?;

    Ok(())
}

struct CuboardInput {
    cuboard: Cuboard,
    keymap: [[[&'static str; 4]; 12]; 2],
    count: Option<u8>,
}

const DEFAULT_KEYMAP: [[[&str; 4]; 12]; 2] = [
    [
        ["d", "u", "c", "k"],
        ["(", "[", "{", "<"],
        ["g", "a", "s", "p"],
        [" ", "0", "z", "q"],
        ["f", "l", "o", "w"],
        [".", ":", "'", "!"],
        ["j", "i", "n", "x"],
        ["+", "-", "*", "/"],
        ["m", "y", "t", "h"],
        ["1", "2", "3", "4"],
        ["v", "e", "r", "b"],
        ["@", "$", "&", "`"],
    ],
    [
        ["D", "U", "C", "K"],
        [")", "]", "}", ">"],
        ["G", "A", "S", "P"],
        ["\n", "9", "Z", "Q"],
        ["F", "L", "O", "W"],
        [",", ";", "\"", "?"],
        ["J", "I", "N", "X"],
        ["=", "|", "^", "\\"],
        ["M", "Y", "T", "H"],
        ["5", "6", "7", "8"],
        ["V", "E", "R", "B"],
        ["#", "%", "~", "_"],
    ],
];

const DEFAULT_CHEATSHEET: &str = r#"cheat sheet:
        2       |        1       |       -1       |       -2
----------------|----------------|----------------|----------------
      DUCK      |      duck      |      ([{<      |      )]}>
 MYTH FLOW GASP | myth flow gasp | 1234 .:'! ⌴0zq | 5678 ,;"? ↵9ZQ
      JINX      |      jinx      |      +-*/      |      =|^\
      VERB      |      verb      |      @$&`      |      #%~_
"#;

impl CuboardInput {
    fn new() -> Self {
        CuboardInput {
            cuboard: Cuboard::new(),
            keymap: DEFAULT_KEYMAP,
            count: None,
        }
    }

    fn complete_part(&self) -> String {
        self.cuboard
            .keys()
            .iter()
            .map(|k| self.keymap[k.0.is_shifted as usize][k.0.main as u8 as usize][k.0.num])
            .collect()
    }

    fn remain_part(&self) -> String {
        format_moves(self.cuboard.remains())
    }
}

struct CuboardInputPrinter {
    input: CuboardInput,
}

impl CuboardInputPrinter {
    fn new() -> Self {
        CuboardInputPrinter { input: CuboardInput::new() }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if self.input.count.is_none() {
            if let ResponseMessage::State { count, state: _ } = msg {
                self.input.count = Some(count);
            }
            return;
        }

        if let ResponseMessage::Moves {
            count,
            moves,
            times: _,
        } = msg
        {
            let curr_count = self.input.count.unwrap();
            let diff = {
                let delta = count.wrapping_add(curr_count.wrapping_neg());
                let delta_ = delta.wrapping_neg();
                delta.min(delta_) as usize
            };
            self.input.count = Some(count);
            if diff > 7 {
                eprintln!("unsynchronized cube movement");
            }

            for &mv in moves[..diff].iter().rev() {
                match mv {
                    Some(mv) => {
                        self.input.cuboard.input(mv);
                    }
                    None => {
                        eprintln!("unknown cube movement");
                    }
                };
            }

            const CREL: &str = "\r\x1b[2K";
            let text = self.input.complete_part();
            if text.contains('\n') {
                print!("{}{}", CREL, text);
                self.input.cuboard.finish();
            }
            print!(
                "{}\x1b[4m{}\x1b[2m{}\x1b[m",
                CREL,
                self.input.complete_part(),
                self.input.remain_part()
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }
}
