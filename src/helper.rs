use bevy_asset::prelude::*;
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_image::prelude::*;
use bevy_log::prelude::*;
use bevy_render::{prelude::*, renderer::RenderDevice};
use gst::prelude::*;
use gstrswebrtc::webrtcsink;
use std::{marker::PhantomData, sync::Arc};

use crate::{
    capture::setup_render_target, encoder::StreamEncoder, gst_webrtc_encoder::GstWebRtcEncoder, ControllerState, GstWebRtcSettings
};
#[cfg(feature = "livekit")]
use crate::livekit::{LiveKitSettings, LiveKitEncoder};

#[cfg(feature = "pixelstreaming")]
use crate::pixelstreaming::{controller::PSControllerState, handler::PSMessageHandler};

#[derive(SystemParam)]
pub struct StreamerHelper<'w, 's, E: StreamEncoder + 'static> {
    commands: Commands<'w, 's>,
    images: ResMut<'w, Assets<Image>>,
    render_device: Res<'w, RenderDevice>,
    _phantom_encoder: PhantomData<E>
}

pub trait StreamerCameraBuilder<E: StreamEncoder, S> {
    fn new_streamer_camera(&mut self, settings: S) -> impl Bundle;
}

impl<'w, 's> StreamerCameraBuilder<GstWebRtcEncoder, GstWebRtcSettings> 
for StreamerHelper<'w, 's, GstWebRtcEncoder>
{
    fn new_streamer_camera(&mut self, settings: GstWebRtcSettings) -> impl Bundle {
        let encoder = GstWebRtcEncoder::with_settings(settings.clone())
            .expect("Unable to create gst encoder");
        encoder.start().expect("Unable to start pipeline");
        
        let controller_state = if settings.enable_controller {
            match &settings.signalling_server {
                #[cfg(feature = "pixelstreaming")]
                crate::SignallingServer::PixelStreaming { .. } => {
                    create_pixelstreaming_controller(&encoder)
                }
                _ => ControllerState::None,
            }
        } else {
            ControllerState::None
        };

        let render_target = setup_render_target(
            &mut self.commands,
            &mut self.images,
            &self.render_device,
            settings.width,
            settings.height,
            Arc::new(encoder),
        );

        let camera = Camera {
            target: render_target,
            ..Default::default()
        };

        (camera, controller_state)
    }
}

#[cfg(feature = "livekit")]
impl<'w, 's> StreamerCameraBuilder<LiveKitEncoder, LiveKitSettings> 
for StreamerHelper<'w, 's, LiveKitEncoder>
{
    fn new_streamer_camera(&mut self, settings: LiveKitSettings) -> impl Bundle {
        let encoder = LiveKitEncoder::new(settings.clone())
            .expect("Unable to create LiveKit encoder");

        let render_target = setup_render_target(
            &mut self.commands,
            &mut self.images,
            &self.render_device,
            settings.width,
            settings.height,
            encoder,
        );

        let camera = Camera {
            target: render_target,
            ..Default::default()
        };

        (camera, ControllerState::None)
    }
}

#[cfg(feature = "pixelstreaming")]
fn create_pixelstreaming_controller(encoder: &GstWebRtcEncoder) -> ControllerState {
    use bevy_platform::collections::HashMap;

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
