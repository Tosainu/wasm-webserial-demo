use std::pin::Pin;

use futures::{
    StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::util::console_log;
use crate::view::*;

type WorkerResult = Result<usize, JsValue>;
type Worker = Pin<Box<dyn Future<Output = WorkerResult>>>;

struct WorkerHandle(web_sys::ReadableStreamDefaultReader);

const SERIAL_BUFFER_SIZE: u32 = 1024;

async fn make_worker(
    view: &View,
    file: &web_sys::FileSystemFileHandle,
    serial_port: &web_sys::SerialPort,
    baud_rate: u32,
) -> Result<(Worker, WorkerHandle), JsValue> {
    let opts = web_sys::SerialOptions::new(baud_rate);
    opts.set_buffer_size(SERIAL_BUFFER_SIZE);
    JsFuture::from(serial_port.open(&opts)).await?;

    let reader = serial_port
        .readable()
        .get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()?;

    let file_info = JsFuture::from(file.get_file())
        .await?
        .dyn_into::<web_sys::File>()?;
    let file_size = file_info.size();
    if file_size != 0.0 {
        view.add_alert(
            AlertType::Info,
            "The file is not empty, appending data to the end.",
        )?;
    }

    let opts = web_sys::FileSystemCreateWritableOptions::new();
    opts.set_keep_existing_data(true);
    let writer = JsFuture::from(file.create_writable_with_options(&opts))
        .await?
        .dyn_into::<web_sys::FileSystemWritableFileStream>()?;

    let _ = JsFuture::from(writer.seek_with_f64(file_size)?).await;

    let handle = WorkerHandle(reader.clone());
    let worker = Box::pin(async move {
        let mut len = 0;
        loop {
            let ret = JsFuture::from(reader.read()).await?;

            if js_sys::Reflect::get(&ret, &JsValue::from("done"))?
                .as_bool()
                .unwrap_or(false)
            {
                reader.release_lock();
                break;
            }

            let value = js_sys::Reflect::get(&ret, &JsValue::from("value"))?
                .dyn_into::<js_sys::Uint8Array>()?
                .to_vec();

            len += value.len();

            JsFuture::from(writer.write_with_u8_array(&value)?).await?;
        }

        JsFuture::from(writer.close()).await?;

        Ok(len)
    });

    Ok((worker, handle))
}

impl WorkerHandle {
    async fn stop(self) -> Result<(), JsValue> {
        JsFuture::from(self.0.cancel()).await?;
        Ok(())
    }
}

pub struct App {
    view: View,
    file: Option<web_sys::FileSystemFileHandle>,
    serial_port: Option<web_sys::SerialPort>,
    handle: Option<WorkerHandle>,
    sender: UnboundedSender<WorkerResult>,
    receiver: UnboundedReceiver<WorkerResult>,
}

impl App {
    pub fn new(view: View) -> App {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        App {
            view,
            file: None,
            serial_port: None,
            handle: None,
            sender,
            receiver,
        }
    }

    pub async fn run(&mut self) {
        loop {
            futures::select! {
                event = self.view.next() => {
                    if let Some(event) = event
                        && let Err(e) = self.handle_view_event(event).await
                    {
                        console_log!("unexpected error: {:?}", e);
                    }
                }

                result = self.receiver.next() => {
                    if let Some(result) = result
                        && let Err(e) = self.handle_worker_event(result).await
                    {
                        console_log!("unexpected error: {:?}", e);
                    }
                }
            }
        }
    }

    async fn handle_view_event(&mut self, event: Events) -> Result<(), JsValue> {
        console_log!("> {:?}", event);

        match event {
            Events::OpenFileClicked if self.handle.is_none() => {
                let picker = web_sys::window().unwrap().show_save_file_picker()?;
                if let Ok(f) = JsFuture::from(picker).await {
                    self.view.set_filename(&f.name());
                    self.file.replace(f);
                }
            }

            Events::SelectDeviceClicked if self.handle.is_none() => {
                let serial = web_sys::window().unwrap().navigator().serial();
                if let Ok(p) = JsFuture::from(serial.request_port()).await {
                    let info = p.get_info();
                    if let Some(vid) = info.get_usb_vendor_id()
                        && let Some(pid) = info.get_usb_product_id()
                    {
                        let s = format!("VID:PID = {:04x}:{:04x}", vid, pid);
                        self.view.set_serial_vid_pid(&s);
                    } else {
                        self.view.set_serial_vid_pid("VID:PID = (unknown)");
                    }
                    self.serial_port.replace(p);
                }
            }

            Events::StartClicked if self.handle.is_none() => {
                match (self.file.as_ref(), self.serial_port.as_ref()) {
                    (None, _) => self
                        .view
                        .add_alert(AlertType::Warning, "Please select the output file!")?,

                    (_, None) => self
                        .view
                        .add_alert(AlertType::Warning, "Please select the serial port!")?,

                    (Some(file), Some(serial_port)) => {
                        let (worker, handle) =
                            make_worker(&self.view, file, serial_port, self.view.baud_rate())
                                .await?;
                        self.handle.replace(handle);

                        self.view.set_button_open_file_disabled(true);
                        self.view.set_button_select_device_disabled(true);
                        self.view.set_button_start_disabled(true);

                        let sender = self.sender.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            sender.unbounded_send(worker.await).unwrap();
                        });
                    }
                }
            }

            Events::StopClicked => {
                if let Some(handle) = self.handle.take() {
                    handle.stop().await?;
                }
            }

            _ => (),
        }

        Ok(())
    }

    async fn handle_worker_event(&mut self, result: WorkerResult) -> Result<(), JsValue> {
        if let Some(serial_port) = self.serial_port.as_ref() {
            JsFuture::from(serial_port.close()).await?;
        }
        match result {
            Ok(len) => self
                .view
                .add_alert(AlertType::Success, &format!("Done, got {} bytes!", len))?,
            Err(e) => self
                .view
                .add_alert(AlertType::Warning, &format!("Worker stopped: {:?}", e))?,
        }

        self.handle.take();
        self.view.set_button_open_file_disabled(false);
        self.view.set_button_select_device_disabled(false);
        self.view.set_button_start_disabled(false);

        Ok(())
    }
}
