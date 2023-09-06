use std::sync::Arc;
use druid::{kurbo::Rect, piet::{Image as _, ImageBuf, InterpolationMode, PietImage}, widget::prelude::*, Data, Selector};
use druid::piet::ImageFormat;
use druid::widget::FillStrat;
use image::DynamicImage;
use tracing::{instrument, trace};

pub const UPDATE_SCREENSHOT: Selector<Arc<DynamicImage>> = Selector::new("Update the screenshot image");

pub struct ScreenshotImage {
    image_data: ImageBuf,
    paint_data: Option<PietImage>,
    fill: FillStrat,
    interpolation: InterpolationMode,
    clip_area: Option<Rect>,
}

#[allow(dead_code)]
impl ScreenshotImage {
    /// Create an image drawing widget from an image buffer.
    ///
    /// By default, the Image will scale to fit its box constraints ([`FillStrat::Fill`])
    /// and will be scaled bilinearly ([`InterpolationMode::Bilinear`])
    ///
    /// The underlying `ImageBuf` uses `Arc` for buffer data, making it cheap to clone.
    ///
    /// [`FillStrat::Fill`]: crate::widget::FillStrat::Fill
    /// [`InterpolationMode::Bilinear`]: crate::piet::InterpolationMode::Bilinear
    #[inline]
    pub fn new(image_data: ImageBuf) -> Self {
        ScreenshotImage {
            image_data,
            paint_data: None,
            fill: FillStrat::default(),
            interpolation: InterpolationMode::Bilinear,
            clip_area: None,
        }
    }

    /// Builder-style method for specifying the fill strategy.
    #[inline]
    pub fn fill_mode(mut self, mode: FillStrat) -> Self {
        self.fill = mode;
        // Invalidation not necessary
        self
    }

    /// Modify the widget's fill strategy.
    #[inline]
    pub fn set_fill_mode(&mut self, newfil: FillStrat) {
        self.fill = newfil;
        // Invalidation not necessary
    }

    /// Builder-style method for specifying the interpolation strategy.
    #[inline]
    pub fn interpolation_mode(mut self, interpolation: InterpolationMode) -> Self {
        self.interpolation = interpolation;
        // Invalidation not necessary
        self
    }

    /// Modify the widget's interpolation mode.
    #[inline]
    pub fn set_interpolation_mode(&mut self, interpolation: InterpolationMode) {
        self.interpolation = interpolation;
        // Invalidation not necessary
    }

    /// Builder-style method for setting the area of the image that will be displayed.
    ///
    /// If `None`, then the whole image will be displayed.
    #[inline]
    pub fn clip_area(mut self, clip_area: Option<Rect>) -> Self {
        self.clip_area = clip_area;
        // Invalidation not necessary
        self
    }

    /// Set the area of the image that will be displayed.
    ///
    /// If `None`, then the whole image will be displayed.
    #[inline]
    pub fn set_clip_area(&mut self, clip_area: Option<Rect>) {
        self.clip_area = clip_area;
        // Invalidation not necessary
    }

    /// Set new `ImageBuf`.
    #[inline]
    pub fn set_image_data(&mut self, image_data: ImageBuf) {
        self.image_data = image_data;
        self.invalidate();
    }

    /// Invalidate the image cache, forcing it to be recreated.
    #[inline]
    fn invalidate(&mut self) {
        self.paint_data = None;
    }

    /// The size of the effective image, considering clipping if it's in effect.
    #[inline]
    fn image_size(&mut self) -> Size {
        self.clip_area
            .map(|a| a.size())
            .unwrap_or_else(|| self.image_data.size())
    }
}

impl<T: Data> Widget<T> for ScreenshotImage {
    #[instrument(name = "Image", level = "trace", skip(self, ctx, event, _data, _env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        match event {
            Event::Command(cmd) => {
                if cmd.is(UPDATE_SCREENSHOT) {
                    let screen_img = cmd.get_unchecked(UPDATE_SCREENSHOT);
                    self.set_image_data(ImageBuf::from_raw(
                        Arc::<[u8]>::from((**screen_img).clone().as_bytes()),
                        ImageFormat::RgbaSeparate,
                        screen_img.width() as usize,
                        screen_img.height() as usize,
                    ));
                    ctx.request_layout();
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    #[instrument(name = "Image", level = "trace", skip(self, _ctx, _event, _data, _env))]
    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &T, _env: &Env) {}

    #[instrument(
    name = "Image",
    level = "trace",
    skip(self, _ctx, _old_data, _data, _env)
    )]
    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    #[instrument(
    name = "Image",
    level = "trace",
    skip(self, _layout_ctx, bc, _data, _env)
    )]
    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.debug_check("Image");

        // If either the width or height is constrained calculate a value so that the image fits
        // in the size exactly. If it is unconstrained by both width and height take the size of
        // the image.
        let max = bc.max();
        let image_size = self.image_size();
        let size = if bc.is_width_bounded() && !bc.is_height_bounded() {
            let ratio = max.width / image_size.width;
            Size::new(max.width, ratio * image_size.height)
        } else if bc.is_height_bounded() && !bc.is_width_bounded() {
            let ratio = max.height / image_size.height;
            Size::new(ratio * image_size.width, max.height)
        } else {
            bc.constrain(image_size)
        };
        trace!("Computed size: {}", size);
        size
    }

    #[instrument(name = "Image", level = "trace", skip(self, ctx, _data, _env))]
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, _env: &Env) {
        let image_size = self.image_size();
        let offset_matrix = self.fill.affine_to_fill(ctx.size(), image_size);

        // The ImageData's to_piet function does not clip to the image's size
        // CairoRenderContext is very like druids but with some extra goodies like clip
        if self.fill != FillStrat::Contain {
            let clip_rect = ctx.size().to_rect();
            ctx.clip(clip_rect);
        }

        let piet_image = {
            let image_data = &self.image_data;
            self.paint_data
                .get_or_insert_with(|| image_data.to_image(ctx.render_ctx))
        };
        if piet_image.size().is_empty() {
            // zero-sized image = nothing to draw
            return;
        }
        ctx.with_save(|ctx| {
            // we have to re-do this because the whole struct is moved into the closure.
            let piet_image = {
                let image_data = &self.image_data;
                self.paint_data
                    .get_or_insert_with(|| image_data.to_image(ctx.render_ctx))
            };
            ctx.transform(offset_matrix);
            if let Some(area) = self.clip_area {
                ctx.draw_image_area(piet_image, area, image_size.to_rect(), self.interpolation);
            } else {
                ctx.draw_image(piet_image, image_size.to_rect(), self.interpolation);
            }
        });
    }
}