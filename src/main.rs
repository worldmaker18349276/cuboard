use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use cube::format_moves;
use std::error::Error;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, Write};
use std::ops::Range;
use tokio::time::{sleep, Duration};

use bluetooth::gancubev2::{GanCubeV2Builder, ResponseMessage};

use crate::cuboard::CuboardBuffer;

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
    print!("scan devices");

    let builder = 'a: loop {
        print!(".");
        let _ = stdout().flush();

        let found = GanCubeV2Builder::find_gancube_device(&adapter).await?;
        if let Some(builder) = found.into_iter().next() {
            break 'a builder;
        }

        sleep(Duration::from_secs(1)).await;
    };
    println!();

    adapter.stop_scan().await?;

    println!("connect to GANCube...");
    let gancube = builder.connect().await?;
    println!("connected! have fun~");
    println!();

    println!("{}", make_cheatsheet(DEFAULT_KEYMAP));
    println!();

    let input = CuboardInput::new(DEFAULT_KEYMAP);
    let input_handler: Box<dyn FnMut(ResponseMessage) + Send> =
        if let Some(text_filename) = text_filename {
            let text = BufReader::new(File::open(text_filename)?)
                .lines()
                .map(|l| l.unwrap());
            let mut trainer = CuboardInputTrainer::new(stdout(), input, text, 3);
            Box::new(move |msg| trainer.handle_message(msg))
        } else {
            let mut printer = CuboardInputPrinter::new(stdout(), input);
            Box::new(move |msg| printer.handle_message(msg))
        };
    let handle = gancube.register_handler(input_handler).await?;

    gancube.subscribe_response().await?;
    gancube.request_cube_state().await?;

    handle.await?;

    Ok(())
}

