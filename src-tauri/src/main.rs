#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_os = "android"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(not(target_os = "android"))]
fn main() {
    aqvn_workspace_lib::run();
}

#[cfg(target_os = "android")]
fn main() {}
