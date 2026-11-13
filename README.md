# Zbody Creative Suite 🚀

**The Open-Source, Local-First Alternative to Adobe Creative Cloud.**
*(A for Adobe, Z for Zero Subscriptions.)*

欢迎来到 **Zbody Creative Suite** —— 一个专为极客、学生和独立创作者打造的纯本地开源创作套件。

我们受够了臃肿的安装包、无休止的订阅弹窗、以及强制上传云端的数据绑架。Zbody 的命名，源自字母表最后的 "Z"，旨在向统治设计界已久的 "A"（Adobe）宣战。我们用 **Rust** 原生代码，为你打造一个真正属于你自己的数字工作台。

---

## 📦 套件成员 (The Modules)

Zbody 采用模块化架构，你可以根据需求独立使用每一个工具（注：项目正在快速迭代中）。

| 模块 | 对标产品 | 当前状态 | 核心功能描述 |
| :--- | :--- | :--- | :--- |
| **[Zbody Photo](./Zbody-Photo)** | Adobe Photoshop | ✅ `v0.1` 预览版 | 极速位图编辑。支持图层管理、混合模式、滤镜调整，以及 `.zbody` 私有工程格式。 |
| **[Zbody PDF](./Zbody-PDF)** | Adobe Acrobat | 🚧 核心框架开发中 | 纯本地 PDF 阅读与处理。拒绝弹窗广告，提供极致的深色阅读体验与基础文档操作。 |
| **[Zbody Vector](./Zbody-Vector)** | Adobe Illustrator | 📅 规划中 | 原生矢量绘图。基于纯数学计算的贝塞尔曲线引擎，无惧放大，极致丝滑。 |

---

## ✨ 核心哲学 (Why Zbody?)

*   **🦀 100% Rust 原生**：基于 `egui` / `eframe` 构建。拒绝 Electron 套壳，拒绝 WebView，安装包极小，启动速度以毫秒计，内存占用极低。
*   **🛡️ 绝对隐私 (Local-First)**：无需注册账号，无需连接网络。断网状态下依然可以完美工作，你的创意和素材永远不会离开你的硬盘。
*   **🔓 免费与开源**：遵循 MIT 协议。没有 VIP 限制，没有功能阉割，代码完全公开，用开源对抗资本垄断。
*   **🚫 无追踪，无广告**：软件内不包含任何遥测代码，没有开屏广告，没有后台偷跑的更新服务。

---

## 🛠️ 技术栈 (Tech Stack)

*   **语言**: Rust
*   **GUI 框架**: `egui` / `eframe` (纯原生，零 Web 依赖)
*   **图像引擎**: `image`, `imageproc`, `rayon` (多线程并行)
*   **矢量引擎**: `kurbo` (底层贝塞尔曲线计算)

---

## 📥 下载与安装 (Download)

请前往本仓库右侧的 **[Releases](https://github.com/yumpepper21/Zbody-Creative-Suite/releases)** 页面，下载最新版本的压缩包（`.7z` 格式）。

> ⚠️ **注意事项：**
> 1. 本软件由个人开发者编译，未购买商业数字签名，杀毒软件可能会误报。请点击“更多信息” -> “仍要运行”。
> 2. 请使用 **7-Zip** 或 **Nanazip** 解压后再运行。
> 3. 目前为早期版本，部分高级功能仍在开发路线图中。

---

## 💻 从源码构建 (Build from Source)

如果你是开发者，想体验最新版本或参与共建：

```bash
# 克隆本仓库及其子模块
git clone https://github.com/yumpepper21/Zbody-Creative-Suite.git
cd Zbody-Creative-Suite

# 运行或编译 (需要安装 Rust 工具链)
cargo run --release
