use std::collections::BTreeMap;

use charming::{
    component::{Axis, Legend},
    datatype::DataPointItem,
    element::{AxisType, Orient},
    series::Line,
    Chart, Echarts, WasmRenderer,
};
use chrono::{Duration, Local, Timelike as _};
use itertools::Itertools;
use leptos::{ReadSignal, SignalSet as _, SignalWithUntracked, WriteSignal};
use meatnet::{temperature::IsTemperature, uart::node::response::ReadLogs};

pub fn chart_handler(
    history: &BTreeMap<u32, ReadLogs>,
    set_chart: WriteSignal<Option<Echarts>>,
    get_chart: ReadSignal<Option<Echarts>>,
) {
    let max_value = match history.last_key_value() {
        Some((k, _)) => *k,
        None => 0,
    };

    let elapsed = Duration::seconds(max_value as i64 * 5);

    let mut start_time = Local::now() - elapsed;
    let extra_seconds = start_time.second() % 5;
    start_time -= Duration::seconds(extra_seconds as i64);

    let data = (0..max_value).map(|i| {
        let key = history.get(&i);
        (
            (start_time + Duration::seconds(i as i64 * 5))
                .format("%H:%M:%S")
                .to_string(),
            key,
        )
    });

    let chart = Chart::new()
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(data.clone().map(|(k, _)| k).collect_vec()),
        )
        .y_axis(Axis::new().type_(AxisType::Value))
        .legend(
            Legend::new()
                .data(vec!["Core", "Surface", "Ambient"])
                .right("10")
                .top("center")
                .orient(Orient::Vertical),
        )
        .series(
            Line::new().name("Core").data(
                data.clone()
                    .map(|(k, v)| {
                        DataPointItem::from((
                            match v {
                                Some(log_item) => {
                                    log_item.get_virtual_core_temperature().get_celsius()
                                }
                                None => f32::NAN,
                            },
                            k,
                        ))
                    })
                    .collect_vec(),
            ),
        )
        .series(
            Line::new().name("Surface").data(
                data.clone()
                    .map(|(k, v)| {
                        DataPointItem::from((
                            match v {
                                Some(log_item) => {
                                    log_item.get_virtual_surface_temperature().get_celsius()
                                }
                                None => f32::NAN,
                            },
                            k,
                        ))
                    })
                    .collect_vec(),
            ),
        )
        .series(
            Line::new().name("Ambient").data(
                data.clone()
                    .map(|(k, v)| {
                        DataPointItem::from((
                            match v {
                                Some(log_item) => {
                                    log_item.get_vitrual_ambient_temperature().get_celsius()
                                }
                                None => f32::NAN,
                            },
                            k,
                        ))
                    })
                    .collect_vec(),
            ),
        );

    let mut updated = false;
    get_chart.with_untracked(|optional_echarts| match optional_echarts {
        Some(echarts) => {
            WasmRenderer::update(echarts, &chart);
            updated = true
        }
        None => (),
    });

    if !updated {
        set_chart.set(match WasmRenderer::new(900, 600).render("chart", &chart) {
            Ok(value) => Some(value),
            Err(error) => {
                println!("Error rendering chart: {:?}", error);
                None
            }
        })
    }
}
