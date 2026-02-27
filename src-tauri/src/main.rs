// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // call the library entry point. the library is named `usage_meter_lib` in
    // Cargo.toml ([lib] name = "usage_meter_lib").
    usage_meter_lib::run();
}
