//! Please consider using SVG and the SVG widget as it scales much better.

use druid::{kurbo::Rect, piet::{Image as _, ImageBuf, InterpolationMode, PietImage}, widget::prelude::*, Data, MouseEvent, Point, Cursor, Screen, Monitor};
use druid::piet::ImageFormat;
use druid::widget::FillStrat;
use tracing::{instrument, trace};
use image::{RgbaImage};
use image::imageops::{FilterType, resize};

const DISTANCE_MARGIN:f64 = 10.0;
const BORDER_WIDTH:f64 = 5.0;

#[derive(Copy, Clone, PartialEq)]
enum IfMousePressedWhere {
    North,
    NorthEst,
    Est,
    SouthEst,
    South,
    SouthWest,
    West,
    NorthWest,
    Inside(Point),
    NotInterested
}

/// A widget that renders a bitmap Image.
///
/// Contains data about how to fill the given space and interpolate pixels.
/// Configuration options are provided via the builder pattern.
///
/// Note: when [scaling a bitmap image], such as supporting multiple
/// screen sizes and resolutions, interpolation can lead to blurry
/// or pixelated images and so is not recommended for things like icons.
/// Instead consider using [SVG files] and enabling the `svg` feature with `cargo`.
///
/// (See also: [`ImageBuf`], [`FillStrat`], [`InterpolationMode`])
///
/// # Example
///
/// Create an image widget and configure it using builder methods
/// ```
/// use druid::{
///     widget::{Image, FillStrat},
///     piet::{ImageBuf, InterpolationMode},
/// };
///
/// let image_data = ImageBuf::empty();
/// let image_widget = Image::new(image_data)
///     // set the fill strategy
///     .fill_mode(FillStrat::Fill)
///     // set the interpolation mode
///     .interpolation_mode(InterpolationMode::Bilinear);
/// ```
/// Create an image widget and configure it using setters
/// ```
/// use druid::{
///     widget::{Image, FillStrat},
///     piet::{ImageBuf, InterpolationMode},
/// };
///
/// let image_data = ImageBuf::empty();
/// let mut image_widget = Image::new(image_data);
/// // set the fill strategy
/// image_widget.set_fill_mode(FillStrat::FitWidth);
/// // set the interpolation mode
/// image_widget.set_interpolation_mode(InterpolationMode::Bilinear);
/// ```
///
/// [scaling a bitmap image]: crate::Scale#pixels-and-display-points
/// [SVG files]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
pub struct OverImage {
    image_data: ImageBuf,
    paint_data: Option<PietImage>,
    fill: FillStrat,
    interpolation: InterpolationMode,
    clip_area: Option<Rect>,
    mouse: IfMousePressedWhere,
}

