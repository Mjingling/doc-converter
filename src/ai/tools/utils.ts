import { basename, dirname, join } from "@tauri-apps/api/path";

/** 生成带时间戳的输出路径（与输入文件同目录，避免覆盖已有文件） */
export async function outputPathFor(inputPath: string, suffix: string, ext = ""): Promise<string> {
  const dir = await dirname(inputPath);
  const base = await basename(inputPath);
  const dot = base.lastIndexOf(".");
  const name = dot > 0 ? base.slice(0, dot) : base;
  const e = ext || (dot > 0 ? base.slice(dot) : "");
  return join(dir, `${name}_${suffix}_${stamp()}${e}`);
}

/** 从输入文件目录派生输出子目录（如提取图片输出目录） */
export async function outDirFor(inputPath: string, suffix: string): Promise<string> {
  const dir = await dirname(inputPath);
  const base = await basename(inputPath);
  const dot = base.lastIndexOf(".");
  const name = dot > 0 ? base.slice(0, dot) : base;
  return join(dir, `${name}_${suffix}_${stamp()}`);
}

/** 当前时间戳（YYYYMMDD_HHMMSS，用于输出命名去重） */
export function stamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}_${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`;
}

/** 安全解析工具参数（LLM 可能省略字段或类型不符） */
export function strArg(args: Record<string, unknown>, key: string): string | undefined {
  const v = args[key];
  return typeof v === "string" && v.trim() ? v.trim() : undefined;
}

export function numArg(args: Record<string, unknown>, key: string, fallback?: number): number | undefined {
  const v = args[key];
  if (typeof v === "number" && Number.isFinite(v)) return v;
  if (typeof v === "string" && v.trim() !== "" && Number.isFinite(Number(v))) return Number(v);
  return fallback;
}

export function strArrArg(args: Record<string, unknown>, key: string): string[] | undefined {
  const v = args[key];
  if (Array.isArray(v) && v.length > 0 && v.every((x) => typeof x === "string")) return v as string[];
  return undefined;
}
