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
    if error {
        eprintln!("{title}: {message}")
    } else {
        println!("{title}: {message}")
    }
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
