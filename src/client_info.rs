#[derive(Default, Debug)]
pub struct ClientInfo {
    thread_id: Option<u32>,
    process_id: Option<u32>,
}

impl ClientInfo {
    pub fn new() -> Self {
        Default::default()
    }
}