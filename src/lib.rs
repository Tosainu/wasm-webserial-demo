mod app;
mod util;
mod view;

use wasm_bindgen::prelude::*;

use crate::view::{View, ViewElements};

fn qs(selectors: &str) -> Result<web_sys::Element, JsValue> {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(selectors)?
        .ok_or(format!("could not locate '{}'", selectors).into())
}

#[wasm_bindgen(start)]
fn run() -> Result<(), JsValue> {
    let view = View::from_elements(ViewElements {
        box_filename: qs("#box-filename")?.dyn_into()?,
        box_serial_vid_pid: qs("#box-serial-vid-pid")?.dyn_into()?,
        button_open_file: qs("#button-open-file")?.dyn_into()?,
        button_select_device: qs("#button-select-device")?.dyn_into()?,
        button_start: qs("#button-start")?.dyn_into()?,
        button_stop: qs("#button-stop")?.dyn_into()?,
        div_alerts: qs("#div-alerts")?.dyn_into()?,
        select_baud_rate: qs("#select-baud-rate")?.dyn_into()?,
        template_alert: qs("#template-alert")?.dyn_into()?,
        template_alert_error: qs("#template-alert-error")?.dyn_into()?,
        template_alert_info: qs("#template-alert-info")?.dyn_into()?,
        template_alert_success: qs("#template-alert-success")?.dyn_into()?,
        template_alert_warning: qs("#template-alert-warning")?.dyn_into()?,
    })?;

    let mut unsupported_browser = false;
    {
        if !util::web_serial_api_supported() {
            view.add_alert(
                view::AlertType::Error,
                "The browser does not support the Web Serial API!",
            )?;
            unsupported_browser = true;
        }
        if !util::file_system_access_api_supported() {
            view.add_alert(
                view::AlertType::Error,
                "The browser does not support the File System Access API!",
            )?;
            unsupported_browser = true;
        }
    }

    wasm_bindgen_futures::spawn_local(async move {
        if unsupported_browser {
            view.set_button_open_file_disabled(true);
            view.set_button_select_device_disabled(true);
            view.set_button_start_disabled(true);
            view.set_button_stop_disabled(true);
            futures::future::pending::<()>().await;
        }
        app::App::new(view).run().await;
    });

    Ok(())
}
