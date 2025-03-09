use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use bevy_asset::{RenderAssetUsages, prelude::*};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_math::UVec2;
use bevy_render::{
    Extract,
    camera::{ManualTextureView, ManualTextureViewHandle, ManualTextureViews, RenderTarget},
    render_resource::Buffer,
    renderer::RenderDevice,
};
use texture::create_texture_view;
use wgpu_types::{
    BufferDescriptor, BufferUsages, Extent3d, TextureDimension, TextureFormat, TextureUsages,
};

use crate::gst_webrtc_encoder::GstWebRtcEncoder;
pub mod driver;
mod texture;

/// `ImageCopier` aggregator in `RenderWorld`
#[derive(Clone, Default, Resource, Deref, DerefMut)]
struct Captures(pub Vec<Capture>);

/// Extracting `ImageCopier`s into render world, because `ImageCopyDriver` accesses them
pub fn capture_extract(mut commands: Commands, captures: Extract<Query<&Capture>>) {
    commands.insert_resource(Captures(captures.iter().cloned().collect::<Vec<Capture>>()));
}

/// Used by `ImageCopyDriver` for copying from render target to buffer
#[derive(Clone, Component)]
pub struct Capture {
    buffer: Buffer,
    enabled: Arc<AtomicBool>,
    src_image: Handle<Image>,
    memory: ash::vk::DeviceMemory,
    memory_size: u64,
    encoder: GstWebRtcEncoder,
}

impl Capture {
    pub fn new(
        src_image: Handle<Image>,
        size: Extent3d,
        render_device: &RenderDevice,
        memory: ash::vk::DeviceMemory,
        memory_size: u64,
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
            memory,
            memory_size,
            encoder,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Capture image state
#[derive(Debug, Default)]
pub enum SceneState {
    #[default]
    // State before any rendering
    BuildScene,
    // Rendering state, stores the number of frames remaining before saving the image
    Render(u32),
}

/// Capture image settings and state
#[derive(Debug, Default, Resource)]
pub struct SceneController {
    state: SceneState,
    name: String,
    width: u32,
    height: u32,
    single_image: bool,
}

impl SceneController {
    pub fn new(width: u32, height: u32, single_image: bool) -> SceneController {
        SceneController {
            state: SceneState::BuildScene,
            name: String::from(""),
            width,
            height,
            single_image,
        }
    }
}

/// Setups render target and cpu image for saving, changes scene state into render mode
pub fn setup_render_target(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    manual_texture_views: &mut ResMut<ManualTextureViews>,
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

    // Get the wgpu device
    let wgpu_device = render_device.wgpu_device();

    // Create the texture view for the camera
    let (texture_view, memory, memory_size) = create_texture_view(wgpu_device, width, height);

    let manual_texture_view =
        ManualTextureView::with_default_format(texture_view.into(), UVec2::new(width, height));

    // TODO add global incremented count
    let manual_texture_view_handle = ManualTextureViewHandle(42);
    manual_texture_views.insert(manual_texture_view_handle, manual_texture_view);

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
        memory,
        memory_size,
        encoder,
    ));

    // commands.spawn(ImageToSave(cpu_image_handle));

    // RenderTarget::Image(render_target_image_handle)
    RenderTarget::TextureView(manual_texture_view_handle)
}
