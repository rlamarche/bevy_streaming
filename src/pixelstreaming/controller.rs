use bevy_platform::collections::HashMap;
use crossbeam_channel::Receiver;

use super::handler::PSMessageHandler;

pub struct PSControllerState {
    pub add_remove_handlers: Receiver<(String, Option<PSMessageHandler>)>,
    pub handlers: HashMap<String, PSMessageHandler>,
}
