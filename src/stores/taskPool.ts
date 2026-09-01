/**
 * 任务池 store：跨面板的任务注册表（全局可见"还有什么在跑"）
 *
 * 面板任务（usePanelTask 或直接调用 store API）注册到这里；
 * SideNav 任务池指示器读取 runningTasks 展示数量与列表，完成后短暂驻留供"已完成"提示。
 * 任务本体仍在各面板闭包中执行——本池只登记状态，不负责调度。
 */
import { defineStore } from "pinia";
import type { NavId } from "../types";

/** 池内任务条目 */
export interface PoolTask {
  /** 自增 id（usePanelTask 持有，用于 update/end） */
  id: number;
  /** 所属功能面板（点击「前往」跳转用；convert 场景统一记 'convert'） */
  panelId: NavId;
  /** 展示名（如「拆分 PDF」「批量处理」） */
  label: string;
  running: boolean;
  /** 结束时的结果（running=false 时有效） */
  ok?: boolean;
  /** 0-100 百分比；undefined = 不确定态 */
  progress?: number;
  startedAt: number;
  endedAt?: number;
}

/** 已完成任务在池内的驻留时长（供"✓ 已完成"提示），超时由指示器清理 */
export const FINISHED_LINGER_MS = 2500;

let nextId = 1;

export const useTaskPoolStore = defineStore("taskPool", {
  state: () => ({
    tasks: [] as PoolTask[],
  }),
  getters: {
    /** 进行中的任务 */
    runningTasks(state): PoolTask[] {
      return state.tasks.filter((t) => t.running);
    },
    /** 刚完成的任务（驻留窗口内，供完成提示） */
    justFinished(state): PoolTask[] {
      const now = Date.now();
      return state.tasks.filter(
        (t) => !t.running && t.endedAt !== undefined && now - t.endedAt < FINISHED_LINGER_MS,
      );
    },
  },
  actions: {
    /** 登记新任务并返回其 id */
    begin(panelId: NavId, label: string, opts?: { progress?: number }): number {
      const id = nextId++;
      this.tasks.push({
        id,
        panelId,
        label,
        running: true,
        progress: opts?.progress,
        startedAt: Date.now(),
      });
      return id;
    },
    /** 更新任务（进度等）；id 不存在静默忽略（容忍异常时序） */
    update(id: number, patch: Pick<PoolTask, "progress">) {
      const t = this.tasks.find((x) => x.id === id);
      if (t) Object.assign(t, patch);
    },
    /** 结束任务（成功/失败），记录结束时间供驻留提示 */
    end(id: number, ok: boolean) {
      const t = this.tasks.find((x) => x.id === id);
      if (t) {
        t.running = false;
        t.ok = ok;
        t.endedAt = Date.now();
      }
    },
    /** 清掉驻留窗口外的已完成任务（指示器定时调用，防列表无限增长） */
    sweep() {
      const now = Date.now();
      this.tasks = this.tasks.filter(
        (t) => t.running || t.endedAt === undefined || now - t.endedAt < FINISHED_LINGER_MS,
      );
    },
  },
});
