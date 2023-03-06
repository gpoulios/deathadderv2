use std::{time::Duration, fmt};
use rusb::{Context, UsbContext, DeviceHandle};
use rgb::RGB8;
use crate::cfg::Config;
use crate::error::{USBResult, USBError};
use crate::common::*;

pub(crate) const USB_VENDOR_ID_RAZER: u16 = 0x1532;
pub(crate) const USB_DEVICE_ID_RAZER_DEATHADDER_V2: u16 = 0x0084;

pub trait RazerDevice: fmt::Display {
    fn vid(&self) -> u16 { USB_VENDOR_ID_RAZER }

    fn pid(&self) -> u16;    

    fn name(&self) -> String;

    fn handle(&self) -> &DeviceHandle<Context>;

    fn default_tx_id(&self) -> u8;

    fn send_payload(&self, request: &mut RazerReport) -> USBResult<RazerReport> {
        request.transaction_id = self.default_tx_id();
        razer_send_payload(self.handle(), request)
    }

    fn get_serial(&self) -> USBResult<String> {
        let mut request = razer_chroma_standard_get_serial();
        let response = self.send_payload(&mut request)?;
        
        let bytes = response.arguments[..22].iter()
            .take_while(|&&c| c != 0)
            .cloned()
            .collect::<Vec<u8>>();

        Ok(String::from_utf8(bytes).unwrap_or(String::from("<non-UTF8 serial>")))
    }
}

/// A default implementation; Some mice need specialization
pub trait RazerMouse: RazerDevice {
    fn get_dpi(&self) -> USBResult<(u16, u16)> {
        let mut request = razer_chroma_misc_get_dpi_xy(LedStorage::NoStore);
        let response = self.send_payload(&mut request)?;
        
        let dpi_x = ((response.arguments[1] as u16) << 8) | (response.arguments[2] as u16) & 0xff;
        let dpi_y = ((response.arguments[3] as u16) << 8) | (response.arguments[4] as u16) & 0xff;

        Ok((dpi_x, dpi_y))
    }

    fn set_dpi(&self, dpi_x: u16, dpi_y: u16) -> USBResult<()> {
        let mut request = razer_chroma_misc_set_dpi_xy(
            LedStorage::NoStore, dpi_x, dpi_y);
        self.send_payload(&mut request)?;
        Ok(())
    }

    fn get_poll_rate(&self) -> USBResult<PollingRate> {
        let mut request = razer_chroma_misc_get_polling_rate();
        let response = self.send_payload(&mut request)?;
        PollingRate::try_from(response.arguments[0])
            .or(Err(USBError::ResponseUnknownValue(response.arguments[0])))
    }

    fn set_poll_rate(&self, poll_rate: PollingRate) -> USBResult<()> {
        let mut request = razer_chroma_misc_set_polling_rate(poll_rate);
        self.send_payload(&mut request)?;
        Ok(())
    }

    fn preview_static(&self, logo_color: RGB8, scroll_color: RGB8) -> USBResult<()>;

    fn set_logo_color(&self, color: RGB8) -> USBResult<()> {
        let mut request = razer_chroma_extended_matrix_effect_static(
            LedStorage::VarStore, Led::Logo, color);
        self.send_payload(&mut request)?;
        Ok(())
    }

    fn set_scroll_color(&self, color: RGB8) -> USBResult<()> {
        let mut request = razer_chroma_extended_matrix_effect_static(
            LedStorage::VarStore, Led::ScrollWheel, color);
        self.send_payload(&mut request)?;
        Ok(())
    }

    fn get_logo_brightness(&self) -> USBResult<u8> {
        let mut request = razer_chroma_extended_matrix_get_brightness(
            LedStorage::VarStore, Led::Logo);

        let response = self.send_payload(&mut request)?;
        Ok((100.0 * response.arguments[2] as f32 / 255.0).round() as u8)
    }

    fn set_logo_brightness(&self, brightness: u8) -> USBResult<()> {
        let mut request = razer_chroma_extended_matrix_brightness(
            LedStorage::VarStore, Led::Logo, brightness);
        self.send_payload(&mut request)?;
        Ok(())
    }

