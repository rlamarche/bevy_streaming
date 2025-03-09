use ash::vk;
use bevy_ecs::world::World;
use bevy_log::prelude::*;
use bevy_render::{
    render_asset::RenderAssets,
    render_graph::{self, NodeRunError, RenderGraphContext, RenderLabel},
    render_resource::CommandEncoderDescriptor,
    renderer::{RenderContext, RenderDevice, RenderInstance, RenderQueue},
};
use gst_allocators::DmaBufAllocator;
use gst_gl::{GLBaseMemory, GLMemory};
use gst_video::{VideoFrameFlags, VideoMeta};
use wgpu::{Texture, util::DeviceExt};
use wgpu_types::{Extent3d, ImageCopyBuffer, ImageDataLayout};

use crate::capture::texture::create_vulkan_texture;

use super::Captures;

/// `RenderGraph` label for `CaptureNode`
#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
pub struct CaptureLabel;

/// `RenderGraph` node
#[derive(Default)]
pub struct CaptureDriver;

// Copies image content from render target to buffer
impl render_graph::Node for CaptureDriver {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let captures = world.get_resource::<Captures>().unwrap();
        let gpu_images = world
            .get_resource::<RenderAssets<bevy_render::texture::GpuImage>>()
            .unwrap();

        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let vk_device = unsafe {
            render_device
                .wgpu_device()
                .as_hal::<wgpu::hal::api::Vulkan, _, _>(|device| {
                    device.expect("missing vulkan device").raw_device().clone()
                })
                .unwrap()
        };

        let vk_instance = unsafe { render_instance.as_hal::<wgpu::hal::api::Vulkan>().unwrap() };

        let shared_vk_instance = vk_instance.shared_instance();

        let external_memory_fd_device = unsafe {
            ash::khr::external_memory_fd::Device::new(shared_vk_instance.raw_instance(), &vk_device)
        };

        for capture in captures.iter() {
            if !capture.enabled() {
                continue;
            }

            let src_image = gpu_images.get(&capture.src_image).unwrap();

            let width = src_image.size.x;
            let height = src_image.size.y;

            let get_fd_info_hkr = vk::MemoryGetFdInfoKHR {
                memory: capture.memory,
                handle_type: vk::ExternalMemoryHandleTypeFlagsKHR::DMA_BUF_EXT,
                ..Default::default()
            };
            // let get_fd_info = vk::MemoryGetFdInfo {
            //     memory: capture.memory,
            //     handle_type: vk::ExternalMemoryHandleTypeFlagsKHR::DMA_BUF_EXT,
            //     ..Default::default()
            // };

            let fd = unsafe {
                external_memory_fd_device
                    .get_memory_fd(&get_fd_info_hkr)
                    .expect("Unable to get memory fd")
            };
            // info!("Got fd: {fd}");

            let dma_buf_allocator = DmaBufAllocator::new();
            let memory = unsafe {
                dma_buf_allocator
                    .alloc(fd, capture.memory_size as usize)
                    .expect("Unagle to alloc gstreamer memory")
            };

            let block_dimensions = src_image.texture_format.block_dimensions();
            let block_size = src_image.texture_format.block_copy_size(None).unwrap();

            // Calculating correct size of image row because
            // copy_texture_to_buffer can copy image only by rows aligned wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            // That's why image in buffer can be little bit wider
            // This should be taken into account at copy from buffer stage
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (src_image.size.x as usize / block_dimensions.0 as usize) * block_size as usize,
            );

            let mut buffer = gst::Buffer::new();
            {
                let buffer_ref = buffer.get_mut().unwrap();
                buffer_ref.append_memory(memory);

                VideoMeta::add_full(
                    buffer_ref,
                    VideoFrameFlags::empty(),
                    gst_video::VideoFormat::Rgba,
                    width,
                    height,
                    &[0],
                    &[padded_bytes_per_row as i32],
                )
                .expect("Unable to add buffer meta");
            }

            capture
                .encoder
                .appsrc
                .push_buffer(buffer)
                .expect("Unable to push buffer");

            // let mut encoder = render_context
            //     .render_device()
            //     .create_command_encoder(&CommandEncoderDescriptor::default());

            // let block_dimensions = src_image.texture_format.block_dimensions();
            // let block_size = src_image.texture_format.block_copy_size(None).unwrap();

            // // Calculating correct size of image row because
            // // copy_texture_to_buffer can copy image only by rows aligned wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            // // That's why image in buffer can be little bit wider
            // // This should be taken into account at copy from buffer stage
            // let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
            //     (src_image.size.x as usize / block_dimensions.0 as usize) * block_size as usize,
            // );

