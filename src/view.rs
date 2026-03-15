use std::pin::Pin;

use futures::{
    channel::mpsc::UnboundedReceiver,
    stream::{FusedStream, Stream},
    task::{Context, Poll},
};
use pin_project_lite::pin_project;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
pub enum Events {
    OpenFileClicked,
    SelectDeviceClicked,
    StartClicked,
    StopClicked,
}

pub struct ViewElements {
    pub box_filename: web_sys::HtmlInputElement,
    pub box_serial_vid_pid: web_sys::HtmlInputElement,
    pub button_open_file: web_sys::HtmlButtonElement,
    pub button_select_device: web_sys::HtmlButtonElement,
    pub button_start: web_sys::HtmlButtonElement,
    pub button_stop: web_sys::HtmlButtonElement,
    pub div_alerts: web_sys::Element,
    pub select_baud_rate: web_sys::HtmlSelectElement,
    pub template_alert: web_sys::HtmlTemplateElement,
    pub template_alert_error: web_sys::HtmlTemplateElement,
    pub template_alert_info: web_sys::HtmlTemplateElement,
    pub template_alert_success: web_sys::HtmlTemplateElement,
    pub template_alert_warning: web_sys::HtmlTemplateElement,
}

pin_project! {
    pub struct View {
        elements: ViewElements,
        #[pin]
        receiver: UnboundedReceiver<Events>,
    }
}

#[allow(unused)]
pub enum AlertType {
    Error,
    Warning,
    Info,
    Success,
    Unspecified,
}

impl View {
    pub fn from_elements(elements: ViewElements) -> Result<Self, JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        for (baud_rate, selected) in [
            ("9600", false),
            ("19200", false),
            ("38400", false),
            ("57600", false),
            ("115200", false),
            ("230400", true),
            ("460800", false),
            ("500000", false),
            ("576000", false),
            ("921600", false),
        ] {
            let option = document
                .create_element("option")?
                .dyn_into::<web_sys::HtmlOptionElement>()?;
            option.set_text(baud_rate);
            option.set_value(baud_rate);
            option.set_selected(selected);
            elements.select_baud_rate.append_child(&option)?;
        }

        let (sender, receiver) = futures::channel::mpsc::unbounded();

        for (button, event) in [
            (&elements.button_open_file, Events::OpenFileClicked),
            (&elements.button_select_device, Events::SelectDeviceClicked),
            (&elements.button_start, Events::StartClicked),
            (&elements.button_stop, Events::StopClicked),
        ] {
            let sender = sender.clone();
            let closure = Closure::<dyn FnMut()>::new(move || {
                sender.unbounded_send(event).unwrap();
            });
            button.set_onclick(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        Ok(Self { elements, receiver })
    }

    pub fn add_alert(&self, alert_type: AlertType, message: &str) -> Result<(), JsValue> {
        let template = match alert_type {
            AlertType::Error => &self.elements.template_alert_error,
            AlertType::Warning => &self.elements.template_alert_warning,
            AlertType::Info => &self.elements.template_alert_info,
            AlertType::Success => &self.elements.template_alert_success,
            AlertType::Unspecified => &self.elements.template_alert,
        };

        let document = web_sys::window().unwrap().document().unwrap();
        let alert = document
            .import_node_with_deep(&template.content(), true)?
            .dyn_into::<web_sys::DocumentFragment>()?;

        if let Some(span) = alert.query_selector("span")? {
            span.set_text_content(Some(message));
        }
        if let Some(button) = alert.query_selector("button")? {
            let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::Event| {
                event
                    .target()
                    .unwrap()
                    .dyn_ref::<web_sys::HtmlElement>()
                    .unwrap()
                    .closest("div.alert")
                    .unwrap()
                    .unwrap()
                    .remove()
            });
            let opts = web_sys::AddEventListenerOptions::new();
            opts.set_once(true);
            button.add_event_listener_with_callback_and_add_event_listener_options(
                "click",
                closure.as_ref().unchecked_ref(),
                &opts,
            )?;
            closure.forget();
        }

        self.elements.div_alerts.append_child(&alert)?;
        Ok(())
    }

    pub fn baud_rate(&self) -> u32 {
        self.elements
            .select_baud_rate
            .value()
            .parse()
            .unwrap_or(230400)
    }

    pub fn set_button_open_file_disabled(&self, value: bool) {
        self.elements.button_open_file.set_disabled(value);
    }

    pub fn set_button_select_device_disabled(&self, value: bool) {
        self.elements.button_select_device.set_disabled(value);
    }

    pub fn set_button_start_disabled(&self, value: bool) {
        self.elements.button_start.set_disabled(value);
    }

    pub fn set_button_stop_disabled(&self, value: bool) {
        self.elements.button_stop.set_disabled(value);
    }

    pub fn set_filename(&self, filename: &str) {
        self.elements.box_filename.set_value(filename);
    }

    pub fn set_serial_vid_pid(&self, serial_vid_pid: &str) {
        self.elements.box_serial_vid_pid.set_value(serial_vid_pid);
    }
}

impl Stream for View {
    type Item = Events;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.receiver.as_mut().poll_next(cx)
    }
}

impl FusedStream for View {
    fn is_terminated(&self) -> bool {
        self.receiver.is_terminated()
    }
}
