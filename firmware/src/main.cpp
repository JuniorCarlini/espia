// espia firmware entry point.

#include <Arduino.h>

#include "boards/board.h"
#include "ui/ui.h"
#include "version.h"

void setup() {
    Serial.begin(115200);
    Serial.printf("espia %s on %s\n", ESPIA_FIRMWARE_VERSION,
                  ESPIA_BOARD_NAME);

    ui::begin();
    ui::showBoot(ESPIA_FIRMWARE_VERSION, ESPIA_BOARD_NAME);
}

void loop() {
    delay(1000);
}
