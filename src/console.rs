use std::{
    error::Error,
    io::{stdout, Read, Write},
    time::{Duration, Instant},
};

use btleplug::{
    api::{Central, Manager, Peripheral, ScanFilter},
    platform,
};
use tokio::time::sleep;

use crate::{
    bluetooth::gancubev2::{GanCubeV2Builder, ResponseMessage},
    cube::CubeState,
};

fn direct_input_mode() -> impl Drop {
    use defer::defer;
    use termios::{tcsetattr, Termios, ECHO, ICANON, TCSANOW};

    let stdin_no = 0;
    let termios = Termios::from_fd(stdin_no).unwrap();
    let mut new_termios = termios;
    new_termios.c_lflag &= !(ICANON | ECHO); // no echo and canonical mode
    tcsetattr(stdin_no, TCSANOW, &new_termios).unwrap();
    defer(move || tcsetattr(stdin_no, TCSANOW, &termios).unwrap())
}

fn read_char() -> std::io::Result<u8> {
    let mut stdout = std::io::stdout();
    let mut stdin = std::io::stdin();
    let mut input = [0; 1];
    stdin.read_exact(&mut input)?;
    let _ = stdout.flush();
    Ok(input[0])
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let _input_handle = direct_input_mode();

    // get the first bluetooth adapter
    let manager = platform::Manager::new().await.unwrap();
    let adapters = manager.adapters().await?;
    let adapter = adapters.into_iter().next().unwrap();
    let info = adapter.adapter_info().await?;
    println!("adapter: {}", info);

    // start scanning for devices
    adapter.start_scan(ScanFilter::default()).await?;
    print!("scan devices");

    let builder = 'a: loop {
        print!(".");
        let _ = stdout().flush();

        let found = GanCubeV2Builder::find_gancube_device(&adapter).await?;
        if found.is_empty() {
            sleep(Duration::from_secs(1)).await;
            continue;
        }

        println!();
        for builder in found.iter() {
            println!("===================================================");
            let name = builder.properties.local_name.clone().unwrap();
            println!("name: {} [{}]", name, builder.device.address());
            println!("{:#?}", builder.device);
            println!("{:#?}", builder.properties);
        }
        println!("===================================================");

        break 'a found.into_iter().next().unwrap();
    };
    println!();

    println!("connect to GANCube...");
    let gancube = builder.connect().await?;
    println!("connected! have fun~");
    println!();

    // handle notifications
    println!("Instructions:");
    println!("  q: exit");
    println!("  Q: disconnect GANCube and exit");
    println!("  s: subscribe/unsubscribe response characteristic");
    println!("  b: request battery state");
    println!("  c: request cube state");
    println!("  r: reset cube state");
    println!();

    // println!("Experimental instructions (may destroy your device):");
    // println!("  a: arbitrary request");
    // println!("  1: unkown characteristic 1");
    // println!("  2: unkown characteristic 2");
    // println!("  3: unkown characteristic 3");
    // println!("  4: unkown characteristic 4");
    // println!();

    let mut handler = ConsoleMessageHandler::new();
    gancube
        .register_handler(Box::new(move |msg| handler.handle_message(msg)))
        .await?;
    gancube.subscribe_response().await?;
    let mut is_subscribed = true;
    loop {
        match read_char()? {
            b'\n' => {
                println!("{}", CREL);
            }
            b'q' => break,
            b'Q' => {
                println!("{}disconnect GANCube...", CREL);
                gancube.device.disconnect().await?;
                break;
            }
            b's' => {
                if is_subscribed {
                    gancube.unsubscribe_response().await?;
                } else {
                    gancube.subscribe_response().await?;
                }
                is_subscribed = !is_subscribed;
                println!("{}", CREL);
            }
            b'b' => {
                gancube.request_battery_state().await?;
                println!("{}request battery state", CREL);
            }
            b'c' => {
                gancube.request_cube_state().await?;
                println!("{}request cube state", CREL);
            }
            b'r' => {
                gancube.reset_cube_state(CubeState::default()).await?;
                println!("{}reset cube state", CREL);
            }
            b'a' => {
                // 04 -> RequestCubeState
                // 09 -> RequestBatteryState
                // 0A -> ResetCubeState

                // unknown:
                // 00 -> [00, 40, ...]
                // 05 -> [50, 00, 01, 07, 4C, 47, 41, 4E, 69, 33, 79, 58, 74, 40, 00, 00, 00, 00, 00, 00]
                // 0C -> [C0, AF, 08, 32, ...]
                // 0E -> [E0, 00, ...]
                // 0F -> [F0, 00, ...]
                // 10 -> destroy GANCube...
                println!("{}input message type (02X)", CREL);
                let s = String::from_utf8([read_char()?; 40].to_vec())?;
                let message_type = u8::from_str_radix(&s, 16)?;
                let mut message = [0; 20];
                message[0] = message_type;
                gancube.arbitrary_request(message, false).await?;
                println!("{}arbitrary request <= {:02X?}", CREL, message_type);
            }
            b'1' => {
                let res = gancube.unknown1().await?;
                println!("{}unknown characteristic 1 => {:02X?}", CREL, res);
            }
            b'2' => {
                let res = gancube.unknown2().await?;
                println!("{}unknown characteristic 2 => {:02X?}", CREL, res);
            }
            b'3' => {
                let res = gancube.unknown3().await?;
                println!("{}unknown characteristic 3 => {:02X?}", CREL, res);
            }
            b'4' => {
                let data = [0; 20];
                gancube.unknown4(data).await?;
                println!("{}unknown characteristic 4 <= {:02X?}", CREL, data);
            }
            _ => {}
        }
    }

    println!();
    Ok(())
}

