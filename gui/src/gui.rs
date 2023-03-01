#![windows_subsystem = "windows"]

use std::{mem::size_of, ffi::CStr, thread, sync::{Arc, Mutex}};
use core::time::Duration;
use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::{HWND, WPARAM, LPARAM, RECT, COLORREF},
        UI::{
            WindowsAndMessaging::{
                WM_INITDIALOG, WM_COMMAND, WM_PAINT, EN_UPDATE, GetWindowTextA,
                SWP_NOSIZE, SWP_NOZORDER, GetWindowRect, GetDesktopWindow, 
                GetClientRect, SetWindowPos, MessageBoxA, MB_OK, MB_ICONERROR
            },
            Controls::Dialogs::*
        },
        System::Diagnostics::Debug::OutputDebugStringA
    },
};
use rgb::RGB8;
use libdeathadder::core::{rgb_from_hex, Config};
use libdeathadder::v2::{preview_color, set_color};


/*
 * This utility operates in either command line mode or UI mode.
 * 
 * In command line mode the colors are specified as cmd line arguments
 * and no UI is shown. The reason for this mode is to be able to automate or
 * schedule this tool using the task scheduler or smth. If this were a true
 * console application (without the #![windows_subsystem = "windows"] directive
 * above), in such scenarios (e.g. scheduled task) a console window would pop
 * up for a split second. We abandon console support to avoid that, and we
 * log error messages to the debugger using the OutputDebugString API.
 * 
 * In UI mode we show a ChooseColor dialog and let the user pick the color
 * while previewing the current selection on the mouse itself.
 * 
 * For this we need a) a separate thread (i.e. previewing thread) to update
 * the device and b) to define the color chooser's hook procedure (CCHOOKPROC)
 * in order to get the color values while the user is selecting and before they
 * press the ok button. 
 * 
 * We get RGB channel updates one-by-one in 3 consecutive WM_COMMAND(EN_UPDATE)
 * messages in CCHOOKPROC, therefore it should be more perfomant not to trigger
 * the preview thread (which would send a USB command to the mouse) on
 * each of those (partial) updates. We store those updates in `CURRENT_RGB`.
 * 
 * A full update is assumed to be when the WM_PAINT message is sent, at which
 * point we update `RGB_TO_SET` to be picked up by the preview thread.
 */
static mut CURRENT_RGB: [u8; 3] = [0u8; 3];
static RGB_TO_SET: Mutex<Option<RGB8>> = Mutex::new(None);


/*
 * Log messages to the debugger using OutputDebugString (only for command line
 * invocation). Use DebugView by Mark Russinovich to view
 */
macro_rules! dbglog {
    ($($args: tt)*) => {
        unsafe {
            let msg = format!($($args)*);
            OutputDebugStringA(PCSTR::from_raw(msg.as_ptr()));
        }
    }
}

macro_rules! dbgpanic {
    ($($args: tt)*) => {
        unsafe {
            let msg = format!($($args)*);
            OutputDebugStringA(PCSTR::from_raw(msg.as_ptr()));
            panic!("{}", msg);
        }
    }
}

