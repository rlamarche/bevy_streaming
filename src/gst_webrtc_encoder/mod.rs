use std::str::FromStr;

use anyhow::Result;
use bevy_image::Image;
use bevy_log::prelude::*;
use derive_more::derive::{Display, Error};
use drm_fourcc::DrmFourcc;
use gst::prelude::*;
use gstrswebrtc::{
    signaller::{Signallable, Signaller},
    webrtcsink::{self, BaseWebRTCSink, WebRTCSinkCongestionControl},
};

#[cfg(feature = "pixelstreaming")]
use crate::pixelstreaming::signaller::UePsSignaller;
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
            #[cfg(feature = "pixelstreaming")]
            SignallingServer::PixelStreaming { uri, streamer_id } => {
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

        let caps = gst::Caps::builder("video/x-raw")
            .features([gst_allocators::CAPS_FEATURE_MEMORY_DMABUF])
            .field("format", "DMA_DRM")
            // .field("format", "RGBA")
            // .field("drm-format", "ABGR8888")
            .field("drm-format", DrmFourcc::Abgr8888.to_string())
            // .field("drm-format", DrmFourcc::Rgba8888.to_string())
            .field("width", settings.width)
            .field("height", settings.height)
            .field("framerate", gst::Fraction::new(60, 1))
            .build();

        info!("caps: {:?}", caps);

        let appsrc = gst_app::AppSrc::builder()
            .name("appsrc")
            // .do_timestamp(true)
            .is_live(true)
            .caps(&caps)
            .format(gst::Format::Bytes)
            // Allocate space for 1 buffer
            .max_bytes((settings.width * settings.height * 4).into())
            .build();

        // let capsfilter = gst::ElementFactory::make("capsfilter")
        //     .property("caps", &caps)
        //     .build()?;

        // let glcolorconvert = gst::ElementFactory::make("glcolorconvert").build()?;
        let glupload = gst::ElementFactory::make("glupload").build()?;

        let glupload_caps = gst::Caps::builder("video/x-raw")
            .features([gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY])
            .field("format", "RGBA")
            // .field("format", "RGBA")
            // .field("drm-format", "ABGR8888")
            // .field("drm-format", DrmFourcc::Abgr8888.to_string())
            // .field("drm-format", DrmFourcc::Rgba8888.to_string())
            .field("width", settings.width)
            .field("height", settings.height)
            .field("framerate", gst::Fraction::new(60, 1))
            .build();

        info!("caps: {:?}", glupload_caps);

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property("caps", &glupload_caps)
            .build()?;

        let nvh264enc = gst::ElementFactory::make("nvh264enc").build()?;

        // let glimagesink = gst::ElementFactory::make("glimagesink").build()?;
        // let videoconvert = gst::ElementFactory::make("videoconvert").build()?;
        let fakesink = gst::ElementFactory::make("fakesink").build()?;
        // let gldownload = gst::ElementFactory::make("gldownload").build()?;
        // let autovideosink = gst::ElementFactory::make("autovideosink").build()?;

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

        pipeline.add_many([appsrc.upcast_ref(), &glupload, &nvh264enc, &fakesink])?;
        gst::Element::link_many([appsrc.upcast_ref(), &glupload, &nvh264enc, &fakesink])?;
        // pipeline.add_many([
        //     appsrc.upcast_ref(),
        //     &capsfilter,
        //     &glupload,
        //     &glcolorconvert,
        //     &glimagesink,
        // ])?;
        // gst::Element::link_many([
        //     appsrc.upcast_ref(),
        //     &capsfilter,
        //     &glupload,
        //     &glcolorconvert,
        //     &glimagesink,
        // ])?;
        // pipeline.add_many([appsrc.upcast_ref(), &videoconvert, webrtcsink.upcast_ref()])?;
        // gst::Element::link_many([appsrc.upcast_ref(), &videoconvert, webrtcsink.upcast_ref()])?;

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

    pub fn encode(&mut self, image: &Image) -> anyhow::Result<()> {
        if !self.started {
            self.start()?;
        }

        let mut buffer = gst::Buffer::with_size(image.data.len()).unwrap();
        {
            let buffer = buffer.get_mut().unwrap();
            buffer.copy_from_slice(0, &image.data).unwrap();
        }

        let _ = self.appsrc.push_buffer(buffer);

        Ok(())
    }
    pub fn finish(self: Box<Self>) {
        self.pipeline.set_state(gst::State::Null).unwrap();
    }
}
