# AI Upscale Lab — Local High-Precision Super-Resolution Studio

A lightweight, high-performance local AI image upscaler for Windows. Powered by **NCNN Vulkan GPU compute shaders**, a **zero-dependency Python streaming backend**, and a **pure vanilla web UI** with real-time Before/After split comparison.

No heavy PyTorch/CUDA runtime, no cloud dependencies, and zero telemetry. Runs 100% on-device.

---

## 🏗️ System Architecture

```
                       ┌───────────────────────────────┐
                       │   Frontend UI (index.html)    │
                       │  - Split Curtain Comparison   │
                       │  - SSE Progress & Stopwatch   │
                       │  - Auto Filename Preservation │
                       └──────────────┬────────────────┘
                                      │ HTTP / SSE (EventSource)
                                      ▼
                       ┌───────────────────────────────┐
                       │      Backend (server.py)      │
                       │  - Native HTTP & SSE Streamer │
                       │  - GPU Subprocess Dispatcher  │
                       │  - Hybrid 10-Min TTL Reaper   │
                       │  - Lanczos Post-Processing    │
                       └──────────────┬────────────────┘
                                      │ Subprocess CLI (-m models -n <model>)
                                      ▼
                       ┌───────────────────────────────┐
                       │  realesrgan-ncnn-vulkan.exe   │
                       │  - Vulkan Compute Engine      │
                       │  - FP16 Tensor Processing     │
                       │  - GPU VRAM Auto-Tiling       │
                       └──────────────┬────────────────┘
                                      │ Weights
                                      ▼
                       ┌───────────────────────────────┐
                       │  NCNN Model Weights (models/) │
                       │  - NMKD, LSDIR, Nomos, ESRGAN │
                       └───────────────────────────────┘
```

---

## ✨ Key Engineering Features

1. **Bare-Metal GPU Acceleration**:
   - Executes directly on GPU tensor cores via Vulkan API. No PyTorch, CUDA, or 5GB environment installations required.
   - Compatible with NVIDIA (GeForce/RTX), AMD (Radeon/RDNA), and Intel GPUs.

2. **Real-Time SSE Progress Streaming**:
   - Asynchronous Server-Sent Events (`text/event-stream`) stream real-time percentage and elapsed stopwatch timer directly from the inference engine stdout to the UI.

3. **2X Scale Tile Distortion Fix**:
   - Native 4X models can suffer from tile stitching seams when forced to run at 2X. The engine automatically executes a seamless 4X neural pass followed by high-quality **Lanczos downsampling** to exact `2X` dimensions (`orig_w * 2, orig_h * 2`).

4. **Zero-Accumulation Storage Architecture (Hybrid TTL)**:
   - **On Download**: Output files requested with `?download=1` are immediately purged from disk after stream delivery.
   - **Background Reaper**: A daemon thread sweeps `outputs/` and `uploads/` every 60s, purging abandoned files older than 10 minutes (600s TTL).

5. **Original Filename & Extension Preservation**:
   - Original filename stems are preserved on download with automatic resolution tagging (`photo.jpg` ➔ `photo_4k.jpg`, `banner.png` ➔ `banner_2k.png`).

6. **Silent Auto-Updater**:
   - `start.bat` performs a silent pre-flight check (`git pull --quiet`) on every launch to fetch upstream model/UI improvements without manual user intervention.

---

## 📋 Prerequisites

- **OS**: Windows 10 / 11 (64-bit)
- **Python**: Python 3.8+ (Standard installation with `Pillow`)
- **GPU**: Vulkan-compatible GPU (Dedicated NVIDIA RTX/GTX or AMD Radeon recommended)

```bash
# Verify Python & Pillow dependency
python -m pip install Pillow
```

---

## 🚀 Quick Start

### Option A: Install Desktop Shortcut (Recommended)
1. Double-click `Install_App.bat`.
2. A shortcut named **"Image Upscaler"** will be created on your Desktop with the app icon.
3. Double-click the shortcut to launch the app anytime.

### Option B: Manual CLI Launch
```bash
# From the project directory:
python server.py

# Open your browser:
# http://127.0.0.1:8080
```

---

## 📁 Repository Structure

