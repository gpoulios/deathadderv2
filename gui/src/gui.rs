#![windows_subsystem = "windows"]

use std::cell::RefCell;
use windows::{
    core::{s, PCSTR},
    Win32::{
        System::Diagnostics::Debug::OutputDebugStringA,
        Foundation::{HWND, WPARAM, LPARAM, HINSTANCE},
        UI::{
            Controls::{TBS_TOOLTIPS, TBS_BOTTOM, TBS_DOWNISLEFT, TBM_SETLINESIZE,
                TBM_SETPAGESIZE, TBM_SETTICFREQ, TBS_NOTIFYBEFOREMOVE,
            },
            WindowsAndMessaging::{SendMessageA, GetWindowLongA, SetWindowLongA,
                GWL_STYLE, MessageBoxA, MB_OK, MB_ICONERROR, BS_TOP,
                SetCursor, LoadCursorW, IDC_HAND, IDC_ARROW},
        },
    },
};
use native_windows_gui as nwg;
use native_windows_derive as nwd;
use nwd::NwgUi;
use nwg::NativeUi;

use rgb::RGB8;
use librazer::{cfg::Config, device::UsbDevice, common::PollingRate};
use librazer::device::{DeathAdderV2, RazerMouse};

pub mod color_chooser;
use color_chooser::ColorDialog;

/*
 * Log messages to the debugger using OutputDebugString (only for command line
 * invocation). Use DebugView by Mark Russinovich to view
 */
macro_rules! dbglog {
    ($($args: tt)*) => {{
        let msg = format!($($args)*);
        let msg_sz = format!("{}{}", msg, "\0");
        println!("{}", msg);
        unsafe {
            OutputDebugStringA(PCSTR::from_raw(msg_sz.as_ptr()));
        }
    }}
}

// macro_rules! dbgpanic {
//     ($($args: tt)*) => {{
//         let msg = format!($($args)*);
//         let msg_sz = format!("{}{}", msg, "\0");
//         unsafe {
//             OutputDebugStringA(PCSTR::from_raw(msg_sz.as_ptr()));
//         }
//         panic!("{}", msg);
//     }}
// }

macro_rules! msgboxpanic {
    ($($args: tt)*) => {{
        let msg = format!($($args)*);
        let msg_sz = format!("{}{}", msg, "\0");
        unsafe {
            let msg_ptr = PCSTR::from_raw(msg_sz.as_ptr());
            MessageBoxA(HWND(0), msg_ptr, s!("Error"), MB_OK | MB_ICONERROR);
        }
        panic!("{}", msg);
    }}
}

macro_rules! msgboxerror {
    ($($args: tt)*) => {{
        let msg = format!($($args)*);
        let msg_sz = format!("{}{}", msg, "\0");
        eprintln!("{}", msg);
        unsafe {
            let msg_ptr = PCSTR::from_raw(msg_sz.as_ptr());
            MessageBoxA(HWND(0), msg_ptr, s!("Error"), MB_OK | MB_ICONERROR);
        }
    }}
}

/// convert bool to nwg::CheckBoxState
macro_rules! to_check_state {
    ($b:expr) => {
        if $b { nwg::CheckBoxState::Checked } else { nwg::CheckBoxState::Unchecked }
    };
}

/// convert nwg::CheckBoxState to bool
macro_rules! from_check_state {
    ($s:expr) => {
        match $s {
            nwg::CheckBoxState::Checked => true,
            _ => false,
        }
    };
}

fn configure_trackbar(bar: &nwg::TrackBar, line: isize, page: isize) {
    unsafe {
        let hbar = HWND(bar.handle.hwnd().unwrap() as isize);
        SendMessageA(hbar, TBM_SETLINESIZE, WPARAM(0), LPARAM(line));
        SendMessageA(hbar, TBM_SETPAGESIZE, WPARAM(0), LPARAM(page));
        SendMessageA(hbar, TBM_SETTICFREQ, WPARAM(line as usize), LPARAM(0));
        add_style(&bar.handle,
            (TBS_TOOLTIPS | TBS_BOTTOM | TBS_DOWNISLEFT | TBS_NOTIFYBEFOREMOVE) as i32);
    }
}

