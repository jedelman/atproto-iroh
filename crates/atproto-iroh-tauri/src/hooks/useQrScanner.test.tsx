// Found in review: the camera MediaStream useQrScanner opens was never
// stopped when the consuming component unmounted mid-scan (e.g.
// navigating away via a Link before a code was found) — only an
// explicit stop() call released it, and a route change never triggers
// that. This proves the fix: a real (mocked) MediaStreamTrack's stop()
// actually gets called on unmount, not just that the component renders.

import { act, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useQrScanner } from "./useQrScanner";

function Probe({ onReady }: { onReady: (start: () => void) => void }) {
  const { start } = useQrScanner(() => {});
  onReady(start);
  return null;
}

describe("useQrScanner", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("stops the camera track when the component unmounts mid-scan", async () => {
    const stop = vi.fn();
    const fakeStream = { getTracks: () => [{ stop }] } as unknown as MediaStream;
    // jsdom doesn't implement navigator.mediaDevices at all — a real
    // browser/Tauri-webview API missing only from the test environment,
    // same category as setup.ts's other polyfills.
    Object.defineProperty(navigator, "mediaDevices", {
      value: { getUserMedia: vi.fn().mockResolvedValue(fakeStream) },
      configurable: true,
    });

    let start: (() => void) | undefined;
    const { unmount } = render(<Probe onReady={(s) => (start = s)} />);

    await act(async () => {
      await start!();
    });

    expect(stop).not.toHaveBeenCalled();
    unmount();
    expect(stop).toHaveBeenCalledTimes(1);
  });
});
