use kiss3d::camera::ArcBall;
use kiss3d::light::Light;
use kiss3d::nalgebra::{Point3, Quaternion, UnitQuaternion};
use kiss3d::scene::SceneNode;
use kiss3d::window::Window;

use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use std::error::Error;
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::bluetooth::gancubev2::{GanCubeV2Builder, ResponseMessage};

use super::virtualcube::VirtualCubeMeshes;

struct VirtualCuboard {
    meshes: VirtualCubeMeshes,
    window: Window,
    node: SceneNode,
}

impl VirtualCuboard {
    pub fn new() -> Self {
        let mut window = Window::new("cube");
        let mut node = window.add_group();
        let meshes = VirtualCubeMeshes::new(0.2, 0.02, 0.1);
        meshes.add_meshes(&mut node);
        VirtualCuboard {
            meshes,
            window,
            node,
        }
    }

    pub fn render_loop<F: FnMut(&mut Self)>(&mut self, mut f: F) {
        self.window.set_light(Light::StickToCamera);

        let mut camera = ArcBall::new(Point3::new(0.5, 0.7, 1.0), Point3::default());
        camera.rebind_drag_button(None);

        while self.window.render_with_camera(&mut camera) {
            f(self)
        }
    }
}

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
            let ResponseMessage::Gyroscope { q1, q1p: _, q2: _, q2p: _ } = msg else {
                return;
            };

            let Ok(mut ori) = orientation_msg.lock() else {
                return;
            };

            ori.put(UnitQuaternion::new_normalize(Quaternion::new(q1.0, q1.2, q1.3, q1.1)))
        }))
        .await?;

    gancube.subscribe_response().await?;

    let orientation_cube = Arc::clone(&orientation);
    let mut cube = VirtualCuboard::new();
    cube.render_loop(move |cube| {
        let Ok(ori) = orientation_cube.lock() else {
            return;
        };

        cube.node.set_local_rotation(ori.get());
    });

    Ok(())
}
