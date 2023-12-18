mod custom_widget;

use crate::custom_widget::{
    Alert, ColoredButton, CustomSlider, CustomZStack, OverImages, ScreenshotImage, SelectedRect,
    ShortcutKeys, StateShortcutKeys, TakeScreenshotButton, CREATE_ZSTACK, SAVE_OVER_IMG,
    SAVE_SCREENSHOT, SHORTCUT_KEYS, SHOW_OVER_IMG, UPDATE_BACK_IMG, UPDATE_COLOR,
};
use druid::commands::SHOW_ABOUT;
use druid::keyboard_types::Code;
use druid::piet::ImageFormat;
use druid::widget::{
    Align, Button, Click, Container, ControllerHost, CrossAxisAlignment, Either, Flex,
    IdentityWrapper, Label, LensWrap, MainAxisAlignment, Scroll, Stepper, TextBox, ViewSwitcher,
    ZStack, LineBreaking, Image,
};
use druid::Target::{Auto, Window};
use druid::{
    commands as sys_cmd, commands, AppDelegate, AppLauncher, Color, Command, Data, DelegateCtx,
    Env, Event, EventCtx, FileDialogOptions, FontDescriptor, FontFamily, Handled, ImageBuf, Lens,
    LocalizedString, Menu, MenuItem, Point, Rect, Screen, Size, Target, TextAlignment, UnitPoint,
    Vec2, Widget, WidgetExt, WidgetId, WindowDesc, WindowId, WindowState,
};
use image::io::Reader;
use random_string::generate;
use std::collections::HashSet;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

//TODO: Set the GUI error messages everywhere they need!
//TODO: Make the main page GUI beautiful
//TODO: Error handling.

const STARTING_IMG_PATH: &'static str = "./src/images/starting_img.png";

lazy_static::lazy_static! {
static ref SCREENSHOT_WIDGET_ID: WidgetId = WidgetId::next();
static ref ZSTACK_ID: WidgetId = WidgetId::next();
}

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Screen Grabbing Application");
const X0: f64 = 0.;
const Y0: f64 = 0.;
const X1: f64 = 500.;
const Y1: f64 = 500.;
#[derive(Clone, PartialEq)]
enum ImageModified {
    NotSavable,
    Savable,
}
#[derive(Clone, PartialEq)]
enum State {
    Start,
    ScreenTaken(ImageModified),
}

#[derive(Clone, Data, Lens)]
struct AppState {
    rect: Rect,
    alpha: f64,
    extension: String,
    name: String,
    delay: f64,
    screen: String,
    #[data(eq)]
    state: State,
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
    #[data(ignore)]
    base_path: String,
    alert: Alert,
    shortcut_keys: ShortcutKeys,
    #[data(ignore)]
    text_field_zstack: bool,
    text_field: String,
}

fn main() {
    let main_window = WindowDesc::new(build_root_widget())
        .title("Welcome!")
        .menu(make_menu)
        .window_size((1000., 670.))
        .set_position((50., 20.));

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
        delay: 0.0,
        screen: "0".to_string(),
        main_window_id: None,
        custom_zstack_id: Some(*ZSTACK_ID),
        screenshot_id: Some(*SCREENSHOT_WIDGET_ID),
        color: None,
        colors_window_opened: None,
        state: State::Start,
        base_path: "./src/screenshots/".to_string(),
        alert: Alert {
            alert_visible: false,
            alert_message: "".to_string(),
        },
        shortcut_keys: ShortcutKeys {
            favorite_hot_keys: HashSet::<Code>::from([Code::KeyB, Code::KeyA]),
            pressed_hot_keys: HashSet::new(),
            state: StateShortcutKeys::NotBusy,
        },
        text_field_zstack: true,
        text_field: "".to_string(),
    };

    let delegate = Delegate;

    // start the application
    AppLauncher::with_window(main_window)
        .delegate(delegate)
        .launch(initial_state)
        .expect("Failed to launch application");
}
struct Delegate;

