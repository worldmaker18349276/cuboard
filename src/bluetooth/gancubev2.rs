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
        let chars = self.device.characteristics();

        let Some(response) = chars.iter().find(|ch| ch.uuid == RESPONSE_UUID).cloned() else {
            return Err(DeviceError::InvaidCharacteristics.into());
        };

        let Some(request) = chars.iter().find(|ch| ch.uuid == REQUEST_UUID).cloned() else {
            return Err(DeviceError::InvaidCharacteristics.into());
        };

        let cipher = cipher::GanCubeV2Cipher::make_cipher(&self.properties)?;
        Ok(GanCubeV2 {
            device: self.device.clone(),
            response,
            request,
            cipher,
        })
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

                let is_disconnected = codec::ResponseMessage::Disconnect == message;

                handler(message);

                if is_disconnected {
                    return;
                }
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

    pub async fn reset_cube_state(&self, state: CubeState) -> Result<(), Error> {
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
        Disconnect   = 0b1101,
    }

    impl ResponseMessageType {
        pub fn repr(self) -> u8 {
            self as u8
        }
    }

    impl TryFrom<u8> for ResponseMessageType {
        type Error = ();

        fn try_from(value: u8) -> Result<Self, Self::Error> {
            match value {
                0b0001 => Ok(ResponseMessageType::Gyroscope),
                0b0010 => Ok(ResponseMessageType::CubeMoves),
                0b0100 => Ok(ResponseMessageType::CubeState),
                0b1001 => Ok(ResponseMessageType::BatteryState),
                0b1101 => Ok(ResponseMessageType::Disconnect),
                _ => Err(()),
            }
        }
    }

    type Quaternion = (f32, f32, f32, f32);
    type QuaternionP = (f32, f32, f32);

    #[derive(PartialEq, PartialOrd)]
    pub enum ResponseMessage {
        Gyroscope {
            q1: Quaternion,
            q1p: QuaternionP,
            q2: Quaternion,
            q2p: QuaternionP,
        },
        Moves {
            count: u8,
            moves: [Option<CubeMove>; 7],
            times: [u32; 7],
        },
        State {
            count: u8,
            state: CubeState,
        },
        Battery {
            charging: bool,
            percentage: u32,
        },
        Disconnect,
    }

    const CREL: &str = "\r\x1b[2K";

    impl ResponseMessage {
        pub fn decode(data: &[u8], cipher: &GanCubeV2Cipher) -> Result<Self, MessageParseError> {
            let Ok(mut data) = <[u8; 20]>::try_from(data) else {
                return Err(MessageParseError::BadMessageLength(data.len()));
            };

            cipher.decrypt(&mut data);

            let mut biter = Biter::new(&data);

            let Ok(message_type) = ResponseMessageType::try_from(biter.extract(4) as u8) else {
                return Err(MessageParseError::UnrecognizedMessage(data));
            };

            let message = match message_type {
                ResponseMessageType::Gyroscope => Self::decode_gyroscope(&mut biter),
                ResponseMessageType::CubeMoves => Self::decode_cube_moves(&mut biter),
                ResponseMessageType::CubeState => Self::decode_cube_state(&mut biter),
                ResponseMessageType::BatteryState => Self::decode_battery_state(&mut biter),
                ResponseMessageType::Disconnect => Self::decode_disconnect(&mut biter),
            };

            Ok(message)
        }

        fn decode_gyroscope(biter: &mut Biter) -> Self {
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

            Self::Gyroscope { q1, q1p, q2, q2p }
        }

        fn decode_cube_moves(biter: &mut Biter) -> Self {
            let count = biter.extract(8) as u8;
            let moves: [Option<CubeMove>; 7] = (0..7)
                .map(|_| CubeMove::from_repr(biter.extract(5) as u8))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            let times: [u32; 7] = (0..7)
                .map(|_| biter.extract(16))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            let remains = biter.extract(1);
            if remains != 0 {
                eprintln!("bad remains data, possibly broken: {:1X}", remains);
            }

            Self::Moves {
                count,
                moves,
                times,
            }
        }

        fn decode_cube_state(biter: &mut Biter) -> Self {
            let count = biter.extract(8) as u8;

            let mut corners_position = [0, 1, 2, 3, 4, 5, 6, 7];
            let mut corners_orientation = [0; 8];
            let mut edges_position = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
            let mut edges_orientation = [0; 12];

            for pos in corners_position.iter_mut().take(7) {
                *pos = biter.extract(3) as u8;
            }
            corners_position[7] = (0..8).find(|a| !corners_position[..7].contains(a)).unwrap();

            for ori in corners_orientation.iter_mut().take(7) {
                *ori = biter.extract(2) as u8;
            }
            corners_orientation[7] = (3 - corners_orientation[..7].iter().sum::<u8>() % 3) % 3;

            for pos in edges_position.iter_mut().take(11) {
                *pos = biter.extract(4) as u8;
            }
            edges_position[11] = (0..12).find(|a| !edges_position[..11].contains(a)).unwrap();

            for ori in edges_position.iter_mut().take(11) {
                *ori = biter.extract(1) as u8;
            }
            edges_orientation[11] = (2 - edges_orientation[..11].iter().sum::<u8>() % 2) % 2;

            let _unknown = biter.extract(10);

            let remains = (0..6).map(|_| biter.extract(8) as u8).collect::<Vec<_>>();
            if remains != [0; 6] {
                eprintln!("bad remains data, possibly broken: {:02X?}", remains);
            }

            let corners: [Corner; 8] = corners_position
                .into_iter()
                .zip(corners_orientation)
                .map(|v| v.try_into().unwrap())
                .collect::<Vec<Corner>>()
                .try_into()
                .unwrap();

            let edges: [Edge; 12] = edges_position
                .into_iter()
                .zip(edges_orientation)
                .map(|v| v.try_into().unwrap())
                .collect::<Vec<Edge>>()
                .try_into()
                .unwrap();

            Self::State {
                count,
                state: CubeState::new(corners, edges),
            }
        }

        fn decode_battery_state(biter: &mut Biter) -> Self {
            let charging = biter.extract(4) != 0;
            let percentage = biter.extract(8);
            let remains = (0..18).map(|_| biter.extract(8) as u8).collect::<Vec<_>>();
            if remains != [0; 18] {
                eprintln!("bad remains data, possibly broken: {:02X?}", remains);
            }

            Self::Battery {
                charging,
                percentage,
            }
        }

        fn decode_disconnect(biter: &mut Biter) -> Self {
            let remains0 = biter.extract(4) as u8;
            let remains = (0..19).map(|_| biter.extract(8) as u8).collect::<Vec<_>>();
            if remains0 != 0 || remains != [0; 19] {
                eprintln!(
                    "bad remains data, possibly broken: {:02X?}, {:02X?}",
                    remains0, remains
                );
            }

            Self::Disconnect
        }

        pub fn show(self) {
            match self {
                Self::Gyroscope { q1, q1p, q2, q2p } => Self::show_gyroscope(q1, q1p, q2, q2p),
                Self::Moves {
                    count,
                    moves,
                    times,
                } => Self::show_moves(count, moves, times),
                Self::State { count, state } => Self::show_cube_state(count, state),
                Self::Battery {
                    charging,
                    percentage,
                } => Self::show_battery_state(charging, percentage),
                Self::Disconnect => Self::show_disconnect(),
            }
        }

        fn show_gyroscope(q1: Quaternion, q1p: QuaternionP, q2: Quaternion, q2p: QuaternionP) {
            const BAR_WIDTH: usize = 12;
            const PBAR_WIDTH: usize = 2;

            fn draw_bar(value: f32, width: usize) -> String {
                const TEMP: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
                let n = (value * width as f32 * 8.0) as usize;
                (0..width)
                    .map(|i| TEMP[n.clamp(8 * i, 8 * (i + 1)) - 8 * i])
                    .collect()
            }

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

        fn show_moves(count: u8, moves: [Option<CubeMove>; 7], times: [u32; 7]) {
            print!("{}", CREL);
            print!("count={:3}, ", count);
            print!("({:.3} s) ", times[0] as f32 / 1000.0);
            for mv in moves {
                print!("{:2} ", mv.map_or("??".to_owned(), |m| m.to_string()));
            }
            println!();
        }

        fn show_cube_state(count: u8, state: CubeState) {
            print!("{}", CREL);
            print!("count={:3}, ", count);

            print!(
                "corners=[{}], ",
                state
                    .corners
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            println!(
                "edges=[{}]",
                state
                    .edges
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        fn show_battery_state(charging: bool, percentage: u32) {
            print!("{}", CREL);
            print!("battery={}%", percentage);
            if charging {
                print!(" (charging)");
            }
            println!();
        }

        fn show_disconnect() {
            print!("{}", CREL);
            println!("auto disconnect");
        }
    }

    #[allow(clippy::enum_variant_names)]
    #[rustfmt::skip]
    #[repr(u8)]
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum RequestMessageType {
        RequestCubeState    = 0b_0000_0100,
        RequestBatteryState = 0b_0000_1001,
        ResetCubeState      = 0b_0000_1010,
    }

    impl RequestMessageType {
        fn repr(self) -> u8 {
            self as u8
        }
    }

    #[allow(clippy::enum_variant_names)]
    pub enum RequestMessage {
        RequestCubeState,
        RequestBatteryState,
        ResetCubeState(CubeState),
    }

    impl RequestMessage {
        pub fn encode(&self, cipher: &GanCubeV2Cipher) -> [u8; 20] {
            let mut message = [0; 20];
            let mut biter = BiterMut::new(&mut message);

            match self {
                Self::RequestCubeState => {
                    biter.assign(8, RequestMessageType::RequestCubeState.repr() as u32);
                }
                Self::RequestBatteryState => {
                    biter.assign(8, RequestMessageType::RequestBatteryState.repr() as u32);
                }
                Self::ResetCubeState(state) => {
                    biter.assign(8, RequestMessageType::ResetCubeState.repr() as u32);
                    for corner in state.corners {
                        biter.assign(3, corner.0.repr() as u32);
                    }
                    for corner in state.corners {
                        biter.assign(2, corner.1.repr() as u32);
                    }
                    for edge in state.edges {
                        biter.assign(4, edge.0.repr() as u32);
                    }
                    for edge in state.edges {
                        biter.assign(1, edge.1.repr() as u32);
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
            let Ok(device_id) = <&[u8; 9]>::try_from(&manufacturer_data[..]) else {
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

            fn add_device_key(secret: &mut [u8; 16], device_key: &[u8; 6]) {
                secret
                    .iter_mut()
                    .zip(device_key)
                    .for_each(|(a, b)| *a = ((*a as u16 + *b as u16) % 255) as u8);
            }

            add_device_key(&mut key, &device_key);
            add_device_key(&mut iv, &device_key);

            let key = GenericArray::from(key);
            let iv = GenericArray::from(iv);
            let aes = Aes128::new(&key);
            Ok(GanCubeV2Cipher { key, iv, aes })
        }

        pub(super) fn encrypt(&self, value: &mut [u8; 20]) {
            fn encrypt_block(cipher: &GanCubeV2Cipher, block: &mut [u8]) {
                let block = GenericArray::from_mut_slice(block);
                block.iter_mut().zip(cipher.iv).for_each(|(a, b)| *a ^= b);
                cipher.aes.encrypt_block(block);
            }

            let offset = value.len() - 16;
            encrypt_block(self, &mut value[..16]);
            encrypt_block(self, &mut value[offset..]);
        }

        pub(super) fn decrypt(&self, value: &mut [u8; 20]) {
            fn decrypt_block(cipher: &GanCubeV2Cipher, block: &mut [u8]) {
                let block = GenericArray::from_mut_slice(block);
                cipher.aes.decrypt_block(block);
                block.iter_mut().zip(cipher.iv).for_each(|(a, b)| *a ^= b);
            }

            let offset = value.len() - 16;
            decrypt_block(self, &mut value[offset..]);
            decrypt_block(self, &mut value[..16]);
        }
    }
}

mod util {
    const FIRST_BIT: u8 = 1 << 7;

    // big-endian bit iterator
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
                if self.data[bit / 8] & (FIRST_BIT >> (bit % 8)) != 0 {
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
                if self.data[bit / 8] & (FIRST_BIT >> (bit % 8)) != 0 {
                    result |= 1;
                }
            }
            self.index += count;
            result
        }
        pub fn assign(&mut self, count: usize, value: u32) {
            for (bit, i) in (self.index..).take(count).zip((0..count).rev()) {
                if value & (1 << i) != 0 {
                    self.data[bit / 8] |= FIRST_BIT >> (bit % 8);
                } else {
                    self.data[bit / 8] &= !(FIRST_BIT >> (bit % 8));
                }
            }
            self.index += count;
        }
    }
}
