// #![windows_subsystem = "windows"]

use std::{thread, sync::{Arc, Mutex}, cell::RefCell};
use core::time::Duration;
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
                GWL_STYLE, MessageBoxA, MB_OK, MB_ICONERROR,
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
    ($($args: tt)*) => {
        unsafe {
            let msg = format!($($args)*);
            OutputDebugStringA(PCSTR::from_raw(msg.as_ptr()));
            println!("{}", msg);
        }
    }
}

// macro_rules! dbgpanic {
//     ($($args: tt)*) => {
//         unsafe {
//             let msg = format!($($args)*);
//             OutputDebugStringA(PCSTR::from_raw(msg.as_ptr()));
//             panic!("{}", msg);
//         }
//     }
// }

macro_rules! msgboxpanic {
    ($($args: tt)*) => {
        unsafe {
            let msg = format!($($args)*);
            let msg_ptr = PCSTR::from_raw(msg.as_ptr());
            MessageBoxA(HWND(0), msg_ptr, s!("Error"), MB_OK | MB_ICONERROR);
            panic!("{}", msg);
        }
    }
}

macro_rules! msgboxerror {
    ($($args: tt)*) => {
        unsafe {
            let msg = format!($($args)*);
            let msg_ptr = PCSTR::from_raw(msg.as_ptr());
            MessageBoxA(HWND(0), msg_ptr, s!("Error"), MB_OK | MB_ICONERROR);
        }
    }
}


#[derive(Default, NwgUi)]
pub struct DeathAdderv2App {
    #[nwg_control(size: (500, 310), center: true, title: "Razer DeathAdder v2 configuration")]
    #[nwg_events( OnWindowClose: [DeathAdderv2App::window_close(SELF)])]
    window: nwg::Window,

    #[nwg_layout(parent: window, min_size: [400, 200], max_column: Some(10))]
    grid: nwg::GridLayout,

    #[nwg_control(text: "Device:", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, col_span: 2)]
    label1: nwg::Label,

    #[nwg_control(v_align: nwg::VTextAlign::Top)] // has trouble aligning vertically
    #[nwg_layout_item(layout: grid, col: 2, col_span: 7)]
    #[nwg_events( OnComboxBoxSelection: [DeathAdderv2App::device_selected(SELF)])]
    cmb_device: nwg::ComboBox<UsbDevice>,

    /*
     * DPI
     */
    #[nwg_control(text: "DPI:", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 1, col_span: 2)]
    lbl_dpi: nwg::Label,

