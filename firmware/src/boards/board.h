// Selects the board definition for the current build.
//
// Every board header must define:
//   ESPIA_BOARD_NAME        Human-readable board name.
//   ESPIA_DISPLAY_WIDTH     Display width in pixels.
//   ESPIA_DISPLAY_HEIGHT    Display height in pixels.
//   ESPIA_DISPLAY_MONO      1 for monochrome displays, 0 for color.
//   ESPIA_DISPLAY_TOUCH     1 if the display has touch input, 0 otherwise.
//   ESPIA_PIN_BUTTON        GPIO of the user button, or -1 if none.
//
// Board-specific pins (display bus, power rails) are defined as needed by
// the matching UI and hardware modules.

#pragma once

#if defined(ESPIA_BOARD_HELTEC_WIFI_KIT_32_V3)
#include "heltec_wifi_kit_32_v3.h"
#elif defined(ESPIA_BOARD_HELTEC_WIFI_KIT_32_V2)
#include "heltec_wifi_kit_32_v2.h"
#else
#error "No board selected. Build with one of the environments in platformio.ini."
#endif
