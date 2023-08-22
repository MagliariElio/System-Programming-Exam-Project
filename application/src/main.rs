mod custom_widget;

use std::path::Path;
use crate::custom_widget::{ColoredButton, CustomZStack, ScreenshotImage, SelectedRect, TakeScreenshotButton};
use druid::piet::ImageFormat;
use druid::widget::{Align, Button, Click, Container, ControllerHost, Flex, IdentityWrapper, Label, LensWrap, MainAxisAlignment, ZStack};
use druid::Target::{Auto, Window};
use druid::{commands as sys_cmd, AppLauncher, Color, Data, Env, EventCtx, FontDescriptor, FontFamily, ImageBuf, Lens, LocalizedString, Menu, Rect, Selector, Target, UnitPoint, Vec2, Widget, WidgetExt, WidgetId, WindowDesc, WindowId, WindowState, Point};
use image::io::Reader;
use std::sync::Arc;

//TODO: Must remove a lot of .clone() methods everywhere!
//TODO: Set the error messages everywhere they need!
//TODO: Make the main page GUI beautiful

const SCREENSHOT_PATH: &'static str = "./src/screenshots/screenshot.png";

pub const UPDATE_COLOR: Selector<Option<Color>> = Selector::new("Update the over-img color");
pub const SAVE_SCREENSHOT: Selector<(Rect,WindowId,WidgetId,&str)> = Selector::new("Save the screenshot image");
pub const UPDATE_SCREENSHOT: Selector<String> = Selector::new("Update the screenshot image");
pub const SHOW_OVER_IMG: Selector<&str> =
    Selector::new("Tell the ZStack to show the over_img, params: over_img path");
pub const SAVE_OVER_IMG: Selector<(&str ,&str, &str, image::ImageFormat)> = Selector::new("Tell the ZStack to save the modified screenshot, params: (Screenshot original img's path, Folder Path Where To Save, New File Name, Image Format)");

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Screen Grabbing Application");

const X0: f64 = 0.;
const Y0: f64 = 0.;
const X1: f64 = 1920.; // 1000.
const Y1: f64 = 1080.; // 500.

#[derive(Clone, Data, Lens)]
struct AppState {
    rect: Rect,
    #[data(ignore)]
    main_window_id: Option<WindowId>,
    #[data(ignore)]
    screenshot_id: Option<WidgetId>,
    #[data(ignore)]
    color: Option<Color>,
    #[data(ignore)]
    colors_window_opened: Option<WindowId>,
}

fn main() {
    let main_window = WindowDesc::new(build_root_widget())
        .title("Welcome!")
        .menu(make_menu)
        .window_size((1000., 640.));
    // create the initial app state
    let initial_state = AppState {
        rect: Rect {
            x0: X0,
            y0: Y0,
            x1: X1,
            y1: Y1,
        },
        main_window_id: None,
        screenshot_id: None,
        color: None,
        colors_window_opened: None,
    };

    // start the application
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}



fn build_screenshot_widget() -> impl Widget<AppState> {
    let rectangle = LensWrap::new(SelectedRect::new(), AppState::rect);

    /*let label = Label::new(|data: &AppState, _env: &Env|
    format!("Resize and drag as you like.\n- Top Left: ({}, {})\n- Bottom Right: ({}, {})",
            data.rect.x0, data.rect.y0, data.rect.x1, data.rect.y1));*/

    let take_screenshot_button = TakeScreenshotButton::from_label(
        Label::new("Take Screenshot")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.),
    )
    .with_color(Color::rgb8(70, 250, 70).with_alpha(1.))
    .on_click(|ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
        ctx.submit_command(SAVE_SCREENSHOT.with((
            data.rect,
            data.main_window_id.expect("How did you open this window?"),
            data.screenshot_id.expect("How did you open this window?"),
            SCREENSHOT_PATH
        )).to(Target::Widget(ctx.widget_id())));
    });

    let close_button = ColoredButton::from_label(
        Label::new("Close")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.)
    )
    .with_color(Color::rgb8(250, 70, 70).with_alpha(1.))
    .on_click(|ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
        let main_id = data.main_window_id.expect("How did you open this window?");
        ctx.get_external_handle()
            .submit_command(sys_cmd::SHOW_WINDOW, (), main_id)
            .expect("Error sending the event");
        ctx.window().close();
    });

    let buttons_flex = Flex::row()
        .with_child(take_screenshot_button)
        .with_default_spacer()
        .with_child(close_button);

    /*let label_container = Container::new(label)
    .background(Color::BLACK.with_alpha(0.35));*/

    let zstack = ZStack::new(rectangle)
        //.with_child(label_container, Vec2::new(1.0, 1.0), Vec2::ZERO, UnitPoint::LEFT, Vec2::new(10.0, 0.0))
        .with_child(
            buttons_flex,
            Vec2::new(1.0, 1.0),
            Vec2::ZERO,
            UnitPoint::BOTTOM_RIGHT,
            Vec2::new(-100.0, -100.0),
        );

    zstack
}



