use btleplug::{api::Manager as _, platform::Manager};
use clap::{arg, ArgGroup, Command, Id};
use std::{error::Error, sync::Arc};
use tokio::task;

use meatnet::ProductType;

mod combustion_device;
mod node;
mod probe;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("meateatrs")
        .version("1.0")
        .arg(arg!(--probe "Sets the product type to PredictiveProbe"))
        .arg(arg!(--node "Sets the product type to MeatNetRepeater"))
        .group(
            ArgGroup::new("device-type")
                .args(["probe", "node"])
                .multiple(false)
                .required(false),
        )
        .get_matches();

    let device_type = match matches.get_one::<Id>("device-type") {
        Some(device_type) => match device_type.as_str() {
            "probe" => Some(&ProductType::PredictiveProbe),
            "node" => Some(&ProductType::MeatNetRepeater),
            _ => panic!("Invalid device type"),
        },
        _ => None,
    };

    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let central = manager
        .adapters()
        .await
        .expect("Unable to fetch adapter list.")
        .into_iter()
        .next()
        .expect("Unable to find adapters.");

    let mut device = combustion_device::find_combustion_device(&central, device_type)
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
