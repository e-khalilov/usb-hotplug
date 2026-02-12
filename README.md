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
  deviceClass: number
  deviceSubclass: number
  deviceProtocol: number
  busId: string
  deviceAddress: number
}
```

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
