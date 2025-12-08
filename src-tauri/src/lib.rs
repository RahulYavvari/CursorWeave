// src-tauri/src/lib.rs
// CursorWeave - Tauri backend (Windows MVP)
// Replace your existing lib.rs with this file.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
struct ThemeManifest {
    // keep optional fields for forward-compatibility
    // name: Option<String>,
    #[serde(default)]
    cursors: HashMap<String, String>,
}

////////////////////////////////////////////////////////////////////////////////
// Windows implementation: apply theme by writing HKCU\Control Panel\Cursors
////////////////////////////////////////////////////////////////////////////////

#[cfg(windows)]
fn apply_theme_windows(theme_dir: &str, manifest: &ThemeManifest) -> Result<(), String> {
    use std::path::Path;
    use winreg::enums::*;
    use winreg::RegKey;

    // Open HKCU\Control Panel\Cursors key for updates
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags("Control Panel\\Cursors", KEY_SET_VALUE)
        .map_err(|e| {
            format!(
                "Failed to open registry key HKCU\\\\Control Panel\\\\Cursors: {}",
                e
            )
        })?;

    // Iterate over all cursor mappings in manifest.json
    let mut any_written = false;
    for (slot, file_name) in &manifest.cursors {
        let full_path = Path::new(theme_dir).join(file_name);

        if !full_path.exists() {
            // Skip missing files (not fatal) but collect warnings
            eprintln!(
                "CursorWeave: Warning - cursor file missing for {} => {}",
                slot,
                full_path.display()
            );
            continue;
        }

        key.set_value(slot, &full_path.to_string_lossy().to_string())
            .map_err(|e| format!("Failed to set registry value for {}: {}", slot, e))?;

        any_written = true;
    }

    if !any_written {
        // nothing to write — warn but allow caller to decide
        eprintln!("CursorWeave: Warning - no cursor registry values were written (no files found in manifest).");
    }

    // Refresh system cursors via Win32 API
    use winapi::um::winuser::{SystemParametersInfoW, SPIF_SENDCHANGE, SPI_SETCURSORS};

    let result = unsafe {
        SystemParametersInfoW(
            SPI_SETCURSORS, // action: reload system cursors
            0,
            std::ptr::null_mut(),
            SPIF_SENDCHANGE, // broadcast change
        )
    };

    if result == 0 {
        return Err("SystemParametersInfoW(SPI_SETCURSORS) failed".into());
    }

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////
// Tauri commands
////////////////////////////////////////////////////////////////////////////////

/// Return list of theme folder names under %LOCALAPPDATA%/CursorWeave/themes (Windows)
#[tauri::command]
fn list_themes() -> Result<Vec<String>, String> {
    // determine themes root
    let themes_root = if let Ok(local) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local).join("CursorWeave").join("themes")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("cursorweave")
            .join("themes")
    } else {
        return Err("Cannot determine themes directory".into());
    };

    if !themes_root.exists() {
        // return empty list rather than error
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    match fs::read_dir(&themes_root) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            out.push(name.to_string());
                        }
                    }
                }
            }
        }
        Err(e) => return Err(format!("Failed to read themes dir: {}", e)),
    }

    Ok(out)
}

/// Return the themes root path (platform-aware)
#[tauri::command]
fn get_themes_root() -> Result<String, String> {
    let root = if let Ok(local) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local).join("CursorWeave").join("themes")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("cursorweave")
            .join("themes")
    } else {
        return Err("Cannot determine themes directory".into());
    };

    Ok(root.to_string_lossy().to_string())
}

/// Read manifest.json in a BOM-safe way and apply theme (Windows only in MVP)
#[tauri::command]
fn apply_theme(theme_dir: String) -> Result<String, String> {
    // Validate theme_dir exists
    let p = PathBuf::from(&theme_dir);
    if !p.exists() || !p.is_dir() {
        return Err(format!("Theme directory does not exist: {}", theme_dir));
    }

    // parse manifest (BOM-safe)
    let manifest_path = p.join("manifest.json");
    let raw =
        fs::read(&manifest_path).map_err(|e| format!("Unable to read manifest.json: {}", e))?;

    // strip UTF-8 BOM if present
    let content = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(raw[3..].to_vec())
            .map_err(|e| format!("Invalid UTF-8 in manifest.json after BOM removal: {}", e))?
    } else {
        String::from_utf8(raw).map_err(|e| format!("Invalid UTF-8 in manifest.json: {}", e))?
    };

    let manifest: ThemeManifest =
        serde_json::from_str(&content).map_err(|e| format!("Invalid manifest.json: {}", e))?;

    #[cfg(windows)]
    {
        apply_theme_windows(&theme_dir, &manifest)?;
        return Ok("Theme applied successfully".into());
    }

    #[cfg(not(windows))]
    {
        return Err("apply_theme only implemented on Windows in MVP".into());
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tauri app runner
////////////////////////////////////////////////////////////////////////////////

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_themes,
            apply_theme,
            get_themes_root
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
