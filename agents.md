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
  - The `uploads` directory was accumulating original uploaded images without cleanup. Deleting the file immediately in the backend would break the frontend comparison slider which relied on fetching `/uploads/{filename}` after the upscale finished.
- **Solution & Modified Files**:
  - `index.html`: Refactored the `origImg` logic inside `handleSSEData` to use `URL.createObjectURL(selectedFile)` directly on the client instead of making a network request for the original image.
  - `server.py`: Added cleanup logic at the end of the `/api/upscale` request to `os.remove(input_path)` immediately after processing is complete.
- **Architectural Gotchas / Invariants**:
  - The frontend MUST always use the local `File` object data for rendering the "Before" state. The backend does not persist original uploads after processing completes.

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
