// Display-independent user interface API.
//
// Each display class (for example, small monochrome OLEDs or color TFTs)
// provides its own implementation, because layouts for a 128x64 OLED and a
// 320x240 TFT have little in common. See docs/adr/0006.

#pragma once

namespace ui {

// Powers up and initializes the display.
void begin();

// Shows the boot screen with the firmware version and board name.
void showBoot(const char *firmwareVersion, const char *boardName);

}  // namespace ui