fn add_style(handle: &nwg::ControlHandle, style: i32) {
    unsafe {
        let hwnd = HWND(handle.hwnd().unwrap() as isize);
        let style = style | GetWindowLongA(hwnd, GWL_STYLE);
        SetWindowLongA(hwnd, GWL_STYLE, style);
    }
}

#[derive(Default, NwgUi)]
pub struct DeathAdderv2App {
    #[nwg_control(size: (700, 310), center: true, title: "Razer DeathAdder v2 configuration")]
    #[nwg_events( OnWindowClose: [DeathAdderv2App::window_close(SELF)])]
    window: nwg::Window,

    #[nwg_layout(parent: window, min_size: [400, 200], max_column: Some(11))]
    grid: nwg::GridLayout,

    #[nwg_control(text: "Device:", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, col_span: 3)]
    lbl_device: nwg::Label,

    #[nwg_control(v_align: nwg::VTextAlign::Top)] // has trouble aligning vertically
    #[nwg_layout_item(layout: grid, col: 3, col_span: 7)]
    #[nwg_events( OnComboxBoxSelection: [DeathAdderv2App::device_selected(SELF)])]
    cmb_device: nwg::ComboBox<UsbDevice>,

    /*
     * DPI
     */
    #[nwg_control(text: "DPI:", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 1, col_span: 3)]
    lbl_dpi: nwg::Label,

