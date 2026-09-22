pub mod client;

/// LLM 调用错误的分类。
/// 为什么不直接用 anyhow？因为"能不能重试"取决于"错在哪一层"，
/// 网络抖动值得重试，API key 写错重试一万次也没用。
// thiserror 的派生宏：自动实现 std::error::Error 与 Display，免去手写样板代码
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("配置错误: {0}")]
    Config(String),

    // #[error("…")] 生成 Display：{0} 引用元组变体的第 0 个字段，{body} 引用命名字段
    #[error("网络错误: {0}")]
    Network(String),

    #[error("请求无效（4xx，重试没有意义）: {body}")]
    InvalidRequest { status: u16, body: String },

    #[error("可重试的 HTTP 错误: {status} {body}")]
    Server { status: u16, body: String },

    #[error("其他错误: {0}")]
    Other(String),
}

impl LlmError {
    /// 把 HTTP 响应翻译成错误。注意：要把响应 body 原样保留，
    /// provider 的报错细节（哪个字段错了、限额还剩多少）都在 body 里。
    pub fn from_http(status: u16, body: String) -> Self {
        match status {
            408 | 429 => LlmError::Server { status, body },
            400..=499 => LlmError::InvalidRequest { status, body },
            500..=599 => LlmError::Server { status, body },
            _ => LlmError::Other(format!("HTTP {status}: {body}")),
        }
    }

    /// 这个错误值得重试吗？
    #[allow(dead_code)]
    pub fn is_retryable(&self) -> bool {
        // matches! 宏：判断表达式是否匹配模式并返回 bool；| 是"或"模式，
        // Server { .. } 里的 .. 忽略结构体变体的全部字段
        matches!(self, LlmError::Network(_) | LlmError::Server { .. })
    }
}

#[cfg(test)]
mod tests;
