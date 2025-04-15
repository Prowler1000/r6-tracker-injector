use num_format::{Locale, ToFormattedString};
use std::{iter::Once, panic::AssertUnwindSafe, sync::Mutex};
use widestring::Utf16String;
use windows::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE};
use windows_fns::{heapwalker::HeapWalker, memwalker::MemoryWalker, region::Region, walk};

use crate::error::IpcError;

use super::Slave;

const LOCALE: &Locale = &Locale::en;

#[allow(dead_code)]
struct Entry {
    start_addr: *const std::ffi::c_void,
    data: Vec<u8>,
}
impl Slave {
    pub fn locate_json(&self) -> Result<Vec<String>, IpcError> {
        let target = Utf16String::from_str("Bulk endpoint response")
            .into_vec()
            .iter()
            .flat_map(|&x| x.to_le_bytes())
            .collect::<Vec<_>>();
        let mut entries: Vec<Vec<u8>> = Vec::new();
        let mut walker = MemoryWalker::new();
        let _ = self.log_debug("Beginning memory walk...");
        let res = unsafe {
            walker.walk_unsafe(target.len().., |data, block| {
                if !entries
                    .iter().chain(std::iter::once(&target))
                    .any(|entry| block.intersects_slice(entry.as_slice()))
                {
                    for i in 0..data.len().saturating_sub(target.len()) {
                        if data[i..].starts_with(&target) {
                            let _ = self.log_debug(format!("Matched at {}", block));
                            if let Some(data) = block.try_copy_range(i..) {
                                entries.push(data);
                            }
                        }
                    }
                }
                true
            })
        };
        match res {
            Ok(_info) => {}
            Err(e) => {
                let _ = self.log_error(format!("Walk returned an error! {}", e));
                return Err(IpcError::MutexPoisoned);
            }
        }
        let _ = self.log_debug(format!(
            "Finished memory walk, scanning {} regions for json entries",
            entries.len()
        ));
        let json = entries
            .into_iter()
            .filter_map(|entry| {
                let utf16_data: &[u16] = unsafe {
                    let (_prefix, aligned, _suffix) = entry.align_to::<u16>();
                    aligned
                };
                let utf16_string = Utf16String::from_slice_lossy(utf16_data).to_string();
                self.search_for_json(utf16_string)
            })
            .collect::<Vec<_>>();
        Ok(json)
    }

    fn search_for_json(&self, data: String) -> Option<String> {
        let mut open_curly_braces = 0;
        let mut open_square_braces = 0;
        let mut open_round_braces = 0;
        let mut backslash_count = 0;
        let mut quotes_open = false;

        if let Some(start) = data.find("{") {
            let mut end = start + 1;
            let iter = data[start..].char_indices();
            for (offset, c) in iter {
                if c == '\\' {
                    backslash_count += 1;
                    continue;
                }

                let is_escaped = backslash_count % 2 == 1;
                backslash_count = 0;

                if c == '"' {
                    //quotes_open = !quotes_open;
                    // If statement doesn't work...
                    if !(quotes_open && is_escaped) {
                        quotes_open = !quotes_open;
                    }
                } else if !quotes_open {
                    match c {
                        '{' => open_curly_braces += 1,
                        '}' => {
                            open_curly_braces -= 1;
                            if open_curly_braces == 0 {
                                end = start + offset + c.len_utf8();
                                break;
                            }
                        }
                        '(' => open_round_braces += 1,
                        ')' => open_round_braces -= 1,
                        '[' => open_square_braces += 1,
                        ']' => open_square_braces -= 1,
                        _ => {}
                    }
                    if [open_curly_braces, open_round_braces, open_square_braces]
                        .iter()
                        .any(|&x| x < 0)
                    {
                        return None;
                    }
                }
            }
            if [open_curly_braces, open_round_braces, open_square_braces]
                .iter()
                .all(|&x| x == 0)
            {
                return Some(data[start..end].to_string());
            }
        }
        None
    }
    
}
