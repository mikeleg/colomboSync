//! Is the app running as administrator? Mapped network drives belong to the user's normal
//! token and are not visible to an elevated one.

const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_ELEVATION: i32 = 20;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> isize;
    fn CloseHandle(handle: isize) -> i32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(process: isize, access: u32, token: *mut isize) -> i32;
    fn GetTokenInformation(
        token: isize,
        class: i32,
        info: *mut u32,
        length: u32,
        returned: *mut u32,
    ) -> i32;
}

/// `false` when the answer cannot be had: the banner is a hint, not a gate.
pub fn is_elevated() -> bool {
    // SAFETY: plain Win32 calls on the current process; `token` is closed on every path and
    // `elevated` is the 4-byte TOKEN_ELEVATION struct that `TokenElevation` fills.
    unsafe {
        let mut token = 0isize;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevated = 0u32;
        let mut returned = 0u32;
        let read = GetTokenInformation(token, TOKEN_ELEVATION, &mut elevated, 4, &mut returned);
        CloseHandle(token);
        read != 0 && elevated != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_can_be_asked_and_the_answer_is_stable() {
        // The value depends on how the tests were started, so only the FFI calls are checked.
        assert_eq!(is_elevated(), is_elevated());
    }
}
