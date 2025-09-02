use bevy_asset::{RenderAssetUsages, prelude::*};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_log::prelude::*;
use bevy_render::{
    Extract,
    camera::RenderTarget,
    render_resource::{
        Buffer, BufferDescriptor, BufferUsages, Extent3d, TextureDimension, TextureFormat,
        TextureUsages,
    },
    renderer::RenderDevice,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crate::encoder::EncoderHandle;
pub mod driver;

/// `Captures` aggregator in `RenderWorld`
#[derive(Clone, Default, Resource, Deref, DerefMut)]
pub struct Captures(pub Vec<Capture>);

/// Extracting `Capture`s into render world, because `ImageCopyDriver` accesses them
pub fn capture_extract(mut commands: Commands, captures: Extract<Query<&Capture>>) {
    commands.insert_resource(Captures(captures.iter().cloned().collect::<Vec<Capture>>()));
}

#[derive(Clone)]
struct CaptureBuffer {
    buffer: Buffer,
    in_use: Arc<AtomicBool>,
}

/// Used by `CaptureDriver` for copying from render target to buffer
#[derive(Clone, Component)]
pub struct Capture {
    buffers: Vec<CaptureBuffer>,
    current: Arc<AtomicUsize>,
    skip: Arc<AtomicBool>,

    enabled: Arc<AtomicBool>,
    src_image: Handle<Image>,
    encoder: EncoderHandle,
}

pub struct SendBufferJob {
    // slice: BufferSlice<'static>,
    buffer: Buffer,
    // len: usize,
    encoder: EncoderHandle,
    // in_use: Arc<AtomicBool>,
    capture_idx: usize,
    buffer_idx: usize,
}

#[derive(Resource, Clone)]
pub struct WorkerSendBuffer {
    pub tx: Sender<SendBufferJob>,
}

#[derive(Resource, Clone)]
pub struct ReleaseBufferSignal {
    pub rx: Receiver<ReleaseSignal>,
}

pub struct ReleaseSignal {
    // index of the capture
    capture_idx: usize,
    // index of the buffer to release
    buffer_idx: usize,
}

impl Capture {
    pub fn new(
        src_image: Handle<Image>,
        size: Extent3d,
        render_device: &RenderDevice,
        encoder: EncoderHandle,
    ) -> Self {
        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row((size.width) as usize) * 4;

        let buffers = (0..3) // triple buffering
            .map(|_| {
                let buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("Capture buffer"),
                    size: padded_bytes_per_row as u64 * size.height as u64,
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                CaptureBuffer {
                    buffer,
                    in_use: Arc::new(AtomicBool::new(false)),
                }
            })
            .collect();

        Self {
            buffers,
            current: Arc::new(AtomicUsize::new(0)),
            skip: Arc::new(AtomicBool::new(false)),
            enabled: Arc::new(AtomicBool::new(true)),
            src_image,
            encoder,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Setups render target and cpu image for saving, changes scene state into render mode
pub fn setup_render_target(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    render_device: &Res<RenderDevice>,
    // render_instance: &Res<RenderInstance>,
    width: u32,
    height: u32,
    encoder: EncoderHandle,
) -> RenderTarget {
    let size = Extent3d {
        width,
        height,
        ..Default::default()
    };

    // This is the texture that will be rendered to.
    let mut render_target_image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::bevy_default(),
        RenderAssetUsages::default(),
    );
    render_target_image.texture_descriptor.usage |=
        TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    let render_target_image_handle = images.add(render_target_image);

    commands.spawn(Capture::new(
        render_target_image_handle.clone(),
        size,
        render_device,
        encoder,
    ));

    // commands.spawn(ImageToSave(cpu_image_handle));

    RenderTarget::Image(render_target_image_handle.into())
}

pub fn spawn_worker() -> (Sender<SendBufferJob>, Receiver<ReleaseSignal>) {
    let (tx_job, rx_job) = unbounded::<SendBufferJob>();
    let (tx_release, rx_release) = unbounded::<ReleaseSignal>();

    std::thread::spawn(move || {
        while let Ok(job) = rx_job.recv() {
            let slice = job.buffer.slice(..);
            let data = slice.get_mapped_range().to_vec();


            let _ = job.encoder.push_frame(&data);

            if let Err(e) = tx_release.send(ReleaseSignal {
                capture_idx: job.capture_idx,
                buffer_idx: job.buffer_idx,
            }) {
                error!("Release channel closed: {:?}", e);
            }
        }
    });

    (tx_job, rx_release)
}
