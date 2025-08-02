# Bevy Streaming

This is a Bevy plugin for Cloud Gaming.

![Alt text](screenshots/simple.jpg)

It allows to stream Bevy's camera to a streaming server (through WebRTC) with ultra-low latency, and play the game through a simple browser or phone.

You can imagine any kind of game of application, using cloud provider's powerful GPUs, and simply stream the content to any device compatible with WebRTC.

The player can then play from his browser or any device compatible with WebRTC. The input events are sent through a WebRTC data channel.

## Features

- Headless GPU/CPU Acceleration for 2D/3D rendering using Vulkan or any other
- NVIDIA NVENC for H264/H265 encoding through GStreamer's provided plugins to provide high-quality low-latency video streaming
- Software encoding for VP8/VP9/H264/H265 codecs using GStreamer's provided plugins
- Congestion Control algorithm (provided by GStreamer's webrtcsink element)
- Multiple signalling server options:
  - GstWebRTC
  - PixelStreaming
  - Soon: (supported by GStreamer natively)
    - Amazon Kinesis
    - Janus
    - livekit
    - WHIP
- Implementation of Unreal's Pixel Streaming signalling server protocol to send video and receive mouse/keyboard controls
- Easy configuration of cameras using an helper
- Support for multiple cameras (each cameras is a streamer, and a streamer is a resource)

## Prerequisites

- Ubuntu 24.04 for up-to-date gstreamer (or you'll have to build it from source)

Install the following libraries:

```bash
sudo apt-get install \
    libssl-dev \
    libvulkan-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-nice  \
    gstreamer1.0-tools \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-plugins-good1.0-dev \
    libgstreamer-plugins-bad1.0-dev \
    libasound2-dev
```

Upgrade Rust if needed (Rust edition 2024):

```bash
rustup update stable
```

## Running the example

First, start a PixelStreaming signalling server with the following command:

```bash
docker run --rm -t -i --network=host pixelstreamingunofficial/pixel-streaming-signalling-server:5.4
```

_Note: 5.5 version has a default feature enabled that makes the WebRTC connection fail on some versions of Chrome._

### Run the example from your computer

Launch the example:

```bash
cargo run --example simple
```

### Build the headless Docker image

I've provided a Dockerfile in `docker/Dockerfile` that runs the example as a starting point for you to build your own Docker images.

The Dockerfile is optimized to allow caching of dependencies and prevent unnecessary rebuilds if Cargo.toml is not changed.

Using a multi-stage build also allows to reduce the Docker image size. Of course, many improvements can still be made, PR welcome!

To build the example docker image, run the following command:

```bash
docker build . -f docker/Dockerfile -t bevy_streaming
```

### Run the Docker image

#### Without GPU (not recommended)

To run the docker image without GPU, run the following command:

```bash
docker run --rm \
    -t -i \
    --network=host \
    bevy_streaming
```

_Note: you can ignore the messages `ERROR wgpu_hal::gles:` see https://github.com/bevyengine/bevy/issues/13115._

#### With NVIDIA GPU acceleration (recommended if you have a NVIDIA GPU)

To run the docker image with NVIDIA GPU acceleration, after having installed NVIDIA Container Toolkit (https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html), run the following command:

```bash
docker run --rm \
    -t -i \
    --network=host \
    --runtime nvidia \
    --gpus all \
    -e NVIDIA_VISIBLE_DEVICES=all \
    -e NVIDIA_DRIVER_CAPABILITIES=video,graphics \
    bevy_streaming
```

_Note: you must have a recent version of NVIDIA Container Toolkit installed on your system.
If the Vulkan backend is not available, upgrading NVIDIA Container Toolkit might fix the issue.
See https://github.com/NVIDIA/nvidia-container-toolkit/issues/16 for more information._

Explanation of the parameters:

- `--rm` : removes the container after it exits.
- `-t -i` : runs the container in interactive mode with a pseudo-TTY (see the logs and stop it easily with Ctrl+C)
- `--network=host` : allows the container to access the host's network interfaces, to easily access the Signalling Server.
- `--runtime nvidia` : specifies the NVIDIA runtime for GPU acceleration.
- `--gpus all` : enables access to all available GPUs.
- `-e NVIDIA_VISIBLE_DEVICES=all` : sets the environment variable to make all GPUs visible to the container.
- `-e NVIDIA_DRIVER_CAPABILITIES=all` : sets the environment variable to enable all driver capabilities (see https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/1.10.0/user-guide.html#driver-capabilities).

#### With DRI GPU acceleration (recommended if you have an intel GPU)

```bash
docker run --rm \
    -t -i \
    --network=host \
    --device /dev/dri:/dev/dri \
    bevy_streaming
```

_Note: it is possible that the `vaapih264enc` encoder does not support CBR bitrate. There is a workaround that will be soon provided. If you're in this situation, you can change the encoder priority with this command:_

```bash
docker run --rm \
    -t -i \
    --network=host \
    --device /dev/dri:/dev/dri \
    -e GST_PLUGIN_FEATURE_RANK="x264enc:1000"
    bevy_streaming
```

This will force to use the CPU H264 encoder.

### Connect to the streamer

- Open the player window: http://localhost/?StreamerId=player&HoveringMouse=true
- Open the spectator window: http://localhost/?StreamerId=spectator

Click in each window to connect to the signalling server.

Freecam Controls:

- Mouse - Move camera orientation
- Scroll - Adjust movement speed
- Left - Hold to grab cursor
- KeyM - Toggle cursor grab
- KeyW & KeyS - Fly forward & backwards
- KeyA & KeyD - Fly sideways left & right
- KeyE & KeyQ - Fly up & down
- ShiftLeft - Fly faster while held

When you move in the player window, the spectator window will always look at you, the big red sphere.

_Note: the parameter `HoveringMouse=true` in url makes sending mouse events by simply hovering the window. If you disable it, the cursor will be grabbed when you click in the window. You can release grabbing of the cursor using `ESC` key._

_Note 2: in the example, the cursor is volontary shown so you can easily have an idea of the latency._

## Thanks

This plugin would not have been possible without the following libraries:

- Bevy Engine (of course)
- Bevy Capture Plugin (https://crates.io/crates/bevy_capture) for headless capturing of frames to Gstreamer
- GStreamer (https://gstreamer.freedesktop.org/) and Rust Bindings
- Unreal Engine for their Pixel Streaming Infrastructure (https://github.com/EpicGamesExt/PixelStreamingInfrastructure)
