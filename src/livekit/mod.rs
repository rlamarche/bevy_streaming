use anyhow::{Context, Result};
use bevy_log::prelude::*;
use gst;
use gst::prelude::*;
use gst_app;
use gst_video::{VideoFormat, VideoInfo};
use std::sync::Arc;
use crate::encoder::StreamEncoder;

#[derive(Clone)]
pub struct LiveKitConfig {
    pub url: String,
    pub api_key: String,
    pub api_secret: String,
    pub room_name: String,
    pub participant_identity: String,
    pub participant_name: String,
    pub width: u32,
    pub height: u32,
}

impl LiveKitConfig {
    pub fn from_env(width: u32, height: u32) -> Result<Self> {
        let livekit_url = std::env::var("LIVEKIT_URL")
            .context("LIVEKIT_URL environment variable must be set")?;
        
        let url = if livekit_url.starts_with("https://") {
            livekit_url.replace("https://", "wss://")
        } else if livekit_url.starts_with("http://") {
            livekit_url.replace("http://", "ws://")
        } else {
            livekit_url
        };
        
        Ok(Self {
            url,
            api_key: std::env::var("LIVEKIT_API_KEY")
                .context("LIVEKIT_API_KEY environment variable must be set")?,
            api_secret: std::env::var("LIVEKIT_API_SECRET")
                .context("LIVEKIT_API_SECRET environment variable must be set")?,
            room_name: std::env::var("LIVEKIT_ROOM_NAME")
                .unwrap_or_else(|_| "bevy_streaming_room".to_string()),
            participant_identity: std::env::var("LIVEKIT_PARTICIPANT_IDENTITY")
                .unwrap_or_else(|_| "bevy_streamer".to_string()),
            participant_name: std::env::var("LIVEKIT_PARTICIPANT_NAME")
                .unwrap_or_else(|_| "Bevy Streaming".to_string()),
            width,
            height,
        })
    }

    pub fn new(
        url: String,
        api_key: String,
        api_secret: String,
        room_name: String,
        participant_identity: String,
        participant_name: String,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            url,
            api_key,
            api_secret,
            room_name,
            participant_identity,
            participant_name,
            width,
            height,
        }
    }
}

#[derive(Clone)]
pub struct LiveKitEncoder {
    pipeline: gst::Pipeline,
    appsrc: gst_app::AppSrc,
    width: u32,
    height: u32,
}

