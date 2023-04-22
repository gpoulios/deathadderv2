# deathadderv2

A tool to configure the Razer DeathAdder v2 and save the settings in the on-board memory.

I just wanted static color without having to run in the background 2-3 apps and 6 services that come with Synapse. Although the device supports it, for some reason Razer's driver does not save the color in the on-board memory. As a result, you need to keep running Synapse and co. or the mouse goes back to those wave effects that I don't like as they keep catching my eye when typing or reading.

Device protocol has been largely ported from [openrazer](https://github.com/openrazer/openrazer) (except for DPI stages which I didn't find in openrazer). GUI mostly built using [native-windows-gui](https://github.com/gabdube/native-windows-gui).

So far, it supports the following (all saved on the device, including the color):

- DPI and DPI stages
- Polling rate
- Static logo and scroll wheel color
- Logo and scroll wheel brightness

It doesn't support:

- Wave/breath/spectrum effects
- Profiles
  - I believe they're emulated by Synapse and not really supported by the hardware, otherwise I'd be glad to implement them

- Other devices

## Requirements

This is not supposed to be for Linux hosts. If you are on Linux, see [openrazer](https://github.com/openrazer/openrazer), it's a great project, and supports many more features, as well as almost all devices.

For Windows users, the only requirement is to be using the [libusb driver](https://github.com/libusb/libusb/wiki/Windows) (either WinUSB or libusb-win32). One way to install it is using [Zadig](https://zadig.akeo.ie/). You only need to do this once. Change the entry "Razer DeathAdder V2 (Interface 3)" by using the spinner to select either "WinUSB (vXXX)" (recommended) or "libusb-win32 (vX.Y.Z)" and hit "Replace driver". In my case (Win11) it seemed to time out while creating a restore point but it actually installed it.

## Usage

The UI  should be self-explanatory. No need to keep it running in the background.

![UI screenshot](screenshot.png?raw=true "UI screenshot")

Contrary to all other settings, I have not found a way to retrieve the current color from the device so the app will save the last applied color to a file under %APPDATA%/deathadder/config/default-config.toml, just so it doesn't reset every time it opens.

---
This project is licensed under the GPL.