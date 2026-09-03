#[cfg(windows)]
pub fn is_elevated() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(windows)]
pub fn restart_elevated() -> Result<(), String> {
    use windows_sys::Win32::{
        System::LibraryLoader::GetModuleFileNameW,
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    let mut executable = vec![0_u16; 32_768];
    let length = unsafe {
        GetModuleFileNameW(
            std::ptr::null_mut(),
            executable.as_mut_ptr(),
            executable.len() as u32,
        )
    } as usize;
    if length == 0 || length == executable.len() {
        return Err("Não foi possível localizar o executável do NodeSweep.".into());
    }
    executable.truncate(length);
    executable.push(0);
    let verb: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            executable.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        return Err("A elevação foi cancelada ou não pôde ser iniciada.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn restart_elevated() -> Result<(), String> {
    Err("Elevação administrativa está disponível apenas no Windows.".into())
}
