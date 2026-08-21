/** 安全提取文件扩展名（无扩展名时返回空字符串，避免全名被误判为 ext） */
export function extOf(path: string): string {
  const base = path.split(/[/\\]/).pop() || "";
  const i = base.lastIndexOf(".");
  return i > 0 ? base.slice(i + 1) : "";
}

/** 返回文件路径的所在目录（兼容 Windows \ 和 Unix / 分隔符） */
export function dirOf(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(0, idx) : path;
}

/** 平台默认输出目录（应用启动时由 settings.hydrate 预计算） */
let _platformDefaultDir = "";

/** 设置平台默认输出目录（settings.hydrate 中调用） */
export function setPlatformDefaultDir(dir: string): void {
  _platformDefaultDir = dir;
}

/**
 * 默认输出目录（三级回退）：
 * 1. 用户设置的全局默认目录（非空时优先）
 * 2. 平台默认目录（Windows: 安装目录/output，macOS: ~/Downloads/docMorph）
 * 3. 源文件所在目录
 * 多源场景（合并 / 图片转 PDF）传第一个源文件路径。
 */
export function defaultOutDir(srcPath: string, settingsDefault?: string): string {
  if (settingsDefault) return settingsDefault;
  if (_platformDefaultDir) return _platformDefaultDir;
  return dirOf(srcPath);
}

/**
 * 默认输出文件路径：源目录（或全局设置）/ 原名 + 后缀 + 扩展名。
 * @param srcPath 源文件路径（多源传第一个）
 * @param suffix 输出后缀，如 "_compressed"
 * @param settingsDefault 设置中的全局默认输出目录（非空时覆盖源同目录）
 * @param ext 输出扩展名，默认 ".pdf"
 */
export function defaultOutputPath(
  srcPath: string,
  suffix: string,
  settingsDefault?: string,
  ext = ".pdf",
): string {
  const dir = defaultOutDir(srcPath, settingsDefault);
  const base = srcPath.split(/[\\/]/).pop() || "output";
  const stem = base.replace(/\.[^.]+$/, "");
  return `${dir}/${stem}${suffix}${ext}`;
}