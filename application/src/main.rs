mod custom_widget;

use crate::custom_widget::{ColoredButton, CustomZStack, SelectedRect};
use druid::piet::ImageFormat;
use druid::widget::{
    Button, Container, Flex, IdentityWrapper, Image, Label, LensWrap, MainAxisAlignment, ZStack,
};
use druid::Target::Auto;
use druid::{
    commands as sys_cmd, AppLauncher, Color, Data, Env, EventCtx, FontDescriptor, FontFamily,
    ImageBuf, Lens, LocalizedString, Menu, Rect, Selector, Target, UnitPoint, Vec2, Widget,
    WidgetExt, WidgetId, WindowDesc, WindowId, WindowState,
};
use image::io::Reader;
use image::DynamicImage;
use std::sync::Arc;

//TODO: Must remove a lot of .clone() methods everywhere!
//TODO: Set the error messages everywhere they need!
//TODO: Make the main page GUI beautiful

pub const SHOW_OVER_IMG: Selector<&'static str> =
    Selector::new("Tell the ZStack to show the over_img, params: over_img path");
pub const SAVE_OVER_IMG: Selector<(DynamicImage ,&'static str, &str, image::ImageFormat)> = Selector::new("Tell the ZStack to save the modified screenshot, params: (Screenshot original img, Folder Path Where To Save, New File Name, Image Format)");
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

    let take_screenshot_button = ColoredButton::from_label(
        Label::new("Take Screenshot")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.),
    )
    .with_color(Color::rgb8(70, 250, 70).with_alpha(1.))
    .on_click(|ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
        ctx.submit_command(sys_cmd::HIDE_WINDOW.to(Auto));
        // TODO: Take the screenshot!
        ctx.submit_command(sys_cmd::SHOW_WINDOW.to(Auto));
    });

    let close_button = ColoredButton::from_label(
        Label::new("Close")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.),
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
    let take_screenshot_button = Button::from_label(Label::new("Take Screenshoot")).on_click(
        |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
            data.main_window_id = Some(ctx.window_id());
            ctx.submit_command(sys_cmd::HIDE_WINDOW.to(Auto));
            ctx.new_window(
                WindowDesc::new(build_screenshot_widget())
                    .title(WINDOW_TITLE)
                    .set_always_on_top(true)
                    .transparent(true)
                    .resizable(false)
                    .show_titlebar(false)
                    .set_window_state(WindowState::Maximized),
            )
        },
    );

    let screen_img = Reader::open("./src/images/Pic.jpg")
        .expect("Can't open the screenshot!")
        .decode()
        .expect("Can't decode the screenshot");
    let screenshot_image = Image::new(ImageBuf::from_raw(
        Arc::<[u8]>::from(screen_img.as_bytes()),
        ImageFormat::Rgb,
        screen_img.width() as usize,
        screen_img.height() as usize,
    ));

    let zstack_id = WidgetId::next();
    let zstack = IdentityWrapper::wrap(CustomZStack::new(screenshot_image), zstack_id);
    let spaced_zstack = Container::new(zstack).padding((10.0, 0.0));

    //TODO: make the image resizable and movable!
    let buttons_bar = Flex::row()
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("⭕")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                // TODO: introduce a switch with meaningful paths containing different images!
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with("./src/over_images/red-circle.png")
                        .to(Target::Widget(zstack_id)),
                );
            },
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("△")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with("./src/over_images/red-circle.png")
                        .to(Target::Widget(zstack_id)),
                );
            },
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("▫")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with("./src/over_images/red-circle.png")
                        .to(Target::Widget(zstack_id)),
                );
            },
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("Save")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                // TODO: use a meaningful name and extension
                ctx.submit_command(SAVE_OVER_IMG.with((
                    screen_img.clone(),
                    "./src/images/",
                    "modified_screen",
                    image::ImageFormat::Png,
                )));
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
