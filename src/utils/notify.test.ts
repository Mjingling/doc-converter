import { describe, it, expect, vi, beforeEach } from "vitest";

/**
 * notifyDone 共享通知工具测试：
 * - 窗口可见时跳过
 * - 权限已授予直接发送
 * - 未授权时请求权限，拒绝则不发送
 * - 异常静默不抛出
 */

const mockIsPermissionGranted = vi.fn();
const mockRequestPermission = vi.fn();
const mockSendNotification = vi.fn();
const mockIsVisible = vi.fn();

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: (...args: any[]) => mockIsPermissionGranted(...args),
  requestPermission: (...args: any[]) => mockRequestPermission(...args),
  sendNotification: (...args: any[]) => mockSendNotification(...args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ isVisible: () => mockIsVisible() }),
}));

import { notifyDone } from "./notify";

describe("notifyDone", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("窗口可见时不发送通知", async () => {
    mockIsVisible.mockResolvedValue(true);
    await notifyDone("标题", "正文");
    expect(mockSendNotification).not.toHaveBeenCalled();
  });

  it("窗口不可见且已有权限时发送通知", async () => {
    mockIsVisible.mockResolvedValue(false);
    mockIsPermissionGranted.mockResolvedValue(true);
    await notifyDone("标题", "正文");
    expect(mockSendNotification).toHaveBeenCalledWith({ title: "标题", body: "正文" });
  });

  it("未授权时请求权限，同意后发送", async () => {
    mockIsVisible.mockResolvedValue(false);
    mockIsPermissionGranted.mockResolvedValue(false);
    mockRequestPermission.mockResolvedValue("granted");
    await notifyDone("t", "b");
    expect(mockRequestPermission).toHaveBeenCalled();
    expect(mockSendNotification).toHaveBeenCalledWith({ title: "t", body: "b" });
  });

  it("权限被拒绝时不发送", async () => {
    mockIsVisible.mockResolvedValue(false);
    mockIsPermissionGranted.mockResolvedValue(false);
    mockRequestPermission.mockResolvedValue("denied");
    await notifyDone("t", "b");
    expect(mockSendNotification).not.toHaveBeenCalled();
  });

  it("内部异常静默不抛出", async () => {
    mockIsVisible.mockRejectedValue(new Error("boom"));
    await expect(notifyDone("t", "b")).resolves.toBeUndefined();
  });
});
