//! iOS-specific bindings

use crate::{MobileError, MobileNodeHandle, NodeConfig};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

/// Opaque handle for Swift
pub struct IOSNodeHandle {
    node: MobileNodeHandle,
}

/// Initialize the mobile node (C-compatible)
///
/// # Safety
/// This function is unsafe because it deals with raw pointers.
/// Callers must ensure:
/// - All pointer arguments are valid, non-null C strings
/// - The returned handle must be freed with `openrustclaw_free`
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_init(
    node_id: *const c_char,
    gateway_url: *const c_char,
    auth_token: *const c_char,
) -> *mut IOSNodeHandle {
    if node_id.is_null() || gateway_url.is_null() || auth_token.is_null() {
        return std::ptr::null_mut();
    }

    let node_id = CStr::from_ptr(node_id).to_string_lossy().into_owned();
    let gateway_url = CStr::from_ptr(gateway_url).to_string_lossy().into_owned();
    let auth_token = CStr::from_ptr(auth_token).to_string_lossy().into_owned();

    let config = NodeConfig::new(node_id, gateway_url, auth_token)
        .with_device_name("iOS Device".to_string())
        .with_capabilities(vec!["mobile".to_string(), "ios".to_string()]);

    match MobileNodeHandle::new(config) {
        Ok(node) => Box::into_raw(Box::new(IOSNodeHandle { node })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Start the mobile node
///
/// # Safety
/// Handle must be a valid pointer returned by `openrustclaw_init`
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_start(handle: *mut IOSNodeHandle) -> c_int {
    if handle.is_null() {
        return -1;
    }

    let handle = &*handle;
    match handle.node.start() {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Stop the mobile node
///
/// # Safety
/// Handle must be a valid pointer returned by `openrustclaw_init`
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_stop(handle: *mut IOSNodeHandle) {
    if !handle.is_null() {
        let handle = &*handle;
        handle.node.stop();
    }
}

/// Send a message through the mobile node
///
/// # Safety
/// Handle must be a valid pointer returned by `openrustclaw_init`
/// Target and content must be valid, non-null C strings
/// Returns a newly allocated string that must be freed with `openrustclaw_free_string`
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_send_message(
    handle: *mut IOSNodeHandle,
    target: *const c_char,
    content: *const c_char,
) -> *mut c_char {
    if handle.is_null() || target.is_null() || content.is_null() {
        return std::ptr::null_mut();
    }

    let handle = &*handle;
    let target = CStr::from_ptr(target).to_string_lossy();
    let content = CStr::from_ptr(content).to_string_lossy();

    match handle.node.send_message(&target, &content) {
        Ok(msg_id) => CString::new(msg_id).unwrap().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get the node status as a JSON string
///
/// # Safety
/// Handle must be a valid pointer returned by `openrustclaw_init`
/// Returns a newly allocated string that must be freed with `openrustclaw_free_string`
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_get_status(handle: *mut IOSNodeHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle = &*handle;
    let status = handle.node.status();

    match serde_json::to_string(&status) {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free the mobile node handle
///
/// # Safety
/// Handle must be a valid pointer returned by `openrustclaw_init`
/// After calling this, the handle is invalid and must not be used
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_free(handle: *mut IOSNodeHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

/// Free a string returned by the library
///
/// # Safety
/// S must be a valid pointer returned by one of the library functions
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// Get the last error message
///
/// # Safety
/// Returns a newly allocated string that must be freed with `openrustclaw_free_string`
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_last_error() -> *mut c_char {
    // This would require thread-local storage for error messages
    // For now, return an empty string
    CString::new("").unwrap().into_raw()
}

/// Initialize with full configuration (JSON)
///
/// # Safety
/// Config_json must be a valid JSON string
#[no_mangle]
pub unsafe extern "C" fn openrustclaw_init_with_config(
    config_json: *const c_char,
) -> *mut IOSNodeHandle {
    if config_json.is_null() {
        return std::ptr::null_mut();
    }

    let config_str = CStr::from_ptr(config_json).to_string_lossy();

    let config: NodeConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    match MobileNodeHandle::new(config) {
        Ok(node) => Box::into_raw(Box::new(IOSNodeHandle { node })),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cstring_roundtrip() {
        let original = "test message";
        let cstring = CString::new(original).unwrap();
        let ptr = cstring.into_raw();

        unsafe {
            let retrieved = CStr::from_ptr(ptr).to_string_lossy();
            assert_eq!(retrieved, original);
            let _ = CString::from_raw(ptr); // Clean up
        }
    }
}
