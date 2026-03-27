use base64::Engine;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use psd::Psd;
use serde::Serialize;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
struct LayerInfo {
    name: String,
    width: u32,
    height: u32,
    top: i32,
    left: i32,
    preview_data_url: String,
    saved_path: String,
}

#[derive(Clone, Serialize)]
struct ProcessResult {
    file_name: String,
    output_dir: String,
    layers: Vec<LayerInfo>,
    composite_preview: String,
}

struct WatcherState {
    watcher: Option<RecommendedWatcher>,
}

#[tauri::command]
fn process_psd(file_path: String) -> Result<ProcessResult, String> {
    let path = Path::new(&file_path);
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let psd_bytes = fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let psd = Psd::from_bytes(&psd_bytes).map_err(|e| format!("Failed to parse PSD: {}", e))?;

    let parent_dir = path.parent().unwrap_or(Path::new("."));
    let output_dir = parent_dir.join(format!("sliced-images-{}", file_name));
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let doc_width = psd.width();
    let doc_height = psd.height();

    // Generate composite preview from PSD's stored flattened image
    let composite_rgba = psd.rgba();
    let composite_preview = {
        let comp_img: RgbaImage =
            ImageBuffer::from_raw(doc_width as u32, doc_height as u32, composite_rgba)
                .ok_or("Failed to create composite image")?;
        let dynamic = DynamicImage::ImageRgba8(comp_img);
        // Resize for preview (max 400px wide)
        let preview = if doc_width > 400 {
            dynamic.resize(400, 400, image::imageops::FilterType::Lanczos3)
        } else {
            dynamic
        };
        let mut buf = Cursor::new(Vec::new());
        preview
            .write_to(&mut buf, ImageFormat::Png)
            .map_err(|e| format!("Failed to encode composite: {}", e))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
        format!("data:image/png;base64,{}", b64)
    };

    let mut layers_info = Vec::new();

    for layer in psd.layers().iter() {
        let layer_name = layer.name().to_string();
        let ext = Path::new(&layer_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "png" && ext != "jpg" {
            continue;
        }

        let rgba_data = layer.rgba();
        let lw = (layer.width()) as u32;
        let lh = (layer.height()) as u32;
        let lt = layer.layer_top();
        let ll = layer.layer_left();

        if lw == 0 || lh == 0 {
            continue;
        }

        let full_img = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
            doc_width as u32,
            doc_height as u32,
            rgba_data,
        )
        .ok_or_else(|| format!("Failed to create image for layer {}", layer_name))?;

        let keep_original = layer_name.starts_with('_');

        let img: RgbaImage = if keep_original {
            full_img
        } else {
            let crop_x = ll.max(0) as u32;
            let crop_y = lt.max(0) as u32;
            let crop_w = lw.min(doc_width as u32 - crop_x);
            let crop_h = lh.min(doc_height as u32 - crop_y);
            image::imageops::crop_imm(&full_img, crop_x, crop_y, crop_w, crop_h).to_image()
        };

        let dynamic_img = DynamicImage::ImageRgba8(img.clone());

        let save_path = output_dir.join(&layer_name);
        if ext == "png" {
            dynamic_img
                .save_with_format(&save_path, ImageFormat::Png)
                .map_err(|e| format!("Failed to save PNG: {}", e))?;
        } else {
            let rgb_img = dynamic_img.to_rgb8();
            DynamicImage::ImageRgb8(rgb_img)
                .save_with_format(&save_path, ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to save JPG: {}", e))?;
        }

        let mut preview_buf = Cursor::new(Vec::new());
        dynamic_img
            .write_to(&mut preview_buf, ImageFormat::Png)
            .map_err(|e| format!("Failed to encode preview: {}", e))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(preview_buf.into_inner());
        let data_url = format!("data:image/png;base64,{}", b64);

        layers_info.push(LayerInfo {
            name: layer_name,
            width: img.width(),
            height: img.height(),
            top: lt,
            left: ll,
            preview_data_url: data_url,
            saved_path: save_path.to_string_lossy().to_string(),
        });
    }

    Ok(ProcessResult {
        file_name,
        output_dir: output_dir.to_string_lossy().to_string(),
        layers: layers_info,
        composite_preview,
    })
}

#[tauri::command]
fn watch_file(app: AppHandle, file_path: String) -> Result<(), String> {
    let state = app.state::<Mutex<WatcherState>>();
    let mut state = state.lock().map_err(|e| e.to_string())?;

    // Stop existing watcher
    state.watcher = None;

    let path = Path::new(&file_path).to_path_buf();
    let app_handle = app.clone();
    let watched_path = path.clone();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_)
                ) {
                    let _ = app_handle.emit("file-changed", watched_path.to_string_lossy().to_string());
                }
            }
        },
        notify::Config::default(),
    )
    .map_err(|e| format!("Failed to create watcher: {}", e))?;

    watcher
        .watch(path.as_path(), RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch file: {}", e))?;

    state.watcher = Some(watcher);
    Ok(())
}

#[tauri::command]
fn unwatch_file(app: AppHandle) -> Result<(), String> {
    let state = app.state::<Mutex<WatcherState>>();
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.watcher = None;
    Ok(())
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| format!("Failed to open folder: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(WatcherState { watcher: None }))
        .invoke_handler(tauri::generate_handler![
            process_psd,
            open_folder,
            watch_file,
            unwatch_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