    fn get_scroll_brightness(&self) -> USBResult<u8> {
        let mut request = razer_chroma_extended_matrix_get_brightness(
            LedStorage::VarStore, Led::ScrollWheel);

        let response = self.send_payload(&mut request)?;
        Ok((100.0 * response.arguments[2] as f32 / 255.0).round() as u8)
    }

    fn set_scroll_brightness(&self, brightness: u8) -> USBResult<()> {
        let mut request = razer_chroma_extended_matrix_brightness(
            LedStorage::VarStore, Led::ScrollWheel, brightness);
        self.send_payload(&mut request)?;
        Ok(())
    }

}

/// A default "to_string()" implementation for all RazerDevices
fn razer_dev_default_fmt<T: RazerDevice>(dev: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let serial = dev.get_serial().unwrap_or(String::from("<couldn't get serial>"));
    write!(f, "Razer {} ({})", dev.name(), serial)
}

pub struct DeathAdderV2 {
    handle: DeviceHandle<Context>,
}

impl RazerDevice for DeathAdderV2 {
    fn pid(&self) -> u16 { USB_DEVICE_ID_RAZER_DEATHADDER_V2 }

    fn name(&self) -> String {
        String::from("DeathAdder v2")
    }

    fn handle(&self) -> &DeviceHandle<Context> {
        &self.handle
    }

    fn default_tx_id(&self) -> u8 {
        0x3f // except for razer_naga_trinity_effect_static which is 0x1f
    }
}

impl RazerMouse for DeathAdderV2 {
    fn preview_static(&self, logo_color: RGB8, scroll_color: RGB8) -> USBResult<()> {
        let mut request = razer_naga_trinity_effect_static(
            LedStorage::NoStore, LedEffect::Static, logo_color, scroll_color);
        self.send_payload(&mut request)?;
        Ok(())
    }
}

impl fmt::Display for DeathAdderV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        razer_dev_default_fmt(self, f)
    }
}

impl DeathAdderV2 {
    pub fn new() -> USBResult<Self> {
        let ctx = Context::new()?;
        let handle = match ctx.open_device_with_vid_pid(
            USB_VENDOR_ID_RAZER, USB_DEVICE_ID_RAZER_DEATHADDER_V2) {
            Some(handle) => Ok(handle),
            None => Err(USBError::DeviceNotFound),
        }?;
        Ok(Self { handle: handle })
    }
}

pub fn preview_color(color: RGB8, wheel_color: Option<RGB8>) -> Result<String, String> {
    _set_color(color, wheel_color, false)
}

pub fn set_color(color: RGB8, wheel_color: Option<RGB8>) -> Result<String, String> {
    _set_color(color, wheel_color, true)
}

fn _set_color(color: RGB8, wheel_color: Option<RGB8>, save: bool) -> Result<String, String> {
    let vid = 0x1532;
    let pid = 0x0084;

    let timeout = Duration::from_secs(1);

    // save regardless of USB result and fail silently
    if save {
        _ = Config {color, scroll_color: wheel_color}.save();
    }

    match Context::new() {
        Ok(context) => match context.open_device_with_vid_pid(vid, pid) {
            Some(handle) => {

                let mut packet: Vec<u8> = vec![
                    // the start (no idea what they are)
                    0x00, 0x1f, 0x00, 0x00, 0x00, 0x0b, 0x0f, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01,

                    // wheel RGB (3B) | body RGB (3B)
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff,

                    // the trailer (no idea what they are either)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00
                ];
                
                packet.splice(16..19, color.iter());
                packet.splice(13..16, wheel_color.unwrap_or(color).iter());
                
                match handle.write_control(0x21, 9, 0x300, 0,
                    &packet, timeout) {
                        Ok(len) => Ok(format!("written {} bytes", len)),
                        Err(e) => Err(format!("could not write ctrl transfer: {}", e))
                }
            }
            None => Err(format!("could not find device {:04x}:{:04x}", vid, pid)),
        },
        Err(e) => Err(format!("could not initialize libusb: {}", e)),
    }
}