//TODO: bug: the over-image disappear while moving

use druid::debug_state::DebugState;
use tracing::{instrument, trace, warn};

use druid::widget::prelude::*;
use druid::widget::{Axis};
use druid::{Color, Cursor, Data, KeyOrValue, MouseEvent, Point, Rect, Selector, WidgetPod};
use druid::piet::{LineJoin, StrokeStyle};

const BORDER_WIDTH:f64 = 2.;
const DISTANCE_MARGIN:f64 = 10.0;

pub(crate) const UPDATE_ORIGIN:Selector<Point> = Selector::new("Tell the customZStack to update the resizableBox origin");

#[derive(Copy, Clone, PartialEq)]
enum IfMousePressedWhere {
    North(Point),
    NorthEst(Point),
    Est(Point),
    SouthEst(Point),
    South(Point),
    SouthWest(Point),
    West(Point),
    NorthWest(Point),
    Inside(Point),
    NotInterested
}

/// A widget with predefined size.
///
/// If given a child, this widget forces its child to have a specific width and/or height
/// (assuming values are permitted by this widget's parent). If either the width or height is not
/// set, this widget will size itself to match the child's size in that dimension.
///
/// If not given a child, SizedBox will try to size itself as close to the specified height
/// and width as possible given the parent's constraints. If height or width is not set,
/// it will be treated as zero.
pub struct ResizableBox<T> {
    child: Option<WidgetPod<T, Box<dyn Widget<T>>>>,
    width: Option<KeyOrValue<f64>>,
    height: Option<KeyOrValue<f64>>,
    mouse: IfMousePressedWhere,
    rect: Option<Rect>,
    new_origin: Option<Point>,
    father_id: WidgetId
}

#[allow(dead_code)]
impl<T> ResizableBox<T> {
    /// Construct container with child, and both width and height not set.
    pub fn new(child: impl Widget<T> + 'static, father_id: WidgetId) -> Self {
        Self {
            child: Some(WidgetPod::new(Box::new(child))),
            width: None,
            height: None,
            mouse: IfMousePressedWhere::NotInterested,
            rect: None,
            new_origin: None,
            father_id
        }
    }

