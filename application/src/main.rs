mod custom_widget;

use crate::custom_widget::{read_from_file, write_to_file, Alert, ColoredButton, CustomSlider, CustomZStack, OverImages, ScreenshotImage, SelectedRect, ShortcutKeys, StateShortcutKeys, TakeScreenshotButton, CREATE_ZSTACK, SAVE_OVER_IMG, SAVE_SCREENSHOT, SHORTCUT_KEYS, SHOW_OVER_IMG, UPDATE_BACK_IMG, UPDATE_COLOR, UPDATE_RECT_SIZE, UPDATE_SCREENSHOT_CROP, UPDATE_SCREENSHOT_CROP_CLOSE, verify_exists_dir};
use druid::commands::SHOW_ABOUT;
use druid::piet::ImageFormat;
use druid::widget::{
    Align, Button, Click, Container, ControllerHost, CrossAxisAlignment, Either, FillStrat, Flex,
    IdentityWrapper, Label, LensWrap, LineBreaking, MainAxisAlignment, Scroll, Stepper, TextBox,
    ViewSwitcher, ZStack,
};
use druid::Target::{Auto, Window};
use druid::{
    commands as sys_cmd, commands, AppDelegate, AppLauncher, Code, Color, Command, Data,
    DelegateCtx, Env, Event, EventCtx, FileDialogOptions, FontDescriptor, FontFamily, Handled,
    ImageBuf, Lens, LocalizedString, Menu, MenuItem, Point, Rect, Screen, Size, Target,
    TextAlignment, UnitPoint, Vec2, Widget, WidgetExt, WidgetId, WindowDesc, WindowId, WindowState,
};
use image::io::Reader;
use random_string::generate;
use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;
use std::string::ToString;
use std::sync::Arc;

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

const BASE_PATH: &str = "./src/";
const BASE_PATH_SCREENSHOT: &str = "./src/screenshots/";
const BASE_PATH_FAVORITE_SHORTCUT: &str = "./src/shortcut/";
const PATH_FAVORITE_SHORTCUT: &str = "./src/shortcut/shortcut_settings.json";

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
    crop_screenshot_enabled: bool,
    rename_file_enabled: bool,
}

