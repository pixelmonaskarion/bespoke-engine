use image::GenericImageView;
use anyhow::*;
use wgpu::{BindGroupLayout, Device, TextureFormat, TextureUsages, TextureView};

use crate::{binding::{Binding, Resource}, shader::ShaderType};

const STORAGE_FORMATS: [TextureFormat; 3] = [TextureFormat::Rgba32Float, TextureFormat::Rgba16Float, TextureFormat::Rgba8Unorm];

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: wgpu::Extent3d,
    pub format: wgpu::TextureFormat,
}

impl Texture {
    #[allow(dead_code)]
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8], 
        label: &str,
        filter_mode: Option<wgpu::FilterMode>,
        address_mode: Option<wgpu::AddressMode>,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label), None, None, filter_mode, address_mode)
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
        format: Option<wgpu::TextureFormat>,
        _sample_type: Option<wgpu::TextureSampleType>,
        filter_mode: Option<wgpu::FilterMode>,
        address_mode: Option<wgpu::AddressMode>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb),
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb)],
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: address_mode.unwrap_or(wgpu::AddressMode::Repeat),
                address_mode_v: address_mode.unwrap_or(wgpu::AddressMode::Repeat),
                address_mode_w: address_mode.unwrap_or(wgpu::AddressMode::Repeat),
                mag_filter: filter_mode.unwrap_or(wgpu::FilterMode::Nearest),
                min_filter: filter_mode.unwrap_or(wgpu::FilterMode::Nearest),
                mipmap_filter: filter_mode.unwrap_or(wgpu::FilterMode::Nearest),
                ..Default::default()
            }
        );
        
        Ok(Self { texture, view, sampler, size, format: format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb) })
    }

    pub fn blank_texture(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Temp Draw Texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT | if STORAGE_FORMATS.contains(&format) { TextureUsages::STORAGE_BINDING } else { TextureUsages::TEXTURE_BINDING },
                view_formats: &[format],
            }
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        Self {
            sampler,
            texture,
            view,
            size,
            format,
        }
    }

    pub fn blank_texture_3d(device: &wgpu::Device, width: u32, height: u32, depth: u32, format: wgpu::TextureFormat) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Temp 3D Texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC | TextureUsages::COPY_DST | if STORAGE_FORMATS.contains(&format) { TextureUsages::STORAGE_BINDING } else { TextureUsages::TEXTURE_BINDING },
                view_formats: &[format],
            }
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        Self {
            sampler,
            texture,
            view,
            size,
            format,
        }
    }
    
    pub fn normalized_dimensions(&self) -> (f32, f32) {
        let dist = ((self.texture.width() as f32).powf(2.0)+(self.texture.height() as f32).powf(2.0)).sqrt();
        (self.texture.width() as f32/dist, self.texture.height() as f32/dist)
    }

    pub fn create_storage_layout(format: TextureFormat, device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format, view_dimension: wgpu::TextureViewDimension::D2 },
                count: None,
            }],
            label: None,
        })
    }
}

impl Binding for Texture {
    fn layout(_ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                // This should match the filterable field of the
                // corresponding Texture entry above.
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ]
    }
    
    fn create_resources<'a>(&'a self) -> Vec<Resource> {
        vec![
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.view)),
            Resource::Bespoke(wgpu::BindingResource::Sampler(&self.sampler))
        ]
    }

    fn shader_type() -> ShaderType {
        ShaderType {
            var_types: vec!["".into(), "".into()],
            wgsl_types: vec!["texture_2d<f32>".into(), "sampler".into()],
        }
    }
}

pub struct DepthTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32, label: &str) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            view_formats: &[Self::DEPTH_FORMAT],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        Self { 
            texture, 
            view,
            sampler,
        }
    }
}

impl Binding for DepthTexture {
    fn layout(_ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                // This should match the filterable field of the
                // corresponding Texture entry above.
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ]
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource> {
        vec![
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.view)),
            Resource::Bespoke(wgpu::BindingResource::Sampler(&self.sampler)),
        ]
    }

    fn shader_type() -> ShaderType {
        ShaderType {
            var_types: vec!["".into(), "".into()],
            wgsl_types: vec!["texture_depth_2d".into(), "sampler".into()],
        }
    }
}

pub struct StorageTexture {
    texture: Texture,
}

impl StorageTexture {
    pub fn from_texture(texture: Texture) -> Self {
        Self {
            texture
        }
    }

    pub fn to_texture(self) -> Texture {
        self.texture
    }

    pub fn view(&self) -> &TextureView {
        &self.texture.view
    }
}

impl Binding for StorageTexture {
    fn layout(_ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::ReadWrite, format: wgpu::TextureFormat::Rgba32Float, view_dimension: wgpu::TextureViewDimension::D2 },
                count: None,
            },
        ]
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource> {
        vec![
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.texture.view))
        ]
    }

    fn shader_type() -> ShaderType {
        ShaderType {
            var_types: vec!["".into()],
            wgsl_types: vec!["texture_storage_2d<rgba32float, read_write>".into()],
        }
    }
}

pub struct StorageTexture3D {
    pub texture: Texture,
}

impl StorageTexture3D {
    pub fn from_texture(texture: Texture) -> Self {
        Self {
            texture
        }
    }

    pub fn to_texture(self) -> Texture {
        self.texture
    }
}

impl Binding for StorageTexture3D {
    fn layout(_ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::ReadWrite, format: wgpu::TextureFormat::Rgba32Float, view_dimension: wgpu::TextureViewDimension::D3 },
                count: None,
            },
        ]
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource> {
        vec![
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.texture.view))
        ]
    }

    fn shader_type() -> ShaderType {
        ShaderType {
            var_types: vec!["".into()],
            wgsl_types: vec!["texture_storage_3d<rgba32float, read_write>".into()],
        }
    }
}

pub struct Texture3D {
    pub texture: Texture,
}

impl Binding for Texture3D {
    fn layout(_ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D3,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                // This should match the filterable field of the
                // corresponding Texture entry above.
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
        ]
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource> {
        vec![
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.texture.view)),
            Resource::Bespoke(wgpu::BindingResource::Sampler(&self.texture.sampler))
        ]
    }

    fn shader_type() -> ShaderType {
        ShaderType {
            var_types: vec!["".into(); 2],
            wgsl_types: vec!["texture_3d<f32>".into(), "sampler".into()],
        }
    }
}