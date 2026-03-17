//! Android-specific bindings

use crate::{MobileError, MobileNodeHandle, NodeConfig, NodeStatus, SyncConfig};
use jnix::jni::JNIEnv;
use jnix::jni::objects::JString;
use jnix::jni::signature::JavaType;
use jnix::jni::strings::JNIString;
use jnix::{FromJava, IntoJava, JnixEnv};
use std::sync::Arc;
use std::sync::Mutex;

/// Wrapper for the mobile node to be used from JNI
pub struct AndroidNodeHandle {
    inner: Arc<Mutex<MobileNodeHandle>>,
}

impl AndroidNodeHandle {
    fn new(node: MobileNodeHandle) -> Self {
        Self {
            inner: Arc::new(Mutex::new(node)),
        }
    }
}

/// JNI: Initialize the mobile node
#[no_mangle]
pub extern "C" fn Java_com_openrustclaw_mobile_RustNode_nativeInit(
    env: &mut JNIEnv,
    _class: jnix::jni::objects::JClass,
    node_id: JString,
    gateway_url: JString,
    auth_token: JString,
) -> jlong {
    let node_id: String = env.get_string(&node_id).unwrap().into();
    let gateway_url: String = env.get_string(&gateway_url).unwrap().into();
    let auth_token: String = env.get_string(&auth_token).unwrap().into();

    let config = NodeConfig::new(node_id, gateway_url, auth_token)
        .with_device_name("Android Device".to_string())
        .with_capabilities(vec!["mobile".to_string(), "android".to_string()]);

    match MobileNodeHandle::new(config) {
        Ok(node) => {
            let handle = Box::new(AndroidNodeHandle::new(node));
            Box::into_raw(handle) as jlong
        }
        Err(_) => 0,
    }
}

/// JNI: Start the mobile node
#[no_mangle]
pub extern "C" fn Java_com_openrustclaw_mobile_RustNode_nativeStart(
    _env: &mut JNIEnv,
    _class: jnix::jni::objects::JClass,
    handle: jlong,
) -> jint {
    if handle == 0 {
        return -1;
    }

    let handle = unsafe { &*(handle as *const AndroidNodeHandle) };
    let node = handle.inner.lock().unwrap();

    match node.start() {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// JNI: Stop the mobile node
#[no_mangle]
pub extern "C" fn Java_com_openrustclaw_mobile_RustNode_nativeStop(
    _env: &mut JNIEnv,
    _class: jnix::jni::objects::JClass,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }

    let handle = unsafe { &*(handle as *const AndroidNodeHandle) };
    let node = handle.inner.lock().unwrap();
    node.stop();
}

/// JNI: Send a message
#[no_mangle]
pub extern "C" fn Java_com_openrustclaw_mobile_RustNode_nativeSendMessage(
    env: &mut JNIEnv,
    _class: jnix::jni::objects::JClass,
    handle: jlong,
    target: JString,
    content: JString,
) -> JString {
    if handle == 0 {
        return JString::default();
    }

    let target: String = env.get_string(&target).unwrap().into();
    let content: String = env.get_string(&content).unwrap().into();

    let handle = unsafe { &*(handle as *const AndroidNodeHandle) };
    let node = handle.inner.lock().unwrap();

    match node.send_message(&target, &content) {
        Ok(msg_id) => env.new_string(msg_id).unwrap(),
        Err(_) => JString::default(),
    }
}

/// JNI: Get node status as JSON
#[no_mangle]
pub extern "C" fn Java_com_openrustclaw_mobile_RustNode_nativeGetStatus(
    env: &mut JNIEnv,
    _class: jnix::jni::objects::JClass,
    handle: jlong,
) -> JString {
    if handle == 0 {
        return env.new_string("{}").unwrap();
    }

    let handle = unsafe { &*(handle as *const AndroidNodeHandle) };
    let node = handle.inner.lock().unwrap();

    let status = node.status();
    let json = serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string());

    env.new_string(json).unwrap()
}

/// JNI: Free the mobile node handle
#[no_mangle]
pub extern "C" fn Java_com_openrustclaw_mobile_RustNode_nativeFree(
    _env: &mut JNIEnv,
    _class: jnix::jni::objects::JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut AndroidNodeHandle);
        }
    }
}

/// JNI: Initialize with JSON configuration
#[no_mangle]
pub extern "C" fn Java_com_openrustclaw_mobile_RustNode_nativeInitWithConfig(
    env: &mut JNIEnv,
    _class: jnix::jni::objects::JClass,
    config_json: JString,
) -> jlong {
    let config_str: String = env.get_string(&config_json).unwrap().into();

    let config: NodeConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    match MobileNodeHandle::new(config) {
        Ok(node) => {
            let handle = Box::new(AndroidNodeHandle::new(node));
            Box::into_raw(handle) as jlong
        }
        Err(_) => 0,
    }
}

/// Kotlin wrapper class structure (for reference):
///
/// ```kotlin
/// package com.openrustclaw.mobile
///
/// class RustNode private constructor(private val handle: Long) {
///     companion object {
///         init {
///             System.loadLibrary("openrustclaw_mobile")
///         }
///
///         @JvmStatic
///         external fun nativeInit(nodeId: String, gatewayUrl: String, authToken: String): Long
///
///         @JvmStatic
///         external fun nativeStart(handle: Long): Int
///
///         @JvmStatic
///         external fun nativeStop(handle: Long)
///
///         @JvmStatic
///         external fun nativeSendMessage(handle: Long, target: String, content: String): String
///
///         @JvmStatic
///         external fun nativeGetStatus(handle: Long): String
///
///         @JvmStatic
///         external fun nativeFree(handle: Long)
///
///         fun create(nodeId: String, gatewayUrl: String, authToken: String): RustNode? {
///             val handle = nativeInit(nodeId, gatewayUrl, authToken)
///             return if (handle != 0L) RustNode(handle) else null
///         }
///     }
///
///     fun start(): Boolean = nativeStart(handle) == 0
///
///     fun stop() = nativeStop(handle)
///
///     fun sendMessage(target: String, content: String): String? {
///         val result = nativeSendMessage(handle, target, content)
///         return if (result.isNotEmpty()) result else null
///     }
///
///     fun getStatus(): String = nativeGetStatus(handle)
///
///     fun release() = nativeFree(handle)
/// }
/// ```

/// Low-level JNI bindings for advanced use cases
pub mod low_level {
    use super::*;

    /// Raw pointer type alias
    pub type NodeHandlePtr = jlong;

    /// Check if a handle is valid
    pub fn is_valid_handle(handle: NodeHandlePtr) -> bool {
        handle != 0
    }

    /// Convert a Rust result to a JNI jint result code
    pub fn result_to_jint<T>(result: Result<T, MobileError>) -> jint {
        match result {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

// Type aliases for JNI types
type jlong = i64;
type jint = i32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_validity() {
        assert!(!low_level::is_valid_handle(0));
        assert!(low_level::is_valid_handle(1));
        assert!(low_level::is_valid_handle(0x12345678));
    }
}
