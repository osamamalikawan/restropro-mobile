use serde::Serialize;
use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Printing::{
    ClosePrinter, EndDocPrinter, EndPagePrinter, EnumPrintersW, GetPrinterW, OpenPrinterW,
    StartDocPrinterW, StartPagePrinter, WritePrinter, DOC_INFO_1W, PRINTER_ENUM_CONNECTIONS,
    PRINTER_ENUM_LOCAL, PRINTER_INFO_2W, PRINTER_STATUS_ERROR, PRINTER_STATUS_NOT_AVAILABLE,
    PRINTER_STATUS_OFFLINE, PRINTER_STATUS_PAPER_JAM, PRINTER_STATUS_PAPER_OUT,
    PRINTER_STATUS_PAUSED,
};

#[derive(Serialize)]
pub struct WindowsPrinterStatus {
    pub installed: bool,
    pub online: bool,
    pub detail: String,
}

#[tauri::command]
pub fn list_windows_printers() -> Result<Vec<String>, String> {
    unsafe {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let mut needed: u32 = 0;
        let mut returned: u32 = 0;

        let _ = EnumPrintersW(
            flags,
            PCWSTR::null(),
            2,
            None,
            &mut needed,
            &mut returned,
        );

        if needed == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = vec![0u8; needed as usize];
        EnumPrintersW(
            flags,
            PCWSTR::null(),
            2,
            Some(&mut buffer),
            &mut needed,
            &mut returned,
        )
        .map_err(|e| format!("Failed to enumerate printers: {e}"))?;

        let printers_slice = std::slice::from_raw_parts(
            buffer.as_ptr() as *const PRINTER_INFO_2W,
            returned as usize,
        );

        let mut names = Vec::new();
        for p in printers_slice {
            if !p.pPrinterName.is_null() {
                if let Ok(name) = p.pPrinterName.to_string() {
                    names.push(name);
                }
            }
        }

        Ok(names)
    }
}

#[tauri::command]
pub fn check_windows_printer(printer_name: String) -> Result<WindowsPrinterStatus, String> {
    unsafe {
        let name_hstring = HSTRING::from(&printer_name);
        let mut handle = HANDLE::default();

        let open_res = OpenPrinterW(PCWSTR(name_hstring.as_ptr()), &mut handle, None);

        if open_res.is_err() || handle.is_invalid() {
            return Ok(WindowsPrinterStatus {
                installed: false,
                online: false,
                detail: format!("Printer '{printer_name}' is not installed or accessible."),
            });
        }

        let mut needed: u32 = 0;
        let _ = GetPrinterW(handle, 2, None, &mut needed);

        if needed == 0 {
            let _ = ClosePrinter(handle);
            return Ok(WindowsPrinterStatus {
                installed: true,
                online: true,
                detail: format!("Printer '{printer_name}' is installed."),
            });
        }

        let mut buffer = vec![0u8; needed as usize];
        let get_res = GetPrinterW(handle, 2, Some(&mut buffer), &mut needed);

        let _ = ClosePrinter(handle);

        if get_res.is_err() {
            return Ok(WindowsPrinterStatus {
                installed: true,
                online: true,
                detail: format!("Printer '{printer_name}' is installed."),
            });
        }

        let info = &*(buffer.as_ptr() as *const PRINTER_INFO_2W);
        let status = info.Status;

        let is_offline = (status & PRINTER_STATUS_OFFLINE) != 0
            || (status & PRINTER_STATUS_NOT_AVAILABLE) != 0
            || (status & PRINTER_STATUS_ERROR) != 0;

        let mut status_msgs = Vec::new();
        if (status & PRINTER_STATUS_PAUSED) != 0 {
            status_msgs.push("Paused");
        }
        if (status & PRINTER_STATUS_ERROR) != 0 {
            status_msgs.push("Error");
        }
        if (status & PRINTER_STATUS_PAPER_JAM) != 0 {
            status_msgs.push("Paper Jam");
        }
        if (status & PRINTER_STATUS_PAPER_OUT) != 0 {
            status_msgs.push("Paper Out");
        }
        if (status & PRINTER_STATUS_OFFLINE) != 0 {
            status_msgs.push("Offline");
        }

        let detail = if status_msgs.is_empty() {
            format!("Printer '{printer_name}' is ready.")
        } else {
            format!("Printer '{printer_name}' status: {}", status_msgs.join(", "))
        };

        Ok(WindowsPrinterStatus {
            installed: true,
            online: !is_offline,
            detail,
        })
    }
}

#[tauri::command]
pub fn print_raw_windows(printer_name: String, data: Vec<u8>) -> Result<(), String> {
    unsafe {
        let name_hstring = HSTRING::from(&printer_name);
        let mut handle = HANDLE::default();

        OpenPrinterW(PCWSTR(name_hstring.as_ptr()), &mut handle, None)
            .map_err(|e| format!("Could not open printer '{printer_name}'. Check printer name. {e}"))?;

        struct PrinterGuard(HANDLE);
        impl Drop for PrinterGuard {
            fn drop(&mut self) {
                unsafe {
                    let _ = ClosePrinter(self.0);
                }
            }
        }
        let _guard = PrinterGuard(handle);

        let doc_name = HSTRING::from("Restro Pro POS Print Job");
        let datatype = HSTRING::from("RAW");

        let doc_info = DOC_INFO_1W {
            pDocName: PWSTR(doc_name.as_ptr() as *mut _),
            pOutputFile: PWSTR::null(),
            pDatatype: PWSTR(datatype.as_ptr() as *mut _),
        };

        let job_id = StartDocPrinterW(handle, 1, &doc_info);
        if job_id == 0 {
            return Err(format!("Could not start print job for '{printer_name}'."));
        }

        struct DocGuard(HANDLE);
        impl Drop for DocGuard {
            fn drop(&mut self) {
                unsafe {
                    let _ = EndDocPrinter(self.0);
                }
            }
        }
        let _doc_guard = DocGuard(handle);

        StartPagePrinter(handle)
            .ok()
            .map_err(|e| format!("Could not start print page for '{printer_name}'. {e}"))?;

        let mut written: u32 = 0;
        let write_res = WritePrinter(
            handle,
            data.as_ptr() as *const _,
            data.len() as u32,
            &mut written,
        );

        let _ = EndPagePrinter(handle);

        write_res
            .ok()
            .map_err(|e| format!("Failed to send data to printer '{printer_name}'. {e}"))?;

        if (written as usize) < data.len() {
            return Err(format!(
                "Only sent {written} of {} bytes to '{printer_name}'.",
                data.len()
            ));
        }

        Ok(())
    }
}
