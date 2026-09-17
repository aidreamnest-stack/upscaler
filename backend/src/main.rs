use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::Stream;
use image::GenericImageView;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Semaphore;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    models_dir: PathBuf,
    uploads_dir: PathBuf,
    outputs_dir: PathBuf,
    engine_path: PathBuf,
    concurrency_limiter: Arc<Semaphore>,
}

#[derive(Serialize)]
struct ModelInfo {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    description: &'static str,
    recommended: bool,
    default_scale: u8,
}

#[derive(Deserialize)]
struct DownloadQuery {
    download: Option<String>,
    name: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    engine_ready: bool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let models_dir = std::env::var("MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dir.join("models"));
    let uploads_dir = std::env::var("UPLOADS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dir.join("uploads"));
    let outputs_dir = std::env::var("OUTPUTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dir.join("outputs"));

    let engine_path = if let Ok(custom_path) = std::env::var("ENGINE_PATH") {
        PathBuf::from(custom_path)
    } else {
        #[cfg(target_os = "windows")]
        let default_bin = "realesrgan-ncnn-vulkan.exe";
        #[cfg(not(target_os = "windows"))]
        let default_bin = "realesrgan-ncnn-vulkan";

        if base_dir.join(default_bin).exists() {
            base_dir.join(default_bin)
        } else {
            PathBuf::from(default_bin)
        }
    };

    tokio::fs::create_dir_all(&uploads_dir).await.ok();
    tokio::fs::create_dir_all(&outputs_dir).await.ok();
    tokio::fs::create_dir_all(&models_dir).await.ok();

    let max_concurrency: usize = std::env::var("MAX_CONCURRENT_JOBS")
        .unwrap_or_else(|_| "2".to_string())
        .parse()
        .unwrap_or(2);

    let state = AppState {
        models_dir,
        uploads_dir: uploads_dir.clone(),
        outputs_dir: outputs_dir.clone(),
        engine_path,
        concurrency_limiter: Arc::new(Semaphore::new(max_concurrency)),
    };

    // Spawn 10-minute background auto-cleanup reaper for uploads and outputs
    let reaper_outputs = outputs_dir.clone();
    let reaper_uploads = uploads_dir.clone();
    tokio::spawn(async move {
        let ttl = Duration::from_secs(600); // 10 minutes TTL
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            for dir in [&reaper_outputs, &reaper_uploads] {
                if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        if path.is_file() {
                            if let Ok(meta) = entry.metadata().await {
                                if let Ok(modified) = meta.modified() {
                                    if let Ok(age) = modified.elapsed() {
                                        if age > ttl {
                                            if tokio::fs::remove_file(&path).await.is_ok() {
                                                info!("[Auto-Cleanup] Pruned expired temporary file: {:?}", path.file_name());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/models", get(list_models))
        .route("/api/upscale", post(handle_upscale))
        .route("/outputs/:filename", get(serve_output_file))
        .route("/uploads/:filename", get(serve_upload_file))
        .layer(DefaultBodyLimit::max(60 * 1024 * 1024)) // 60MB max image upload limit
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 AI Upscale Lab Rust Backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");
    axum::serve(listener, app).await.unwrap();
}

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let engine_ready = state.engine_path.exists()
        || which::which(&state.engine_path).is_ok()
        || state.models_dir.exists();

    Json(HealthResponse {
        status: "healthy",
        service: "ai-upscale-backend-rust",
        version: "1.0.0",
        engine_ready,
    })
}

mod which {
    use std::path::Path;
    pub fn which(binary: &Path) -> Result<(), ()> {
        if binary.is_file() {
            return Ok(());
        }
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                if dir.join(binary).is_file() {
                    return Ok(());
                }
            }
        }
        Err(())
    }
}

async fn list_models() -> Json<Vec<ModelInfo>> {
    Json(vec![
        ModelInfo {
            id: "4x_NMKD-Superscale-SP_178000_G",
            name: "NMKD Superscale (Photo Clarity & Textures)",
            category: "Photorealistic & High Detail",
            description: "Midjourney-level photo sharpening and realistic skin/fabric texture synthesis.",
            recommended: true,
            default_scale: 4,
        },
        ModelInfo {
            id: "4xLSDIR",
            name: "LSDIR (Large-Scale Crisp & Natural Photos)",
            category: "Photorealistic & High Detail",
            description: "Exceptional general photo restoration with zero hallucination.",
            recommended: false,
            default_scale: 4,
        },
        ModelInfo {
            id: "4xNomos8kSC",
            name: "Nomos 8K (Cinematic Landscapes & Textures)",
            category: "Photorealistic & High Detail",
            description: "Ultra-sharp detail preservation for scenery, architecture, and textures.",
            recommended: false,
            default_scale: 4,
        },
        ModelInfo {
            id: "RealESRGAN_General_x4_v3",
            name: "RealESRGAN General v3 (Fast & Clean)",
            category: "Photorealistic & High Detail",
            description: "Balanced, noise-reduced general photo upscaling.",
            recommended: false,
            default_scale: 4,
        },
        ModelInfo {
            id: "realesrgan-x4plus",
            name: "RealESRGAN x4 Plus (Standard Classic)",
            category: "Photorealistic & High Detail",
            description: "The classic Real-ESRGAN model for general restoration.",
            recommended: false,
            default_scale: 4,
        },
        ModelInfo {
            id: "4x_NMKD-Siax_200k",
            name: "NMKD Siax (Digital Art / 3D Renders / CGI)",
            category: "Digital Art, Anime & 3D",
            description: "Engineered specifically for digital artwork, Midjourney AI generations, and 3D renders.",
            recommended: false,
            default_scale: 4,
        },
        ModelInfo {
            id: "realesrgan-x4plus-anime",
            name: "RealESRGAN Anime Plus (2D Illustration & Cartoons)",
            category: "Digital Art, Anime & 3D",
            description: "Crisp line art reconstruction without color bleeding.",
            recommended: false,
            default_scale: 4,
        },
        ModelInfo {
            id: "realesr-animevideov3",
            name: "RealESR AnimeVideo v3 (Ultra Fast / Smooth)",
            category: "Digital Art, Anime & 3D",
            description: "Lightweight video and anime model with 2X, 3X, and 4X native scaling.",
            recommended: false,
            default_scale: 2,
        },
    ])
}

async fn handle_upscale(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut model_name = "realesrgan-x4plus".to_string();
    let mut scale = "4".to_string();
    let mut gpu = "auto".to_string();
    let mut enable_tta = false;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "image" {
            if let Ok(bytes) = field.bytes().await {
                image_bytes = Some(bytes.to_vec());
            }
        } else if name == "model" {
            if let Ok(text) = field.text().await {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    model_name = trimmed.to_string();
                }
            }
        } else if name == "scale" {
            if let Ok(text) = field.text().await {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    scale = trimmed.to_string();
                }
            }
        } else if name == "gpu" {
            if let Ok(text) = field.text().await {
                gpu = text.trim().to_string();
            }
        } else if name == "tta" {
            if let Ok(text) = field.text().await {
                let lower = text.trim().to_lowercase();
                enable_tta = lower == "true" || lower == "1";
            }
        }
    }

    let file_bytes = image_bytes.ok_or(StatusCode::BAD_REQUEST)?;
    let timestamp = Instant::now();
    let job_id = Uuid::new_v4().to_string();
    let input_filename = format!("input_{}.png", job_id);
    let output_filename = format!("upscaled_{}.png", job_id);

    let input_path = state.uploads_dir.join(&input_filename);
    let output_path = state.outputs_dir.join(&output_filename);

    if let Err(e) = tokio::fs::write(&input_path, &file_bytes).await {
        error!("Failed to write uploaded file: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let mut orig_w = 0u32;
    let mut orig_h = 0u32;
    if let Ok(img) = image::load_from_memory(&file_bytes) {
        let (w, h) = img.dimensions();
        orig_w = w;
        orig_h = h;
    }

    let orig_size_kb = (file_bytes.len() as f64 / 1024.0 * 10.0).round() / 10.0;
    let stream_state = state.clone();

    let stream = async_stream::stream! {
        // Acquire semaphore permit for concurrency management
        let _permit = match stream_state.concurrency_limiter.acquire().await {
            Ok(p) => p,
            Err(_) => {
                yield Ok(Event::default().data(r#"{"type":"error","message":"Server busy"}"#));
                return;
            }
        };

        let start_msg = serde_json::json!({
            "type": "start",
            "message": "Image loaded. Processing neural upscaling...",
            "orig_dims": [orig_w, orig_h],
            "orig_size_kb": orig_size_kb
        });
        yield Ok(Event::default().data(start_msg.to_string()));

        let mut gpu_args = Vec::new();
        if gpu == "0" {
            gpu_args.extend(["-g".to_string(), "0".to_string()]);
        } else if gpu == "1" {
            gpu_args.extend(["-g".to_string(), "1".to_string()]);
        }

        let models_dir_str = stream_state.models_dir.to_string_lossy().to_string();
        let pct_regex = Regex::new(r"([0-9]+\.?[0-9]*)%").unwrap();

        let run_cmd = |mut cmd: Command, stage_label: &'static str, offset: f64, weight: f64| {
            let pct_regex = pct_regex.clone();
            let start_t = timestamp;
            async move {
                let mut child = match cmd
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to spawn engine process: {}", e);
                        return (Err(e.to_string()), Vec::new());
                    }
                };

                let stderr = child.stderr.take().unwrap();
                let mut reader = BufReader::new(stderr).lines();
                let mut events = Vec::new();

                while let Ok(Some(line)) = reader.next_line().await {
                    if let Some(captures) = pct_regex.captures(&line) {
                        if let Some(matched) = captures.get(1) {
                            if let Ok(val) = matched.as_str().parse::<f64>() {
                                let overall_pct = ((offset + (val * (weight / 100.0))).min(99.0).max(0.5) * 10.0).round() / 10.0;
                                let elapsed = (start_t.elapsed().as_secs_f64() * 10.0).round() / 10.0;

                                let evt = serde_json::json!({
                                    "type": "progress",
                                    "stage": stage_label,
                                    "percent": overall_pct,
                                    "elapsed_sec": elapsed
                                });
                                events.push(evt.to_string());
                            }
                        }
                    }
                }

                let status = child.wait().await;
                (Ok(status.map(|s| s.success()).unwrap_or(false)), events)
            }
        };

        let mut success = false;
        if scale == "8" {
            let pass1_file = stream_state.outputs_dir.join(format!("temp_pass1_{}.png", job_id));
            let mut cmd1 = Command::new(&stream_state.engine_path);
            cmd1.args([
                "-i", input_path.to_str().unwrap(),
                "-o", pass1_file.to_str().unwrap(),
                "-m", &models_dir_str,
                "-n", &model_name,
                "-s", "4",
                "-f", "png",
            ]);
            cmd1.args(&gpu_args);
            if enable_tta { cmd1.arg("-x"); }

            let (res1, events1) = run_cmd(cmd1, "Pass 1/2: 4X Base Neural Reconstruction", 0.0, 50.0).await;
            for ev in events1 { yield Ok(Event::default().data(ev)); }

            if res1.unwrap_or(false) && pass1_file.exists() {
                let mut cmd2 = Command::new(&stream_state.engine_path);
                cmd2.args([
                    "-i", pass1_file.to_str().unwrap(),
                    "-o", output_path.to_str().unwrap(),
                    "-m", &models_dir_str,
                    "-n", "realesr-animevideov3",
                    "-s", "2",
                    "-f", "png",
                ]);
                cmd2.args(&gpu_args);
                if enable_tta { cmd2.arg("-x"); }

                let (res2, events2) = run_cmd(cmd2, "Pass 2/2: 8X Sub-Pixel Synthesis", 50.0, 50.0).await;
                for ev in events2 { yield Ok(Event::default().data(ev)); }

                tokio::fs::remove_file(&pass1_file).await.ok();
                if res2.unwrap_or(false) && output_path.exists() {
                    success = true;
                }
            }
        } else if scale == "2" {
            let mut cmd = Command::new(&stream_state.engine_path);
            cmd.args([
                "-i", input_path.to_str().unwrap(),
                "-o", output_path.to_str().unwrap(),
                "-m", &models_dir_str,
                "-n", &model_name,
                "-s", "2",
                "-f", "png",
            ]);
            cmd.args(&gpu_args);
            if enable_tta { cmd.arg("-x"); }

            let (res, events) = run_cmd(cmd, "2X HD Super-Resolution", 0.0, 100.0).await;
            for ev in events { yield Ok(Event::default().data(ev)); }
            if res.unwrap_or(false) && output_path.exists() {
                success = true;
            }
        } else {
            let mut cmd = Command::new(&stream_state.engine_path);
            cmd.args([
                "-i", input_path.to_str().unwrap(),
                "-o", output_path.to_str().unwrap(),
                "-m", &models_dir_str,
                "-n", &model_name,
                "-s", "4",
                "-f", "png",
            ]);
            cmd.args(&gpu_args);
            if enable_tta { cmd.arg("-x"); }

            let (res, events) = run_cmd(cmd, "4X UHD Super-Resolution", 0.0, 100.0).await;
            for ev in events { yield Ok(Event::default().data(ev)); }
            if res.unwrap_or(false) && output_path.exists() {
                success = true;
            }
        }

        // Clean up input upload immediately
        tokio::fs::remove_file(&input_path).await.ok();

        if success {
            let mut out_w = 0u32;
            let mut out_h = 0u32;
            let mut out_size_kb = 0.0;

            if let Ok(meta) = tokio::fs::metadata(&output_path).await {
                out_size_kb = (meta.len() as f64 / 1024.0 * 10.0).round() / 10.0;
            }
            if let Ok(out_bytes) = tokio::fs::read(&output_path).await {
                if let Ok(img) = image::load_from_memory(&out_bytes) {
                    let (w, h) = img.dimensions();
                    out_w = w;
                    out_h = h;
                }
            }

            let elapsed_total = (timestamp.elapsed().as_secs_f64() * 100.0).round() / 100.0;
            let complete_msg = serde_json::json!({
                "type": "complete",
                "success": true,
                "percent": 100,
                "time_sec": elapsed_total,
                "upscaled_url": format!("/outputs/{}", output_filename),
                "orig_dims": [orig_w, orig_h],
                "out_dims": [out_w, out_h],
                "orig_size_kb": orig_size_kb,
                "out_size_kb": out_size_kb
            });
            yield Ok(Event::default().data(complete_msg.to_string()));
        } else {
            let err_msg = serde_json::json!({
                "type": "error",
                "message": "Upscale processing failed"
            });
            yield Ok(Event::default().data(err_msg.to_string()));
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn serve_output_file(
    State(state): State<AppState>,
    Path(filename): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> Response {
    let clean_filename = PathBuf::from(&filename)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let file_path = state.outputs_dir.join(&clean_filename);
    if !file_path.exists() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => {
            let should_delete = query.download.as_deref() == Some("1")
                || query.download.as_deref() == Some("true");

            if should_delete {
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    tokio::fs::remove_file(file_path).await.ok();
                });
            }

            let mut res = Response::new(Body::from(bytes));
            res.headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));

            if should_delete || query.name.is_some() {
                let dl_name = query.name.as_deref().unwrap_or(&clean_filename);
                if let Ok(hdr_val) = HeaderValue::from_str(&format!("attachment; filename=\"{}\"", dl_name)) {
                    res.headers_mut().insert(header::CONTENT_DISPOSITION, hdr_val);
                }
            }

            res
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response(),
    }
}

async fn serve_upload_file(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Response {
    let clean_filename = PathBuf::from(&filename)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let file_path = state.uploads_dir.join(&clean_filename);
    if !file_path.exists() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => {
            let mut res = Response::new(Body::from(bytes));
            res.headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
            res
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response(),
    }
}