    #[nwg_control(range: Some(100..20000), pos: Some(20000))]
    #[nwg_layout_item(layout: grid, row: 1, col: 3, col_span: 5)]
    #[nwg_events(
        // Unfortunately 'TrackBarUpdated' doesn't trigger with keyboard or
        // scroll, so we update on each change, even if during mouse drag
        // this might be spamming the device
        OnHorizontalScroll: [DeathAdderv2App::dpi_selected(SELF)],
    )]
    bar_dpi: nwg::TrackBar,

    #[nwg_control(text: "20000", h_align: nwg::HTextAlign::Left, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 1, col: 8, col_span: 2)]
    txt_dpi: nwg::Label,

    /*
     * Polling rate
     */
    #[nwg_control(text: "Polling rate:", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 2, col_span: 3)]
    lbl_pollrate: nwg::Label,

    #[nwg_control(collection: PollingRate::all(), v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 2, col: 3, col_span: 2)]
    #[nwg_events( OnComboxBoxSelection: [DeathAdderv2App::pollrate_selected(SELF)])]
    cmb_pollrate: nwg::ComboBox<PollingRate>,

    /*
     * Logo color
     */
    #[nwg_control(text: "Logo color:", h_align: nwg::HTextAlign::Right)]
    #[nwg_layout_item(layout: grid, row: 3, col_span: 3)]
    lbl_logocolor: nwg::Label,

    #[nwg_control(text: "", line_height: Some(20))]
    #[nwg_layout_item(layout: grid, row: 3, col: 3, col_span: 2)]
    #[nwg_events(
        MousePressLeftUp: [DeathAdderv2App::logo_color_clicked(SELF)],
        OnMouseMove: [DeathAdderv2App::set_cursor_hand(SELF)],
    )]
    btn_logocolor: nwg::RichLabel,

    /*
     * Scroll color
     */
    #[nwg_control(text: "Scroll wheel color:", h_align: nwg::HTextAlign::Right)]
    #[nwg_layout_item(layout: grid, row: 4, col_span: 3)]
    lbl_scrollcolor: nwg::Label,

    #[nwg_control(text: "", line_height: Some(20))]
    #[nwg_layout_item(layout: grid, row: 4, col: 3, col_span: 2)]
    #[nwg_events(
        MousePressLeftUp: [DeathAdderv2App::scroll_color_clicked(SELF)],
        OnMouseMove: [DeathAdderv2App::set_cursor_hand(SELF)],
    )]
    btn_scrollcolor: nwg::RichLabel,

    /*
     * Same color check box
     */
    #[nwg_control(text: "Same as logo")]
    #[nwg_layout_item(layout: grid, row: 4, col: 5, col_span: 3)]
    #[nwg_events(
        MousePressLeftUp: [DeathAdderv2App::same_color_changed(SELF, EVT, EVT_DATA)],
        OnKeyRelease: [DeathAdderv2App::same_color_changed(SELF, EVT, EVT_DATA)]
    )]
    chk_samecolor: nwg::CheckBox,

    /*
     * Logo brightness
     */
    #[nwg_control(text: "Logo brightness:", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 5, col_span: 3)]
    lbl_logobright: nwg::Label,

    #[nwg_control(range: Some(0..100), pos: Some(50))]
    #[nwg_layout_item(layout: grid, row: 5, col: 3, col_span: 4)]
    #[nwg_events(
        // Unfortunately 'TrackBarUpdated' doesn't trigger with keyboard or
        // scroll, so we update on each change, even if during mouse drag
        // this might be spamming the device
        OnHorizontalScroll: [DeathAdderv2App::logo_brightness_selected(SELF)],
    )]
    bar_logobright: nwg::TrackBar,

    #[nwg_control(text: "50", h_align: nwg::HTextAlign::Left, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 5, col: 7)]
    txt_logobright: nwg::Label,

    /*
     * Scroll brightness
     */
    #[nwg_control(text: "Scroll wheel brightness:", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 6, col_span: 3)]
    lbl_scrollbright: nwg::Label,

    #[nwg_control(range: Some(0..100), pos: Some(50))]
    #[nwg_layout_item(layout: grid, row: 6, col: 3, col_span: 4)]
    #[nwg_events(
        // Unfortunately 'TrackBarUpdated' doesn't trigger with keyboard or
        // scroll, so we update on each change, even if during mouse drag
        // this might be spamming the device
        OnHorizontalScroll: [DeathAdderv2App::scroll_brightness_selected(SELF)],
    )]
    bar_scrollbright: nwg::TrackBar,

    #[nwg_control(text: "50", h_align: nwg::HTextAlign::Left, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 6, col: 7)]
    txt_scrollbright: nwg::Label,

    /*
     * Same brightness check box
     */
    #[nwg_control(text: "Same as logo")]
    #[nwg_layout_item(layout: grid, row: 6, col: 8, col_span: 3)]
    #[nwg_events(
        MousePressLeftUp: [DeathAdderv2App::same_brightness_changed(SELF, EVT, EVT_DATA)],
        OnKeyRelease: [DeathAdderv2App::same_brightness_changed(SELF, EVT, EVT_DATA)]
    )]
    chk_samebright: nwg::CheckBox,

    device: RefCell<Option<DeathAdderV2>>,
    config: RefCell<Config>,
}

impl DeathAdderv2App {
    /// Sugar to avoid typing self.device.borrow().as_ref().map
    /// Note: will not execute if device is None
    fn with_device<U, F>(&self, dav2: F) -> Option<U>
    where
        F: FnOnce(&DeathAdderV2) -> U,
    {
        self.device.borrow().as_ref().map(dav2)
    }

    /// Borrow config and apply closure
    fn with_config<U, F>(&self, cfg_cb: F) -> U
    where
        F: FnOnce(&Config) -> U,
    {
        let cfg = self.config.borrow();
        cfg_cb(&cfg)
    }

    /// Borrow mutable config and apply closure
    fn with_mut_config<U, F>(&self, cfg_cb: F) -> U
    where
        F: FnOnce(&mut Config) -> U,
    {
        let mut cfg = self.config.borrow_mut();
        cfg_cb(&mut (*cfg))
    }

