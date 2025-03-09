use ash::vk;
use bevy_log::prelude::*;

pub fn create_texture_view(
    wgpu_device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::TextureView, vk::DeviceMemory, u64) {
    let (texture, memory, memory_size) = create_texture(wgpu_device, width, height);

    let desc = wgpu::TextureViewDescriptor {
        label: Some("Imported Vulkan Texture View"),
        // format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        // dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        // mip_level_count: Some(1),
        base_array_layer: 0,
        ..Default::default()
    };

    (texture.create_view(&desc), memory, memory_size)
}

fn create_texture(
    wgpu_device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, vk::DeviceMemory, u64) {
    unsafe {
        let device = wgpu_device
            .as_hal::<wgpu::hal::api::Vulkan, _, _>(|device| {
                device.expect("No vulkan device").raw_device().clone()
            })
            .expect("No vulkan device");

        let (image, memory, memory_size) = create_vulkan_texture(&device, width, height);
        info!("image: {image:?} memory: {memory:?}");

        let desc = wgpu::hal::TextureDescriptor {
            label: Some("Imported Vulkan Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::hal::TextureUses::STORAGE_READ,
            memory_flags: wgpu::hal::MemoryFlags::empty(),
            view_formats: Vec::new(),
        };

        let descriptor = wgpu::TextureDescriptor {
            label: Some("Imported Vulkan Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = wgpu::hal::vulkan::Device::texture_from_raw(image, &desc, None);

        (
            wgpu_device.create_texture_from_hal::<wgpu::hal::api::Vulkan>(texture, &descriptor),
            memory,
            memory_size,
        )
    }
}

pub fn create_vulkan_texture(
    device: &ash::Device,
    width: u32,
    height: u32,
) -> (vk::Image, vk::DeviceMemory, u64) {
    // let plane_layouts = [vk::SubresourceLayout {
    //     offset: offset as u64,
    //     size: 0, // Must be zero, according to the spec.
    //     row_pitch: stride as u64,
    //     ..Default::default()
    // }];

    // let mut format_modifier_info = vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
    //     .drm_format_modifier(modifier.into())
    //     .plane_layouts(&plane_layouts);

    // let mut external_memory_info = vk::ExternalMemoryImageCreateInfo::default()
    //     .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

    // let create_info = vk::ImageCreateInfo::default()
    //     .image_type(vk::ImageType::TYPE_2D)
    //     .format(vk::Format::R8G8B8A8_UNORM)
    //     .extent(vk::Extent3D {
    //         width,
    //         height,
    //         depth: 1,
    //     })
    //     .mip_levels(1)
    //     .array_layers(1)
    //     .samples(vk::SampleCountFlags::TYPE_1)
    //     .tiling(vk::ImageTiling::OPTIMAL)
    //     .usage(vk::ImageUsageFlags::VIDEO_ENCODE_DPB_KHR)
    //     .sharing_mode(vk::SharingMode::EXCLUSIVE)
    //     .initial_layout(vk::ImageLayout::UNDEFINED)
    //     .push_next(&mut external_memory_info)
    //     .push_next(&mut format_modifier_info);

    let image_create_info = vk::ImageCreateInfo {
        image_type: vk::ImageType::TYPE_2D,
        format: vk::Format::R8G8B8A8_UNORM,
        extent: vk::Extent3D {
            width,
            height,
            depth: 1,
        },
        mip_levels: 1,
        array_layers: 1,
        samples: vk::SampleCountFlags::TYPE_1,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        flags: vk::ImageCreateFlags::empty(),
        queue_family_index_count: 0,
        p_queue_family_indices: std::ptr::null(),
        p_next: std::ptr::null(),
        ..Default::default()
    };

    let image = unsafe { device.create_image(&image_create_info, None) }.unwrap();

    // Get memory requirements
    let mem_requirements = unsafe { device.get_image_memory_requirements(image) };

    let mem_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: mem_requirements.size,
        memory_type_index: 0, // Sélectionnez le bon type de mémoire
        p_next: std::ptr::null(),
        ..Default::default()
    };

    let memory = unsafe { device.allocate_memory(&mem_allocate_info, None) }.unwrap();
    unsafe { device.bind_image_memory(image, memory, 0) }.unwrap();

    // let fd_info = vk::MemoryGetFdInfoKHR {
    //     memory,
    //     handle_type: vk::ExternalMemoryHandleTypeFlagsKHR::DMA_BUF_EXT,
    //     ..Default::default()
    // };

    // ash::khr::external_memory_fd::Device::new(instance, device)

    // device.get_memory_fd(&fd_info);

    // unsafe { device. }
    (image, memory, mem_requirements.size)
}
