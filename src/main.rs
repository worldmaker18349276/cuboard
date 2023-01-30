use btleplug::api::{Central, Manager, ScanFilter};
use btleplug::platform;
use std::error::Error;

use bluetooth::gancubev2::GanCubeV2Builder;

mod bluetooth;
mod cube;

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
    let handle = gancube.register_handler(Box::new(|msg| msg.show())).await?;
    println!("connected! have fun~");
    println!();

    gancube.subscribe_response().await?;
    gancube.request_battery_state().await?;
    gancube.request_cube_state().await?;

    handle.await?;

    Ok(())
}
