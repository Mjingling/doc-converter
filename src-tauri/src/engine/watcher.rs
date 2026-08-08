//! 文件夹监控：监听指定目录，新文件出现时按配置的格式规则自动转换
//!
//! 使用 notify crate 的事件驱动监听（macOS 走 FSEvents）。
//! 监控线程在自身线程内创建 watcher 并在退出时销毁，避免跨线程移动监听器。
use crate::engine::format::Format;
use crate::engine::libreoffice::LibreOfficeEngine;
use crate::engine::{light, pdf};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::Emitter;

/// 监控实例句柄（发送 stop_tx 后监控线程退出）
pub struct WatcherHandle {
    pub folder: PathBuf,
    pub stop_tx: mpsc::Sender<()>,
}

/// 上报给前端的转换结果（tauri 事件 watcher-event 的载荷）
#[derive(Serialize, Clone)]
pub struct WatcherEvent {
    pub input: String,
    pub output: Option<String>,
    pub ok: bool,
    pub error: Option<String>,
}

/// 启动文件夹监控；目录无效或监听建立失败时返回 Err
pub fn start_watcher(
    app: tauri::AppHandle,
    folder: PathBuf,
    targets: HashMap<String, String>,
) -> Result<WatcherHandle, String> {
    if !folder.is_dir() {
        return Err("所选路径不是文件夹".to_string());
    }
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let handle = WatcherHandle {
        folder: folder.clone(),
        stop_tx,
    };
    thread::spawn(move || run_watch_loop(app, folder, targets, stop_rx));
    Ok(handle)
}

/// 监控主循环：接收文件创建事件，按扩展名查规则并转换
fn run_watch_loop(
    app: tauri::AppHandle,
    folder: PathBuf,
    targets: HashMap<String, String>,
    stop_rx: mpsc::Receiver<()>,
) {
    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
    let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("创建文件夹监控失败: {e}");
            return;
        }
    };
    if let Err(e) = watcher.watch(&folder, RecursiveMode::NonRecursive) {
        eprintln!("添加监控目录失败: {e}");
        return;
    }
    let mut recent = RecentFiles::new();
    loop {
        // 停止信号优先处理
        if stop_rx.try_recv().is_ok() {
            break;
        }
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(event)) if matches!(event.kind, EventKind::Create(_)) => {
                for path in event.paths {
                    if !path.is_file() || is_ignored(&path) || recent.contains(&path) {
                        continue;
                    }
                    recent.insert(path.clone());
                    // 等待文件写完（写盘中事件可能先于内容落盘到达）
                    thread::sleep(Duration::from_millis(800));
                    if !path.is_file() {
                        continue;
                    }
                    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                        continue;
                    };
                    let ext = ext.to_lowercase();
                    // 未配置该格式规则，或目标与源相同则跳过
                    let Some(target) = targets.get(&ext) else {
                        continue;
                    };
                    if target == &ext {
                        continue;
                    }
                    handle_new_file(&app, &path, target);
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => eprintln!("文件夹监控错误: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    // watcher 在此线程 drop，释放目录监听
}

/// 转换单个文件并把结果通过 watcher-event 事件上报前端
fn handle_new_file(app: &tauri::AppHandle, input: &Path, target_ext: &str) {
    let input_s = input.to_string_lossy().to_string();
    match convert_file(input, target_ext) {
        Ok(out) => {
            let _ = app.emit(
                "watcher-event",
                WatcherEvent {
                    input: input_s,
                    output: Some(out.to_string_lossy().to_string()),
                    ok: true,
                    error: None,
                },
            );
        }
        Err(e) => {
            eprintln!("自动转换失败 {input_s}: {e}");
            let _ = app.emit(
                "watcher-event",
                WatcherEvent {
                    input: input_s,
                    output: None,
                    ok: false,
                    error: Some(e),
                },
            );
        }
    }
}

/// 按文件类型选择转换路径：
/// - 图片 → PDF：内置引擎合成
/// - 轻量引擎支持的格式（txt/md/html→pdf、docx→txt/html/md、xlsx→csv、pptx→txt、epub→txt/html/md）：零依赖转换
/// - 其余：LibreOffice 引擎（不可用时报错）
/// 输出到源文件所在目录，同名不同扩展名
fn convert_file(input: &Path, target_ext: &str) -> Result<PathBuf, String> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "无效的文件名".to_string())?;
    let out = parent.join(format!("{}.{}", stem, target_ext));

    // 图片 → PDF
    if is_image_ext(&ext) {
        if target_ext != "pdf" {
            return Err(format!("图片仅支持转换为 PDF，当前目标为 {}", target_ext));
        }
        pdf::images_to_pdf(&[input.to_path_buf()], &out, "auto")?;
        return Ok(out);
    }

    // 轻量引擎支持的目标直接转换
    let light_supported = Format::from_ext(&ext)
        .map(|f| f.light_targets().iter().any(|t| t.ext() == target_ext))
        .unwrap_or(false);
    if light_supported {
        return light::convert_light(input, target_ext, parent);
    }

    // 其余格式需要 LibreOffice 引擎
    let engine = LibreOfficeEngine::detect();
    if !engine.available() {
        return Err("需要 LibreOffice 引擎".to_string());
    }
    engine.convert(input, target_ext, parent)
}

fn is_image_ext(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp")
}

/// 跳过隐藏文件（. 开头、~ 结尾）与常见临时文件
fn is_ignored(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.starts_with('.') || name.ends_with('~') {
        return true;
    }
    let lower = name.to_lowercase();
    lower.ends_with(".tmp")
        || lower.ends_with(".temp")
        || lower.ends_with(".part")
        || lower.ends_with(".crdownload")
}

/// 近期处理过的文件缓存：同一路径在 TTL 内不重复转换（部分程序会触发多次 Create 事件）
struct RecentFiles {
    files: VecDeque<(PathBuf, Instant)>,
    ttl: Duration,
    max: usize,
}

impl RecentFiles {
    fn new() -> Self {
        Self {
            files: VecDeque::new(),
            ttl: Duration::from_secs(5),
            max: 512,
        }
    }

    fn contains(&mut self, p: &Path) -> bool {
        self.files.retain(|(_, t)| t.elapsed() < self.ttl);
        self.files.iter().any(|(f, _)| f == p)
    }

    fn insert(&mut self, p: PathBuf) {
        if self.files.len() >= self.max {
            self.files.pop_front();
        }
        self.files.push_back((p, Instant::now()));
    }
}
