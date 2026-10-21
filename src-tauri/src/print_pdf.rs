use crate::app::error::{AppError, AppResult};

#[cfg(windows)]
pub fn export_pdf<R: tauri::Runtime>(window: tauri::WebviewWindow<R>, path: String) -> AppResult<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    window
        .with_webview(move |platform| {
            let result = print_to_pdf(&platform, &path);
            let _ = tx.send(result);
        })
        .map_err(|err| AppError::msg(format!("无法访问页面：{err}")))?;
    rx.recv().unwrap_or_else(|_| Err(AppError::msg("导出 PDF 已取消")))
}

#[cfg(not(windows))]
pub fn export_pdf<R: tauri::Runtime>(_window: tauri::WebviewWindow<R>, _path: String) -> AppResult<()> {
    Err(AppError::msg("当前平台不支持导出 PDF"))
}

#[cfg(windows)]
fn print_to_pdf(platform: &tauri::webview::PlatformWebview, path: &str) -> AppResult<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Environment6, ICoreWebView2PrintSettings, ICoreWebView2_7,
    };
    use webview2_com::PrintToPdfCompletedHandler;
    use windows::core::{Interface, HSTRING};

    let webview2 = unsafe {
        platform
            .controller()
            .CoreWebView2()
            .map_err(|err| AppError::msg(format!("无法读取页面：{err}")))?
    };
    let webview7: ICoreWebView2_7 = webview2
        .cast()
        .map_err(|err| AppError::msg(format!("当前 WebView2 不支持导出 PDF：{err}")))?;
    let environment6: ICoreWebView2Environment6 = platform
        .environment()
        .cast()
        .map_err(|err| AppError::msg(format!("当前 WebView2 不支持导出 PDF：{err}")))?;
    let settings: ICoreWebView2PrintSettings = unsafe {
        environment6
            .CreatePrintSettings()
            .map_err(|err| AppError::msg(format!("无法创建打印设置：{err}")))?
    };

    configure_print_settings(&settings)?;

    let path = HSTRING::from(path);
    let succeeded = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&succeeded);

    PrintToPdfCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| {
            unsafe { webview7.PrintToPdf(&path, &settings, &handler) }.map_err(Into::into)
        }),
        Box::new(move |result, ok| {
            result?;
            flag.store(ok, Ordering::SeqCst);
            Ok(())
        }),
    )
    .map_err(|err| AppError::msg(format!("导出 PDF 失败：{err}")))?;

    if !succeeded.load(Ordering::SeqCst) {
        return Err(AppError::msg("导出 PDF 失败"));
    }
    Ok(())
}

#[cfg(windows)]
fn configure_print_settings(
    settings: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PrintSettings,
) -> AppResult<()> {
    unsafe {
        settings
            .SetPageWidth(8.27)
            .map_err(|err| AppError::msg(format!("无法设置纸张宽度：{err}")))?;
        settings
            .SetPageHeight(11.69)
            .map_err(|err| AppError::msg(format!("无法设置纸张高度：{err}")))?;
        settings
            .SetMarginTop(0.47)
            .map_err(|err| AppError::msg(format!("无法设置页边距：{err}")))?;
        settings
            .SetMarginBottom(0.47)
            .map_err(|err| AppError::msg(format!("无法设置页边距：{err}")))?;
        settings
            .SetMarginLeft(0.47)
            .map_err(|err| AppError::msg(format!("无法设置页边距：{err}")))?;
        settings
            .SetMarginRight(0.47)
            .map_err(|err| AppError::msg(format!("无法设置页边距：{err}")))?;
        settings
            .SetShouldPrintHeaderAndFooter(false)
            .map_err(|err| AppError::msg(format!("无法关闭页眉页脚：{err}")))?;
        settings
            .SetShouldPrintBackgrounds(true)
            .map_err(|err| AppError::msg(format!("无法设置打印背景：{err}")))?;
    }
    Ok(())
}
