import os
import sys
import time
import subprocess
import json
import re
import threading
from http.server import HTTPServer, SimpleHTTPRequestHandler
from urllib.parse import urlparse, parse_qs
from PIL import Image

PORT = 8080
LAB_DIR = os.path.dirname(os.path.abspath(__file__))
EXE_PATH = os.path.join(LAB_DIR, 'realesrgan-ncnn-vulkan.exe')
UPLOADS_DIR = os.path.join(LAB_DIR, 'uploads')
OUTPUTS_DIR = os.path.join(LAB_DIR, 'outputs')
FILE_TTL_SECONDS = 600  # 10 minutes

os.makedirs(UPLOADS_DIR, exist_ok=True)
os.makedirs(OUTPUTS_DIR, exist_ok=True)

def cleanup_stale_files_loop():
    """Background daemon reaper that removes files older than 10 minutes from uploads and outputs."""
    while True:
        try:
            now = time.time()
            for target_dir in [OUTPUTS_DIR, UPLOADS_DIR]:
                if os.path.exists(target_dir):
                    for fname in os.listdir(target_dir):
                        fpath = os.path.join(target_dir, fname)
                        if os.path.isfile(fpath):
                            file_age = now - os.path.getmtime(fpath)
                            if file_age > FILE_TTL_SECONDS:
                                try:
                                    os.remove(fpath)
                                    print(f"[Auto-Cleanup] Pruned expired temporary file: {fname} (age: {int(file_age)}s)")
                                except Exception:
                                    pass
        except Exception:
            pass
        time.sleep(60)  # Scan every 60 seconds

# Start background reaper thread immediately
threading.Thread(target=cleanup_stale_files_loop, daemon=True).start()

