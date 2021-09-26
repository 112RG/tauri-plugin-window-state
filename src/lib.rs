use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Plugin, Result as PluginResult},
    AppHandle, Event, Manager, PhysicalPosition, PhysicalSize, Position, Runtime, Size, Window,
};

use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::Write,
    sync::{Arc, Mutex},
};

const STATE_FILENAME: &str = ".window-state";

#[derive(Debug, Default, Deserialize, Serialize)]
struct WindowMetadata {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

#[derive(Default)]
pub struct WindowState {
    cache: Arc<Mutex<HashMap<String, WindowMetadata>>>,
}

impl<R: Runtime> Plugin<R> for WindowState {
    fn name(&self) -> &'static str {
        "window-state"
    }

    fn initialize(&mut self, app: &AppHandle<R>, _config: serde_json::Value) -> PluginResult<()> {
        if let Some(app_dir) = app.path_resolver().app_dir() {
            let state_path = app_dir.join(STATE_FILENAME);
            if state_path.exists() {
                self.cache = Arc::new(Mutex::new(
                    tauri::api::file::read_binary(state_path)
                        .and_then(|state| bincode::deserialize(&state).map_err(Into::into))
                        .unwrap_or_default(),
                ));
            }
        }
        Ok(())
    }

    fn created(&mut self, window: Window<R>) {
        if let Some(state) = self.cache.lock().unwrap().get(window.label()) {
            window
                .set_position(Position::Physical(PhysicalPosition {
                    x: state.x,
                    y: state.y,
                }))
                .unwrap();
            window
                .set_size(Size::Physical(PhysicalSize {
                    width: state.width,
                    height: state.height,
                }))
                .unwrap();
        }
    }

    fn on_event(&mut self, app: &AppHandle<R>, event: &Event) {
        match event {
            Event::CloseRequested { label, api: _, .. } => {
                let window = app.get_window(&label).unwrap();
                let position = window.outer_position().unwrap();
                let size = window.inner_size().unwrap();

                let mut c = self.cache.lock().unwrap();
                let state = c.entry(label.clone()).or_insert_with(Default::default);
                state.x = position.x;
                state.y = position.y;
                state.width = size.width;
                state.height = size.height;
            }
            Event::Exit => {
                if let Some(app_dir) = app.path_resolver().app_dir() {
                    let state_path = app_dir.join(STATE_FILENAME);
                    let state = self.cache.lock().unwrap();
                    let _ = create_dir_all(&app_dir)
                        .map_err(tauri::api::Error::Io)
                        .and_then(|_| File::create(state_path).map_err(Into::into))
                        .and_then(|mut f| {
                            f.write_all(
                                &bincode::serialize(&*state).map_err(tauri::api::Error::Bincode)?,
                            )
                            .map_err(Into::into)
                        });
                }
            }
            _ => (),
        }
    }
}