            // let texture_extent = Extent3d {
            //     width: src_image.size.x,
            //     height: src_image.size.y,
            //     depth_or_array_layers: 1,
            // };

            // encoder.copy_texture_to_buffer(
            //     src_image.texture.as_image_copy(),
            //     ImageCopyBuffer {
            //         buffer: &capture.buffer,
            //         layout: ImageDataLayout {
            //             offset: 0,
            //             bytes_per_row: Some(
            //                 std::num::NonZero::<u32>::new(padded_bytes_per_row as u32)
            //                     .unwrap()
            //                     .into(),
            //             ),
            //             rows_per_image: None,
            //         },
            //     },
            //     texture_extent,
            // );

            // // encoder.copy_texture_to_texture(
            // //     src_image.texture.as_image_copy(),
            // //     texture.as_image_copy(),
            // //     texture_extent,
            // // );

            // let render_queue = world.get_resource::<RenderQueue>().unwrap();
            // render_queue.submit(std::iter::once(encoder.finish()));
        }

        Ok(())
    }
}

fn export_wgpu_texture(
    wgpu_texture: &Texture,
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> () {
    unsafe {
        // device.as_hal::<wgpu::hal::api::Vulkan, _, _>(|device| {
        //     device.expect("No Vulkan device").
        // });
        // 1. Extraire le VkImage de wgpu
        let vk_image = wgpu_texture.as_hal::<wgpu::hal::api::Vulkan, _, _>(|texture| {
            texture.expect("No Vulkan texture").raw_handle()
        });

        // let vk_image = vulkan_texture.raw_handle();

        // let get_fd_info = ash::vk::MemoryGetFdInfoKHR {
        //     memory: vk_memory,
        //     handle_type: ash::vk::ExternalMemoryHandleTypeFlagsKHR::OPAQUE_FD,
        //     ..Default::default()
        // };

        // // let vk_memory = vulkan_texture.memory()?.raw_handle();

        // // 2. Obtenir un file descriptor (FD) Vulkan
        // let mut fd: i32 = -1;
        // let export_info = ash::vk::MemoryGetFdInfoKHR {
        //     memory: vk_image as ash::vk::DeviceMemory,
        //     handle_type: vk::ExternalMemoryHandleTypeFlagsKHR::DMA_BUF,
        //     ..Default::default()
        // };

        // let instance = ash::Instance::new();
        // let device = instance.enumerate_physical_devices().unwrap()[0];
        // let vk_device = ash::Device::new(instance.clone(), device);

        // vk_device
        //     .get_memory_fd_khr(&export_info, &mut fd)
        //     .map_err(|_| "Échec de l'export du VkImage")?;

        // // 3. Importer la mémoire dans CUDA
        // let ext_mem_handle = cudaExternalMemoryHandleDesc {
        //     type_: cudaExternalMemoryHandleType_enum::cudaExternalMemoryHandleTypeOpaqueFd,
        //     handle: fd as *mut c_void,
        //     size: (width * height * 4) as u64,
        //     ..Default::default()
        // };

        // let mut ext_mem: CUexternalMemory = ptr::null_mut();
        // cuImportExternalMemory(&mut ext_mem, &ext_mem_handle)
        //     .map_err(|_| "Impossible d'importer mémoire externe")?;

        // // 4. Obtenir un pointeur CUDA sans copie
        // let array_desc = cudaExternalMemoryBufferDesc {
        //     offset: 0,
        //     size: (width * height * 4) as u64,
        //     flags: 0,
        // };

        // let mut dev_ptr: CUdeviceptr = ptr::null_mut();
        // cuExternalMemoryGetMappedBuffer(&mut dev_ptr, ext_mem, &array_desc)
        //     .map_err(|_| "Erreur de mapping du buffer")?;

        // Ok(dev_ptr)
    }
}

// fn import_vulkan_texture_to_wgpu(
//     device: &wgpu::Device,
//     width: u32,
//     height: u32,
//     texture: vk::Image,
// ) -> wgpu::Texture {
//     let descriptor = wgpu::TextureDescriptor {
//         label: Some("Imported Vulkan Texture"),
//         size: wgpu::Extent3d {
//             width,
//             height,
//             depth_or_array_layers: 1,
//         },
//         mip_level_count: 1,
//         sample_count: 1,
//         dimension: wgpu::TextureDimension::D2,
//         format: wgpu::TextureFormat::Rgba8Unorm,
//         usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
//         view_formats: &[],
//     };
//     device.import
//     unsafe { device.create_texture_from_hal::<wgpu::hal::api::Vulkan>(texture, &descriptor) }
// }
