#include "device_id.h"

namespace device_id {

const String &get() {
    static String id;
    if (id.isEmpty()) {
        uint64_t mac = ESP.getEfuseMac();
        char buf[20];
        snprintf(buf, sizeof(buf), "espia-%012llx", static_cast<unsigned long long>(mac));
        id = buf;
    }
    return id;
}

}  // namespace device_id
