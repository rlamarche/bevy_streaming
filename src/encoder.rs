use anyhow::Result;
use std::sync::Arc;

pub trait StreamEncoder: Send + Sync {
    fn push_frame(&self, frame_data: &[u8]) -> Result<()>;
    fn start(&self) -> Result<()>;
}

pub type EncoderHandle = Arc<dyn StreamEncoder>;