    fn set_device_controls_enabled(&self, enabled: bool) {
        self.bar_dpi.set_enabled(enabled);
        self.cmb_pollrate.set_enabled(enabled);
        self.chk_samecolor.set_enabled(enabled);
        self.bar_logobright.set_enabled(enabled);
        self.bar_scrollbright.set_enabled(enabled);
        self.chk_samebright.set_enabled(enabled);
    }

    fn update_values(&self) {
        match self.device.borrow().as_ref() {
            Some(dav2) => {

                match dav2.get_dpi() {
                    Ok((dpi_x, _)) => self.bar_dpi.set_pos(dpi_x as usize),
                    Err(e) => {
                        msgboxerror!("Failed to get DPI: {}", e);
                        self.bar_dpi.set_enabled(false);
                    }
                };

                match dav2.get_poll_rate() {
                    Ok(pollrate) => {
                        let collection = self.cmb_pollrate.collection();
                        let index = collection.iter().position(|&p| p == pollrate);
                        self.cmb_pollrate.set_selection(index);
                    },
                    Err(e) => {
                        msgboxerror!("Failed to get polling rate: {}", e);
                        self.cmb_pollrate.set_enabled(false);
                    }
                };

                match dav2.get_logo_brightness() {
                    Ok(b) => self.bar_logobright.set_pos(b as usize),
                    Err(e) => {
                        msgboxerror!("Failed to get logo brightness: {}", e);
                        self.bar_logobright.set_enabled(false);
                    }
                };

                match dav2.get_scroll_brightness() {
                    Ok(b) => self.bar_scrollbright.set_pos(b as usize),
                    Err(e) => {
                        msgboxerror!("Failed to get scroll whell brightness: {}", e);
                        self.bar_scrollbright.set_enabled(false);
                    }
                };
            },

            None => { // no device; set some defaults
                self.bar_dpi.set_pos(self.bar_dpi.range_min());
                self.cmb_pollrate.set_selection(None);
                self.bar_logobright.set_pos(self.bar_logobright.range_min());
                self.bar_scrollbright.set_pos(self.bar_scrollbright.range_min());
            },
        };

        // updates that need to happen irrespective of the result
        self.txt_dpi.set_text(&self.bar_dpi.pos().to_string());
        self.txt_logobright.set_text(&self.bar_logobright.pos().to_string());
        self.txt_scrollbright.set_text(&self.bar_scrollbright.pos().to_string());

        self.with_config(|cfg| {
            // can't take these from the device; assume they're what the config says
            self.set_logo_color(cfg.logo_color);
            self.set_scroll_color(cfg.scroll_color);
            self.set_same_color(cfg.same_color, true);
            self.set_same_brightness(cfg.same_brightness, true);
        });

    }

    fn device_selected(&self) {
        let collection = self.cmb_device.collection();
        let dev = self.cmb_device.selection().and_then(|i| collection.get(i));
        let dav2 = dev.and_then(|d| {
            match DeathAdderV2::from(d) {
                Ok(d) => Some(d),
                Err(e) => {
                    msgboxerror!("Error opening device: {}", e);
                    None
                }
            }
        });

        self.set_device_controls_enabled(dav2.is_some());
        self.device.replace(dav2);
        self.update_values();
    }

    fn dpi_selected(&self) {
        let dpi = self.bar_dpi.pos() as u16;
        self.txt_dpi.set_text(&self.bar_dpi.pos().to_string());
        self.with_device(|dav2| dav2.set_dpi(dpi, dpi));
    }

    fn pollrate_selected(&self) {
        let collection = self.cmb_pollrate.collection();
        self.cmb_pollrate.selection()
            .and_then(|i| collection.get(i))
            .map(|&pollrate| {
                self.with_device(|dav2| dav2.set_poll_rate(pollrate));
            });
    }

