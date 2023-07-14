use kiss3d::camera::Camera;
use kiss3d::nalgebra::{Quaternion, UnitQuaternion};

use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use std::error::Error;
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::bluetooth::gancubev2::{GanCubeV2Builder, ResponseMessage};
use crate::view::virtualcube::{VirtualCuboard, set_colors_spin};

struct UnitQuaternionSmoother<const N: usize>([UnitQuaternion<f32>; N], usize);

impl<const N: usize> UnitQuaternionSmoother<N> {
    fn new() -> Self {
        UnitQuaternionSmoother([UnitQuaternion::default(); N], 0)
    }

    fn put(&mut self, q: UnitQuaternion<f32>) {
        self.0[self.1] = q;
        self.1 = (self.1 + 1) % N;
    }

    fn get(&self) -> UnitQuaternion<f32> {
        let q = self
            .0
            .iter()
            .map(|q| q.quaternion())
            .fold(Quaternion::default(), |acc, q| acc + q);
        UnitQuaternion::new_normalize(q)
    }
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let orientation = Arc::new(Mutex::new(UnitQuaternionSmoother::<5>::new()));

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

    let orientation_msg = Arc::clone(&orientation);
    gancube
        .register_handler(Box::new(move |msg| {
            let ResponseMessage::Gyroscope { q1, q1p: _, q2, q2p: _ } = msg else {
                return;
            };

            let Ok(mut ori) = orientation_msg.lock() else {
                return;
            };

            let q1 = Quaternion::new(q1.0, q1.2, q1.3, q1.1);
            let q2 = Quaternion::new(q2.0, q2.2, q2.3, q2.1);
            ori.put(UnitQuaternion::new_normalize(q1 + q2))
        }))
        .await?;

    gancube.subscribe_response().await?;

    let orientation_cube = Arc::clone(&orientation);
    let mut cube = VirtualCuboard::new();
    cube.render_loop(move |cube| {
        let Ok(ori) = orientation_cube.lock() else {
            return;
        };

        let orientation = ori.get();
        cube.set_orientation(orientation);
        // set_colors_spin(&mut cube.components, cube.camera.eye(), orientation);
    });

    Ok(())
}