impl AppDelegate<AppState> for Delegate {
    fn event(
        &mut self,
        ctx: &mut DelegateCtx,
        window_id: WindowId,
        event: Event,
        data: &mut AppState,
        _env: &Env,
    ) -> Option<Event> {
        match event.clone() {
            Event::KeyDown(key) => {
                data.shortcut_keys.pressed_hot_keys.insert(key.code);
            }
            Event::KeyUp(_) => {
                if data.shortcut_keys.state == StateShortcutKeys::SetFavoriteShortcut {
                    // check if there is a not available combination
                    if data.shortcut_keys.pressed_hot_keys
                        == HashSet::from([Code::ControlLeft, Code::KeyC])
                        || data.shortcut_keys.pressed_hot_keys == HashSet::from([Code::Escape])
                    {
                        // ctrl + c : this is reserved for the copy shortcut, Esc is reserved to close the screen window
                        data.shortcut_keys.state = StateShortcutKeys::ShortcutNotAvailable;
                    } else {
                        data.shortcut_keys.favorite_hot_keys =
                            data.shortcut_keys.pressed_hot_keys.clone();
                        data.shortcut_keys.state = StateShortcutKeys::NotBusy;
                        data.shortcut_keys.pressed_hot_keys = HashSet::new();
                    }
                } else if data.shortcut_keys.pressed_hot_keys == HashSet::from([Code::Escape]) {
                    // Key Escape has been pressed
                    if let Some(main_id) = data.main_window_id {
                        data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map
                        data.shortcut_keys.state = StateShortcutKeys::NotBusy; // it has finished its job

                        ctx.get_external_handle()
                            .submit_command(sys_cmd::SHOW_WINDOW, (), main_id)
                            .expect("Error sending the event");
                    }
                    ctx.submit_command(sys_cmd::CLOSE_WINDOW.to(Target::Window(window_id)));
                } else if data.shortcut_keys.pressed_hot_keys.len()
                    == data.shortcut_keys.favorite_hot_keys.len()
                    && data.shortcut_keys.pressed_hot_keys == data.shortcut_keys.favorite_hot_keys
                    && (data.shortcut_keys.state == StateShortcutKeys::NotBusy
                        || data.shortcut_keys.state == StateShortcutKeys::ShortcutNotAvailable)
                {
                    data.shortcut_keys.state = StateShortcutKeys::StartScreenGrabber; // started to capture the screen
                    data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map

                    // start the screen grabber
                    data.main_window_id = Some(window_id);
                    ctx.submit_command(sys_cmd::HIDE_WINDOW.to(Target::Window(window_id)));
                    let monitors = Screen::get_monitors();
                    let index: usize =
                        std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap();
                    let monitor = monitors.get(index).unwrap();
                    ctx.new_window(
                        WindowDesc::new(build_screenshot_widget(index))
                            .title(WINDOW_TITLE)
                            .set_always_on_top(true)
                            .transparent(true)
                            .resizable(false)
                            .show_titlebar(false)
                            .set_window_state(WindowState::Maximized)
                            .set_position(monitor.virtual_rect().origin()),
                    );
                    ctx.submit_command(
                        SHOW_OVER_IMG
                            .with((OverImages::Remove, None))
                            .to(Target::Widget(WidgetId::next())),
                    );
                    data.state = State::ScreenTaken(ImageModified::NotSavable);
                }

                data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map
            }
            _ => {}
        }

        Some(event)
    }

    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppState,
        _env: &Env,
    ) -> Handled {
        // this gets the open file command when a directory has been selectioned
        if let Some(file_info) = cmd.get(commands::OPEN_FILE) {
            data.base_path = file_info
                .path
                .to_string_lossy()
                .to_string()
                .replace("\\", "/");
            data.base_path.push('/');
            return Handled::Yes;
        } else if cmd.is(SHOW_ABOUT){
            let monitors = Screen::get_monitors();
            let index: usize =
                std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap();
            let monitor = monitors.get(index).unwrap();
            let window_aboutus = WindowDesc::new(build_about_us_widget())
                .title(LocalizedString::new("About Us"))
                .set_always_on_top(false)
                .transparent(true)
                .resizable(true)
                .show_titlebar(true)
                .window_size((400., 200.))
                .set_position(monitor.virtual_rect().origin())
                .with_min_size(Size::new(450., 300.));

            ctx.new_window(window_aboutus);
        } else if cmd.is(SHORTCUT_KEYS) {
            let monitors = Screen::get_monitors();
            let index: usize =
                std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap();
            let monitor = monitors.get(index).unwrap();

            let window_shortcut = WindowDesc::new(build_shortcut_keys_widget())
                .title(LocalizedString::new("Shortcut Keys Configuration"))
                .set_always_on_top(false)
                .transparent(true)
                .resizable(true)
                .show_titlebar(true)
                .window_size((400., 200.))
                .set_position(monitor.virtual_rect().origin())
                .with_min_size(Size::new(450., 300.));

            ctx.new_window(window_shortcut);
        }
        Handled::No
    }
}
fn build_about_us_widget() -> impl Widget<AppState> {

    let flex_default = Flex::row()
        .with_child(Label::new("This application was brought to you by Elio Magliari, Pietro Bertorelle and Francesco Abate")
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.)
            .with_text_alignment(TextAlignment::Center)
            .fix_size(200.,200.)
            .center()
            .align_vertical(UnitPoint::TOP)
            .align_horizontal(UnitPoint::CENTER))
        .center()
        .background(Color::WHITE);
        
        
    flex_default
}
fn build_shortcut_keys_widget() -> impl Widget<AppState> {
    let message_comb_not_available = Label::new("Combination Not Available!")
        .with_text_color(Color::RED)
        .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
        .with_text_size(15.)
        .padding(10.)
        .border(Color::RED, 0.7)
        .rounded(5.)
        .align_horizontal(UnitPoint::CENTER);

    let flex_default = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Center)
        .with_child(
            Flex::column()
                .with_child(Either::new(
                    |data: &AppState, _env| {
                        data.shortcut_keys.state == StateShortcutKeys::ShortcutNotAvailable
                    },
                    message_comb_not_available,
                    Label::new(""),
                ))
                .with_default_spacer()
                .with_default_spacer()
                .with_child(
                    Label::new("Favorite Shortcut Keys:")
                        .with_text_color(Color::BLACK)
                        .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                        .with_text_size(20.)
                        .align_horizontal(UnitPoint::CENTER),
                )
                .with_default_spacer()
                .with_child(
                    Label::dynamic(|data: &AppState, _env| {
                        data.shortcut_keys
                            .favorite_hot_keys
                            .iter()
                            .map(|code| code.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .with_line_break_mode(druid::widget::LineBreaking::WordWrap)
                    .with_text_color(Color::BLACK)
                    .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                    .with_text_size(20.)
                    .padding(10.)
                    .border(Color::BLACK, 1.)
                    .rounded(7.)
                    .fix_width(350.)
                    .align_horizontal(UnitPoint::CENTER)
                    .center(),
                )
                .with_default_spacer()
                .with_default_spacer()
                .with_child(
                    Button::new("Change Shortcut")
                        .background(Color::rgb(0.0, 0.5, 0.8))
                        .rounded(5.)
                        .align_horizontal(UnitPoint::CENTER)
                        .on_click(|_ctx, data: &mut AppState, _| {
                            data.shortcut_keys.state = StateShortcutKeys::SetFavoriteShortcut;
                        }),
                ),
        );
    let container_default = Container::new(flex_default)
        .background(Color::WHITE)
        .rounded(10.0)
        .padding(10.0);

    let flex_setting_favorite_shortcut = Flex::row().with_child(
        Label::new("Enter some keys...")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.)
            .align_horizontal(UnitPoint::CENTER)
            .center(),
    );
    let container_setting_favorite_shortcut = Container::new(flex_setting_favorite_shortcut)
        .background(Color::WHITE)
        .rounded(10.0)
        .padding(10.0);

    Either::new(
        |data: &AppState, _env| data.shortcut_keys.state != StateShortcutKeys::SetFavoriteShortcut,
        container_default,
        container_setting_favorite_shortcut,
    )
}

fn alert_widget() -> impl Widget<AppState> {
    let alert = Flex::row()
        .with_child(
            Label::new(|data: &AppState, _env: &_| data.alert.alert_message.clone())
                .with_text_color(Color::WHITE)
                .padding(10.0),
        )
        .with_default_spacer()
        .with_child(
            Button::new("Close")
                .on_click(|ctx, data: &mut AppState, _| {
                    data.alert.hide_alert();
                    ctx.request_update();
                })
                .fix_height(30.0)
                .fix_width(60.0),
        )
        .with_default_spacer()
        .background(Color::rgb8(0, 120, 200))
        .border(Color::rgb8(0, 100, 160), 2.0)
        .fix_width(800.0)
        .fix_height(40.0)
        .center();
    alert
}
fn crop_screenshot_widget(monitor: usize) -> impl Widget<AppState>{
    //let spaced_zstack = Container::new(zstack);//.padding((10.0, 0.0));
    let screenshot_image = IdentityWrapper::wrap(
        ScreenshotImage::new(ImageBuf::from_raw(
            Arc::<[u8]>::from(Vec::from([0, 0, 0, 0]).as_slice()),
            ImageFormat::RgbaSeparate,
            1usize,
            1usize,
        ))
        .on_added(move |img, ctx, data: &AppState, _env| {
            if Path::new(STARTING_IMG_PATH).exists() {
                let screen_img = Arc::new(
                    Reader::open(STARTING_IMG_PATH)
                        .expect("Can't open the screenshot!")
                        .decode()
                        .expect("Can't decode the screenshot"),
                );
                img.set_image_data(ImageBuf::from_raw(
                    Arc::<[u8]>::from(screen_img.as_bytes()),
                    ImageFormat::RgbaSeparate,
                    screen_img.width() as usize,
                    screen_img.height() as usize,
                ));
                ctx.submit_command(
                    UPDATE_BACK_IMG
                        .with(screen_img)
                        .to(Target::Widget(data.custom_zstack_id.unwrap())),
                );
            }
        }),
        *SCREENSHOT_WIDGET_ID,
    );

    let zstack = IdentityWrapper::wrap(
        CustomZStack::new(screenshot_image, *SCREENSHOT_WIDGET_ID),
        *ZSTACK_ID,
    );

    let rectangle = LensWrap::new(SelectedRect::new(monitor), AppState::rect);

    let crop_screenshot_button = TakeScreenshotButton::from_label(
        Label::new("Crop Screenshot")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.),
    )
    .with_color(Color::rgb8(70, 250, 70).with_alpha(1.))
    .on_click(|ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
        let (base_path, name) = file_name(data.name.clone(), data.base_path.clone());
        data.alert
            .show_alert("The cropped version of the image has been saved on the disk!"); // TODO: it should be moved after saving the image

        ctx.submit_command(
            SAVE_SCREENSHOT
                .with((
                    data.rect,
                    data.main_window_id.expect("How did you open this window?"),
                    data.custom_zstack_id
                        .expect("How did you open this window?"),
                    data.screenshot_id.expect("How did you open this window?"),
                    base_path,
                    name,
                    image::ImageFormat::from_extension(data.extension.trim_start_matches("."))
                        .unwrap(),
                    data.delay as u64,
                    std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap(),
                ))
                .to(Target::Widget(ctx.widget_id())),
        );
    });

    let delay_value = Label::dynamic(|data: &AppState, _env| data.delay.to_string())
        .with_text_color(Color::WHITE)
        .background(Color::BLACK.with_alpha(0.55));
    let delay_stepper = Stepper::new()
        .with_range(0.0, 20.0)
        .with_step(1.0)
        .with_wraparound(true)
        .lens(AppState::delay);

    let close_button = ColoredButton::from_label(
        Label::new("Close")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.),
    )
    .with_color(Color::rgb8(250, 70, 70).with_alpha(1.))
    .on_click(|ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
        let main_id = data.main_window_id.expect("How did you open this window?");

        data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map
        data.shortcut_keys.state = StateShortcutKeys::NotBusy; // it has finished its job

        data.state = State::Start;
        ctx.get_external_handle()
            .submit_command(sys_cmd::SHOW_WINDOW, (), main_id)
            .expect("Error sending the event");
        ctx.window().close();
    });

    let buttons_flex = Flex::row()
        .with_child(crop_screenshot_button)
        .with_default_spacer()
        .with_child(delay_value)
        .with_child(delay_stepper)
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
fn build_screenshot_widget(monitor: usize) -> impl Widget<AppState> {
    let rectangle = LensWrap::new(SelectedRect::new(monitor), AppState::rect);

    let take_screenshot_button = TakeScreenshotButton::from_label(
        Label::new("Take Screenshot")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.),
    )
    .with_color(Color::rgb8(70, 250, 70).with_alpha(1.))
    .on_click(|ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
        let (base_path, name) = file_name(data.name.clone(), data.base_path.clone());
        data.alert
            .show_alert("The image has been saved on the disk!"); // TODO: it should be moved after saving the image

        data.shortcut_keys.state = StateShortcutKeys::NotBusy; // reset of shortcut state

        ctx.submit_command(
            SAVE_SCREENSHOT
                .with((
                    data.rect,
                    data.main_window_id.expect("How did you open this window?"),
                    data.custom_zstack_id
                        .expect("How did you open this window?"),
                    data.screenshot_id.expect("How did you open this window?"),
                    base_path,
                    name,
                    image::ImageFormat::from_extension(data.extension.trim_start_matches("."))
                        .unwrap(),
                    data.delay as u64,
                    std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap(),
                ))
                .to(Target::Widget(ctx.widget_id())),
        );
        data.state = State::ScreenTaken(ImageModified::NotSavable);
    });

    let delay_value = Label::dynamic(|data: &AppState, _env| data.delay.to_string())
        .with_text_color(Color::WHITE)
        .background(Color::BLACK.with_alpha(0.55));
    let delay_stepper = Stepper::new()
        .with_range(0.0, 20.0)
        .with_step(1.0)
        .with_wraparound(true)
        .lens(AppState::delay);

    let close_button = ColoredButton::from_label(
        Label::new("Close")
            .with_text_color(Color::BLACK)
            .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
            .with_text_size(20.),
    )
    .with_color(Color::rgb8(250, 70, 70).with_alpha(1.))
    .on_click(|ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
        let main_id = data.main_window_id.expect("How did you open this window?");

        data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map
        data.shortcut_keys.state = StateShortcutKeys::NotBusy; // it has finished its job

        data.state = State::Start;
        ctx.get_external_handle()
            .submit_command(sys_cmd::SHOW_WINDOW, (), main_id)
            .expect("Error sending the event");
        ctx.window().close();
    });

    let buttons_flex = Flex::row()
        .with_child(take_screenshot_button)
        .with_default_spacer()
        .with_child(delay_value)
        .with_child(delay_stepper)
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
    //let *SCREENSHOT_WIDGET_ID = WidgetId::next();
    //let zstack_id = WidgetId::next();

    let take_screenshot_button =
        ColoredButton::from_label(Label::new(|data: &AppState, _env: &_| match data.state {
            State::Start => "Take Screenshot",
            State::ScreenTaken(_) => "New Screenshot",
        }))
        .with_color(Color::rgb(160. / 256., 0., 0.))
        .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
            data.main_window_id = Some(ctx.window_id());
            data.custom_zstack_id = Some(*ZSTACK_ID);
            data.screenshot_id = Some(*SCREENSHOT_WIDGET_ID);
            ctx.submit_command(sys_cmd::HIDE_WINDOW.to(Auto));
            let monitors = Screen::get_monitors();
            let index: usize =
                std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap();
            let monitor = monitors.get(index).unwrap();
            ctx.new_window(
                WindowDesc::new(build_screenshot_widget(index))
                    .title(WINDOW_TITLE)
                    .set_always_on_top(true)
                    .transparent(true)
                    .resizable(false)
                    .show_titlebar(false)
                    .set_window_state(WindowState::Maximized)
                    .set_position(monitor.virtual_rect().origin()),
            );
            ctx.submit_command(
                SHOW_OVER_IMG
                    .with((OverImages::Remove, None))
                    .to(Target::Widget(data.custom_zstack_id.unwrap())),
            );
            data.state = State::ScreenTaken(ImageModified::NotSavable);
        });

    
    let screenshot_image = IdentityWrapper::wrap(
        ScreenshotImage::new(ImageBuf::from_raw(
            Arc::<[u8]>::from(Vec::from([0, 0, 0, 0]).as_slice()),
            ImageFormat::RgbaSeparate,
            1usize,
            1usize,
        ))
        .on_added(move |img, ctx, data: &AppState, _env| {
            if Path::new(STARTING_IMG_PATH).exists() {
                let screen_img = Arc::new(
                    Reader::open(STARTING_IMG_PATH)
                        .expect("Can't open the screenshot!")
                        .decode()
                        .expect("Can't decode the screenshot"),
                );
                img.set_image_data(ImageBuf::from_raw(
                    Arc::<[u8]>::from(screen_img.as_bytes()),
                    ImageFormat::RgbaSeparate,
                    screen_img.width() as usize,
                    screen_img.height() as usize,
                ));
                ctx.submit_command(
                    UPDATE_BACK_IMG
                        .with(screen_img)
                        .to(Target::Widget(data.custom_zstack_id.unwrap())),
                );
            }
        }),
        *SCREENSHOT_WIDGET_ID,
    );

    

    let zstack = IdentityWrapper::wrap(
        CustomZStack::new(screenshot_image, *SCREENSHOT_WIDGET_ID),
        *ZSTACK_ID,
    )
    .on_added(move |_this, ctx, _data: &AppState, _env| {
        let mut args = Vec::<&'static str>::new();
        args.push("./src/images/icons/red-circle.png");
        args.push("./src/images/icons/triangle.png");
        args.push("./src/images/icons/red-arrow.png");
        args.push("./src/images/icons/highlighter.png");
        ctx.submit_command(CREATE_ZSTACK.with(args).to(Target::Widget(*ZSTACK_ID)));
    });
    let spaced_zstack = Container::new(zstack).padding((10.0, 0.0));

    let crop_screenshot_button = Either::new(|data: &AppState, _env | data.state == State::ScreenTaken(ImageModified::NotSavable),
    ColoredButton::from_label(Label::new("Crop ScreenShot"))
    .with_color(Color::rgb(0., 0., 255.))
    .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
        data.main_window_id = Some(ctx.window_id());
        data.custom_zstack_id = Some(*ZSTACK_ID);
        data.screenshot_id = Some(*SCREENSHOT_WIDGET_ID);
        let monitors = Screen::get_monitors();
        let screen_img = Arc::new(
                    Reader::open(STARTING_IMG_PATH)
                        .expect("Can't open the screenshot!")
                        .decode()
                        .expect("Can't decode the screenshot"),
        );
        let immagine = Image::new(ImageBuf::from_raw(
        Arc::<[u8]>::from(screen_img.as_bytes()),
        ImageFormat::RgbaSeparate,
        screen_img.width() as usize,
        screen_img.height() as usize,
        ));
        ctx.submit_command(UPDATE_BACK_IMG
            .with(screen_img)
            .to(Target::Widget(data.custom_zstack_id.unwrap())),);
        
        let index: usize =
            std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap();
        let monitor = monitors.get(index).unwrap();
        ctx.new_window(
            WindowDesc::new(screen_img)//crop_screenshot_widget(index))
                .title(WINDOW_TITLE)
                .set_always_on_top(true)
                .transparent(true)
                .resizable(false)
                .show_titlebar(false)
                .set_window_state(WindowState::Maximized)
                .set_position(monitor.virtual_rect().origin()),
        );
        ctx.submit_command(
            SHOW_OVER_IMG
                .with((OverImages::Remove, None))
                .to(Target::Widget(data.custom_zstack_id.unwrap())),
        );
    }),
    Label::new(""),
    );

    let name_selector = Either::new(
        |data: &AppState, _env| {
            data.state == State::ScreenTaken(ImageModified::Savable) || data.state == State::Start
        },
        TextBox::new()
            .with_placeholder("file name")
            .with_text_alignment(TextAlignment::Center)
            .lens(AppState::name),
        Label::new(""),
    );

    let extension_selector = Either::new(
        |data: &AppState, _env| {
            data.state == State::ScreenTaken(ImageModified::Savable) || data.state == State::Start
        },
        ViewSwitcher::new(
            |data: &AppState, _env| data.clone(),
            |selector, _data, _env| match selector.extension.as_str() {
                "png" => Box::new(
                    Label::new("PNG ▼")
                        .with_text_color(Color::BLACK)
                        .border(Color::BLACK, 2.)
                        .on_click(|_, data: &mut AppState, _| {
                            data.extension = String::from(".png")
                        }),
                ),
                "jpg" => Box::new(
                    Label::new("JPG ▼")
                        .with_text_color(Color::BLACK)
                        .border(Color::BLACK, 2.)
                        .on_click(|_, data: &mut AppState, _| {
                            data.extension = String::from(".jpg")
                        }),
                ),
                "gif" => Box::new(
                    Label::new("GIF ▼")
                        .with_text_color(Color::BLACK)
                        .border(Color::BLACK, 2.)
                        .on_click(|_, data: &mut AppState, _| {
                            data.extension = String::from(".gif")
                        }),
                ),
                _ => {
                    let text_alpha = match selector.extension.as_str() {
                        ".png" => (1f64, 0.4f64, 0.4f64),
                        ".jpg" => (0.4f64, 1f64, 0.4f64),
                        ".gif" => (0.4f64, 0.4f64, 1f64),
                        _ => panic!(),
                    };
                    Box::new(
                        Scroll::new(
                            Flex::column()
                                .with_child(
                                    Label::new("PNG")
                                        .with_text_color(Color::BLACK.with_alpha(text_alpha.0))
                                        .border(Color::BLACK.with_alpha(text_alpha.0), 2.)
                                        .fix_size(40., 27.)
                                        .on_click(|_, data: &mut AppState, _| {
                                            data.extension = String::from("png")
                                        }),
                                )
                                .with_child(
                                    Label::new("JPG")
                                        .with_text_color(Color::BLACK.with_alpha(text_alpha.1))
                                        .border(Color::BLACK.with_alpha(text_alpha.1), 2.)
                                        .fix_size(40., 27.)
                                        .on_click(|_, data: &mut AppState, _| {
                                            data.extension = String::from("jpg")
                                        }),
                                )
                                .with_child(
                                    Label::new("GIF")
                                        .with_text_color(Color::BLACK.with_alpha(text_alpha.2))
                                        .border(Color::BLACK.with_alpha(text_alpha.2), 2.)
                                        .fix_size(40., 27.)
                                        .on_click(|_, data: &mut AppState, _| {
                                            data.extension = String::from("gif")
                                        }),
                                ),
                        )
                        .border(Color::BLACK.with_alpha(0.6), 4.),
                    )
                }
            },
        ),
        Label::new(""),
    );

    let screen_selector = ViewSwitcher::new(
        |data: &AppState, _env| data.clone(),
        |selector, data: &AppState, _env| {
            if selector.screen.chars().all(char::is_numeric) {
                Box::new(
                    Label::new(format!("{} ▼", data.screen))
                        .with_text_color(Color::BLACK)
                        .border(Color::BLACK, 2.)
                        .on_click(|_, data: &mut AppState, _| {
                            data.screen = format!(".{}", data.screen)
                        }),
                )
            } else {
                let screens = Screen::get_monitors();
                let dim = screens.len();
                let number: u8 =
                    std::str::FromStr::from_str(selector.screen.trim_start_matches(".")).unwrap();
                let mut flex = Flex::column();
                for i in 0..dim {
                    let color = if i == number as usize {
                        Color::BLACK.with_alpha(1.)
                    } else {
                        Color::BLACK.with_alpha(0.4)
                    };
                    flex.add_child(
                        Label::new(format!("{} ◀", i))
                            .with_text_color(color)
                            .border(color, 2.)
                            .on_click(move |_, data: &mut AppState, _| {
                                data.screen = format!("{}", i)
                            }),
                    );
                }

                Box::new(Scroll::new(flex).border(Color::BLACK.with_alpha(0.6), 4.))
            }
        },
    );

    let circle_button = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::NotSavable),
        Button::from_label(Label::new("⭕")).on_click(
            move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with((OverImages::Circles, None))
                        .to(Target::Widget(*ZSTACK_ID)),
                );
                data.state = State::ScreenTaken(ImageModified::Savable);
            },
        ),
        Label::new(""),
    );

    let triangle_button = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::NotSavable),
        Button::from_label(Label::new("△")).on_click(
            move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with((OverImages::Triangle, None))
                        .to(Target::Widget(*ZSTACK_ID)),
                );
                data.state = State::ScreenTaken(ImageModified::Savable);
            },
        ),
        Label::new(""),
    );
    let arrow_button = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::NotSavable),
        Button::from_label(Label::new("→")).on_click(
            move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with((OverImages::Arrow, None))
                        .to(Target::Widget(*ZSTACK_ID)),
                );
                data.state = State::ScreenTaken(ImageModified::Savable);
            },
        ),
        Label::new(""),
    );
    let highlighter_button = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::NotSavable),
        Button::from_label(Label::new("⎚")).on_click(
            move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with((OverImages::Highlighter, None))
                        .to(Target::Widget(*ZSTACK_ID)),
                );
                data.state = State::ScreenTaken(ImageModified::Savable);
            },
        ),
        Label::new(""),
    );

    let text_field = Either::new(
        |data: &AppState, _| data.text_field_zstack == true && data.state == State::ScreenTaken(ImageModified::NotSavable),
        Flex::row()
            .with_child(druid::widget::TextBox::new()
                .with_placeholder("Insert a text...")
                .lens(AppState::text_field))
            .with_default_spacer()
            .with_child(Button::from_label(Label::new("Save")).on_click(
                move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                    ctx.submit_command(
                        SHOW_OVER_IMG
                            .with((OverImages::Text, Some(data.text_field.clone())))
                            .to(Target::Widget(*ZSTACK_ID)),
                    );
                    data.text_field = "".to_string();
                    data.text_field_zstack = false;
                    data.state = State::ScreenTaken(ImageModified::Savable);
                    ctx.request_update();
                },
            ))
            .with_default_spacer()
            .with_child(Button::from_label(Label::new("Cancel")).on_click(
                move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                    data.text_field = "".to_string();
                    data.text_field_zstack = false;
                    ctx.request_update();
                },
            )),
        Label::new(""),
    );

    let text_button = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::NotSavable),
        Button::from_label(Label::new("Text")).on_click(
            move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                data.text_field_zstack = true;
                ctx.submit_command(sys_cmd::SHOW_ALL);
            },
        ),
        Label::new(""),
    );

    let colors_button = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::NotSavable),
        ColoredButton::from_label(Label::new("Color"))
            .with_color(Color::PURPLE)
            .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                if data.colors_window_opened.is_none() {
                    let mut init_pos = ctx.to_screen(Point::new(1., ctx.size().height - 1.));
                    init_pos.y += 5.;
                    let wd = WindowDesc::new(build_colors_window(*ZSTACK_ID))
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
                    ctx.get_external_handle()
                        .submit_command(
                            sys_cmd::CLOSE_WINDOW,
                            (),
                            Window(data.colors_window_opened.unwrap()),
                        )
                        .unwrap();
                    data.colors_window_opened = None;
                }
            }),
        Label::new(""),
    );

    let remove_over_img = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::Savable),
        Button::from_label(Label::new("❌")).on_click(
            move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    SHOW_OVER_IMG
                        .with((OverImages::Remove, None))
                        .to(Target::Widget(*ZSTACK_ID)),
                );
                data.state = State::ScreenTaken(ImageModified::NotSavable);
            },
        ),
        Label::new(""),
    );
    let path_button = Either::new(
        |data: &AppState, _env| {
            data.state == State::ScreenTaken(ImageModified::Savable) || data.state == State::Start
        },
        Button::from_label(Label::new("path")).on_click(
            move |ctx: &mut EventCtx, _data: &mut AppState, _env: &Env| {
                ctx.submit_command(
                    commands::SHOW_OPEN_PANEL
                        .with(FileDialogOptions::default().select_directories()),
                )
            },
        ),
        Label::new(""),
    );
    let save_button = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::Savable),
        ColoredButton::from_label(Label::new("Save"))
            .with_color(Color::rgb(0., 120. / 256., 0.))
            .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                let (base_path, name) = file_name(data.name.clone(), data.base_path.clone());
                data.alert
                    .show_alert("The image has been saved on the disk!"); // TODO: it should be moved after saving the image
                ctx.submit_command(
                    SAVE_OVER_IMG.with((
                        base_path,
                        name,
                        image::ImageFormat::from_extension(data.extension.trim_start_matches("."))
                            .unwrap(),
                    )),
                );
                data.state = State::ScreenTaken(ImageModified::NotSavable);
            }),
        Label::new(""),
    );

    let file_name_label = Label::new(|data: &AppState, _env: &_| match data.state {
        State::Start => "Screenshot:",
        State::ScreenTaken(ImageModified::Savable) => "Modified Image:",
        State::ScreenTaken(ImageModified::NotSavable) => "",
    })
    .with_text_color(Color::BLACK.with_alpha(0.85));

    let buttons_bar = Flex::row()
        .with_default_spacer()
        .with_child(remove_over_img)
        .with_default_spacer()
        .with_child(circle_button)
        .with_default_spacer()
        .with_child(triangle_button)
        .with_default_spacer()
        .with_child(arrow_button)
        .with_default_spacer()
        .with_child(highlighter_button)
        .with_default_spacer()
        .with_child(text_button)
        .with_default_spacer()
        .with_child(colors_button)
        .with_spacer(40.)
        .with_child(text_field)
        .with_default_spacer()
        .with_child(file_name_label)
        .with_default_spacer()
        .with_child(path_button)
        .with_default_spacer()
        .with_child(
            Label::new(|data: &AppState, _env: &_| {
                if data.state == State::ScreenTaken(ImageModified::Savable)
                    || data.state == State::Start
                {
                    "/"
                } else {
                    ""
                }
            })
            .with_text_color(Color::BLACK),
        )
        .with_default_spacer()
        .with_child(name_selector)
        .with_default_spacer()
        .with_child(
            Label::new(|data: &AppState, _env: &_| {
                if data.state == State::ScreenTaken(ImageModified::Savable)
                    || data.state == State::Start
                {
                    "."
                } else {
                    ""
                }
            })
            .with_text_color(Color::BLACK),
        )
        .with_default_spacer()
        .with_child(extension_selector)
        .with_default_spacer()
        .with_child(save_button)
        .with_default_spacer()
        .with_child(Container::new(crop_screenshot_button))
        .with_default_spacer()
        .with_flex_child(Container::new(take_screenshot_button), 1.0)
        .with_default_spacer()
        .with_child(Label::new("Select Screen:").with_text_color(Color::BLACK.with_alpha(0.85)))
        .with_default_spacer()
        .with_child(screen_selector);
    //.with_child(ShortcutKeys {favorite_hot_keys: HashSet::new(), pressed_hot_keys: HashSet::new(), state: NotBusy});

    let alert_row = Flex::row()
        .with_child(druid::widget::Either::new(
            |data: &AppState, _| data.alert.alert_visible,
            alert_widget(),
            Label::new(""),
        ))
        .center();

    let scroll = Scroll::new(
        Flex::column()
            .with_default_spacer()
            .with_child(Flex::row().with_child(buttons_bar))
            .with_default_spacer()
            .with_child(Flex::row().with_child(alert_row))
            .with_default_spacer()
            //flex.set_must_fill_main_axis(true);
            .with_child(spaced_zstack),
    )
    .vertical();
    //flex.set_main_axis_alignment(MainAxisAlignment::Center);
    let layout = scroll.background(Color::WHITE).expand().padding(5.);
    layout
}

