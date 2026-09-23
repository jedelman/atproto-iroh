import "@testing-library/jest-dom/vitest";

// jsdom doesn't implement File.arrayBuffer() (used by TableDetail's
// photo upload) or URL.createObjectURL/revokeObjectURL (used by
// ImageThumb to render the uploaded bytes back) — real browser and
// Tauri webview APIs, just missing from the test environment. Polyfilled
// here rather than worked around in production code, so the app code
// stays exactly what would run in a real browser.
if (typeof File !== "undefined" && !File.prototype.arrayBuffer) {
  File.prototype.arrayBuffer = function (this: File) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as ArrayBuffer);
      reader.onerror = () => reject(reader.error);
      reader.readAsArrayBuffer(this);
    });
  };
}
if (typeof URL !== "undefined" && !URL.createObjectURL) {
  URL.createObjectURL = () => "blob:mock-url";
  URL.revokeObjectURL = () => {};
}
