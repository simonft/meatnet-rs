use leptos::*;
use meatnet::temperature::IsTemperature as _;

use crate::bluetooth::ConnectionState;

#[component]
fn TemperatureDisplay<F>(temperature: F, label: String) -> impl IntoView
where
    F: Fn() -> Option<f32> + 'static + Copy,
{
    view! {
        <div class="m-5">
            <div class="label">{label}</div>
            <div class="temperature text-3xl" class:text-neutral-200=move || temperature().is_none()>
                {move || format!("{:.1}°C", temperature().unwrap_or(0.0))}
            </div>
        </div>
    }
}

#[component]
pub fn LiveTempContainer(state: ReadSignal<ConnectionState>) -> impl IntoView {
    view! {
        <div class="mx-auto flex flex-col justify-center">
            <div class="m-5">
                Mode:
                <div class="text-2xl">
                    {move || match state.get() {
                        ConnectionState::Connected(state) => {
                            match state.mode {
                                meatnet::Mode::Normal => "Normal",
                                meatnet::Mode::InstantRead => "Instant Read",
                                _ => "Unknown or Error",
                            }
                        }
                        _ => "Not Connected",
                    }}

                </div>
            </div>
            <TemperatureDisplay
                temperature=move || match state.get() {
                    ConnectionState::Connected(state) => Some(state.core_temperature.get_celsius()),
                    _ => None,
                }

                label="Core Temperature".to_string()
            />
            <Show when=move || match state.get() {
                ConnectionState::Connected(state) => state.mode == meatnet::Mode::Normal,
                _ => false,
            }>
                <TemperatureDisplay
                    label="Surface Temperature".to_string()
                    temperature=move || match state.get() {
                        ConnectionState::Connected(state) => {
                            Some(state.surface_temperature.get_celsius())
                        }
                        _ => None,
                    }
                />

                <TemperatureDisplay
                    label="Ambient Temperature".to_string()
                    temperature=move || match state.get() {
                        ConnectionState::Connected(state) => {
                            Some(state.ambient_temperature.get_celsius())
                        }
                        _ => None,
                    }
                />

            </Show>
        </div>
    }
}
