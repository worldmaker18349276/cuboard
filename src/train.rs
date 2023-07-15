use crate::cuboard::{CuboardInputEvent, CuboardKeymap};
use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use std::error::Error;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, Write};
use std::iter::repeat;
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
    accepted_text: String,
    input: CuboardInput,
}

impl<F: Write> CuboardInputPrinter<F> {
    fn new(terminal: F, input: CuboardInput) -> Self {
        CuboardInputPrinter {
            terminal,
            accepted_text: String::new(),
            input,
        }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if matches!(msg, ResponseMessage::Disconnect) {
            let _ = writeln!(self.terminal);
            return;
        }

        match self.input.handle_message(msg) {
            Some(CuboardInputEvent::Uninit) => {
                return;
            }
            Some(CuboardInputEvent::Init) => {
                let _ = write!(self.terminal, "\r\x1b[7m \x1b[m\n\x1b[100m\x1b[2K\x1b[m");
                let _ = self.terminal.flush();
                return;
            }
            None => {}
            Some(CuboardInputEvent::Cancel) => {
                self.input.cancel();
            }
            Some(CuboardInputEvent::Finish(accept))
            | Some(CuboardInputEvent::Input { accept, skip: _ }) => {
                self.accepted_text += &accept;
            }
        }

        let buffered_text = self.input.buffered_text();
        let _ = write!(
            self.terminal,
            "\x1b[A\r\x1b[2K{}\x1b[4m{}\x1b[m\x1b[K\x1b[0;7m \x1b[m\n",
            self.accepted_text, buffered_text
        );

        if buffered_text.contains('\n') {
            self.accepted_text += &self.input.finish();
        }

        if let Some(i) = self.accepted_text.rfind('\n') {
            self.accepted_text.drain(0..=i);
        }

        show_input_prompt(&mut self.terminal, &self.input, Self::INPUT_PROMPT_WIDTH);
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
    accepted_text: String,
    input: CuboardInput,
    textgen: T,
    lines: Box<[String]>,
}

impl<F: Write, T: Iterator<Item = String>> CuboardInputTrainer<F, T> {
    fn new(terminal: F, input: CuboardInput, mut textgen: T, margin: usize) -> Self {
        let lines = (0..margin)
            .map(|_| textgen.next().unwrap_or_default())
            .collect();
        CuboardInputTrainer {
            terminal,
            accepted_text: String::new(),
            input,
            textgen,
            lines,
        }
    }

    fn handle_message(&mut self, msg: ResponseMessage) {
        if matches!(msg, ResponseMessage::Disconnect) {
            let _ = writeln!(self.terminal);
            return;
        }

        match self.input.handle_message(msg) {
            Some(CuboardInputEvent::Uninit) => {
                return;
            }
            Some(CuboardInputEvent::Init) => {
                let cursor = self.lines[0].chars().next().unwrap_or(' ');
                let _ = write!(self.terminal, "\x1b[2m{}\x1b[m", self.lines[0]);
                let _ = write!(self.terminal, "\r\x1b[7m{}\x1b[m\n", cursor);
                for line in self.lines.iter().skip(1) {
                    let _ = writeln!(self.terminal, "\x1b[2m{}\x1b[m", line);
                }
                let _ = write!(self.terminal, "\r\x1b[100m\x1b[2K \x1b[m\r");
                let _ = self.terminal.flush();
                return;
            }
            None => {}
            Some(CuboardInputEvent::Cancel) => {
                self.input.cancel();
            }
            Some(CuboardInputEvent::Finish(accept))
            | Some(CuboardInputEvent::Input { accept, skip: _ }) => {
                self.accepted_text += &accept;
            }
        }

        let _ = write!(self.terminal, "\x1b[{}A", self.lines.len());
        for line in self.lines.iter() {
            let _ = writeln!(self.terminal, "\r\x1b[2m\x1b[2K{}\x1b[m", line);
        }

        let buffered_text = self.input.buffered_text();
        let text = self.accepted_text.clone() + &buffered_text;
        let decorated_texts = text
            .split('\n')
            .zip(self.lines.iter().chain(repeat(&String::new())))
            .map(|(input, expect)| {
                input
                    .chars()
                    .zip(expect.chars().chain(repeat(' ')))
                    .map(|(a, b)| {
                        if a == b {
                            format!("{}", a)
                        } else {
                            format!("\x1b[41m{}\x1b[m", a)
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        let _ = write!(self.terminal, "\x1b[{}A", self.lines.len());
        for decorated_text in decorated_texts[..decorated_texts.len() - 1].iter() {
            let _ = write!(self.terminal, "\r{}\n", decorated_text);
        }
        let last_decorated_text = decorated_texts.last().unwrap();
        let char_on_cursor = self.lines[decorated_texts.len() - 1]
            .chars()
            .nth(text.split('\n').last().unwrap().len())
            .unwrap_or(' ');
        let _ = write!(
            self.terminal,
            "\r{}\x1b[7m{}\x1b[m\n",
            last_decorated_text, char_on_cursor
        );
        let _ = write!(
            self.terminal,
            "\x1b[{}B\r",
            self.lines.len() - decorated_texts.len()
        );

        for _ in 0..decorated_texts.len() - 1 {
            let new_line = self.textgen.next().unwrap_or_default();
            let _ = write!(
                self.terminal,
                "\r\x1b[m\x1b[2K\r\x1b[2m{}\x1b[m\n",
                new_line
            );
            self.lines.rotate_left(1);
            *self.lines.last_mut().unwrap() = new_line;
        }

        if let Some(i) = self.accepted_text.rfind('\n') {
            self.accepted_text.drain(0..=i);
        }

        show_input_prompt(&mut self.terminal, &self.input, Self::INPUT_PROMPT_WIDTH);
    }

    const INPUT_PROMPT_WIDTH: usize = 12;
}
