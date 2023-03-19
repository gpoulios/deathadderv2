# deathadderv2

A tool to configure Razer DeathAdder v2 without Synapse and co. Does not support RGB effects or profiles (at least not yet).

I just wanted static settings without having to run in the background 2-3 apps and 6 services that Razer provides. I don't know if it was just me, but every time I closed Synapse, the mouse would go back to those wave effects that were super annoying and catching the eye when typing or reading. (Also served as a reason to practice in rust a little bit; which I'm new to)

Device protocol has been largely ported from [openrazer](https://github.com/openrazer/openrazer). GUI mostly built using [native-windows-gui](https://github.com/gabdube/native-windows-gui).

So far supports the following:

- DPI
- Polling rate
- Static logo / scroll wheel color
- Logo / scroll wheel brightness

It doesn't support:

- Wave/breath/spectrum/whatnot effects
- Profiles
- Other devices

## Requirements

This is not supposed to be for Linux hosts. If you are on Linux, see [openrazer](https://github.com/openrazer/openrazer), it's a great project, and supports many more features, as well as almost all devices.

For Windows users, the only requirement is to be using the [libusb driver](https://github.com/libusb/libusb/wiki/Windows) (either WinUSB or libusb-win32).

One way to install it is using [Zadig](https://zadig.akeo.ie/). You only need to do this once. Change the entry "Razer DeathAdder V2 (Interface 3)" by using the spinner to select either "WinUSB (vXXX)" or "libusb-win32 (vX.Y.Z)" and hitting "Replace driver". In my case (Win11) it timed out while creating a restore point but it actually installed it.

## Usage

The UI  should be self-explanatory. No need to keep it running in the background.

![UI screenshot](screenshot.png?raw=true "UI screenshot")

Contrary to all other settings, I have not found a way to retrieve the current color from the device so the app will save the last applied color to a file under %APPDATA%/deathadder/config/default-config.toml, just so it doesn't reset every time it opens.

---
This project is licensed under the GPL.