    fn set_cursor_hand(&self) {
        let lpcursorname = match self.device.borrow().as_ref() {
            Some(_) => IDC_HAND,
            None => IDC_ARROW,
        };

        unsafe {
            _ = LoadCursorW(HINSTANCE(0), lpcursorname)
                .map(|cursor| SetCursor(cursor));
        }
    }

    fn logo_color_clicked(&self) {
        self.with_mut_config(|cfg| {
            self.with_device(|dav2| {

                // dav2 here must outlive dialog and therefore change_cb
                let mut dialog = ColorDialog::new();

                // ColorDialog arguments
                let parent = HWND(self.window.handle.hwnd().unwrap() as isize);
                let init_logo = Some(cfg.logo_color);
                let init_scroll = cfg.scroll_color;
                let same_color = cfg.same_color;
                let change_cb = Some(move |_: &ColorDialog, &color: &RGB8| {
                    _ = dav2.preview_static(
                        color, if same_color { color } else { init_scroll });
                });

                // show the dialog and choose what to apply (either initial or new)
                let color = match dialog.show(parent, init_logo, change_cb) {
                    Some(chosen_color) => chosen_color,
                    None => cfg.logo_color,
                };

                // set the color
                cfg.logo_color = color;
                self.set_logo_color(color);
                if same_color {
                    self.set_scroll_color(color);
                }

            }); // <- dialog, change_cb dropped here
        });
    }

    fn logo_color(&self) -> RGB8 {
        self.with_config(|cfg| cfg.logo_color)
    }

    /// Does not update the config
    fn set_logo_color(&self, color: RGB8) {
        self.with_device(|dav2| dav2.set_logo_color(color));
        self.btn_logocolor.set_background_color(color.into());
    }

    fn scroll_color_clicked(&self) {
        self.with_mut_config(|cfg| {
            self.with_device(|dav2| {

                // dav2 here must outlive dialog and therefore change_cb
                let mut dialog = ColorDialog::new();

                // ColorDialog arguments
                let parent = HWND(self.window.handle.hwnd().unwrap() as isize);
                let logo_color = cfg.logo_color;
                let init_scroll = Some(if cfg.same_color {
                    cfg.logo_color
                } else {
                    cfg.scroll_color
                });
                let change_cb = Some(move |_: &ColorDialog, &color: &RGB8| {
                    _ = dav2.preview_static(logo_color, color);
                });

                // show the dialog and choose what to apply (either initial or new)
                let color = match dialog.show(parent, init_scroll, change_cb) {
                    Some(chosen_color) => {
                        // if the user pressed ok, we no longer use same colors
                        cfg.same_color = false;
                        self.chk_samecolor.set_check_state(to_check_state!(false));
                        cfg.scroll_color = chosen_color;
                        chosen_color
                    },
                    None => {
                        // if the user pressed cancel, revert (nothing to save in cfg)
                        if cfg.same_color {
                            logo_color
                        } else {
                            cfg.scroll_color
                        }
                    },
                };

                // set the color
                self.set_scroll_color(color);

            }); // <- dialog, change_cb dropped here
        });
    }

    fn scroll_color(&self) -> RGB8 {
        self.with_config(|cfg| cfg.scroll_color)
    }

    /// Does not update the config
    fn set_scroll_color(&self, color: RGB8) {
        self.with_device(|dav2| dav2.set_scroll_color(color));
        self.btn_scrollcolor.set_background_color(color.into());
    }

    fn same_color_changed(&self, evt: nwg::Event, evtdata: &nwg::EventData) {
        // only interested in space key
        if evt == nwg::Event::OnKeyRelease && evtdata.on_key() != 32u32 {
            return
        }

        // unfortunately there is no 'state_changed' event so we get the
        // mouse up and space key events which trigger before the state
        // has actually changed so we negate it to get what will become

        let same = !from_check_state!(self.chk_samecolor.check_state());
        self.set_same_color(same, false);
        self.with_mut_config(|cfg| cfg.same_color = same);
    }

