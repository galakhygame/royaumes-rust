use anyhow::anyhow;
use event_model::{Command, Event, ModelEvent};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum SimpleCommand {
    Add(u32),
    Remove(u32),
    Set(u32),
}

impl Command for SimpleCommand {
    fn command_name(&self) -> &str {
        match &self {
            SimpleCommand::Add(_) => "Add",
            SimpleCommand::Remove(_) => "Remove",
            SimpleCommand::Set(_) => "Set",
        }
    }
}

#[derive(Deserialize, Serialize)]
pub enum SimpleEvent {
    Added(u32),
    Removed(u32),
}

impl Event for SimpleEvent {
    fn event_name(&self) -> &str {
        match &self {
            SimpleEvent::Added(_) => "added",
            SimpleEvent::Removed(_) => "removed",
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimpleModel {
    pub nb: u32,
    pub position: u64,
}

impl ModelEvent for SimpleModel {
    type Event = SimpleEvent;
    type Command = SimpleCommand;

    fn play_event(&mut self, event: &Self::Event) {
        match event {
            SimpleEvent::Added(n) => self.nb += n,
            SimpleEvent::Removed(n) => self.nb -= n,
        }
    }

    fn try_command(&self, command: &Self::Command) -> anyhow::Result<Vec<Self::Event>> {
        match command {
            SimpleCommand::Add(n) => {
                if self.nb.checked_add(*n).is_none() {
                    Err(anyhow!("{} cannot be added to {}", n, self.nb))
                } else {
                    Ok(vec![SimpleEvent::Added(*n)])
                }
            }
            SimpleCommand::Remove(n) => {
                if *n > self.nb {
                    Err(anyhow!("{} cannot be removed to {}", n, self.nb))
                } else {
                    Ok(vec![SimpleEvent::Removed(*n)])
                }
            }
            SimpleCommand::Set(n) => {
                Ok(vec![SimpleEvent::Removed(self.nb), SimpleEvent::Added(*n)])
            }
        }
    }

    fn get_position(&self) -> u64 {
        self.position
    }

    fn set_position(&mut self, pos: u64) {
        self.position = pos;
    }
}
