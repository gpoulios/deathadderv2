#![windows_subsystem = "windows"]

use std::{thread, sync::{Arc, Mutex}};
use core::time::Duration;
use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::HWND,
        UI::{
            WindowsAndMessaging::{MessageBoxA, MB_OK, MB_ICONERROR},
        },
    },
};
use rgb::RGB8;
use librazer::cfg::Config;
use librazer::device::{DeathAdderV2, RazerMouse};

pub mod color_chooser;
use color_chooser::ColorDialog;


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


/*
 * Log messages to the debugger using OutputDebugString (only for command line
 * invocation). Use DebugView by Mark Russinovich to view
 */
// macro_rules! dbglog {
//     ($($args: tt)*) => {
//         unsafe {
//             let msg = format!($($args)*);
//             OutputDebugStringA(PCSTR::from_raw(msg.as_ptr()));
//         }
//     }
// }

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

fn main() {

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
    let initial = match cfg {
        Some(ref cfg) => cfg.color,
        None => RGB8::default()
    };

    let mut cdlg = ColorDialog::new();

    // our 'change' event listener
    let change_cb = |_: &ColorDialog, color: &RGB8| {
        // commit the RGB change for the previewing thread to pick up
        let mut rgb = RGB_TO_SET.lock().unwrap();
        *rgb = Some(*color);
    };

    // block waiting the user to choose
    let chosen = cdlg.show(HWND(0), initial, Some(change_cb));

    // make sure the thread has stopped previewing on the device
    *keep_previewing.lock().unwrap() = false;
    preview_thread.join().unwrap();

    // final value based on user's choice 
    let (logo_rgb, scroll_rgb) = if chosen.is_some() {
        (chosen.unwrap(), chosen.unwrap())
    } else {
        (initial, initial)
    };

    _ = (*dav2_rc).set_logo_color(logo_rgb)
        .map_err(|e| msgboxpanic!("Error setting logo color: {}", e))
        .and_then(|_| (*dav2_rc).set_scroll_color(scroll_rgb))
        .map_err(|e| msgboxpanic!("Error setting scroll wheel color: {}", e));

    _ = Config {
        color: logo_rgb,
        scroll_color: Some(scroll_rgb),
    }.save().map_err(|e| msgboxpanic!("Failed to save config: {}", e));
}