```
ai-upscale-lab/
├── Install_App.bat            # Automated Desktop shortcut installer (PowerShell)
├── start.bat                  # One-click launcher with silent git auto-update
├── server.py                  # HTTP server, SSE progress streaming & cleanup daemon
├── index.html                 # Interactive studio UI (Split slider, dark theme, metrics)
├── icon.ico                   # Application shortcut icon
├── realesrgan-ncnn-vulkan.exe # Linux/Windows NCNN Vulkan binary engine
├── vcomp140.dll / vcomp140d.dll# OpenMP runtime libraries
├── models/                    # Pre-compiled NCNN neural models (.bin & .param)
│   ├── realesrgan-x4plus.*            # Standard Classic General Upscaler
│   ├── 4x_NMKD-Superscale-SP_178000_G.*# Photorealistic Texture & Detail Upscaler
│   ├── 4xLSDIR.*                      # Large-Scale Natural Photo Restoration
│   ├── 4xNomos8kSC.*                  # Cinematic Landscapes & Scenery
│   ├── RealESRGAN_General_x4_v3.*     # Fast Lightweight General v3
│   ├── 4x_NMKD-Siax_200k.*            # Digital Art, 3D Renders & CGI
│   ├── realesrgan-x4plus-anime.*      # 2D Anime & Cartoons
│   └── realesr-animevideov3-*.*       # Ultra-Fast Smooth Video/Anime (2X/3X/4X)
├── uploads/                   # Temporary upload cache (Auto-cleaned, gitignored)
└── outputs/                   # Temporary output cache (Auto-cleaned, gitignored)
```

---

## 🧠 Neural Model Catalog

| Model Identifier | Category | Scaling | Best Use Case |
|---|---|---|---|
| **`realesrgan-x4plus`** *(Default)* | Photorealistic | 2X / 4X / 8X | Balanced default for real-world photos and graphics. |
| **`4x_NMKD-Superscale`** | Photorealistic | 2X / 4X / 8X | Highest skin, hair, and cloth texture fidelity (Midjourney-style). |
| **`4xLSDIR`** | Photorealistic | 2X / 4X / 8X | Natural landscapes and diverse real-world photographic scenes. |
| **`4xNomos8kSC`** | Photorealistic | 2X / 4X / 8X | Micro-textures, architecture, and scenery sharpness. |
| **`RealESRGAN_General_x4_v3`** | Photorealistic | 2X / 4X / 8X | Lightweight and fast with mild denoising. |
| **`4x_NMKD-Siax_200k`** | Art & 3D | 2X / 4X / 8X | AI-generated art, CGI, 3D renderings, and digital paintings. |
| **`realesrgan-x4plus-anime`** | Anime / 2D | 2X / 4X / 8X | High-contrast line art, manga, and cartoons. |
| **`realesr-animevideov3`** | Anime / Fast | 2X / 4X / 8X | Native multi-scale, ultra-fast smooth anime upscaling. |

---

## ⚙️ Configuration & Customization

All primary configurations can be modified directly in `server.py`:

```python
PORT = 8080               # Port for the local web server
FILE_TTL_SECONDS = 600    # Background cleanup TTL (Default: 10 minutes)
```

### Adding Custom Models
To add any community ESRGAN model converted to NCNN format:
1. Place `<model_name>.bin` and `<model_name>.param` in the `models/` directory.
2. Add the `<option value="<model_name>">` entry inside the `#modelSelect` dropdown in `index.html`.
3. The server will automatically dispatch CLI arguments `-m models -n <model_name>` dynamically.

---

## 🔧 Developer & Troubleshooting Notes

### GPU Device Selection
- By default, GPU device `auto` utilizes the primary dedicated GPU (e.g., NVIDIA RTX 3050).
- If multi-GPU setup is detected, you can toggle between Device `1` (NVIDIA) and Device `0` (Integrated AMD/Intel) from the UI dropdown.

### Test-Time Augmentation (TTA)
- TTA performs 8-pass rotational inference for anti-aliasing edge cases.
- *Note:* TTA increases processing time by ~8x. Keep disabled for standard real-time usage.

### Port Conflicts
- If port `8080` is in use by another local service (e.g., Tomcat, Docker), change `PORT = <custom_port>` in `server.py` and `start.bat`.

---

## 📄 License & Attribution

- Real-ESRGAN Engine by [Xintao Wang et al.](https://github.com/xinntao/Real-ESRGAN)
- NCNN Vulkan framework by [Tencent / Nihui](https://github.com/Tencent/ncnn)
- Custom weights provided by community modelers via [OpenModelDB](https://openmodeldb.info/) & [Upscayl](https://github.com/upscayl/custom-models).

---

## 👨‍💻 Author

- Built & Maintained by **JPX**
