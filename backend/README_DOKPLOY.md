# 🚀 Dokploy Deployment Guide (api.domain.com)

This backend is a high-performance **Rust (Axum)** AI upscaler service containerized with Docker and Linux NCNN Vulkan AI engine.

---

## 📋 Features
- **API Endpoints**:
  - `GET /health` : Service health & engine status check.
  - `GET /api/models` : List of available AI models (Photorealistic, Anime, Digital Art).
  - `POST /api/upscale` : Multipart image upload with Server-Sent Events (SSE) real-time progress streaming.
  - `GET /outputs/:filename` : Download upscaled image (with auto-delete on `?download=1`).
- **Zero Configuration**: All 8 pre-converted NCNN models are pre-packaged.
- **CPU & GPU Ready**: Works out of the box on standard VPS (CPU) and NVIDIA GPU nodes automatically.
- **Memory Footprint**: ~10MB RAM at runtime.

---

## 🛠️ Step-by-Step Dokploy Deployment

### Step 1: Push / Upload to Git
Push your repository to GitHub / GitLab (or connect your repo in Dokploy).

### Step 2: Create an Application in Dokploy
1. In your **Dokploy Dashboard**, click **Create Service** → **Application**.
2. Select your Git Provider (GitHub/GitLab) and choose this repository.
3. In **Branch**, select `main`.
4. In **Root Directory / Build Path**, enter: `/backend` (or leave as root if repo only contains backend).
5. In **Build Type**, select **Dockerfile** (or **Docker Compose**).

### Step 3: Domain & SSL Setup (`api.domain.com`)
1. Go to the **Domains** tab in your Dokploy application.
2. Add your domain: `api.domain.com`.
3. Set **Port**: `8080`.
4. Enable **HTTPS / LetsEncrypt Certificate** (Dokploy will issue the SSL certificate automatically).

### Step 4: Click Deploy!
Dokploy will build the Docker container and start the service.

---

## 🔍 Verification

Once deployed, you can verify your API:

```bash
# 1. Health check
curl https://api.domain.com/health

# Response:
# {"status":"healthy","service":"ai-upscale-backend-rust","version":"1.0.0","engine_ready":true}

# 2. Get available models
curl https://api.domain.com/api/models
```

---

## 🌐 Connecting Your Frontend

In your frontend application (e.g. Next.js, Vite, or Static HTML), set your API base URL:

```javascript
const API_BASE_URL = "https://api.domain.com";

// POST to /api/upscale
const response = await fetch(`${API_BASE_URL}/api/upscale`, {
  method: "POST",
  body: formData
});
```
