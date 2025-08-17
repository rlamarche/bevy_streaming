use anyhow::Result;
use bevy_log::prelude::*;
use derive_more::derive::{Display, Error};
use gst::prelude::*;
use gstrswebrtc::{
    signaller::{Signallable, Signaller},
    webrtcsink::{self, BaseWebRTCSink, WebRTCSinkCongestionControl},
};

#[cfg(feature = "pixelstreaming")]
use crate::pixelstreaming::signaller::UePsSignaller;
use crate::{CongestionControl, SignallingServer, StreamerSettings, encoder::StreamEncoder};

#[derive(Debug, Display, Error)]
#[display("Received error from {src}: {error} (debug: {debug:?})")]
struct ErrorMessage {
    src: glib::GString,
    error: glib::Error,
    debug: Option<glib::GString>,
}

impl Into<Signallable> for &SignallingServer {
    fn into(self) -> Signallable {
        match self {
            SignallingServer::GstWebRtc { uri, peer_id } => {
                let signaller = Signaller::default();
                signaller.set_property_from_str("uri", uri);
                if let Some(peer_id) = peer_id {
                    signaller.set_property_from_str("peer-id", peer_id);
                }
                signaller.upcast()
            }
            #[cfg(feature = "pixelstreaming")]
            SignallingServer::PixelStreaming { uri, streamer_id } => {
                let signaller = UePsSignaller::default();
                signaller.set_property_from_str("uri", uri);
                if let Some(streamer_id) = streamer_id {
                    signaller.set_property_from_str("streamer-id", streamer_id);
                }
                signaller.upcast()
            }
            #[cfg(feature = "livekit")]
            SignallingServer::LiveKit { .. } => {
                panic!("LiveKit signalling should use LiveKitEncoder instead of GstWebRtcEncoder")
            }
        }
    }
}

#[derive(Clone)]
pub struct GstWebRtcEncoder {
    #[allow(dead_code)]
    settings: StreamerSettings,
    pipeline: gst::Pipeline,
    pub appsrc: gst_app::AppSrc,
    pub webrtcsink: BaseWebRTCSink,
}

impl GstWebRtcEncoder {
    pub fn with_settings(settings: StreamerSettings) -> Result<Self> {
        gst::init()?;

        let pipeline = gst::Pipeline::default();

        // Specify the format we want to provide as application into the pipeline
        // by creating a video info with the given format and creating caps from it for the appsrc element.
        let video_info = gst_video::VideoInfo::builder(
            gst_video::VideoFormat::Rgba,
            settings.width,
            settings.height,
        )
        .build()
        .expect("Failed to create video info");

        let appsrc = gst_app::AppSrc::builder()
            .name("appsrc")
            .do_timestamp(true)
            .is_live(true)
            .caps(&video_info.to_caps().unwrap())
            .format(gst::Format::Bytes)
            // Allocate space for 1 buffer
            .max_bytes((settings.width * settings.height * 4).into())
            .build();

        // let queue = gst::ElementFactory::make("queue").build()?;
        // queue.set_property_from_str("leaky", "downstream");

        let videoconvert = gst::ElementFactory::make("videoconvert").build()?;

        let webrtcsink =
            webrtcsink::BaseWebRTCSink::with_signaller(settings.signalling_server.as_ref().into());

        if let Some(video_caps) = &settings.video_caps {
            webrtcsink.set_property_from_str("video-caps", video_caps);
        }
        if let Some(congestion_control) = &settings.congestion_control {
            webrtcsink.set_property(
                "congestion-control",
                match congestion_control {
                    CongestionControl::Disabled => WebRTCSinkCongestionControl::Disabled,
                    CongestionControl::Homegrown => WebRTCSinkCongestionControl::Homegrown,
                    CongestionControl::GoogleCongestionControl => {
                        WebRTCSinkCongestionControl::GoogleCongestionControl
                    }
                },
            );
        }

        pipeline.add_many([
            appsrc.upcast_ref(),
            // &queue,
            &videoconvert,
            webrtcsink.upcast_ref(),
        ])?;
        gst::Element::link_many([
            appsrc.upcast_ref(),
            // &queue,
            &videoconvert,
            webrtcsink.upcast_ref(),
        ])?;

        Ok(Self {
            settings,
            pipeline,
            appsrc,
            webrtcsink,
        })
    }

    pub fn start(&self) -> Result<()> {
        info!("Start pipeline");
        self.pipeline.set_state(gst::State::Playing)?;

        Ok(())
    }

    pub fn process_events(&self) -> Result<()> {
        let bus = self
            .pipeline
            .bus()
            .expect("Pipeline without bus. Shouldn't happen!");

        for msg in bus.iter() {
            use gst::MessageView;

            // info!("Msg: {:#?}", msg.view());
            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    self.pipeline.set_state(gst::State::Null)?;
                    return Err(ErrorMessage {
                        src: msg
                            .src()
                            .map(|s| s.path_string())
                            .unwrap_or_else(|| glib::GString::from("UNKNOWN")),
                        error: err.error(),
                        debug: err.debug(),
                    }
                    .into());
                }
                _ => (),
            }
        }

        Ok(())
    }

    pub fn push_buffer(&self, data: &Vec<u8>) -> anyhow::Result<()> {
        let mut buffer = gst::Buffer::with_size(data.len()).unwrap();
        {
            let buffer = buffer.get_mut().unwrap();
            buffer.copy_from_slice(0, data).unwrap();
        }

        let _ = self.appsrc.push_buffer(buffer);

        Ok(())
    }
    pub fn finish(self: Box<Self>) {
        self.pipeline.set_state(gst::State::Null).unwrap();
    }
}

impl StreamEncoder for GstWebRtcEncoder {
    fn push_frame(&self, frame_data: &[u8]) -> Result<()> {
        self.push_buffer(&frame_data.to_vec())
    }

    fn start(&self) -> Result<()> {
        GstWebRtcEncoder::start(self)
    }
}
