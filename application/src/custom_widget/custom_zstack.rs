use std::sync::Arc;
use druid::{BoxConstraints, Data, Env, Event, EventCtx, InternalEvent, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, Size, UnitPoint, UpdateCtx, Vec2, Widget, WidgetExt, WidgetPod, ImageBuf};
use druid::piet::ImageFormat;
use druid::widget::{Image, SizedBox};
use image::{DynamicImage, GenericImage, GenericImageView, Pixel};
use image::imageops::FilterType;
use image::io::Reader;
use crate::{SAVE_OVER_IMG, SHOW_OVER_IMG};

/// A container that stacks its children on top of each other.
///
/// The container has a baselayer which has the lowest z-index and determines the size of the
/// container.
pub struct CustomZStack<T> {
    layers: Vec<ZChild<T>>,
    over_img: Option<DynamicImage>,
    back_img: Option<DynamicImage>,
}

struct ZChild<T> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    relative_size: Vec2,
    absolute_size: Vec2,
    position: UnitPoint,
    offset: Vec2,
}

impl <T: Data> CustomZStack<T>  {
    /// Creates a new ZStack with a background Image.
    ///
    /// The Image is used by the ZStack to determine its own size.
    pub fn new(base_layer: impl Widget<T> + 'static) -> Self {
        Self {
            layers: vec![ZChild {
                child: WidgetPod::new(base_layer.boxed()),
                relative_size: Vec2::new(1.0, 1.0),
                absolute_size: Vec2::ZERO,
                position: UnitPoint::CENTER,
                offset: Vec2::ZERO,
            }],
            over_img: None,
            back_img: None,
        }
    }

    /// Builder-style method to add a new child to the Z-Stack.
    ///
    /// The child is added directly above the base layer.
    ///
    /// `relative_size` is the space the child is allowed to take up relative to its parent. The
    ///                 values are between 0 and 1.
    /// `absolute_size` is a fixed amount of pixels added to `relative_size`.
    ///
    /// `position`      is the alignment of the child inside the remaining space of its parent.
    ///
    /// `offset`        is a fixed amount of pixels added to `position`.
    fn with_child(
        self: &mut Self,
        child: impl Widget<T> + 'static,
        relative_size: Vec2,
        absolute_size: Vec2,
        position: UnitPoint,
        offset: Vec2,
    ) -> &mut Self {
        if self.layers.len() as i32 - 1 < 0 {
            self.layers = vec![ZChild {
                child: WidgetPod::new(child.boxed()),
                relative_size: Vec2::new(1.0, 1.0),
                absolute_size: Vec2::ZERO,
                position: UnitPoint::CENTER,
                offset: Vec2::ZERO,
            }]
        } else {
            let next_index = self.layers.len() - 1;
            self.layers.insert(
                next_index,
                ZChild {
                    child: WidgetPod::new(child.boxed()),
                    relative_size,
                    absolute_size,
                    position,
                    offset,
                },
            );
        }
        self
    }

    fn rm_child(self: &mut Self)-> ZChild<T>{
        self.layers.remove(0)
    }

    pub fn show_over_img(self: &mut Self, open_path: &'static str) -> bool{
        if self.over_img.is_none() {
            let img = Reader::open(open_path).unwrap().decode().unwrap();
            let over_image = SizedBox::new(Image::new(ImageBuf::from_raw(
                Arc::<[u8]>::from(img.as_bytes()), ImageFormat::RgbaSeparate, img.width() as usize, img.height() as usize
            ))).expand().border(druid::Color::BLACK,2.);
            self.with_child(over_image, Vec2::new(1., 1.), Vec2::ZERO, UnitPoint::CENTER, Vec2::new(5., 5.))
                .over_img = Some(img);
            true
        } else {
            false
        }
    }
    ///return true if has correctly save, false otherwise
    pub fn save_new_img(self: &mut Self, back_img: &mut DynamicImage, save_path: &'static str, file_name: &str, img_format: image::ImageFormat) ->bool{
        //TODO: add more over-images at once!
        //TODO: find a more efficent way without resizing the back image (maybe using the GPU or without resize)!
        //TODO: if over-image is bigger then back-image crash.
        if self.layers.len() > 1 && self.over_img.is_some(){
            let _back_img_resolution = Size::new(back_img.width() as f64, back_img.height() as f64);

            let mut back_img_rect: Rect= self.layers.get(1).unwrap().child.layout_rect();
            back_img_rect.x0 = back_img_rect.x0.round();
            back_img_rect.y0 = back_img_rect.y0.round();
            back_img_rect.x1 = back_img_rect.x1.round();
            back_img_rect.y1 = back_img_rect.y1.round();
            let back_img = back_img.resize(back_img_rect.width() as u32, back_img_rect.height() as u32, FilterType::Lanczos3);

            let mut over_img_rect: Rect = self.layers.get(0).unwrap().child.layout_rect();
            over_img_rect.x0 = over_img_rect.x0.round();
            over_img_rect.y0 = over_img_rect.y0.round();
            over_img_rect.x1 = over_img_rect.x1.round();
            over_img_rect.y1 = over_img_rect.y1.round();
            let over_img = self.over_img.as_mut().unwrap().resize(over_img_rect.width() as u32, over_img_rect.height() as u32, FilterType::Lanczos3);

            let mut out = DynamicImage::new_rgba8(back_img.width(),back_img.height());

            let mut over_i=0;
            let mut over_j=0;
            for back_j in back_img_rect.y0 as u32..back_img_rect.y1 as u32{
                for back_i in back_img_rect.x0 as u32..back_img_rect.x1 as u32{
                    if over_img_rect.contains(Point::new(back_i as f64, back_j as f64)) {
                        if over_img.get_pixel(over_i, over_j).channels()[3] > 0 {
                            out.put_pixel(back_i, back_j, over_img.get_pixel(over_i, over_j));
                        } else {
                            out.put_pixel(back_i, back_j, back_img.get_pixel(back_i, back_j));
                        }
                        over_i += 1;
                        if over_i >= over_img.width(){
                            over_i = 0;
                            over_j += 1;
                            assert!(over_j<=over_img.height());
                        }
                    } else {
                        assert!(back_img_rect.contains(Point::new(back_i as f64, back_j as f64)));
                        out.put_pixel(back_i, back_j, back_img.get_pixel(back_i, back_j));
                    }
                }
            }

            //let out = out.resize(back_img_resolution.width as u32, back_img_resolution.height as u32, FilterType::Lanczos3);

            while self.layers.len() > 1 {
                self.rm_child();
            }
            let old = self.rm_child();
            self.with_child(Image::new(ImageBuf::from_raw(
                Arc::<[u8]>::from(out.as_bytes()), ImageFormat::RgbaSeparate, out.width() as usize, out.height() as usize
            )), old.relative_size, old.absolute_size, old.position, old.offset);
            out.save_with_format(format!("{}{}.{}",save_path,file_name,img_format.extensions_str().first().unwrap()),img_format).unwrap();
            self.over_img = None;
            self.back_img = Some(out);
            true
        } else {
           panic!("trying to add 2 over-img");
        }
    }
}

