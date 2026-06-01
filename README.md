# usb-hotplug

Cross-platform USB hotplug listener for Node.js. Native Rust core via [nusb](https://github.com/kevinmehall/nusb) -- no libusb, no C dependencies, built-in TypeScript types.

**Supports macOS, Linux, and Windows.**

## Features

- Real-time USB connect/disconnect events
- Built-in debouncing -- OS-level re-enumeration noise (e.g. iPhones) is coalesced into a single event
- List all currently connected USB devices
- Zero JS dependencies -- single native binary
- Full TypeScript support out of the box

## Install

```bash
npm install usb-hotplug
```

## Quick start

```ts
import { watchDevices, stopWatching, listDevices } from "usb-hotplug"

// List devices currently connected
const devices = listDevices()
console.log(devices)

// Watch for connect / disconnect
watchDevices((err, event) => {
  if (err) throw err
  console.log(event.eventType, event.serialNumber)
  if (event.device) console.log(event.device)
})

// Stop when done
process.on("SIGINT", () => {
  stopWatching()
  process.exit(0)
})
```

## API

### `listDevices(): UsbDevice[]`

Returns all currently connected USB devices.

### `watchDevices(callback, debounceMs?): void`

Starts listening for USB hotplug events. Only one watcher can be active at a time.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `callback` | `(err, event) => void` | -- | Called for each event |
| `debounceMs` | `number` | `500` | Debounce window in ms |

The debounce window coalesces rapid event bursts caused by OS-level USB re-enumeration (a device appearing to disconnect and reconnect within milliseconds). With the default 500 ms window, you receive exactly one `Connected` event per physical plug-in.

### `stopWatching(): void`

Stops the active watcher. The background thread exits after the current event or on process exit.

### Types

```ts
interface UsbEvent {
  eventType: "Connected" | "Disconnected"
  deviceId: string
  serialNumber?: string
  device?: UsbDevice       // present on Connected, absent on Disconnected
}

interface UsbDevice {
  vendorId: number
  productId: number
  manufacturer?: string
  product?: string
  serialNumber?: string
  deviceClass: number       // raw bDeviceClass; 0 for HID/composite devices (see note)
  deviceSubclass: number
  deviceProtocol: number
  busId: string
  deviceAddress: number
  interfaces: UsbInterface[] // per-interface class info (the real function class)
}

interface UsbInterface {
  interfaceNumber: number
  class: number
  subclass: number
  protocol: number
  interfaceString?: string
}
```

> **Note on `deviceClass`**
>
> The device-level `deviceClass` (`bDeviceClass` in the USB spec) is
> legitimately `0x00` for HID and composite devices -- keyboards, trackpads,
> mice, iPhones, etc. A value of `0x00` means the class is declared
> *per-interface*, not at the device level. To determine what such a device
> actually is, read `interfaces[].class` (e.g. `0x03` = HID, `0x06` =
> still-image/PTP, `0xFF` = vendor-specific). On Windows, `interfaces` may be
> empty for non-composite devices bound to a single driver.

## Platform support

| Platform | Architecture | Status |
|---|---|---|
| macOS | arm64, x86_64 | Supported |
| Linux | x86_64 | Supported |
| Windows | x86_64 | Supported |

## Building from source

Requires [Rust](https://rustup.rs/) and Node.js >= 16.

```bash
npm install
npm run build
```

## License

[MIT](LICENSE)
