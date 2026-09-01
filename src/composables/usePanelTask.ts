import { computed, ref, toValue } from "vue";
import { emitPetProgress } from "../utils/petProgress";
import { useTaskPoolStore } from "../stores/taskPool";
import type { NavId } from "../types";

export interface PanelTaskOptions {
  /** 多文件批量场景的任务总数（可传 ref 或 getter 动态跟随文件列表）；缺省表示单文件不确定态 */
  total?: number | (() => number);
  /** 所属面板 id 与任务名：提供后任务登记进全局任务池（侧栏任务池指示器可见），缺省不登记 */
  panelId?: NavId;
  label?: string;
}

/**
 * 面板任务状态：running + 进度计数，配合 TaskProgress 组件展示。
 *
 * - 单文件操作：不传 total，进度条走不确定态动画
 * - 多文件批量：传 total（如文件数），循环内每完成一个调用 tick()
 * - run() 包装器自动处理 start / done（finally 保证异常时也复位；成败同步给宠物表情）
 * - 传 panelId + label 时同步登记全局任务池（SideNav 指示器展示"还有什么在跑"）
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

  /** 任务池登记 id（0 = 未登记；面板未传 panelId 时不进池） */
  let poolId = 0;
  const pooled = computed(() => !!(options.panelId && options.label));

  /** 开始任务（批量场景重置已完成计数；同步给桌面宠物起进度气泡、登记任务池） */
  function start() {
    doneCount.value = 0;
    running.value = true;
    void emitPetProgress({ phase: "start", progress: isIndeterminate.value ? undefined : 0 });
    if (pooled.value) {
      poolId = useTaskPoolStore().begin(options.panelId!, options.label!, {
        progress: isIndeterminate.value ? undefined : 0,
      });
    }
  }

  /** 结束任务 */
  function done() {
    running.value = false;
    if (poolId) {
      const pool = useTaskPoolStore();
      // run() 已按真实成败 end 过则跳过；手动 start/done 模式默认按成功收尾
      const t = pool.tasks.find((x) => x.id === poolId);
      if (t?.running) pool.end(poolId, true);
    }
  }

  /** 批量场景：完成一个文件后调用（同步宠物进度条与任务池进度） */
  function tick() {
    doneCount.value++;
    void emitPetProgress({ phase: "tick", progress: progress.value });
    if (poolId) useTaskPoolStore().update(poolId, { progress: progress.value });
  }

  /** 包装执行：自动 start / done（finally 保证异常时也复位；成败同步给宠物表情与任务池） */
  async function run<T>(fn: () => Promise<T>): Promise<T> {
    start();
    try {
      const result = await fn();
      void emitPetProgress({ phase: "done" });
      if (poolId) useTaskPoolStore().end(poolId, true);
      return result;
    } catch (e) {
      void emitPetProgress({ phase: "error" });
      if (poolId) useTaskPoolStore().end(poolId, false);
      throw e;
    } finally {
      done();
    }
  }

  return { running, doneCount, total, isIndeterminate, progress, start, done, tick, run };
}
