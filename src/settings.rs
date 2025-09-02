#[derive(Clone)]
pub enum SignallingServer {
    GstWebRtc {
        uri: String,
        peer_id: Option<String>,
    },
    #[cfg(feature = "pixelstreaming")]
    PixelStreaming {
        uri: String,
        streamer_id: Option<String>,
    },
}

impl AsRef<Self> for SignallingServer {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[derive(Clone, Default)]
pub enum CongestionControl {
    #[default]
    Disabled,
    Homegrown,
    GoogleCongestionControl,
}

#[derive(Clone)]
pub struct GstWebRtcSettings {
    pub signalling_server: SignallingServer,
    pub width: u32,
    pub height: u32,
    pub video_caps: Option<String>,
    pub congestion_control: Option<CongestionControl>,
    /// Enables converting controller events to mouse/keyboard events
    pub enable_controller: bool,
}
