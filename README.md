# TLC59xxx

Embedded-hal implementation for the TLC5947 & TLC59711


# Example


```rust
use linux_embedded_hal::{Pin, Spidev};
use tlc59xxx::TLC5947;

fn main() {
    let spi = Spidev::open("/dev/spidev0.0").unwrap();
    let lat = Pin::new(127);
    let mut tlc = TLC5947::new(spi, lat, 1);

    tlc.set_rgb(0, (200, 100, 4000));
    tlc.write().unwrap();

    let (_spi, _pin) = tlc.destroy();
}
```