pub fn show_about<T: Data>() -> MenuItem<T> {
    MenuItem::new(LocalizedString::new("About Us")).command(sys_cmd::SHOW_ABOUT)
}

pub fn set_shortcutkeys<T: Data>() -> MenuItem<T> {
    MenuItem::new(LocalizedString::new("Shortcut Keys")).command(SHORTCUT_KEYS)
}
pub fn set_path<T: Data>() -> MenuItem<T> {
    /*let png = FileSpec::new("PNG file", &["png"]);
    let jpg = FileSpec::new("JPG file", &["jpg"]);
    let gif = FileSpec::new("GIF file", &["gif"]);*/
    MenuItem::new(LocalizedString::new("Set Path"))
        .command(commands::SHOW_OPEN_PANEL.with(FileDialogOptions::default().select_directories()))
}

fn make_menu(_window: Option<WindowId>, _data: &AppState, _env: &Env) -> Menu<AppState> {
    let base = Menu::empty();
    base.entry(Menu::new(LocalizedString::new("Edit")).entry(druid::platform_menus::common::copy()))
        .entry(
            Menu::new(LocalizedString::new("Settings"))
                .entry(show_about())
                .entry(set_path())
                .entry(set_shortcutkeys()),
        )
}

fn build_colors_window(zstack_id_param: WidgetId) -> impl Widget<AppState> {
    let none = create_color_button(None, zstack_id_param);
    let green = create_color_button(Some(Color::GREEN), zstack_id_param);
    let red = create_color_button(Some(Color::RED), zstack_id_param);
    let black = create_color_button(Some(Color::BLACK), zstack_id_param);
    let white = create_color_button(Some(Color::WHITE), zstack_id_param);
    let aqua = create_color_button(Some(Color::AQUA), zstack_id_param);
    let gray = create_color_button(Some(Color::GRAY), zstack_id_param);
    let blue = create_color_button(Some(Color::BLUE), zstack_id_param);
    let fuchsia = create_color_button(Some(Color::FUCHSIA), zstack_id_param);
    let lime = create_color_button(Some(Color::LIME), zstack_id_param);
    let maroon = create_color_button(Some(Color::MAROON), zstack_id_param);
    let navy = create_color_button(Some(Color::NAVY), zstack_id_param);
    let olive = create_color_button(Some(Color::OLIVE), zstack_id_param);
    let purple = create_color_button(Some(Color::PURPLE), zstack_id_param);
    let teal = create_color_button(Some(Color::TEAL), zstack_id_param);
    let yellow = create_color_button(Some(Color::YELLOW), zstack_id_param);
    let silver = create_color_button(Some(Color::SILVER), zstack_id_param);

    let label = Label::new("Transparency:").with_text_color(Color::WHITE);
    let alpha_slider = CustomSlider::new()
        .with_range(1., 100.)
        .with_step(1.)
        .on_added(move |this, _ctx, _data, _env| {
            this.set_zstack_id(zstack_id_param);
        })
        .lens(AppState::alpha)
        .padding(5.0);

    let flex = Flex::column()
        .with_default_spacer()
        .with_child(none)
        .with_default_spacer()
        .with_child(
            Flex::row()
                .with_default_spacer()
                .with_child(green)
                .with_default_spacer()
                .with_child(red)
                .with_default_spacer()
                .with_child(black)
                .with_default_spacer()
                .with_child(white)
                .with_default_spacer(),
        )
        .with_default_spacer()
        .with_child(
            Flex::row()
                .with_default_spacer()
                .with_child(aqua)
                .with_default_spacer()
                .with_child(gray)
                .with_default_spacer()
                .with_child(blue)
                .with_default_spacer()
                .with_child(fuchsia)
                .with_default_spacer(),
        )
        .with_default_spacer()
        .with_child(
            Flex::row()
                .with_default_spacer()
                .with_child(lime)
                .with_default_spacer()
                .with_child(maroon)
                .with_default_spacer()
                .with_child(navy)
                .with_default_spacer()
                .with_child(olive)
                .with_default_spacer(),
        )
        .with_default_spacer()
        .with_child(
            Flex::row()
                .with_default_spacer()
                .with_child(purple)
                .with_default_spacer()
                .with_child(teal)
                .with_default_spacer()
                .with_child(yellow)
                .with_default_spacer()
                .with_child(silver)
                .with_default_spacer(),
        )
        .with_default_spacer()
        .with_default_spacer()
        .with_child(Flex::row().with_child(label))
        .with_child(Flex::row().with_child(alpha_slider))
        .background(Color::BLACK.with_alpha(0.3));

    Align::centered(flex)
}

