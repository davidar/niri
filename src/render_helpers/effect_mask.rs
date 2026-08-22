//! Masking of background effects by the alpha of the surface they are drawn under.
//!
//! Clients that draw their own rounded corners (or any other non-rectangular shape) leave those
//! pixels fully transparent, yet they request the effect over a rectangular region, which leaves
//! the effect visible around the shape. Sampling the surface's alpha and using it as a coverage
//! mask limits the effect to the pixels that the surface actually covers.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use glam::{Affine2, Mat3, Vec2};
use smithay::backend::drm::DrmDeviceFd;
use smithay::backend::renderer::gles::{ffi, GlesError, GlesFrame, GlesRenderer, GlesTexture, Uniform};
use smithay::backend::renderer::multigpu::gbm::GbmGlesBackend;
use smithay::backend::renderer::multigpu::MultiTexture;
use smithay::backend::renderer::utils::RendererSurfaceStateUserData;
use smithay::backend::renderer::{buffer_has_alpha, ContextId, Texture as _};
use smithay::utils::{Logical, Rectangle, Transform};
use smithay::wayland::compositor::SurfaceData;

use crate::render_helpers::shaders::mat3_uniform;

/// Texture unit that the mask texture is bound to.
///
/// Smithay binds the texture it draws to unit 0 and leaves that unit active, so any other unit is
/// free for us to use.
const MASK_TEXTURE_UNIT: i32 = 1;

/// A surface texture used to mask a background effect.
#[derive(Debug, Clone)]
pub struct EffectMask {
    /// Texture whose alpha channel is the mask.
    pub texture: GlesTexture,
    /// Rectangle that the mask covers, in the same coordinate space as `RenderParams::geometry`.
    pub rect: Rectangle<f64, Logical>,
    /// Maps [0, 1] coordinates within `rect` to texture coordinates.
    pub rect_to_tex: Mat3,
    /// Alpha at which the mask reaches full coverage.
    ///
    /// A translucent surface still covers its pixels completely, it just lets some light through,
    /// so the effect below it must be drawn at full strength; only fully transparent pixels get
    /// no effect. A nonzero threshold keeps client-side antialiased edges from turning into a
    /// staircase. Configured per rule as `mask-threshold`.
    pub full_coverage_alpha: f32,
}

impl EffectMask {
    /// Builds a mask from a surface's current buffer.
    ///
    /// `rect` is where the surface is drawn, in the coordinate space of the effect geometry, and
    /// `full_coverage_alpha` is the alpha at which the mask saturates. Returns `None` if the
    /// surface has no texture in this renderer, which happens before its first commit is
    /// imported.
    pub fn for_surface(
        states: &SurfaceData,
        context_id: &ContextId<GlesTexture>,
        rect: Rectangle<f64, Logical>,
        full_coverage_alpha: f64,
    ) -> Option<Self> {
        match Self::try_for_surface(states, context_id, rect, full_coverage_alpha) {
            Ok(mask) => Some(mask),
            Err(reason) => {
                log_outcome(states, &reason);
                None
            }
        }
    }

    fn try_for_surface(
        states: &SurfaceData,
        context_id: &ContextId<GlesTexture>,
        rect: Rectangle<f64, Logical>,
        full_coverage_alpha: f64,
    ) -> Result<Self, String> {
        if rect.size.w <= 0. || rect.size.h <= 0. {
            return Err("off: empty effect rect".into());
        }

        let data = states
            .data_map
            .get::<RendererSurfaceStateUserData>()
            .ok_or("off: no renderer state").map_err(String::from)?;
        let data = data.lock().unwrap();

        let view = data.view().ok_or("off: no surface view")?;
        let who = format!("surface {}x{}", view.dst.w, view.dst.h);

        // Buffers without an alpha channel carry undefined bytes where the alpha would be, so
        // there is nothing to mask with. A declared opaque region is deliberately not trusted:
        // clients that draw rounded corners into an alpha buffer still declare the whole surface
        // opaque (libcosmic does, through winit's transparency hint), and the real alpha is right
        // there to sample.
        if data
            .buffer()
            .is_some_and(|buffer| buffer_has_alpha(buffer) == Some(false))
        {
            return Err(format!("off ({who}): buffer format has no alpha channel"));
        }

        // The TTY backend renders through Smithay's multi-GPU renderer, whose context id is the
        // render device's GLES context id retyped; surfaces it imported are stored under the same
        // key as a `MultiTexture` wrapping the GLES texture of each device.
        let texture = match data.texture::<GlesTexture>(context_id.clone()) {
            Some(texture) => texture.clone(),
            None => data
                .texture::<MultiTexture>(context_id.clone().map())
                .and_then(|texture| {
                    texture.get::<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>(context_id)
                })
                .ok_or_else(|| format!("off ({who}): no texture in this renderer"))?,
        };
        let buffer_size = data
            .buffer_size()
            .ok_or_else(|| format!("off ({who}): no buffer size"))?;
        let transform = data.buffer_transform();

        // Same mapping that Smithay computes for drawing the surface itself, but starting from
        // coordinates normalized to `rect` rather than from destination pixels. Note that, like
        // Smithay's own surface drawing, this ignores the buffer's y-inverted flag, so that the
        // mask always lines up with the surface as it is drawn.
        let src = view.src.to_buffer(
            f64::from(data.buffer_scale()),
            transform,
            &buffer_size.to_f64(),
        );
        let src_size = transform.transform_size(src.size);
        if src_size.w <= 0. || src_size.h <= 0. {
            return Err(format!("off ({who}): empty source rect"));
        }

        let src_size = Vec2::new(src_size.w as f32, src_size.h as f32);
        let translation = match transform {
            Transform::Normal | Transform::Flipped90 => Affine2::IDENTITY,
            Transform::_90 => Affine2::from_translation(Vec2::new(0., src_size.x)),
            Transform::_180 => Affine2::from_translation(src_size),
            Transform::_270 => Affine2::from_translation(Vec2::new(src_size.y, 0.)),
            Transform::Flipped => Affine2::from_translation(Vec2::new(src_size.x, 0.)),
            Transform::Flipped180 => Affine2::from_translation(Vec2::new(0., src_size.y)),
            Transform::Flipped270 => Affine2::from_translation(Vec2::new(src_size.y, src_size.x)),
        };

        let tex_size = texture.size();
        log_outcome(
            states,
            &format!(
                "on ({who}): rect {:?} tex {}x{} buffer {:?} src {:?} buffer-scale {} transform {:?}",
                rect.size, tex_size.w, tex_size.h, buffer_size, src, data.buffer_scale(), transform
            ),
        );
        let rect_to_tex = Affine2::from_scale(Vec2::new(
            1. / tex_size.w as f32,
            1. / tex_size.h as f32,
        )) * Affine2::from_translation(Vec2::new(src.loc.x as f32, src.loc.y as f32))
            * translation
            * transform.matrix()
            * Affine2::from_scale(src_size);

        Ok(Self {
            texture,
            rect,
            rect_to_tex: rect_to_tex.into(),
            full_coverage_alpha: full_coverage_alpha as f32,
        })
    }

