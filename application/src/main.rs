mod custom_widget;

use random_string::generate;
use std::path::Path;
use crate::custom_widget::{ColoredButton, CREATE_ZSTACK, CustomSlider, CustomZStack, OverImages, SAVE_OVER_IMG, SAVE_SCREENSHOT, ScreenshotImage, SelectedRect, SHOW_OVER_IMG, TakeScreenshotButton, UPDATE_BACK_IMG, UPDATE_COLOR};
use druid::piet::ImageFormat;
use druid::widget::{Align, Button, Click, Container, ControllerHost, Flex, IdentityWrapper, Label, LensWrap, Scroll, TextBox, ViewSwitcher, ZStack};
use druid::Target::{Auto, Window};
use druid::{commands as sys_cmd, AppLauncher, Color, Data, Env, EventCtx, FontDescriptor, FontFamily, ImageBuf, Lens, LocalizedString, Menu, Rect, Target, UnitPoint, Vec2, Widget, WidgetExt, WidgetId, WindowDesc, WindowId, WindowState, Point, TextAlignment};
use image::io::Reader;
use std::sync::Arc;

//TODO: Set the GUI error messages everywhere they need!
//TODO: Make the main page GUI beautiful
//TODO: Error handling.


const STARTING_IMG_PATH: &'static str = "./src/images/starting_img.png";

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Screen Grabbing Application");

const X0: f64 = 0.;
const Y0: f64 = 0.;
const X1: f64 = 1920.; // 1000.
const Y1: f64 = 1080.; // 500.