    #[nwg_control(range: Some(100..20000), pos: Some(20000))]
    #[nwg_layout_item(layout: grid, row: 1, col: 2, col_span: 5)]
    #[nwg_events( 
        // Unfortunately 'TrackBarUpdated' doesn't trigger with keyboard or
        // scroll, so we update on each change, even if during mouse drag
        // this might be spamming the device
        OnHorizontalScroll: [DeathAdderv2App::dpi_selected(SELF)],
    )]
    bar_dpi: nwg::TrackBar,

    #[nwg_control(text: "20000", h_align: nwg::HTextAlign::Left, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 1, col: 7, col_span: 2)]
    txt_dpi: nwg::Label,

    /*
     * Polling rate
     */
    #[nwg_control(text: "Polling rate:", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 2, col_span: 2)]
    lbl_pollrate: nwg::Label,

    #[nwg_control(collection: PollingRate::all(), v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: grid, row: 2, col: 2, col_span: 3)]
    #[nwg_events( OnComboxBoxSelection: [DeathAdderv2App::pollrate_selected(SELF)])]
    cmb_pollrate: nwg::ComboBox<PollingRate>,

    /*
     * Logo colour
     */
    #[nwg_control(text: "Logo color:", h_align: nwg::HTextAlign::Right)]
    #[nwg_layout_item(layout: grid, row: 3, col_span: 2)]
    lbl_logocolor: nwg::Label,

    #[nwg_control(text: "", line_height: Some(20))]
    #[nwg_layout_item(layout: grid, row: 3, col: 2, col_span: 3)]
    #[nwg_events(
        MousePressLeftUp: [DeathAdderv2App::logo_color_clicked(SELF)],
        OnMouseMove: [DeathAdderv2App::set_cursor_hand(SELF)],
    )]
    btn_logocolor: nwg::RichLabel,

    // Min size
    #[nwg_control(text: "Min size:", h_align: nwg::HTextAlign::Right)]
    #[nwg_layout_item(layout: grid, row: 4, col_span: 2)]
    label4: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: grid, row: 4, col: 2, col_span: 7)]
    edit_min_size_width: nwg::TextInput,

    device: RefCell<Option<DeathAdderV2>>,
    config: RefCell<Option<Config>>,
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

    /// Sugar to avoid typing self.config.borrow().as_ref().map
    /// Note: will not execute if config is None
    fn with_config<U, F>(&self, cfg: F) -> Option<U>
    where
        F: FnOnce(&Config) -> U,
    {
        self.config.borrow().as_ref().map(cfg)
    }

    /// Sugar to avoid typing self.config.borrow().as_ref().map
    /// Note: will not execute if config is None
    fn _with_mut_config<U, F>(&self, cff: F) -> Option<U>
    where
        F: FnOnce(&mut Config) -> U,
    {
        self.config.borrow_mut().as_mut().map(cff)
    }

    fn set_enabled(&self, enabled: bool) {
        self.bar_dpi.set_enabled(enabled);
        self.cmb_pollrate.set_enabled(enabled);
        // self.btn_logocolor.set_enabled(enabled);
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
            },

            None => { // no device; set some defaults
                self.bar_dpi.set_pos(self.bar_dpi.range_min());
                self.cmb_pollrate.set_selection(None);
            },
        };

        // updates that need to happen irrespective of the result
        self.txt_dpi.set_text(&self.bar_dpi.pos().to_string());

        self.with_config(|cfg| {
            // can't take it from the device; assume it's what the config says
            self.btn_logocolor.set_background_color([cfg.color.r, cfg.color.g, cfg.color.b]);
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

        self.set_enabled(dav2.is_some());
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
        let mut cfg_binding = self.config.borrow_mut();
        let cfg = cfg_binding.as_mut();

        let device_binding = self.device.borrow();
        device_binding.as_ref().map(|dav2| {
            // map serves also as a scope constraint cause we can't let cdlg
            // and device binding both be dropped at the end of the function
            // body
            let mut dialog = ColorDialog::new();

            // ColorDialog arguments
            let parent = HWND(self.window.handle.hwnd().unwrap() as isize);
            let init_logo = cfg.as_ref().map(|cfg| cfg.color);
            let init_scroll = cfg.as_ref().map(|cfg| cfg.scroll_color)
                    .unwrap_or(init_logo);
            let change_cb = Some(move |_: &ColorDialog, &color: &RGB8| {
                _ = dav2.preview_static(color, init_scroll.unwrap_or(color));
            });

            // show the dialog and choose what to apply
            let color = match dialog.show(parent, init_logo, change_cb) {
                Some(color) => Some(color),
                None => init_logo,
            };

            // apply the color (either back to initial or the newly chosen)
            color.map(|c| {
                self.with_device(|dav2| {
                    _ = dav2.set_logo_color(c);
                    _ = dav2.set_scroll_color(init_scroll.unwrap_or(c));
                });
                cfg.map(|cfg| cfg.color = c);
            });

        }); // <- cdlg dropped and destroyed here

        // device_binding dropped here so that it may outlive cdlg (and change_cb)
    }

    fn window_close(&self) {
        self.with_config(|cfg| cfg.save());
        nwg::stop_thread_dispatch();
    }
}

/*
 * We show a ChooseColor dialog and let the user pick the color
 * while previewing the current selection on the mouse itself.
 *
 * For this we need a) a separate thread (i.e. previewing thread) to update
 * the device and b) to define the color chooser's hook procedure (CCHOOKPROC)
 * in order to get the color values while the user is selecting and before they
 * press the ok button. (a) happens here, while (b) in color_chooser.rs.
 */
static RGB_TO_SET: Mutex<Option<RGB8>> = Mutex::new(None);