    /// Uniforms for masking with `input_to_rect` mapping shader input coords to [0, 1] in `rect`.
    pub fn uniforms(&self, input_to_rect: Mat3) -> [Uniform<'static>; 3] {
        [
            Uniform::new("niri_mask_tex", MASK_TEXTURE_UNIT),
            mat3_uniform("niri_input_to_mask", self.rect_to_tex * input_to_rect),
            // A zero threshold means any nonzero alpha is full coverage; keep the reciprocal
            // finite so the shader never multiplies zero by infinity.
            Uniform::new("niri_mask", 1. / self.full_coverage_alpha.max(f32::EPSILON)),
        ]
    }

    /// Binds the mask texture for the next draw.
    ///
    /// Returns `false` if the texture cannot be sampled as a `sampler2D`, which is the case for
    /// dma-bufs imported as external textures. The caller must then draw unmasked.
    pub fn bind(&self, frame: &mut GlesFrame<'_, '_>) -> Result<bool, GlesError> {
        frame.with_context(|gl| unsafe {
            while gl.GetError() != ffi::NO_ERROR {}

            gl.ActiveTexture(ffi::TEXTURE0 + MASK_TEXTURE_UNIT as u32);
            gl.BindTexture(ffi::TEXTURE_2D, self.texture.tex_id());

            // Binding a texture that was created with a different target is the only way to find
            // out that it is external, since Smithay doesn't expose that.
            let bound = gl.GetError() == ffi::NO_ERROR;
            if bound {
                // The texture may never have been sampled before, in which case its defaults make
                // it incomplete for non-power-of-two sizes. These are the same parameters that
                // Smithay sets when it draws the surface.
                gl.TexParameteri(ffi::TEXTURE_2D, ffi::TEXTURE_MIN_FILTER, ffi::LINEAR as i32);
                gl.TexParameteri(ffi::TEXTURE_2D, ffi::TEXTURE_MAG_FILTER, ffi::LINEAR as i32);
                gl.TexParameteri(
                    ffi::TEXTURE_2D,
                    ffi::TEXTURE_WRAP_S,
                    ffi::CLAMP_TO_EDGE as i32,
                );
                gl.TexParameteri(
                    ffi::TEXTURE_2D,
                    ffi::TEXTURE_WRAP_T,
                    ffi::CLAMP_TO_EDGE as i32,
                );
            } else {
                gl.BindTexture(ffi::TEXTURE_2D, 0);
            }

            gl.ActiveTexture(ffi::TEXTURE0);
            bound
        })
    }

    /// Unbinds the mask texture bound by [`EffectMask::bind`].
    pub fn unbind(frame: &mut GlesFrame<'_, '_>) -> Result<(), GlesError> {
        frame.with_context(|gl| unsafe {
            gl.ActiveTexture(ffi::TEXTURE0 + MASK_TEXTURE_UNIT as u32);
            gl.BindTexture(ffi::TEXTURE_2D, 0);
            gl.ActiveTexture(ffi::TEXTURE0);
        })
    }
}

/// Last logged mask outcome for a surface, so changes are logged once rather than per frame.
struct MaskDebug(Mutex<String>);

fn log_outcome(states: &SurfaceData, outcome: &str) {
    let dbg = states
        .data_map
        .get_or_insert(|| MaskDebug(Mutex::new(String::new())));
    let mut last = dbg.0.lock().unwrap();
    if *last != outcome {
        debug!("effect mask {outcome}");
        *last = outcome.to_owned();
    }
}

/// Logs, once per process, that a mask texture could not be bound and the effect drew unmasked.
pub fn log_unbindable() {
    static LOGGED: AtomicBool = AtomicBool::new(false);
    if !LOGGED.swap(true, Ordering::Relaxed) {
        warn!("effect mask off: surface texture is external (dma-buf); drawing unmasked");
    }
}

/// Uniforms that disable masking.
///
/// Uniforms keep their value across draws with the same program, so every draw must set these.
pub fn disabled_uniforms() -> [Uniform<'static>; 3] {
    [
        Uniform::new("niri_mask_tex", MASK_TEXTURE_UNIT),
        mat3_uniform("niri_input_to_mask", Mat3::IDENTITY),
        Uniform::new("niri_mask", 0f32),
    ]
}
