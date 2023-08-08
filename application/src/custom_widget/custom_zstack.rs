use std::any::Any;
use std::collections::linked_list::LinkedList;
use std::path::PathBuf;
use std::sync::Arc;
use druid::{commands as sys_cmd, AppDelegate, BoxConstraints, Command, Data, DelegateCtx, Env, Event, EventCtx, Handled, InternalEvent, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, Size, Target, UnitPoint, UpdateCtx, Vec2, Widget, WidgetExt, WidgetPod, ImageBuf, WindowDesc, FileInfo, WidgetId, Key};
use druid::piet::ImageFormat;
use druid::widget::{Button, Image, Label, Svg, SvgData};
use image::{DynamicImage, GenericImage, GenericImageView, RgbaImage};
use image::io::Reader;
use tracing::error;
use crate::{AppState, SHOW_OVER_IMG};

/// A container that stacks its children on top of each other.
///
/// The container has a baselayer which has the lowest z-index and determines the size of the
/// container.
pub struct CustomZStack<T> {
    layers: Vec<ZChild<T>>,
    over_img: Option<DynamicImage>,
    id: Option<WidgetId>
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
            id: None,
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
        self
    }

    fn rm_child(self: &mut Self)-> ZChild<T>{
        self.layers.pop().unwrap()
    }

    pub fn show_over_svg(self: &mut Self, open_path: &'static str){
        let img = Reader::open(open_path).unwrap().decode().unwrap();
        /*let svg_img = match include_str!("../../icons/open-file.svg").parse::<SvgData>() {
            Ok(svg) => svg,
            Err(err) => {
                error!("{}", err);
                error!("Using an empty SVG instead.");
                SvgData::default()
            }
        };
        let over_image = Svg::new(svg_img.clone());*/
        let over_image = Image::new(ImageBuf::from_raw(
            Arc::<[u8]>::from(img.as_bytes()), ImageFormat::RgbaSeparate, img.width() as usize, img.height() as usize
        ));
        self.with_child(over_image, Vec2::new(1.,1.), Vec2::ZERO, UnitPoint::CENTER, Vec2::new(5.,5.))
            .over_img = Some(img);
    }
    ///return true if has correctly save, false otherwise
    pub fn save_new_img(self: &mut Self, back_img: &mut DynamicImage) ->bool{
        if self.layers.len() > 1 && self.over_img.is_some(){
            let img_rect = self.layers.first().unwrap().child.layout_rect();
            let over_img_rect: Rect = self.layers.get(1).unwrap().child.layout_rect();
            assert_eq!(img_rect.x0,0.);
            assert_eq!(img_rect.y0,0.);
            assert_eq!(img_rect.x1,back_img.width() as f64);
            assert_eq!(img_rect.y1,back_img.height() as f64);
            assert!(img_rect.x0<=over_img_rect.x0);
            assert!(img_rect.y0<=over_img_rect.y0);
            assert!(img_rect.x1>=over_img_rect.x1);
            assert!(img_rect.y1>=over_img_rect.y1);
            let mut out = DynamicImage::new_rgba8(back_img.width(),back_img.height());
            for j in 0..back_img.height(){
                for i in 0..back_img.width(){
                    if over_img_rect.contains(Point::new(i.clone() as f64, j.clone() as f64)){
                        out.put_pixel(i.clone(), j.clone(), self.over_img.as_ref().unwrap().get_pixel(i.clone(), j.clone()))
                    } else {
                        assert!(img_rect.contains(Point::new(i.clone() as f64, j.clone() as f64)));
                        out.put_pixel(i.clone(), j.clone(), back_img.get_pixel(i.clone(), j.clone()))
                    }
                }
            }
            while self.layers.len() > 1 {
                self.rm_child();
            }
            let old = self.rm_child();
            self.with_child(Image::new(ImageBuf::from_raw(
                Arc::<[u8]>::from(back_img.as_bytes()), ImageFormat::RgbaSeparate, back_img.width() as usize, back_img.height() as usize
            )), old.relative_size, old.absolute_size, old.position, old.offset);
            true
        } else {
            false
        }
    }
}

impl<T: Data> Widget<T> for CustomZStack<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) => {
                if cmd.is(SHOW_OVER_IMG){
                    let path = cmd.get_unchecked(SHOW_OVER_IMG);
                    self.show_over_svg(*path);
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