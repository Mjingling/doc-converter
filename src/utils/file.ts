/** 安全提取文件扩展名（无扩展名时返回空字符串，避免全名被误判为 ext） */
export function extOf(path: string): string {
  const base = path.split(/[/\\]/).pop() || "";
  const i = base.lastIndexOf(".");
  return i > 0 ? base.slice(i + 1) : "";
}