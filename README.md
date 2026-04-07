# Auralis

A real-time speech-to-speech translation system built with Rust, Tauri, and Svelte.

## Overview

Auralis is a cutting-edge desktop application that provides real-time speech translation across multiple languages. By combining advanced speech recognition (Whisper), voice activity detection (Silero VAD), and neural machine translation (MADLAD), Auralis delivers accurate and responsive translation for desktop platforms.

### Key Features

- **Real-time Speech Recognition**: High-accuracy STT using Whisper models
- **Voice Activity Detection**: Intelligent speech detection with Silero VAD
- **Neural Machine Translation**: Multi-language translation via MADLAD models
- **Cross-Platform Desktop App**: Native performance on macOS, Windows, and Linux
- **Modern Web UI**: Responsive Svelte frontend with Vite
- **Modular Architecture**: Clean separation of concerns with dependency injection
- **Comprehensive Testing**: Unit and integration tests with performance benchmarks
- **Structured Logging**: Production-ready logging with tracing infrastructure

## Architecture

Auralis follows a clean, layered architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend (Svelte)                        │
│                  User Interface & Controls                   │
└────────────────────────┬────────────────────────────────────┘
                         │ Tauri Commands
┌────────────────────────▼────────────────────────────────────┐
│                  Application Layer                           │
│              Orchestrator & Event Bus                        │
└────────────────────────┬────────────────────────────────────┘
                         │ Domain Traits
┌────────────────────────▼────────────────────────────────────┐
│                   Infrastructure Layer                       │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │  Audio   │ │   STT    │ │  VAD     │ │Translation│       │
│  │ Capture  │ │ Whisper  │ │  Silero  │ │  MADLAD   │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
└─────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Backend**: Rust with Tokio async runtime
- **Desktop Framework**: Tauri 2.1
- **Frontend**: SvelteKit with Vite
- **Audio**: CPAL for cross-platform audio capture
- **Speech Recognition**: Whisper via whisper-rs
- **Voice Activity Detection**: Silero VAD via PyO3
- **Translation**: MADLAD via PyO3 Python integration
- **Logging**: tracing and tracing-subscriber
- **Testing**: Built-in Rust test framework with Criterion benchmarks

## Project Structure

```
auralis/
├── src/                      # Rust library code
│   ├── domain/              # Domain models and traits
│   ├── application/         # Application orchestration
│   ├── infrastructure/      # Concrete implementations
│   └── lib.rs              # Library entry point
├── src-tauri/              # Tauri desktop application
│   └── src/
│       ├── main.rs         # Application entry point
│       ├── commands.rs     # Tauri command handlers
│       └── state.rs        # Application state management
├── benches/                # Performance benchmarks
│   ├── latency.rs          # Latency measurements
│   └── memory.rs           # Memory usage benchmarks
├── tests/                  # Integration tests
│   ├── integration/        # Integration test suites
│   └── unit/              # Unit tests
├── docs/                   # Documentation
│   └── setup.md           # Detailed setup guide
├── python/                # Python integration code
│   └── models/            # Python model wrappers
└── Cargo.toml             # Rust dependencies
```

## Quick Start

### Prerequisites

- **Rust**: 1.70 or later
- **Node.js**: 18 or later
- **Python**: 3.10 or later
- **System Audio**: Working microphone and speakers

### Installation

1. **Clone the repository**:
   ```bash
   git clone <repository-url>
   cd auralis
   ```

2. **Install Rust dependencies**:
   ```bash
   cargo build
   ```

3. **Install frontend dependencies**:
   ```bash
   npm install
   ```

4. **Set up Python environment**:
   ```bash
   cd python
   pip install -r requirements.txt
   cd ..
   ```

5. **Download required models** (see [docs/setup.md](docs/setup.md) for details):
   ```bash
   # Download models to the models directory
   mkdir -p models
   # Add your model files here
   ```

### Running the Application

**Development mode** (with hot reload):
```bash
npm run tauri dev
```

**Production build**:
```bash
npm run tauri build
```

### Running Tests

**Unit tests**:
```bash
cargo test
```

**Integration tests**:
```bash
cargo test --test integration
```

**Performance benchmarks**:
```bash
cargo bench
```

## Usage

### Basic Translation

1. Launch the application
2. Select source and target languages
3. Click "Start Translation"
4. Speak into your microphone
5. View real-time transcription and translation

### Language Support

- **Speech-to-Text**: Multi-language support via Whisper
- **Translation**: Major language pairs via MADLAD
- **Supported Languages**: English, Spanish, Chinese, French, German, and more

See [docs/setup.md](docs/setup.md) for the complete language list.

## Configuration

### Model Paths

Configure model paths in the application settings or by setting environment variables:

```bash
export AURALIS_MODELS_DIR="/path/to/models"
export AURALIS_STT_MODEL="$AURALIS_MODELS_DIR/whisper.bin"
export AURALIS_TRANSLATION_MODEL="$AURALIS_MODELS_DIR/madlad"
export AURALIS_VAD_MODEL="$AURALIS_MODELS_DIR/silero_vad.torch"
```

### Logging

Control logging verbosity:

```bash
# Development (debug level)
RUST_LOG=debug npm run tauri dev

# Production (info level)
RUST_LOG=info ./auralis

# Component-specific logging
RUST_LOG=auralis::infrastructure=trace ./auralis
```

## Performance

### Benchmarks

Run performance benchmarks to measure system performance:

```bash
cargo bench
```

### Expected Performance

- **Latency**: < 200ms for end-to-end translation
- **Memory Usage**: ~500MB base + model size
- **CPU Usage**: Variable based on model complexity

## Troubleshooting

### Common Issues

**Models not found**: Ensure models are in the correct directory
- Check `models/` directory structure
- Verify model file permissions

**Audio capture issues**: Verify microphone permissions
- macOS: System Preferences > Security > Privacy > Microphone
- Windows: Settings > Privacy > Microphone
- Linux: Check ALSA/PulseAudio configuration

**Python integration errors**: Ensure Python environment is set up correctly
- Verify Python version compatibility
- Check PyO3 installation
- Review Python path configuration

For detailed troubleshooting, see [docs/setup.md](docs/setup.md).

## Development

### Code Structure

- **Domain Layer**: Core business logic and interfaces
- **Application Layer**: Orchestration and workflow management
- **Infrastructure Layer**: External service implementations
- **Presentation Layer**: UI components and user interactions

### Adding New Features

1. Define domain traits in `src/domain/traits.rs`
2. Implement interfaces in `src/infrastructure/`
3. Add orchestration in `src/application/`
4. Expose via Tauri commands in `src-tauri/src/commands.rs`
5. Build UI components in the Svelte frontend

### Testing Strategy

- **Unit Tests**: Test individual components in isolation
- **Integration Tests**: Test component interactions
- **Benchmarks**: Measure performance characteristics
- **Property Tests**: Verify invariants across inputs

## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

### Code Style

- Follow Rust naming conventions
- Use `rustfmt` for formatting
- Run `clippy` for linting
- Document public APIs with rustdoc

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- **Whisper**: OpenAI for speech recognition
- **Silero VAD**: Voice activity detection models
- **MADLAD**: Neural machine translation
- **Tauri**: Desktop application framework
- **Svelte**: Reactive UI framework

## Contact

For questions, issues, or contributions, please visit the project repository.

---

**Version**: 0.1.0
**Authors**: Sentia Lab
**Year**: 2025
