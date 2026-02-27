// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // call the library entry point. the library is named `timeman_lib` in
    // Cargo.toml ([lib] name = "timeman_lib").
    timeman_lib::run();
}
