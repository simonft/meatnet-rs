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
use leptos::{
    ReadSignal, SignalGetUntracked as _, SignalSet as _, SignalWithUntracked, WriteSignal,
};
use meatnet::{temperature::IsTemperature, uart::node::response::ReadLogs};
use wasm_bindgen_futures::wasm_bindgen;

use wasm_bindgen::prelude::*;

use crate::bluetooth::ConnectionState;

// charming doesn't bring in the .clear() method, so we bring it in ourselves.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = echarts)]
    pub type OurEcharts;

    #[wasm_bindgen(method, js_name = "clear")]
    fn clear(this: &OurEcharts);
}

pub fn chart_handler(
    history: &BTreeMap<u32, ReadLogs>,
    set_chart: WriteSignal<Option<Echarts>>,
    get_chart: ReadSignal<Option<Echarts>>,
    state: ReadSignal<ConnectionState>,
) {
    let max_value = match history.last_key_value() {
        Some((k, _)) => *k,
        None => 0,
    };

    let max_log = match state.get_untracked() {
        ConnectionState::Connected(state) => state.log_end,
        _ => max_value,
    };

    let elapsed = Duration::seconds(max_log as i64 * 5);

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
            if !history.is_empty() {
                WasmRenderer::update(echarts, &chart);
                updated = true
            } else {
                // Just updating with an empty chart doesn't work, so we clear it first.
                OurEcharts::from(JsValue::from(echarts)).clear();
            }
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
