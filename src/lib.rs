pub mod ui;

use fpsdk::{
    create_plugin,
    plugin::{message::DebugLogMsg, Plugin, PluginProxy},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, io::Read, panic::RefUnwindSafe};

type Sample = [f32; 2];

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SaveState {
    Ver0 {},
}

#[derive(Debug, Clone)]
pub enum PluginStateChange {
    Param1,
}

#[derive(Debug)]
struct Template {
    host: Mutex<fpsdk::host::Host>,
    tag: fpsdk::plugin::Tag,
    handle: Option<fpsdk::plugin::PluginProxy>,

    ui_handle: ui::UIHandle,
}

unsafe impl Send for Template {}
unsafe impl Sync for Template {}

impl Template {
    fn log(&self, msg: String) {
        self.host
            .lock()
            .on_message(self.tag, DebugLogMsg(format!("template-plugin: {msg}")));
    }
}

// TODO(emily): This is what we call a _lie_
impl RefUnwindSafe for Template {}

impl Plugin for Template {
    fn new(host: fpsdk::host::Host, tag: fpsdk::plugin::Tag) -> Self
    where
        Self: Sized,
    {
        Self {
            host: Mutex::new(host),
            tag,
            handle: None,
            ui_handle: ui::UIHandle::new("emilydotgg-template"),
        }
    }

    fn info(&self) -> fpsdk::plugin::Info {
        fpsdk::plugin::InfoBuilder::new_effect("emilydotgg-template", "template", 0)
            .want_new_tick()
            .build()
    }

    fn save_state(&mut self, writer: fpsdk::plugin::StateWriter) {
        let state = SaveState::Ver0 {};
        bincode::serialize_into(writer, &state).unwrap();
    }

    fn load_state(&mut self, mut reader: fpsdk::plugin::StateReader) {
        let mut buf: Vec<u8> = vec![];
        if let Ok(save_state) = reader.read_to_end(&mut buf).and_then(|_| {
            bincode::deserialize::<SaveState>(&buf).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("error deserializing save state {}", e),
                )
            })
        }) {
            match save_state {
                SaveState::Ver0 {} => {}
            };
        } else {
            self.log(format!("error reading state"));
        }
    }

    fn on_message(&mut self, message: fpsdk::host::Message<'_>) -> Box<dyn fpsdk::AsRawPtr> {
        match message {
            fpsdk::host::Message::ShowEditor(hwnd) => {
                self.ui_handle
                    .send_sync(ui::UIMessage::ShowEditor(hwnd.into()))
                    .unwrap();
            }
            _ => {}
        }

        // TODO(emily): This really needs to happen somewhere else because we need to be able
        // to handle messages even if there isn't one from FL. This probably cannot go in tick() however
        // because that needs to be fast?
        while let Ok(msg) = self.ui_handle.rx.try_recv() {
            match msg {
                ui::PluginMessage::SetEditor(hwnd) => {
                    if let Some(handle) = self.handle.as_ref() {
                        handle.set_editor_hwnd(hwnd.as_ptr().unwrap_or(0 as *mut c_void));
                    }
                }
                ui::PluginMessage::UIInit => {
                    self.log(format!("UI initialised"));
                }
            }
        }
        Box::new(0)
    }

    fn name_of(&self, _value: fpsdk::host::GetName) -> String {
        "No names".into()
    }

    fn render(&mut self, input: &[[f32; 2]], output: &mut [[f32; 2]]) {
        for (o, i) in output.iter_mut().zip(input.iter()) {
            *o = *i;
        }
    }

    fn tick(&mut self) {
        // No tick
    }

    fn process_param(
        &mut self,
        _index: usize,
        _value: fpsdk::ValuePtr,
        _flags: fpsdk::ProcessParamFlags,
    ) -> Box<dyn fpsdk::AsRawPtr> {
        // No params
        Box::new(0)
    }

    fn proxy(&mut self, handle: PluginProxy) {
        self.handle = Some(handle)
    }
}

impl Drop for Template {
    fn drop(&mut self) {
        self.ui_handle.send_sync(ui::UIMessage::Die).unwrap();
        self.ui_handle.join();
    }
}

create_plugin!(Template);