impl<T: Data> Widget<T> for CustomZStack<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) => {
                if cmd.is(SHOW_OVER_IMG) {
                    let path = cmd.get_unchecked(SHOW_OVER_IMG);
                    self.show_over_img(*path);
                } else if cmd.is(SAVE_OVER_IMG){
                    let (mut back_img,path,file_name,file_format) = cmd.get_unchecked(SAVE_OVER_IMG).clone();
                    if self.back_img.is_none() {
                        self.save_new_img(&mut back_img, path, file_name, file_format);
                    } else {
                        self.save_new_img(&mut self.back_img.clone().unwrap(), path, file_name, file_format);
                    }
                }
                ctx.children_changed();
                ctx.request_paint();
            }
            _ => {
                let mut previous_hot = false;
                for layer in self.layers.iter_mut() {
                    if event.is_pointer_event() && previous_hot.clone() {
                        if layer.child.is_active() {
                            ctx.set_handled();
                            layer.child.event(ctx, event, data, env);
                        } else {
                            layer
                                .child
                                .event(ctx, &Event::Internal(InternalEvent::MouseLeave), data, env);
                        }
                    } else {
                        layer.child.event(ctx, event, data, env);
                    }

                    previous_hot |= layer.child.is_hot();
                }
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let mut previous_hot = false;
        for layer in self.layers.iter_mut() {
            let inner_event = event.ignore_hot(previous_hot.clone());
            layer.child.lifecycle(ctx, &inner_event, data, env);
            previous_hot |= layer.child.is_hot();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {

        for layer in self.layers.iter_mut().rev() {
            layer.child.update(ctx, data, env);
        }
        ctx.request_paint();
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        //Layout base layer

        let base_layer = self.layers.last_mut().unwrap();
        let base_size = base_layer.child.layout(ctx, bc, data, env);

        //Layout other layers
        let other_layers = self.layers.len() - 1;

        for layer in self.layers.iter_mut().take(other_layers) {
            let max_size = layer.resolve_max_size(base_size);
            layer
                .child
                .layout(ctx, &BoxConstraints::new(Size::ZERO, max_size), data, env);
        }

        //Set origin for all Layers and calculate paint insets
        let mut paint_rect = Rect::ZERO;

        for layer in self.layers.iter_mut() {
            let remaining = base_size - layer.child.layout_rect().size();
            let origin = layer.resolve_point(remaining);
            layer.child.set_origin(ctx, origin);

            paint_rect = paint_rect.union(layer.child.paint_rect());
        }

        ctx.set_paint_insets(paint_rect - base_size.to_rect());
        ctx.set_baseline_offset(self.layers.last().unwrap().child.baseline_offset());

        base_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {

        //Painters algorithm (Painting back to front)
        for layer in self.layers.iter_mut().rev() {
            layer.child.paint(ctx, data, env);
        }
    }
}

impl<T: Data> ZChild<T> {
    fn resolve_max_size(&self, availible: Size) -> Size {
        self.absolute_size.to_size()
            + Size::new(
            availible.width * self.relative_size.x.clone(),
            availible.height * self.relative_size.y.clone(),
        )
    }

    fn resolve_point(&self, remaining_space: Size) -> Point {
        (self.position.resolve(remaining_space.to_rect()).to_vec2() + self.offset).to_point()
    }
}