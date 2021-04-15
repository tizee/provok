use crate::font::FontConfiguration;
use crate::glyphcache::GlyphCache;
use crate::input::TextStyle;
use crate::quad::*;
use crate::utils::{IntPixelLength, PixelLength, Size};
use crate::utilsprites::UtilSprites;
use failure::Fallible;
use glium::texture::SrgbTexture2d;
use glium::{Display, IndexBuffer, VertexBuffer};

const ATLAS_SIZE: usize = 8192;

static VERTEX_SHADER: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/shaders/vertex.glsl"));

static FRAGMENT_SHADER: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/shaders/fragment.glsl"));

pub struct RenderMetrics {
    pub descender: PixelLength,
    pub descender_row: IntPixelLength,
    pub descender_plus_two: IntPixelLength,
    pub underline_height: IntPixelLength,
    pub strike_row: IntPixelLength,
    pub win_size: Size,
    pub cell_size: Size,
}

impl RenderMetrics {
    pub fn new(
        font_config: &FontConfiguration,
        style: &TextStyle,
        width: f32,
        height: f32,
    ) -> Self {
        let win_size = Size::new(width as isize, height as isize);
        let metrics = font_config.font_metrics(style).expect("failed to get font metrics!?");

        let (cell_height, cell_width) =
            (metrics.cell_height.get().ceil() as usize, metrics.cell_width.get().ceil() as usize);
        let cell_size = Size::new(cell_width as isize, cell_height as isize);
        let underline_height = metrics.underline_thickness.get().round() as isize;

        let descender_row =
            (cell_height as f64 + (metrics.descender - metrics.underline_position).get()) as isize;
        let descender_plus_two =
            (2 * underline_height + descender_row).min(cell_height as isize - 1);
        let strike_row = descender_row / 2;
        Self {
            descender: metrics.descender,
            descender_row,
            descender_plus_two,
            strike_row,
            underline_height,
            win_size,
            cell_size,
        }
    }
}

pub fn compile_shaders(display: &Display) -> Fallible<glium::Program> {
    let glyph_source = glium::program::ProgramCreationInput::SourceCode {
        vertex_shader: VERTEX_SHADER,
        fragment_shader: FRAGMENT_SHADER,
        outputs_srgb: true,
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        transform_feedback_varyings: None,
        uses_point_size: false,
        geometry_shader: None,
    };
    let program = glium::Program::new(display, glyph_source)?;
    Ok(program)
}

pub struct RenderState {
    pub glyph_cache: GlyphCache<SrgbTexture2d>,
    pub util_sprites: UtilSprites<SrgbTexture2d>,
    pub glyph_vertex_buffer: VertexBuffer<Vertex>,
    pub glyph_index_buffer: IndexBuffer<u32>,
}

impl RenderState {
    pub fn new(
        display: &Display,
        render_metrics: &RenderMetrics,
        num_cells: &usize,
    ) -> Fallible<Self> {
        let mut glyph_cache = GlyphCache::new(&display, ATLAS_SIZE)?;
        let util_sprites = UtilSprites::new(&mut glyph_cache, render_metrics)?;
        let (glyph_vertex_buffer, glyph_index_buffer) =
            Self::compute_glyph_vertices(&render_metrics, *num_cells, display)?;
        Ok(Self { glyph_cache, util_sprites, glyph_vertex_buffer, glyph_index_buffer })
    }

    pub fn compute_glyph_vertices(
        render_metrics: &RenderMetrics,
        num_cells: usize,
        display: &Display,
    ) -> Fallible<(VertexBuffer<Vertex>, IndexBuffer<u32>)> {
        let cell_width = render_metrics.cell_size.width as f32;
        let cell_height = render_metrics.cell_size.height as f32;
        let mut verts = Vec::new();
        let mut indices = Vec::new();

        let y_pos = cell_height / -2.0;
        for x in 0..num_cells {
            let x_pos = (num_cells as f32 * cell_width / -2.0) + (x as f32 * cell_width);

            let idx = verts.len() as u32;
            verts.push(Vertex { position: (x_pos, y_pos), ..Default::default() });
            verts.push(Vertex { position: (x_pos + cell_width, y_pos), ..Default::default() });
            verts.push(Vertex { position: (x_pos, y_pos + cell_height), ..Default::default() });
            verts.push(Vertex {
                position: (x_pos + cell_width, y_pos + cell_height),
                ..Default::default()
            });

            indices.push(idx + V_TOP_LEFT as u32);
            indices.push(idx + V_TOP_RIGHT as u32);
            indices.push(idx + V_BOT_LEFT as u32);

            indices.push(idx + V_TOP_RIGHT as u32);
            indices.push(idx + V_BOT_LEFT as u32);
            indices.push(idx + V_BOT_RIGHT as u32);
        }

        Ok((
            VertexBuffer::dynamic(display, &verts)?,
            IndexBuffer::new(display, glium::index::PrimitiveType::TrianglesList, &indices)?,
        ))
    }
}
