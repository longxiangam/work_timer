export SSID=redmi_longxiang
export PASSWORD=595744640+
cargo build --release
cargo espflash flash --chip esp32c3 --release --monitor
