use btleplug::{api::Manager as _, platform::Manager};
use std::{error::Error, sync::Arc};
use tokio::task;

use meatnet::ProductType;

mod combustion_device;
mod node;
mod probe;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let device_type = ProductType::MeatNetRepeater;

    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let central = manager
        .adapters()
        .await
        .expect("Unable to fetch adapter list.")
        .into_iter()
        .next()
        .expect("Unable to find adapters.");

    let mut device = combustion_device::find_combustion_device(&central, Some(&device_type))
        .await
        .expect("Unable to find device.");

    device.setup().await?;

    let device = Arc::new(device);

    let device_for_task1 = device.clone();
    let task1 = task::spawn(async move { device_for_task1.process_notifications().await });

    let device_for_task2 = device.clone();
    let task2 = task::spawn(async move { device_for_task2.request_log_updates().await });

    task1.await??;
    task2.await??;

    Ok(())
}
