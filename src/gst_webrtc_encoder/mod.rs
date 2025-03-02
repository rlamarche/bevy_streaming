use anyhow::Result;
use bevy_capture::Encoder;
use bevy_image::Image;
use bevy_log::prelude::*;
use derive_more::derive::{Display, Error};
use gst::prelude::*;
use gstrswebrtc::{
    signaller::{Signallable, Signaller},
    webrtcsink::{self, BaseWebRTCSink, WebRTCSinkCongestionControl},
};

#[cfg(feature = "ue_pixelstreaming")]
use crate::ue_pixelstreaming::signaller::UePsSignaller;
use crate::{CongestionControl, SignallingServer, StreamerSettings};

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
            #[cfg(feature = "ue_pixelstreaming")]
            SignallingServer::UePixelStreaming { uri, streamer_id } => {
                let signaller = UePsSignaller::default();
                signaller.set_property_from_str("uri", uri);
                if let Some(streamer_id) = streamer_id {
                    signaller.set_property_from_str("streamer-id", streamer_id);
                }
                signaller.upcast()
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
    started: bool,
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
        .fps(gst::Fraction::new(60, 1))
        .build()
        .expect("Failed to create video info");

        let appsrc = gst_app::AppSrc::builder()
            .name("appsrc")
            .do_timestamp(true)
            .is_live(true)
            .caps(&video_info.to_caps().unwrap())
            .format(gst::Format::Time)
            .build();

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

        pipeline.add_many([appsrc.upcast_ref(), &videoconvert, webrtcsink.upcast_ref()])?;
        gst::Element::link_many([appsrc.upcast_ref(), &videoconvert, webrtcsink.upcast_ref()])?;

        Ok(Self {
            settings,
            pipeline,
            appsrc,
            webrtcsink,
            started: false,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        info!("Start pipeline");
        self.pipeline.set_state(gst::State::Playing)?;
        self.started = true;

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
}

impl Encoder for GstWebRtcEncoder {
    fn encode(&mut self, image: &Image) -> bevy_capture::encoder::Result<()> {
        if !self.started {
            self.start()?;
        }
        // let image = image.clone().try_into_dynamic()?;
        // let img_buffer = image.to_rgb8();
        // let mut buffer = gst::Buffer::with_size(img_buffer.len()).unwrap();

        let mut buffer = gst::Buffer::with_size(image.data.len()).unwrap();
        {
            let buffer = buffer.get_mut().unwrap();
            // buffer.copy_from_slice(0, &img_buffer).unwrap();
            buffer.copy_from_slice(0, &image.data).unwrap();
        }

        let _ = self.appsrc.push_buffer(buffer);

        Ok(())
    }
    fn finish(self: Box<Self>) {
        self.pipeline.set_state(gst::State::Null).unwrap();
    }
}
