use druid::widget::{Flex};
use druid::{AppLauncher, Command, Data, Env, FileDialogOptions, LocalizedString, Menu, MenuItem, Widget, WindowDesc, WindowId};

fn main() {
    // main window of the application
    let main_window = WindowDesc::new(ui_builder())
        .menu(menu_builder::<String>)
        .title(LocalizedString::new("Screen-Capture-Title").with_placeholder("Screen Capture"));

    // launcher application
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(String::new())
        .expect("Failed to launch application");
}

/**
 It will build all start widgets useful to launch the application
*/
fn ui_builder() -> impl Widget<String> {
    Flex::column().with_spacer(50.0)
}

/**
 It builds the main menù on the top of the interface
*/
fn menu_builder<T: Data>(_: Option<WindowId>, _: &T, _: &Env) -> Menu<String> {
    let open_file_command = Command::new(
        druid::commands::SHOW_OPEN_PANEL,
        FileDialogOptions::new(),
        druid::Target::Auto,
    );
    let open_file = MenuItem::new(LocalizedString::new("open-screen-file").with_placeholder("Open")).command(open_file_command);

    let save_file = MenuItem::new(LocalizedString::new("save-screen-file").with_placeholder("Save"));
    let save_as_file = MenuItem::new(LocalizedString::new("save-as-screen-file").with_placeholder("Save As"));
    let copy_file= MenuItem::new(LocalizedString::new("copy-screen-file").with_placeholder("Copy"));

    //it creates the file menu on the top of the application
    let mut file_menu = Menu::new("File").entry(open_file).separator();
    file_menu = file_menu.entry(save_file);
    file_menu = file_menu.entry(save_as_file).separator();
    file_menu = file_menu.entry(copy_file);

    let shortcut_keys_preference =  MenuItem::new(LocalizedString::new("shortcut-keys-preferences").with_placeholder("Shortcut Key Preferences"));
    let save_location =  MenuItem::new(LocalizedString::new("save-location").with_placeholder("Save Location"));
    let about_us =  MenuItem::new(LocalizedString::new("about-us").with_placeholder("About Us"));

    let mut settings_menu = Menu::new("Settings").entry(shortcut_keys_preference);
    settings_menu = settings_menu.entry(save_location);
    settings_menu = settings_menu.entry(about_us);

    let mut menu = Menu::empty();
    menu = menu.entry(file_menu);
    menu = menu.entry(settings_menu);
    return menu;
}















