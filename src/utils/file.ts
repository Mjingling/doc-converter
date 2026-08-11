/** 安全提取文件扩展名（无扩展名时返回空字符串，避免全名被误判为 ext） */
export function extOf(path: string): string {
  const base = path.split(/[/\\]/).pop() || "";
  const i = base.lastIndexOf(".");
  return i > 0 ? base.slice(i + 1) : "";
}

/** 返回文件路径的所在目录（兼容 Windows \\ 和 Unix / 分隔符） */
export function dirOf(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(0, idx) : path;
}