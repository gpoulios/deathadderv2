use std::ops::Deref;
use std::fmt;
use rusb::{Context, UsbContext, DeviceHandle, Device, DeviceList};
use rgb::RGB8;

use crate::error::{USBResult, USBError};
use crate::common::*;

pub(crate) const USB_VENDOR_ID_RAZER: u16 = 0x1532;
pub(crate) const USB_DEVICE_ID_RAZER_DEATHADDER_V2: u16 = 0x0084;

/// A wrapper for rusb:Device<Context> with Display, and Default
pub struct UsbDevice(Option<Device<Context>>);

impl Deref for UsbDevice {
    type Target = Option<Device<Context>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn get_device_name(handle: &DeviceHandle<Context>) -> String {
    let dev = handle.device();
    match dev.device_descriptor() {
        Ok(dd) => {
            let serial = handle.read_serial_number_string_ascii(&dd)
                                        .unwrap_or_default();
            let product = handle.read_product_string_ascii(&dd)
                                        .unwrap_or_default();
            format!("{}{}{}", product, if serial.len() > 0 {" "} else {""}, serial)
        },
        Err(_) => String::new(),
    }
}

impl fmt::Display for UsbDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsbDevice(Some(dev)) => {
                let devname = match dev.open() {
                    Ok(h) => {
                        get_device_name(&h)
                    },
                    Err(_) => String::new(),
                };
                write!(f, "{} ({}-{})", devname, dev.bus_number(), dev.address())
            },
            UsbDevice(None) => write!(f, "None")
        }
    }
}

impl Default for UsbDevice {
    fn default() -> Self {
        UsbDevice(None)
    }
}

impl UsbDevice {
    /// List all usb devices
    pub fn list() -> USBResult<Vec<UsbDevice>> {
        let ctx = Context::new()?;
        let device_list = DeviceList::new_with_context(ctx)?;
        let res = device_list.iter()
            .map(|d| UsbDevice(Some(d)))
            .collect::<Vec<UsbDevice>>();
        Ok(res)
    }

    /// List all usb devices of the specified vendor
    pub fn by_vendor(vid: u16) -> USBResult<Vec<UsbDevice>> {
        let ctx = Context::new()?;
        let device_list = DeviceList::new_with_context(ctx)?;
        let res = device_list.iter()
            .filter_map(|device| {
                match device.device_descriptor() {
                    Ok(descr) => if descr.vendor_id() == vid {
                        Some(UsbDevice(Some(device)))
                    } else {
                        None
                    },
                    Err(_) => None
                }
            })
            .collect::<Vec<UsbDevice>>();
        Ok(res)
    }

    /// List all usb devices of the specified vendor and with the specified product ID
    pub fn by_product(vid: u16, pid: u16) -> USBResult<Vec<UsbDevice>> {
        let ctx = Context::new()?;
        let device_list = DeviceList::new_with_context(ctx)?;
        let res = device_list.iter()
            .filter_map(|device| {
                match device.device_descriptor() {
                    Ok(descr) => 
                        if descr.vendor_id() == vid && descr.product_id() == pid {
                            Some(UsbDevice(Some(device)))
                        } else {
                            None
                        },
                    Err(_) => None
                }
            })
            .collect::<Vec<UsbDevice>>();
        Ok(res)
    }
}

pub trait RazerDevice: fmt::Display {
    fn list() -> USBResult<Vec<UsbDevice>> {
        UsbDevice::by_vendor(USB_VENDOR_ID_RAZER)
    }

    fn vid(&self) -> u16 { USB_VENDOR_ID_RAZER }

    fn pid(&self) -> u16;    

    fn name(&self) -> String {
        get_device_name(self.handle())
    }

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
        let b = (255.0 * brightness.clamp(0, 100) as f32 / 100.0).round() as u8;
        let mut request = razer_chroma_extended_matrix_brightness(
            LedStorage::VarStore, Led::Logo, b);
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
        let b = (255.0 * brightness.clamp(0, 100) as f32 / 100.0).round() as u8;
        let mut request = razer_chroma_extended_matrix_brightness(
            LedStorage::VarStore, Led::ScrollWheel, b);
        self.send_payload(&mut request)?;
        Ok(())
    }

}

/// A default "to_string()" implementation for all RazerDevices
fn razer_dev_default_fmt<T: RazerDevice>(dev: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let serial = dev.get_serial().unwrap_or(String::from("<couldn't get serial>"));
    write!(f, "{} ({})", dev.name(), serial)
}

pub struct DeathAdderV2 {
    handle: DeviceHandle<Context>,
}

impl RazerDevice for DeathAdderV2 {
    fn pid(&self) -> u16 { USB_DEVICE_ID_RAZER_DEATHADDER_V2 }

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

    pub fn list() -> USBResult<Vec<UsbDevice>> {
        UsbDevice::by_product(
            USB_VENDOR_ID_RAZER, USB_DEVICE_ID_RAZER_DEATHADDER_V2)
    }

    pub fn from(device: &UsbDevice) -> USBResult<Self> {
        let device = match device.as_ref() {
            Some(device) => Ok(device),
            None => Err(USBError::DeviceNotFound),
        }?;
        let handle = device.open()?;
        Ok(Self { handle: handle })
    }
}