const CREL: &str = "\r\x1b[2K";

fn draw_bar(value: f32, width: usize) -> String {
    const TEMP: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
    let n = (value * width as f32 * 8.0) as usize;
    (0..width)
        .map(|i| TEMP[n.clamp(8 * i, 8 * (i + 1)) - 8 * i])
        .collect()
}

struct ConsoleMessageHandler {
    prev_time: Instant,
}

impl ConsoleMessageHandler {
    fn new() -> Self {
        ConsoleMessageHandler {
            prev_time: Instant::now(),
        }
    }

    fn ping(&mut self) -> Duration {
        let time = Instant::now();
        let duration = time - self.prev_time;
        self.prev_time = time;
        duration
    }

    fn handle_message(&mut self, message: ResponseMessage) {
        const BAR_WIDTH: usize = 12;
        const PBAR_WIDTH: usize = 2;

        match message {
            ResponseMessage::Gyroscope { q1, q1p, q2, q2p } => {
                let duration = self.ping().as_secs_f32();

                print!("{}<!> gyroscope: ", CREL);
                let abar = draw_bar((q1.0 + 1.0) / 2.0, BAR_WIDTH);
                let bbar = draw_bar((q1.1 + 1.0) / 2.0, BAR_WIDTH);
                let cbar = draw_bar((q1.2 + 1.0) / 2.0, BAR_WIDTH);
                let dbar = draw_bar((q1.3 + 1.0) / 2.0, BAR_WIDTH);
                let bpbar = draw_bar((q1p.0 + 1.0) / 2.0, PBAR_WIDTH);
                let cpbar = draw_bar((q1p.1 + 1.0) / 2.0, PBAR_WIDTH);
                let dpbar = draw_bar((q1p.2 + 1.0) / 2.0, PBAR_WIDTH);
                print!(
                    "q=[\x1b[2m{}\x1b[0;31m{}\x1b[34m{}\x1b[37m{}\x1b[m], ",
                    abar, bbar, cbar, dbar
                );
                print!(
                    "q'=[\x1b[31m{}\x1b[34m{}\x1b[37m{}\x1b[m]",
                    bpbar, cpbar, dpbar
                );
                print!(" ; ");

                let abar_ = draw_bar((q2.0 + 1.0) / 2.0, BAR_WIDTH);
                let bbar_ = draw_bar((q2.1 + 1.0) / 2.0, BAR_WIDTH);
                let cbar_ = draw_bar((q2.2 + 1.0) / 2.0, BAR_WIDTH);
                let dbar_ = draw_bar((q2.3 + 1.0) / 2.0, BAR_WIDTH);
                let bpbar_ = draw_bar((q2p.0 + 1.0) / 2.0, PBAR_WIDTH);
                let cpbar_ = draw_bar((q2p.1 + 1.0) / 2.0, PBAR_WIDTH);
                let dpbar_ = draw_bar((q2p.2 + 1.0) / 2.0, PBAR_WIDTH);
                print!(
                    "q=[\x1b[2m{}\x1b[0;31m{}\x1b[34m{}\x1b[37m{}\x1b[m], ",
                    abar_, bbar_, cbar_, dbar_
                );
                print!(
                    "q'=[\x1b[31m{}\x1b[34m{}\x1b[37m{}\x1b[m] ",
                    bpbar_, cpbar_, dpbar_
                );
                print!("({:0.3}s)", duration);
                let _ = std::io::stdout().flush();
            }
            ResponseMessage::Moves {
                count,
                moves,
                times,
            } => {
                print!("{}<!> ", CREL);
                print!("count={:3}, ", count);
                print!("({}) ", times[0].as_millis());
                for mv in moves {
                    print!("{} ", mv.map_or("??".to_owned(), |m| m.to_string()));
                }
                println!();
            }
            ResponseMessage::State { count, state } => {
                print!("{}<!> ", CREL);
                print!("count={:3}, ", count);
                if let Some(CubeState {
                    corners,
                    edges,
                    centers: _,
                }) = state
                {
                    print!(
                        "corners={:X?} / {:X?}, ",
                        corners.map(|c| c.0.repr()),
                        corners.map(|c| c.1.repr()),
                    );
                    print!(
                        "edges={:X?} / {:X?}, ",
                        edges.map(|e| e.0.repr()),
                        edges.map(|e| e.1.repr()),
                    );
                } else {
                    print!("<unknown state>");
                }
                println!();
            }
            ResponseMessage::Battery {
                charging,
                percentage,
            } => {
                print!("{}<!> ", CREL);
                print!("battery={}%", percentage);
                if charging {
                    print!(" (charging)");
                }
                println!();
            }
            ResponseMessage::Disconnect => {
                print!("{}<!> ", CREL);
                println!("cube auto-disconnect");
            }
        }
    }
}
