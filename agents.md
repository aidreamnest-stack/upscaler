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
  - `index.html`: Replaced exaggerated wording with standard, user-friendly labels (e.g., "Image Upscaler", "Model", "GPU Device"). Removed the `features-grid` section entirely for a cleaner layout.
- **Architectural Gotchas / Invariants**:
  - Keep UI copy concise and functionally descriptive. Avoid adding unnecessary feature showcase banners for single-purpose local tools.
