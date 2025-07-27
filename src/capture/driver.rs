use std::time::{Duration, Instant};

use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_render::{
    render_asset::RenderAssets,
    render_graph::{self, NodeRunError, RenderGraphContext, RenderLabel},
    render_resource::{
        CommandEncoderDescriptor, Maintain, MapMode, TexelCopyBufferInfo, TexelCopyBufferLayout,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
};

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

        let mut encoder = render_context
            .render_device()
            .create_command_encoder(&CommandEncoderDescriptor::default());

        for capture in captures.iter() {
            if !capture.enabled() {
                continue;
            }

            let src_image = gpu_images.get(&capture.src_image).unwrap();

            let block_dimensions = src_image.texture_format.block_dimensions();
            let block_size = src_image.texture_format.block_copy_size(None).unwrap();

            // Calculating correct size of image row because
            // copy_texture_to_buffer can copy image only by rows aligned wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            // That's why image in buffer can be little bit wider
            // This should be taken into account at copy from buffer stage
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (src_image.size.width as usize / block_dimensions.0 as usize) * block_size as usize,
            );

            encoder.copy_texture_to_buffer(
                src_image.texture.as_image_copy(),
                TexelCopyBufferInfo {
                    buffer: &capture.buffer,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZero::<u32>::new(padded_bytes_per_row as u32)
                                .unwrap()
                                .into(),
                        ),
                        rows_per_image: None,
                    },
                },
                src_image.size,
            );
        }

        let render_queue = world.get_resource::<RenderQueue>().unwrap();
        render_queue.submit(std::iter::once(encoder.finish()));

        // render_context.render_device().poll(Maintain::Wait);
        // render_device.poll(Maintain::Wait);

        Ok(())
    }
}

// /// runs in render world after Render stage to send image from buffer via channel (receiver is in main world)
// pub fn receive_image_from_buffer(mut captures: ResMut<Captures>, render_device: Res<RenderDevice>) {
//     for capture in captures.0.iter_mut() {
//         if !capture.enabled() {
//             continue;
//         }

//         // Finally time to get our data back from the gpu.
//         // First we get a buffer slice which represents a chunk of the buffer (which we
//         // can't access yet).
//         // We want the whole thing so use unbounded range.
//         let buffer_slice = capture.buffer.slice(..);

//         // Now things get complicated. WebGPU, for safety reasons, only allows either the GPU
//         // or CPU to access a buffer's contents at a time. We need to "map" the buffer which means
//         // flipping ownership of the buffer over to the CPU and making access legal. We do this
//         // with `BufferSlice::map_async`.
//         //
//         // The problem is that map_async is not an async function so we can't await it. What
//         // we need to do instead is pass in a closure that will be executed when the slice is
//         // either mapped or the mapping has failed.
//         //
//         // The problem with this is that we don't have a reliable way to wait in the main
//         // code for the buffer to be mapped and even worse, calling get_mapped_range or
//         // get_mapped_range_mut prematurely will cause a panic, not return an error.
//         //
//         // Using channels solves this as awaiting the receiving of a message from
//         // the passed closure will force the outside code to wait. It also doesn't hurt
//         // if the closure finishes before the outside code catches up as the message is
//         // buffered and receiving will just pick that up.
//         //
//         // It may also be worth noting that although on native, the usage of asynchronous
//         // channels is wholly unnecessary, for the sake of portability to Wasm
//         // we'll use async channels that work on both native and Wasm.

//         let (s, r) = crossbeam_channel::bounded(1);

//         // Maps the buffer so it can be read on the cpu
//         buffer_slice.map_async(MapMode::Read, move |r| match r {
//             // This will execute once the gpu is ready, so after the call to poll()
//             Ok(r) => s.send(r).expect("Failed to send map update"),
//             Err(err) => panic!("Failed to map buffer {err}"),
//         });

//         // In order for the mapping to be completed, one of three things must happen.
//         // One of those can be calling `Device::poll`. This isn't necessary on the web as devices
//         // are polled automatically but natively, we need to make sure this happens manually.
//         // `Maintain::Wait` will cause the thread to wait on native but not on WebGpu.

//         // This blocks until the gpu is done executing everything
//         render_device.poll(Maintain::wait()).panic_on_timeout();

//         // This blocks until the buffer is mapped
//         r.recv().expect("Failed to receive the map_async message");

//         // This could fail on app exit, if Main world clears resources (including receiver) while Render world still renders
//         // let _ = sender.send(buffer_slice.get_mapped_range().to_vec());

//         let data = buffer_slice.get_mapped_range().to_vec();
//         capture
//             .encoder
//             .push_buffer(&data)
//             .expect("Unable to push buffer to encoder");

//         // We need to make sure all `BufferView`'s are dropped before we do what we're about
//         // to do.
//         // Unmap so that we can copy to the staging buffer in the next iteration.
//         capture.buffer.unmap();
//     }
// }

/// Optimized: batches GPU -> CPU buffer reads for all active captures in one render stage frame
pub fn receive_image_from_buffer(mut captures: ResMut<Captures>, render_device: Res<RenderDevice>) {
    use crossbeam_channel::bounded;

    let mut receivers = Vec::new();
    let mut buffer_slices = Vec::new();

    for capture in captures.0.iter_mut() {
        if !capture.enabled() {
            continue;
        }

        let buffer_slice = capture.buffer.slice(..);
        let (sender, receiver) = bounded(1);

        // info!("Map async");
        buffer_slice.map_async(MapMode::Read, move |result| match result {
            Ok(_) => {
                // info!("Map async done");
                sender.send(()).expect("Failed to send map completion")
            }
            Err(err) => panic!("Failed to map buffer: {err}"),
        });

        receivers.push(receiver);
        buffer_slices.push((
            capture.encoder.appsrc.clone(),
            &capture.buffer,
            buffer_slice,
        ));
    }

    if !receivers.is_empty() {
        let start = Instant::now();
        let timeout = Duration::from_millis(1000); // Ajustable

        loop {
            render_device.poll(Maintain::Poll);

            // Check if all receivers are ready
            let all_ready = receivers.iter().all(|r| r.try_recv().is_ok());

            if all_ready {
                break;
            }

            if start.elapsed() > timeout {
                panic!("Timeout while waiting for GPU buffer mapping");
            }

            std::thread::sleep(Duration::from_millis(1));
        }

        info!("All ready");

        for (appsrc, buffer, buffer_slice) in buffer_slices {
            let data = buffer_slice.get_mapped_range().to_vec();
            let mut gst_buffer = gst::Buffer::with_size(data.len()).unwrap();
            {
                info!("Copying buffer");
                let buffer = gst_buffer.get_mut().unwrap();
                buffer.copy_from_slice(0, &data).unwrap();
                info!("Buffer copied");
            }

            let _ = appsrc.push_buffer(gst_buffer);
            info!("Buffer sent");

            buffer.unmap();
        }

        info!("Finished sending buffers");
    }
}
