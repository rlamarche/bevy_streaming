use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use bevy_asset::{RenderAssetUsages, prelude::*};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_render::{
    Extract,
    camera::RenderTarget,
    render_resource::{
        Buffer, BufferDescriptor, BufferUsages, Extent3d, TextureDimension, TextureFormat,
        TextureUsages,
    },
    renderer::RenderDevice,
};

use crate::gst_webrtc_encoder::GstWebRtcEncoder;
pub mod driver;

/// `Captures` aggregator in `RenderWorld`
#[derive(Clone, Default, Resource, Deref, DerefMut)]
pub struct Captures(pub Vec<Capture>);

/// Extracting `Capture`s into render world, because `ImageCopyDriver` accesses them
pub fn capture_extract(mut commands: Commands, captures: Extract<Query<&Capture>>) {
    commands.insert_resource(Captures(captures.iter().cloned().collect::<Vec<Capture>>()));
}

/// Used by `CaptureDriver` for copying from render target to buffer
#[derive(Clone, Component)]
pub struct Capture {
    buffer: Buffer,
    enabled: Arc<AtomicBool>,
    src_image: Handle<Image>,
    encoder: GstWebRtcEncoder,
}

impl Capture {
    pub fn new(
        src_image: Handle<Image>,
        size: Extent3d,
        render_device: &RenderDevice,
        encoder: GstWebRtcEncoder,
    ) -> Self {
        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row((size.width) as usize) * 4;

        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: padded_bytes_per_row as u64 * size.height as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer: cpu_buffer,
            src_image,
            enabled: Arc::new(AtomicBool::new(true)),
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
    encoder: GstWebRtcEncoder,
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

    // This is the texture that will be copied to.
    // let cpu_image = Image::new_fill(
    //     size,
    //     TextureDimension::D2,
    //     &[0; 4],
    //     TextureFormat::bevy_default(),
    //     RenderAssetUsages::default(),
    // );
    // let cpu_image_handle = images.add(cpu_image);

    commands.spawn(Capture::new(
        render_target_image_handle.clone(),
        size,
        render_device,
        encoder,
    ));

    // commands.spawn(ImageToSave(cpu_image_handle));

    RenderTarget::Image(render_target_image_handle.into())
}
