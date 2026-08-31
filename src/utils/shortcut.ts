import { isMac } from './platform'

/** 全局快捷键默认值（唤起主窗口） */
export const DEFAULT_GLOBAL_SHORTCUT = 'CommandOrControl+Shift+D'

/** 修饰键的 KeyboardEvent.key 值（不参与主键判断） */
const MODIFIER_KEYS = new Set(['Control', 'Shift', 'Alt', 'Meta', 'OS'])

/** 键盘事件中可构造的按键信息（从 KeyboardEvent 提取的最小集合，便于测试） */
export interface ShortcutKeyEvent {
    key: string
    ctrlKey: boolean
    metaKey: boolean
    altKey: boolean
    shiftKey: boolean
}

/**
 * 将键盘事件转换为 Tauri 快捷键格式（如 "CommandOrControl+Shift+D"）。
 * 仅按下修饰键、或按下不支持的特殊键时返回 null（调用方忽略该次按键）。
 */
export function buildShortcutFromEvent(e: ShortcutKeyEvent): string | null {
    const main = normalizeKey(e.key)
    if (!main) return null
    const parts: string[] = []
    if (e.metaKey || e.ctrlKey) parts.push('CommandOrControl')
    if (e.shiftKey) parts.push('Shift')
    if (e.altKey) parts.push('Alt')
    parts.push(main)
    return parts.join('+')
}

/** 主键规范化：字母大写、数字/符号原样、F1-F24 原样；修饰键与其他特殊键返回 null */
function normalizeKey(key: string): string | null {
    if (MODIFIER_KEYS.has(key)) return null
    if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) return key
    if (key.length === 1) return key.toUpperCase()
    return null
}

/** 校验快捷键字符串：至少一个修饰键 + 一个主键（与 Rust 侧解析规则对齐） */
export function isValidShortcut(shortcut: string): boolean {
    const parts = shortcut.split('+').filter(Boolean)
    if (parts.length < 2) return false
    const hasMod = parts.some(
        (p) =>
            p === 'CommandOrControl' ||
            p === 'Shift' ||
            p === 'Alt' ||
            p === 'Super',
    )
    return hasMod
}

/** 展示格式化：macOS 显示为 "⌘⇧D"，其他平台显示为 "Ctrl+Shift+D" */
export function formatShortcut(shortcut: string): string {
    if (!shortcut) return ''
    const map: Record<string, string> = isMac
        ? { CommandOrControl: '⌘', Shift: '⇧', Alt: '⌥', Super: '⌘' }
        : {
              CommandOrControl: 'Ctrl',
              Shift: 'Shift',
              Alt: 'Alt',
              Super: 'Super',
          }
    return shortcut
        .split('+')
        .map((p) => map[p] ?? p)
        .join(isMac ? '' : '+')
}
