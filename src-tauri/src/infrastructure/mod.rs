//! 文件基础设施。目录且名为 `.textbundle` 的路径由 [`FsTextBundle`] 处理，Markdown 文件由 [`FsMarkdown`] 处理。

mod asset;
mod atomic_write;
mod classify;
mod encoding;
mod image_sniff;
mod markdown;
mod paths;
mod stamp;
mod textbundle;

pub use asset::FsAsset;
pub use classify::FsClassify;
pub use markdown::FsMarkdown;
pub use textbundle::FsTextBundle;
