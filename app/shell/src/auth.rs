//! Asks Windows to verify the user before saved passwords are shown or exported.
//!
//! First choice is Windows Hello's consent dialog (PIN, fingerprint, face or password, the same
//! one Chrome uses). When Hello isn't set up it falls back to the classic "enter your Windows
//! password" prompt, checked with `LogonUserW`. Not for autofill: that only needs the user's
//! click, as in Chrome and Brave.
//!
//! The call blocks until the user answers, so it must run on its own thread, never on the UI
//! thread.

#[derive(Debug, PartialEq, Eq)]
pub enum Auth {
    Verified,
    Cancelled,
    Unavailable(String),
}

/// `hwnd` is our main window (the dialog is shown over it). `reason` is shown in the dialog.
pub fn verify(hwnd: isize, reason: &str) -> Auth {
    // Debug builds only (the automated live checks can't click a Windows security dialog).
    #[cfg(debug_assertions)]
    match std::env::var("CONSTELLATIONS_TEST_AUTH").as_deref() {
        Ok("allow") => return Auth::Verified,
        Ok("deny") => return Auth::Cancelled,
        _ => {}
    }
    #[cfg(windows)]
    {
        windows_impl::verify(hwnd, reason)
    }
    #[cfg(not(windows))]
    {
        // Needs testing on Linux: no OS verification is wired up there yet.
        let _ = (hwnd, reason);
        Auth::Verified
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::Auth;
    use windows::Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
    };
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE, HWND};
    use windows::Win32::Security::Credentials::{
        CRED_PACK_FLAGS, CREDUI_INFOW, CREDUIWIN_ENUMERATE_CURRENT_USER,
        CredUIPromptForWindowsCredentialsW, CredUnPackAuthenticationBufferW,
    };
    use windows::Win32::Security::{
        LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT, LogonUserW,
    };
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::System::WinRT::{
        IUserConsentVerifierInterop, RO_INIT_MULTITHREADED, RoInitialize,
    };
    use windows::core::{HSTRING, PCWSTR, PWSTR, factory};
    use windows_future::IAsyncOperation;

    pub fn verify(hwnd: isize, reason: &str) -> Auth {
        // WinRT needs the thread initialised; this runs on its own thread, not the UI thread.
        unsafe {
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
        }
        match hello(hwnd, reason) {
            Some(a) => a,
            None => password_prompt(hwnd, reason),
        }
    }

    /// `None` means Hello can't be used here: fall back to the password prompt.
    fn hello(hwnd: isize, reason: &str) -> Option<Auth> {
        let avail = UserConsentVerifier::CheckAvailabilityAsync()
            .ok()?
            .join()
            .ok()?;
        if avail != UserConsentVerifierAvailability::Available {
            return None;
        }
        let interop = factory::<UserConsentVerifier, IUserConsentVerifierInterop>().ok()?;
        let op: IAsyncOperation<UserConsentVerificationResult> = unsafe {
            interop.RequestVerificationForWindowAsync(HWND(hwnd as *mut _), &HSTRING::from(reason))
        }
        .ok()?;
        match op.join().ok()? {
            UserConsentVerificationResult::Verified => Some(Auth::Verified),
            UserConsentVerificationResult::Canceled
            | UserConsentVerificationResult::RetriesExhausted => Some(Auth::Cancelled),
            _ => None,
        }
    }

    fn password_prompt(hwnd: isize, reason: &str) -> Auth {
        let message = HSTRING::from(reason);
        let caption = HSTRING::from("Constellations");
        let info = CREDUI_INFOW {
            cbSize: std::mem::size_of::<CREDUI_INFOW>() as u32,
            hwndParent: HWND(hwnd as *mut _),
            pszMessageText: PCWSTR(message.as_ptr()),
            pszCaptionText: PCWSTR(caption.as_ptr()),
            ..Default::default()
        };
        let mut package = 0u32;
        let mut out: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut out_size = 0u32;
        let rc = unsafe {
            CredUIPromptForWindowsCredentialsW(
                Some(&info),
                0,
                &mut package,
                None,
                0,
                &mut out,
                &mut out_size,
                None,
                CREDUIWIN_ENUMERATE_CURRENT_USER,
            )
        };
        if rc == ERROR_CANCELLED.0 {
            return Auth::Cancelled;
        }
        if rc != 0 || out.is_null() {
            return Auth::Unavailable(format!("Windows verification failed ({rc})."));
        }

        let mut user = vec![0u16; 513];
        let mut domain = vec![0u16; 257];
        let mut pass = vec![0u16; 513];
        let (mut ul, mut dl, mut pl) = (user.len() as u32, domain.len() as u32, pass.len() as u32);
        let unpacked = unsafe {
            CredUnPackAuthenticationBufferW(
                CRED_PACK_FLAGS(0),
                out,
                out_size,
                Some(PWSTR(user.as_mut_ptr())),
                &mut ul,
                Some(PWSTR(domain.as_mut_ptr())),
                Some(&mut dl),
                Some(PWSTR(pass.as_mut_ptr())),
                &mut pl,
            )
        };
        unsafe { CoTaskMemFree(Some(out)) };
        let result = if unpacked.is_err() {
            Auth::Unavailable(
                "That sign-in method can't be checked here. Use your Windows password.".into(),
            )
        } else {
            let mut token = HANDLE::default();
            let logon = unsafe {
                LogonUserW(
                    PCWSTR(user.as_ptr()),
                    PCWSTR(domain.as_ptr()),
                    PCWSTR(pass.as_ptr()),
                    LOGON32_LOGON_INTERACTIVE,
                    LOGON32_PROVIDER_DEFAULT,
                    &mut token,
                )
            };
            match logon {
                Ok(()) => {
                    unsafe {
                        let _ = CloseHandle(token);
                    }
                    Auth::Verified
                }
                Err(_) => Auth::Cancelled,
            }
        };
        // Don't leave the typed password in memory.
        pass.iter_mut().for_each(|c| *c = 0);
        result
    }
}
