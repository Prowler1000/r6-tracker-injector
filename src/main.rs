use client::{
    control::{
        command::{Command, Instruction},
        message::{DataMessage, Message},
    },
    master::Master,
};
use client_info::ClientInfo;
use device_query::{DeviceEvents, DeviceEventsHandler, Keycode};
use dll_syringe::{process::OwnedProcess, Syringe};
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use logger::{
    loggers::{console::ConsoleLogger, filter::LogFilter, null::NullLogger},
    severity::LogSeverity,
    LogManager, LogMessage,
};
use siege::MatchData;
use std::{collections::HashMap, fs::File, io::{BufReader, BufWriter, Stdin}};
use std::fs;
use std::path::Path;
use std::{env::current_exe, sync::Arc, time::Duration};
use std::process::{Command as ProcessCommand, Stdio};
use std::io::Write;
use std::io::{self, Read};

mod client_info;

static DLL_PATH: &str = "deps/payload.dll";

fn setup(path: impl AsRef<Path>) -> Option<(IpcSender<Instruction>, IpcReceiver<Message>)> {
    if let Some(target_process) = OwnedProcess::find_first_by_name("Overwolf.exe") {
        let syringe = Syringe::for_process(target_process);
        let payload = syringe.inject(&path);
        match payload {
            Ok(module) => {
                println!("Success!");
                let remote_fn = unsafe {
                    syringe.get_payload_procedure::<fn(String) -> u32>(module, "set_channels")
                }
                .unwrap()
                .unwrap();
                let (server, name) = ipc_channel::ipc::IpcOneShotServer::<
                    IpcSender<(IpcSender<Message>, IpcSender<IpcSender<Instruction>>)>,
                >::new()
                .unwrap();
                let (mut sender, mut receiver) = (None, None);
                match remote_fn.call(&name) {
                    Ok(_) => {
                        let (_, thing) = server.accept().unwrap();
                        let (message_sender, message_receiver) =
                            ipc_channel::ipc::channel().unwrap();
                        let (meta_sender, meta_receiver) = ipc_channel::ipc::channel().unwrap();
                        if thing.send((message_sender, meta_sender)).is_ok() {
                            if let Ok(inst_sender) = meta_receiver.recv() {
                                sender.replace(inst_sender);
                                receiver.replace(message_receiver);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to set channels! {}", e);
                        syringe.eject(module).unwrap();
                    }
                }
                if let (Some(sender), Some(receiver)) = (sender, receiver) {
                    return Some((sender, receiver));
                }
            }
            Err(e) => {
                println!("Failed to inject module! {}", e);
            }
        }
    } else {
        println!("Failed to find process!");
    }
    None
}

lazy_static::lazy_static! {
    static ref KEYBINDS: HashMap<Keycode, Command> = {
        let mut map = HashMap::new();
        map.insert(Keycode::Numpad5, Command::FindJSON);
        map.insert(Keycode::Numpad4, Command::GetProcessId);
        map.insert(Keycode::Numpad6, Command::GetThreadId);
        map.insert(Keycode::Numpad8, Command::Quit);
        map
    };
}

fn generate_keybinds_callback(
    master: Arc<Master>,
) -> impl Fn(&device_query::Keycode) + Sync + Send + 'static {
    move |key| {
        if let Some(cmd) = KEYBINDS.get(key) {
            let _ = master.send(cmd.clone());
        }
    }
}

fn main() {
    let mut path = current_exe().unwrap();
    path.pop();
    path.push(DLL_PATH);
    if let Some((sender, receiver)) = setup(&path) {
        let console_logger = ConsoleLogger::new();
        //let console_logger = NullLogger::new();
        let log_manager = LogManager::new(LogFilter::new(LogSeverity::Debug, console_logger));

        let master = Arc::new(Master::new(sender, receiver, log_manager.get_log_worker()));

        let device_events = DeviceEventsHandler::new(Duration::from_millis(10)).unwrap();
        let _guard = device_events.on_key_down(generate_keybinds_callback(master.clone()));

        master_loop(master, log_manager);
    }
}

fn load_latest_json(client_info: &mut ClientInfo) {
    let output_dir = current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("output");
    if let Ok(entries) = fs::read_dir(output_dir) {
        let mut latest_file = None;
        let mut latest_time = std::time::SystemTime::UNIX_EPOCH;

        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified > latest_time {
                        latest_time = modified;
                        latest_file = Some(entry.path());
                    }
                }
            }
        }

        if let Some(latest_path) = latest_file {
            if let Ok(json_data) = fs::read_to_string(&latest_path) {
                if let Some(match_data) = MatchData::new(&json_data) {
                    client_info.add_game_info(match_data);
                    println!("Loaded latest JSON from {:?}", latest_path);
                } else {
                    println!("Failed to parse MatchData from {:?}", latest_path);
                }
            } else {
                println!("Failed to read file {:?}", latest_path);
            }
        } else {
            println!("No JSON files found in the output directory.");
        }
    } else {
        println!("Failed to read the output directory.");
    }
}

fn master_loop(master: Arc<Master>, log_manager: LogManager) {
    let mut client_info = ClientInfo::new();
    //load_latest_json(&mut client_info);
    //client_info.redraw_console();
    while let Ok(res) = master.recv() {
        match res {
            DataMessage::ProcessId(id) => {
                client_info.set_process_id(id);
                let _ = log_manager.log(LogMessage::new(
                    LogSeverity::Info,
                    format!("Returned process ID: {}", id),
                ));
            }
            DataMessage::ThreadId(id) => {
                client_info.set_thread_id(id);
                let _ = log_manager.log(LogMessage::new(
                    LogSeverity::Info,
                    format!("Returned thread ID: {}", id),
                ));
            }
            DataMessage::Json(items) => {
                println!("Located {} json elements!", items.len());
                let output_dir = current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("output");
                if !output_dir.exists() {
                    fs::create_dir(&output_dir).unwrap();
                }
                for json in &items {
                    if let Some(data) = MatchData::new(json) {
                        client_info.add_game_info(data);
                    }
                }

                let mut i = 0;
                while output_dir.join(format!("output_{}.json", i)).exists() {
                    i += 1;
                }
                for json_element in items.iter() {
                    let file_path = output_dir.join(format!("output_{}.json", i));
                    fs::write(file_path, json_element).unwrap();
                    i += 1;
                }
            }
        }
        //client_info.redraw_console();
    }
}
