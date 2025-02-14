# ESP32C3 + Embassy 练习项目

这是一个练习项目，旨在研究如何在ESP32C3上使用Embassy进行开发。由于很难找到系统性的参考项目，大多数都是简单的main函数示例，因此只能自己实践一下。本项目的目标是实现一些实用功能，尽管功能比较杂，但都是我在考虑实际业务需求时需要的一些功能。由于时间有限，项目实现较为粗糙，但希望能对有类似需求的人有所参考。觉得有参考意义的人就看一看，本来也只是作为我自己的练习与记录。


项目功能如下：

## 1. 通过 AP 模式启动 WiFi
- 设备启动时进入 AP 模式。
- 通过手机连接后进行 WiFi 配网操作。

## 2. 简单的 DHCP 服务
- 提供一个固定 IP 地址，只用于初次配网。
- 无需完整的 DHCP 服务。

## 3. 对 WiFi 使用进行处理
- 在程序内可随时方便使用网络请求。
- 当WiFi长时间未使用时自动关闭WiFi节省电量，当使用WiFi时自动拉起。

## 4. 简易 Web 服务
- 启动一个基本的 Web 服务器。
- 通过手机访问网页进行设备设置。

## 5. EC11 和按键事件监测
- 监测 EC11 旋转编码器和按键事件。
- 检测长按、短按、双击等事件。
- 实现 EC11 的转动速度计算。

## 6. 简易事件监听功能
- 注册事件回调函数。
- 事件触发时运行相应的回调函数。

## 7. 时间功能
- 使用 SNTP 同步时间。
- 显示当前时间。

## 8. 天气预报
- 通过心知天气 API 获取天气信息。
- 显示天气预报。

## 9. CHIP-8 模拟器
- 在程序内含一个 CHIP-8 模拟器。
- 运行 CHIP-8 游戏或应用。

## 10. 通过 PWM 播放简单的 WAV 音频文件
- 使用 PWM 播放 WAV 音频文件。
- 提供基本的音频播放功能。

## 11. 设置界面显示二维码
- 生成和显示一个设置地址的二维码。
- 方便手机扫码进入设置页面。

## 12. 通过 NVS 保存信息
- 使用 NVS（非易失性存储）保存配置信息。
- 确保重要数据在重启后仍然可用。

## 13. 低功耗处理
- 方便进入睡眠模式降低功耗。

## 14. 程序页面管理
- 简易的页面管理系统，方便从主窗口进入各子程序页面。

---

### 结语

整个项目的设计和实现过程中仍然存在许多问题，需要进一步优化和改进。由于时间有限，只能粗略实现部分功能。希望这些内容对有相似需求的开发者有所帮助，如果有不妥之处，欢迎指正，一同学习进步。

希望这个项目能为大家在使用ESP32C3和Embassy开发时提供一些参考和借鉴。

​
参考的项目：

[claudiomattera/esp32c3-embassy: A Rust async firmware for ESP32-C3 for reading and displaying sensor values using Embassy (github.com)](https://github.com/claudiomattera/esp32c3-embassy)

[vpikulik/sntpc_embassy: Example how to integrate sntpc crate with emabssy on rpi-pico w ([github.com](https://github.com/vpikulik/sntpc_embassy))](https://github.com/vpikulik/sntpc_embassy)


​
