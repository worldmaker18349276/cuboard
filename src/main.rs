use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use std::error::Error;
use std::fmt::Display;

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

    let cheatsheet = r#"cheat sheet:
       1       |        2       |       -1       |       -2
---------------|----------------|----------------|----------------
     duck      |      DUCK      |      ([{<      |      )]}>
myth flow gasp | MYTH FLOW GASP | 1234 .:'! ⌴0zq | 5678 ,;"? ↵9ZQ
     jinx      |      JINX      |      -~/_      |      +=\|
     verb      |      VERB      |      @$&`      |      #%*^
    "#;

    println!("{}", cheatsheet);
    println!();

    let mut cuboard = CuboardInputHandler::new();
    let handle = gancube
        .register_handler(Box::new(move |msg| cuboard.handle_message(msg)))
        .await?;

    gancube.subscribe_response().await?;
    gancube.request_cube_state().await?;

    handle.await?;

    Ok(())
}

struct CuboardInputHandler {
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
        ["-", "~", "/", "_"],
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
        ["+", "=", "\\", "|"],
        ["M", "Y", "T", "H"],
        ["5", "6", "7", "8"],
        ["V", "E", "R", "B"],
        ["#", "%", "*", "^"],
    ],
];

impl CuboardInputHandler {
    fn new() -> Self {
        CuboardInputHandler {
            cuboard: Cuboard::new(),
            keymap: DEFAULT_KEYMAP,
            count: None,
        }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if self.count.is_none() {
            if let ResponseMessage::State { count, state: _ } = msg {
                self.count = Some(count);
            }
            return;
        }

        if let ResponseMessage::Moves {
            count,
            moves,
            times: _,
        } = msg
        {
            let curr_count = self.count.unwrap();
            let diff = {
                let delta = count.wrapping_add(curr_count.wrapping_neg());
                let delta_ = delta.wrapping_neg();
                delta.min(delta_) as usize
            };
            self.count = Some(count);
            if diff > 7 {
                eprintln!("unsynchronized cube movement");
            }

            for &mv in moves[..diff].iter().rev() {
                match mv {
                    Some(mv) => {
                        self.cuboard.input(mv);
                    }
                    None => {
                        eprintln!("unknown cube movement");
                    }
                };
            }

            const CREL: &str = "\r\x1b[2K";
            let text = self.complete_part();
            if text.contains('\n') {
                print!("{}{}", CREL, text);
                self.cuboard.finish();
            }
            print!(
                "{}\x1b[4m{}\x1b[2m{}\x1b[m",
                CREL,
                self.complete_part(),
                self.remain_part()
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }
}

impl CuboardInputHandler {
    fn complete_part(&self) -> String {
        self.cuboard
            .keys()
            .iter()
            .map(|k| self.keymap[k.0.is_shifted as usize][k.0.main as u8 as usize][k.0.num])
            .collect()
    }

    fn remain_part(&self) -> String {
        self.cuboard
            .remains()
            .iter()
            .map(|mv| mv.to_string())
            .collect()
    }
}
