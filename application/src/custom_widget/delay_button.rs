use std::sync::Arc;
use std::thread;
use std::time::Duration;
use druid::debug_state::DebugState;
use druid::widget::prelude::*;
use druid::widget::{Click, ControllerHost, Label, LabelText};
use druid::{commands as sys_cmd, theme, Affine, Data, Insets, LinearGradient, UnitPoint, Color, Rect, WindowId, Selector};
use image::{DynamicImage, ImageFormat};
use screenshots::{Screen};
use tracing::{instrument, trace};
use crate::custom_widget::screenshot_image::UPDATE_SCREENSHOT;
use crate::custom_widget::UPDATE_BACK_IMG;

pub const DELAY_SCREENSHOT: Selector<(Rect, WindowId, WidgetId, WidgetId, &'static str, Box<str>, ImageFormat, u64)> = Selector::new("Save the screenshot image, last param: where to save");

// the minimum padding added to a button.
// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 2.);

/// A button with a text label.
pub struct DelayButton<T> {
    label: Label<T>,
    label_size: Size,
    color: Option<Color>,
    taking_screenshot: Option<(Rect,WindowId,WidgetId,WidgetId,&'static str,Box<str>,ImageFormat,Duration)>,
}

#[allow(dead_code)]
impl<T: Data> DelayButton<T> {
    /// Create a new button with a text label.
    ///
    /// Use the [`on_click`] method to provide a closure to be called when the
    /// button is clicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::Button;
    ///
    /// let button = Button::new("Increment").on_click(|_ctx, data: &mut u32, _env| {
    ///     *data += 1;
    /// });
    /// ```
    ///
    /// [`on_click`]: #method.on_click
    pub fn new(text: impl Into<LabelText<T>>) -> DelayButton<T> {
        DelayButton::from_label(Label::new(text))
    }

    /// Create a new button with the provided [`Label`].
    ///
    /// Use the [`on_click`] method to provide a closure to be called when the
    /// button is clicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::Color;
    /// use druid::widget::{Button, Label};
    ///
    /// let button = Button::from_label(Label::new("Increment").with_text_color(Color::grey(0.5))).on_click(|_ctx, data: &mut u32, _env| {
    ///     *data += 1;
    /// });
    /// ```
    ///
    /// [`on_click`]: #method.on_click
    pub fn from_label(label: Label<T>) -> DelayButton<T> {
        DelayButton {
            label,
            label_size: Size::ZERO,
            color: None,
            taking_screenshot: None,
        }
    }

    /// Construct a new dynamic button.
    ///
    /// The contents of this button are generated from the data using a closure.
    ///
    /// This is provided as a convenience; a closure can also be passed to [`new`],
    /// but due to limitations of the implementation of that method, the types in
    /// the closure need to be annotated, which is not true for this method.
    ///
    /// # Examples
    ///
    /// The following are equivalent.
    ///
    /// ```
    /// use druid::Env;
    /// use druid::widget::Button;
    /// let button1: Button<u32> = Button::new(|data: &u32, _: &Env| format!("total is {}", data));
    /// let button2: Button<u32> = Button::dynamic(|data, _| format!("total is {}", data));
    /// ```
    ///
    /// [`new`]: #method.new
    pub fn dynamic(text: impl Fn(&T, &Env) -> String + 'static) -> Self {
        let text: LabelText<T> = text.into();
        DelayButton::new(text)
    }

    /// Provide a closure to be called when this button is clicked.
    pub fn on_click(
        self,
        f: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> ControllerHost<Self, Click<T>> {
        ControllerHost::new(self, Click::new(f))
    }

    pub fn with_color(self, color: Color)->Self{
        Self{
            label:self.label,
            label_size:self.label_size,
            color:Some(color),
            taking_screenshot: self.taking_screenshot,
        }
    }
}

impl<T: Data> Widget<T> for DelayButton<T> {
    #[instrument(name = "Button", level = "trace", skip(self, ctx, event, _data, _env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {

        if self.taking_screenshot.is_some(){
            let (rect,main_window_id,custom_zstack_id,screenshot_id,path,file_name,file_format, d) = self.taking_screenshot.as_ref().unwrap();
            let d = d.clone();
            let tj = thread::spawn(move||{
                thread::sleep(d);
            });
            tj.join().unwrap();
            println!("end");
            let new_img = Arc::new(save_screenshot(&rect,path,file_name.clone(),*file_format));
            let main_id = main_window_id;
            ctx.get_external_handle()
                .submit_command(sys_cmd::SHOW_WINDOW, (), *main_id)
                .expect("Error sending the event to the window");
            ctx.get_external_handle()
                .submit_command(UPDATE_SCREENSHOT, new_img.clone(), *screenshot_id)
                .expect("Error sending the event to the screenshot widget");
            ctx.get_external_handle()
                .submit_command(UPDATE_BACK_IMG,new_img,*custom_zstack_id)
                .expect("Error sending the event to the screenshot widget");
            self.taking_screenshot = None;
            ctx.window().close();
        }

        match event {
            Event::Command(cmd) => {
                if cmd.is(DELAY_SCREENSHOT) {
                    ctx.window().hide();
                    let (rect,main_window_id,custom_zstack_id,screenshot_id,path,file_name,file_format, delay) = cmd.get_unchecked(DELAY_SCREENSHOT);
                    let d = Duration::from_secs(*delay);
                    self.taking_screenshot = Some((*rect,*main_window_id,*custom_zstack_id,*screenshot_id,*path,file_name.clone(),*file_format,d));
                    ctx.request_layout();
                }
            }
            Event::MouseDown(_) => {
                if !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                    trace!("Button {:?} pressed", ctx.widget_id());
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() && !ctx.is_disabled() {
                    ctx.request_paint();
                    trace!("Button {:?} released", ctx.widget_id());
                }
                ctx.set_active(false);
            }
            _ => (),
        }
    }

    #[instrument(name = "Button", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::HotChanged(_) | LifeCycle::DisabledChanged(_) = event {
            ctx.request_paint();
        }
        self.label.lifecycle(ctx, event, data, env)
    }

    #[instrument(name = "Button", level = "trace", skip(self, ctx, old_data, data, env))]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.label.update(ctx, old_data, data, env)
    }

    #[instrument(name = "Button", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Button");
        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let label_bc = bc.shrink(padding).loosen();
        self.label_size = self.label.layout(ctx, &label_bc, data, env);
        // HACK: to make sure we look okay at default sizes when beside a textbox,
        // we make sure we will have at least the same height as the default textbox.
        let min_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let baseline = self.label.baseline_offset();
        ctx.set_baseline_offset(baseline + LABEL_INSETS.y1);

        let button_size = bc.constrain(Size::new(
            self.label_size.width + padding.width,
            (self.label_size.height + padding.height).max(min_height),
        ));
        trace!("Computed button size: {}", button_size);
        button_size
    }

    #[instrument(name = "Button", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let is_active = ctx.is_active() && !ctx.is_disabled();
        let is_hot = ctx.is_hot();
        let size = ctx.size();
        let stroke_width = env.get(theme::BUTTON_BORDER_WIDTH);

        let rounded_rect = size
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));

        let bg_gradient = if ctx.is_disabled() {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (
                    env.get(theme::DISABLED_BUTTON_LIGHT),
                    env.get(theme::DISABLED_BUTTON_DARK),
                ),
            )
        } else if is_active {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::BUTTON_DARK), env.get(theme::BUTTON_LIGHT)),
            )
        } else {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::BUTTON_LIGHT), env.get(theme::BUTTON_DARK)),
            )
        };

        let border_color = if is_hot && !ctx.is_disabled() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        if self.color.is_none() {
            ctx.stroke(rounded_rect, &border_color, stroke_width);
        } else {
            ctx.stroke(rounded_rect, &Color::BLACK, 2.);
        }

        if self.color.is_none() {
            ctx.fill(rounded_rect, &bg_gradient);
        } else {
            ctx.fill(rounded_rect, &self.color.unwrap());
        }

        let label_offset = (size.to_vec2() - self.label_size.to_vec2()) / 2.0;

        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(label_offset));
            self.label.paint(ctx, data, env);
        });
    }

    fn debug_state(&self, _data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            main_value: self.label.text().to_string(),
            ..Default::default()
        }
    }
}


