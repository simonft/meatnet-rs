// #[tokio::main]
// async fn main2() -> Result<(), Box<dyn Error>> {
//     let manager = Manager::new().await.unwrap();

//     // get the first bluetooth adapter
//     let central = manager
//         .adapters()
//         .await
//         .expect("Unable to fetch adapter list.")
//         .into_iter()
//         .next()
//         .expect("Unable to find adapters.");

//     // start scanning for devices
//     central.start_scan(ScanFilter::default()).await?;
//     // instead of waiting, you can use central.events() to get a stream which will
//     // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
//     time::sleep(Duration::from_secs(2)).await;
//     let maybe_thermometer = central
//         .peripherals()
//         .await
//         .unwrap()
//         .into_iter()
//         .find(|p| p.address().to_string() == *"C2:71:04:91:14:D0");

//     let thermometer = maybe_thermometer.unwrap();

//     thermometer.connect().await?;
//     thermometer.discover_services().await?;

//     let characteristics = thermometer.characteristics();

//     let probe_status_characteristic = characteristics
//         .iter()
//         .find(|c| c.uuid == PROBE_STATUS_CHARACTERISTIC_UUID)
//         .expect("Unable to find probe status characteristic");
//     thermometer.subscribe(probe_status_characteristic).await?;

//     let mut i = 0;

//     let mut notification_stream = thermometer.notifications().await?;
//     // Process while the BLE connection is not broken or stopped.
//     while let Some(data) = notification_stream.next().await {
//         data.value
//             .as_slice()
//             .iter()
//             .for_each(|j| print!("{j:08b} "));
//         let probe_status = ProbeStatus::try_from(data.value.as_slice());
//         println!("{:#?}", probe_status);
//         // if let Mode::Normal = probe_status.mode_and_id().mode() {
//         //     println!("{:#?}", probe_status)
//         // }

//         println!("{}", i);
//         i += 1;
//     }

//     Ok(())
// }
