import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { usePoll } from "./usePoll";

function setHidden(hidden: boolean) {
  Object.defineProperty(document, "hidden", { configurable: true, get: () => hidden });
}

describe("usePoll", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setHidden(false);
  });
  afterEach(() => {
    vi.useRealTimers();
    setHidden(false);
  });

  it("calls back on the interval while visible, not while hidden", () => {
    const callback = vi.fn();
    renderHook(() => usePoll(callback, 1_000));

    vi.advanceTimersByTime(3_000);
    expect(callback).toHaveBeenCalledTimes(3);

    setHidden(true);
    vi.advanceTimersByTime(3_000);
    expect(callback).toHaveBeenCalledTimes(3);
  });

  it("refreshes right away when the app comes back to the foreground", () => {
    const callback = vi.fn();
    renderHook(() => usePoll(callback, 60_000));

    setHidden(false);
    document.dispatchEvent(new Event("visibilitychange"));
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("stops after unmount", () => {
    const callback = vi.fn();
    const { unmount } = renderHook(() => usePoll(callback, 1_000));
    unmount();
    vi.advanceTimersByTime(5_000);
    document.dispatchEvent(new Event("visibilitychange"));
    expect(callback).not.toHaveBeenCalled();
  });
});
