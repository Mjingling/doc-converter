import { computed, ref, toValue } from "vue";

export interface PanelTaskOptions {
  /** 多文件批量场景的任务总数（可传 ref 或 getter 动态跟随文件列表）；缺省表示单文件不确定态 */
  total?: number | (() => number);
}

/**
 * 面板任务状态：running + 进度计数，配合 TaskProgress 组件展示。
 *
 * - 单文件操作：不传 total，进度条走不确定态动画
 * - 多文件批量：传 total（如文件数），循环内每完成一个调用 tick()
 * - run() 包装器自动处理 start/done（含异常路径），面板只需把原有逻辑放进去
 */
export function usePanelTask(options: PanelTaskOptions = {}) {
  const running = ref(false);
  const doneCount = ref(0);

  /** 任务总数（0 表示单文件不确定态） */
  const total = computed(() => (options.total !== undefined ? toValue(options.total) : 0));
  const isIndeterminate = computed(() => total.value <= 0);
  const progress = computed(() => {
    if (isIndeterminate.value) return 0;
    return Math.min(100, Math.round((doneCount.value / total.value) * 100));
  });

  /** 开始任务（批量场景重置已完成计数） */
  function start() {
    doneCount.value = 0;
    running.value = true;
  }

  /** 结束任务 */
  function done() {
    running.value = false;
  }

  /** 批量场景：完成一个文件后调用 */
  function tick() {
    doneCount.value++;
  }

  /** 包装执行：自动 start / done（finally 保证异常时也复位） */
  async function run<T>(fn: () => Promise<T>): Promise<T> {
    start();
    try {
      return await fn();
    } finally {
      done();
    }
  }

  return { running, doneCount, total, isIndeterminate, progress, start, done, tick, run };
}
