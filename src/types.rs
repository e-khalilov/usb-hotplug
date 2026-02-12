use napi_derive::napi;

#[napi(object)]
pub struct UsbDevice {
  pub vendor_id: u32,
  pub product_id: u32,
  pub manufacturer: Option<String>,
  pub product: Option<String>,
  pub serial_number: Option<String>,
  pub device_class: u32,
  pub device_subclass: u32,
  pub device_protocol: u32,
  pub bus_id: String,
  pub device_address: u32,
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
