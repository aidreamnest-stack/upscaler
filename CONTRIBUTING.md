# Contributing to AI Upscale Lab

Thank you for your interest in contributing! To maintain a clean and reliable codebase, please follow the guidelines below.

---

## 🌿 Branching Strategy

- **`main`**: Production release branch (Protected, direct push disabled).
- **`Dev-ache-😂`** (or active dev branch): Default development branch for all active pull requests and features.

> [!IMPORTANT]
> **Always target your Pull Requests against the development branch (`Dev-ache-😂`), NOT `main`.**

---

## 🛠️ Contribution Workflow

1. **Fork the Repository**:
   Click the **Fork** button at the top-right of the GitHub repository page.

2. **Clone your Fork locally**:
   ```bash
   git clone https://github.com/<your-username>/upscaler.git
   cd upscaler
   ```

3. **Create a Feature Branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

4. **Make & Test your Changes**:
   - Verify that `server.py` starts without syntax errors: `python server.py`
   - Test image upscaling on a local GPU / Vulkan device.
   - Ensure temporary files in `uploads/` and `outputs/` are not committed (covered by `.gitignore`).

5. **Commit & Push**:
   ```bash
   git add .
   git commit -m "feat: concise description of your changes"
   git push origin feature/your-feature-name
   ```

6. **Submit a Pull Request (PR)**:
   - Open a PR on GitHub targeting the development branch.
   - Describe what problem your PR solves and what was tested.

---

## 📋 Code Standards

- **Zero Heavy Frameworks**: Keep the local Windows runner lightweight (pure Python standard library + Pillow).
- **GPU Invariants**: Maintain NCNN `-m models` argument resolution and Lanczos 2X downsampling logic for 4X models.
- **Documentation**: If adding new models or features, update the model catalog in `README.md`.