fn build_root_widget() -> impl Widget<AppState> {
    let screenshot_widget_id = WidgetId::next();
    let take_screenshot_button = Button::from_label(Label::new("Take Screenshoot")).on_click(
        move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
            data.main_window_id = Some(ctx.window_id());
            data.screenshot_id = Some(screenshot_widget_id.clone());
            ctx.submit_command(sys_cmd::HIDE_WINDOW.to(Auto));
            ctx.new_window(
                WindowDesc::new(build_screenshot_widget())
                    .title(WINDOW_TITLE)
                    .set_always_on_top(true)
                    .transparent(true)
                    .resizable(false)
                    .show_titlebar(false)
                    .set_window_state(WindowState::Maximized)
            )
        },
    );
    let screenshot_image = IdentityWrapper::wrap(
        ScreenshotImage::new(ImageBuf::from_raw(
            Arc::<[u8]>::from(Vec::from([0,0,0,0]).as_slice()),
            ImageFormat::RgbaSeparate,
            1 as usize,
            1 as usize,
        )).on_added(|img, _ctx,_data:&AppState, _env|{
            if Path::new(SCREENSHOT_PATH).exists() {
                let screen_img = Reader::open(SCREENSHOT_PATH)
                    .expect("Can't open the screenshot!")
                    .decode()
                    .expect("Can't decode the screenshot");
                img.set_image_data(ImageBuf::from_raw(
                    Arc::<[u8]>::from(screen_img.as_bytes()),
                    ImageFormat::RgbaSeparate,
                    screen_img.width() as usize,
                    screen_img.height() as usize,
                ));
            }
        }),screenshot_widget_id);

    let zstack_id = WidgetId::next();
    let zstack = IdentityWrapper::wrap(CustomZStack::new(screenshot_image, screenshot_widget_id ), zstack_id);
    let spaced_zstack = Container::new(zstack).padding((10.0, 0.0));

    let buttons_bar = Flex::row()
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("⭕")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                // TODO: introduce a switch with meaningful paths containing different images!
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with("./src/images/icons/red-circle.png")
                        .to(Target::Widget(zstack_id)),
                );
            },
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("△")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with("./src/images/icons/triangle.png")
                        .to(Target::Widget(zstack_id)),
                );
            },
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("→")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with("./src/images/icons/red-arrow.png")
                        .to(Target::Widget(zstack_id)),
                );
            }
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("⎚")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with("./src/images/icons/highlighter.png")
                        .to(Target::Widget(zstack_id)),
                );
            }
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("Color")).on_click(
            move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                if data.colors_window_opened.is_none() {
                    let mut init_pos = ctx.to_screen(Point::new(1.,ctx.size().height-1.));
                    init_pos.y += 5.;
                    let wd = WindowDesc::new(build_colors_window(zstack_id))
                        .title(WINDOW_TITLE)
                        .set_always_on_top(true)
                        .show_titlebar(false)
                        .set_window_state(WindowState::Restored)
                        .window_size((1., 180.))
                        .set_position(init_pos)
                        .resizable(false)
                        .transparent(false);
                    data.colors_window_opened = Some(wd.id);
                    ctx.new_window(wd);
                } else {
                    ctx.get_external_handle().submit_command(
                        sys_cmd::CLOSE_WINDOW,
                        (),
                        Window(data.colors_window_opened.unwrap())
                    ).unwrap();
                    data.colors_window_opened = None;
                }
            }
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("Save")).on_click(
            move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                // TODO: use a meaningful name and extension
                ctx.submit_command(SAVE_OVER_IMG.with((
                    SCREENSHOT_PATH,
                    "./src/images/",
                    "screenshot",
                    image::ImageFormat::Png,
                )));
                if data.screenshot_id.is_some() {
                    ctx.submit_command(UPDATE_SCREENSHOT.with(String::from(SCREENSHOT_PATH)).to(data.screenshot_id.unwrap()));
                }
            },
        ))
        .with_default_spacer()
        .with_flex_child(Container::new(take_screenshot_button), 1.0);

    let mut flex = Flex::column();
    flex.add_default_spacer();
    flex.add_child(buttons_bar);
    flex.add_default_spacer();
    flex.set_must_fill_main_axis(true);
    flex.add_child(spaced_zstack);
    flex.add_default_spacer();
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
    //TODO: implement the menù
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

