use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use cube::format_moves;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

use bluetooth::gancubev2::{GanCubeV2Builder, ResponseMessage};

use crate::cuboard::Cuboard;

mod bluetooth;
mod cube;
mod cuboard;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let text_filename: Option<String> = args.get(1).cloned();

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

    let input_handler: Box<dyn FnMut(ResponseMessage) + Send> =
        if let Some(text_filename) = text_filename {
            let text = BufReader::new(File::open(text_filename)?)
                .lines()
                .map(|l| l.unwrap());
            let mut trainer = CuboardInputTrainer::<_, 3>::new(CuboardInput::new(), text);
            Box::new(move |msg| trainer.handle_message(msg))
        } else {
            let mut printer = CuboardInputPrinter::new(CuboardInput::new());
            Box::new(move |msg| printer.handle_message(msg))
        };
    let handle = gancube.register_handler(input_handler).await?;

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
    fn new(input: CuboardInput) -> Self {
        CuboardInputPrinter { input }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if self.input.count.is_none() {
            if let ResponseMessage::State { count, state: _ } = msg {
                self.input.count = Some(count);
            }
            print!("\x1b[100m\r\x1b[2K\r");
            let _ = std::io::Write::flush(&mut std::io::stdout());
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

            let text = self.input.complete_part();
            if text.contains('\n') {
                print!("\r\x1b[2K{}", text);
                self.input.cuboard.finish();
            }
            print!(
                "\r\x1b[100m\x1b[2K\x1b[4m{}\x1b[2m{}\x1b[m\r",
                self.input.complete_part(),
                self.input.remain_part()
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }
}

struct CuboardInputTrainer<T: Iterator<Item = String>, const N: usize> {
    input: CuboardInput,
    text: T,
    lines: [String; N],
}

impl<T: Iterator<Item = String>, const N: usize> CuboardInputTrainer<T, N> {
    fn new(input: CuboardInput, mut text: T) -> Self {
        let mut lines = Vec::new();
        for _ in 0..N {
            lines.push(text.next().unwrap_or_default())
        }
        let lines = lines.try_into().unwrap();
        CuboardInputTrainer { input, text, lines }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if self.input.count.is_none() {
            if let ResponseMessage::State { count, state: _ } = msg {
                self.input.count = Some(count);

                let cursor = self.lines[0].chars().next().unwrap_or(' ');
                print!("\x1b[2m{}\x1b[m", self.lines[0]);
                print!("\r\x1b[7m{}\x1b[m\n", cursor);
                for line in self.lines.iter().skip(1) {
                    println!("\x1b[2m{}\x1b[m", line);
                }
                print!("\r\x1b[100m\x1b[2K \x1b[m\r");
                let _ = std::io::Write::flush(&mut std::io::stdout());
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

            for &mv in moves[..diff].iter().rev() {
                if let Some(mv) = mv {
                    self.input.cuboard.input(mv);
                }
            }

            print!("\x1b[{}A", N);
            for line in self.lines.iter() {
                println!("\r\x1b[2m\x1b[2K{}\x1b[m", line);
            }

            let text = self.input.complete_part();
            let decoreated_text: String = text
                .trim_end_matches('\n')
                .chars()
                .zip(self.lines[0].chars().chain([' '].into_iter().cycle()))
                .map(|(a, b)| {
                    if a == b {
                        format!("{}", a)
                    } else {
                        format!("\x1b[41m{}\x1b[m", a)
                    }
                })
                .collect();

            print!("\x1b[{}A", N);
            if text.contains('\n') {
                let cursor = self.lines[1].chars().next().unwrap_or(' ');
                print!("\r{}\n\x1b[7m{}\x1b[m", decoreated_text, cursor);
            } else {
                let cursor = self.lines[0].chars().nth(text.len()).unwrap_or(' ');
                print!("\r{}\x1b[7m{}\x1b[m\n", decoreated_text, cursor);
            }
            print!("\x1b[{}B\r", N - 1);

            if text.contains('\n') {
                assert!(!text[..text.len() - 1].contains('\n'));
                let new_line = self.text.next().unwrap_or_default();
                print!("\r\x1b[m\x1b[2K\r\x1b[2m{}\x1b[m\n", new_line);
                self.input.cuboard.finish();
                self.lines.rotate_left(1);
                self.lines[N - 1] = new_line;
            }

            print!(
                "\r\x1b[100m\x1b[2K\x1b[4m{}\x1b[2m{}\x1b[m\r",
                self.input.complete_part(),
                self.input.remain_part(),
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }
}
