// React port of the QR-scan feature the old plain-JS frontend had
// (dist/main.js, deleted along with the rest of it in the React
// rebuild — see README.md's "Not yet ported" note, now closed). Same
// approach as before, same reason: getUserMedia + a decode loop is
// plain web platform API, works the same on desktop and mobile with no
// native plugin — the only maintained Tauri barcode-scanner plugin
// needs an incompatible Tauri 3 alpha. `jsqr` is a real npm dependency
// now instead of a vendored file, since this rebuild has a real
// package manager where the old static-file frontend didn't.
//
// Camera permission on Android: confirmed earlier this session against
// the vendored wry 0.55.1 source that its WebChromeClient.
// onPermissionRequest already handles getUserMedia's video-capture
// request and prompts for android.permission.CAMERA at runtime — this
// app only needed the manifest declaration
// (scripts/patch-android-manifest.sh), unaffected by this rebuild.

import { useCallback, useRef, useState } from "react";
import jsQR from "jsqr";

export type ScanStatus = "idle" | "requesting" | "scanning" | "error";

export function useQrScanner(onDecode: (text: string) => void) {
  const [status, setStatus] = useState<ScanStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const runningRef = useRef(false);

  const stop = useCallback(() => {
    runningRef.current = false;
    streamRef.current?.getTracks().forEach((t) => t.stop());
    streamRef.current = null;
    if (videoRef.current) videoRef.current.srcObject = null;
    setStatus("idle");
  }, []);

  const tick = useCallback(() => {
    if (!runningRef.current) return;
    const video = videoRef.current;
    if (video && video.readyState === video.HAVE_ENOUGH_DATA) {
      canvasRef.current ??= document.createElement("canvas");
      const canvas = canvasRef.current;
      canvas.width = video.videoWidth;
      canvas.height = video.videoHeight;
      const ctx = canvas.getContext("2d");
      if (ctx) {
        ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
        const frame = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const result = jsQR(frame.data, frame.width, frame.height);
        if (result?.data) {
          const text = result.data;
          stop();
          onDecode(text);
          return;
        }
      }
    }
    requestAnimationFrame(tick);
  }, [onDecode, stop]);

  const start = useCallback(async () => {
    if (!navigator.mediaDevices?.getUserMedia) {
      setStatus("error");
      setError("Camera access isn't available here (no getUserMedia — an older webview, or a non-secure context)");
      return;
    }
    setStatus("requesting");
    setError(null);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment" } });
      streamRef.current = stream;
      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        await videoRef.current.play();
      }
      runningRef.current = true;
      setStatus("scanning");
      requestAnimationFrame(tick);
    } catch (err) {
      setStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [tick]);

  return { status, error, videoRef, start, stop };
}
