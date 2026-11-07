use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;

use crate::command::VortiDeckCommand;
use crate::{dll_add_ref, dll_release};

/// Class factory that creates VortiDeckCommand instances.
/// `slot` is None for the main "VortiDeck" parent menu,
/// or Some(0..7) for a root-level slot.
#[implement(IClassFactory)]
pub struct VortiDeckClassFactory {
    slot: Option<u8>,
}

impl VortiDeckClassFactory {
    pub fn new(slot: Option<u8>) -> Self {
        Self { slot }
    }
}

impl IClassFactory_Impl for VortiDeckClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> Result<()> {
        unsafe {
            if ppvobject.is_null() {
                return Err(Error::new(E_POINTER, "null ppvobject"));
            }
            *ppvobject = std::ptr::null_mut();

            // No aggregation support
            if punkouter.is_some() {
                return Err(Error::new(CLASS_E_NOAGGREGATION, "aggregation not supported"));
            }

            dll_add_ref();

            let command = match self.slot {
                None => VortiDeckCommand::new_root(),
                Some(idx) => VortiDeckCommand::new_root_slot(idx),
            };
            let unknown: IUnknown = command.into();
            let hr = unknown.query(&*riid, ppvobject);

            if hr.is_err() {
                dll_release();
                return Err(Error::from(hr));
            }

            Ok(())
        }
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            dll_add_ref();
        } else {
            dll_release();
        }
        Ok(())
    }
}
