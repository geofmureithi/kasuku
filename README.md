# Kasuku

Kasuku is lightweight, extensible all-in-one planning tool that makes every day more efficient.

## Features 🌟

- **Efficient & Lightweight**: Built with Rust and WASM with a low memory footprint and light plugins.
- **Cross-Platform**: Runs smoothly on Web, Mobile, and Desktop.
- **Customizable**: Extend functionality with Rust-based plugins compiled into WASM.
- **Dynamic SQL Engine**: Powerful data management with an inbuilt SQL engine based on sqlite.


## In Progress plugins 🎉

### 📝 Tasks

Prioritize, organize, and tackle your goals. [🔗](/plugins/tasks/)

### 🤓 DataView

SQL + Markdown = Magic! Query your heart out and manipulate data like a wizard. [🔗](/plugins/dataview/)

### 🎨 Templates

Say goodbye to monotony. Automate with templates and free up time for the fun stuff. [🔗](/plugins/template/)

## But Wait, There's More! 🚀 Upcoming Plugins:

### 📊 Dashboard

Visualize your progress with customizable dashboards.

### 📅 Calendar

Seamlessly integrate with your favorite calendars

## Help us Build

### 🧠 Mind Mapper

Create a plugin that can generate a mind map

### 🌐 Web Clipper

Allow the ability to clip from the web into a Kasuku file

### 🕹️ Gamify

Turn tasks into a game. Earn points, level up, and make productivity an adventure.

## Installation

Check the Releases page for ready made binaries.

## Development

1. Clone this repository: `git clone https://github.com/geofmureithi/kasuku.git`

2. Navigate to the project directory: `cd kasuku`

3. Build and run the backend: `cargo xtask dev`

4. Open your web browser and go to: `http://localhost:8080`

## Roadmap

### Core
- [x] Plugin Interface API
- [x] Context APi
- [x] Robust and dynamic SQL Engine
- [x] Markdown parsing
- [ ] LLM integration
- [ ] Cross plugin communication
- [ ] Plugin distribution via OCI

### Backend
- [x] Basic REST API
- [ ] Indexing files
- [ ] Document API

### Frontend
- [x] Basic UI
- [ ] Listing entries
- [ ] Single view
- [ ] Extensible UI
- [ ] Dashboard
- [ ] Block editor using *edita*
- [ ] Live-view integration with plugins

### Tauri
- [x] Basic setup
- [ ] Use random port
- [ ] Setup CI to produce binaries

### Plugins
- [x] Tasks
- [x] DataView
- [ ] Templating

## Contributing

Contributions are welcome! If you find any bugs or have ideas for improvements or a cool awesome plugin, please open an issue or submit a pull request.

## License

This project is licensed under the [GPLv3 License](LICENSE.md).

---