    /// Does not update the config
    fn set_same_color(&self, same: bool, update_ui: bool) {
        if update_ui {
            self.chk_samecolor.set_check_state(to_check_state!(same));
        }
        if same {
            self.set_scroll_color(self.logo_color());
        } else {
            self.set_scroll_color(self.scroll_color());
        }
    }

    fn logo_brightness_selected(&self) {
        let brightness = self.bar_logobright.pos() as u8;
        self.txt_logobright.set_text(&brightness.to_string());
        self.with_device(|dav2| dav2.set_logo_brightness(brightness));
        self.with_config(|cfg| if cfg.same_brightness {
            self.set_scroll_brightness(brightness as usize);
        });
    }

    fn scroll_brightness_selected(&self) {
        let brightness = self.bar_scrollbright.pos();
        self.txt_scrollbright.set_text(&brightness.to_string());
        self.with_device(|dav2| dav2.set_scroll_brightness(brightness as u8));
    }

    /// Does not update the config
    fn set_scroll_brightness(&self, brightness: usize) {
        self.txt_scrollbright.set_text(&brightness.to_string());
        self.bar_scrollbright.set_pos(brightness);
        self.with_device(|dav2| dav2.set_scroll_brightness(brightness as u8));
    }

    fn same_brightness_changed(&self, evt: nwg::Event, evtdata: &nwg::EventData) {
        // only interested in space key
        if evt == nwg::Event::OnKeyRelease && evtdata.on_key() != 32u32 {
            return
        }

        // unfortunately there is no 'state_changed' event so we get the
        // mouse up and space key events which trigger before the state
        // has actually changed so we negate it to get what will become

        let same = !from_check_state!(self.chk_samebright.check_state());
        self.set_same_brightness(same, false);
        self.with_mut_config(|cfg| cfg.same_brightness = same);
    }

    /// Does not update the config
    fn set_same_brightness(&self, same: bool, update_ui: bool) {
        if update_ui {
            self.chk_samebright.set_check_state(to_check_state!(same));
        }

        self.bar_scrollbright.set_enabled(!same);

        if same {
            self.set_scroll_brightness(self.bar_logobright.pos());
        } else {
            self.with_device(|dav2| match dav2.get_scroll_brightness() {
                Ok(brightness) => self.set_scroll_brightness(brightness as usize),
                Err(_) => ()
            });
        }
    }

    fn window_close(&self) {
        _ = self.with_config(|cfg| cfg.save()).map_err(|e|{
            msgboxerror!("Failed to save config: {}", e);
        });
        nwg::stop_thread_dispatch();
    }
}

fn main() {
    _ = nwg::init().map_err(
        |e| msgboxpanic!("Failed to init Native Windows GUI: {}", e));
    _ = nwg::Font::set_global_family("Segoe UI").map_err(
        |e| dbglog!("Failed to set default font: {}", e));

    let app = DeathAdderv2App::build_ui(Default::default())
        .unwrap_or_else(|e| msgboxpanic!("Failed to build UI: {}", e));

    app.config.replace(Config::load().unwrap_or(Config::default()));

    // default to false and if a valid device is selected they will be enabled
    app.set_device_controls_enabled(false);

    // configure a few things on the trackbars
    configure_trackbar(&app.bar_dpi, 1000, 5000);
    configure_trackbar(&app.bar_logobright, 5, 20);
    configure_trackbar(&app.bar_scrollbright, 5, 20);

    add_style(&app.chk_samebright.handle, BS_TOP);
    // add_style(&app.bar_scrollbright.handle, BS_CENTER);

    let available_devices = DeathAdderV2::list().unwrap_or_else(
        |e| msgboxpanic!("Error querying DeathAdder v2 devices: {}", e)
    );

    app.cmb_device.set_collection(available_devices);
    // if only 1, select it by default and show appropriate error if failed to open
    if app.cmb_device.len() == 1 {
        app.cmb_device.set_selection(Some(0));
        app.device_selected();
    }
    nwg::dispatch_thread_events();
}