#[derive(Clone, Data, Lens)]
struct AppState {
    rect: Rect,
    alpha: f64,
    extension: String,
    name: String,
    #[data(ignore)]
    main_window_id: Option<WindowId>,
    #[data(ignore)]
    custom_zstack_id: Option<WidgetId>,
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
        .window_size((1000., 670.))
        .set_position((50.,20.));
    // create the initial app state
    let initial_state = AppState {
        rect: Rect {
            x0: X0,
            y0: Y0,
            x1: X1,
            y1: Y1,
        },
        alpha: 100.0,
        extension: "png".to_string(),
        name: "".to_string(),
        main_window_id: None,
        custom_zstack_id: None,
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
        let (base_path,name) = file_name(data.name.clone());
        ctx.submit_command(SAVE_SCREENSHOT.with((
            data.rect,
            data.main_window_id.expect("How did you open this window?"),
            data.custom_zstack_id.expect("How did you open this window?"),
            data.screenshot_id.expect("How did you open this window?"),
            base_path,
            name,
            image::ImageFormat::from_extension(data.extension.as_str()).unwrap(),
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
    let zstack_id = WidgetId::next();
    let take_screenshot_button = Button::from_label(Label::new("Take Screenshoot")).on_click(
        move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
            data.main_window_id = Some(ctx.window_id());
            data.custom_zstack_id = Some(zstack_id);
            data.screenshot_id = Some(screenshot_widget_id);
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
            1usize,
            1usize,
        )).on_added(move |img, ctx,_data:&AppState, _env|{
            if Path::new(STARTING_IMG_PATH).exists() {
                let screen_img = Arc::new(Reader::open(STARTING_IMG_PATH)
                    .expect("Can't open the screenshot!")
                    .decode()
                    .expect("Can't decode the screenshot"));
                img.set_image_data(ImageBuf::from_raw(
                    Arc::<[u8]>::from(screen_img.as_bytes()),
                    ImageFormat::RgbaSeparate,
                    screen_img.width() as usize,
                    screen_img.height() as usize,
                ));
                ctx.submit_command(UPDATE_BACK_IMG.with(screen_img).to(Target::Widget(zstack_id)));
            }
        }),screenshot_widget_id);


    let zstack = IdentityWrapper::wrap(CustomZStack::new(screenshot_image, screenshot_widget_id ), zstack_id)
        .on_added(move |_this,ctx,_data:&AppState, _env|{
            let mut args = Vec::<&'static str>::new();
            args.push("./src/images/icons/red-circle.png");
            args.push("./src/images/icons/triangle.png");
            args.push("./src/images/icons/red-arrow.png");
            args.push("./src/images/icons/highlighter.png");
            ctx.submit_command(CREATE_ZSTACK.with(args).to(Target::Widget(zstack_id)));

        });
    let spaced_zstack = Container::new(zstack).padding((10.0, 0.0));


    let name_selector = TextBox::new()
        .with_placeholder("file name").with_text_alignment(TextAlignment::Center)
        .lens(AppState::name).border(Color::BLACK,2.);

    let extension_selector = ViewSwitcher::new(
        |data: &AppState, _env| data.clone(),
        |selector, _data, _env| match selector.extension.as_str() {
            "png" => Box::new(Label::new("PNG").with_text_color(Color::BLACK)
                .border(Color::BLACK,2.)
                .on_click(|_,data: &mut AppState,_| data.extension = String::from(""))),
            "jpg" => Box::new(Label::new("JPG").with_text_color(Color::BLACK)
                .border(Color::BLACK,2.)
                .on_click(|_,data: &mut AppState,_| data.extension = String::from(""))),
            "gif" => Box::new(Label::new("GIF").with_text_color(Color::BLACK)
                .border(Color::BLACK,2.)
                .on_click(|_,data: &mut AppState,_| data.extension = String::from(""))),
            _ => Box::new(Scroll::new(
                Flex::column()
                    .with_child(Label::new("PNG").with_text_color(Color::BLACK)
                        .border(Color::BLACK,2.)
                        .on_click(|_,data: &mut AppState,_| data.extension = String::from("png")))
                    .with_child(Label::new("JPG").with_text_color(Color::BLACK)
                        .border(Color::BLACK,2.)
                        .on_click(|_,data: &mut AppState,_| data.extension = String::from("jpg")))
                    .with_child(Label::new("GIF").with_text_color(Color::BLACK)
                        .border(Color::BLACK,2.)
                        .on_click(|_,data: &mut AppState,_| data.extension = String::from("gif"))))
                .border(Color::BLACK,4.))
        },);

    let buttons_bar = Flex::row()
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("⭕")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with(OverImages::Circles)
                        .to(Target::Widget(zstack_id)),
                );
            },
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("△")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with(OverImages::Triangle)
                        .to(Target::Widget(zstack_id)),
                );
            },
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("→")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with(OverImages::Arrow)
                        .to(Target::Widget(zstack_id)),
                );
            }
        ))
        .with_default_spacer()
        .with_child(Button::from_label(Label::new("⎚")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with(OverImages::Highlighter)
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
                        .window_size((1., 250.))
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
                let (base_path,name) = file_name(data.name.clone());
                ctx.submit_command(SAVE_OVER_IMG.with((
                    base_path,
                    name,
                    image::ImageFormat::from_extension(data.extension.as_str()).unwrap(),
                )));
            },
        ))
        .with_default_spacer()
        .with_flex_child(Container::new(take_screenshot_button), 1.0);


    let scroll = Scroll::new(Flex::column()
        .with_default_spacer()
        .with_child(Flex::row().with_child(buttons_bar.with_default_spacer()
            .with_child(name_selector).with_default_spacer()
            .with_child(extension_selector)))
        .with_default_spacer()
        //flex.set_must_fill_main_axis(true);
        .with_child(spaced_zstack)).vertical();
        //flex.set_main_axis_alignment(MainAxisAlignment::Center);
    let layout = scroll.background(Color::WHITE).expand().padding(5.);
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

    let label = Label::new("Transparency:").with_text_color(Color::WHITE);
    let alpha_slider = CustomSlider::new().with_range(1.,100.).with_step(1.)
        .on_added(move |this,_ctx,_data,_env|{
            this.set_zstack_id(zstack_id);
        })
        .lens(AppState::alpha).padding(5.0);

    let flex = Flex::column().with_default_spacer().with_child(none).with_default_spacer()
        .with_child(Flex::row().with_default_spacer().with_child(green).with_default_spacer().with_child(red).with_default_spacer().with_child(black).with_default_spacer().with_child(white).with_default_spacer()).with_default_spacer()
        .with_child(Flex::row().with_default_spacer().with_child(aqua).with_default_spacer().with_child(gray).with_default_spacer().with_child(blue).with_default_spacer().with_child(fuchsia).with_default_spacer()).with_default_spacer()
        .with_child(Flex::row().with_default_spacer().with_child(lime).with_default_spacer().with_child(maroon).with_default_spacer().with_child(navy).with_default_spacer().with_child(olive).with_default_spacer()).with_default_spacer()
        .with_child(Flex::row().with_default_spacer().with_child(purple).with_default_spacer().with_child(teal).with_default_spacer().with_child(yellow).with_default_spacer().with_child(silver).with_default_spacer()).with_default_spacer()
        .with_default_spacer().with_child(Flex::row().with_child(label)).with_child(Flex::row().with_child(alpha_slider))
        .background(Color::BLACK.with_alpha(0.3));

    Align::centered(flex)
}

fn create_color_button(color: Option<Color>,zstack_id: WidgetId) -> ControllerHost<ColoredButton<AppState>, Click<AppState>> {
    ColoredButton::from_label(Label::new(if color.is_some(){" "} else{"None"}))
        .with_color(color.unwrap_or(Color::SILVER.with_alpha(0.8)))
        .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env|{
            ctx.get_external_handle().submit_command(UPDATE_COLOR,(color,None),Target::Widget(zstack_id)).unwrap();
            data.color = color;
            ctx.window().close();
        })
}

fn file_name(data_name: String) -> (&'static str,Box<str>){
    const BASE_PATH: &'static str = "./src/screenshots/";
    let charset = "1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let name = if data_name.as_str() == "" && data_name.chars().all(char::is_alphanumeric){
        let mut name = generate(16, charset);
        let mut path = format!("{}{}", BASE_PATH, name);
        while Path::new(path.as_str()).exists() {
            name = generate(12, charset);
            path = format!("{}{}", BASE_PATH, name);
        }
        format!("screenshot-{}", name).into_boxed_str()
    } else {
        data_name.into_boxed_str()
    };
    (BASE_PATH,name)
}