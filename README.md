# deathadderv2

Set (a constant) color on the Razer DeathAdder v2. (while practising in rust)

A little utility for those of us that don't want to run 2-3 apps and 6 services (!!) just for keeping this mouse from changing colors like a christmas tree. I don't need the auto-switching of profiles that Synapse provides and (most if not all) of the other functionality I can have without running Razer's apps in the background.

Device protocol largely ported from [openrazer](https://github.com/openrazer/openrazer). So far I've ported all of the functionality for this particular mouse except wave/breath/spectrum/whatnot effects. The plan is to write a small UI to control settings like DPI, poll rate and brightness before I integrate RGB effects, if ever.

## Requirements

This is not supposed to be for Linux hosts. If you are on Linux, see [openrazer](https://github.com/openrazer/openrazer), it's a great project, and supports many more features, as well as almost all devices.

For Windows users, the only requirement is to be using the [libusb driver](https://github.com/libusb/libusb/wiki/Windows) (either WinUSB or libusb-win32).

One way to install it is using [Zadig](https://zadig.akeo.ie/). You only need to do this once. Change the entry "Razer DeathAdder V2 (Interface 3)" by using the spinner to select either "WinUSB (vXXX)" or "libusb-win32 (vX.Y.Z)" and hitting "Replace driver". In my case (Win11) it timed out while creating a restore point but it actually installed it.

## Usage

The tool comes in two forms, a console executable that you can use like so:

```
> deathadder-rgb-cli.exe [COLOR|LOGO_COLOR] [SCROLL_WHEEL_COLOR]

where *COLOR above should be in hex [0x/#]RGB[h] or [0x/#]RRGGBB[h] format.
If no arguments are specified the saved color will be applied.
If scroll wheel color is not specified, the specified color
will be applied to both the logo and the scroll wheel.
```

and a GUI version that will just pop up a color picker prompt to preview and/or set both logo and scroll wheel colors to the same value.

Contrary to all other settings, I have not found a way to retrieve the current color from the device so both apps will save the last applied color to a file under %APPDATA%/deathadder/config/default-config.toml, just so it doesn't reset every time one uses the GUI tool.

---
This project is licensed under the GPL.