mod custom_widget;

use druid::widget::{Container, Flex, Label, LensWrap, ZStack};
use druid::{AppLauncher, Data, Env, Lens, LocalizedString, Widget, WindowDesc, WindowState, Color, Rect, Vec2, UnitPoint, EventCtx, FontDescriptor, FontFamily};
use crate::custom_widget::{SelectedRect,ColoredButton};

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("screen grabbing utility");
const X0:f64 = 0.;
const Y0:f64 = 0.;
const X1:f64 = 1000.;
const Y1:f64 = 500.;

#[derive(Clone, Data, Lens)]
struct AppState{
    rect: Rect
}

fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title(WINDOW_TITLE)
        .set_always_on_top(true)
        .transparent(true)
        .resizable(false)
        .show_titlebar(false)
        .set_window_state(WindowState::Maximized);

    // create the initial app state
    let initial_state = AppState {
        rect: Rect{
            x0: X0,
            y0: Y0,
            x1: X1,
            y1: Y1,
        }
    };

    // start the application
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    let rectangle = LensWrap::new(SelectedRect::new(),AppState::rect);
    let label = Label::new(|data: &AppState, _env: &Env|
        format!("Resize and drag as you like.\n- Top Left: ({}, {})\n- Bottom Right: ({}, {})",
                data.rect.x0,data.rect.y0,data.rect.x1,data.rect.y1));

    let take_screenshot_button=ColoredButton::from_label(
        Label::new("Take Screen").with_text_color(Color::BLACK).with_font(FontDescriptor::new(FontFamily::MONOSPACE)).with_text_size(20.)
    )
        .with_color(Color::rgb8(70,250,70).with_alpha(0.40))
        .on_click(|_ctx:&mut EventCtx,_data: &mut AppState,_env: &Env|{
            //TODO: take the screen shot!!
        });
    let close_button = ColoredButton::from_label(
        Label::new("Close").with_text_color(Color::BLACK).with_font(FontDescriptor::new(FontFamily::MONOSPACE)).with_text_size(20.)
    )
        .with_color(Color::rgb8(250, 70, 70).with_alpha(0.40))
        .on_click(|ctx:&mut EventCtx,_data: &mut AppState,_env: &Env|{
            ctx.window().close();
        });
    let buttons_flex = Flex::row().with_child(take_screenshot_button).with_default_spacer().with_child(close_button);

    let mut label_container =  Container::new(label);
    label_container.set_background(Color::BLACK.with_alpha(0.35));

    let zstack = ZStack::new(rectangle)
        .with_child(label_container,Vec2::new(1.0, 1.0),Vec2::ZERO, UnitPoint::LEFT,Vec2::new(10.0, 0.0))
        .with_child(buttons_flex,Vec2::new(1.0, 1.0),Vec2::ZERO, UnitPoint::BOTTOM_RIGHT,Vec2::new(-100.0, -100.0));
    zstack
}