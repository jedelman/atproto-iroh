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

import { useCallback, useEffect, useRef, useState } from "react";
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

  // Found in review, second pass: the first unmount fix below only
  // stops a stream already assigned to streamRef — it doesn't cover
  // unmounting while status is still "requesting" (the OS/browser
  // permission prompt hasn't been answered yet). getUserMedia() can
  // resolve *after* that unmount already ran its cleanup once, and
  // nothing was left to catch a stream that shows up late — the camera
  // would turn on with no mounted component left to ever stop it.
  // mountedRef makes start()'s own continuation the thing that checks.
  const mountedRef = useRef(true);

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
      if (!mountedRef.current) {
        // The component was unmounted while the permission prompt was
        // still pending — there's no video element or scan loop left
        // to hand this to, so shut it straight down instead of turning
        // the camera on for nobody.
        stream.getTracks().forEach((t) => t.stop());
        return;
      }
      streamRef.current = stream;
      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        await videoRef.current.play();
      }
      runningRef.current = true;
      setStatus("scanning");
      requestAnimationFrame(tick);
    } catch (err) {
      if (!mountedRef.current) return;
      setStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [tick]);

  // Found in review: nothing stopped the camera when the consuming
  // component unmounted mid-scan (e.g. navigating away via a Link
  // while `status === "scanning"`) — only an explicit stop() call did,
  // which a route change never triggers. Left the live tracks running
  // and the browser's camera indicator on indefinitely. `stop` is
  // referenced via a ref rather than as a dependency so this effect's
  // cleanup always runs, not just on the render where `stop`'s
  // (stable, but this is defensive) identity happened to change.
  const stopRef = useRef(stop);
  stopRef.current = stop;
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      stopRef.current();
    };
  }, []);

  return { status, error, videoRef, start, stop };
}
