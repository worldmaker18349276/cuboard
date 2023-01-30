#![allow(dead_code)]

use btleplug::api::{Central, Characteristic, Peripheral, PeripheralProperties, WriteType};
use futures::StreamExt;
use thiserror;
use uuid::{uuid, Uuid};

use crate::cube::*;

pub struct GanCubeV2<P: Peripheral> {
    pub device: P,
    response: Characteristic,
    request: Characteristic,
    cipher: cipher::GanCubeV2Cipher,
}

pub struct GanCubeV2Builder<P: Peripheral> {
    pub device: P,
    pub properties: PeripheralProperties,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the GANCube device is invalid")]
    InvalidDevice(#[from] DeviceError),
    #[error("something wrong with the bluetooth connection")]
    BluetoothConnectionFail(#[from] btleplug::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("cannot find required characteristics")]
    InvaidCharacteristics,
    #[error("manufacturer data missing device identifier")]
    NoDeviceIdentifier,
    #[error("device identifier data invalid")]
    InvalidDeviceIdentifier,
}

const REQUEST_UUID: Uuid = uuid!("28be4a4a-cd67-11e9-a32f-2a2ae2dbcce4");

const RESPONSE_UUID: Uuid = uuid!("28be4cb6-cd67-11e9-a32f-2a2ae2dbcce4");

impl<P: Peripheral> GanCubeV2Builder<P> {
    pub async fn find_gancube_device<A>(adapter: &A) -> Result<Vec<Self>, Error>
    where
        A: Central<Peripheral = P>,
    {
        let mut res = vec![];
        let peripherals = adapter.peripherals().await?;
        for device in peripherals {
            let Some(properties) = device.properties().await? else { continue; };
            let Some(ref name) = properties.local_name else { continue; };
            if name.starts_with("GAN") {
                res.push(GanCubeV2Builder { device, properties });
            }
        }
        Ok(res)
    }

    pub async fn connect(&self) -> Result<GanCubeV2<P>, Error> {
        if !self.device.is_connected().await? {
            self.device.connect().await?;
        }
        self.device.discover_services().await?;

        let (response, request) = discover_characteristics(&self.device)?;
        let cipher = cipher::GanCubeV2Cipher::make_cipher(&self.properties)?;
        return Ok(GanCubeV2 {
            device: self.device.clone(),
            response,
            request,
            cipher,
        });

        fn discover_characteristics(
            device: &impl Peripheral,
        ) -> Result<(Characteristic, Characteristic), DeviceError> {
            let chars = device.characteristics();

            let mut response = None;
            let mut request = None;

            for cmd_char in chars {
                if cmd_char.uuid == REQUEST_UUID {
                    request = Some(cmd_char);
                } else if cmd_char.uuid == RESPONSE_UUID {
                    response = Some(cmd_char);
                }
            }

            if response.is_none() || request.is_none() {
                return Err(DeviceError::InvaidCharacteristics);
            }

            Ok((response.unwrap(), request.unwrap()))
        }
    }
}

impl<P: Peripheral> GanCubeV2<P> {
    pub async fn disconnect(&self) -> Result<(), btleplug::Error> {
        self.device.disconnect().await
    }

    pub async fn register_handler(
        &self,
        mut handler: Box<dyn FnMut(codec::ResponseMessage) + Send>,
    ) -> Result<tokio::task::JoinHandle<()>, btleplug::Error> {
        let mut notifications = self.device.notifications().await?;
        let cipher = self.cipher.clone();
        Ok(tokio::spawn(async move {
            loop {
                let Some(notification) = notifications.next().await else {
                    continue;
                };

                if notification.uuid != RESPONSE_UUID {
                    continue;
                }

                let message = match codec::ResponseMessage::decode(&notification.value, &cipher) {
                    Ok(message) => message,
                    Err(err) => {
                        eprintln!("{}", err);
                        continue;
                    }
                };

                handler(message);
            }
        }))
    }

    pub async fn subscribe_response(&self) -> Result<(), btleplug::Error> {
        self.device.subscribe(&self.response).await
    }

    pub async fn unsubscribe_response(&self) -> Result<(), btleplug::Error> {
        self.device.unsubscribe(&self.response).await
    }

    pub async fn request_battery_state(&self) -> Result<(), Error> {
        let message = codec::RequestMessage::RequestBatteryState.encode(&self.cipher);
        self.device
            .write(&self.request, &message, WriteType::WithResponse)
            .await?;
        Ok(())
    }

    pub async fn request_cube_state(&self) -> Result<(), Error> {
        let message = codec::RequestMessage::RequestCubeState.encode(&self.cipher);
        self.device
            .write(&self.request, &message, WriteType::WithResponse)
            .await?;
        Ok(())
    }

    pub async fn reset_cube_state(&self, state: CubeStateRaw) -> Result<(), Error> {
        let message = codec::RequestMessage::ResetCubeState(state).encode(&self.cipher);
        self.device
            .write(&self.request, &message, WriteType::WithResponse)
            .await?;

        Ok(())
    }
}

mod codec {
    use std::io::prelude::*;
    use thiserror::Error;

    use super::{
        cipher::GanCubeV2Cipher,
        util::{Biter, BiterMut},
    };
    use crate::cube::*;

    #[derive(Debug, Error)]
    pub enum MessageParseError {
        #[error("bad message length: {0}")]
        BadMessageLength(usize),
        #[error("unrecognized message: {0:02X?}")]
        UnrecognizedMessage([u8; 20]),
    }

    #[rustfmt::skip]
    #[repr(u8)]
    enum ResponseMessageType {
        Gyroscope    = 0b0001,
        CubeMoves    = 0b0010,
        CubeState    = 0b0100,
        BatteryState = 0b1001,
    }

    impl TryFrom<u8> for ResponseMessageType {
        type Error = ();

        fn try_from(value: u8) -> Result<Self, Self::Error> {
            match value {
                0b0001 => Ok(ResponseMessageType::Gyroscope),
                0b0010 => Ok(ResponseMessageType::CubeMoves),
                0b0100 => Ok(ResponseMessageType::CubeState),
                0b1001 => Ok(ResponseMessageType::BatteryState),
                _ => Err(()),
            }
        }
    }

    pub enum ResponseMessage {
        Gyroscope {
            q1: (f32, f32, f32, f32),
            q1p: (f32, f32, f32),
            q2: (f32, f32, f32, f32),
            q2p: (f32, f32, f32),
        },
        Moves {
            count: u8,
            moves: [Option<CubeMove>; 7],
            times: [u32; 7],
        },
        State {
            count: u8,
            state: CubeStateRaw,
        },
        Battery {
            charging: bool,
            percentage: u32,
        },
    }

    impl ResponseMessage {
        pub fn decode(data: &[u8], cipher: &GanCubeV2Cipher) -> Result<Self, MessageParseError> {
            let Ok(mut data): Result<[u8; 20], _> = data.try_into() else {
                return Err(MessageParseError::BadMessageLength(data.len()));
            };

            cipher.decrypt(&mut data);

            let mut biter = Biter::new(&data);

            let Ok(message_type) = ResponseMessageType::try_from(biter.extract(4) as u8) else {
                return Err(MessageParseError::UnrecognizedMessage(data));
            };

            let message = match message_type {
                ResponseMessageType::Gyroscope => decode_gyroscope(&mut biter),
                ResponseMessageType::CubeMoves => decode_cube_moves(&mut biter),
                ResponseMessageType::CubeState => decode_cube_state(&mut biter, &data),
                ResponseMessageType::BatteryState => decode_battery_state(&mut biter, &data),
            };

            return Ok(message);

            fn decode_gyroscope(biter: &mut Biter) -> ResponseMessage {
                fn from_signed_u3(e: u32) -> f32 {
                    const MAGNITUDE: f32 = (1 << 3) as f32;
                    const MASK: u8 = 0b0111;
                    let e = e as u8;
                    let sign = if e & !MASK != 0 { -1 } else { 1 };
                    let val = ((e & MASK) as i8) * sign;
                    val as f32 / MAGNITUDE
                }

                fn from_signed_u15(e: u32) -> f32 {
                    const MAGNITUDE: f32 = (1 << 15) as f32;
                    const MASK: u16 = 0b0111_1111_1111_1111;
                    let e = e as u16;
                    let sign = if e & !MASK != 0 { -1 } else { 1 };
                    let val = ((e & MASK) as i16) * sign;
                    val as f32 / MAGNITUDE
                }

                let scalar = from_signed_u15(biter.extract(16));
                let red = from_signed_u15(biter.extract(16));
                let blue = from_signed_u15(biter.extract(16));
                let white = from_signed_u15(biter.extract(16));
                let redp = from_signed_u3(biter.extract(4));
                let bluep = from_signed_u3(biter.extract(4));
                let whitep = from_signed_u3(biter.extract(4));
                let q1 = (scalar, red, blue, white);
                let q1p = (redp, bluep, whitep);

                let scalar_ = from_signed_u15(biter.extract(16));
                let red_ = from_signed_u15(biter.extract(16));
                let blue_ = from_signed_u15(biter.extract(16));
                let white_ = from_signed_u15(biter.extract(16));
                let redp_ = from_signed_u3(biter.extract(4));
                let bluep_ = from_signed_u3(biter.extract(4));
                let whitep_ = from_signed_u3(biter.extract(4));
                let q2 = (scalar_, red_, blue_, white_);
                let q2p = (redp_, bluep_, whitep_);

                let remains = biter.extract(4);
                if remains != 0b1010 {
                    eprintln!("bad remains data, possibly broken: {:1X}", remains);
                }

                ResponseMessage::Gyroscope { q1, q1p, q2, q2p }
            }

            fn decode_cube_moves(biter: &mut Biter) -> ResponseMessage {
                let count = biter.extract(8) as u8;
                let mut moves = [None; 7];
                for mv in moves.iter_mut() {
                    *mv = CubeMove::try_from(biter.extract(5) as u8).ok();
                }
                let mut times = [0; 7];
                for time in times.iter_mut() {
                    *time = biter.extract(16);
                }

                let remains = biter.extract(1);
                if remains != 0 {
                    eprintln!("bad remains data, possibly broken: {:1X}", remains);
                }

                ResponseMessage::Moves {
                    count,
                    moves,
                    times,
                }
            }

            fn decode_cube_state(biter: &mut Biter, data: &[u8; 20]) -> ResponseMessage {
                let count = biter.extract(8) as u8;

                let mut state = CubeStateRaw::default();

                for i in 0..7 {
                    state.corners_position[i] = biter.extract(3) as u8;
                }
                state.corners_position[7] = (0..8)
                    .into_iter()
                    .find(|a| !state.corners_position[..7].contains(a))
                    .unwrap();

                for i in 0..7 {
                    state.corners_orientation[i] = biter.extract(2) as u8;
                }
                state.corners_orientation[7] =
                    (3 - state.corners_orientation[..7].iter().sum::<u8>() % 3) % 3;

                for i in 0..11 {
                    state.edges_position[i] = biter.extract(4) as u8;
                }
                state.edges_position[11] = (0..12)
                    .into_iter()
                    .find(|a| !state.edges_position[..11].contains(a))
                    .unwrap();

                for i in 0..11 {
                    state.edges_orientation[i] = biter.extract(1) as u8;
                }
                state.edges_orientation[11] =
                    (2 - state.edges_orientation[..11].iter().sum::<u8>() % 2) % 2;

                let _unknown = biter.extract(10);

                let remains = &data[14..];
                if remains != [0; 6] {
                    eprintln!("bad remains data, possibly broken: {:02X?}", remains);
                }

                ResponseMessage::State { count, state }
            }

            fn decode_battery_state(biter: &mut Biter, data: &[u8; 20]) -> ResponseMessage {
                let charging = biter.extract(4) != 0;
                let percentage = biter.extract(8);
                let remains = &data[2..];
                if remains != [0; 18] {
                    eprintln!("bad remains data, possibly broken: {:02X?}", remains);
                }

                ResponseMessage::Battery {
                    charging,
                    percentage,
                }
            }
        }

        pub fn show(self) {
            match self {
                ResponseMessage::Gyroscope { q1, q1p, q2, q2p } => show_gyroscope(q1, q1p, q2, q2p),
                ResponseMessage::Moves {
                    count,
                    moves,
                    times,
                } => show_moves(count, moves, times),
                ResponseMessage::State { count, state } => show_cube_state(count, state),
                ResponseMessage::Battery {
                    charging,
                    percentage,
                } => show_battery_state(charging, percentage),
            }

            const CREL: &str = "\r\x1b[2K";

            fn draw_bar(value: f32, width: usize) -> String {
                const TEMP: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
                let n = (value * width as f32 * 8.0) as usize;
                (0..width)
                    .map(|i| TEMP[n.clamp(8 * i, 8 * (i + 1)) - 8 * i])
                    .collect()
            }

            fn show_gyroscope(
                q1: (f32, f32, f32, f32),
                q1p: (f32, f32, f32),
                q2: (f32, f32, f32, f32),
                q2p: (f32, f32, f32),
            ) {
                const BAR_WIDTH: usize = 12;
                const PBAR_WIDTH: usize = 2;

                print!("{}gyroscope: ", CREL);
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
                    "q'=[\x1b[31m{}\x1b[34m{}\x1b[37m{}\x1b[m]",
                    bpbar_, cpbar_, dpbar_
                );
                let _ = std::io::stdout().flush();
            }

            fn show_moves(move_count: u8, moves: [Option<CubeMove>; 7], move_times: [u32; 7]) {
                print!("{}", CREL);
                print!("count={:3}, ", move_count);
                print!("({:.3} s) ", move_times[0] as f32 / 1000.0);
                for mv in moves {
                    print!("{} ", mv.map_or("??".to_owned(), |m| m.to_string()));
                }
                println!();
            }

            fn show_cube_state(move_count: u8, state: CubeStateRaw) {
                print!("{}", CREL);
                print!("count={:3}, ", move_count);

                let Ok(state): Result<CubeState, _> = state.try_into() else {
                    println!("invalid state");
                    return;
                };

                print!(
                    "corners=[{}], ",
                    state
                        .corners
                        .iter()
                        .map(|c| c.show())
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                println!(
                    "edges=[{}]",
                    state
                        .edges
                        .iter()
                        .map(|c| c.show())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            fn show_battery_state(battery_charging: bool, battery_percentage: u32) {
                print!("{}", CREL);
                print!("battery={}%", battery_percentage);
                if battery_charging {
                    print!(" (charging)");
                }
                println!();
            }
        }
    }

    #[allow(clippy::enum_variant_names)]
    #[rustfmt::skip]
    #[repr(u8)]
    enum RequestMessageType {
        RequestCubeState    = 0b_0000_0100,
        RequestBatteryState = 0b_0000_1001,
        ResetCubeState      = 0b_0000_1010,
    }

    #[allow(clippy::enum_variant_names)]
    pub enum RequestMessage {
        RequestCubeState,
        RequestBatteryState,
        ResetCubeState(CubeStateRaw),
    }

    impl RequestMessage {
        pub fn encode(&self, cipher: &GanCubeV2Cipher) -> [u8; 20] {
            let mut message = [0; 20];
            let mut biter = BiterMut::new(&mut message);

            match self {
                RequestMessage::RequestCubeState => {
                    biter.assign(8, RequestMessageType::RequestCubeState as u8 as u32);
                }
                RequestMessage::RequestBatteryState => {
                    biter.assign(8, RequestMessageType::RequestBatteryState as u8 as u32);
                }
                RequestMessage::ResetCubeState(state) => {
                    biter.assign(8, RequestMessageType::ResetCubeState as u8 as u32);
                    for val in state.corners_position {
                        biter.assign(3, val as u32);
                    }
                    for val in state.corners_orientation {
                        biter.assign(2, val as u32);
                    }
                    for val in state.edges_position {
                        biter.assign(4, val as u32);
                    }
                    for val in state.edges_orientation {
                        biter.assign(1, val as u32);
                    }
                }
            }

            cipher.encrypt(&mut message);
            message
        }
    }
}

pub use codec::ResponseMessage;

mod cipher {
    use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
    use aes::{Aes128, Block};
    use btleplug::api::PeripheralProperties;

    use super::DeviceError;

    #[derive(Clone)]
    pub struct GanCubeV2Cipher {
        key: Block,
        iv: Block,
        aes: Aes128,
    }

    impl GanCubeV2Cipher {
        pub(super) fn make_cipher(
            device_props: &PeripheralProperties,
        ) -> Result<Self, DeviceError> {
            let Some(manufacturer_data) = device_props.manufacturer_data.get(&1) else {
                return Err(DeviceError::NoDeviceIdentifier);
            };
            let Ok(device_id): Result<&[u8; 9], _> = manufacturer_data.as_slice().try_into() else {
                return Err(DeviceError::InvalidDeviceIdentifier);
            };

            let device_key: [u8; 6] = device_id[3..9].try_into().unwrap();
            let mut key = [
                0x01, 0x02, 0x42, 0x28, 0x31, 0x91, 0x16, 0x07, 0x20, 0x05, 0x18, 0x54, 0x42, 0x11,
                0x12, 0x53,
            ];
            let mut iv = [
                0x11, 0x03, 0x32, 0x28, 0x21, 0x01, 0x76, 0x27, 0x20, 0x95, 0x78, 0x14, 0x32, 0x12,
                0x02, 0x43,
            ];

            key.iter_mut()
                .zip(device_key.into_iter())
                .for_each(|(a, b)| *a = ((*a as u16 + b as u16) % 255) as u8);
            iv.iter_mut()
                .zip(device_key.into_iter())
                .for_each(|(a, b)| *a = ((*a as u16 + b as u16) % 255) as u8);

            let key: Block = GenericArray::clone_from_slice(&key);
            let iv: Block = GenericArray::clone_from_slice(&iv);
            let aes = Aes128::new(&key);
            Ok(GanCubeV2Cipher { key, iv, aes })
        }

        pub(super) fn encrypt(&self, value: &mut [u8; 20]) {
            fn encrypt_block(cipher: &GanCubeV2Cipher, block: &mut [u8]) {
                let block = GenericArray::from_mut_slice(block);
                block
                    .iter_mut()
                    .zip(cipher.iv.into_iter())
                    .for_each(|(a, b)| *a ^= b);
                cipher.aes.encrypt_block(block);
            }

            encrypt_block(self, &mut value[..16]);
            let offset = value.len() - 16;
            encrypt_block(self, &mut value[offset..]);
        }

        pub(super) fn decrypt(&self, value: &mut [u8; 20]) {
            fn decrypt_block(cipher: &GanCubeV2Cipher, block: &mut [u8]) {
                let block = GenericArray::from_mut_slice(block);
                cipher.aes.decrypt_block(block);
                block
                    .iter_mut()
                    .zip(cipher.iv.into_iter())
                    .for_each(|(a, b)| *a ^= b);
            }

            let offset = value.len() - 16;
            decrypt_block(self, &mut value[offset..]);
            decrypt_block(self, &mut value[..16]);
        }
    }
}

mod util {
    // Bit iterator
    pub struct Biter<'a> {
        data: &'a [u8],
        index: usize,
    }

    pub struct BiterMut<'a> {
        data: &'a mut [u8],
        index: usize,
    }

    impl<'a> Biter<'a> {
        pub fn new(data: &'a [u8]) -> Self {
            Biter { data, index: 0 }
        }

        pub fn reset(&mut self) {
            self.index = 0;
        }

        pub fn skip(&mut self, count: usize) {
            self.index += count;
        }

        pub fn extract(&mut self, count: usize) -> u32 {
            let mut result = 0;
            for bit in (self.index..).take(count) {
                result <<= 1;
                if self.data[bit / 8] & (1 << (7 - (bit % 8))) != 0 {
                    result |= 1;
                }
            }
            self.index += count;
            result
        }
    }

    impl<'a> BiterMut<'a> {
        pub fn new(data: &'a mut [u8]) -> Self {
            BiterMut { data, index: 0 }
        }

        pub fn reset(&mut self) {
            self.index = 0;
        }

        pub fn skip(&mut self, count: usize) {
            self.index += count;
        }

        pub fn extract(&mut self, count: usize) -> u32 {
            let mut result = 0;
            for bit in (self.index..).take(count) {
                result <<= 1;
                if self.data[bit / 8] & (1 << (7 - (bit % 8))) != 0 {
                    result |= 1;
                }
            }
            self.index += count;
            result
        }
        pub fn assign(&mut self, count: usize, value: u32) {
            for (bit, i) in (self.index..).take(count).zip((0..count).rev()) {
                if value & (1 << i) != 0 {
                    self.data[bit / 8] |= 1 << (7 - (bit % 8));
                }
            }
            self.index += count;
        }
    }
}
