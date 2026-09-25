// The startup gate and the mock's matching "call spawn_node first" —
// the pair that would have caught the first-device-install bug where
// nothing ever spawned the node (USER_FLOW.md §3 #1).

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NodeGate } from "./App";
import { api } from "./api";
import { resetMockNodeForTests } from "./api/mockClient";

afterEach(async () => {
  vi.restoreAllMocks();
  await api.spawnNode();
});

describe("mock backend before spawn", () => {
  it("refuses Table calls and answers empty/null where the real backend does", async () => {
    resetMockNodeForTests();
    await expect(api.listMessages("anything")).rejects.toThrow("call spawn_node first");
    await expect(api.createNamespaceWithProfile("x", "individual", null, "y")).rejects.toThrow("call spawn_node first");
    expect(await api.nodeDid()).toBeNull();
    expect(await api.listTables()).toEqual([]);
  });
});

describe("NodeGate", () => {
  it("spawns the node before rendering anything", async () => {
    resetMockNodeForTests();
    const spawn = vi.spyOn(api, "spawnNode");
    render(
      <NodeGate>
        <p>inside the app</p>
      </NodeGate>,
    );
    expect(await screen.findByText("inside the app")).toBeInTheDocument();
    expect(spawn).toHaveBeenCalled();
  });

  it("shows the real error and retries", async () => {
    vi.spyOn(api, "spawnNode").mockRejectedValueOnce(new Error("disk full"));
    render(
      <NodeGate>
        <p>inside the app</p>
      </NodeGate>,
    );
    expect(await screen.findByText("disk full")).toBeInTheDocument();
    expect(screen.queryByText("inside the app")).not.toBeInTheDocument();

    await userEvent.click(screen.getByText("Try again"));
    expect(await screen.findByText("inside the app")).toBeInTheDocument();
  });
});
