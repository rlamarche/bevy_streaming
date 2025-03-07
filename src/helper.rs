use bevy_asset::prelude::*;
use bevy_capture::{CameraTargetHeadless, CaptureBundle};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_image::prelude::*;
use bevy_log::prelude::*;
use bevy_render::prelude::*;
use bevy_utils::HashMap;
use gst::prelude::*;
use gstrswebrtc::webrtcsink;

use crate::{ControllerState, Encoder, StreamerSettings, gst_webrtc_encoder::GstWebRtcEncoder};

#[cfg(feature = "pixelstreaming")]
use crate::pixelstreaming::{controller::PSControllerState, handler::PSMessageHandler};

#[derive(SystemParam)]
pub struct StreamerHelper<'w> {
    images: ResMut<'w, Assets<Image>>,
}

impl<'w> StreamerHelper<'w> {
    pub fn new_streamer_camera(&mut self, settings: StreamerSettings) -> impl Bundle {
        let camera =
            Camera::default().target_headless(settings.width, settings.height, &mut self.images);

        let encoder = GstWebRtcEncoder::with_settings(settings.clone())
            .expect("Unable to create gst encoder");

        // Bind controller if enabled
        let controller_state = if settings.enable_controller {
            match settings.signalling_server {
                crate::SignallingServer::GstWebRtc { .. } => {
                    // TODO bind navigation events
                    ControllerState::None
                }

                #[cfg(feature = "pixelstreaming")]
                crate::SignallingServer::PixelStreaming { .. } => {
                    create_pixelstreaming_controller(&encoder)
                }
            }
        } else {
            ControllerState::None
        };

        (
            camera,
            CaptureBundle::default(),
            Encoder(encoder),
            controller_state,
        )
    }
}

#[cfg(feature = "pixelstreaming")]
fn create_pixelstreaming_controller(encoder: &GstWebRtcEncoder) -> ControllerState {
    let (sender, receiver) = crossbeam_channel::unbounded::<(String, Option<PSMessageHandler>)>();

    encoder
        .webrtcsink
        .connect_closure("consumer-added", false, {
            let sender = sender.clone();
            glib::closure!(move |sink: &webrtcsink::BaseWebRTCSink,
                                 peer_id: &str,
                                 webrtcbin: &gst::Element| {
                info!("New consumer: {}", peer_id);

                let message_handler = PSMessageHandler::new(sink, webrtcbin, peer_id);

                sender
                    .send((peer_id.to_string(), Some(message_handler)))
                    .unwrap();
            })
        });

    encoder
        .webrtcsink
        .connect_closure("consumer-removed", false, {
            let sender = sender.clone();
            glib::closure!(move |_sink: &webrtcsink::BaseWebRTCSink,
                                 peer_id: &str,
                                 _webrtcbin: &gst::Element| {
                info!("Consumer removed: {}", peer_id);

                sender.send((peer_id.to_string(), None)).unwrap();
            })
        });

    ControllerState::PSControllerState(PSControllerState {
        add_remove_handlers: receiver,
        handlers: HashMap::new(),
    })
}
