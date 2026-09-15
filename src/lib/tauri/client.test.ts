import { describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

describe("invoke", () => {
  it("passes through a successful call", async () => {
    const { invoke } = await import("./client");
    invokeMock.mockResolvedValueOnce({ id: "1" });

    await expect(invoke("get_current_workspace")).resolves.toEqual({ id: "1" });
    expect(invokeMock).toHaveBeenCalledWith("get_current_workspace", undefined);
  });

  it("passes an already well-formed AppError straight through", async () => {
    const { invoke } = await import("./client");
    const appError = {
      code: "VALIDATION",
      user_message: "Workspace name cannot be empty",
      developer_message: "validation failed: workspace name cannot be empty",
    };
    invokeMock.mockRejectedValueOnce(appError);

    await expect(invoke("create_workspace", { name: "" })).rejects.toEqual(appError);
  });

  it("normalizes an unexpected throw into an AppError shape", async () => {
    const { invoke } = await import("./client");
    invokeMock.mockRejectedValueOnce(new Error("bridge unavailable"));

    await expect(invoke("get_settings")).rejects.toMatchObject({
      code: "INTERNAL",
      developer_message: "bridge unavailable",
    });
  });
});