fn main() {

    /*
     * Command line mode if at least one argument
     */
    let args: Vec<String> = std::env::args().collect();

    let parse_arg = |input: &str| -> RGB8 {
        match rgb_from_hex(input) {
            Ok(rgb) => rgb,
            Err(e) => { dbgpanic!("argument '{}' should be in the \
                form [0x/#]RGB[h] or [0x/#]RRGGBB[h] where R, G, and B are hex \
                digits: {}", input, e); } 
        }
    };

    if args.len() > 1 {
        let (color, wheel_color) = if args[1] == "--last" {
            match Config::load() {
                Some(cfg) => (cfg.color, cfg.wheel_color),
                None => dbgpanic!("failed to load configuration; please specify \
                    arguments manually")
            }
        } else {
            (parse_arg(args[1].as_ref()), if args.len() > 2 {
                Some(parse_arg(args[2].as_ref()))
            } else {
                None
            })
        };

        match set_color(color, wheel_color) {
            Ok(msg) => dbglog!("{}", msg),
            Err(e) => dbgpanic!("Failed to set color(s): {}", e)
        }
        return;
    };


    /*
     * no arguments; UI mode
     */

    // this will be the master signal to end the device preview thread
    let keep_previewing = Arc::new(Mutex::new(true));

    let preview_thread = {

        // make a copy of the master signal and loop on it
        let keep_previewing = Arc::clone(&keep_previewing);
        thread::spawn(move || {

            // save some resources by setting each color once
            let mut last_set: Option<RGB8> = None;

            while *keep_previewing.lock().unwrap() {

                match *RGB_TO_SET.lock().unwrap() {
                    same if same == last_set => (),
                    None => (),
                    Some(rgb) => {
                        _ = preview_color(rgb, None);
                        last_set = Some(rgb);
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

    // block waiting the user to choose
    let chosen = ui_choose_color(initial);

    // make sure the thread has stopped previewing on the device
    *keep_previewing.lock().unwrap() = false;
    preview_thread.join().unwrap();

    // set the final value based on user's selection
    let result = if chosen.is_some() {
        set_color(chosen.unwrap(), None)
    } else if cfg.is_some() {
        let cfgu = cfg.unwrap();
        set_color(cfgu.color, cfgu.wheel_color)
    } else {
        set_color(initial, None)
    };

    // show error, if any
    if result.is_err() {
        unsafe {
            let message = PCSTR::from_raw(result.unwrap().as_ptr());
            MessageBoxA(HWND(0), message, s!("Error"), MB_OK | MB_ICONERROR);
        }
    }
}

/*
 * Init and show ChooseColor dialog, blocking until the user dismisses it.
 * In the meantime, preview colors by hooking it with a CCHOOKPROC.
 */
fn ui_choose_color(initial: RGB8) -> Option<RGB8> {
    unsafe {
        let mut initial_cr = COLORREF(
            initial.r as u32 | 
            (initial.g as u32) << 8 | 
            (initial.b as u32) << 16);

        let mut cc = CHOOSECOLORA {
            lStructSize: size_of::<CHOOSECOLORA>() as u32,
            rgbResult: initial_cr,
            lpCustColors: &mut initial_cr,
            Flags: CC_FULLOPEN | CC_ANYCOLOR | CC_RGBINIT | CC_ENABLEHOOK | CC_PREVENTFULLOPEN,
            lpfnHook: Some(cc_hook_proc),
            lpTemplateName: PCSTR::null(),
            ..Default::default()
        };

        let res = ChooseColorA(&mut cc).into();
        if res {
            Some(RGB8{
                r: (cc.rgbResult.0 & 0xff) as u8,
                g: ((cc.rgbResult.0 >> 8) & 0xff) as u8,
                b: ((cc.rgbResult.0 >> 16) & 0xff) as u8,
            })
        } else {
            None
        }
    }
}

/*
 * std::ffi::CStr::from_bytes_until_nul() is atm nightly experimental API so
 * we need this to convert a byte array with one or more null terminators in it
 */
unsafe fn u8sz_to_u8(s: &[u8]) -> u8 {
    let str = CStr::from_ptr(s.as_ptr() as *const _).to_str().unwrap();
    str.parse::<u8>().unwrap()
}

/*
 * The CCHOOKPROC used for 2 things: 1) to center our orphan dialog and 2)
 * to fetch color udpates before pressing Ok.
 */
unsafe extern "system" fn cc_hook_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> usize {
    match msg {

        WM_INITDIALOG => {
            // center the color chooser on the desktop
            let mut rc = RECT::default();
            let mut desktop_rc = RECT::default();
            if GetWindowRect(hwnd, &mut rc).into() && 
                GetClientRect(GetDesktopWindow(), &mut desktop_rc).into() {
                rc.left = (desktop_rc.right/2) - ((rc.right - rc.left)/2);
                rc.top = (desktop_rc.bottom/2) - ((rc.bottom - rc.top)/2);
                SetWindowPos(hwnd, HWND(0), rc.left, rc.top, 0, 0,
                    SWP_NOZORDER | SWP_NOSIZE);
            }
        },

        WM_COMMAND => {
            // update one RGB channel
            let cmd = (wparam.0 >> 16) as u32;
            let ctrl_id = wparam.0 & 0xffff;
            let ctrl_handle = HWND(lparam.0);
            
            // used WinId to get the textboxes' ids (0x2c2,3,4)
            if cmd == EN_UPDATE && 0x2c2 <= ctrl_id && ctrl_id <= 0x2c4 {
                let mut text = [0u8; 10];
                let len = GetWindowTextA(ctrl_handle, &mut text);
                if 0 < len && len <= 3 {
                    CURRENT_RGB[ctrl_id - 0x2c2] = u8sz_to_u8(&text);
                }
            }
        },

        WM_PAINT => {
            // commit the full RGB change
            let mut rgb = RGB_TO_SET.lock().unwrap();
            *rgb = Some(
                RGB8::new(CURRENT_RGB[0], CURRENT_RGB[1], CURRENT_RGB[2])
            );
        }
        _ => ()
    }
    0
}