#[allow(dead_code)]
impl OverImage {
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
        OverImage {
            image_data,
            paint_data: None,
            fill: FillStrat::default(),
            interpolation: InterpolationMode::Bilinear,
            clip_area: None,
            mouse: IfMousePressedWhere::NotInterested,
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

    fn where_mouse_is(self: &mut Self, me:&MouseEvent) -> IfMousePressedWhere {
        let pos = me.pos.clone();
        let x0 = 0.;
        let x1 = self.image_size().width;
        let y0 = 0.;
        let y1 = self.image_size().height;
        return if f64::abs(pos.x.clone() - x0) < DISTANCE_MARGIN {
            if f64::abs(pos.y.clone() - y0) < DISTANCE_MARGIN {
                IfMousePressedWhere::NorthWest
            } else if f64::abs(pos.y.clone() - y1) < DISTANCE_MARGIN {
                IfMousePressedWhere::SouthWest
            } else {
                IfMousePressedWhere::West
            }
        } else if f64::abs(pos.x.clone() - x1) < DISTANCE_MARGIN {
            if f64::abs(pos.y.clone() - y0) < DISTANCE_MARGIN {
                IfMousePressedWhere::NorthEst
            } else if f64::abs(pos.y.clone() - y1) < DISTANCE_MARGIN {
                IfMousePressedWhere::SouthEst
            } else {
                IfMousePressedWhere::Est
            }
        } else if f64::abs(pos.y.clone() - y0) < DISTANCE_MARGIN {
            IfMousePressedWhere::North
        } else if f64::abs(pos.y.clone() - y1) < DISTANCE_MARGIN {
            IfMousePressedWhere::South
        } else if pos.y.clone() > y0 && pos.y.clone() < y1 && pos.x.clone() > x0 && pos.x.clone() < x1 {
            IfMousePressedWhere::Inside(pos)
        } else {
            IfMousePressedWhere::NotInterested
        }
    }
}

impl<T: Data> Widget<T> for OverImage {
    #[instrument(name = "Image", level = "trace", skip(self, ctx, event, _data, _env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        let mut image_size = self.image_size().to_rect();
        match event {
            Event::MouseDown(me) => {
                ctx.set_active(true);
                self.mouse = self.where_mouse_is(me);
            }
            Event::MouseMove(me) => {
                if self.mouse != IfMousePressedWhere::NotInterested{ //if the mouse has been pressed
                    let pos = me.pos.clone();
                    match self.mouse {
                        IfMousePressedWhere::Est => {
                            image_size.x1 = pos.x;
                        }
                        IfMousePressedWhere::SouthEst => {
                            image_size.y1 = pos.y;
                            image_size.x1 = pos.x;
                        }
                        IfMousePressedWhere::South => {
                            image_size.y1 = pos.y;
                        }
                        _ => {}
                    }
                } else { //the mouse has not been pressed
                    match self.where_mouse_is(me) {
                        IfMousePressedWhere::Est => {
                            ctx.override_cursor(&Cursor::ResizeLeftRight);
                        }
                        IfMousePressedWhere::SouthEst => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        IfMousePressedWhere::South => {
                            ctx.override_cursor(&Cursor::ResizeUpDown);
                        }
                        _ => ctx.clear_cursor()
                    }
                }
            }
            Event::MouseUp(_)=>{
                self.mouse = IfMousePressedWhere::NotInterested;
                ctx.set_active(false);
            }
            _ => (),
        }
        //Keeps validity

        while image_size.x1.clone()<=image_size.x0.clone()+BORDER_WIDTH{
            image_size.x1 += 1.+BORDER_WIDTH;
            image_size.x0 -= 1.+BORDER_WIDTH;
        }
        while image_size.y1.clone()<=image_size.y0.clone()+BORDER_WIDTH{
            image_size.y1 += 1.+BORDER_WIDTH;
            image_size.y0 -= 1.+BORDER_WIDTH;
        }
        //Validity check: inside the monitor size
        let primary_monitor_rect = Screen::get_monitors()
            .into_iter().filter(|m|m.is_primary()).collect::<Vec<Monitor>>()
            .first().expect("No primary monitor found!")
            .virtual_rect();
        if image_size.x0< primary_monitor_rect.x0{
            image_size.x0 = primary_monitor_rect.x0;
        }
        if image_size.y0< primary_monitor_rect.y0{
            image_size.y0 = primary_monitor_rect.y0;
        }
        if image_size.x1> primary_monitor_rect.x1{
            image_size.x1 = primary_monitor_rect.x1-BORDER_WIDTH;
        }
        if image_size.y1> primary_monitor_rect.y1{
            image_size.y1 = primary_monitor_rect.y1-BORDER_WIDTH;
        }
        let img = RgbaImage::from_raw(self.image_data.width() as u32, self.image_data.height() as u32, self.image_data.raw_pixels().to_vec())
            .expect("Can't convert the image from Druid to Image crate");
        let resized_img = resize(&img, image_size.width() as u32, image_size.height() as u32, FilterType::Nearest);
        let width = resized_img.width() as usize; let height = resized_img.height() as usize;
        self.image_data = ImageBuf::from_raw(resized_img.into_raw(), ImageFormat::RgbaSeparate, width, height);
        println!("{}",self.image_data.width());
        ctx.request_paint();
        ctx.window().invalidate(/*Rect::new(ctx.window_origin().x,ctx.window_origin().y,ctx.window_origin().x+image_size.x1,ctx.window_origin().y+image_size.y1)*/);
        //*data=self.image_size;
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
            let ratio = max.width.clone() / image_size.width;
            Size::new(max.width.clone(), ratio * image_size.height)
        } else if bc.is_height_bounded() && !bc.is_width_bounded() {
            let ratio = max.height / image_size.height;
            Size::new(ratio * image_size.width, max.height.clone())
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
            let clip_rect = self.image_size().to_rect();
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