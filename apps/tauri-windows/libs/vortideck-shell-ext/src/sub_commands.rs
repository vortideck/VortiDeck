use std::panic::AssertUnwindSafe;
use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Shell::*;

use crate::command::{TopLevelEntry, VortiDeckCommand};
use crate::config::ContextMenuItem;
use crate::{dll_add_ref, dll_release};

/// Wraps either a leaf item or a mixed top-level entry for enumeration.
#[derive(Clone)]
enum EnumEntry {
    Leaf(ContextMenuItem),
    Mixed(TopLevelEntry),
}

/// Enumerates dynamic sub-commands from the config.
#[implement(IEnumExplorerCommand)]
pub struct VortiDeckEnumCommands {
    entries: Vec<EnumEntry>,
    index: Mutex<usize>,
}

impl VortiDeckEnumCommands {
    /// Create an enumerator for plain leaf items (used by groups).
    pub fn new(items: Vec<ContextMenuItem>) -> Self {
        dll_add_ref();
        Self {
            entries: items.into_iter().map(EnumEntry::Leaf).collect(),
            index: Mutex::new(0),
        }
    }

    /// Create an enumerator for mixed top-level entries (groups + ungrouped items).
    pub fn new_mixed(entries: Vec<TopLevelEntry>) -> Self {
        dll_add_ref();
        Self {
            entries: entries.into_iter().map(EnumEntry::Mixed).collect(),
            index: Mutex::new(0),
        }
    }

    fn make_command(entry: &EnumEntry) -> IExplorerCommand {
        match entry {
            EnumEntry::Leaf(item) => {
                dll_add_ref();
                let command = VortiDeckCommand::new_sub(item.clone());
                command.into()
            }
            EnumEntry::Mixed(top_level) => match top_level {
                TopLevelEntry::Item(item) => {
                    dll_add_ref();
                    let command = VortiDeckCommand::new_sub(item.clone());
                    command.into()
                }
                TopLevelEntry::Separator => {
                    dll_add_ref();
                    let command = VortiDeckCommand::new_separator();
                    command.into()
                }
            },
        }
    }
}

impl Drop for VortiDeckEnumCommands {
    fn drop(&mut self) {
        dll_release();
    }
}

impl IEnumExplorerCommand_Impl for VortiDeckEnumCommands_Impl {
    fn Next(
        &self,
        celt: u32,
        puilist: *mut Option<IExplorerCommand>,
        pceltfetched: *mut u32,
    ) -> HRESULT {
        // Wrap in catch_unwind because Next returns HRESULT directly,
        // so the windows crate's #[implement] macro won't catch panics.
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mut index = match self.index.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let mut fetched: u32 = 0;

            for i in 0..celt {
                if *index >= self.entries.len() {
                    break;
                }

                let explorer_cmd = VortiDeckEnumCommands::make_command(&self.entries[*index]);

                unsafe {
                    *puilist.add(i as usize) = Some(explorer_cmd);
                }

                *index += 1;
                fetched += 1;
            }

            unsafe {
                if !pceltfetched.is_null() {
                    *pceltfetched = fetched;
                }
            }

            if fetched == celt {
                S_OK
            } else {
                S_FALSE
            }
        }));

        match result {
            Ok(hr) => hr,
            Err(_) => {
                unsafe {
                    if !pceltfetched.is_null() {
                        *pceltfetched = 0;
                    }
                }
                E_FAIL
            }
        }
    }

    fn Skip(&self, celt: u32) -> Result<()> {
        let mut index = match self.index.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *index = (*index + celt as usize).min(self.entries.len());
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        let mut index = match self.index.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *index = 0;
        Ok(())
    }

    fn Clone(&self) -> Result<IEnumExplorerCommand> {
        let index = match self.index.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => *poisoned.into_inner(),
        };
        dll_add_ref();
        Ok(VortiDeckEnumCommands {
            entries: self.entries.clone(),
            index: Mutex::new(index),
        }
        .into())
    }
}