class UpscaleHandler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', '*')
        SimpleHTTPRequestHandler.end_headers(self)

    def do_OPTIONS(self):
        self.send_response(200)
        self.end_headers()

    def do_GET(self):
        parsed = urlparse(self.path)
        params = parse_qs(parsed.query)
        clean_path = parsed.path.lstrip('/')
        
        # Fast direct streaming of output and upload images with proper MIME
        if clean_path.startswith('outputs/') or clean_path.startswith('uploads/'):
            file_path = os.path.join(LAB_DIR, clean_path)
            if os.path.exists(file_path):
                is_download = 'download' in params and params['download'][0] in ('1', 'true')
                custom_name = params.get('name', [None])[0]
                self.send_response(200)
                if file_path.endswith('.png'):
                    self.send_header('Content-Type', 'image/png')
                elif file_path.endswith('.jpg') or file_path.endswith('.jpeg'):
                    self.send_header('Content-Type', 'image/jpeg')
                elif file_path.endswith('.webp'):
                    self.send_header('Content-Type', 'image/webp')
                else:
                    self.send_header('Content-Type', 'application/octet-stream')
                
                size = os.path.getsize(file_path)
                self.send_header('Content-Length', str(size))
                self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
                if is_download:
                    dl_filename = custom_name if custom_name else os.path.basename(file_path)
                    self.send_header('Content-Disposition', f'attachment; filename="{dl_filename}"')
                self.end_headers()
                
                with open(file_path, 'rb') as f:
                    while chunk := f.read(65536):
                        self.wfile.write(chunk)
                
                # Auto-delete output file after download to keep outputs/ folder clean
                if is_download and clean_path.startswith('outputs/'):
                    try:
                        os.remove(file_path)
                    except Exception as del_err:
                        print(f"Output cleanup error: {del_err}")
                return

        SimpleHTTPRequestHandler.do_GET(self)

    def do_POST(self):
        if self.path == '/api/upscale':
            try:
                content_length = int(self.headers.get('Content-Length', 0))
                body = self.rfile.read(content_length)
                
                content_type = self.headers.get('Content-Type', '')
                boundary = content_type.split('boundary=')[-1].encode('utf-8')
                parts = body.split(b'--' + boundary)
                
                file_bytes = None
                model_name = 'realesrgan-x4plus'
                scale = '4'
                gpu_id = 'auto'
                enable_tta = False

                for part in parts:
                    if b'name="image"' in part and b'filename=' in part:
                        header_end = part.find(b'\r\n\r\n')
                        if header_end != -1:
                            file_bytes = part[header_end + 4 : -2]
                    elif b'name="model"' in part:
                        header_end = part.find(b'\r\n\r\n')
                        if header_end != -1:
                            model_name = part[header_end + 4 : -2].decode('utf-8').strip()
                    elif b'name="scale"' in part:
                        header_end = part.find(b'\r\n\r\n')
                        if header_end != -1:
                            scale = part[header_end + 4 : -2].decode('utf-8').strip()
                    elif b'name="gpu"' in part:
                        header_end = part.find(b'\r\n\r\n')
                        if header_end != -1:
                            gpu_id = part[header_end + 4 : -2].decode('utf-8').strip()
                    elif b'name="tta"' in part:
                        header_end = part.find(b'\r\n\r\n')
                        if header_end != -1:
                            val = part[header_end + 4 : -2].decode('utf-8').strip().lower()
                            enable_tta = (val == 'true' or val == '1')

                if not file_bytes:
                    self.send_response(400)
                    self.end_headers()
                    self.wfile.write(b'{"error": "No image found in request"}')
                    return

                # Send SSE headers for live progress streaming
                self.send_response(200)
                self.send_header('Content-Type', 'text/event-stream')
                self.send_header('Cache-Control', 'no-cache')
                self.send_header('Connection', 'close')
                self.end_headers()

                def send_event(data):
                    try:
                        msg = 'data: ' + json.dumps(data) + '\n\n'
                        self.wfile.write(msg.encode('utf-8'))
                        self.wfile.flush()
                    except:
                        pass

                timestamp = int(time.time() * 1000)
                input_filename = f'input_{timestamp}.png'
                output_filename = f'upscaled_{timestamp}.png'
                input_path = os.path.join(UPLOADS_DIR, input_filename)
                output_path = os.path.join(OUTPUTS_DIR, output_filename)

                with open(input_path, 'wb') as f:
                    f.write(file_bytes)

                # Get original dimensions
                orig_w, orig_h = 0, 0
                try:
                    with Image.open(input_path) as img:
                        orig_w, orig_h = img.size
                except:
                    pass

                start_time = time.time()
                send_event({
                    'type': 'start',
                    'message': 'Image loaded. Initializing GPU tensor cores...',
                    'orig_dims': [orig_w, orig_h],
                    'orig_size_kb': round(len(file_bytes) / 1024, 1)
                })

                def build_gpu_args():
                    if gpu_id == '1':
                        return ['-g', '1']
                    elif gpu_id == '0':
                        return ['-g', '0']
                    elif gpu_id == 'auto':
                        return ['-g', '1']
                    return []

                def run_process_with_progress(cmd, pass_label, overall_weight_offset=0, overall_weight_scale=100):
                    proc = subprocess.Popen(
                        cmd,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.STDOUT,
                        text=True,
                        bufsize=1,
                        cwd=LAB_DIR
                    )

                    pct_pattern = re.compile(r'([0-9]+\.?[0-9]*)%')

                    for line in iter(proc.stdout.readline, ''):
                        if not line:
                            break
                        line_str = line.strip()
                        match = pct_pattern.search(line_str)
                        if match:
                            try:
                                raw_pct = float(match.group(1))
                                overall_pct = round(overall_weight_offset + (raw_pct * (overall_weight_scale / 100)), 1)
                                overall_pct = min(99.0, max(0.5, overall_pct))
                                
                                elapsed = time.time() - start_time
                                if overall_pct > 0.5:
                                    total_est = elapsed / (overall_pct / 100.0)
                                    eta_sec = max(0, round(total_est - elapsed, 1))
                                else:
                                    eta_sec = None

                                send_event({
                                    'type': 'progress',
                                    'stage': pass_label,
                                    'percent': overall_pct,
                                    'elapsed_sec': round(elapsed, 1),
                                    'eta_sec': eta_sec
                                })
                            except:
                                pass

                    proc.stdout.close()
                    proc.wait()
                    return proc.returncode

                gpu_flags = build_gpu_args()

                models_dir = os.path.join(LAB_DIR, 'models')

                if scale == '8':
                    pass1_output = os.path.join(OUTPUTS_DIR, f'temp_pass1_{timestamp}.png')
                    cmd1 = [EXE_PATH, '-i', input_path, '-o', pass1_output, '-m', models_dir, '-n', model_name, '-s', '4', '-f', 'png'] + gpu_flags
                    if enable_tta: cmd1.append('-x')

                    ret1 = run_process_with_progress(cmd1, 'Pass 1/2: 4X Base Neural Reconstruction', 0, 50)
                    if ret1 != 0 or not os.path.exists(pass1_output):
                        send_event({'type': 'error', 'message': '8X Pass 1 failed'})
                        return

                    cmd2 = [EXE_PATH, '-i', pass1_output, '-o', output_path, '-m', models_dir, '-n', 'realesr-animevideov3', '-s', '2', '-f', 'png'] + gpu_flags
                    if enable_tta: cmd2.append('-x')

                    ret2 = run_process_with_progress(cmd2, 'Pass 2/2: 8X Sub-Pixel Synthesis', 50, 50)
                    
                    if os.path.exists(pass1_output):
                        try: os.remove(pass1_output)
                        except: pass

                    if ret2 != 0 or not os.path.exists(output_path):
                        send_event({'type': 'error', 'message': '8X Pass 2 failed'})
                        return
                elif scale == '2':
                    cmd = [EXE_PATH, '-i', input_path, '-o', output_path, '-m', models_dir, '-n', model_name, '-s', '2', '-f', 'png'] + gpu_flags
                    if enable_tta: cmd.append('-x')
                    ret = run_process_with_progress(cmd, '2X HD Super-Resolution', 0, 100)
                    if ret != 0 or not os.path.exists(output_path):
                        send_event({'type': 'error', 'message': '2X upscale failed'})
                        return
                else:
                    cmd = [EXE_PATH, '-i', input_path, '-o', output_path, '-m', models_dir, '-n', model_name, '-s', '4', '-f', 'png'] + gpu_flags
                    if enable_tta: cmd.append('-x')
                    ret = run_process_with_progress(cmd, '4X UHD Super-Resolution', 0, 100)
                    if ret != 0 or not os.path.exists(output_path):
                        send_event({'type': 'error', 'message': '4X upscale failed'})
                        return

                elapsed_total = round(time.time() - start_time, 2)
                
                out_w, out_h = 0, 0
                out_size_kb = 0
                try:
                    out_size_kb = round(os.path.getsize(output_path) / 1024, 1)
                    with Image.open(output_path) as img:
                        out_w, out_h = img.size
                except:
                    pass

                # Send complete event
                send_event({
                    'type': 'complete',
                    'success': True,
                    'percent': 100,
                    'time_sec': elapsed_total,
                    'original_url': f'/uploads/{input_filename}',
                    'upscaled_url': f'/outputs/{output_filename}',
                    'orig_dims': [orig_w, orig_h],
                    'out_dims': [out_w, out_h],
                    'orig_size_kb': round(len(file_bytes) / 1024, 1),
                    'out_size_kb': out_size_kb,
                    'model': model_name,
                    'scale': f'{scale}X',
                    'tta': enable_tta
                })

                # Close connection gracefully so browser never freezes
                self.close_connection = True

            except Exception as e:
                try:
                    err_msg = 'data: ' + json.dumps({'type': 'error', 'message': str(e)}) + '\n\n'
                    self.wfile.write(err_msg.encode('utf-8'))
                except:
                    pass
                self.close_connection = True
            finally:
                try:
                    if os.path.exists(input_path):
                        os.remove(input_path)
                except Exception as cleanup_err:
                    print("Cleanup error:", cleanup_err)
        else:
            self.send_error(404, 'Endpoint not found')

    def translate_path(self, path):
        path_clean = urlparse(path).path.lstrip('/')
        if not path_clean:
            path_clean = 'index.html'
        return os.path.join(LAB_DIR, path_clean)

if __name__ == '__main__':
    server = HTTPServer(('127.0.0.1', PORT), UpscaleHandler)
    print('================================================================')
    print(' ✨ Real-ESRGAN Studio (Zero-Freeze & Instant Download Engine)')
    print(f' 🌐 Open: http://127.0.0.1:{PORT}')
    print('================================================================')
    server.serve_forever()
