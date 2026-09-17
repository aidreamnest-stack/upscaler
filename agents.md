# Workspace Knowledge & Implementation History

## UI Bug Fixes for AI Upscale Lab
- **Problem & Root Cause**: 
  - The resolution scale label dynamically updated to only display "UHD" regardless of selection.
  - The Side-by-Side comparison mode would display prematurely when clicked before any image was processed, because an empty `src` attribute resolved to the base URL and bypassed the existence check.
  - Results badges lacked the correct scale multiplier display if the backend didn't supply it.
- **Solution & Modified Files**: 
  - `index.html`: Updated `selectScale` to dynamically parse the correct subtitle text.
  - `index.html`: Refactored `setViewMode` to check for `currentUpscaledUrl` instead of `imgAfter.src` to prevent early display of broken placeholders.
  - `index.html`: Added a fallback in `handleSSEData` to use `scaleSelect.value` for badges if the backend omits the scale field.
- **Architectural Gotchas / Invariants**:
  - Always use `currentUpscaledUrl` for determining state validity of upscaled results instead of querying the DOM `src` attributes, since `src=""` evaluates truthfully on some browsers.

## Upload Cleanup & Frontend Object URL
- **Problem & Root Cause**:
  - The `uploads` directory was accumulating original uploaded images without cleanup. Deleting the file immediately in the backend would break the frontend comparison slider which relied on fetching `/uploads/{filename}` after the upscale finished. Also, early returns or exceptions during processing would bypass the cleanup logic completely, leaving files behind.
- **Solution & Modified Files**:
  - `index.html`: Refactored the `origImg` logic inside `handleSSEData` to use `URL.createObjectURL(selectedFile)` directly on the client instead of making a network request for the original image.
  - `server.py`: Added cleanup logic in a `finally:` block at the end of the `/api/upscale` request to `os.remove(input_path)` immediately after processing is complete or if an error occurs.
- **Architectural Gotchas / Invariants**:
  - The frontend MUST always use the local `File` object data for rendering the "Before" state. The backend does not persist original uploads after processing completes.

## Desktop Shortcut Installer
- **Problem & Root Cause**: 
  - Users checking out the repository needed an easy, automated way to create a desktop shortcut to start the tool, rather than manually finding and executing `start.bat`.
- **Solution & Modified Files**: 
  - `Install_App.bat`: Created a new batch script that utilizes PowerShell to programmatically generate an `Image Upscaler.lnk` desktop shortcut pointing directly to the application's `start.bat`.
- **Architectural Gotchas / Invariants**:
  - Do not assume fixed absolute paths for the shortcut target. Always dynamically resolve `%~dp0` to ensure the shortcut works regardless of where the repository is cloned on the user's machine.

## Fake Progress & Bouncing ETA Bug
- **Problem & Root Cause**:
  - The UI progress bar was using a hardcoded, time-based interpolation logic designed for an RTX 3050 (`Math.min(92, ...)`), which artificially pushed the UI progress bar to 92% on slower machines while the actual backend process was only at e.g., 12%. This mismatch caused the ETA calculation to wildly jump around between seconds and minutes because it used the 92% display value instead of the real backend value.
- **Solution & Modified Files**:
  - `index.html`: Removed the fake `currentDisplayPct` frontend interpolation completely. The UI now strictly displays the exact, real-time progress (`latestServerPct`) sent by the Python backend via SSE. Added "Analyzing..." (for 0%) and "Finalizing..." (if stuck >95% for 2s) states to handle the ETA edge cases cleanly without jumping numbers.

## Output Auto-Cleanup After Download
- **Problem & Root Cause**: 
  - After upscaling, the resulting file was stored permanently in `outputs/`. Even after the user clicked "Download", the file remained on disk indefinitely, causing the folder to fill up over time.
- **Solution & Modified Files**: 
  - `server.py`: In the `do_GET` handler, after fully streaming the response for any request containing `?download=1` that targets the `outputs/` folder, the server now immediately calls `os.remove()` to delete the file. The file is only deleted on an explicit download request, never on a plain preview/view request, so the Before/After comparison UI still works correctly.

## Upload Image Preview
- **Problem & Root Cause**: 
  - Upon selecting an image, the main canvas remained empty displaying only the placeholder text until execution started.
