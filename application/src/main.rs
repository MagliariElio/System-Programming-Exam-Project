mod custom_widget;

use std::fs::File;
use std::io::BufWriter;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::TryRecvError;
use druid::widget::{Button, Container, CrossAxisAlignment, FillStrat, Flex, FlexParams, IdentityWrapper, Image, Label, LensWrap, MainAxisAlignment, Maybe, Svg, SvgData, ZStack};
use druid::{commands as sys_cmd, AppLauncher, Data, Env, Lens, LocalizedString, Widget, WindowDesc, WindowState, Color, Rect, Vec2, UnitPoint, EventCtx, FontDescriptor, FontFamily, WindowId, Menu, ImageBuf, WidgetExt, BoxConstraints, LayoutCtx, FileInfo, FileSpec, Target, WidgetId, DelegateCtx, Selector};
use druid::kurbo::SvgArc;
use druid::piet::{ImageFormat, InterpolationMode};
use druid::Target::{Auto, Global};
use image::io::Reader;
use tracing::{error, Id};
use crate::custom_widget::{SelectedRect, ColoredButton, CustomZStack, OverImage};

pub const SHOW_OVER_IMG: Selector<&'static str> = Selector::new("../../icons/open-file.svg");
const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("screen grabbing utility");
const X0:f64 = 0.;
const Y0:f64 = 0.;
const X1:f64 = 1000.;
const Y1:f64 = 500.;

#[derive(Clone, Data, Lens)]
struct AppState{
    rect: Rect,
    #[data(ignore)]
    main_window_id: Option<WindowId>
}

fn main() {
    let main_window=WindowDesc::new(build_root_widget())
        .title("Welcome!")
        .menu(make_menu)
        .window_size((1000.,500.));
    // create the initial app state
    let initial_state = AppState {
        rect: Rect{
            x0: X0,
            y0: Y0,
            x1: X1,
            y1: Y1,
        },
        main_window_id: None,
    };

    // start the application
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_screenshot_widget() -> impl Widget<AppState> {
    let rectangle = LensWrap::new(SelectedRect::new(),AppState::rect);

    let label = Label::new(|data: &AppState, _env: &Env|
        format!("Resize and drag as you like.\n- Top Left: ({}, {})\n- Bottom Right: ({}, {})",
                data.rect.x0,data.rect.y0,data.rect.x1,data.rect.y1));

    let take_screenshot_button=ColoredButton::from_label(
        Label::new("Take Screen")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.)
    ).with_color(Color::rgb8(70,250,70)
        .with_alpha(0.40))
        .on_click(|ctx:&mut EventCtx, _data: &mut AppState, _env: &Env|{
            ctx.submit_command(sys_cmd::HIDE_WINDOW.to(Auto));
            //TODO: take the screen shot!!
            ctx.submit_command(sys_cmd::SHOW_WINDOW.to(Auto));
        });

    let close_button = ColoredButton::from_label(
        Label::new("Close")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.)
    ).with_color(Color::rgb8(250, 70, 70)
        .with_alpha(0.40))
        .on_click(|ctx:&mut EventCtx, data: &mut AppState, _env: &Env|{
            let main_id = data.main_window_id
                .expect("How did you opened this window?");
            ctx.get_external_handle().submit_command(sys_cmd::SHOW_WINDOW, (), main_id)
                .expect("Error sending the event");
            ctx.window().close();
        });

    let buttons_flex = Flex::row()
        .with_child(take_screenshot_button)
        .with_default_spacer()
        .with_child(close_button);

    let mut label_container =  Container::new(label);
    label_container.set_background(Color::BLACK.with_alpha(0.35));

    let zstack = ZStack::new(rectangle)
        .with_child(label_container,Vec2::new(1.0, 1.0),Vec2::ZERO, UnitPoint::LEFT,Vec2::new(10.0, 0.0))
        .with_child(buttons_flex,Vec2::new(1.0, 1.0),Vec2::ZERO, UnitPoint::BOTTOM_RIGHT,Vec2::new(-100.0, -100.0));
    zstack
}

fn build_root_widget()-> impl Widget<AppState>{
    let take_screenshot_button = Button::from_label(Label::new("Take screen"))
        .on_click(|ctx:&mut EventCtx, data: &mut AppState, _env: &Env|{
            data.main_window_id = Some(ctx.window_id());
            ctx.submit_command(sys_cmd::HIDE_WINDOW.to(Auto));
            ctx.new_window(WindowDesc::new(build_screenshot_widget())
                               .title(WINDOW_TITLE)
                               .set_always_on_top(true)
                               .transparent(true)
                               .resizable(false)
                               .show_titlebar(false)
                               .set_window_state(WindowState::Maximized))
        });

    let img = Reader::open("./src/images/PicWithAlpha.png")
        .expect("Can't open the screenshot!")
        .decode()
        .expect("Can't decode the screenshot");
    let mut screenshot_image = Image::new(
        ImageBuf::from_raw(
            Arc::<[u8]>::from(img.as_bytes()), ImageFormat::RgbaSeparate, img.width() as usize, img.height() as usize
        )
    );

    let zstack_id = WidgetId::next();
    let mut zstack = IdentityWrapper::wrap(CustomZStack::new(screenshot_image),zstack_id);
    let add_img_button = Button::from_label(Label::new("+"))
        .on_click(move |ctx:&mut EventCtx, data: &mut AppState, env: &Env|{
            ctx.submit_command(SHOW_OVER_IMG.with("./icons/open-file.png").to(Target::Widget(zstack_id)));
        });

    let mut flex = Flex::row();
    flex.set_must_fill_main_axis(true);
    flex.add_child(zstack);
    flex.add_default_spacer();
    flex.add_flex_child(Container::new(take_screenshot_button),FlexParams::new(1.,CrossAxisAlignment::End));
    flex.add_child(add_img_button);
    flex.set_main_axis_alignment(MainAxisAlignment::Center);
    let layout = flex.background(Color::SILVER);
    layout
}

fn make_menu(_window: Option<WindowId>, _data: &AppState, _env: &Env) -> Menu<AppState> {
    let mut base = Menu::empty();
    #[cfg(target_os = "macos")]
    {
        base = base.entry(druid::platform_menus::mac::application::default())
    }
    #[cfg(any(
    target_os = "windows",
    target_os = "freebsd",
    target_os = "linux",
    target_os = "openbsd"
    ))]
    {
        base = base.entry(druid::platform_menus::win::file::default());
    }
    base.entry(
        Menu::new(LocalizedString::new("common-menu-edit-menu"))
            .entry(druid::platform_menus::common::undo())
            .entry(druid::platform_menus::common::redo())
            .separator()
            .entry(druid::platform_menus::common::cut())
            .entry(druid::platform_menus::common::copy())
            .entry(druid::platform_menus::common::paste()),
    )
}