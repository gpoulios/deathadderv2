use std::{mem::{size_of, MaybeUninit}, thread::{self, JoinHandle}, ffi::CStr};
use windows::{
    core::{PCSTR},
    Win32::{
        Foundation::{HWND, WPARAM, LRESULT, LPARAM, RECT, COLORREF},
        UI::{
            WindowsAndMessaging::{
                WM_INITDIALOG, WM_COMMAND, WM_PAINT, EN_UPDATE, GetWindowTextA,
                SWP_NOSIZE, SWP_NOZORDER, GetWindowRect, GetDesktopWindow,
                GetClientRect, SetWindowPos,
                SetWindowLongPtrA, GetWindowLongPtrA, DWLP_MSGRESULT, WINDOW_LONG_PTR_INDEX,
            },
            Controls::Dialogs::*
        },
    },
};
use rgb::RGB8;

/*
 * trying hard to follow:
 * https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowlongptra
 * We're gonna need DWLP_USER to set a pointer to our Color Dialog struct
 * on the winapi Dialog window (GWLP_USERDATA only works for window class)
 */
const DWLP_DLGPROC: u32 = DWLP_MSGRESULT + size_of::<LRESULT>() as u32;
const DWLP_USER: WINDOW_LONG_PTR_INDEX = WINDOW_LONG_PTR_INDEX(
    (DWLP_DLGPROC + size_of::<LPARAM>() as u32) as i32);

/*
 * std::ffi::CStr::from_bytes_until_nul() is atm nightly experimental API so
 * we need this to convert a byte array with one or more null terminators in it
 */
unsafe fn u8sz_to_u8(s: &[u8]) -> u8 {
    let str = CStr::from_ptr(s.as_ptr() as *const _).to_str().unwrap();
    str.parse::<u8>().unwrap()
}

pub type ColorChangeCallback<'a> = dyn Fn(&ColorDialog, &RGB8) + Send + Sync + 'a;

pub struct ColorDialog<'a> {
    current: [u8; 3],
    last_notified: RGB8,
    change_cb: Option<Box<ColorChangeCallback<'a>>>,
}

impl Default for ColorDialog<'_> {
    fn default() -> Self {
        unsafe {
            // safe as 0 a valid bit-pattern for all fields
            MaybeUninit::<Self>::zeroed().assume_init()
        }
    }
}

impl<'a> ColorDialog<'a> {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn show_async<F>(
        &'static mut self,
        parent: HWND,
        initial: Option<RGB8>,
        change_cb: Option<F>
    ) -> JoinHandle<Option<RGB8>> 
    where F: Fn(&ColorDialog, &RGB8) + Send + Sync + 'a {
        thread::spawn(move || {
            self.show(parent, initial, change_cb)
        })
    }

    pub fn show<F>(
        &mut self,
        parent: HWND,
        initial: Option<RGB8>,
        change_cb: Option<F>,
    ) -> Option<RGB8> 
    where F: Fn(&ColorDialog, &RGB8) + Send + Sync + 'a {
        unsafe {
            let initial = initial.unwrap_or(RGB8::new(0xaa, 0xaa, 0xaa));

            // init these so we don't trigger an unnecessary 'change' event on bootstrap
            self.current = [initial.r, initial.g, initial.b];
            self.last_notified = initial;

            self.change_cb = match change_cb {
                Some(cb) => Some(Box::new(cb)),
                None => None
            };

            // will set lCustData to self so we can access it in the hook proc
            let this_lp = LPARAM((self as *mut Self) as isize);

            // this will be both the initial and custom colors for now
            let mut initial_cr = COLORREF(
                initial.r as u32 | 
                (initial.g as u32) << 8 | 
                (initial.b as u32) << 16);
    
            let mut cc = CHOOSECOLORA {
                lStructSize: size_of::<CHOOSECOLORA>() as u32,
                hwndOwner: parent,
                rgbResult: initial_cr,
                lpCustColors: &mut initial_cr,
                Flags: CC_FULLOPEN | CC_ANYCOLOR | CC_RGBINIT | CC_ENABLEHOOK | CC_PREVENTFULLOPEN,
                lpfnHook: Some(cc_hook_proc),
                lpTemplateName: PCSTR::null(),
                lCustData: this_lp,
                ..Default::default()
            };
    
            let ok = ChooseColorA(&mut cc).into();
            if ok {
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
}

/*
 * The CCHOOKPROC used for 2 things: 
 *  1) to center our orphan dialog if it has no parent and
 *  2) to generate color change events
 * 
 * We get RGB channel updates one-by-one in 3 consecutive WM_COMMAND(EN_UPDATE)
 * messages in CCHOOKPROC, therefore it should be more perfomant not to trigger
 * any listener callbacks (in this case our preview thread which would send a
 * USB command to the mouse) on each of those (partial) updates. We store those
 * updates in `ColorDialog.current`.
 * 
 * A full update is assumed to be when the WM_PAINT message is sent, at which
 * point we invoke the `change_cb`.
 */
unsafe extern "system" fn cc_hook_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM
) -> usize {
    match msg {

        WM_INITDIALOG => {
            // save our pointer to this instance of the ColorDialog struct
            let cc_ptr = lparam.0 as *const CHOOSECOLORA;
            let cc = &cc_ptr.read();
            SetWindowLongPtrA(hwnd, DWLP_USER, cc.lCustData.0);

            if cc.hwndOwner.0 == 0 {
                // center our dialog window on the desktop
                let mut rc = RECT::default();
                let mut desktop_rc = RECT::default();

                if GetWindowRect(hwnd, &mut rc).into() && 
                    GetClientRect(GetDesktopWindow(), &mut desktop_rc).into() {

                    rc.left = (desktop_rc.right/2) - ((rc.right - rc.left)/2);
                    rc.top = (desktop_rc.bottom/2) - ((rc.bottom - rc.top)/2);

                    SetWindowPos(hwnd, HWND(0), rc.left, rc.top, 0, 0,
                        SWP_NOZORDER | SWP_NOSIZE);
                }
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

                    // update this.current
                    let this_lp = GetWindowLongPtrA(hwnd, DWLP_USER);
                    if this_lp != 0 {
                        let this_ptr = this_lp as *mut ColorDialog;
                        let this = this_ptr.as_mut().unwrap();

                        this.current[ctrl_id - 0x2c2] = u8sz_to_u8(&text);
                    }
                }
            }
        },

        WM_PAINT => {
            // trigger the change event
            let this_lp = GetWindowLongPtrA(hwnd, DWLP_USER);
            if this_lp != 0 {
                let this_ptr = this_lp as *mut ColorDialog;
                let this = this_ptr.as_mut().unwrap();

                let rgb = RGB8::from(this.current);
                if rgb != this.last_notified {
                    this.last_notified = rgb;

                    if this.change_cb.is_some() {
                        let cb = this.change_cb.as_ref().unwrap().as_ref();
                        cb(this_ptr.as_ref().unwrap(), &rgb);
                    }
                }
            }
        },
        
        _ => ()
    }
    0
}