fn main() {
    // Verify if the screenshot dir exists
    verify_exists_dir(BASE_PATH);
    verify_exists_dir(BASE_PATH_SCREENSHOT);
    verify_exists_dir(BASE_PATH_FAVORITE_SHORTCUT);

    let default_shortcut: HashSet<Code> = HashSet::<Code>::from([Code::KeyB, Code::KeyA]);

    let main_window = WindowDesc::new(build_root_widget())
        .title("Welcome!")
        .menu(make_menu)
        .with_min_size((1200., 670.))
        .set_window_state(WindowState::Maximized)
        .set_position((50., 20.));

    // create the initial app state
    let mut initial_state = AppState {
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
        base_path: BASE_PATH_SCREENSHOT.to_string(),
        alert: Alert {
            alert_visible: false,
            alert_message: "".to_string(),
        },
        shortcut_keys: ShortcutKeys {
            favorite_hot_keys: default_shortcut.clone(),
            pressed_hot_keys: HashSet::new(),
            state: StateShortcutKeys::NotBusy,
        },
        text_field_zstack: true,
        text_field: "".to_string(),
        crop_screenshot_enabled: false,
        rename_file_enabled: false,
    };

    // Reading and deserialization from file to set the favourite shortcut
    if let Some(deserialized) = read_from_file::<HashSet<String>>(PATH_FAVORITE_SHORTCUT) {
        let mut convert_code = HashSet::<Code>::new();
        for code in deserialized {
            match Code::from_str(code.as_str()) {
                Ok(code_deserialized) => {
                    convert_code.insert(code_deserialized);
                }
                Err(_) => {
                    convert_code = default_shortcut.clone();
                    break;
                }
            }
        }
        initial_state.shortcut_keys.favorite_hot_keys = convert_code;
    } else {
        initial_state.shortcut_keys.favorite_hot_keys = default_shortcut.clone();
    }

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
                        || data.shortcut_keys.pressed_hot_keys
                            == HashSet::from([Code::ControlLeft, Code::KeyW])
                    {
                        // ctrl + c : this is reserved for the copy shortcut, Esc is reserved to close the subwindows and ctrl + w is reserved to close the main window
                        data.shortcut_keys.state = StateShortcutKeys::ShortcutNotAvailable;
                    } else {
                        data.shortcut_keys.favorite_hot_keys =
                            data.shortcut_keys.pressed_hot_keys.clone();
                        data.shortcut_keys.state = StateShortcutKeys::NotBusy;
                        data.shortcut_keys.pressed_hot_keys = HashSet::new();

                        let mut convert_code = HashSet::<String>::new();
                        for code in data.shortcut_keys.favorite_hot_keys.clone() {
                            convert_code.insert(code.to_string());
                        }

                        match write_to_file(PATH_FAVORITE_SHORTCUT, &convert_code) {
                            Ok(_) => data
                                .alert
                                .show_alert("Favorite Shortcut Saved Successfully!"),
                            Err(_) => data
                                .alert
                                .show_alert("Error during writing to the shortcut settings file!"),
                        }
                    }
                } else if data.shortcut_keys.pressed_hot_keys == HashSet::from([Code::Escape]) {
                    data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map
                    data.shortcut_keys.state = StateShortcutKeys::NotBusy; // it has finished its job

                    // Key Escape has been pressed
                    if let Some(main_id) = data.main_window_id {
                        ctx.get_external_handle()
                            .submit_command(sys_cmd::SHOW_WINDOW, (), main_id)
                            .expect("Error sending the event");

                        if main_id != window_id {
                            ctx.submit_command(sys_cmd::CLOSE_WINDOW.to(Target::Window(window_id)));
                        }
                    }
                } else if data.shortcut_keys.pressed_hot_keys
                    == HashSet::from([Code::ControlLeft, Code::KeyW])
                {
                    data.shortcut_keys.state = StateShortcutKeys::NotBusy; // it has finished its job

                    // Keys ctrl + w has been pressed
                    if let Some(main_id) = data.main_window_id {
                        data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map

                        if main_id == window_id {
                            ctx.submit_command(sys_cmd::CLOSE_WINDOW.to(Target::Window(main_id)));
                        }
                    } else {
                        ctx.submit_command(sys_cmd::CLOSE_WINDOW.to(Target::Window(window_id)));
                    }
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
                    let mut monitors = Screen::get_monitors();
                    monitors.sort_by_key(|monitor| !monitor.is_primary());
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
                            .window_size((monitor.virtual_rect().x1, monitor.virtual_rect().y1))
                            .set_window_state(WindowState::Maximized)
                            .set_position(monitor.virtual_rect().origin()),
                    );
                    ctx.submit_command(
                        SHOW_OVER_IMG
                            .with((OverImages::Remove, None))
                            .to(Target::Widget(WidgetId::next())),
                    );
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
        } else if cmd.is(SHOW_ABOUT) {
            let mut monitors = Screen::get_monitors();
            monitors.sort_by_key(|monitor| !monitor.is_primary());
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
            let mut monitors = Screen::get_monitors();
            monitors.sort_by_key(|monitor| !monitor.is_primary());
            let index: usize =
                std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap();
            let monitor = monitors.get(index).unwrap();

            let window_shortcut = WindowDesc::new(build_shortcut_keys_widget())
                .title(LocalizedString::new("Shortcut Keys Configuration"))
                .set_always_on_top(false)
                .transparent(true)
                .resizable(true)
                .show_titlebar(true)
                .window_size((500., 460.))
                .set_position(monitor.virtual_rect().origin())
                .with_min_size(Size::new(500., 450.));

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

    let shortcut_keys_not_available = Flex::column()
        .with_child(
            Label::new("Reserved Combinations")
                .with_text_color(Color::RED)
                .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                .with_text_size(19.)
                .padding(5.),
        )
        .with_child(
            Label::new("Ctrl + C: Copy")
                .with_text_color(Color::BLUE)
                .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                .with_text_size(15.)
                .padding(5.),
        )
        .with_child(
            Label::new("Ctrl + W: Close the main window")
                .with_text_color(Color::BLUE)
                .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                .with_text_size(15.)
                .padding(5.),
        )
        .with_child(
            Label::new("Esc: Close the subwindow")
                .with_text_color(Color::BLUE)
                .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                .with_text_size(15.)
                .padding(5.),
        )
        .with_child(
            Label::new("Note: The 'Esc' key will be disabled if no images are")
                .with_text_color(Color::BLUE)
                .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                .with_text_size(10.)
                .padding(10.),
        )
        .with_child(
            Label::new("captured, and you can use 'Ctrl + W' to close the subwindow.")
                .with_text_color(Color::BLUE)
                .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
                .with_text_size(10.),
        )
        .padding(10.)
        .border(Color::RED, 0.7)
        .rounded(5.)
        .align_horizontal(UnitPoint::LEFT);

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
                )
                .with_default_spacer()
                .with_child(shortcut_keys_not_available),
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
        data.shortcut_keys.state = StateShortcutKeys::NotBusy; // reset of shortcut state

        if data.state != State::Start {
            data.name = "".to_string(); // reset name file
        }
        let (base_path, name) = file_name(data.name.clone(), data.base_path.clone());
        data.name = (*name.clone()).to_string();

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
        data.delay = 0.;
        data.alert
            .show_alert("The image has been saved on the disk!");
    });

    let delay_value = Label::dynamic(|data: &AppState, _env| data.delay.to_string())
        .with_text_color(Color::WHITE)
        .background(Color::BLACK.with_alpha(0.55));
    let delay_stepper = Stepper::new()
        .with_range(0.0, 10.0)
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

        //data.state = State::Start;
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

    let zstack = ZStack::new(rectangle).with_child(
        buttons_flex,
        Vec2::new(1.0, 1.0),
        Vec2::ZERO,
        UnitPoint::BOTTOM_RIGHT,
        Vec2::new(-100.0, -100.0),
    );

    zstack
}

