//! 字符串级批量翻译队列
//!
//! 对已加载的字符串进行并发批量翻译。独立于文件级批处理。
//! 本模块无 tokio 依赖，由调用方（Tauri 命令）处理异步调度。

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// 单条翻译输入
#[derive(Clone, Debug)]
pub struct BatchItem {
    pub str_id: u32,
    pub source_text: String,
}

/// 单条翻译结果
#[derive(Clone, Debug, Serialize)]
pub struct BatchResult {
    pub str_id: u32,
    pub translated: String,
    pub error: Option<String>,
}

/// 批量翻译进度
#[derive(Clone, Debug, Serialize)]
pub struct BatchProgress {
    pub completed: u32,
    pub total: u32,
}

/// 批量翻译完成汇总
#[derive(Clone, Debug, Serialize)]
pub struct BatchSummary {
    pub total: u32,
    pub succeeded: u32,
    pub failed: u32,
    pub errors: Vec<BatchErrorEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BatchErrorEntry {
    pub str_id: u32,
    pub source: String,
    pub error: String,
}

/// 批量翻译队列（同步控制，用于异步调用方）
pub struct BatchQueue {
    #[allow(dead_code)]
    concurrency: u8,
    cancel_flag: Arc<AtomicBool>,
    completed: AtomicU32,
    total: u32,
}

impl BatchQueue {
    pub fn new(concurrency: u8, total: u32) -> Self {
        Self {
            concurrency: concurrency.clamp(1, 10),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            completed: AtomicU32::new(0),
            total,
        }
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    /// 获取下一个待处理的 job（调用方持有信号量保证并发数）
    pub fn try_acquire(&self) -> Option<usize> {
        if self.is_cancelled() {
            return None;
        }
        let c = self.completed.load(Ordering::SeqCst);
        if c >= self.total {
            return None;
        }
        Some(c as usize)
    }

    /// 标记一个 job 完成并获取进度
    pub fn mark_done(&self) -> BatchProgress {
        let c = self.completed.fetch_add(1, Ordering::SeqCst) + 1;
        BatchProgress {
            completed: c,
            total: self.total,
        }
    }

    pub fn get_progress(&self) -> BatchProgress {
        BatchProgress {
            completed: self.completed.load(Ordering::SeqCst),
            total: self.total,
        }
    }
}

/// 重试逻辑：指数退避，最多 3 次（同步版本）
pub fn translate_with_retry<F>(source: &str, translate: &F) -> Result<String, String>
where
    F: Fn(&str) -> Result<String, String>,
{
    let mut delay = 1u64;
    let mut last_err = String::new();

    for attempt in 0..3 {
        match translate(source) {
            Ok(text) if !text.is_empty() => return Ok(text),
            Ok(_) => return Ok(String::new()),
            Err(e) => {
                last_err = e;
                let is_retriable = last_err.contains("timeout")
                    || last_err.contains("429")
                    || last_err.contains("503")
                    || last_err.contains("502");

                if !is_retriable || attempt == 2 {
                    return Err(last_err);
                }

                std::thread::sleep(std::time::Duration::from_secs(delay));
                delay *= 2;
            }
        }
    }

    Err(last_err)
}
