use std::collections::HashMap;
// use std::ffi::CStr;
// use std::os::raw::c_char;
use std::{io, fs};

pub fn load_env_from_proc() -> io::Result<HashMap<String, String>> {
    let mut variables = HashMap::new();
    
    // Read from /proc/self/environ (Linux/Unix only)
    let environ_data = fs::read("/proc/self/environ")?;
    
    // Split by null bytes and parse key=value pairs
    for env_pair in environ_data.split(|&b| b == 0) {
        if let Ok(env_str) = std::str::from_utf8(env_pair) {
            if let Some((key, value)) = env_str.split_once('=') {
                variables.insert(key.to_string(), value.to_string());
            }
        }
    }
    
    Ok(variables)
}

// unsafe extern "C" {
//     #[cfg(not(target_os = "windows"))]
//     static environ: *const *const c_char;
// }

// #[cfg(target_os = "windows")]
// unsafe extern "C" {
//     fn GetEnvironmentStringsA() -> *mut c_char;
//     fn FreeEnvironmentStringsA(env_block: *mut c_char) -> i32;
// }

// pub fn load_env() -> HashMap<String, String> {
//     let mut variables = HashMap::new();
    
//     #[cfg(not(target_os = "windows"))]
//     {
//         unsafe {
//             let mut env_ptr = environ;
//             while !(*env_ptr).is_null() {
//                 let c_str = CStr::from_ptr(*env_ptr);
//                 if let Ok(env_str) = c_str.to_str() {
//                     if let Some((key, value)) = env_str.split_once('=') {
//                         variables.insert(key.to_string(), value.to_string());
//                     }
//                 }
//                 env_ptr = env_ptr.add(1);
//             }
//         }
//     }
    
//     #[cfg(target_os = "windows")]
//     {
//         unsafe {
//             let env_block = GetEnvironmentStringsA();
//             if !env_block.is_null() {
//                 let mut current = env_block;
//                 loop {
//                     let c_str = CStr::from_ptr(current);
//                     let bytes = c_str.to_bytes();
//                     if bytes.is_empty() {
//                         break;
//                     }
                    
//                     if let Ok(env_str) = std::str::from_utf8(bytes) {
//                         if let Some((key, value)) = env_str.split_once('=') {
//                             variables.insert(key.to_string(), value.to_string());
//                         }
//                     }
                    
//                     current = current.add(bytes.len() + 1);
//                 }
//                 FreeEnvironmentStringsA(env_block);
//             }
//         }
//     }
    
//     variables
// }