- **Solution & Modified Files**: 
  - `index.html`: Added a `.preview-state` container to instantly render the raw uploaded image using `URL.createObjectURL(file)` when a file is dropped or selected.
- **Architectural Gotchas / Invariants**:
  - Preview elements must be manually hidden when transitioning to `comparisonStage` or `sideBySideStage`.

## UI Copy Cleanup
- **Problem & Root Cause**: 
  - The UI contained excessively verbose, AI-generated technical jargon (e.g., "Neural Super-Resolution Studio", "Enterprise Engine") and an unnecessary features banner, making it look generic and cluttered.
- **Solution & Modified Files**: 
  - `index.html`: Replaced placeholder jargon with clean, concise labels. Removed unnecessary feature banners.
- **Architectural Gotchas / Invariants**:
  - Keep UI copy concise and functionally descriptive.

## High-Quality NCNN Community Models Integration
- **Problem & Root Cause**:
  - The default Real-ESRGAN models lacked fine-grained photographic realism and texture synthesis (such as Midjourney-like clarity), and large diffusion upscalers like SUPIR are too heavy for 4GB VRAM.
- **Solution & Modified Files**:
  - `models/`: Downloaded top-tier NCNN-converted community weights (`4x_NMKD-Superscale-SP_178000_G`, `4xLSDIR`, `4xNomos8kSC`, `4x_NMKD-Siax_200k`, `RealESRGAN_General_x4_v3`).
  - `server.py`: Updated backend subprocess execution to explicitly pass `-m models` and use the user's selected model across 2X, 4X, and 8X upscale paths.
  - `index.html`: Enhanced the `#modelSelect` dropdown with categorized optgroups (`Photorealistic & High Detail` vs `Digital Art, Anime & 3D`) and user-friendly labels.
- **Architectural Gotchas / Invariants**:
  - NCNN models (.bin and .param) run natively through `realesrgan-ncnn-vulkan.exe` with zero extra Python runtime overhead or VRAM footprint issues on 4GB GPUs. Always pass explicit `-m models` path to avoid path resolution differences.

## Remaining Time Display Removal
- **Problem & Root Cause**:
  - The dynamic estimated remaining time (`etaVal`) added visual clutter and was prone to fluctuation on variable-length GPU workloads.
- **Solution & Modified Files**:
  - `index.html`: Removed the "Remaining" stat from `.stats-ticking-row` and eliminated all associated ETA calculation logic in JavaScript. The progress bar now cleanly focuses only on **Elapsed time (⏱️)** and **real-time Progress percentage (⚡)**.
- **Architectural Gotchas / Invariants**:
  - Keep the progress overlay minimal: show elapsed seconds and actual server-reported percent without speculative ETA numbers.

## Dokploy-Ready Rust Production Backend
- **Problem & Root Cause**:
  - Deploying a Python script with system dependencies onto Dokploy/VPS created overhead, high memory footprint, and lacked standard containerized API structure for custom domain routing (`api.domain.com`).
- **Solution & Modified Files**:
  - `backend/Cargo.toml`, `backend/src/main.rs`: Implemented a standalone, ultra-low-memory (~10MB) Rust Axum backend with streaming SSE endpoints (`/api/upscale`), model introspection (`/api/models`), and health checks (`/health`).
  - `backend/models/`: Packaged all 8 top-tier NCNN models directly into the backend directory.
  - `backend/Dockerfile`: Created a multi-stage Docker build that bundles the official Linux 64-bit NCNN Vulkan binary and Mesa/Vulkan drivers.
  - `backend/docker-compose.yml`, `backend/README_DOKPLOY.md`: Configured one-click Dokploy deployment guide for domain mapping and SSL termination.
- **Architectural Gotchas / Invariants**:
  - The Linux NCNN Vulkan binary inside Docker works seamlessly on both standard CPU VPS (software Vulkan fallback) and GPU VPS instances (via nvidia-container-toolkit). Always bind `0.0.0.0:3000` for Docker container communication.

## Hybrid Output & Upload Auto-Cleanup (Instant + 10-Min TTL)
- **Problem & Root Cause**:
  - If a user generated an upscale but closed their browser tab without clicking "Download", the generated image remained on disk indefinitely, slowly consuming server storage.
