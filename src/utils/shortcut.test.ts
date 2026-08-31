import { describe, expect, it, vi } from 'vitest'

// 固定平台为非 macOS，测 "Ctrl+Shift+D" 分支；mac 分支用 vi.mock 单测
vi.mock('./platform', () => ({ isMac: false }))

import {
    buildShortcutFromEvent,
    formatShortcut,
    isValidShortcut,
    DEFAULT_GLOBAL_SHORTCUT,
} from './shortcut'

describe('buildShortcutFromEvent', () => {
    it('构造 Ctrl+Shift+D（非 mac 平台 ctrlKey 映射为 CommandOrControl）', () => {
        const e = {
            key: 'd',
            ctrlKey: true,
            metaKey: false,
            altKey: false,
            shiftKey: true,
        }
        expect(buildShortcutFromEvent(e)).toBe('CommandOrControl+Shift+D')
    })

    it('mac metaKey 映射为 CommandOrControl', () => {
        const e = {
            key: 'e',
            ctrlKey: false,
            metaKey: true,
            altKey: false,
            shiftKey: false,
        }
        expect(buildShortcutFromEvent(e)).toBe('CommandOrControl+E')
    })

    it('Alt 组合正确排序', () => {
        const e = {
            key: 'f',
            ctrlKey: true,
            metaKey: false,
            altKey: true,
            shiftKey: true,
        }
        expect(buildShortcutFromEvent(e)).toBe('CommandOrControl+Shift+Alt+F')
    })

    it('F 键可用', () => {
        const e = {
            key: 'F5',
            ctrlKey: false,
            metaKey: false,
            altKey: true,
            shiftKey: false,
        }
        expect(buildShortcutFromEvent(e)).toBe('Alt+F5')
    })

    it('仅修饰键（无主键）返回 null', () => {
        const e = {
            key: 'Shift',
            ctrlKey: false,
            metaKey: false,
            altKey: false,
            shiftKey: true,
        }
        expect(buildShortcutFromEvent(e)).toBeNull()
    })

    it('不支持的特殊键（如 Home）返回 null', () => {
        const e = {
            key: 'Home',
            ctrlKey: true,
            metaKey: false,
            altKey: false,
            shiftKey: false,
        }
        expect(buildShortcutFromEvent(e)).toBeNull()
    })

    it('无修饰键的普通按键返回主键（是否允许由 Rust 注册校验决定）', () => {
        const e = {
            key: 'a',
            ctrlKey: false,
            metaKey: false,
            altKey: false,
            shiftKey: false,
        }
        expect(buildShortcutFromEvent(e)).toBe('A')
    })
})

describe('isValidShortcut', () => {
    it('默认快捷键合法', () => {
        expect(isValidShortcut(DEFAULT_GLOBAL_SHORTCUT)).toBe(true)
    })

    it('无修饰键不合法', () => {
        expect(isValidShortcut('D')).toBe(false)
        expect(isValidShortcut('Shift')).toBe(false)
    })

    it('空字符串不合法', () => {
        expect(isValidShortcut('')).toBe(false)
    })
})

describe('formatShortcut', () => {
    it('非 mac 平台使用 Ctrl 展开显示', () => {
        expect(formatShortcut('CommandOrControl+Shift+D')).toBe('Ctrl+Shift+D')
    })

    it('空字符串返回空', () => {
        expect(formatShortcut('')).toBe('')
    })
})
