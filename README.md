# deathadderv2-rgb

Set (a constant) color on the Razer DeathAdder v2. (while practising in rust)

A little utility for those of us that don't want to run 2-3 apps and 6 services (!!) just for keeping this mouse from changing colors like a christmas tree. Personally, I don't care about the auto-switching of profiles that Synapse provides and all the other functionality I can have without running Razer's apps in the background.

Unfortunately, the device does not remember the color and it comes back with the rainbow on power on (either after sleep/hibernation or on boot). Not going to bother making it into a service as the task scheduler suits me just fine (read below if you're interested in maintaining the setting).

## Requirements

This is not for supposed to be for Linux hosts. If you are on Linux, see [openrazer](https://github.com/openrazer/openrazer).

Windows users, only requirement is to be using the [libusb driver](https://github.com/libusb/libusb/wiki/Windows) (either WinUSB or libusb-win32).

One way to install it is using [Zadig](https://zadig.akeo.ie/). You only need to do this once. Change the entry "Razer DeathAdder V2 (Interface 3)". Use the spinner to select either "WinUSB (vXXX)" or "libusb-win32 (vX.Y.Z)" and hit "Replace driver". In my case (Win11) it timed out while creating a restore point but it actually installed it.

## Usage

The tool comes in two forms, a console executable that you can use like so:

```
> deathadder-rgb-cli.exe aabbcc
```

and a GUI app that will just pop up a color picker prompt (check the mouse while selecting). 

You can use the GUI version with command line arguments too (same usage as above), except a console window will not be allocated (this is intentional).

I have not found a way to retrieve the current color from the device so both apps will save the last sent color to a file under %APPDATA%/deathadder/config/default-config.toml.

### Bonus

It is actually possible to set a different color on the scroll wheel (Synapse doesn't support this at the time of this writing). But there's a catch: most combinations don't work and I don't understand why. For sure it accepts combinations when the RGB components in both colors are the same even if in different order. For instance, the following will work:

```
> deathadder-rgb-cli.exe 1bc c1b
> deathadder-rgb-cli.exe 1155AA AA5511
> deathadder-rgb-cli.exe 10f243 f24310
```

Edit: apparently I'm missing a simple XOR kind-of checksum calculation in the USB report packet, which, for the following combinations, ends up the same (!). Thanks [openrazer](https://github.com/openrazer/openrazer).

### Task Scheduler: re-applying the setting

The GUI version also supports `--last` as the first argument in which case it sets the last applied color (either from cli or gui). This is useful if you want to schedule a task that does not pop up any windows.

A tested setup is to set a trigger at log on, and for waking up from sleep, a custom trigger on Power-Troubleshooter with event ID 1 and delay 5 seconds. In Action tab use the absolute path to `deathadder-rgb-gui.exe` and in the arguments put `--last`. I've added the (redacted) xml to the task I used in case you want to try importing it; just make sure to edit the required fields therein, it is not supposed to work as is.

## Technical

I captured the USB using UsbPcap while Synapse was sending the color-setting commands (it was a single control transfer-write, multiple times to provide that fade effect) and replaced the RGB values in it. The rest of the packet is identical. Haven't tested in any mouse other than mine; not sure if there's anything device-specific in there that would prevent others from using it. 

The USB message header was:

```
Setup Data
    bmRequestType: 0x21
    bRequest: SET_REPORT (0x09)
    wValue: 0x0300
    wIndex: 0
    wLength: 90
    Data Fragment: 001f[...]
```

And this is would be the payload for setting the color to bright white:

```
File: lib/src/lib.rs

[...]
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

[...]
```

