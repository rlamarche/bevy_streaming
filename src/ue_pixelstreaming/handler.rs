use bevy_log::prelude::*;
use crossbeam_channel::Receiver;
use gst::glib::prelude::*;
use gst_webrtc::WebRTCDataChannel;
use gstrswebrtc::webrtcsink::BaseWebRTCSink;

use super::message::UeMessage;

#[allow(dead_code)]
#[derive(Debug)]
pub struct UeMessageHandler {
    signal_handler_id: glib::SignalHandlerId,
    data_channel: WebRTCDataChannel,
    pub message_receiver: Receiver<UeMessage>,
}

impl UeMessageHandler {
    pub fn new(element: &BaseWebRTCSink, webrtcbin: &gst::Element, session_id: &str) -> Self {
        info!("Creating Pixel Streaming data channel");
        let channel = webrtcbin.emit_by_name::<WebRTCDataChannel>(
            "create-data-channel",
            &[
                &"input",
                &gst::Structure::builder("config")
                    .field("priority", gst_webrtc::WebRTCPriorityType::High)
                    .build(),
            ],
        );

        let session_id = session_id.to_string();

        let (sender, receiver) = crossbeam_channel::unbounded::<UeMessage>();

        #[allow(unused)]
        Self {
            signal_handler_id: channel.connect_closure("on-message-data", false, {
                let sender = sender.clone();
                glib::closure!(
                    #[watch]
                    element,
                    #[strong]
                    session_id,
                    move |_channel: &WebRTCDataChannel, data: &glib::Bytes| {
                        match UeMessage::try_from(data.get(..).unwrap()) {
                            Ok(message) => {
                                sender.send(message).unwrap();
                            }
                            Err(error) => {
                                warn!("Unable to decode UE Message: {}", error);
                            }
                        }
                    }
                )
            }),
            data_channel: channel,
            message_receiver: receiver,
        }
    }
}
