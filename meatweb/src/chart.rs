use std::collections::BTreeMap;

use charming::{
    component::Axis, datatype::DataPointItem, element::AxisType, series::Line, Chart, Echarts,
    WasmRenderer,
};
use chrono::{Duration, Local, Timelike as _};
use itertools::Itertools;
use leptos::{ReadSignal, SignalSet as _, SignalWith as _, WriteSignal};

use crate::history::LogItem;

pub fn chart_handler(
    history: BTreeMap<u32, LogItem>,
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
            match key {
                Some(key) => key.temperature.get_celsius() as f64,
                None => f64::NAN,
            },
        )
    });

    let chart = Chart::new()
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(data.clone().map(|(k, _)| k).collect_vec()),
        )
        .y_axis(Axis::new().type_(AxisType::Value))
        .series(Line::new().data(data.map(|(k, v)| DataPointItem::from((v, k))).collect_vec()));

    let mut updated = false;
    get_chart.with(|optional_echarts| match optional_echarts {
        Some(echarts) => {
            WasmRenderer::update(echarts, &chart);
            updated = true
        }
        None => (),
    });

    if !updated {
        set_chart.set(match WasmRenderer::new(800, 600).render("chart", &chart) {
            Ok(value) => Some(value),
            Err(error) => {
                println!("Error rendering chart: {:?}", error);
                None
            }
        })
    }
}
