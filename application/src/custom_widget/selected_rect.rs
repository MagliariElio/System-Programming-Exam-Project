use tracing::{instrument};
use druid::widget::prelude::*;
use druid::{Cursor, MouseEvent, Point, Rect, Screen, theme};
use druid::piet::{LineJoin, StrokeStyle};

///the distance in pixels from the SelectedRegion borders where a click is relevated
const DISTANCE_MARGIN:f64 = 10.0;
const BORDER_WIDTH:f64 = 5.;

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

#[derive(Clone)]
pub struct SelectedRect {
    rect: Rect,
    mouse: IfMousePressedWhere,
    show_overlay: bool,
}

impl SelectedRect {
    /// Construct SelectedRegion with coordinates set.
    pub fn new(monitor: usize) -> Self {
        //TODO : select the screen in a multi-monitor context!

        let primary_monitor_rect = Screen::get_monitors()
            .get(monitor).expect("Can't find the selected monitor!")
            .virtual_rect();

        Self {
            rect: Rect{
                x0 : primary_monitor_rect.x0,
                y0 : primary_monitor_rect.y0,
                x1 : primary_monitor_rect.x1,
                y1 : primary_monitor_rect.y1,
            },
            mouse: IfMousePressedWhere::NotInterested,
            show_overlay: false,
        }
    }

    fn where_mouse_is(self: &Self, me:&MouseEvent) -> IfMousePressedWhere{
        let pos = me.pos;
        let x0 = self.rect.x0;
        let x1 = self.rect.x1;
        let y0 = self.rect.y0;
        let y1 = self.rect.y1;
        return if f64::abs(pos.x - x0) < DISTANCE_MARGIN {
            if f64::abs(pos.y - y0) < DISTANCE_MARGIN{
                IfMousePressedWhere::NorthWest
            } else if f64::abs(pos.y - y1) < DISTANCE_MARGIN{
                IfMousePressedWhere::SouthWest
            } else {
                IfMousePressedWhere::West
            }
        } else if f64::abs(pos.x - x1) < DISTANCE_MARGIN {
            if f64::abs(pos.y - y0) < DISTANCE_MARGIN{
                IfMousePressedWhere::NorthEst
            } else if f64::abs(pos.y - y1) < DISTANCE_MARGIN{
                IfMousePressedWhere::SouthEst
            } else {
                IfMousePressedWhere::Est
            }
        } else if f64::abs(pos.y - y0) < DISTANCE_MARGIN{
            IfMousePressedWhere::North
        } else if f64::abs(pos.y - y1) < DISTANCE_MARGIN{
            IfMousePressedWhere::South
        } else if pos.y > y0 && pos.y < y1 && pos.x > x0 && pos.x < x1{
            IfMousePressedWhere::Inside(pos)
        } else {
            IfMousePressedWhere::NotInterested
        }
    }
}