- **Solution & Modified Files**:
  - `server.py`: Added a background daemon thread that scans `outputs/` and `uploads/` every 60s and deletes any file with an `mtime` older than 10 minutes (600 seconds). Instant deletion on download (`?download=1`) remains active.
  - `backend/src/main.rs`: Spawned an asynchronous `tokio` background task implementing the same 10-minute TTL cleanup logic across `outputs/` and `uploads/`.
- **Architectural Gotchas / Invariants**:
  - Keep TTL at 10 minutes (600s). This provides ample time for users to compare Before/After and download, while guaranteeing zero disk accumulation for abandoned sessions.

## Original Filename Preservation with Resolution Suffix
- **Problem & Root Cause**:
  - The download action was assigning a generic randomized timestamp name (`upscaled_master_<timestamp>.png`), discarding the original upload filename.
- **Solution & Modified Files**:
  - `index.html`: Updated `triggerDownload()` to extract the original base filename from `selectedFile.name` and append the appropriate scale suffix (`_2k`, `_4k`, or `_8k`) alongside the original file extension.
  - `server.py`: Enhanced `do_GET` handler to read `?name=` query parameter from download requests and pass it directly into the `Content-Disposition: attachment; filename="..."` HTTP header.
  - `backend/src/main.rs`: Added optional `name` parameter support to `DownloadQuery` and injected the custom filename into the `Content-Disposition` header in the Rust API.
- **Architectural Gotchas / Invariants**:
  - Always preserve the original image filename stem and only append the resolution tag (e.g., `landscape_photo.jpg` -> `landscape_photo_4k.jpg`).

## Git Ignore Configuration for Temporary Folders
- **Problem & Root Cause**:
  - Image files stored in `uploads/` and `outputs/` during local development were appearing as untracked files in Git.
- **Solution & Modified Files**:
  - `.gitignore`: Configured rules to ignore all files within `uploads/*` and `outputs/*` while keeping empty folder structures tracked via `.gitkeep`.
  - Also added standard patterns for `target/`, `__pycache__/`, `.env`, and OS metadata files.
- **Architectural Gotchas / Invariants**:
  - Always maintain `.gitkeep` inside `uploads/` and `outputs/` so that fresh clones contain the required folders.

## 2X Scale Tile Distortion Fix (Lanczos Downscaling for 4X Models)
- **Problem & Root Cause**:
  - In `realesrgan-ncnn-vulkan`, passing `-s 2` with natively 4X-trained models (`realesrgan-x4plus`, `4x_NMKD-Superscale`, etc.) caused internal NCNN tile stitching mismatches, producing visible checkerboard / shifted grid artifact lines across the output image.
- **Solution & Modified Files**:
  - `server.py`: When `scale == '2'`, models with native 2X weights (`realesr-animevideov3`) continue running `-s 2`. For all 4X models, the backend runs a seamless 4X neural pass followed by high-quality Lanczos downsampling to exact `2X` dimensions (`orig_w * 2, orig_h * 2`).
  - `backend/src/main.rs`: Implemented the identical 4X neural pass + Lanczos downsampling pipeline using the `image::imageops` crate in Rust.
- **Architectural Gotchas / Invariants**:
  - Never pass `-s 2` directly to 4X NCNN models through the CLI binary; always run the native 4X neural pass and downscale 50% via Lanczos filter for seamless tile blending.

## Silent Startup Auto-Updater
- **Problem & Root Cause**:
  - Users who cloned the repository had to manually execute `git pull` in a terminal whenever upstream updates or bug fixes were pushed.
- **Solution & Modified Files**:
  - `start.bat`: Added a silent pre-flight `git pull --quiet` check during launch. If a valid Git work tree is detected, it automatically fetches and pulls the latest code before launching `server.py`, with graceful fallback if offline.
- **Architectural Gotchas / Invariants**:
  - Always run `cd /d "%~dp0"` prior to executing Git commands to ensure path resolution remains correct when launched via desktop shortcut.

## Senior-Dev Documentation & Local Isolation
- **Problem & Root Cause**:
  - The project documentation needed to be decoupled from cloud/Dokploy references since the backend API is moving to a separate repository, and provide concise, engineering-focused instructions for Windows local users.
- **Solution & Modified Files**:
  - `README.md` (root & local): Created a comprehensive, technical architecture guide, model catalog, and troubleshooting section with author branding (**JPX**), strictly scoped to the local environment.
- **Architectural Gotchas / Invariants**:
  - Keep local `README.md` free of Dokploy/containerized backend configs.









