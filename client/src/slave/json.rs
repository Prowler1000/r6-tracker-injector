use std::{panic::AssertUnwindSafe, sync::Mutex};
use widestring::Utf16String;
use windows_fns::{
    heapwalker::HeapWalker,
    memwalker::{MemWalkerError, MemoryReadAction, MemoryWalker},
};

use crate::error::IpcError;

use super::Slave;

#[allow(dead_code)]
struct Entry {
    start_addr: *const std::ffi::c_void,
    data: Vec<u8>,
}

fn search_for_json(data: String) -> Option<String> {
    let mut open_curly_braces = 1;
    let mut open_square_braces = 0;
    let mut open_round_braces = 0;
    let mut quotes_open = false;
    let mut escaped = false;
    if let Some(start) = data.find("{") {
        let mut end = start + 1;
        for (i, c) in data.chars().collect::<Vec<_>>()[start..].iter().enumerate() {
            if c == &'\\' {
                escaped = true;
                continue;
            }
            if c == &'"' {
                if !(quotes_open && escaped) {
                    quotes_open = !quotes_open;
                }
            } else if !quotes_open {
                match *c {
                    '{' => open_curly_braces += 1,
                    '}' => {
                        open_curly_braces -= 1;
                        end = start + i + 1;
                    }
                    '(' => open_round_braces += 1,
                    ')' => open_round_braces -= 1,
                    '[' => open_square_braces += 1,
                    ']' => open_square_braces -= 1,
                    _ => {}
                }
                if open_curly_braces == 0 {
                    break;
                }
                if [open_curly_braces, open_round_braces, open_square_braces]
                    .iter()
                    .any(|&x| x < 0)
                {
                    return None;
                }
            }
            escaped = false;
        }
        if [open_curly_braces, open_round_braces, open_square_braces]
            .iter()
            .all(|&x| x == 0)
        {
            return Some(data[start..=end].to_string());
        }
    }
    None
}

impl Slave {
    pub fn locate_json_heap(&self) -> Result<Vec<String>, IpcError> {
        let target = Utf16String::from_str("Bulk endpoint response")
            .into_vec()
            .iter()
            .flat_map(|&x| x.to_le_bytes())
            .collect::<Vec<_>>();
        let entries: Vec<Vec<u8>> = Vec::new();
        let mutex = Mutex::new(entries);
        let mut walker = HeapWalker::new().expect("Failed to create heap walker.");
        let _ = self.log_debug("Beginning memory walk...");
        let res = walker.walk(|data| {
            if data.len() < target.len() {
                return;
            }
            if let Ok(mut lock) = mutex.lock() {
                for i in 0..data.len() - target.len() {
                    if data[i..].starts_with(&target) {
                        let matched_data = data[i..].to_vec();
                        lock.push(matched_data);
                    }
                }
            }
        });
        if res.is_err() {
            return Err(IpcError::MutexPoisioned);
        }
        let entries = match mutex.into_inner() {
            Ok(inner) => inner,
            Err(e) => {
                let _ = self.log_debug("Entries poisioned");
                let mut inner = e.into_inner();
                inner.pop(); // Remove the last element as it's likely corrupt
                inner
            }
        };
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
                search_for_json(utf16_string)
            })
            .collect::<Vec<_>>();
        Ok(json)
    }
    pub fn locate_json(&self) -> Result<Vec<String>, IpcError> {
        let target = Utf16String::from_str("Bulk endpoint response")
            .into_vec()
            .iter()
            .flat_map(|&x| x.to_le_bytes())
            .collect::<Vec<_>>();
        let mut entries: Vec<Vec<u8>> = Vec::new();
        let mut wrapper = AssertUnwindSafe(&mut entries);
        let walker = MemoryWalker::new();
        let _ = self.log_debug("Beginning memory walk...");
        let _ = walker
            .walk_owned(MemoryReadAction::Skip, move |data, _addr| {
                if data.len() < target.len() {
                    return;
                }
                for i in 0..data.len() - target.len() {
                    if data[i..].starts_with(&target) {
                        let matched_data = data[i..].to_vec();
                        wrapper.push(matched_data);
                    }
                }
            })
            .inspect_err(|e| match e {
                MemWalkerError::InvalidMBI(ptr) => {
                    let _ = self.log_error(format!("Received invalid MBI at address {:#?}", ptr));
                }
                MemWalkerError::WindowsError(error) => {
                    let _ = self.log_error(format!(
                        "Windows error occurred. {} : {}",
                        error.code(),
                        error.message()
                    ));
                }
                MemWalkerError::PanicUnwind(any) => {
                    let _ = self.log_error(format!("A panic unwind ocurred! {:#?}", any));
                }
            });
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
                search_for_json(utf16_string)
            })
            .collect::<Vec<_>>();
        Ok(json)
    }
}
