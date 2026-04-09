pub mod models;
pub mod frontend;

#[cfg(feature = "ssr")]
pub mod db;
#[cfg(feature = "ssr")]
pub mod api;
#[cfg(feature = "ssr")]
pub mod scanners;
#[cfg(feature = "ssr")]
pub mod services;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(frontend::app::App);
}
