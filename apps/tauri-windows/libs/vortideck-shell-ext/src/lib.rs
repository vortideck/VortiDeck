mod command;
mod config;
mod factory;
mod sub_commands;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;

use factory::VortiDeckClassFactory;

/// Stored DLL directory path (resolved once from HMODULE).
static DLL_DIR: OnceLock<String> = OnceLock::new();

/// The COM CLSID for the VortiDeck shell extension (parent menu).
/// {D5EF25C7-8556-4838-97D9-53E265083794}
pub const CLSID_VORTIDECK_SHELL_EXT: GUID = GUID::from_u128(
    0xd5ef25c7_8556_4838_97d9_53e265083794u128,
);

/// Pre-reserved CLSIDs for root-level context menu slots (0-7).
/// These map to D5EF25D0..D5EF25D7 in the AppxManifest.
pub const ROOT_SLOT_CLSIDS: [GUID; 8] = [
    GUID::from_u128(0xd5ef25d0_8556_4838_97d9_53e265083794u128),
    GUID::from_u128(0xd5ef25d1_8556_4838_97d9_53e265083794u128),
    GUID::from_u128(0xd5ef25d2_8556_4838_97d9_53e265083794u128),
    GUID::from_u128(0xd5ef25d3_8556_4838_97d9_53e265083794u128),
    GUID::from_u128(0xd5ef25d4_8556_4838_97d9_53e265083794u128),
    GUID::from_u128(0xd5ef25d5_8556_4838_97d9_53e265083794u128),
    GUID::from_u128(0xd5ef25d6_8556_4838_97d9_53e265083794u128),
    GUID::from_u128(0xd5ef25d7_8556_4838_97d9_53e265083794u128),
];

/// Global reference count for the DLL.
static DLL_REF_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn dll_add_ref() {
    DLL_REF_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub fn dll_release() {
    DLL_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
}

/// Returns the directory containing the DLL (and icon.ico).
pub fn dll_dir() -> Option<&'static str> {
    DLL_DIR.get().map(|s| s.as_str())
}

/// DLL entry point.
// SAFETY: This is the unique Windows loader entry point with its required ABI.
#[unsafe(no_mangle)]
extern "system" fn DllMain(hmodule: HMODULE, reason: u32, _reserved: *mut std::ffi::c_void) -> BOOL {
    if reason == 1 {
        // DLL_PROCESS_ATTACH
        let _ = DLL_DIR.get_or_init(|| {
            let mut buf = vec![0u16; 512];
            let len = unsafe {
                windows::Win32::System::LibraryLoader::GetModuleFileNameW(Some(hmodule), &mut buf)
            };
            if len > 0 {
                let path = String::from_utf16_lossy(&buf[..len as usize]);
                if let Some(pos) = path.rfind('\\') {
                    return path[..pos].to_string();
                }
            }
            String::new()
        });
    }
    TRUE
}

/// COM entry point: Creates a class factory for the requested CLSID.
/// Dispatches to the main VortiDeck menu or one of the 8 root-level slots.
// SAFETY: This is the unique COM export with its required name and ABI.
#[unsafe(no_mangle)]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> HRESULT {
    unsafe {
        if ppv.is_null() {
            return E_INVALIDARG;
        }
        *ppv = std::ptr::null_mut();

        let clsid = &*rclsid;

        // Determine which slot (if any) this CLSID corresponds to
        let slot: Option<u8> = if *clsid == CLSID_VORTIDECK_SHELL_EXT {
            None // Main "VortiDeck" parent menu
        } else if let Some(idx) = ROOT_SLOT_CLSIDS.iter().position(|c| c == clsid) {
            Some(idx as u8) // Root-level slot 0-7
        } else {
            return CLASS_E_CLASSNOTAVAILABLE;
        };

        let factory: IClassFactory = VortiDeckClassFactory::new(slot).into();

        let hr = factory.query(&*riid, ppv);
        if hr.is_ok() {
            S_OK
        } else {
            hr
        }
    }
}

/// COM entry point: Determines if the DLL can be unloaded.
// SAFETY: This is the unique COM export with its required name and ABI.
#[unsafe(no_mangle)]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REF_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

/// Self-registration (kept for regsvr32 compatibility, but sparse package
/// handles COM registration via manifest in normal operation).
// SAFETY: This is the unique COM registration export with its required ABI.
#[unsafe(no_mangle)]
extern "system" fn DllRegisterServer() -> HRESULT {
    S_OK
}

// SAFETY: This is the unique COM registration export with its required ABI.
#[unsafe(no_mangle)]
extern "system" fn DllUnregisterServer() -> HRESULT {
    S_OK
}
