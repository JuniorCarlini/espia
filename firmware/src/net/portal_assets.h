// Static assets for the captive portal page (net::provisioning) — vendored
// rather than loaded from a CDN, since the device's isolated setup access
// point has no path to the internet. Generated from
// agent/node_modules/tucano/dist/tucano.min.css (pinned to the same
// version the desktop agent vendors, ADR 0012) and the "Bitcount Grid
// Single" Google Font's basic-Latin WOFF2 subset (the same font the
// wordmark elsewhere in the project uses) — do not hand-edit either array.

#pragma once

#include <cstddef>
#include <cstdint>

extern const char TUCANO_MIN_CSS[];
extern const uint8_t BITCOUNT_WOFF2[];
extern const size_t BITCOUNT_WOFF2_LEN;