impl LiveKitEncoder {
    pub fn new(config: LiveKitConfig) -> Result<Arc<Self>> {
        // Initialize GStreamer if not already initialized
        gst::init()?;
        
        info!("Creating LiveKit encoder with GStreamer...");
        
        // Calculate appropriate bitrate based on resolution
        // Roughly 0.1 bits per pixel for 60fps as baseline
        let pixels = config.width * config.height;
        let bitrate = ((pixels as f32 * 0.1 * 60.0 / 1000.0) as u32).max(1000).min(10000);
        info!("Using bitrate: {} kbps for {}x{} resolution", bitrate, config.width, config.height);
        
        let pipeline_str = format!(
            "appsrc name=video_src format=time is-live=true do-timestamp=true ! \
            video/x-raw,format=RGBA,width={},height={},framerate=60/1 ! \
            queue ! \
            videoconvert ! \
            video/x-raw,format=I420 ! \
            queue ! \
            x264enc tune=zerolatency speed-preset=ultrafast bitrate={} key-int-max=60 ! \
            video/x-h264,profile=baseline ! \
            queue ! \
            livekitwebrtcsink name=livekit \
                signaller::ws-url={} \
                signaller::api-key={} \
                signaller::secret-key={} \
                signaller::room-name={} \
                signaller::identity={} \
                signaller::participant-name=\"{}\" \
                video-caps=\"video/x-h264\"",
            config.width,
            config.height,
            bitrate,
            config.url,
            config.api_key,
            config.api_secret,
            config.room_name,
            config.participant_identity,
            config.participant_name
        );
        
        info!("Creating LiveKit pipeline");
        
        let pipeline = match gst::parse::launch(&pipeline_str) {
            Ok(pipeline) => {
                info!("Successfully created LiveKit WebRTC pipeline");
                pipeline
            }
            Err(e) => {
                error!("Failed to create LiveKit WebRTC pipeline: {}", e);
                
                if gst::ElementFactory::find("livekitwebrtcsink").is_none() {
                    error!("livekitwebrtcsink element not found. Please install gst-plugins-rs with livekit feature enabled.");
                    error!("Build from source: https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs");
                }
                
                return Err(anyhow::anyhow!("Failed to create LiveKit pipeline: {}", e));
            }
        };
        
        let pipeline = pipeline.downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to cast to pipeline"))?;
        
        let appsrc = pipeline
            .by_name("video_src")
            .ok_or_else(|| anyhow::anyhow!("Could not get appsrc element"))?
            .downcast::<gst_app::AppSrc>()
            .map_err(|_| anyhow::anyhow!("Not an appsrc"))?;
        
        appsrc.set_property("format", gst::Format::Time);
        appsrc.set_property("is-live", true);
        appsrc.set_property("do-timestamp", true);
        
        let video_info = VideoInfo::builder(VideoFormat::Rgba, config.width, config.height)
            .fps(gst::Fraction::new(60, 1))
            .build()
            .context("Failed to create video info")?;
        
        let caps = video_info.to_caps()
            .context("Failed to create caps from video info")?;
        appsrc.set_caps(Some(&caps));
        
        let bus = pipeline.bus().ok_or_else(|| anyhow::anyhow!("Pipeline has no bus"))?;
        
        // Spawn a thread to monitor the bus for messages
        let pipeline_weak = pipeline.downgrade();
        std::thread::spawn(move || {
            let Some(pipeline) = pipeline_weak.upgrade() else { return; };
            let Some(bus) = pipeline.bus() else { return; };
            
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                match msg.view() {
                    gst::MessageView::Error(err) => {
                        error!(
                            "LiveKit pipeline error from {:?}: {} ({:?})",
                            err.src().map(|s| s.path_string()),
                            err.error(),
                            err.debug()
                        );
                    }
                    gst::MessageView::Warning(warning) => {
                        warn!(
                            "LiveKit pipeline warning from {:?}: {} ({:?})",
                            warning.src().map(|s| s.path_string()),
                            warning.error(),
                            warning.debug()
                        );
                    }
                    gst::MessageView::StateChanged(state_changed) => {
                        // Log all state changes to understand what's happening
                        let src_name = state_changed.src()
                            .map(|s| s.path_string().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        
                        if src_name.contains("livekit") || src_name.contains("pipeline") {
                            info!(
                                "LiveKit state change [{}]: {:?} -> {:?} (pending: {:?})",
                                src_name,
                                state_changed.old(),
                                state_changed.current(),
                                state_changed.pending()
                            );
                        }
                    }
                    gst::MessageView::Element(element) => {
                        if let Some(structure) = element.structure() {
                            if structure.name() == "GstBinForwarded" {
                                // Skip forwarded messages
                            } else {
                                info!("LiveKit element message: {:?}", structure.name());
                            }
                        }
                    }
                    gst::MessageView::Eos(_) => {
                        warn!("LiveKit pipeline: End of stream - this shouldn't happen!");
                        break;
                    }
                    _ => {}
                }
            }
        });
        
        pipeline.set_state(gst::State::Playing)
            .context("Failed to set pipeline to playing state")?;
        
        info!("LiveKit pipeline started successfully");
        
        Ok(Arc::new(Self {
            pipeline,
            appsrc,
            width: config.width,
            height: config.height,
        }))
    }

    pub fn push_frame(&self, frame_data: &[u8]) -> Result<()> {
        let buffer_size = frame_data.len();
        if buffer_size == 0 {
            return Ok(());
        }
        
        let expected_size = (self.width * self.height * 4) as usize;
        if buffer_size != expected_size {
            warn!("Frame size mismatch: expected {} bytes ({}x{}x4), got {} bytes",
                expected_size, self.width, self.height, buffer_size);
        }
        
        // Check pipeline state
        let state = self.pipeline.state(gst::ClockTime::from_seconds(0));
        if state.1 != gst::State::Playing {
            warn!("Pipeline not in playing state: {:?}", state.1);
        }
        
        let mut buffer = gst::Buffer::with_size(buffer_size)
            .context("Could not allocate buffer")?;
        
        {
            let buffer_ref = buffer.get_mut().unwrap();
            
            let mut map = buffer_ref.map_writable()
                .context("Could not map buffer writable")?;
            map.copy_from_slice(frame_data);
        }
        
        match self.appsrc.push_buffer(buffer) {
            Ok(flow) => {
                if flow != gst::FlowSuccess::Ok {
                    warn!("Push buffer returned non-OK flow: {:?}", flow);
                }
                Ok(())
            },
            Err(e) => {
                error!("Failed to push buffer to LiveKit pipeline: {:?}", e);
                Err(anyhow::anyhow!("Failed to push buffer: {:?}", e))
            }
        }
    }
}

impl Drop for LiveKitEncoder {
    fn drop(&mut self) {
        info!("Shutting down LiveKit pipeline");
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

impl StreamEncoder for LiveKitEncoder {
    fn push_frame(&self, frame_data: &[u8]) -> Result<()> {
        LiveKitEncoder::push_frame(self, frame_data)
    }

    fn start(&self) -> Result<()> {
        Ok(())
    }
}