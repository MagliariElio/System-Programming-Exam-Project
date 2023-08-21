use std::sync::Arc;
use druid::{BoxConstraints, Data, Env, Event, EventCtx, InternalEvent, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, Size, UnitPoint, UpdateCtx, Vec2, Widget, WidgetExt, WidgetPod, ImageBuf, WidgetId, Target, Color};
use druid::kurbo::common::FloatExt;
use druid::piet::ImageFormat;
use image::ImageFormat as imgFormat;
use druid::widget::{Image};
use image::{DynamicImage, GenericImage, GenericImageView, Pixel};
use image::imageops::FilterType;
use image::io::Reader;
use crate::{SAVE_OVER_IMG, SHOW_OVER_IMG, UPDATE_COLOR, UPDATE_SCREENSHOT};
use crate::custom_widget::resizable_box::UPDATE_ORIGIN;
use crate::custom_widget::{ResizableBox};

/// A container that stacks its children on top of each other.
///
/// The container has a baselayer which has the lowest z-index and determines the size of the
/// container.
pub struct CustomZStack<T> {
    layers: Vec<ZChild<T>>,
    over_img: Option<DynamicImage>,
    back_img: Option<DynamicImage>,
    back_img_origin: Option<Point>,
    screenshot_id: WidgetId,
    color:Option<Color>,
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
    pub fn new(base_layer: impl Widget<T> + 'static, screenshot_id: WidgetId) -> Self {
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
            back_img_origin: None,
            screenshot_id,
            color: None,
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

    pub fn show_over_img(self: &mut Self, open_path: &'static str, id: WidgetId, color: Option<Color>){
        if self.over_img.is_none() {
            let mut img = Reader::open(open_path).unwrap().decode().unwrap();
            if color.is_some() {
                let color = color.unwrap().as_rgba8();
                for j in 0..img.height() {
                    for i in 0..img.width() {
                        let mut cur_px = img.get_pixel(i, j);
                        let ch = cur_px.channels_mut();
                        if ch[3] > 0 {
                            ch[0] = color.0;
                            ch[1] = color.1;
                            ch[2] = color.2;
                            img.put_pixel(i,j,cur_px);
                        }
                    }
                }
            }
            let over_image = ResizableBox::new(Image::new(ImageBuf::from_raw(
                Arc::<[u8]>::from(img.as_bytes()), ImageFormat::RgbaSeparate, img.width() as usize, img.height() as usize
            )),id).height(50.).width(50.);
            self.with_child(over_image, Vec2::new(1., 1.), Vec2::ZERO, UnitPoint::CENTER, Vec2::new(5., 5.))
                .over_img = Some(img);
        } else {
            self.rm_child();
            self.over_img = None;
        }
    }

    pub fn save_new_img(self: &mut Self, new_img_path: &String, img_format: imgFormat) {
        if self.layers.len() > 1 && self.over_img.is_some(){
            let back_img = self.back_img.as_mut().unwrap();
            let back_img_resolution = Size::new(back_img.width() as f64, back_img.height() as f64);
            let mut back_img_rect: Rect= self.layers.get(1).unwrap().child.layout_rect();
            let scale_factor_x = (back_img_resolution.width/back_img_rect.x1).expand();
            let scale_factor_y = (back_img_resolution.height/back_img_rect.y1).expand();

            back_img_rect.x0 = (back_img_rect.x0).floor();
            back_img_rect.y0 = (back_img_rect.y0).floor();
            back_img_rect.x1 = (back_img_rect.x1*scale_factor_x).expand();
            back_img_rect.y1 = (back_img_rect.y1*scale_factor_y).expand();
            let back_img = back_img.resize(back_img_rect.width() as u32, back_img_rect.height() as u32, FilterType::Lanczos3);

            let mut over_img_rect: Rect = self.layers.get(0).unwrap().child.layout_rect();
            over_img_rect.x0 = (over_img_rect.x0*scale_factor_x).floor();
            over_img_rect.y0 = (over_img_rect.y0*scale_factor_y).floor();
            over_img_rect.x1 = (over_img_rect.x1*scale_factor_x).expand();
            over_img_rect.y1 = (over_img_rect.y1*scale_factor_y).expand();
            let over_img = self.over_img.as_mut().unwrap().resize_exact(over_img_rect.width() as u32, over_img_rect.height() as u32, FilterType::Lanczos3);

            let mut out = back_img;
            let mut i2: u32 = 0; let mut j2: u32 = 0;
            for j1 in over_img_rect.y0 as u32 .. (over_img_rect.y1) as u32 {
                for i1 in over_img_rect.x0 as u32 .. (over_img_rect.x1) as u32 {
                    if over_img.get_pixel(i2, j2).channels()[3] > 50 {
                        out.put_pixel(i1,j1,over_img.get_pixel(i2, j2));
                    }
                    i2 += 1;
                }
                i2 = 0;
                j2 += 1;
            }

            //let out = out.resize(back_img_resolution.width as u32, back_img_resolution.height as u32, FilterType::Lanczos3);

            while self.layers.len() > 1 {
                self.rm_child();
            }

            out.save_with_format(new_img_path, img_format).unwrap();

            self.over_img = None;
            self.back_img = Some(out);
            self.back_img_origin = None;

        } else {

        }
    }
}

impl<T: Data> Widget<T> for CustomZStack<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) => {
                if cmd.is(SHOW_OVER_IMG) {
                    let path = cmd.get_unchecked(SHOW_OVER_IMG);
                    self.show_over_img(*path, ctx.widget_id(),self.color);
                } else if cmd.is(SAVE_OVER_IMG){

                    let (back_img_path,path,file_name,file_format) = cmd.get_unchecked(SAVE_OVER_IMG);
                    let new_img_path = format!("{}{}.{}", path, file_name, file_format.extensions_str().first().unwrap());
                    if self.back_img.is_none() {
                        let screen_img = Reader::open(back_img_path)
                            .expect("Can't open the screenshot!")
                            .decode()
                            .expect("Can't decode the screenshot");
                        self.back_img = Some(screen_img);
                    }
                    self.save_new_img(&new_img_path, *file_format);

                    ctx.submit_command(
                        UPDATE_SCREENSHOT
                            .with(new_img_path)
                            .to(Target::Widget(self.screenshot_id)));
                } else if cmd.is(UPDATE_ORIGIN){
                    let new_origin = cmd.get_unchecked(UPDATE_ORIGIN);
                    self.back_img_origin = Some(*new_origin);
                } else if cmd.is(UPDATE_COLOR){
                    let color = cmd.get_unchecked(UPDATE_COLOR);
                    self.color = *color;
                    if self.over_img.is_some(){
                        self.rm_child();
                        self.over_img = None;
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

        let len = self.layers.len();
        for (i,layer) in self.layers.iter_mut().enumerate() {
            let remaining = base_size - layer.child.layout_rect().size();
            let mut origin = layer.resolve_point(remaining);
            if self.back_img_origin.is_some() && i==0 && len == 2 {
                let dif_point = self.back_img_origin.unwrap();
                origin.x += dif_point.x;
                origin.y += dif_point.y;
            }


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