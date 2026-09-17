# ==========================================
# STAGE 1: Build the Rust Axum Backend
# ==========================================
FROM rust:1.80-slim-bookworm AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy real source code
COPY src ./src

# Build release binary
RUN touch src/main.rs && cargo build --release

# ==========================================
# STAGE 2: Minimal Production Runtime
# ==========================================
FROM debian:bookworm-slim

WORKDIR /app

# Install Vulkan runtime, drivers & SSL certs (works on CPU & GPU)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    wget \
    unzip \
    libvulkan1 \
    mesa-vulkan-drivers \
    vulkan-tools \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Download official Linux 64-bit NCNN Vulkan binary
RUN wget -q https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-ubuntu.zip -O /tmp/realesrgan.zip \
    && unzip -q /tmp/realesrgan.zip -d /tmp/realesrgan \
    && mv /tmp/realesrgan/realesrgan-ncnn-vulkan /app/realesrgan-ncnn-vulkan \
    && chmod +x /app/realesrgan-ncnn-vulkan \
    && rm -rf /tmp/realesrgan*

# Copy compiled Rust binary from builder
COPY --from=builder /usr/src/app/target/release/ai-upscale-backend /app/ai-upscale-backend

# Copy all pre-trained NCNN models
COPY models /app/models

# Setup working directories
RUN mkdir -p /app/uploads /app/outputs

# Environment configuration
ENV PORT=8080
ENV MODELS_DIR=/app/models
ENV UPLOADS_DIR=/app/uploads
ENV OUTPUTS_DIR=/app/outputs
ENV ENGINE_PATH=/app/realesrgan-ncnn-vulkan
ENV RUST_LOG=info

EXPOSE 8080

CMD ["/app/ai-upscale-backend"]
