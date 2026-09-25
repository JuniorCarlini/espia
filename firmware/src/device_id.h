// A stable, unique id for this device (protocol `hello.device_id`).
//
// Derived from the chip's factory-programmed MAC address (`ESP.getEfuseMac()`)
// rather than a generated/persisted UUID — the MAC is already unique and
// stable for the life of the chip, so there's nothing to store.

#pragma once

#include <Arduino.h>

namespace device_id {

const String &get();

}  // namespace device_id
