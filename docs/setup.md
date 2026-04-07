# Auralis Setup Guide

This guide provides detailed instructions for setting up and configuring Auralis on your system.

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Installation](#installation)
3. [Model Setup](#model-setup)
4. [Configuration](#configuration)
5. [Platform-Specific Setup](#platform-specific-setup)
6. [Troubleshooting](#troubleshooting)
7. [Development Setup](#development-setup)

## System Requirements

### Minimum Requirements

- **Operating System**:
  - macOS 11.0 (Big Sur) or later
  - Windows 10 or later
  - Linux (Ubuntu 20.04 or equivalent)

- **Processor**:
  - x86_64 (Intel/AMD) with AVX2 support
  - Apple Silicon (M1/M2/M3)

- **Memory**:
  - 8GB RAM minimum
  - 16GB RAM recommended for optimal performance

- **Storage**:
  - 5GB free space for application
  - Additional 2-4GB for AI models

- **Audio**:
  - Working microphone
  - Audio output (speakers or headphones)

### Software Requirements

- **Rust**: 1.70 or later
- **Node.js**: 18.0 or later
- **Python**: 3.10 or later
- **Git**: For cloning the repository

## Installation

### Step 1: Install Prerequisites

#### macOS

```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Rust
brew install rust

# Install Node.js
brew install node

# Install Python
brew install python@3.11
```

#### Windows

```powershell
# Install Rust from rustup
curl https://sh.rustup.rs -sSf | sh

# Install Node.js from nodejs.org or via chocolatey
choco install nodejs

# Install Python from python.org or via chocolatey
choco install python
```

#### Linux (Ubuntu/Debian)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install Python
sudo apt-get update
sudo apt-get install -y python3.11 python3-pip
```

### Step 2: Clone the Repository

```bash
git clone <repository-url>
cd auralis
```

### Step 3: Install Rust Dependencies

```bash
# Build the Rust library
cargo build --release
```

### Step 4: Install Frontend Dependencies

```bash
# Install Node.js packages
npm install
```

### Step 5: Set Up Python Environment

```bash
# Create virtual environment (recommended)
cd python
python3 -m venv venv

# Activate virtual environment
# macOS/Linux:
source venv/bin/activate
# Windows:
# venv\Scripts\activate

# Install Python dependencies
pip install -r requirements.txt

# Go back to project root
cd ..
```

## Model Setup

Auralis requires three AI models for operation:

1. **Whisper** - Speech-to-Text
2. **MADLAD** - Translation
3. **Silero VAD** - Voice Activity Detection

### Model Directory Structure

Create the models directory:

```bash
mkdir -p models
```

Your directory structure should look like:

```
auralis/
├── models/
│   ├── whisper.bin           # Whisper STT model
│   ├── madlad/               # MADLAD translation model
│   │   ├── model.pth
│   │   └── config.json
│   └── silero_vad.torch      # Silero VAD model
```

### Downloading Models

#### Whisper Model

1. Visit the Whisper model repository
2. Download the desired model size:
   - `tiny`: ~39MB - Fastest, lowest accuracy
   - `base`: ~74MB - Good balance
   - `small`: ~244MB - Better accuracy
   - `medium`: ~769MB - High accuracy
   - `large`: ~1550MB - Best accuracy

3. Save as `models/whisper.bin`

```bash
# Example: Download base model using curl
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin -o models/whisper.bin
```

#### MADLAD Model

1. Download MADLAD model from Hugging Face
2. Extract to `models/madlad/` directory

```bash
# Example directory structure
mkdir -p models/madlad
# Copy model files to models/madlad/
```

#### Silero VAD Model

```bash
# Download Silero VAD model
curl -L https://huggingface.co/snakers4/silero-vad/resolve/main/silero_vad.torch -o models/silero_vad.torch
```

### Model Verification

Verify models are properly installed:

```bash
# Check model files exist
ls -lh models/

# Expected output:
# whisper.bin
# madlad/
# silero_vad.torch

# Test model loading
cargo test model_loading -- --nocapture
```

## Configuration

### Environment Variables

Create a `.env` file in the project root or set environment variables:

```bash
# Model paths
AURALIS_MODELS_DIR="./models"
AURALIS_STT_MODEL="${AURALIS_MODELS_DIR}/whisper.bin"
AURALIS_TRANSLATION_MODEL="${AURALIS_MODELS_DIR}/madlad"
AURALIS_VAD_MODEL="${AURALIS_MODELS_DIR}/silero_vad.torch"

# Audio configuration
AURALIS_SAMPLE_RATE="16000"
AURALIS_CHANNELS="1"

# Logging
RUST_LOG="info"
```

### Application Configuration

Configure settings through the application UI or by editing configuration files:

**Default Configuration** (`src/infrastructure/container.rs`):

```rust
pub struct ContainerConfig {
    pub models_dir: PathBuf,
    pub audio: AudioCaptureConfig,
    pub stt: WhisperConfig,
    pub translation: MadladConfig,
    pub vad: SileroConfig,
    pub source_language: String,
    pub target_language: String,
}
```

## Platform-Specific Setup

### macOS

#### Microphone Permissions

1. Open System Preferences
2. Go to Security & Privacy > Privacy
3. Select Microphone from the list
4. Enable Auralis

#### Accessibility (for global shortcuts)

1. System Preferences > Security & Privacy > Privacy
2. Select Accessibility
3. Add Auralis to the list

#### Developer Setup

```bash
# Install additional development tools
brew install llvm cmake

# Enable debug builds
export RUST_LOG=debug
npm run tauri dev
```

### Windows

#### Microphone Access

1. Open Settings > Privacy > Microphone
2. Enable "Allow apps to access your microphone"
3. Enable Auralis in the app list

#### Firewall Configuration

1. Windows Defender Firewall > Allow an app through firewall
2. Add Auralis to allowed applications

#### Developer Setup

```powershell
# Install Visual Studio Build Tools
# Install C++ build tools for Rust compilation

# Enable debug logging
set RUST_LOG=debug
npm run tauri dev
```

### Linux

#### Audio Configuration

**PulseAudio**:

```bash
# Install PulseAudio
sudo apt-get install pulseaudio pulseaudio-utils

# Test microphone
pactl list sources short
arecord -f cd -d 5 test.wav
```

**ALSA**:

```bash
# Install ALSA tools
sudo apt-get install alsa-utils

# Test microphone
arecord -l
arecord -f cd -d 5 test.wav
```

#### Permissions

```bash
# Add user to audio group
sudo usermod -aG audio $USER

# Log out and log back in for changes to take effect
```

#### Developer Setup

```bash
# Install ALSA development files
sudo apt-get install libasound2-dev

# Install X11 development files (for GUI)
sudo apt-get install libx11-dev libxext-dev libxft-dev libxrandr-dev

# Enable debug logging
RUST_LOG=debug npm run tauri dev
```

## Troubleshooting

### Common Issues and Solutions

#### Issue: Models not found

**Symptoms**: Application shows "Models not ready" error

**Solutions**:
1. Verify model files exist in the correct location:
   ```bash
   ls -lh models/whisper.bin
   ls -lh models/madlad/
   ls -lh models/silero_vad.torch
   ```

2. Check file permissions:
   ```bash
   chmod 644 models/whisper.bin
   chmod -R 755 models/madlad/
   chmod 644 models/silero_vad.torch
   ```

3. Verify environment variables:
   ```bash
   echo $AURALIS_MODELS_DIR
   echo $AURALIS_STT_MODEL
   ```

#### Issue: Audio capture fails

**Symptoms**: No audio input or "Audio device error"

**Solutions**:
1. Test microphone with system tools:
   ```bash
   # macOS/Linux
   arecord -f cd -d 5 test.wav

   # Windows
   # Use Sound Recorder app
   ```

2. Check application permissions:
   - macOS: System Preferences > Security > Privacy > Microphone
   - Windows: Settings > Privacy > Microphone
   - Linux: Verify ALSA/PulseAudio configuration

3. List available audio devices:
   ```bash
   # Linux
   pactl list sources short
   arecord -l

   # macOS
   ffmpeg -f avfoundation -list_devices true -i ""
   ```

#### Issue: Python integration fails

**Symptoms**: "Python module not found" or PyO3 errors

**Solutions**:
1. Verify Python installation:
   ```bash
   python3 --version
   which python3
   ```

2. Check PyO3 installation:
   ```bash
   pip list | grep pyo3
   ```

3. Reinstall Python dependencies:
   ```bash
   cd python
   pip install -r requirements.txt --upgrade
   ```

4. Set Python path explicitly:
   ```bash
   export PYTHONPATH="/path/to/python/site-packages"
   ```

#### Issue: Performance problems

**Symptoms**: High latency or slow response times

**Solutions**:
1. Use smaller model sizes:
   ```rust
   // In configuration, use smaller models
   let config = WhisperConfig {
       model_type: ModelType::Tiny,  // Instead of Medium or Large
   };
   ```

2. Adjust audio buffer size:
   ```rust
   let audio_config = AudioCaptureConfig {
       buffer_size: 512,  // Smaller buffer = lower latency
   };
   ```

3. Run performance benchmarks:
   ```bash
   cargo bench
   ```

4. Monitor system resources:
   ```bash
   # macOS
   top -o cpu

   # Linux
   htop

   # Windows
   Task Manager
   ```

#### Issue: Build errors

**Symptoms**: Cargo build fails with compilation errors

**Solutions**:
1. Update Rust toolchain:
   ```bash
   rustup update
   rustup default stable
   ```

2. Clean build cache:
   ```bash
   cargo clean
   cargo build
   ```

3. Check for missing system dependencies:
   ```bash
   # macOS
   brew install llvm cmake

   # Linux
   sudo apt-get install build-essential libssl-dev pkg-config

   # Windows
   # Install Visual Studio Build Tools
   ```

### Debug Mode

Enable detailed logging for troubleshooting:

```bash
# Maximum verbosity
RUST_LOG=trace npm run tauri dev

# Component-specific logging
RUST_LOG=auralis::infrastructure::audio=debug npm run tauri dev

# Multiple components
RUST_LOG=auralis::infrastructure=debug,auralis::application=info npm run tauri dev
```

### Log Files

Check log files for detailed error information:

```bash
# View main log
tail -f logs/auralis.log

# View error log
tail -f logs/errors.log

# Search for specific errors
grep ERROR logs/auralis.log
```

## Development Setup

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Specific test
cargo test test_audio_capture

# With output
cargo test -- --nocapture
```

### Running Benchmarks

```bash
# All benchmarks
cargo bench

# Specific benchmark
cargo bench --bench latency

# Save results
cargo bench -- --save-baseline main
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run linter
cargo clippy

# Fix linter warnings
cargo clippy --fix
```

### Documentation

```bash
# Generate documentation
cargo doc --open

# Document all dependencies
cargo doc --document-private-items --open
```

## Additional Resources

### Language Support

**Speech-to-Text (Whisper)**:
- Supports 99 languages
- Best performance with major languages (English, Spanish, Chinese, etc.)

**Translation (MADLAD)**:
- 200+ language pairs
- Best for major world languages

**Common Language Pairs**:
- English ↔ Spanish
- English ↔ Chinese
- English ↔ French
- English ↔ German
- English ↔ Japanese
- And many more

### Performance Tuning

1. **Model Selection**:
   - Use smaller models for faster performance
   - Use larger models for better accuracy

2. **Hardware Acceleration**:
   - GPU support not yet implemented
   - CPU optimization improvements planned

3. **Buffer Sizes**:
   - Smaller buffers = lower latency, higher CPU usage
   - Larger buffers = higher latency, lower CPU usage

### Community Support

- **Issues**: Report bugs on GitHub Issues
- **Discussions**: Use GitHub Discussions for questions
- **Contributions**: Pull requests welcome

---

**Last Updated**: 2025
**Version**: 0.1.0
