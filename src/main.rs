// Desktop entry point. All app code lives in lib.rs so the same code also builds for Android.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    restropro_pos_lib::run();
}
