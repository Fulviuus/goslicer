use base64::Engine;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage};
use psd::Psd;
use serde::Serialize;
use std::fs;
use std::io::Cursor;
use std::path::Path;

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

        // The psd crate may return rgba data sized to the full document rather than the layer.
        // Detect which case we're in and handle accordingly.
        let expected_layer_bytes = (lw as usize) * (lh as usize) * 4;
        let expected_doc_bytes = (doc_width as usize) * (doc_height as usize) * 4;
        let data_is_doc_sized = rgba_data.len() == expected_doc_bytes && rgba_data.len() != expected_layer_bytes;

        // Check if layer name starts with _ (keep original size)
        let keep_original = layer_name.starts_with('_');

        let img: RgbaImage = if keep_original || data_is_doc_sized {
            // Data is already document-sized — use it directly as full canvas
            if data_is_doc_sized {
                ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(doc_width as u32, doc_height as u32, rgba_data)
                    .ok_or_else(|| format!("Failed to create doc-sized image for layer {}", layer_name))?
            } else {
                let mut canvas =
                    ImageBuffer::<Rgba<u8>, Vec<u8>>::new(doc_width as u32, doc_height as u32);
                if let Some(layer_img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(lw, lh, rgba_data)
                {
                    image::imageops::overlay(
                        &mut canvas,
                        &layer_img,
                        ll as i64,
                        lt as i64,
                    );
                }
                canvas
            }
        } else {
            let data_len = rgba_data.len();
            ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(lw, lh, rgba_data)
                .ok_or_else(|| format!("Failed to create image for layer {} (expected {} bytes, got {})", layer_name, expected_layer_bytes, data_len))?
        };

        // If data was doc-sized but layer is not marked as full-size, crop to layer bounds
        let dynamic_img = if data_is_doc_sized && !keep_original {
            let x = ll.max(0) as u32;
            let y = lt.max(0) as u32;
            let crop_w = lw.min(doc_width as u32 - x);
            let crop_h = lh.min(doc_height as u32 - y);
            DynamicImage::ImageRgba8(img).crop_imm(x, y, crop_w, crop_h)
        } else {
            DynamicImage::ImageRgba8(img)
        };

        let (save_w, save_h) = (dynamic_img.width(), dynamic_img.height());

        // Save to disk
        let save_path = output_dir.join(&layer_name);
        if ext == "png" {
            dynamic_img
                .save_with_format(&save_path, ImageFormat::Png)
                .map_err(|e| format!("Failed to save PNG: {}", e))?;
        } else {
            // JPEG doesn't support alpha — convert RGBA → RGB before saving
            let rgb_img = DynamicImage::ImageRgb8(dynamic_img.to_rgb8());
            rgb_img
                .save_with_format(&save_path, ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to save JPG: {}", e))?;
        }

        // Generate base64 preview (always PNG for preview)
        let mut preview_buf = Cursor::new(Vec::new());
        dynamic_img
            .write_to(&mut preview_buf, ImageFormat::Png)
            .map_err(|e| format!("Failed to encode preview: {}", e))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(preview_buf.into_inner());
        let data_url = format!("data:image/png;base64,{}", b64);

        layers_info.push(LayerInfo {
            name: layer_name,
            width: save_w,
            height: save_h,
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
    })
}

#[tauri::command]
async fn pick_psd_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let file = app
        .dialog()
        .file()
        .add_filter("Photoshop Files", &["psd"])
        .blocking_pick_file();
    Ok(file.map(|f| f.into_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
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
        .plugin(tauri_plugin_drag::init())
        .invoke_handler(tauri::generate_handler![process_psd, pick_psd_file, open_folder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
