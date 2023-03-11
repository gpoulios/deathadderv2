pub mod cfg;
pub mod error;
pub mod device;

pub mod common {
    use std::{num::ParseIntError, thread, time::Duration};
    use rusb::{Context, DeviceHandle, UsbContext};
    use core::mem::{size_of, size_of_val, MaybeUninit};
    use rgb::{RGB8, FromSlice};
    use crate::error::{ParseRGBError, USBResult, USBError};
    
    pub fn rgb_from_hex(input: &str) -> Result<RGB8, ParseRGBError> {
        let s = input
            .trim_start_matches("0x")
            .trim_start_matches("#")
            .trim_end_matches("h");
    
        match s.len() {
            3 => {
                match s.chars()
                    .map(|c| u8::from_str_radix(format!("{}{}", c, c).as_str(), 16))
                    .collect::<Result<Vec<u8>, ParseIntError>>() {
                    Ok(res) => Ok(res.as_rgb()[0]),
                    Err(pie) => Err(ParseRGBError::from(pie))
                }
            },
            6 => {
                match (0..s.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
                    .collect::<Result<Vec<u8>, ParseIntError>>() {
                    Ok(res) => Ok(res.as_rgb()[0]),
                    Err(pie) => Err(ParseRGBError::from(pie))
                }
            },
            _ => {
                Err(ParseRGBError::WrongLength(s.len()))
            }
        }
    }

    // tried also 1ms with varying results
    static USB_RECEIVER_WAIT: Duration = Duration::from_millis(10);
    static USB_TXFER_TIMEOUT: Duration = Duration::from_secs(1);

    // const RAZER_USB_REPORT_LEN: usize = 0x5A;

    #[repr(u8)]
    #[derive(Debug, Copy, Clone)]
    pub enum LedState {
        Off = 0x00,
        On = 0x01,
    }

    #[repr(u8)]
    #[derive(Debug, Copy, Clone)]
    pub enum LedStorage {
        NoStore = 0x00,
        VarStore = 0x01,
    }

    #[repr(u8)]
    #[derive(Debug, Copy, Clone)]
    pub enum Led {
        Zero = 0x00,
        ScrollWheel = 0x01,
        Battery = 0x03,
        Logo = 0x04,
        Backlight = 0x05,
        Macro = 0x07,
        Game = 0x08,
        RedProfile = 0x0C,
        GreenProfile = 0x0D,
        BlueProfile = 0x0E,
        RightSide = 0x10,
        LeftSide = 0x11,
        ArgbCh1 = 0x1A,
        ArgbCh2 = 0x1B,
        ArgbCh3 = 0x1C,
        ArgbCh4 = 0x1D,
        ArgbCh5 = 0x1E,
        ArgbCh6 = 0x1F,
        Charging = 0x20,
        FastCharging = 0x21,
        FullyCharged = 0x22
    }

    #[repr(u8)]
    #[derive(Debug, Copy, Clone)]
    pub enum LedEffect {
        None = 0x00,
        Static = 0x01,
        Breathing = 0x02,
        Spectrum = 0x03,
        Wave = 0x04,
        Reactive = 0x05,
        Starlight = 0x07,
        CustomFrame = 0x08,
    }

    #[repr(u8)]
    #[derive(Debug, Copy, Clone)]
    enum CmdStatus {
        Busy = 0x01,
        Successful = 0x02,
        Failure = 0x03,
        Timeout = 0x04,
        NotSupported = 0x05,
    }

    impl TryFrom<u8> for CmdStatus {
        type Error = u8;

        fn try_from(byte: u8) -> Result<CmdStatus, Self::Error> {
            match byte {
                x if x == CmdStatus::Busy as u8 => Ok(CmdStatus::Busy),
                x if x == CmdStatus::Successful as u8 => Ok(CmdStatus::Successful),
                x if x == CmdStatus::Failure as u8 => Ok(CmdStatus::Failure),
                x if x == CmdStatus::Timeout as u8 => Ok(CmdStatus::Timeout),
                x if x == CmdStatus::NotSupported as u8 => Ok(CmdStatus::NotSupported),
                _ => Err(byte),
            }
        }
    }

    #[repr(u8)]
    #[derive(Debug, Copy, Clone)]
    pub enum PollingRate {
        Hz1000 = 0x01,
        Hz500 = 0x02,
        Hz250 = 0x04,
        Hz125 = 0x08,
    }

    impl TryFrom<u8> for PollingRate {
        type Error = u8;

        fn try_from(flag: u8) -> Result<PollingRate, Self::Error> {
            match flag {
                x if x == PollingRate::Hz1000 as u8 => Ok(PollingRate::Hz1000),
                x if x == PollingRate::Hz500 as u8 => Ok(PollingRate::Hz500),
                x if x == PollingRate::Hz250 as u8 => Ok(PollingRate::Hz250),
                x if x == PollingRate::Hz125 as u8 => Ok(PollingRate::Hz125),
                _ => Err(flag),
            }
        }
    }

    #[repr(C, packed)]
    #[derive(Debug, Copy, Clone)]
    pub struct RazerReport {
        status: u8,
        pub(crate) transaction_id: u8,
        remaining_packets: u16, // big endian
        protocol_type: u8, // 0x0
        data_size: u8,
        command_class: u8,
        command_id: u8,
        pub(crate) arguments: [u8; 80],
        crc: u8, // xor'ed bytes of report
        reserved: u8, // 0x0
    }

    impl Default for RazerReport {
        fn default() -> Self {
            unsafe {
                // safe as 0 a valid bit-pattern for all fields
                MaybeUninit::<Self>::zeroed().assume_init()
            }
        }
    }

    impl RazerReport {
        fn init(cmd_cls: u8, cmd_id: u8, data_size: u8) -> Self {
            Self {
                command_class: cmd_cls,
                command_id: cmd_id,
                data_size: data_size,
                ..Default::default()
            }
        }

        fn new(cmd_cls: u8, cmd_id: u8, args: &[u8]) -> Self {
            let mut r = Self {
                command_class: cmd_cls,
                command_id: cmd_id,
                data_size: args.len() as u8,
                ..Default::default()
            };
            r.arguments[..args.len()].copy_from_slice(args);
            r
        }

        fn update_crc(&mut self) -> &mut Self {
            let s = self.bytes();

            self.crc = s[2..88].iter().fold(0, |crc, x| crc ^ x);
            self
        }

        /// Converts this struct to network byte order in-place
        fn to_network_byte_order_mut(&mut self) -> &mut Self {
            self.remaining_packets =
                (self.remaining_packets & 0xff) << 8 | 
                (self.remaining_packets >> 8);
            self
        }

        /// Returns a copy of this struct in network byte order
        fn to_network_byte_order(mut self) -> Self {
            self.to_network_byte_order_mut();
            self
        }

        /// Converts this struct to host byte order in-place
        fn to_host_byte_order_mut(&mut self) -> &mut Self {
            self.to_network_byte_order_mut()
        }

        /// Returns a copy of this struct in host byte order
        fn to_host_byte_order(mut self) -> Self {
            self.to_host_byte_order_mut();
            self
        }

        /// Struct as a slice; fast, zero-copy; host byte order
        fn bytes(&self) -> &[u8] {
            unsafe {
                core::slice::from_raw_parts(
                    (self as *const Self) as *const u8,
                    size_of::<Self>(),
                )
            }
        }

        /// Initializes this struct from the given slice. No conversion to byte order.
        fn from(buffer: &[u8]) -> Option<Self> {
            let c_buf = buffer.as_ptr();
            let s = c_buf as *mut Self;
        
            if size_of::<Self>() == size_of_val(buffer) {
                unsafe {
                    let ref s2 = *s;
                    Some(*s2)
                }
            } else {
                None
            }
        }

        /// Converts to network byte order and returns a copy as_slice
        fn pack(self) -> Vec<u8> {
            self.to_network_byte_order().bytes().into()
        }

        /// Converts to network byte order in-place(!) and returns as_slice.
        /// Equivalent to self.to_network_byte_order_mut().bytes()
        #[allow(dead_code)]
        fn pack_mut(&mut self) -> &[u8] {
            self.to_network_byte_order_mut().bytes()
        }

        /// Construct from slice and return a copy in host byte order
        fn unpack(buffer: &[u8]) -> Option<Self> {
            match Self::from(buffer) {
                Some(rep) => Some(rep.to_host_byte_order()),
                None => None
            }
        }

    }

    fn razer_send_control_msg<C: UsbContext>(
        usb_dev: &DeviceHandle<C>,
        data: &RazerReport,
        report_index: u16
    ) -> USBResult<usize> {
        let request = 0x09u8; // HID_REQ_SET_REPORT
        let request_type = 0x21u8; // USB_TYPE_CLASS | USB_RECIP_INTERFACE | USB_DIR_OUT
        let value = 0x300u16;

        let written = usb_dev.write_control(
                                    request_type, request, value, report_index,
                                    &data.pack(), USB_TXFER_TIMEOUT)?;

        // wait here otherwise we fail on any subsequent HID_REQ_GET_REPORTs
        thread::sleep(USB_RECEIVER_WAIT);

        Ok(written)
    }

    fn razer_get_usb_response<C: UsbContext>(
        usb_dev: &DeviceHandle<C>,
        report_index: u16,
        request_report: &RazerReport,
        response_index: u16
    ) -> USBResult<RazerReport> {
        let written = razer_send_control_msg(
                                usb_dev, request_report, report_index)?;
        if written != size_of_val(request_report) {
            return Err(USBError::IncompleteWrite(
                        size_of_val(request_report), written));
        }

        let request = 0x01u8; // HID_REQ_GET_REPORT
        let request_type = 0xA1u8; // USB_TYPE_CLASS | USB_RECIP_INTERFACE | USB_DIR_IN
        let value = 0x300u16;
        let mut buffer = [0u8; size_of::<RazerReport>()];
        let read = usb_dev.read_control(
                            request_type, request, value, response_index,
                            &mut buffer, USB_TXFER_TIMEOUT)?;
        if read != size_of::<RazerReport>() {
            return Err(USBError::IncompleteRead(
                        size_of::<RazerReport>(), read));
        }

        // RazerReport::from() won't fail with this buf
        Ok(RazerReport::unpack(&buffer).unwrap())
    }

    fn razer_get_report<C: UsbContext>(
        usb_dev: &DeviceHandle<C>,
        request: &RazerReport
    ) -> USBResult<RazerReport> {
        let index = 0u16;
        razer_get_usb_response(usb_dev, index, request, index)
    }

    pub(crate) fn razer_send_payload<C: UsbContext>(
        usb_dev: &DeviceHandle<C>,
        request: &mut RazerReport
    ) -> USBResult<RazerReport> {
        request.update_crc();
        let response = razer_get_report(usb_dev, request)?;

        if response.remaining_packets != request.remaining_packets || 
            response.command_class != request.command_class ||
            response.command_id != request.command_id {
            return Err(USBError::ResponseMismatch);
        }

        match CmdStatus::try_from(response.status) {
            Ok(CmdStatus::Busy) => Err(USBError::DeviceBusy),
            Ok(CmdStatus::Failure) => Err(USBError::CommandFailed),
            Ok(CmdStatus::NotSupported) => Err(USBError::CommandNotSupported),
            Ok(CmdStatus::Timeout) => Err(USBError::CommandTimeout),
            Ok(CmdStatus::Successful) => Ok(response),
            Err(status) => Err(USBError::ResponseUnknownStatus(status)),
        }
    }

    pub(crate) fn razer_chroma_standard_get_serial() -> RazerReport {
        RazerReport::init(0x00, 0x82, 0x16)
    }

    pub(crate) fn razer_chroma_misc_get_dpi_xy(variable_storage: LedStorage) -> RazerReport {
        let mut report = RazerReport::init(0x04, 0x85, 0x07);
        report.arguments[0] = variable_storage as u8;
        report
    }

    pub(crate) fn razer_chroma_misc_set_dpi_xy(
        variable_storage: LedStorage,
        dpi_x: u16,
        dpi_y: u16
    ) -> RazerReport {
        // Keep the DPI within bounds
        let dpi_x = dpi_x.clamp(100, 30000);
        let dpi_y = dpi_y.clamp(100, 30000);
        RazerReport::new(0x04, 0x05, &[
            variable_storage as u8,
            ((dpi_x >> 8) & 0xFF) as u8,
            (dpi_x & 0xFF) as u8,
            ((dpi_y >> 8) & 0xFF) as u8,
            (dpi_y & 0xFF) as u8,
            0x00u8,
            0x00u8,
        ])
    }

    pub(crate) fn razer_chroma_misc_get_polling_rate() -> RazerReport {
        RazerReport::init(0x00, 0x85, 0x01)
    }

    pub(crate) fn razer_chroma_misc_set_polling_rate(polling_rate: PollingRate) -> RazerReport {
        RazerReport::new(0x00, 0x05, &[
            polling_rate as u8,
        ])
    }

    pub(crate) fn razer_naga_trinity_effect_static(
        variable_storage: LedStorage,
        effect: LedEffect,
        logo_rgb: RGB8,
        scroll_rgb: RGB8,
    ) -> RazerReport {
        RazerReport::new(0x0f, 0x03, &[
            variable_storage as u8,
            0x00, // LED ID ?
            0x00, // Unknown
            0x00, // Unknown
            effect as u8,
            scroll_rgb.r, scroll_rgb.g, scroll_rgb.b,
            logo_rgb.r, logo_rgb.g, logo_rgb.b,
        ])
    }

    fn razer_chroma_extended_matrix_effect_base(
        arg_size: u8,
        variable_storage: LedStorage,
        led: Led,
        effect: LedEffect,
    ) -> RazerReport {
        let mut report = RazerReport::init(0x0f, 0x02, arg_size);
        report.arguments[0] = variable_storage as u8;
        report.arguments[1] = led as u8;
        report.arguments[2] = effect as u8;
        report
    }

    #[allow(dead_code)]
    pub(crate) fn razer_chroma_extended_matrix_effect_none(
        variable_storage: LedStorage,
        led: Led,
    ) -> RazerReport {
        razer_chroma_extended_matrix_effect_base(
            0x06, variable_storage, led, LedEffect::None)
    }

    pub(crate) fn razer_chroma_extended_matrix_effect_static(
        variable_storage: LedStorage,
        led: Led,
        rgb: RGB8,
    ) -> RazerReport {
        let mut report = razer_chroma_extended_matrix_effect_base(
            0x09, variable_storage, led, LedEffect::Static);
        report.arguments[5] = 0x01;
        report.arguments[6] = rgb.r;
        report.arguments[7] = rgb.g;
        report.arguments[8] = rgb.b;
        report
    }

    pub(crate) fn razer_chroma_extended_matrix_brightness(
        variable_storage: LedStorage,
        led: Led,
        brightness: u8,
    ) -> RazerReport {
        RazerReport::new(0x0F, 0x04, &[
            variable_storage as u8,
            led as u8,
            (255.0 * brightness.clamp(0, 100) as f32 / 100.0).round() as u8,
        ])
    }

    pub(crate) fn razer_chroma_extended_matrix_get_brightness(
        variable_storage: LedStorage,
        led: Led,
    ) -> RazerReport {
        RazerReport::new(0x0F, 0x84, &[
            variable_storage as u8,
            led as u8,
            0x00, // brightness
        ])
    }
}
