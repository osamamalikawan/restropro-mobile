fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&[
                "print_raw",
                "test_printer_connection",
                "list_windows_printers",
                "check_windows_printer",
                "print_raw_windows",
                "list_usb_printers",
                "print_raw_usb",
            ])),
    )
    .unwrap();
}