fn build_screenshot_crop_widget(monitor: usize) -> impl Widget<AppState> {
    let rectangle = LensWrap::new(SelectedRect::new(monitor), AppState::rect);
    Container::new(rectangle)
}

fn build_root_widget() -> impl Widget<AppState> {
    let take_screenshot_button = Either::new(
        |data: &AppState, _env| data.crop_screenshot_enabled == false,
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

            let mut monitors = Screen::get_monitors();
            monitors.sort_by_key(|monitor| !monitor.is_primary());
            let index: usize =
                std::str::FromStr::from_str(data.screen.trim_start_matches(".")).unwrap();
            monitors.sort_by_key(|monitor| !monitor.is_primary());
            let monitor = monitors.get(index).unwrap();

            let primary_monitor_rect = monitors
                .get(index)
                .expect("Can't find the selected monitor!")
                .virtual_rect();
            let screen_img_rect = Rect {
                x0: 0.,
                y0: 0.,
                x1: primary_monitor_rect.width() as f64,
                y1: primary_monitor_rect.height() as f64,
            };
            ctx.submit_command(UPDATE_RECT_SIZE.with(screen_img_rect)); // reset of the rect

            ctx.new_window(
                WindowDesc::new(build_screenshot_widget(index))
                    .title(WINDOW_TITLE)
                    .set_always_on_top(true)
                    .transparent(true)
                    .resizable(false)
                    .show_titlebar(false)
                    .set_position(monitor.virtual_rect().origin())
                    .window_size((monitor.virtual_rect().x1, monitor.virtual_rect().y1))
                    .set_window_state(WindowState::Maximized),
            );

            ctx.submit_command(
                SHOW_OVER_IMG
                    .with((OverImages::Remove, None))
                    .to(Target::Widget(data.custom_zstack_id.unwrap())),
            );
        }),
        Label::new(""),
    );

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

    let screenshot_image_crop = IdentityWrapper::wrap(
        ScreenshotImage::new(ImageBuf::from_raw(
            Arc::<[u8]>::from(Vec::from([0, 0, 0, 0]).as_slice()),
            ImageFormat::RgbaSeparate,
            1usize,
            1usize,
        ))
        .fill_mode(FillStrat::ScaleDown)
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

    let zstack_crop = IdentityWrapper::wrap(
        CustomZStack::new(screenshot_image_crop, *SCREENSHOT_WIDGET_ID),
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

    let spaced_zstack = Either::new(
        |data: &AppState, _env| {
            data.state == State::ScreenTaken(ImageModified::Savable)
                && data.crop_screenshot_enabled == true
        },
        ZStack::new(zstack_crop)
            .with_child(
                build_screenshot_crop_widget(0),
                Vec2::new(1.0, 1.0),
                Vec2::ZERO,
                UnitPoint::CENTER,
                Vec2::new(0., 0.),
            )
            .padding((10.0, 10.0)),
        Container::new(zstack).padding((10.0, 10.0)),
    );

    let crop_screenshot_save_button = TakeScreenshotButton::from_label(Label::new("Update Image"))
        .with_color(Color::rgb8(0, 150, 0).with_alpha(1.))
        .on_click(|ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
            let (base_path, name) = file_name(data.name.clone(), data.base_path.clone());
            data.name = (*name.clone()).to_string();

            ctx.submit_command(
                UPDATE_SCREENSHOT_CROP
                    .with((
                        data.rect,
                        base_path,
                        name,
                        image::ImageFormat::from_extension(data.extension.trim_start_matches("."))
                            .unwrap(),
                        data.custom_zstack_id
                            .expect("How did you open this window?"),
                    ))
                    .to(Target::Widget(*SCREENSHOT_WIDGET_ID)),
            );

            data.crop_screenshot_enabled = false;

            data.shortcut_keys.state = StateShortcutKeys::NotBusy; // reset of shortcut state
            data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map

            data.alert
                .show_alert("The image has been saved on the disk!");

            data.state = State::ScreenTaken(ImageModified::NotSavable);
        });

    let close_crop_screenshop_button = ColoredButton::from_label(Label::new("Cancel"))
        .with_color(Color::rgb8(150, 0, 0).with_alpha(1.))
        .on_click(|ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
            data.crop_screenshot_enabled = false; // disable the crop screenshot widget
            data.state = State::ScreenTaken(ImageModified::NotSavable);

            ctx.submit_command(
                UPDATE_SCREENSHOT_CROP_CLOSE.to(Target::Widget(*SCREENSHOT_WIDGET_ID)),
            );

            data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map
            data.shortcut_keys.state = StateShortcutKeys::NotBusy; // it has finished its job
        });

    let buttons_crop_screenshot_flex = Flex::row()
        .with_child(
            Label::new("Crop Screenshot:  ")
                .with_text_color(Color::BLACK)
                .padding(2.),
        )
        .with_child(crop_screenshot_save_button)
        .with_default_spacer()
        .with_child(close_crop_screenshop_button);

    let crop_screenshot_button = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::NotSavable),
        Either::new(
            |data: &AppState, _env| data.crop_screenshot_enabled == false,
            ColoredButton::from_label(Label::new("Crop ScreenShot"))
                .with_color(Color::rgb(0., 0., 255.))
                .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                    data.main_window_id = Some(ctx.window_id());
                    data.custom_zstack_id = Some(*ZSTACK_ID);
                    data.screenshot_id = Some(*SCREENSHOT_WIDGET_ID);

                    data.shortcut_keys.state = StateShortcutKeys::StartScreenGrabber; // started to capture the screen
                    data.shortcut_keys.pressed_hot_keys = HashSet::new(); // clean map

                    data.crop_screenshot_enabled = true;
                    data.state = State::ScreenTaken(ImageModified::Savable);
                }),
            Label::new(""),
        ),
        Either::new(
            |data: &AppState, _env| {
                data.state == State::ScreenTaken(ImageModified::Savable)
                    && data.crop_screenshot_enabled == true
            },
            buttons_crop_screenshot_flex,
            Label::new(""),
        ),
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

    let screen_selector = Either::new(
        |data: &AppState, _env| data.crop_screenshot_enabled == false,
        ViewSwitcher::new(
            |data: &AppState, _env| data.clone(),
            |selector, data: &AppState, _env| {
                if selector.screen.chars().all(char::is_numeric) {
                    Box::new(
                        Label::new(format!("{} ▼", data.screen.parse::<i32>().unwrap_or(0) + 1))
                            .with_text_color(Color::BLACK)
                            .border(Color::BLACK, 2.)
                            .on_click(|_, data: &mut AppState, _| {
                                data.screen = format!(".{}", data.screen)
                            }),
                    )
                } else {
                    let mut screens = Screen::get_monitors();
                    screens.sort_by_key(|monitor| !monitor.is_primary());
                    let dim = screens.len();
                    let number: u8 =
                        std::str::FromStr::from_str(selector.screen.trim_start_matches("."))
                            .unwrap();
                    let mut flex = Flex::column();
                    for i in 0..dim {
                        let color = if i == number as usize {
                            Color::BLACK.with_alpha(1.)
                        } else {
                            Color::BLACK.with_alpha(0.4)
                        };
                        flex.add_child(
                            Label::new(format!("{} ◀", i + 1))
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
        ),
        Label::new(""),
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
        |data: &AppState, _| {
            data.text_field_zstack == true
                && data.state == State::ScreenTaken(ImageModified::NotSavable)
        },
        Flex::row()
            .with_child(
                druid::widget::TextBox::new()
                    .with_placeholder("Enter a text")
                    .fix_width(150.)
                    .lens(AppState::text_field),
            )
            .with_default_spacer()
            .with_child(
                Button::from_label(Label::new("Save"))
                    .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                        ctx.submit_command(
                            SHOW_OVER_IMG
                                .with((OverImages::Text, Some(data.text_field.clone())))
                                .to(Target::Widget(*ZSTACK_ID)),
                        );
                        data.text_field = "".to_string();
                        data.text_field_zstack = false;
                        data.state = State::ScreenTaken(ImageModified::Savable);
                        ctx.request_update();
                    })
                    .disabled_if(move |data: &AppState, _env: &Env| {
                        return data.text_field.len() == 0;
                    }),
            )
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
        |data: &AppState, _env| {
            data.state == State::ScreenTaken(ImageModified::Savable)
                && data.crop_screenshot_enabled == false
                && data.rename_file_enabled == false
        },
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
        |data: &AppState, _env| {
            data.state == State::ScreenTaken(ImageModified::Savable)
                && data.crop_screenshot_enabled == false
        },
        Either::new(
            |data: &AppState, _env| data.rename_file_enabled == false,
            ColoredButton::from_label(Label::new("Save"))
                .with_color(Color::rgb(0., 120. / 256., 0.))
                .on_click(move |ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                    let (base_path, name) = file_name(data.name.clone(), data.base_path.clone());

                    data.alert
                        .show_alert("The image has been saved on the disk!");

                    ctx.submit_command(
                        SAVE_OVER_IMG.with((
                            base_path,
                            name,
                            image::ImageFormat::from_extension(
                                data.extension.trim_start_matches("."),
                            )
                            .unwrap(),
                        )),
                    );

                    data.state = State::ScreenTaken(ImageModified::NotSavable);
                    data.rename_file_enabled = false;
                }),
            ColoredButton::from_label(Label::new("Back").with_text_color(Color::BLACK))
                .with_color(Color::rgb(0.8, 0.8, 0.))
                .on_click(
                    move |_ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                        data.state = State::ScreenTaken(ImageModified::NotSavable);
                        data.rename_file_enabled = false;
                    },
                ),
        ),
        Label::new(""),
    );

    let file_name_label = Label::new(|data: &AppState, _env: &_| match data.state {
        State::Start => "Screenshot:",
        State::ScreenTaken(ImageModified::Savable) => "Modified Image:",
        State::ScreenTaken(ImageModified::NotSavable) => "",
    })
    .with_text_color(Color::BLACK.with_alpha(0.85));

    /*let save_button_later = Either::new(
        |data: &AppState, _env| data.state == State::ScreenTaken(ImageModified::NotSavable),
        ColoredButton::from_label(Label::new("Change Name Image"))
            .with_color(Color::rgb(0., 120. / 256., 0.))
            .on_click(
                move |_ctx: &mut EventCtx, data: &mut AppState, _env: &Env| {
                    data.rename_file_enabled = true;
                    data.state = State::ScreenTaken(ImageModified::Savable);
                },
            ),
        Label::new(""),
    );*/

    let screen_selector_label = Either::new(
        |data: &AppState, _env| data.crop_screenshot_enabled == false,
        Label::new("Select Screen:").with_text_color(Color::BLACK.with_alpha(0.85)),
        Label::new(""),
    );

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
        //.with_default_spacer()
        //.with_child(save_button_later)
        .with_default_spacer()
        .with_child(Container::new(crop_screenshot_button))
        .with_default_spacer()
        .with_flex_child(Container::new(take_screenshot_button), 1.0)
        .with_default_spacer()
        .with_child(screen_selector_label)
        .with_default_spacer()
        .with_child(screen_selector);

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
            .with_child(spaced_zstack),
    )
    .vertical();
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
