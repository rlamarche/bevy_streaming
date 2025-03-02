use bevy_utils::HashMap;
use crossbeam_channel::Receiver;

use super::handler::UeMessageHandler;

pub struct UeControllerState {
    pub add_remove_handlers: Receiver<(String, Option<UeMessageHandler>)>,
    pub handlers: HashMap<String, UeMessageHandler>,
}
