//! Checking a password with PAM (`pam_start`, `pam_authenticate`, `pam_end`), with a
//! conversation that answers every password prompt with the given password.
//!
//! Written against the stable C API (`security/pam_appl.h`) to avoid a binding crate that
//! needs libclang to build.

use std::{
    ffi::{c_char, c_int, c_void, CString},
    ptr,
};

const PAM_SUCCESS: c_int = 0;
const PAM_BUF_ERR: c_int = 5;
const PAM_CONV_ERR: c_int = 19;
const PAM_PROMPT_ECHO_OFF: c_int = 1;
const PAM_PROMPT_ECHO_ON: c_int = 2;

#[repr(C)]
struct PamMessage {
    msg_style: c_int,
    msg: *const c_char,
}

#[repr(C)]
struct PamResponse {
    resp: *mut c_char,
    resp_retcode: c_int,
}

type ConvFn = unsafe extern "C" fn(
    num_msg: c_int,
    msg: *mut *const PamMessage,
    resp: *mut *mut PamResponse,
    appdata: *mut c_void,
) -> c_int;

#[repr(C)]
struct PamConv {
    conv: ConvFn,
    appdata_ptr: *mut c_void,
}

#[repr(C)]
struct PamHandle {
    _private: [u8; 0],
}

extern "C" {
    fn pam_start(
        service: *const c_char,
        user: *const c_char,
        conv: *const PamConv,
        pamh: *mut *mut PamHandle,
    ) -> c_int;
    fn pam_authenticate(pamh: *mut PamHandle, flags: c_int) -> c_int;
    fn pam_acct_mgmt(pamh: *mut PamHandle, flags: c_int) -> c_int;
    fn pam_end(pamh: *mut PamHandle, status: c_int) -> c_int;
}

extern "C" {
    fn calloc(n: usize, size: usize) -> *mut c_void;
    fn strdup(s: *const c_char) -> *mut c_char;
}

/// Answers password prompts (echo off) and user-name prompts (echo on, answered with the
/// login name); other messages get no answer.
unsafe extern "C" fn converse(
    num_msg: c_int,
    msg: *mut *const PamMessage,
    resp: *mut *mut PamResponse,
    appdata: *mut c_void,
) -> c_int {
    if num_msg <= 0 || msg.is_null() || resp.is_null() || appdata.is_null() {
        return PAM_CONV_ERR;
    }
    // SAFETY: PAM passes `num_msg` messages and frees the responses with free(); we allocate
    // them with calloc/strdup accordingly. `appdata` is our `Credentials`.
    unsafe {
        let creds = &*(appdata as *const Credentials);
        let replies =
            calloc(num_msg as usize, std::mem::size_of::<PamResponse>()) as *mut PamResponse;
        if replies.is_null() {
            return PAM_BUF_ERR;
        }
        for i in 0..num_msg as usize {
            let m = *msg.add(i);
            let answer = match (*m).msg_style {
                PAM_PROMPT_ECHO_OFF => Some(&creds.password),
                PAM_PROMPT_ECHO_ON => Some(&creds.user),
                _ => None,
            };
            if let Some(a) = answer {
                (*replies.add(i)).resp = strdup(a.as_ptr());
            }
        }
        *resp = replies;
    }
    PAM_SUCCESS
}

struct Credentials {
    user: CString,
    password: CString,
}

/// Whether `password` is right for `user` according to PAM service `service`.
pub fn authenticate(service: &str, user: &str, password: &str) -> bool {
    let (Ok(service), Ok(user_c), Ok(pw)) = (
        CString::new(service),
        CString::new(user),
        CString::new(password),
    ) else {
        return false;
    };
    let creds = Credentials {
        user: user_c.clone(),
        password: pw,
    };
    let conv = PamConv {
        conv: converse,
        appdata_ptr: &creds as *const Credentials as *mut c_void,
    };
    let mut handle: *mut PamHandle = ptr::null_mut();
    // SAFETY: the strings and the conversation outlive the PAM transaction, which ends here.
    unsafe {
        let r = pam_start(service.as_ptr(), user_c.as_ptr(), &conv, &mut handle);
        if r != PAM_SUCCESS || handle.is_null() {
            eprintln!("patina-lock: pam_start failed ({r})");
            return false;
        }
        let mut status = pam_authenticate(handle, 0);
        if status == PAM_SUCCESS {
            status = pam_acct_mgmt(handle, 0);
        }
        pam_end(handle, status);
        status == PAM_SUCCESS
    }
}
