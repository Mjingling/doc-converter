/**
 * @deprecated 平台默认输出目录已自动设置
 * （Windows: 安装目录/output，macOS: ~/Downloads/docMorph），
 * 不再需要首次提示。保留函数签名以兼容已有面板调用，实际为空操作。
 */
export function triggerOutputDirPrompt(_srcPath: string): void {
  // no-op: 平台默认目录已自动设置
}