impl Widget<Rect> for SelectedRect {
    #[instrument(name = "SelectedRegion", level = "trace", skip(self, ctx, event, data, _env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Rect, _env: &Env) {
        match event {
            Event::MouseDown(me) => {
                ctx.set_active(true);
                self.mouse = self.where_mouse_is(me);
                self.show_overlay = true;
            }
            Event::MouseMove(me) => {
                if self.mouse != IfMousePressedWhere::NotInterested{ //if the mouse has been pressed
                    let pos = me.pos;
                    match self.mouse {
                        IfMousePressedWhere::NotInterested => (),
                        IfMousePressedWhere::North => {
                            self.rect.y0 = pos.y;
                        }
                        IfMousePressedWhere::NorthEst => {
                            self.rect.y0 = pos.y;
                            self.rect.x1 = pos.x;
                        }
                        IfMousePressedWhere::Est => {
                            self.rect.x1 = pos.x;
                        }
                        IfMousePressedWhere::SouthEst => {
                            self.rect.y1 = pos.y;
                            self.rect.x1 = pos.x;
                        }
                        IfMousePressedWhere::South => {
                            self.rect.y1 = pos.y;
                        }
                        IfMousePressedWhere::SouthWest => {
                            self.rect.y1 = pos.y;
                            self.rect.x0 = pos.x;
                        }
                        IfMousePressedWhere::West => {
                            self.rect.x0 = pos.x;
                        }
                        IfMousePressedWhere::NorthWest => {
                            self.rect.y0 = pos.y; self.rect.x0 = pos.x;
                        }
                        IfMousePressedWhere::Inside(old_pos) => {
                            self.rect.y0 += pos.y - old_pos.y;
                            self.rect.y1 += pos.y - old_pos.y;
                            self.rect.x0 += pos.x - old_pos.x;
                            self.rect.x1 += pos.x - old_pos.x;
                            self.mouse = IfMousePressedWhere::Inside(pos);
                            self.show_overlay = true;
                        }
                    }
                } else { //the mouse has not been pressed
                    match self.where_mouse_is(me) {
                        IfMousePressedWhere::North => {
                            ctx.override_cursor(&Cursor::ResizeUpDown);
                        }
                        IfMousePressedWhere::NorthEst => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        IfMousePressedWhere::Est => {
                            ctx.override_cursor(&Cursor::ResizeLeftRight);
                        }
                        IfMousePressedWhere::SouthEst => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        IfMousePressedWhere::South => {
                            ctx.override_cursor(&Cursor::ResizeUpDown);
                        }
                        IfMousePressedWhere::SouthWest => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        IfMousePressedWhere::West => {
                            ctx.override_cursor(&Cursor::ResizeLeftRight);
                        }
                        IfMousePressedWhere::NorthWest => {
                            ctx.override_cursor(&Cursor::Crosshair);
                        }
                        _ => ctx.clear_cursor()
                    }
                }
            }
            Event::MouseUp(_)=>{
                self.mouse = IfMousePressedWhere::NotInterested;
                ctx.set_active(false);
                self.show_overlay = false;
            }
            _ => (),
        }
        //Keeps validity
        while self.rect.x1<=self.rect.x0+BORDER_WIDTH{
            self.rect.x1 += 1.+BORDER_WIDTH;
            self.rect.x0 -= 1.+BORDER_WIDTH;
        }
        while self.rect.y1<=self.rect.y0+BORDER_WIDTH{
            self.rect.y1 += 1.+BORDER_WIDTH;
            self.rect.y0 -= 1.+BORDER_WIDTH;
        }
        //Validity check: inside the monitor size
        let primary_monitor_rect = Screen::get_monitors()
            //.into_iter().filter(|m|m.is_primary()).collect::<Vec<Monitor>>()
            .first().expect("No primary monitor found!")
            .virtual_rect();
        if self.rect.x0<primary_monitor_rect.x0{
            self.rect.x0 = primary_monitor_rect.x0;
        }
        if self.rect.y0<primary_monitor_rect.y0{
            self.rect.y0 = primary_monitor_rect.y0;
        }
        if self.rect.x1>primary_monitor_rect.x1{
            self.rect.x1 = primary_monitor_rect.x1-BORDER_WIDTH;
        }
        if self.rect.y1>primary_monitor_rect.y1{
            self.rect.y1 = primary_monitor_rect.y1-BORDER_WIDTH;
        }
        *data=self.rect;
    }

    #[instrument(name = "SelectedRegion", level = "trace", skip(self, ctx, event, _data, _env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &Rect, _env: &Env) {
        if let LifeCycle::HotChanged(_) | LifeCycle::DisabledChanged(_) | LifeCycle::ViewContextChanged(_) | LifeCycle::FocusChanged(_) = event {
            ctx.request_paint();
        }
    }

    #[instrument(
    name = "SelectedRegion",
    level = "trace",
    skip(self, ctx, _old_data, _data, _env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &Rect, _data: &Rect, _env: &Env) {
        ctx.request_paint();
    }

    #[instrument(name = "SelectedRegion", level = "trace", skip(self, _ctx, bc, _data, _env))]
    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &Rect, _env: &Env) -> Size {
        bc.debug_check("SelectedRegion");
        let overlay_padding = if self.show_overlay { 0.0 } else { BORDER_WIDTH };
        bc.constrain(Size::new(
            self.rect.x1 - self.rect.x0 + overlay_padding * 2.0,
            self.rect.y1 - self.rect.y0 + overlay_padding * 2.0,
        ))
    }


    #[instrument(name = "SelectedRegion", level = "trace", skip(self, ctx, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &Rect, env: &Env) {
        let rect = Rect {
            x0: self.rect.x0,
            y0: self.rect.y0,
            x1: self.rect.x1,
            y1: self.rect.y1,
        };

        // Draw the overlay
        let overlay_color = env.get(theme::BACKGROUND_DARK).with_alpha(0.5);
        ctx.fill(rect, &overlay_color);

        // Draw the selected rectangle border
        let border_color = if ctx.is_hot() && !ctx.is_disabled() {
            env.get(theme::BORDER_DARK)
        } else {
            env.get(theme::BORDER_LIGHT)
        };
        let style: StrokeStyle = StrokeStyle::new()
            .dash_pattern(&[12.0, 7.0])
            .line_join(LineJoin::Round)
            .line_cap(Default::default())
            .dash_offset(0.0);
        ctx.stroke_styled(rect, &border_color, BORDER_WIDTH, &style);
    }


}