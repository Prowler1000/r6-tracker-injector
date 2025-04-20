use crate::control::command::{CommandID, Instruction, PendingInstruction};

pub struct ClientState {
    next_available_id: CommandID,
    pub pending_instructions: Vec<PendingInstruction>,
    pub inprogres_instructions: Vec<PendingInstruction>,
    pub ready: bool,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            next_available_id: Default::default(),
            pending_instructions: Default::default(),
            inprogres_instructions: Default::default(),
            ready: Default::default(),
        }
    }
}

impl ClientState {
    pub fn next_id(&mut self) -> CommandID {
        let next_id = self.next_available_id;
        self.next_available_id += 1;
        next_id
    }

    pub fn add_pending_inst(&mut self, inst: PendingInstruction) {
        self.pending_instructions.push(inst);
    }

    pub fn acknowledge_instruction(&mut self, id: CommandID) -> (Option<Instruction>, bool) {
        if let Some(inst) = self
            .pending_instructions
            .iter()
            .position(|inst| inst.id == id)
            .map(|pos| self.pending_instructions.remove(pos))
        {
            let tmp = (Some((&inst).into()), inst.mark_started());
            self.inprogres_instructions.push(inst);
            tmp
        } else {
            (None, false)
        }
    }

    pub fn complete_instructions(
        &mut self,
        id: CommandID,
        data: Box<dyn std::any::Any + Send>,
    ) -> (Option<Instruction>, bool) {
        if let Some(inst) = self
            .inprogres_instructions
            .iter()
            .position(|inst| inst.id == id)
            .map(|pos| self.inprogres_instructions.remove(pos))
        {
            (Some((&inst).into()), inst.mark_completed(data))
        } else {
            (None, false)
        }
    }
}