    /// Set container's width.
    pub fn width(mut self, width: impl Into<KeyOrValue<f64>>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: impl Into<KeyOrValue<f64>>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Expand container to fit the parent.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: #method.expand_height
    /// [`expand_width`]: #method.expand_width
    pub fn expand(mut self) -> Self {
        self.width = Some(KeyOrValue::Concrete(f64::INFINITY));
        self.height = Some(KeyOrValue::Concrete(f64::INFINITY));
        self
    }

    /// Expand the container on the x-axis.
    ///
    /// This will force the child to have maximum width.
    pub fn expand_width(mut self) -> Self {
        self.width = Some(KeyOrValue::Concrete(f64::INFINITY));
        self
    }

    /// Expand the container on the y-axis.
    ///
    /// This will force the child to have maximum height.
    pub fn expand_height(mut self) -> Self {
        self.height = Some(KeyOrValue::Concrete(f64::INFINITY));
        self
    }

    fn child_constraints(&self, bc: &BoxConstraints, env: &Env) -> BoxConstraints {
        // if we don't have a width/height, we don't change that axis.
        // if we have a width/height, we clamp it on that axis.
        let (min_width, max_width) = match &self.width {
            Some(width) => {
                let width = width.resolve(env);
                let w = width.clamp(bc.min().width, bc.max().width);
                (w, w)
            }
            None => (bc.min().width, bc.max().width),
        };

        let (min_height, max_height) = match &self.height {
            Some(height) => {
                let height = height.resolve(env);
                let h = height.clamp(bc.min().height, bc.max().height);
                (h, h)
            }
            None => (bc.min().height, bc.max().height),
        };

        BoxConstraints::new(
            Size::new(min_width, min_height),
            Size::new(max_width, max_height),
        )
    }

    fn where_mouse_is(self: &Self, me:&MouseEvent) -> IfMousePressedWhere{
        let pos = me.pos.clone();
        let x0 = self.rect.unwrap().x0;
        let x1 = self.rect.unwrap().x1;
        let y0 = self.rect.unwrap().y0;
        let y1 = self.rect.unwrap().y1;
        return if f64::abs(pos.x.clone() - x0) < DISTANCE_MARGIN {
            if f64::abs(pos.y.clone() - y0) < DISTANCE_MARGIN{
                IfMousePressedWhere::NorthWest(pos)
            } else if f64::abs(pos.y.clone() - y1) < DISTANCE_MARGIN{
                IfMousePressedWhere::SouthWest(pos)
            } else {
                IfMousePressedWhere::West(pos)
            }
        } else if f64::abs(pos.x.clone() - x1) < DISTANCE_MARGIN {
            if f64::abs(pos.y.clone() - y0) < DISTANCE_MARGIN{
                IfMousePressedWhere::NorthEst(pos)
            } else if f64::abs(pos.y.clone() - y1) < DISTANCE_MARGIN{
                IfMousePressedWhere::SouthEst(pos)
            } else {
                IfMousePressedWhere::Est(pos)
            }
        } else if f64::abs(pos.y.clone() - y0) < DISTANCE_MARGIN{
            IfMousePressedWhere::North(pos)
        } else if f64::abs(pos.y.clone() - y1) < DISTANCE_MARGIN{
            IfMousePressedWhere::South(pos)
        } else if pos.y.clone() > y0 && pos.y.clone() < y1 && pos.x.clone() > x0 && pos.x.clone() < x1{
            IfMousePressedWhere::Inside(pos)
        } else {
            IfMousePressedWhere::NotInterested
        }
    }

    fn set_rect(&mut self, origin: Point ,size: Size){
        let rect =Rect::new(
            origin.x,
            origin.y,
            size.width + origin.x,
            size.height + origin.y,
        );
        self.width = Some(KeyOrValue::Concrete(rect.width()));
        self.height = Some(KeyOrValue::Concrete(rect.height()));
        self.rect = Some(rect);
        self.new_origin = Some(origin);
    }
}

impl<T: Data> Widget<T> for ResizableBox<T> {
    #[instrument(name = "SizedBox", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::WindowConnected => {
                self.set_rect(Point::ZERO,ctx.size());
                ctx.request_layout();
            }
            Event::MouseDown(me) => {
                ctx.set_active(true);
                self.mouse = self.where_mouse_is(me);
                ctx.request_paint();
            }
            Event::MouseMove(me) => {
                if self.mouse != IfMousePressedWhere::NotInterested { //if the mouse has been pressed
                    let pos = me.pos.clone();
                    let mut rect = self.rect.unwrap();
                    match self.mouse {
                        IfMousePressedWhere::NotInterested => (),
                        IfMousePressedWhere::North(old_pos) => {
                            rect.x0 += pos.y - old_pos.y;
                            rect.y0 += pos.y - old_pos.y;
                            self.mouse = IfMousePressedWhere::North(pos);
                        }
                        IfMousePressedWhere::NorthEst(old_pos) => {
                            if f64::abs(pos.x - old_pos.x) > f64::abs(pos.y - old_pos.y){
                                let dif = pos.x - old_pos.x;
                                rect.y0 -= dif;
                                rect.x1 += dif;
                            } else {
                                let dif = pos.y - old_pos.y;
                                rect.y0 += dif;
                                rect.x1 -= dif;
                            };
                            self.mouse = IfMousePressedWhere::NorthEst(pos);
                        }
                        IfMousePressedWhere::Est(old_pos) => {
                            rect.y1 += pos.x - old_pos.x;
                            rect.x1 = pos.x;
                            self.mouse = IfMousePressedWhere::Est(pos);
                        }
                        IfMousePressedWhere::SouthEst(old_pos) => {
                            let dif = if f64::abs(pos.x - old_pos.x) > f64::abs(pos.y - old_pos.y){
                                pos.x - old_pos.x
                            } else {
                                pos.y - old_pos.y
                            };
                            rect.y1 += dif;
                            rect.x1 += dif;
                            self.mouse = IfMousePressedWhere::SouthEst(pos);
                        }
                        IfMousePressedWhere::South(old_pos) => {
                            rect.x1 += pos.y - old_pos.y;
                            rect.y1 = pos.y;
                            self.mouse = IfMousePressedWhere::South(pos);
                        }
                        IfMousePressedWhere::SouthWest(old_pos) => {
                            if f64::abs(pos.x - old_pos.x) > f64::abs(pos.y - old_pos.y){
                                let dif = pos.x - old_pos.x;
                                rect.y1 -= dif;
                                rect.x0 += dif;
                            } else {
                                let dif = pos.y - old_pos.y;
                                rect.y1 += dif;
                                rect.x0 -= dif;
                            };
                            self.mouse = IfMousePressedWhere::SouthWest(pos);
                        }
                        IfMousePressedWhere::West(old_pos) => {
                            rect.y0 += pos.x - old_pos.x;
                            rect.x0 = pos.x;
                            self.mouse = IfMousePressedWhere::West(pos);
                        }
                        IfMousePressedWhere::NorthWest(old_pos) => {
                            let dif = if f64::abs(pos.x - old_pos.x) > f64::abs(pos.y - old_pos.y){
                                pos.x - old_pos.x
                            } else {
                                pos.y - old_pos.y
                            };
                            rect.y0 += dif;
                            rect.x0 += dif;
                            self.mouse = IfMousePressedWhere::NorthWest(pos);
                        }
                        IfMousePressedWhere::Inside(old_pos) => {
                            rect.y0 += pos.y.clone() - old_pos.y.clone();
                            rect.y1 += pos.y.clone() - old_pos.y;
                            rect.x0 += pos.x.clone() - old_pos.x.clone();
                            rect.x1 += pos.x.clone() - old_pos.x;
                            self.mouse = IfMousePressedWhere::Inside(pos);
                        }
                    }
                    /* TODO
                    //Keeps validity
                    while rect.x1 <= rect.x0+BORDER_WIDTH+10.{
                        rect.x1 += 3.+BORDER_WIDTH;
                        rect.x0 -= 3.+BORDER_WIDTH;
                    }
                    while rect.y1 <= rect.y0+BORDER_WIDTH+10.{
                        rect.y1 += 3.+BORDER_WIDTH;
                        rect.y0 -= 3.+BORDER_WIDTH;
                    }
                    //Validity check: inside the window size
                    let window_rect = ctx.window().get_size().to_rect();
                    let mut inside_rect = Rect::ZERO;
                    inside_rect.x0 = rect.x0 + ctx.window_origin().x;
                    inside_rect.x1 = rect.x1 + ctx.window_origin().x;
                    inside_rect.y0 = rect.y0 + ctx.window_origin().y;
                    inside_rect.y1 = rect.y1 + ctx.window_origin().y;
                    if inside_rect.x0<= window_rect.x0+1.{
                        self.mouse = IfMousePressedWhere::NotInterested;
                    }
                    if inside_rect.y0<= window_rect.y0+1.{
                        self.mouse = IfMousePressedWhere::NotInterested;
                    }
                    if inside_rect.x1 >= window_rect.x1-1.{
                        self.mouse = IfMousePressedWhere::NotInterested;
                    }
                    if inside_rect.y1 >= window_rect.y1-1.{
                        self.mouse = IfMousePressedWhere::NotInterested;
                    }
                    */


                    self.rect = Some(rect);
                    self.width = Some(KeyOrValue::Concrete(rect.width()));
                    self.height = Some(KeyOrValue::Concrete(rect.height()));
                    self.child.as_mut().unwrap().set_origin(ctx,rect.origin());
                    ctx.request_layout();
                } else { //the mouse has not been pressed
                    match self.where_mouse_is(me) {
                        IfMousePressedWhere::North(_) => {
                            ctx.override_cursor(&Cursor::ResizeUpDown);
                        }
                        IfMousePressedWhere::NorthEst(_) => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        IfMousePressedWhere::Est(_) => {
                            ctx.override_cursor(&Cursor::ResizeLeftRight);
                        }
                        IfMousePressedWhere::SouthEst(_) => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        IfMousePressedWhere::South(_) => {
                            ctx.override_cursor(&Cursor::ResizeUpDown);
                        }
                        IfMousePressedWhere::SouthWest(_) => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        IfMousePressedWhere::West(_) => {
                            ctx.override_cursor(&Cursor::ResizeLeftRight);
                        }
                        IfMousePressedWhere::NorthWest(_) => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        _ => ctx.clear_cursor()
                    }
                }
            }
            Event::MouseUp(_)=>{
                self.mouse = IfMousePressedWhere::NotInterested;
                ctx.set_active(false);
                if self.rect.is_some() {
                    let rect = self.rect.unwrap();
                    let mut new_origin = self.new_origin.unwrap();
                    new_origin.x += rect.x0;
                    new_origin.y += rect.y0;
                    ctx.submit_command(
                        UPDATE_ORIGIN
                            .with(new_origin)
                            .to(druid::Target::Widget(self.father_id))
                    );
                    self.new_origin = Some(new_origin);
                    self.rect = Some(Rect::new(
                        0.,
                        0.,
                        rect.width(),
                        rect.height(),
                    ));
                    self.child.as_mut().unwrap().set_origin(ctx,Point::ZERO);
                }

                ctx.request_layout();
            }
            _ => {}
        }
        if let Some(ref mut child) = self.child {
            child.event(ctx, event, data, env);
        }
    }

    #[instrument(name = "SizedBox", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let Some(ref mut child) = self.child {
            child.lifecycle(ctx, event, data, env)
        }
    }

    #[instrument(
    name = "SizedBox",
    level = "trace",
    skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let Some(ref mut child) = self.child {
            child.widget_mut().update(ctx, old_data, data, env);
        }
        ctx.request_layout();
    }

    #[instrument(name = "SizedBox", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("SizedBox");

        let child_bc = self.child_constraints(bc, env);

        let size = match self.child.as_mut() {
            Some(child) => {
                let size = child.layout(ctx, &child_bc, data, env);

                if self.rect.is_some() {
                    let rect = self.rect.unwrap();
                    //child.set_origin(ctx,rect.origin());

                    rect.size()
                } else {
                    //child.set_origin(ctx,Point::ZERO);

                    size
                }
            },
            None => bc.constrain((
                self.width
                    .as_ref()
                    .unwrap_or(&KeyOrValue::Concrete(0.0))
                    .resolve(env),
                self.height
                    .as_ref()
                    .unwrap_or(&KeyOrValue::Concrete(0.0))
                    .resolve(env),
            )),
        };

        trace!("Computed size: {}", size);
        if size.width.is_infinite() {
            warn!("SizedBox is returning an infinite width.");
        }

        if size.height.is_infinite() {
            warn!("SizedBox is returning an infinite height.");
        }

        size
    }

    #[instrument(name = "SizedBox", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.rect.is_none() {
            self.set_rect(Point::ZERO,ctx.size());
        }

        let rect = self.rect.unwrap();
        let border_color = Color::BLACK;

        let style: StrokeStyle = StrokeStyle::new()
            .dash_pattern(&[4., 2.])
            .line_join(LineJoin::Round)
            .line_cap(Default::default())
            .dash_offset(0.0);
        ctx.stroke_styled(rect, &border_color, BORDER_WIDTH, &style);

        if let Some(ref mut child) = self.child {
            child.paint(ctx, data, env);
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.child.as_ref().and_then(|child| Option::from(child.id()))
    }

    fn debug_state(&self, data: &T) -> DebugState {
        let children = if let Some(child) = &self.child {
            vec![child.widget().debug_state(data)]
        } else {
            vec![]
        };
        DebugState {
            display_name: self.short_type_name().to_string(),
            children,
            ..Default::default()
        }
    }

    fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> f64 {
        let kv = match axis {
            Axis::Horizontal => self.width.as_ref(),
            Axis::Vertical => self.height.as_ref(),
        };
        match (self.child.as_mut(), kv) {
            (Some(c), Some(v)) => {
                let v = v.resolve(env);
                if v == f64::INFINITY {
                    c.widget_mut().compute_max_intrinsic(axis, ctx, bc, data, env)
                } else {
                    v
                }
            }
            (Some(c), None) => c.widget_mut().compute_max_intrinsic(axis, ctx, bc, data, env),
            (None, Some(v)) => {
                let v = v.resolve(env);
                if v == f64::INFINITY {
                    // If v infinite, we can only warn.
                    warn!("SizedBox is without a child and its dim is infinite. Either give SizedBox a child or make its dim finite. ")
                }
                v
            }
            (None, None) => 0.,
        }
    }
}
