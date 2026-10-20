//! 文件基础设施。对前端只暴露 [`FsDocument`]，内部按路径把 Markdown、TextBundle 和 TextPack 分开处理。

mod asset;
mod atomic_write;
mod classify;
mod document;
mod encoding;
mod image_sniff;
mod markdown;
mod paths;
mod stamp;
mod textbundle;
mod textpack;

pub use document::FsDocument;