fn build_colors_window(zstack_id: WidgetId) -> impl Widget<AppState>{
    let none = create_color_button(None,zstack_id);
    let green = create_color_button(Some(Color::GREEN),zstack_id);
    let red = create_color_button(Some(Color::RED),zstack_id);
    let black = create_color_button(Some(Color::BLACK),zstack_id);
    let white = create_color_button(Some(Color::WHITE),zstack_id);
    let aqua = create_color_button(Some(Color::AQUA),zstack_id);
    let gray = create_color_button(Some(Color::GRAY),zstack_id);
    let blue = create_color_button(Some(Color::BLUE),zstack_id);
    let fuchsia = create_color_button(Some(Color::FUCHSIA),zstack_id);
    let lime = create_color_button(Some(Color::LIME),zstack_id);
    let maroon = create_color_button(Some(Color::MAROON),zstack_id);
    let navy = create_color_button(Some(Color::NAVY),zstack_id);
    let olive = create_color_button(Some(Color::OLIVE),zstack_id);
    let purple = create_color_button(Some(Color::PURPLE),zstack_id);
    let teal = create_color_button(Some(Color::TEAL),zstack_id);
    let yellow = create_color_button(Some(Color::YELLOW),zstack_id);
    let silver = create_color_button(Some(Color::SILVER),zstack_id);

    let flex = Flex::column().with_default_spacer().with_child(none).with_default_spacer()
        .with_child(Flex::row().with_default_spacer().with_child(green).with_default_spacer().with_child(red).with_default_spacer().with_child(black).with_default_spacer().with_child(white).with_default_spacer()).with_default_spacer()
        .with_child(Flex::row().with_default_spacer().with_child(aqua).with_default_spacer().with_child(gray).with_default_spacer().with_child(blue).with_default_spacer().with_child(fuchsia).with_default_spacer()).with_default_spacer()
        .with_child(Flex::row().with_default_spacer().with_child(lime).with_default_spacer().with_child(maroon).with_default_spacer().with_child(navy).with_default_spacer().with_child(olive).with_default_spacer()).with_default_spacer()
        .with_child(Flex::row().with_default_spacer().with_child(purple).with_default_spacer().with_child(teal).with_default_spacer().with_child(yellow).with_default_spacer().with_child(silver).with_default_spacer()).with_default_spacer()
        .background(Color::BLACK.with_alpha(0.3));

    Align::centered(flex)
}

fn create_color_button(color: Option<Color>,zstack_id: WidgetId) -> ControllerHost<ColoredButton<AppState>, Click<AppState>> {
    ColoredButton::from_label(Label::new(if color.is_some(){" "} else{"None"}))
        .with_color(color.unwrap_or(Color::SILVER.with_alpha(0.8)))
        .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env|{
            ctx.get_external_handle().submit_command(UPDATE_COLOR,color,Target::Widget(zstack_id)).unwrap();
            data.color = color;
            ctx.window().close();
        })
}