fn save_screenshot(rect: &Rect, base_path: &str, file_name: Box<str>, format: ImageFormat) -> DynamicImage{
    //TODO:
    /* READ THIS TO IMPLEMENT MULTI-SCREEN GRABBING
    let screens = Screen::all().unwrap();

    for screen in screens {
        println!("capturer {screen:?}");
        let mut image = screen.capture().unwrap();
        let mut buffer = image.to_png(Compression::Best).unwrap();
        fs::write(format!("./src/screenshots/{}.png", screen.display_info.id), buffer).unwrap();

        image = screen.capture_area(rect.x0 as i32, rect.y0 as i32, rect.width() as u32, rect.height() as u32).unwrap();
        buffer = image.to_png(None).unwrap();
        fs::write(format!("./src/screenshots/{}-2.png", screen.display_info.id), buffer).unwrap();
    }
    */

    let screen = Screen::from_point(rect.x0 as i32, rect.y0 as i32).unwrap();
    println!("capturer {:?}",screen);

    let image = screen.capture_area(rect.x0 as i32, rect.y0 as i32, rect.width() as u32, rect.height() as u32).unwrap();

    let dyn_img = DynamicImage::from(
        image
    );
    let path = format!("{}{}.{}", base_path, file_name, format.extensions_str().first().unwrap());
    dyn_img.save_with_format(path,format).unwrap();
    dyn_img
}