fn main() {

    let available_devices = DeathAdderV2::list().unwrap_or_else(
        |e| msgboxpanic!("Error querying DeathAdder v2 devices: {}", e)
    );

    _ = nwg::init().map_err(
        |e| msgboxpanic!("Failed to init Native Windows GUI: {}", e));
    _ = nwg::Font::set_global_family("Segoe UI").map_err(
        |e| dbglog!("Failed to set default font: {}", e));

    let app = DeathAdderv2App::build_ui(Default::default())
        .unwrap_or_else(|e| msgboxpanic!("Failed to build UI: {}", e));

    app.config.replace(Config::load());

    // default to all disabled, and if a valid device is selected we'll enable
    app.set_enabled(false);

    // configure a few things on the trackbar
    unsafe {
        let hbar = HWND(app.bar_dpi.handle.hwnd().unwrap() as isize);
        SendMessageA(hbar, TBM_SETLINESIZE, WPARAM(0), LPARAM(1000));
        SendMessageA(hbar, TBM_SETPAGESIZE, WPARAM(0), LPARAM(5000));
        SendMessageA(hbar, TBM_SETTICFREQ, WPARAM(2000usize), LPARAM(0));

        let mut style = GetWindowLongA(hbar, GWL_STYLE);
        style = style | (TBS_TOOLTIPS | TBS_BOTTOM | TBS_DOWNISLEFT | TBS_NOTIFYBEFOREMOVE) as i32;
        SetWindowLongA(hbar, GWL_STYLE, style);
    }

    app.cmb_device.set_collection(available_devices);
    // if only 1, select it by default and show appropriate error if failed to open
    if app.cmb_device.len() == 1 {
        app.cmb_device.set_selection(Some(0));
        app.device_selected();
    }
    nwg::dispatch_thread_events();
    return;

    let dav2 = DeathAdderV2::new()
        .unwrap_or_else(|e| msgboxpanic!("Error opening device: {}", e));

    // this will be the master signal to end the device preview thread
    let keep_previewing = Arc::new(Mutex::new(true));
    let dav2_rc = Arc::new(dav2);
    let preview_thread = {

        // make a copy of the master signal and loop on it
        let keep_previewing = Arc::clone(&keep_previewing);
        let dav2_rc = Arc::clone(&dav2_rc);
        thread::spawn(move || {

            // save some resources by setting each color once
            let mut last_set: Option<RGB8> = None;

            while *keep_previewing.lock().unwrap() {

                match *RGB_TO_SET.lock().unwrap() {
                    // same as last set color: do nothing
                    same if same == last_set => (),

                    // would like this to be matched in arm above but it doesn't
                    None => (),

                    // some new color to preview
                    Some(rgb) => {
                        match (*dav2_rc).preview_static(rgb, rgb) {
                            Ok(()) => last_set = Some(rgb),
                            Err(_) => break
                        }
                    },
                }

                // don't overkill; 10ms interval is smooth enough
                thread::sleep(Duration::from_millis(10));
            }
            // preview thread exit

        }) // return the thread handle
    };

    // set initial chooser UI color based on config (if any)
    let cfg = Config::load();
    let initial = cfg.map(|cfg| cfg.color);

    let mut cdlg = ColorDialog::new();

    // our 'change' event listener
    let change_cb = Some(|_: &ColorDialog, color: &RGB8| {
        // commit the RGB change for the previewing thread to pick up
        let mut rgb = RGB_TO_SET.lock().unwrap();
        *rgb = Some(*color);
    });

    let color = match cdlg.show(HWND(0), initial, change_cb) {
        Some(color) => Some(color),
        None => initial,
    };

    // make sure the thread has stopped previewing on the device
    *keep_previewing.lock().unwrap() = false;
    preview_thread.join().unwrap();

    color.map(|rgb| {
        _ = (*dav2_rc).set_logo_color(rgb)
        .map_err(|e| msgboxpanic!("Error setting logo color: {}", e))
        .and_then(|_| (*dav2_rc).set_scroll_color(rgb))
        .map_err(|e| msgboxpanic!("Error setting scroll wheel color: {}", e));

        _ = Config {
            color: rgb,
            scroll_color: Some(rgb),
        }.save().map_err(|e| msgboxpanic!("Failed to save config: {}", e));
    });

}