fn create_color_button(
    color: Option<Color>,
    zstack_id_param: WidgetId,
) -> ControllerHost<ColoredButton<AppState>, Click<AppState>> {
    ColoredButton::from_label(Label::new(if color.is_some() { " " } else { "None" }))
        .with_color(color.unwrap_or(Color::SILVER.with_alpha(0.8)))
        .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
            ctx.get_external_handle()
                .submit_command(UPDATE_COLOR, (color, None), Target::Widget(zstack_id_param))
                .unwrap();
            data.color = color;
            ctx.window().close();
        })
}
/**
* This function assigns a name and a file path to an image stored on the disk.
*/
fn file_name(data_name: String, base_path: String) -> (Box<str>, Box<str>) {
    //TODO: use a date.
    let charset = "1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let base_path_modified = base_path.into_boxed_str();

    let name = if data_name.as_str() == "" && data_name.chars().all(char::is_alphanumeric) {
        let mut name = generate(16, charset);
        let mut path = format!("{}{}", base_path_modified, name);
        while Path::new(path.as_str()).exists() {
            name = generate(12, charset);
            path = format!("{}{}", base_path_modified, name);
        }
        format!("screenshot-{}", name).into_boxed_str()
    } else {
        data_name.into_boxed_str()
    };
    (base_path_modified, name)
}
