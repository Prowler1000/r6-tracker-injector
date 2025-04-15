use std::ops::DerefMut;

use client::{
    messages::{Instruction, Message},
    slave::Slave,
};
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use lazy_static::lazy_static;
use std::io::Write;
use thread_safe_utils::signal::{Signal, SignallableData};
use windows::Win32::{
    Foundation::{FreeLibrary, HANDLE, HMODULE},
    System::{
        LibraryLoader::{DisableThreadLibraryCalls, FreeLibraryAndExitThread},
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        Threading::{CreateThread, THREAD_CREATION_FLAGS},
    },
};

#[derive(Default)]
pub struct RuntimeStorage {
    pub current_module: HMODULE,
    pub code_thread: Option<HANDLE>,
    pub sender: Option<IpcSender<Message>>,
    pub receiver: Option<IpcReceiver<Instruction>>,
}

unsafe impl Send for RuntimeStorage {}
unsafe impl Sync for RuntimeStorage {}

lazy_static! {
    static ref PARAMS: SignallableData<RuntimeStorage> = Default::default();
}

fn log_to_temp_console(msg: impl Into<String>) {
    let mut idk = std::process::Command::new("cmd");
    let formatted_msg = msg.into()
        .lines()
        .map(|line| format!("echo {}", line))
        .collect::<Vec<_>>()
        .join(" & ");
    let _ = idk.arg("/K").arg(formatted_msg).spawn();
}

fn log_to_file(msg: impl AsRef<str>) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(r"C:\Users\braed\Documents\GitHub\r6-tracker-injector\dll_crash.txt")
    {
        let _ = writeln!(file, "{}", msg.as_ref());
    }
}

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Panic: {}", info);
        log_to_file(msg.clone());
        log_to_temp_console(msg);
    }));
}

#[no_mangle]
unsafe extern "system" fn code_runner(_ptr: *mut std::ffi::c_void) -> u32 {
    set_panic_hook();
    let mut lock = match PARAMS.lock_wait_while(|lock, signal| {
        (lock.receiver.is_none() || lock.sender.is_none()) && !signal
    }) {
        Ok(val) => {
            if val.is_signalled() {
                FreeLibraryAndExitThread(val.current_module, 0);
            } else {
                val
            }
        }
        Err(_) => unsafe {
            let curr_module = PARAMS.ignore_poision().current_module;
            FreeLibraryAndExitThread(curr_module, 1);
        },
    };
    let storage = lock.deref_mut();
    let tx = storage.sender.take().unwrap();
    let rx = storage.receiver.take().unwrap();
    let client = Slave::new(
        tx,
        rx,
        r"C:\Users\braed\Documents\GitHub\r6-tracker-injector\dll.log",
    );
    let _ = client.run_client();
    drop(client);
    FreeLibraryAndExitThread(lock.current_module, 0);
}

fn receive_ipc_channels(
    meta_receiver: IpcReceiver<(IpcSender<Message>, IpcSender<IpcSender<Instruction>>)>,
) {
    if let Ok(mut lock) = PARAMS.lock() {
        if let Ok(bundle) = meta_receiver.recv() {
            lock.sender.replace(bundle.0);
            let (inst_sender, inst_recv) = ipc_channel::ipc::channel().unwrap();
            if bundle.1.send(inst_sender).is_ok() {
                lock.receiver.replace(inst_recv);
            } else {
                PARAMS.set_signal(true);
            }
        } else {
            PARAMS.set_signal(true);
        }
    }
}

fn set_channels_with_syntax_highlighting(channel: String) -> u32 {
    let oneoff =
        IpcSender::<IpcSender<(IpcSender<Message>, IpcSender<IpcSender<Instruction>>)>>::connect(
            channel,
        )
        .unwrap();
    let (meta_sender, meta_receiver) = ipc_channel::ipc::channel().unwrap();
    oneoff.send(meta_sender).unwrap();

    std::thread::spawn(move || receive_ipc_channels(meta_receiver));
    1
}
dll_syringe::payload_procedure! {
    fn set_channels(channel: String) -> u32 {
        set_channels_with_syntax_highlighting(channel)
    }
}

/**
 * # Safety
 * I'm lazy tbh
 */
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    hinst_dll: HMODULE,
    fdw_reason: u32,
    _lpv_reserved: *mut (),
) -> bool {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            let _ = DisableThreadLibraryCalls(hinst_dll);
            if let Ok(Some(mut lock)) = PARAMS.try_lock() {
                if lock.code_thread.is_none() {
                    lock.current_module = hinst_dll;
                    if let Ok(handle) = CreateThread(
                        None,
                        0,
                        Some(code_runner),
                        None,
                        THREAD_CREATION_FLAGS(0),
                        None,
                    ) {
                        lock.code_thread = Some(handle);
                    } else {
                        PARAMS.set_signal(true);
                        let _ = FreeLibrary(hinst_dll);
                        return false;
                    }
                }
            }
        }
        DLL_PROCESS_DETACH => {
            // Optional cleanup here
        }
        _ => {}
    }
    true
}
