#[cfg(windows)]
pub fn show_message(title: &str, message: &str, error: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
    };
    let title = wide(title);
    let message = wide(message);
    let icon = if error {
        MB_ICONERROR
    } else {
        MB_ICONINFORMATION
    };
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | icon,
        );
    }
}

#[cfg(not(windows))]
pub fn show_message(title: &str, message: &str, error: bool) {
    #[cfg(target_os = "macos")]
    {
        let script = macos_dialog_script(title, message);
        if std::process::Command::new("/usr/bin/osascript")
            .args(["-e", &script])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
        {
            return;
        }
    }
    if error {
        eprintln!("{title}: {message}")
    } else {
        println!("{title}: {message}")
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_dialog_script(title: &str, message: &str) -> String {
    format!(
        "display dialog {} with title {} buttons {{\"好\"}} default button \"好\"",
        apple_script_string(message),
        apple_script_string(title)
    )
}

#[cfg(any(target_os = "macos", test))]
fn apple_script_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

pub fn show_error(title: &str, message: &str) {
    show_message(title, message, true);
}

pub fn show_info(title: &str, message: &str) {
    show_message(title, message, false);
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::macos_dialog_script;

    #[test]
    fn builds_a_valid_macos_dialog_script() {
        assert_eq!(
            macos_dialog_script("标题", "第一行\n第二行 \"内容\""),
            "display dialog \"第一行\\n第二行 \\\"内容\\\"\" with title \"标题\" buttons {\"好\"} default button \"好\""
        );
    }
}
