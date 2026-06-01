use napi_derive::napi;

/// Summary information about a single USB interface, taken from the
/// interface descriptor of the device's active configuration.
///
/// This is where the real device function class lives for HID and
/// composite devices (keyboards, trackpads, iPhones, etc.), whose
/// device-level `bDeviceClass` is `0x00` per the USB specification.
#[napi(object)]
pub struct UsbInterface {
  pub interface_number: u32,
  pub class: u32,
  pub subclass: u32,
  pub protocol: u32,
  pub interface_string: Option<String>,
}

#[napi(object)]
pub struct UsbDevice {
  pub vendor_id: u32,
  pub product_id: u32,
  pub manufacturer: Option<String>,
  pub product: Option<String>,
  pub serial_number: Option<String>,
  /// Raw `bDeviceClass` from the device descriptor. This is legitimately
  /// `0x00` for HID and composite devices, which declare their class
  /// per-interface. Inspect `interfaces` for the actual function class.
  pub device_class: u32,
  pub device_subclass: u32,
  pub device_protocol: u32,
  pub bus_id: String,
  pub device_address: u32,
  /// Interfaces in the device's active configuration. May be empty on
  /// Windows for non-composite devices bound to a single driver.
  pub interfaces: Vec<UsbInterface>,
}

impl UsbDevice {
  pub fn from_device_info(info: &nusb::DeviceInfo) -> Self {
    Self {
      vendor_id: info.vendor_id() as u32,
      product_id: info.product_id() as u32,
      manufacturer: info.manufacturer_string().map(|s| s.to_string()),
      product: info.product_string().map(|s| s.to_string()),
      serial_number: info.serial_number().map(|s| s.to_string()),
      device_class: info.class() as u32,
      device_subclass: info.subclass() as u32,
      device_protocol: info.protocol() as u32,
      bus_id: info.bus_id().to_string(),
      device_address: info.device_address() as u32,
      interfaces: info
        .interfaces()
        .map(|iface| UsbInterface {
          interface_number: iface.interface_number() as u32,
          class: iface.class() as u32,
          subclass: iface.subclass() as u32,
          protocol: iface.protocol() as u32,
          interface_string: iface.interface_string().map(|s| s.to_string()),
        })
        .collect(),
    }
  }
}

#[napi(string_enum)]
pub enum UsbEventType {
  Connected,
  Disconnected,
}

#[napi(object)]
pub struct UsbEvent {
  pub event_type: UsbEventType,
  pub device: Option<UsbDevice>,
  pub device_id: String,
  pub serial_number: Option<String>,
}