fn make_cheatsheet(keymap: CuboardKeymap) -> String {
    const STYLED_TEMPLATE: &str = "
     \x1b[30;44m  {B.3}  \x1b[m     
     \x1b[30;44m{B.2}   {B.0}\x1b[m     
     \x1b[30;44m  {B.1}  \x1b[m     
     \x1b[30;47m  {U.1}  \x1b[m     
     \x1b[30;47m{U.0}   {U.2}\x1b[m     
     \x1b[30;47m  {U.3}  \x1b[m     
\x1b[30;45m  {L.3}  \x1b[42m  {F.0}  \x1b[41m  {R.2}  \x1b[m
\x1b[30;45m{L.2}   {L.0}\x1b[42m{F.3}   {F.1}\x1b[41m{R.1}   {R.3}\x1b[m
\x1b[30;45m  {L.1}  \x1b[42m  {F.2}  \x1b[41m  {R.0}  \x1b[m
     \x1b[30;43m  {D.2}  \x1b[m     
     \x1b[30;43m{D.1}   {D.3}\x1b[m     
     \x1b[30;43m  {D.0}  \x1b[m     
";
    const STYLED_TEMPLATE_BAR: &str = "CHEAT SHEET:
     double     |      single     |     single      |     double
    clockwise   |     clockwise   |counter-clockwise|counter-clockwise
----------------|-----------------|-----------------|-----------------
";
    use cube::CubeMove::*;
    let mut a = STYLED_TEMPLATE.to_string();
    let mut b = STYLED_TEMPLATE.to_string();
    let mut c = STYLED_TEMPLATE.to_string();
    let mut d = STYLED_TEMPLATE.to_string();

    for side in [U, D, F, B, L, R] {
        for i in 0..4 {
            fn f(s: &str) -> String {
                s.replace('\n', "↵").replace(' ', "⌴")
            }
            let name = format!("{{{}.{}}}", &side.to_string(), i);
            a = a.replace(&name, &f(keymap[1][side as u8 as usize][i]));
            b = b.replace(&name, &f(keymap[0][side as u8 as usize][i]));
            c = c.replace(&name, &f(keymap[0][side.rev() as u8 as usize][i]));
            d = d.replace(&name, &f(keymap[1][side.rev() as u8 as usize][i]));
        }
    }

    let a = a.trim_matches('\n').split('\n');
    let b = b.trim_matches('\n').split('\n');
    let c = c.trim_matches('\n').split('\n');
    let d = d.trim_matches('\n').split('\n');
    STYLED_TEMPLATE_BAR.to_string()
        + &a.zip(b)
            .zip(c)
            .zip(d)
            .map(|(((a, b), c), d)| [a, b, c, d].join(" | "))
            .collect::<Vec<_>>()
            .join("\n")
}

struct CuboardInput {
    buffer: CuboardBuffer,
    keymap: CuboardKeymap,
    count: Option<u8>,
}

type CuboardKeymap = [[[&'static str; 4]; 12]; 2];

const DEFAULT_KEYMAP: CuboardKeymap = [
    [
        ["d", "u", "c", "k"], // U
        ["(", "[", "{", "<"], // U'
        ["g", "a", "s", "p"], // R
        ["0", " ", "z", "q"], // R'
        ["f", "l", "o", "w"], // F
        ["'", ".", ":", "!"], // F'
        ["j", "i", "n", "x"], // D
        ["+", "-", "*", "/"], // D'
        ["m", "y", "t", "h"], // L
        ["1", "2", "3", "4"], // L'
        ["v", "e", "r", "b"], // B
        ["#", "~", "&", "_"], // B'
    ],
    [
        ["D", "U", "C", "K"],  // U
        [")", "]", "}", ">"],  // U'
        ["G", "A", "S", "P"],  // R
        ["9", "\n", "Z", "Q"], // R'
        ["F", "L", "O", "W"],  // F
        ["\"", ",", ";", "?"], // F'
        ["J", "I", "N", "X"],  // D
        ["=", "|", "^", "\\"], // D'
        ["M", "Y", "T", "H"],  // L
        ["5", "6", "7", "8"],  // L'
        ["V", "E", "R", "B"],  // B
        ["@", "$", "%", "`"],  // B'
    ],
];

impl CuboardInput {
    fn new(keymap: CuboardKeymap) -> Self {
        CuboardInput {
            buffer: CuboardBuffer::new(),
            keymap,
            count: None,
        }
    }

    fn text(&self) -> String {
        self.buffer
            .keys()
            .iter()
            .map(|k| self.keymap[k.0.is_shifted as usize][k.0.main as u8 as usize][k.0.num])
            .collect()
    }

    fn complete_part(&self) -> String {
        let moves = self.buffer.moves();
        let complete = &moves[..moves.len() - self.buffer.remains().len()];
        format_moves(complete)
    }

    fn remain_part(&self) -> String {
        format_moves(self.buffer.remains())
    }
}

struct CuboardInputPrinter<F: Write> {
    terminal: F,
    input: CuboardInput,
}

impl<F: Write> CuboardInputPrinter<F> {
    fn new(terminal: F, input: CuboardInput) -> Self {
        CuboardInputPrinter { terminal, input }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        // ignore messages until the current count is known
        if self.input.count.is_none() {
            if let ResponseMessage::State { count, state: _ } = msg {
                self.input.count = Some(count);
                let _ = write!(self.terminal, "\r\x1b[7m \x1b[m\n\x1b[100m\x1b[2K\x1b[m");
                let _ = self.terminal.flush();
            }
            return;
        }

        let (count, moves) = match msg {
            ResponseMessage::Moves {
                count,
                moves,
                times: _,
            } => (count, moves),
            ResponseMessage::Disconnect => {
                let _ = writeln!(self.terminal);
                return;
            }
            _ => {
                return;
            }
        };

        let prev_count = self.input.count.unwrap();
        self.input.count = Some(count);

        let diff = count.wrapping_sub(prev_count).clamp(0, 7) as usize;
        for &mv in moves[..diff].iter().rev() {
            if let Some(mv) = mv {
                self.input.buffer.input(mv);
            }
        }

        let text = self.input.text();
        let _ = write!(
            self.terminal,
            "\x1b[A\r\x1b[2K{}\x1b[K\x1b[0;7m \x1b[m\n",
            text
        );
        if text.contains('\n') {
            assert!(!text[..text.len() - 1].contains('\n'));
            self.input.buffer.finish();
        }

        let complete_part = self.input.complete_part();
        let remain_part = self.input.remain_part();
        const MAX_LEN: usize = 12;

        let complete_range = 0..complete_part.len();
        let remain_range = complete_part.len()..complete_part.len() + remain_part.len();
        let total = complete_part + &remain_part;
        let mut visible_range = total.len().saturating_sub(MAX_LEN)..total.len();
        if visible_range.start > 0 {
            // remain space for overflow symbol
            visible_range.start += 1;
        }
        let visible_range = visible_range;

        fn clamp(range1: &Range<usize>, range2: &Range<usize>) -> Range<usize> {
            range1.start.clamp(range2.start, range2.end)..range1.end.clamp(range2.start, range2.end)
        }
        let complete_range = clamp(&complete_range, &visible_range);
        let remain_range = clamp(&remain_range, &visible_range);
        let overflow = if visible_range.start > 0 { "…" } else { "" };

        let _ = write!(
            self.terminal,
            "\r\x1b[100m\x1b[2K{}\x1b[4m{}\x1b[2m{}\x1b[m",
            overflow, &total[complete_range], &total[remain_range],
        );
        let _ = self.terminal.flush();
    }
}

struct CuboardInputTrainer<F: Write, T: Iterator<Item = String>> {
    terminal: F,
    input: CuboardInput,
    text: T,
    lines: Box<[String]>,
}

impl<F: Write, T: Iterator<Item = String>> CuboardInputTrainer<F, T> {
    fn new(terminal: F, input: CuboardInput, mut text: T, margin: usize) -> Self {
        let mut lines = Vec::new();
        for _ in 0..margin {
            lines.push(text.next().unwrap_or_default())
        }
        let lines = lines.try_into().unwrap();
        CuboardInputTrainer {
            terminal,
            input,
            text,
            lines,
        }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if self.input.count.is_none() {
            if let ResponseMessage::State { count, state: _ } = msg {
                self.input.count = Some(count);

                let cursor = self.lines[0].chars().next().unwrap_or(' ');
                let _ = write!(self.terminal, "\x1b[2m{}\x1b[m", self.lines[0]);
                let _ = write!(self.terminal, "\r\x1b[7m{}\x1b[m\n", cursor);
                for line in self.lines.iter().skip(1) {
                    let _ = writeln!(self.terminal, "\x1b[2m{}\x1b[m", line);
                }
                let _ = write!(self.terminal, "\r\x1b[100m\x1b[2K \x1b[m\r");
                let _ = self.terminal.flush();
            }

            return;
        }

        let (count, moves) = match msg {
            ResponseMessage::Moves {
                count,
                moves,
                times: _,
            } => (count, moves),
            ResponseMessage::Disconnect => {
                let _ = writeln!(self.terminal);
                return;
            }
            _ => {
                return;
            }
        };

        let prev_count = self.input.count.unwrap();
        self.input.count = Some(count);

        let diff = count.wrapping_sub(prev_count).clamp(0, 7) as usize;
        for &mv in moves[..diff].iter().rev() {
            if let Some(mv) = mv {
                self.input.buffer.input(mv);
            }
        }

        let _ = write!(self.terminal, "\x1b[{}A", self.lines.len());
        for line in self.lines.iter() {
            let _ = writeln!(self.terminal, "\r\x1b[2m\x1b[2K{}\x1b[m", line);
        }

        let text = self.input.text();
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

        let _ = write!(self.terminal, "\x1b[{}A", self.lines.len());
        if text.contains('\n') {
            let cursor = self.lines[1].chars().next().unwrap_or(' ');
            let _ = write!(
                self.terminal,
                "\r{}\n\x1b[7m{}\x1b[m",
                decoreated_text, cursor
            );
        } else {
            let cursor = self.lines[0].chars().nth(text.len()).unwrap_or(' ');
            let _ = write!(
                self.terminal,
                "\r{}\x1b[7m{}\x1b[m\n",
                decoreated_text, cursor
            );
        }
        let _ = write!(self.terminal, "\x1b[{}B\r", self.lines.len() - 1);

        if text.contains('\n') {
            assert!(!text[..text.len() - 1].contains('\n'));
            let new_line = self.text.next().unwrap_or_default();
            let _ = write!(
                self.terminal,
                "\r\x1b[m\x1b[2K\r\x1b[2m{}\x1b[m\n",
                new_line
            );
            self.input.buffer.finish();
            self.lines.rotate_left(1);
            self.lines[self.lines.len() - 1] = new_line;
        }

        let complete_part = self.input.complete_part();
        let remain_part = self.input.remain_part();
        const MAX_LEN: usize = 12;

        let complete_range = 0..complete_part.len();
        let remain_range = complete_part.len()..complete_part.len() + remain_part.len();
        let total = complete_part + &remain_part;
        let mut visible_range = total.len().saturating_sub(MAX_LEN)..total.len();
        if visible_range.start > 0 {
            // remain space for overflow symbol
            visible_range.start += 1;
        }
        let visible_range = visible_range;

        fn clamp(range1: &Range<usize>, range2: &Range<usize>) -> Range<usize> {
            range1.start.clamp(range2.start, range2.end)..range1.end.clamp(range2.start, range2.end)
        }
        let complete_range = clamp(&complete_range, &visible_range);
        let remain_range = clamp(&remain_range, &visible_range);
        let overflow = if visible_range.start > 0 { "…" } else { "" };

        let _ = write!(
            self.terminal,
            "\r\x1b[100m\x1b[2K{}\x1b[4m{}\x1b[2m{}\x1b[m",
            overflow, &total[complete_range], &total[remain_range],
        );
        let _ = self.terminal.flush();
    }
}
