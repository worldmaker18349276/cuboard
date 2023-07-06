use crate::cuboard::{CuboardInputState, CuboardKeymap};
use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use std::error::Error;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, Write};
use std::ops::Range;
use tokio::time::{sleep, Duration};

use crate::bluetooth::gancubev2::{GanCubeV2Builder, ResponseMessage};

use crate::cuboard::{CuboardInput, DEFAULT_KEYMAP};

pub async fn cuboard_input_printer() -> Result<(), Box<dyn Error>> {
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

    let input = CuboardInput::new(DEFAULT_KEYMAP);
    println!("{}", make_cheatsheet(&DEFAULT_KEYMAP));
    println!();

    let mut printer = CuboardInputPrinter::new(stdout(), input);
    let input_handler: Box<dyn FnMut(ResponseMessage) + Send> =
        Box::new(move |msg| printer.handle_message(msg));
    let handle = gancube.register_handler(input_handler).await?;

    gancube.subscribe_response().await?;
    gancube.request_cube_state().await?;

    handle.await?;

    Ok(())
}

pub async fn cuboard_input_trainer(text_filename: String) -> Result<(), Box<dyn Error>> {
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

    let input = CuboardInput::new(DEFAULT_KEYMAP);
    println!("{}", make_cheatsheet(&DEFAULT_KEYMAP));
    println!();

    let text = BufReader::new(File::open(text_filename)?)
        .lines()
        .map_while(|l| l.ok());
    let mut trainer = CuboardInputTrainer::new(stdout(), input, text, 3);
    let input_handler: Box<dyn FnMut(ResponseMessage) + Send> =
        Box::new(move |msg| trainer.handle_message(msg));
    let handle = gancube.register_handler(input_handler).await?;

    gancube.subscribe_response().await?;
    gancube.request_cube_state().await?;

    handle.await?;

    Ok(())
}

fn make_cheatsheet(keymap: &CuboardKeymap) -> String {
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
    use crate::cube::CubeMove::*;
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

struct CuboardInputPrinter<F: Write> {
    terminal: F,
    input: CuboardInput,
}

impl<F: Write> CuboardInputPrinter<F> {
    fn new(terminal: F, input: CuboardInput) -> Self {
        CuboardInputPrinter { terminal, input }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if matches!(msg, ResponseMessage::Disconnect) {
            let _ = writeln!(self.terminal);
            return;
        }

        match self.input.handle_message(msg) {
            CuboardInputState::Uninit => {}
            CuboardInputState::Init => {
                let _ = write!(self.terminal, "\r\x1b[7m \x1b[m\n\x1b[100m\x1b[2K\x1b[m");
                let _ = self.terminal.flush();
            }
            CuboardInputState::None | CuboardInputState::Input { accept: _, skip: _ } => {
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

                show_input_prompt(&mut self.terminal, &self.input, Self::INPUT_PROMPT_WIDTH);
            }
        }
    }

    const INPUT_PROMPT_WIDTH: usize = 12;
}

fn show_input_prompt<F: Write>(terminal: &mut F, input: &CuboardInput, width: usize) {
    let complete_part = input.complete_part();
    let remain_part = input.remain_part();

    let complete_range = 0..complete_part.len();
    let remain_range = complete_part.len()..complete_part.len() + remain_part.len();
    let total = complete_part + &remain_part;
    let mut visible_range = total.len().saturating_sub(width)..total.len();
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
        terminal,
        "\r\x1b[100m\x1b[2K{}\x1b[4m{}\x1b[2m{}\x1b[m",
        overflow, &total[complete_range], &total[remain_range],
    );
    let _ = terminal.flush();
}

struct CuboardInputTrainer<F: Write, T: Iterator<Item = String>> {
    terminal: F,
    input: CuboardInput,
    text: T,
    lines: Box<[String]>,
}

impl<F: Write, T: Iterator<Item = String>> CuboardInputTrainer<F, T> {
    fn new(terminal: F, input: CuboardInput, mut text: T, margin: usize) -> Self {
        let lines = (0..margin)
            .map(|_| text.next().unwrap_or_default())
            .collect();
        CuboardInputTrainer {
            terminal,
            input,
            text,
            lines,
        }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if matches!(msg, ResponseMessage::Disconnect) {
            let _ = writeln!(self.terminal);
            return;
        }

        match self.input.handle_message(msg) {
            CuboardInputState::Uninit => {}
            CuboardInputState::Init => {
                let cursor = self.lines[0].chars().next().unwrap_or(' ');
                let _ = write!(self.terminal, "\x1b[2m{}\x1b[m", self.lines[0]);
                let _ = write!(self.terminal, "\r\x1b[7m{}\x1b[m\n", cursor);
                for line in self.lines.iter().skip(1) {
                    let _ = writeln!(self.terminal, "\x1b[2m{}\x1b[m", line);
                }
                let _ = write!(self.terminal, "\r\x1b[100m\x1b[2K \x1b[m\r");
                let _ = self.terminal.flush();
            }
            CuboardInputState::None | CuboardInputState::Input { accept: _, skip: _ } => {
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

                show_input_prompt(&mut self.terminal, &self.input, Self::INPUT_PROMPT_WIDTH);
            }
        }
    }

    const INPUT_PROMPT_WIDTH: usize = 12;
}
